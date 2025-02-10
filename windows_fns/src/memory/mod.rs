use std::ptr::addr_of_mut;

use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, VirtualQuery};

#[inline]
pub fn query(ptr: *const std::ffi::c_void) -> Option<MEMORY_BASIC_INFORMATION> {
    unsafe {
        let mut mbi: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
        let size = VirtualQuery(
            Some(ptr),
            addr_of_mut!(mbi),
            size_of::<MEMORY_BASIC_INFORMATION>(),
        );
        if size == size_of::<MEMORY_BASIC_INFORMATION>() {
            Some(mbi)
        } else {
            None
        }
    }
}
