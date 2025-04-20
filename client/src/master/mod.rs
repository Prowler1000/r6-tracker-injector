use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::Duration,
};

use client_state::ClientState;
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use logger::{LogMessage, LogWorker, severity::LogSeverity};
use thread_safe_utils::{
    queue::ThreadSafeQueue,
    signal::{Signal, SignallableData},
};

mod client_state;

use crate::{
    IpcEnd,
    control::{
        command::{Command, CommandID, Instruction, PendingCommand},
        message::{DataMessage, Message},
    },
    error::IpcError,
};

#[derive(Default)]
struct InstHelper {
    next_id: CommandID,
    pending_acknowledge: Vec<Instruction>,
}

pub struct Master {
    ipc: IpcEnd<Instruction, Message>,
    state: Arc<SignallableData<ClientState>>,
    logger: LogWorker,
}

impl Drop for Master {
    fn drop(&mut self) {
        self.terminate();
    }
}

impl Master {
    pub fn new(
        sender: IpcSender<Instruction>,
        receiver: IpcReceiver<Message>,
        logger: LogWorker,
    ) -> Self {
        let ipc = IpcEnd::new_with_timeout(sender, receiver, None, Some(Duration::from_secs(5)));
        let helper = Arc::new(SignallableData::<ClientState>::default());
        Self {
            ipc,
            state: helper.clone(),
            logger,
        }
    }

    pub fn terminate(&self) {
        let _ = self.send(Command::Quit);
        self.state.set_signal(true);
    }

    fn log(&self, message: LogMessage) {
        let _ = self.logger.log(message);
    }

    pub fn send(&self, data: Command) -> Result<Arc<PendingCommand>, IpcError> {
        let mut lock = self.state.lock().unwrap();
        let id = lock.next_id();
        let (data, pending_inst, pending_cmd) = Instruction::new(id, data);
        lock.add_pending_inst(pending_inst);
        self.log(LogMessage::new(
            logger::severity::LogSeverity::Info,
            format!("Send Command {} with ID {}", data.command, data.id),
        ));
        self.ipc.send(data).map_err(|_| IpcError::PipeClosed)?;
        Ok(pending_cmd)
    }

    pub fn try_recv_one(&self) -> Result<Option<DataMessage>, IpcError> {
        let message = self.ipc.recv().inspect_err(|e| {
            self.terminate();
            self.log(LogMessage::new(
                LogSeverity::Error,
                format!("An error occured while receiving a message. {}", e),
            ));
        })?;
        match message {
            Message::Ready => {
                self.log(LogMessage::new(LogSeverity::Info, "Client ready"));
            }
            Message::Ack(id) => {
                let mut lock = self.state.lock().unwrap();
                let (inst, has_strong_ref) = lock.acknowledge_instruction(id);
                let log_msg = if let Some(inst) = inst {
                    LogMessage::new(
                        logger::severity::LogSeverity::Info,
                        format!(
                            "Received acknowledgement for ID {} ({}){}",
                            id,
                            inst.command,
                            if !has_strong_ref {
                                " (No strong refs remain)"
                            } else {
                                ""
                            }
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
                self.log(log_msg);
                drop(lock);
            }
            Message::Exiting => {
                self.log(LogMessage::new(LogSeverity::Info, "Client exiting..."));
            }
            Message::Log(dll_log_message) => {
                self.log(dll_log_message);
            }
            Message::DataMessage(data_message) => {
                return Ok(Some(data_message));
            }
        };
        Ok(None)
    }

    pub fn recv(&self) -> Result<DataMessage, IpcError> {
        loop {
            if let Some(message) = self.try_recv_one()? {
                return Ok(message);
            }
        }
    }
}
