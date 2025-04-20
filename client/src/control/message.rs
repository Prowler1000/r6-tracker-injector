use logger::LogMessage;
use serde::{Deserialize, Serialize};

use super::command::CommandID;

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