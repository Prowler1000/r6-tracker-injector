use std::fmt::Display;

use logger::LogMessage;
use serde::{Deserialize, Serialize};

pub type CommandID = usize;

#[derive(Serialize, Deserialize, Clone)]
pub struct Instruction {
    pub id: CommandID,
    pub command: Command,
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}) {}", self.id, self.command)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Command {
    FindJSON,
    GetProcessId,
    GetThreadId,
    Quit,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Ready,
    Ack(CommandID),
    Exiting,
    Log(LogMessage),
    DataMessage(DataMessage)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DataMessage {
    Json(Vec<String>),
    ProcessId(u32),
    ThreadId(u32),
}

impl From<DataMessage> for Message {
    fn from(value: DataMessage) -> Self {
        Message::DataMessage(value)
    }
}

impl From<LogMessage> for Message {
    fn from(value: LogMessage) -> Self {
        Message::Log(value)
    }
}

impl From<&LogMessage> for Message {
    fn from(value: &LogMessage) -> Self {
        Message::Log(value.clone())
    }
}