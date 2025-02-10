use std::{thread::JoinHandle, time::{Duration, Instant}};

use error::IpcError;
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use serde::{Deserialize, Serialize};
use thread_safe_utils::{queue::ThreadSafeQueue, signal::Signal};

mod error;
pub mod master;
pub mod messages;
pub mod slave;

trait PipeData: Serialize + for<'a> Deserialize<'a> + Send + 'static {}
impl<T> PipeData for T where T: Serialize + for<'a> Deserialize<'a> + Send + 'static {}

#[allow(dead_code)]
struct IpcEnd<S: PipeData, R: PipeData> {
    send_queue: ThreadSafeQueue<S>,
    recv_queue: ThreadSafeQueue<R>,
    send_thread: Option<JoinHandle<Result<(), IpcError>>>,
    recv_thread: Option<JoinHandle<Result<(), IpcError>>>,
}

unsafe impl<S: PipeData, R: PipeData> Send for IpcEnd<S, R> {}
unsafe impl<S: PipeData, R: PipeData> Sync for IpcEnd<S, R> {}

impl<S: PipeData, R: PipeData> Drop for IpcEnd<S, R> {
    fn drop(&mut self) {
        self.send_queue.set_signal(true);
        self.recv_queue.set_signal(true);
        if let Some(thread) = self.send_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.recv_thread.take() {
            let _ = thread.join();
        }
    }
}

impl<S: PipeData, R: PipeData> IpcEnd<S, R> {
    pub fn new(sender: IpcSender<S>, receiver: IpcReceiver<R>) -> Self {
        Self::new_with_timeout(sender, receiver, Some(Duration::from_secs(5)), Some(Duration::from_secs(5)))
    }

    pub fn new_with_timeout(sender: IpcSender<S>, receiver: IpcReceiver<R>, recv_timeout: Option<Duration>, send_cleanup: Option<Duration>) -> Self {
        let recv_timeout = if let Some(time) = recv_timeout {
            time
        } else {
            Duration::from_secs(15)
        };
        let send_cleanup = if let Some(time) = send_cleanup {
            time
        } else {
            Duration::from_secs(15)
        };
        let sender = sender;
        let receiver = receiver;
        let send_queue = ThreadSafeQueue::new();
        let recv_queue = ThreadSafeQueue::new();
        let send_thread = {
            let send_queue = send_queue.clone();
            Some(std::thread::spawn(move || -> Result<(), IpcError> {
                // Dequeue will return ThreadSafeQueueError::StatusNotOk on signal
                while let Ok(data) = send_queue.dequeue() {
                    sender.send(data).inspect_err(|_e| {
                        send_queue.set_signal(true);
                    })?;
                }
                // Give threads a chance to place important messages in the queue
                let deadline = Instant::now() + send_cleanup;
                while let Ok(Some(data)) = send_queue.try_dequeue_timeout(deadline.duration_since(Instant::now())) {
                    sender.send(data)?;
                }
                Ok(())
            }))
        };
        let recv_thread = {
            let recv_queue = recv_queue.clone();
            Some(std::thread::spawn(move || -> Result<(), IpcError> {
                loop {
                    match receiver.try_recv_timeout(recv_timeout) {
                        Ok(data) => {
                            if recv_queue.enqueue(data).is_err() {
                                break;
                            }
                        }
                        Err(e) => match e {
                            ipc_channel::ipc::TryRecvError::IpcError(error) => {
                                recv_queue.set_signal(true);
                                return Err(error.into());
                            }
                            ipc_channel::ipc::TryRecvError::Empty => {}
                        },
                    }
                }
                Ok(())
            }))
        };
        Self {
            send_queue,
            recv_queue,
            send_thread,
            recv_thread,
        }
    }

    pub fn send(&self, data: S) -> Result<(), ()> {
        if let Some(thread) = self.send_thread.as_ref() {
            if !thread.is_finished() {
                // ThreadSafeQueue should never have its mutex poisioned, so this shouldn't be an issue
                self.send_queue.enqueue(data).unwrap();
                return Ok(());
            }
        }
        Err(())
    }

    pub fn recv(&self) -> Result<R, IpcError> {
        self.recv_queue.dequeue().map_err(|e| e.into())
    }

    #[allow(dead_code)]
    pub fn try_recv(&self) -> Result<Option<R>, IpcError> {
        if let Some(thread) = self.recv_thread.as_ref() {
            if thread.is_finished() {
                Err(IpcError::ThreadFinished)
            } else {
                Ok(self.recv_queue.try_dequeue())
            }
        } else {
            Err(IpcError::ThreadNotRunning)
        }
    }

    #[allow(dead_code)]
    pub fn try_recv_timeout(&self, duration: Duration) -> Result<Option<R>, IpcError> {
        self.recv_queue
            .try_dequeue_timeout(duration)
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::IpcEnd;

    #[test]
    fn basic_test() {
        let (tx1, rx1) = ipc_channel::ipc::channel::<String>().unwrap();
        let (tx2, rx2) = ipc_channel::ipc::channel::<String>().unwrap();
        let end1 = IpcEnd::new(tx1, rx2);
        let end2 = IpcEnd::new(tx2, rx1);
        let thread = std::thread::spawn(move || {
            while let Ok(thing) = end2.recv() {
                let _ = end2.send(format!("End 2 received: {}", thing));
            }
        });
        let _ = end1.send(String::from("I am a test string!"));
        let response = end1.recv().unwrap();
        println!("{}", response);
        assert!(response.eq(&format!("End 2 received: {}", "I am a test string!")));

        thread.join().unwrap();
    }
}
