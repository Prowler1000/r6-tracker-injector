use std::fmt::Display;
use super::block::MemoryBlock;

#[derive(Clone, PartialEq)]
pub struct Region {
    pub first_valid_address: *const std::ffi::c_void,
    pub first_invalid_address: *const std::ffi::c_void,
    pub region_size: usize,
    pub blocks: Vec<MemoryBlock>,
}

impl Region {
    pub fn new(
        first_valid_address: *const std::ffi::c_void,
        first_invalid_address: *const std::ffi::c_void,
        region_size: usize,
    ) -> Self {
        assert!(
            first_invalid_address >= first_valid_address,
            "Invalid address range"
        );
        Self {
            first_valid_address,
            first_invalid_address,
            region_size,
            blocks: Vec::new(),
        }
    }

    pub fn get_first_addr(&self) -> *const std::ffi::c_void {
        self.first_valid_address
    }

    pub fn get_last_addr(&self) -> *const std::ffi::c_void {
        self.first_valid_address.wrapping_byte_add(self.region_size)
    }

    pub fn contains(&self, ptr: *const std::ffi::c_void) -> bool {
        ptr >= self.get_first_addr() && ptr < self.get_last_addr()
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:#?} - {:#?}",
            self.get_first_addr(),
            self.get_last_addr()
        )
    }
}

pub struct RegionInfo {
    pub first_valid_address: *const std::ffi::c_void,
    pub first_invalid_address: *const std::ffi::c_void,
    pub region_size: usize,
    pub num_blocks: usize,
}

impl RegionInfo {
    pub fn get_first_addr(&self) -> *const std::ffi::c_void {
        self.first_valid_address
    }
    pub fn get_last_addr(&self) -> *const std::ffi::c_void {
        self.first_valid_address.wrapping_byte_add(self.region_size)
    }
}

impl Display for RegionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:#?} - {:#?}",
            self.get_first_addr(),
            self.get_last_addr()
        )
    }
}

impl From<&Region> for RegionInfo {
    fn from(value: &Region) -> Self {
        Self {
            first_valid_address: value.first_valid_address,
            first_invalid_address: value.first_invalid_address,
            region_size: value.region_size,
            num_blocks: value.blocks.len(),
        }
    }
}

impl From<Region> for RegionInfo {
    fn from(value: Region) -> Self {
        Self::from(&value)
    }
}
