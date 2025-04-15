use std::path::PathBuf;

use ipc_channel::ipc::{IpcReceiver, IpcSender};
use logger::{loggers::{file::{FileConflictBehavior, FileLogger}, filter::LogFilter, multi::MultiLogger}, severity::LogSeverity, LogManager, LogMessage, Logger};
use thread_safe_utils::queue::ThreadSafeQueue;
use widestring::Utf16String;
use windows::Win32::System::Threading::{GetCurrentProcessId, GetCurrentThreadId};

use crate::{
    error::IpcError, messages::{Command, DataMessage, Instruction, Message}, IpcEnd
};

mod json;

struct IpcLogger {
    queue: ThreadSafeQueue<Message>
}

impl Logger for IpcLogger {
    fn log(&mut self, message: &LogMessage) -> bool {
        self.queue.enqueue(message.into()).is_ok()
    }
}

pub struct Slave {
    ipc: IpcEnd<Message, Instruction>,
    log_manager: LogManager,
}

// Logging functions
impl Slave {
    pub fn log(&self, msg: LogMessage) -> Result<(), IpcError> {
        self.log_manager.log(msg);
        Ok(())
    }

    pub fn log_error(&self, msg: impl Into<String>) -> Result<(), IpcError> {
        let msg = LogMessage::new(LogSeverity::Error, msg.into());
        self.log(msg)
    }

    pub fn log_warn(&self, msg: impl Into<String>) -> Result<(), IpcError> {
        let msg = LogMessage::new(LogSeverity::Warning, msg.into());
        self.log(msg)
    }

    pub fn log_info(&self, msg: impl Into<String>) -> Result<(), IpcError> {
        let msg = LogMessage::new(LogSeverity::Info, msg.into());
        self.log(msg)
    }

    pub fn log_debug(&self, msg: impl Into<String>) -> Result<(), IpcError> {
        let msg = LogMessage::new(LogSeverity::Debug, msg.into());
        self.log(msg)
    }

    pub fn log_verbose(&self, msg: impl Into<String>) -> Result<(), IpcError> {
        let msg = LogMessage::new(LogSeverity::Verbose, msg.into());
        self.log(msg)
    }

}

impl Slave {
    pub fn new(sender: IpcSender<Message>, receiver: IpcReceiver<Instruction>, log_path: impl Into<PathBuf>) -> Self {
        let ipc = IpcEnd::new(sender, receiver);
        let ipc_logger = IpcLogger { queue: ipc.send_queue.clone() };
        let file_logger = FileLogger::new(log_path.into(), FileConflictBehavior::Overwrite).unwrap();
        let multi_logger = MultiLogger::new().with_logger(ipc_logger).with_logger(file_logger);
        let filter = LogFilter::new(LogSeverity::Verbose, multi_logger);
        let log_manager = LogManager::new(filter);
        Self {
            ipc,
            log_manager
        }
    }
    fn send(&self, msg: Message) -> Result<(), IpcError> {
        //self.log_verbose(format!("Sent `{:?}` message", msg))?;
        self.ipc.send(msg).map_err(|_| IpcError::SendError)
    }

    fn acknowledge(&self, inst: &Instruction) -> Result<(), IpcError> {
        self.log_verbose(format!("Acknowledging instruction {} ( {} )", inst.id, inst.command))?;
        self.send(Message::Ack(inst.id))
    }

    pub fn run_client(&self) -> Result<(), IpcError> {
        self.send(Message::Ready)?;
        while let Ok(inst) = self.ipc.recv() {
            self.acknowledge(&inst)?;
            match inst.command {
                crate::messages::Command::Quit => {
                    let _ = self.log_info("Quitting...");
                    break;
                },
                crate::messages::Command::FindJSON => {
                    match self.locate_json() {
                        Ok(strs) => self.send(DataMessage::Json(strs).into())?,
                        Err(e) => self.log_error(e.to_string())?,
                    }
                },
                Command::GetThreadId => {
                    let id = unsafe { GetCurrentThreadId() };
                    let _ = self.send(DataMessage::ThreadId(id).into());
                }
                Command::GetProcessId => {
                    let id = unsafe { GetCurrentProcessId() };
                    let _ = self.send(DataMessage::ProcessId(id).into());
                }
            }
        }
        self.send(Message::Exiting)?;
        Ok(())
    }
}
