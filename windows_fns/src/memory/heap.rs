use windows::Win32::{Foundation::{ERROR_NO_MORE_ITEMS, HANDLE}, System::{Memory::{HeapLock, HeapUnlock, HeapWalk, PROCESS_HEAP_ENTRY}, SystemServices::PROCESS_HEAP_REGION}};

use super::region::Region;


pub struct Heap {
    handle: HANDLE,
}

impl Heap {
    pub fn new(handle: HANDLE) -> Self {
        Self {
            handle
        }
    }

    pub fn create_commit_iter(self) -> Result<CommitedIter, windows::core::Error> {
        CommitedIter::new(self.handle)
    }
}

pub struct CommitedIter {
    handle: HANDLE,
    entry: Option<PROCESS_HEAP_ENTRY>,
}

impl Drop for CommitedIter {
    fn drop(&mut self) {
        if self.entry.is_some() {
            let _ = unsafe { HeapUnlock(self.handle) };
        }
    }
}

impl CommitedIter {
    fn new(handle: HANDLE) -> Result<Self, windows::core::Error> {
        Ok(
            Self {
                handle,
                entry: None,
            }
        )
    }

    pub fn next_region(&mut self) -> Result<Option<Vec<Region>>, windows::core::Error> {
        let entry = if let Some(entry) = self.entry.as_mut() {
            entry
        } else {
            unsafe { HeapLock(self.handle)? }
            self.entry = Some(unsafe { std::mem::zeroed() });
            self.entry.as_mut().unwrap()
        };

        loop {
            match unsafe { HeapWalk(self.handle, entry) } { 
                Ok(_) => {
                    if entry.wFlags as u32 & PROCESS_HEAP_REGION != 0 {
                        return Ok(Some(Region::from_heap_entry(*entry)?));
                    }
                },
                Err(e) => {
                    if e == ERROR_NO_MORE_ITEMS.into() {
                        return Ok(None);
                    } else {
                        return Err(e);
                    }
                },
            }
        }
    }
}