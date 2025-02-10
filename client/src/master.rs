use std::{thread::{self, JoinHandle}, time::Duration};

use ipc_channel::ipc::{IpcReceiver, IpcSender};
use logger::{severity::LogSeverity, LogMessage, LogWorker};
use thread_safe_utils::{queue::ThreadSafeQueue, signal::{Signal, SignallableData}};

use crate::{
    IpcEnd,
    error::IpcError,
    messages::{Command, CommandID, DataMessage, Instruction, Message},
};

#[derive(Default)]
struct InstHelper {
    next_id: CommandID,
    pending_acknowledge: Vec<Instruction>
}

pub struct Master {
    ipc: IpcEnd<Instruction, Message>,
    helper: SignallableData<InstHelper>,
    logger: LogWorker,
    message_queue: ThreadSafeQueue<DataMessage>,
    receiver_thread: Option<JoinHandle<()>>,
    log_level: LogSeverity,
}

impl Drop for Master {
    fn drop(&mut self) {
        self.message_queue.set_signal(true);
        if let Some(thread) = self.receiver_thread.take() {
            let _ = thread.join();
        }
    }
}

impl Master {
    pub fn new(
        sender: IpcSender<Instruction>,
        receiver: IpcReceiver<Message>,
        logger: LogWorker,
    ) -> Self {
        let ipc = IpcEnd::new_with_timeout(sender, receiver, None, Some(Duration::from_secs(5)));
        let message_queue = ThreadSafeQueue::new();
        let queue = ipc.recv_queue.clone();
        let log = logger.clone();
        let helper = SignallableData::<InstHelper>::default();
        Self {
            ipc,
            helper: helper.clone(),
            logger,
            message_queue: message_queue.clone(),
            receiver_thread: Some({
                thread::spawn(move || {
                    while let Ok(message) = queue.dequeue() {
                        match message {
                            Message::Ready => {
                                let _ = log.log(LogMessage::new(LogSeverity::Info, "Client ready"));
                            }
                            Message::Ack(id) => {
                                let mut lock = helper.lock().unwrap();
                                let instructions = &mut lock.pending_acknowledge;
                                let inst = instructions
                                    .iter()
                                    .position(|instr| instr.id == id)
                                    .map(|ind| instructions.remove(ind));
                                let log_msg = if let Some(inst) = inst {
                                    LogMessage::new(
                                        logger::severity::LogSeverity::Info,
                                        format!(
                                            "Received acknowledgement for ID {} ({})",
                                            id, inst.command
                                        ),
                                    )
                                } else {
                                    LogMessage::new(
                                        logger::severity::LogSeverity::Warning,
                                        format!(
                                            "Received acknowledgement for unknown command with ID {}",
                                            id
                                        ),
                                    )
                                };
                                let _ = log.log(log_msg);
                                drop(lock);
                            }
                            Message::Exiting => {
                                let _ = log.log(LogMessage::new(LogSeverity::Info, "Client exiting..."));
                            }
                            Message::Log(dll_log_message) => {
                                let _ = log.log(dll_log_message);
                            }
                            Message::DataMessage(data_message) => {
                                let _ = message_queue.enqueue(data_message);
                            }
                        }
                    }
                    message_queue.set_signal(true);
                })
            }),
            log_level: LogSeverity::Info,
        }
    }

    pub fn with_log_level(mut self, sev: LogSeverity) -> Self {
        self.log_level = sev;
        self
    }

    fn log(&self, message: LogMessage) {
        let _ = self.logger.log(message);
    }

    pub fn send(&self, data: Command) -> Result<usize, IpcError> {
        let mut lock = self.helper.lock().unwrap();
        let id = {
            let tmp = lock.next_id;
            lock.next_id += 1;
            tmp
        };
        let data = Instruction { id, command: data };
        let instructions = &mut lock.pending_acknowledge;
        instructions.push(data.clone());
        self.log(LogMessage::new(
            logger::severity::LogSeverity::Info,
            format!("Sent Command {} with ID {}", data.command, data.id),
        ));
        let _ = self.ipc.send(data);
        Ok(id)
    }

    pub fn recv(&self) -> Result<DataMessage, IpcError> {
        self.message_queue.dequeue().map_err(|e| e.into())
    }
}
