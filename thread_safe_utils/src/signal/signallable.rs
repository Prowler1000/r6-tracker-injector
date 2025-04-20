use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError},
    time::Duration,
};

use super::{Signal, SignalResult};

#[derive(Default, Debug)]
struct DataSignalPair<T> {
    data: T,
    signal: bool,
}

pub struct SignallableLock<'sd, T> {
    guard: MutexGuard<'sd, DataSignalPair<T>>,
    source: &'sd SignallableData<T>,
}

impl<'sd, T> SignallableLock<'sd, T> {
    pub fn is_signalled(&self) -> bool {
        self.guard.signal
    }
}

impl<'sd, T> Drop for SignallableLock<'sd, T> {
    fn drop(&mut self) {
        self.source.condvar.notify_all();
    }
}

impl<'sd, T> Deref for SignallableLock<'sd, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.guard.data
    }
}

impl<'sd, T> DerefMut for SignallableLock<'sd, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard.data
    }
}

impl<T> From<PoisonError<MutexGuard<'_, DataSignalPair<T>>>> for SignalResult {
    fn from(value: PoisonError<MutexGuard<'_, DataSignalPair<T>>>) -> Self {
        Self::SignalPoisoned(value.get_ref().signal)
    }
}

#[derive(Default)]
pub struct SignallableData<T> {
    data: Mutex<DataSignalPair<T>>,
    condvar: Condvar,
}

unsafe impl<T> Send for SignallableData<T> {}
unsafe impl<T> Sync for SignallableData<T> {}

impl<T> SignallableData<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: Mutex::new(DataSignalPair {
                data,
                signal: false,
            }),
            condvar: Default::default(),
        }
    }

    pub fn into_inner(self) -> Result<(T, bool), (T, bool)> {
        match self.data.into_inner() {
            Ok(data) => Ok((data.data, data.signal)),
            Err(poison) => {
                let data = poison.into_inner();
                Err((data.data, data.signal))
            },
        }
    }

    fn get_guard(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, DataSignalPair<T>>,
        std::sync::PoisonError<std::sync::MutexGuard<'_, DataSignalPair<T>>>,
    > {
        self.data.lock()
    }
}

impl<'sd, T> SignallableData<T> {
    pub fn lock(&'sd self) -> Result<SignallableLock<'sd, T>, SignalResult> {
        let guard = self.get_guard()?;
        Ok(self.create_lock_thingy_idk(guard))
    }

    pub fn try_lock(&'sd self) -> Result<Option<SignallableLock<'sd, T>>, SignalResult> {
        match self.data.try_lock() {
            Ok(lock) => Ok(Some(self.create_lock_thingy_idk(lock))),
            Err(e) => match e {
                std::sync::TryLockError::Poisoned(poison_error) => {
                    Err(SignalResult::SignalPoisoned(poison_error.get_ref().signal))
                }
                std::sync::TryLockError::WouldBlock => Ok(None),
            },
        }
    }

    fn create_lock_thingy_idk(
        &'sd self,
        guard: MutexGuard<'sd, DataSignalPair<T>>,
    ) -> SignallableLock<'sd, T> {
        SignallableLock {
            guard,
            source: self,
        }
    }

    pub fn lock_wait_while<F: FnMut(&T, &bool) -> bool>(
        &'sd self,
        mut condition: F,
    ) -> Result<SignallableLock<'sd, T>, SignalResult> {
        let guard = self.get_guard()?;
        let lock = self
            .condvar
            .wait_while(guard, |pair| condition(&pair.data, &pair.signal))
            .unwrap();
        Ok(self.create_lock_thingy_idk(lock))
    }

    pub fn lock_wait_while_timeout<F: FnMut(&T, &bool) -> bool>(
        &'sd self,
        dur: Duration,
        mut condition: F,
    ) -> Result<Option<SignallableLock<'sd, T>>, SignalResult> {
        let guard = self.get_guard()?;
        self.condvar
            .wait_timeout_while(guard, dur, |lock| condition(&lock.data, &lock.signal))
            .map(|(lock, to)| {
                if to.timed_out() {
                    None
                } else {
                    Some(self.create_lock_thingy_idk(lock))
                }
            })
            .map_err(|e| SignalResult::SignalPoisoned(e.get_ref().0.signal))
    }

    pub fn lock_wait_for_signal(&'sd self) -> Result<SignallableLock<'sd, T>, SignalResult> {
        let guard = self.get_guard()?;
        let guard = self.condvar.wait_while(guard, |g| !g.signal)?;
        Ok(self.create_lock_thingy_idk(guard))
    }

    /// # Safety
    /// This function accesses the data regardless of whether or not its poisoned.
    /// The caller must ensure that using the poisoned data will not cause undefined behavior.
    pub unsafe fn lock_ignore_poison(&'sd self) -> SignallableLock<'sd, T> {
        match self.data.lock() {
            Ok(guard) => self.create_lock_thingy_idk(guard),
            Err(poison) => self.create_lock_thingy_idk(poison.into_inner()),
        }
    }
}

impl<T> Signal for SignallableData<T> {
    fn is_signalled(&self) -> bool {
        match self.get_guard() {
            Ok(guard) => guard.signal,
            Err(poison) => poison.get_ref().signal,
        }
    }

    fn wait_for_signal(&self) -> Result<(), SignalResult> {
        let guard = self.get_guard()?;
        let _unused = self.condvar.wait_while(guard, |l| !l.signal)?;
        Ok(())
    }

    fn set_signal(&self, value: bool) -> bool {
        let mut guard = match self.get_guard() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let old = guard.signal;
        guard.signal = value;
        drop(guard);
        self.condvar.notify_all();
        old
    }
}
