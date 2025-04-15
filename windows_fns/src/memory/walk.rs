use super::{block::MemoryBlock, region::RegionInfo};

#[derive(Default)]
pub struct WalkInfo {
    pub heaps_walked: usize,
    pub bytes_total: usize,
    pub regions_walked: Vec<RegionInfo>,
    pub blocks_walked: Vec<MemoryBlock>,
    pub blocks_skipped: Vec<(MemoryBlock, Option<String>)>,
    pub stray_blocks: Vec<MemoryBlock>,
}