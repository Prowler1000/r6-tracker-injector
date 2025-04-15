use windows::Win32::System::Memory::*;
use windows::Win32::System::Diagnostics::Debug::*;
use windows::Win32::System::SystemServices::{PROCESS_HEAP_ENTRY_BUSY, PROCESS_HEAP_REGION};
use windows::Win32::System::Threading::*;
use windows::Win32::Foundation::*;
use windows::core::Error;
use crate::region::{Region, RegionInfo};
use crate::block::MemoryBlock;
use crate::walk::WalkInfo;
use std::panic::AssertUnwindSafe;

pub struct Heap {
    handle: HANDLE,
}

impl Heap {
    pub fn new(handle: HANDLE) -> Self {
        Self { handle }
    }
}

pub struct HeapWalkerInternal {
    handle: HANDLE,
    entry: Option<PROCESS_HEAP_ENTRY>,
}

impl Drop for HeapWalkerInternal {
    fn drop(&mut self) {
        if self.entry.is_some() {
            unsafe {
                let _ = HeapUnlock(self.handle);
            }
        }
    }
}

impl HeapWalkerInternal {
    pub fn new(handle: HANDLE) -> Self {
        Self {
            handle,
            entry: None,
        }
    }

    pub fn next_entry(&mut self) -> Result<Option<PROCESS_HEAP_ENTRY>, Error> {
        let entry = if let Some(entry) = self.entry.as_mut() {
            entry
        } else {
            unsafe {
                HeapLock(self.handle)?;
                self.entry = Some(std::mem::zeroed());
                self.entry.as_mut().unwrap()
            }
        };

        match unsafe { HeapWalk(self.handle, entry) } {
            Ok(_) => Ok(Some(*entry)),
            Err(e) if e.code() == ERROR_NO_MORE_ITEMS.to_hresult() => Ok(None),
            Err(e) => Err(e),
        }
    }
}
