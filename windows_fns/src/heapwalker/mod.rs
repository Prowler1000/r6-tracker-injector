use thiserror::Error;
use windows::Win32::System::SystemServices::PROCESS_HEAP_ENTRY_BUSY;
use windows::Win32::System::{Memory::GetProcessHeaps, SystemServices::PROCESS_HEAP_REGION};
use windows::Win32::Foundation::HANDLE;
use windows::core::Error;

use crate::{block::MemoryBlock, heap::HeapWalkerInternal, region::Region, walk::WalkInfo};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(Error, Debug)]
pub enum HeapWalkerError {
    #[error("Failed to retrieve process heaps: {0}")]
    HeapRetrievalFailed(#[from] windows::core::Error),
}

pub struct HeapWalker {
    heaps: Box<[HANDLE]>,
    next_walk: usize,
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
            })
        }
    }

    /// Walks through each heap, locking it during iteration, and applies the provided function to each memory block.
    ///
    /// # Safety
    /// `func` must not unwind across FFI boundaries.
    pub unsafe fn walk_unsafe<T: FnMut(&[u8], Option<&Region>, &MemoryBlock)>(
        &mut self,
        mut func: T,
    ) -> Result<WalkInfo, Error> {
        let mut info = WalkInfo::default();

        while self.next_walk < self.heaps.len() {
            let heap_handle = self.heaps[self.next_walk];
            self.next_walk += 1;
            info.heaps_walked += 1;

            let mut walker = HeapWalkerInternal::new(heap_handle);
            let mut current_region: Option<Region> = None;

            while let Some(entry) = walker.next_entry()? {
                if entry.wFlags as u32 & PROCESS_HEAP_REGION != 0 {
                    if let Some(region) = current_region.take() {
                        info.regions_walked.push(region.clone().into());
                    }
                    let region_union = unsafe { entry.Anonymous.Region };
                    current_region = Some(Region::new(
                        region_union.lpFirstBlock,
                        region_union.lpLastBlock,
                        entry.cbData as usize,
                    ));
                } else if entry.wFlags as u32 & PROCESS_HEAP_ENTRY_BUSY != 0 {
                    if let Some(region) = current_region.as_mut() {
                        let block = MemoryBlock::from_heap_entry(&entry)?;
                        if region.contains(entry.lpData) {
                            if block.is_commit() && block.is_readwrite() && !block.is_reserved() && !block.is_guard() {
                                info.blocks_walked.push(block);
                                if let Some(data) = block.align_to::<u8>() {
                                    catch_unwind(AssertUnwindSafe(|| func(data, Some(region), &block)))
                                        .map_err(|_| Error::empty())?;
                                }
                                else {
                                    panic!("Failed to align block at address {:#?}", block.get_start_ptr());
                                }
                            } else {
                                info.blocks_skipped.push((block, Some(format!("(COMMIT: {} | RW: {} | RSRV: {} | GUARD: {})", block.is_commit(), block.is_readwrite(), block.is_reserved(), block.is_guard()))));
                            }
                        } else {
                            info.regions_walked.push(region.clone().into());
                            current_region = None;
                        }
                    } else {
                        let block = MemoryBlock::from_heap_entry(&entry)?;
                        info.stray_blocks.push(block);
                        if block.is_commit() && block.is_readwrite() && !block.is_reserved() && !block.is_guard() {
                            info.blocks_walked.push(block);
                            if let Some(data) = block.align_to::<u8>() {
                                catch_unwind(AssertUnwindSafe(|| func(data, None, &block)))
                                    .map_err(|_| Error::empty())?;
                            }
                            else {
                                panic!("Failed to align block at address {:#?}", block.get_start_ptr());
                            }
                        } else {
                            info.blocks_skipped.push((block, None));
                        }
                    }
                }
            }

            if let Some(region) = current_region.take() {
                info.regions_walked.push(region.into());
            }
        }

        Ok(info)
    }
}
