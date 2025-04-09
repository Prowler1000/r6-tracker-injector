use windows::Win32::System::{Memory::{MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_READWRITE, PROCESS_HEAP_ENTRY}, SystemServices::PROCESS_HEAP_REGION};

use super::query;

pub struct Region {
    data_ptr: *const std::ffi::c_void,
    pub size: usize,
    pub mbi: MEMORY_BASIC_INFORMATION,
}

impl Region {
    pub fn new(data_ptr: *const std::ffi::c_void) -> Result<Self, windows::core::Error> {
        let mbi = query(data_ptr)?;
        Ok(Self {
            data_ptr,
            size: mbi.RegionSize,
            mbi,
        })
    }

    pub fn from_heap_entry(entry: PROCESS_HEAP_ENTRY) -> Result<Vec<Self>, windows::core::Error> {
        assert!(entry.wFlags as u32 & PROCESS_HEAP_REGION != 0);
        let mut next_block = unsafe { entry.Anonymous.Region.lpFirstBlock };
        let last_block = unsafe { entry.Anonymous.Region.lpLastBlock };
        let mut regions = Vec::new();
        while unsafe { next_block.offset_from(last_block) < 0 } {
            let region = Region::new(next_block)?;
            next_block = unsafe { next_block.add(region.size) };
            regions.push(region);
        }
        Ok(regions)
    }

    pub fn is_guard(&self) -> bool {
        self.mbi.Protect.contains(PAGE_GUARD)
    }

    pub fn is_readwrite(&self) -> bool {
        self.mbi.Protect.contains(PAGE_READWRITE)
    }

    // To protect from accidentally modifying the pointer, which would cause panics when calling align_to
    pub fn get_ptr(&self) -> *const std::ffi::c_void {
        self.data_ptr
    }

    pub fn align_to<'a, T>(&self) -> &'a[T] {
        let (_prefix, aligned, _suffix) = unsafe {
            std::slice::from_raw_parts(self.data_ptr, self.size).align_to::<T>()
        };
        aligned
    }
}