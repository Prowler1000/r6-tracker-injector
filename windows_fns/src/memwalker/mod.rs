use std::{panic::{AssertUnwindSafe, UnwindSafe}, ptr::addr_of_mut};

use thiserror::Error;
use windows::Win32::{Foundation::ERROR_PARTIAL_COPY, System::{
    Diagnostics::Debug::ReadProcessMemory,
    Memory::{MEM_COMMIT, PAGE_GUARD, PAGE_READWRITE},
    SystemInformation::{GetSystemInfo, SYSTEM_INFO},
    Threading::GetCurrentProcess,
}};

use crate::memory::query;

#[derive(Error, Debug)]
pub enum MemWalkerError {
    #[error("Failed to get Memory Basic Information for address {:#?}", _0)]
    InvalidMBI(*const std::ffi::c_void),
    #[error("Windows function call failed with error {} : {}", _0.code(), _0.message())]
    WindowsError(#[from] windows::core::Error),
    #[error("An unwind panic occurred")]
    PanicUnwind(Box<dyn std::any::Any + Send>)
}

pub struct MemoryWalker {
    sys_info: SYSTEM_INFO,
}

impl Default for MemoryWalker {
    fn default() -> Self {
        let mut sys_info = Default::default();
        unsafe { GetSystemInfo(addr_of_mut!(sys_info)) };
        Self { sys_info }
    }
}

#[derive(Debug)]
pub enum MemoryReadAction {
    /// Skip this region entirely, continue scan
    Skip,
    /// Use the `bytes_read` portion of the data
    UsePartial,
    /// Fail the walk and return the error
    Fail,
}


impl MemoryWalker {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn walk_slice<T: FnMut(&[u8], *const std::ffi::c_void)>(
        &self,
        mut func: T,
    ) -> Result<(), MemWalkerError> {
        let mut addr = self.sys_info.lpMinimumApplicationAddress;
        let max_addr = self.sys_info.lpMaximumApplicationAddress;
        while addr < max_addr {
            if let Ok(mbi) = query(addr) {
                if (mbi.State == MEM_COMMIT)
                    && mbi.Protect.contains(PAGE_READWRITE)
                    && !mbi.Protect.contains(PAGE_GUARD)
                {
                    unsafe {
                        let arr = std::slice::from_raw_parts(
                            mbi.BaseAddress as *const u8,
                            mbi.RegionSize,
                        );
                        func(arr, addr);
                    }
                }
                unsafe {
                    addr = addr.byte_add(mbi.RegionSize);
                }
            } else {
                return Err(MemWalkerError::InvalidMBI(addr));
            }
        }
        Ok(())
    }

    pub fn walk_owned<T: FnMut(Vec<u8>, *const std::ffi::c_void) + UnwindSafe>(
        &self,
        partial_read_action: MemoryReadAction,
        mut func: T,
    ) -> Result<(), MemWalkerError> {
        std::panic::catch_unwind(AssertUnwindSafe(|| {
            let mut addr = self.sys_info.lpMinimumApplicationAddress;
            let max_addr = self.sys_info.lpMaximumApplicationAddress;
            while addr < max_addr {
            if let Ok(mbi) = query(addr) {
                if (mbi.State == MEM_COMMIT) && mbi.Protect.contains(PAGE_READWRITE) && !mbi.Protect.contains(PAGE_GUARD) {
                    let mut data = vec![0u8; mbi.RegionSize];
                    let mut bytes_read = 0_usize;
                    unsafe {
                        match ReadProcessMemory(
                            GetCurrentProcess(),
                            addr,
                            data.as_mut_ptr() as *mut _,
                            mbi.RegionSize,
                            Some(addr_of_mut!(bytes_read)),
                        ) {
                            Ok(_) => {
                                func(data, addr);
                            },
                            Err(e) => {
                                if e.code() == ERROR_PARTIAL_COPY.into() {
                                    match partial_read_action {
                                        MemoryReadAction::Skip => {},
                                        MemoryReadAction::UsePartial => {
                                            if bytes_read > 0 {
                                                func(data[..bytes_read].to_vec(), addr);
                                            }
                                        },
                                        MemoryReadAction::Fail => return Err(e.into()),
                                    }
                                } else {
                                    return Err(e.into())
                                }
                            },
                        }
                    }
                }
                addr = unsafe { addr.byte_add(mbi.RegionSize) }
            }
            else {
                return Err(MemWalkerError::InvalidMBI(addr));
            }
        }
        Ok(())
        })).map_err(MemWalkerError::PanicUnwind)?
    }
}
