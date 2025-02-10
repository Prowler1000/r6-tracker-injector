use std::sync::{Arc, Condvar, Mutex};

use super::{Signal, SignalResult};


#[derive(Default, Debug)]
pub struct IdleSignal {
    state: Arc<Mutex<bool>>,
    signal: Condvar,
}

unsafe impl Send for IdleSignal {}
unsafe impl Sync for IdleSignal {}

impl IdleSignal {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Signal for IdleSignal {
    fn is_signalled(&self) -> bool {
        *self.state.lock().unwrap()
    }

    fn wait_for_signal(&self) -> Result<(), SignalResult> {
        let guard = self.state.lock().unwrap();
        let _unused = self.signal.wait_while(guard, |lock| !*lock).unwrap();
        drop(_unused);
        Ok(())
    }

    fn set_signal(&self, value: bool) -> bool {
        let mut lock = self.state.lock().unwrap();
        let old_val = *lock;
        *lock = value;
        drop(lock);
        self.signal.notify_all();
        old_val
    }
}