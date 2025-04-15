use std::ptr::addr_of_mut;

use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, VirtualQuery};

pub mod block;
pub mod heap;
pub mod region;
pub mod walk;
pub mod memwalker;

#[inline]
pub fn query(ptr: *const std::ffi::c_void) -> Result<MEMORY_BASIC_INFORMATION, windows::core::Error> {
    unsafe {
        let mut mbi: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
        let size = VirtualQuery(
            Some(ptr),
            addr_of_mut!(mbi),
            size_of::<MEMORY_BASIC_INFORMATION>(),
        );
        if size == size_of::<MEMORY_BASIC_INFORMATION>() {
            Ok(mbi)
        } else {
            Err(windows::core::Error::from_win32())
        }
    }
}
