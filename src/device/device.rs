//! High-level block device representation.

use super::properties::DeviceProperties;
use crate::partition::table::PartitionTable;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// High-level representation of a physical block device (e.g. `/dev/sda`, `/dev/nvme0n1`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDevice {
    pub path: PathBuf,
    pub sysfs_name: String,
    pub size_bytes: u64,
    pub properties: DeviceProperties,
    pub is_system_disk: bool,
    pub is_mounted: bool,
    pub partition_table: Option<PartitionTable>,
}

impl BlockDevice {
    /// Formats human-readable capacity (e.g. "500.1 GB").
    pub fn formatted_size(&self) -> String {
        todo!("Phase 2: implement formatted_size")
    }

    /// Checks if device is eligible as a cloning destination.
    pub fn is_eligible_destination(&self, _source_size_bytes: u64) -> bool {
        todo!("Phase 2: implement is_eligible_destination")
    }
}
