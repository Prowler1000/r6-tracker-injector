mod signallable;
mod idlesignal;
use std::ops::Deref;

pub use idlesignal::IdleSignal;
pub use signallable::SignallableData;

pub trait Signal {
    fn is_signalled(&self) -> bool;
    fn wait_for_signal(&self) -> Result<(), SignalResult>;
    fn set_signal(&self, value: bool) -> bool;
}

#[derive(Debug)]
pub enum SignalResult {
    SignalOk(bool),
    SignalPoisoned(bool),
}

impl SignalResult {
    pub fn is_signalled(&self) -> bool {
        match self {
            SignalResult::SignalOk(val) => *val,
            SignalResult::SignalPoisoned(val) => *val,
        }
    }
}

impl From<SignalResult> for Result<bool, bool> {
    fn from(value: SignalResult) -> Self {
        match value {
            SignalResult::SignalOk(val) => Ok(val),
            SignalResult::SignalPoisoned(val) => Err(val),
        }
    }
}

impl From<SignalResult> for bool {
    fn from(value: SignalResult) -> Self {
        match value {
            SignalResult::SignalOk(val) => val,
            SignalResult::SignalPoisoned(val) => val,
        }
    }
}

impl Deref for SignalResult {
    type Target = bool;
    fn deref(&self) -> &Self::Target {
        match self {
            SignalResult::SignalOk(val) => val,
            SignalResult::SignalPoisoned(val) => val,
        }
    }
}