use std::{
    fmt::Display,
    sync::{Arc, Weak},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use thread_safe_utils::signal::SignallableData;

pub type CommandID = usize;

#[derive(Serialize, Deserialize, Clone)]
pub struct Instruction {
    pub id: CommandID,
    pub command: Command,
}

impl From<&PendingInstruction> for Instruction {
    fn from(value: &PendingInstruction) -> Self {
        Self {
            id: value.id,
            command: value.variant.clone(),
        }
    }
}

impl Instruction {
    pub fn new(
        id: CommandID,
        command: Command,
    ) -> (Instruction, PendingInstruction, Arc<PendingCommand>) {
        let (pend_inst, pend_cmd) = PendingInstruction::new(id, command.clone());
        (Self { id, command }, pend_inst, pend_cmd)
    }
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

#[derive(Debug, Default)]
struct PendingCommandInternal {
    started: bool,
    completed: bool,
    data: Option<Box<dyn std::any::Any + Send>>,
}

#[derive(Default)]
pub struct PendingCommand(SignallableData<PendingCommandInternal>);

#[derive(Error, Debug)]
pub enum PendingCommandError {
    #[error("The command was signalled")]
    Signalled,
    #[error("More than 1 strong reference")]
    StrongReference,
    #[error("The internal mutex was poisoned")]
    Poisoned,
}

impl PendingCommand {
    pub fn wait_for_start(&self) {
        let _ = self
            .0
            .lock_wait_while(|cmd, signal| !cmd.started && !*signal);
    }

    pub fn wait_for_complete(
        arc_self: Arc<Self>,
    ) -> Result<Option<Box<dyn std::any::Any + Send>>, PendingCommandError> {
        let lock = arc_self
            .0
            .lock_wait_while(|cmd, signal| !cmd.completed && !*signal)
            .map_err(|_| PendingCommandError::Poisoned)?;
        if lock.is_signalled() {
            return Err(PendingCommandError::Signalled);
        }
        drop(lock);
        if let Some(inner) = Arc::into_inner(arc_self) {
            let command = inner.0;
            let (cmd, _) = command.into_inner().unwrap();
            Ok(cmd.data)
        } else {
            Err(PendingCommandError::StrongReference)
        }
    }
}

pub struct PendingInstruction {
    pub id: CommandID,
    pub variant: Command,
    data: Weak<PendingCommand>,
}

impl PendingInstruction {
    pub fn new(id: CommandID, variant: Command) -> (Self, Arc<PendingCommand>) {
        let cmd = Arc::new(PendingCommand::default());
        let data = Arc::downgrade(&cmd);
        (Self { id, variant, data }, cmd)
    }

    // Returns true if the value hasn't been dropped
    pub fn mark_started(&self) -> bool {
        if let Some(cmd) = self.data.upgrade() {
            if let Ok(mut internal) = cmd.0.lock() {
                internal.started = true;
            }
            true
        } else {
            false
        }
    }

    // Returns true if the value hasn't been dropped
    pub fn mark_completed(&self, data: Box<dyn std::any::Any + Send>) -> bool {
        if let Some(cmd) = self.data.upgrade() {
            if let Ok(mut internal) = cmd.0.lock() {
                internal.completed = true;
                internal.data = Some(data);
            }
            true
        } else {
            false
        }
    }
}
