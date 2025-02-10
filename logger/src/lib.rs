use std::{ops::Deref, thread::JoinHandle};

use chrono::{DateTime, Local, TimeDelta};
use serde::{Deserialize, Serialize};
use severity::LogSeverity;
use thread_safe_utils::{queue::ThreadSafeQueue, signal::Signal};

pub mod severity;
pub mod loggers;

pub trait Logger {
    fn log(&mut self, message: &LogMessage) -> bool;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogMessage {
    pub time: DateTime<Local>,
    pub severity: LogSeverity,
    pub content: String,
}

impl LogMessage {
    pub fn new(severity: LogSeverity, content: impl Into<String>) -> Self {
        let content = content.into();
        LogMessage { time: Local::now(), severity, content }
    }
}

/// A struct designed to hold references to queues, threads, and anything else that may be needed
/// for logging
pub struct LogManager {
    queue: ThreadSafeQueue<LogMessage>,
    thread: Option<JoinHandle<()>>,
    default_worker: LogWorker,
}

impl Drop for LogManager {
    fn drop(&mut self) {
        self.queue.set_signal(true);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl LogManager {
    pub fn new<F>(mut logger: F) -> Self
    where
        F: Logger,
        F: Send + 'static
    {
        let queue = ThreadSafeQueue::new();
        let thread = Some({
            let queue = queue.clone();
            std::thread::spawn(move || {
                while let Ok(message) = queue.dequeue() {
                    if !logger.log(&message) {
                        queue.set_signal(true);
                    }
                }
            })
        });
        let default_worker = LogWorker::new(queue.clone(), Local::now());
        Self {
            queue,
            thread,
            default_worker,
        }
    }

    pub fn get_log_worker(&self) -> LogWorker {
        self.default_worker.clone()
    }
}

impl Deref for LogManager {
    type Target = LogWorker;

    fn deref(&self) -> &Self::Target {
        &self.default_worker
    }
}

#[derive(Clone)]
pub struct LogWorker {
    queue: ThreadSafeQueue<LogMessage>,
    manager_start_time: DateTime<Local>
}

impl LogWorker {
    fn new(queue: ThreadSafeQueue<LogMessage>, manager_start_time: DateTime<Local>) -> Self {
        Self {
            queue,
            manager_start_time,
        }
    }
    pub fn log(&self, message: LogMessage) -> bool {
        self.queue.enqueue(message).is_ok()
    }

    pub fn time_since_start(&self) -> TimeDelta {
        self.manager_start_time.signed_duration_since(Local::now())
    }
}

