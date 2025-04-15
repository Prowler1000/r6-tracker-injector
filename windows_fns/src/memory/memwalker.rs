use std::{
    ops::{Range, RangeBounds},
    panic::{AssertUnwindSafe, UnwindSafe},
    ptr::addr_of_mut,
};

use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};

use super::{block::MemoryBlock, query, walk::WalkInfo};

pub struct MemoryWalker {
    sys_info: SYSTEM_INFO,
}

impl Default for MemoryWalker {
    fn default() -> Self {
        let mut sys_info = Default::default();
        unsafe {
            GetSystemInfo(addr_of_mut!(sys_info));
        }
        Self { sys_info }
    }
}

impl MemoryWalker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Walks through memory blocks and applies the provided function to each block that matches the criteria.
    ///
    /// # Parameters
    /// - `byte_range`: The range of byte sizes to filter memory blocks.
    /// - `f`: A closure that processes each memory block and returns a boolean indicating whether to continue.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw pointer operations and assumes that `f` is unwind safe
    pub unsafe fn walk_unsafe<F: FnMut(&[u8], MemoryBlock) -> bool>(
        &mut self,
        byte_range: impl RangeBounds<usize>,
        mut f: F,
    ) -> Result<(), windows::core::Error> {
        let mut addr = self.sys_info.lpMinimumApplicationAddress;
        let max_addr = self.sys_info.lpMaximumApplicationAddress;
        while addr < max_addr {
            let block = unsafe { MemoryBlock::from_raw_ptr(addr) }?;
            if byte_range.contains(&block.size)
                && block.is_commit()
                && block.is_readwrite()
                && !block.is_reserved()
                && !block.is_guard()
                && !std::panic::catch_unwind(AssertUnwindSafe(|| {
                    f(block.align_to::<u8>().unwrap(), block)
                }))
                .map_err(|_e| {
                    windows::core::Error::new(
                        windows::core::HRESULT(0x80004005u32 as i32), // E_FAIL
                        format!("Panic occurred while processing memory block {:#?} ({} bytes) [COMMIT: {} | RW: {} | RSV: {} | GUARD: {}]", addr, block.size, block.is_commit(), block.is_readwrite(), block.is_reserved(), block.is_guard()),
                    )
                })?
            {
                break;
            }
            addr = unsafe { addr.byte_add(block.size) };
        }
        Ok(())
    }

    pub fn walk<F: Fn(&[u8], MemoryBlock) -> bool + UnwindSafe>(
        &mut self,
        byte_range: impl RangeBounds<usize>,
        f: F,
    ) -> Result<(), windows::core::Error> {
        unsafe { self.walk_unsafe(byte_range, f) }
    }
}
