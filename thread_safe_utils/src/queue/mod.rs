use std::{
    collections::VecDeque,
    time::Duration,
};

use thiserror::Error;

use crate::signal::{self, Signal, SignallableData};

#[derive(Error, Debug)]
pub enum ThreadSafeQueueError {
    #[error("The queue mutex was poisioned.")]
    MutexPoision,
    #[error("The status is not OK.")]
    StatusNotOk,
}

#[derive(Default)]
pub struct ThreadSafeQueue<T: Send + 'static> {
    queue: SignallableData<VecDeque<T>>,
}

impl<T: Send + 'static> Clone for ThreadSafeQueue<T> {
    fn clone(&self) -> Self {
        Self { queue: self.queue.clone() }
    }
}

unsafe impl<T: Send + 'static> Send for ThreadSafeQueue<T> {}
unsafe impl<T: Send + 'static> Sync for ThreadSafeQueue<T> {}

impl<T: Send + 'static> Signal for ThreadSafeQueue<T> {
    fn is_signalled(&self) -> bool {
        self.queue.is_signalled()
    }

    fn wait_for_signal(&self) -> Result<(), signal::SignalResult> {
        self.queue.wait_for_signal()
    }

    fn set_signal(&self, value: bool) -> bool {
        self.queue.set_signal(value)
    }
}

impl<T: Send + 'static> ThreadSafeQueue<T> {
    pub fn new() -> Self {
        let queue = SignallableData::default();
        Self { queue }
    }

    pub fn dequeue(&self) -> Result<T, ThreadSafeQueueError> {
        let mut lock = self
            .queue
            .lock_wait_while(|queue, signal| queue.is_empty() && !signal)
            .map_err(|_| ThreadSafeQueueError::MutexPoision)?;
        if !lock.is_signalled() {
            Ok(lock.pop_front().unwrap())
        } else {
            Err(ThreadSafeQueueError::StatusNotOk)
        }
    }

    pub fn elements(&self) -> usize {
        self.queue.lock().map(|l| l.len()).unwrap_or_default()
    }

    pub fn try_dequeue(&self) -> Option<T> {
        self.queue
            .lock()
            .map_or_else(|_| None, |mut l| l.pop_front())
    }

    pub fn try_dequeue_timeout(&self, dur: Duration) -> Result<Option<T>, ThreadSafeQueueError> {
        self.queue
            .lock_wait_while_timeout(dur, |queue, _| queue.is_empty())
            .map(|opt| opt.map(|mut queue| queue.pop_front().unwrap()))
            .map_err(|_| ThreadSafeQueueError::MutexPoision)
    }

    pub fn enqueue(&self, data: T) -> Result<(), ThreadSafeQueueError> {
        let mut lock = self.queue.lock().map_err(|_| ThreadSafeQueueError::MutexPoision)?;
        lock.push_back(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_test() {
        let queue = ThreadSafeQueue::<String>::new();
        let str1 = "I am some string sent into the queue!";
        let str2 = "I am some other string sent into the queue";
        let str3 =
            "I am a third string, sent after reading one or two of the others from the queue";
        assert!(queue.enqueue(String::from(str1)).is_ok());
        assert!(queue.enqueue(String::from(str2)).is_ok());
        assert!(queue.dequeue().is_ok_and(|s| s.eq(str1)));
        assert!(queue.enqueue(String::from(str3)).is_ok());
        assert!(queue.dequeue().is_ok_and(|s| s.eq(str2)));
        assert!(queue.dequeue().is_ok_and(|s| s.eq(str3)));
        drop(queue);
    }
}
