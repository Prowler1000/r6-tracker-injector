use std::{fmt::Display, ops::RangeBounds};
use windows::Win32::System::Memory::{
    MEMORY_BASIC_INFORMATION, MEM_COMMIT, MEM_RESERVE, PAGE_GUARD, PAGE_READWRITE, PROCESS_HEAP_ENTRY,
};

use crate::memory::query;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MemoryBlock {
    data_ptr: *const std::ffi::c_void,
    pub size: usize,
    pub mbi: MEMORY_BASIC_INFORMATION,
}

impl MemoryBlock {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(data_ptr: *const std::ffi::c_void, size: usize) -> Result<Self, windows::core::Error> {
        let mbi = query(data_ptr)?;
        let offset = unsafe { data_ptr.byte_offset_from(mbi.BaseAddress) };
        assert!(offset >= 0);
        Ok(Self {
            data_ptr,
            size,
            mbi,
        })
    }

    pub unsafe fn from_raw_ptr(ptr: *const std::ffi::c_void) -> Result<Self, windows::core::Error> {
        let mbi = query(ptr)?;
        Ok(Self {
            data_ptr: ptr,
            size: mbi.RegionSize,
            mbi,
        })
    }

    pub fn from_heap_entry(entry: &PROCESS_HEAP_ENTRY) -> Result<Self, windows::core::Error> {
        assert!(!entry.lpData.is_null());
        let data_ptr = entry.lpData;
        let size = entry.cbData as usize;
        let mbi = query(data_ptr)?;

        if mbi.State != MEM_COMMIT || mbi.Protect.contains(PAGE_GUARD) {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(0x80070005u32 as i32),
                "Block is not committed or is guarded",
            ));
        }

        let mbi_range_end = unsafe { mbi.BaseAddress.byte_add(mbi.RegionSize) };
        let block_end = unsafe { data_ptr.byte_add(size) };
        if block_end > mbi_range_end {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(0x80070005u32 as i32),
                "Heap block size exceeds committed memory bounds",
            ));
        }

        Ok(Self {
            data_ptr,
            size,
            mbi,
        })
    }

    pub fn is_reserved(&self) -> bool {
        self.mbi.State.contains(MEM_RESERVE)
    }

    pub fn is_commit(&self) -> bool {
        self.mbi.State.contains(MEM_COMMIT)
    }

    pub fn is_guard(&self) -> bool {
        self.mbi.Protect.contains(PAGE_GUARD)
    }

    pub fn is_readwrite(&self) -> bool {
        self.mbi.Protect.contains(PAGE_READWRITE)
    }

    pub fn is_accessible(&self) -> bool {
        self.is_readwrite() && self.is_commit() && !self.is_guard() && !self.is_reserved()
    }

    pub fn try_copy_range(&self, range: impl RangeBounds<usize>) -> Option<Vec<u8>> {
        let start_ind = match range.start_bound() {
            std::ops::Bound::Included(ind) => *ind,
            std::ops::Bound::Excluded(ind) => ind+1,
            std::ops::Bound::Unbounded => 0,
        };
        let end_ind = match range.end_bound() {
            std::ops::Bound::Included(ind) => *ind,
            std::ops::Bound::Excluded(ind) => *ind-1,
            std::ops::Bound::Unbounded => self.size - 1,
        };
        if start_ind < self.size && end_ind < self.size {
            if self.is_accessible() {
                let mut copy = vec![0; end_ind - start_ind + 1];
                if !self.intersects_slice(&copy) {
                    copy.copy_from_slice(&self.get_byte_slice()[start_ind..=end_ind]);
                    Some(copy)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_byte_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.data_ptr as _, self.size) }
    }

    pub fn get_start_ptr(&self) -> *const std::ffi::c_void {
        self.data_ptr
    }

    pub fn get_end_ptr(&self) -> *const std::ffi::c_void {
        unsafe { self.data_ptr.byte_add(self.size.saturating_sub(1)) }
    }

    pub fn align_to<T>(&self) -> Option<&[T]> {
        let offset = self.data_ptr.align_offset(std::mem::align_of::<T>());
        if offset == usize::MAX {
            return None;
        }
        let size = self.size.saturating_sub(offset);
        let aligned_ptr = self.data_ptr.wrapping_add(offset);
        if unsafe { aligned_ptr.offset_from(self.data_ptr) } < 0 {
            return None;
        }
        let slice = unsafe {
            std::slice::from_raw_parts(aligned_ptr as *const T, size / std::mem::size_of::<T>())
        };
        Some(slice)
    }

    pub fn align_to_vec<T: Clone>(&self) -> Option<Vec<T>> {
        let offset = self.data_ptr.align_offset(std::mem::align_of::<T>());
        if offset == usize::MAX {
            return None;
        }
        let size = self.size.saturating_sub(offset);
        let aligned_ptr = self.data_ptr.wrapping_add(offset);
        if unsafe { aligned_ptr.offset_from(self.data_ptr) } < 0 {
            return None;
        }
        let slice = unsafe {
            std::slice::from_raw_parts(aligned_ptr as *const T, size / std::mem::size_of::<T>())
        };
        Some(slice.to_vec())
    }

    pub fn contains(&self, ptr: *const std::ffi::c_void) -> bool {
        (self.get_start_ptr().addr()..=self.get_end_ptr().addr()).contains(&ptr.addr())
    }

    pub fn intersects_slice<T>(&self, slice: &[T]) -> bool {
        let start = slice.as_ptr();
        self.get_start_ptr() as *const T <= start && start <= self.get_end_ptr() as *const T
    }
}

impl Display for MemoryBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?} - {:#?}", self.get_start_ptr(), self.get_end_ptr())
    }
}
