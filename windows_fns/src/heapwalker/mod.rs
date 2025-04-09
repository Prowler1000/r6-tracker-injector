use std::{
    panic::{AssertUnwindSafe, UnwindSafe},
    ptr::{addr_of_mut, null_mut},
};

use thiserror::Error;
use windows::{
    Win32::{
        Foundation::{ERROR_NO_MORE_ITEMS, HANDLE},
        System::{
            Memory::{
                GetProcessHeaps, HeapLock, HeapUnlock, HeapWalk, MEM_COMMIT,
                MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_READWRITE, PROCESS_HEAP_ENTRY,
                VirtualQuery,
            },
            SystemServices::PROCESS_HEAP_REGION,
        },
    },
    core::Error,
};

use crate::{heap::Heap, memory::query};

#[derive(Error, Debug)]
pub enum HeapWalkerError {
    #[error("Failed to retrieve process heaps with error : {0}")]
    HeapRetrievalFailed(Error),
}

pub struct HeapWalker {
    heaps: Box<[HANDLE]>,
    next_walk: usize,
    entry: PROCESS_HEAP_ENTRY,
}

impl HeapWalker {
    pub fn new() -> Result<Self, HeapWalkerError> {
        unsafe {
            let mut empty_handles: [HANDLE; 0] = std::mem::zeroed();
            let num_heaps = GetProcessHeaps(&mut empty_handles);
            if num_heaps == 0 {
                return Err(HeapWalkerError::HeapRetrievalFailed(Error::from_win32()));
            }
            let mut heaps = vec![HANDLE::default(); num_heaps as usize].into_boxed_slice();
            let new_num_heaps = GetProcessHeaps(&mut heaps);
            if new_num_heaps == 0 {
                return Err(HeapWalkerError::HeapRetrievalFailed(Error::from_win32()));
            }
            let heaps = if new_num_heaps < num_heaps {
                heaps[..new_num_heaps as usize].to_vec().into_boxed_slice()
            } else {
                heaps
            };
            Ok(Self {
                heaps,
                next_walk: 0,
                entry: Default::default(),
            })
        }
    }

    pub fn walk<T: FnMut(&[u8]) + UnwindSafe>(
        &mut self,
        mut func: T,
    ) -> Result<(), windows::core::Error> {
        while self.next_walk < self.heaps.len() {
            let heap_handle = self.heaps[self.next_walk];
            let heap = Heap::new(heap_handle);
            let mut iter = heap.create_commit_iter()?;
            while let Some(regions) = iter.next_region()? {
                for region in regions {
                    if region.is_readwrite() && !region.is_guard() {
                        let arr = region.align_to::<u8>();
                        std::panic::catch_unwind(AssertUnwindSafe(|| func(arr))).map_err(|_| windows::core::Error::from_win32())?;
                    }
                }
            }
            self.next_walk += 1;
        }
        Ok(())
    }
}

fn heap_walk(
    heap_handle: HANDLE,
    entry: &mut PROCESS_HEAP_ENTRY,
) -> Result<Option<usize>, windows::core::Error> {
    unsafe {
        while next_heap_entry(heap_handle, entry)? {
            if let Ok(mbi) = query(entry.lpData) {
                if mbi.State == MEM_COMMIT
                    && mbi.Protect.contains(PAGE_READWRITE)
                    && !mbi.Protect.contains(PAGE_GUARD)
                {
                    // I don't know enough about memory pages and heaps (I could literally just look it up though...)
                    // to know whether or not a heap can contain more than one page, or whether a page can contain
                    // more than one heap
                    let difference = entry.lpData.byte_offset_from(mbi.BaseAddress);
                    return Ok(Some((mbi.RegionSize as isize - difference) as usize));
                }
            } else {
                return Err(windows::core::Error::from_win32());
            }
        }
        Ok(None)
    }
}

fn next_heap_entry(
    heap_handle: HANDLE,
    entry: &mut PROCESS_HEAP_ENTRY,
) -> Result<bool, windows::core::Error> {
    unsafe {
        HeapWalk(heap_handle, entry as *mut _)
            .map(|_| true)
            .or_else(|e| {
                if e == ERROR_NO_MORE_ITEMS.into() {
                    Ok(false)
                } else {
                    Err(e)
                }
            })
    }
}
