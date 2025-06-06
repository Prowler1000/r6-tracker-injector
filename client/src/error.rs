use thiserror::Error;
use thread_safe_utils::queue::ThreadSafeQueueError;

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("An IPC error occured. {0}")]
    Ipc(#[from] ipc_channel::ipc::IpcError),
    #[error("An IO error occured. {0}")]
    IO(#[from] std::io::Error),
    #[error("Failure while encoding. {0}")]
    Encode(#[from] Box<ipc_channel::ErrorKind>),
    #[error("Failed to send data into queue. Receive channel likely hung up")]
    SendError,
    #[error("Thread was finished")]
    ThreadFinished,
    #[error("Thread not running")]
    ThreadNotRunning,
    #[error("Ipc Queue Mutex was poisoned")]
    MutexPoisoned,
    #[error("Ipc Queue signal set")]
    Signalled,
    #[error("Named pipe was closed")]
    PipeClosed,
}

impl From<ThreadSafeQueueError> for IpcError {
    fn from(val: ThreadSafeQueueError) -> Self {
        match val {
            ThreadSafeQueueError::MutexPoison => IpcError::MutexPoisoned,
            ThreadSafeQueueError::StatusNotOk => IpcError::Signalled,
        }
    }
}
