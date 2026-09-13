//! Partition table and entry representations.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionScheme {
    Gpt,
    Mbr,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionEntry {
    pub index: u32,
    pub start_lba: u64,
    pub end_lba: u64,
    pub size_bytes: u64,
    pub partition_type_guid: Option<String>,
    pub mbr_type_byte: Option<u8>,
    pub name: Option<String>,
    pub is_bootable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionTable {
    pub scheme: PartitionScheme,
    pub sector_size: u32,
    pub total_sectors: u64,
    pub partitions: Vec<PartitionEntry>,
}
