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
    /// Formats human-readable capacity using power-of-1000 SI units.
    ///
    /// Examples: `"1.0 TB"`, `"500.1 GB"`, `"2.0 MB"`, `"512 B"`
    pub fn formatted_size(&self) -> String {
        const TB: u64 = 1_000_000_000_000;
        const GB: u64 = 1_000_000_000;
        const MB: u64 = 1_000_000;
        const KB: u64 = 1_000;

        let b = self.size_bytes;
        if b >= TB {
            format!("{:.1} TB", b as f64 / TB as f64)
        } else if b >= GB {
            format!("{:.1} GB", b as f64 / GB as f64)
        } else if b >= MB {
            format!("{:.1} MB", b as f64 / MB as f64)
        } else if b >= KB {
            format!("{:.1} KB", b as f64 / KB as f64)
        } else {
            format!("{b} B")
        }
    }

    /// Returns `true` if this device is eligible as a clone destination.
    ///
    /// A device is eligible when:
    /// - It is **not** the system (OS) disk
    /// - It is **not** currently mounted
    /// - Its capacity is **≥** the source device's size
    pub fn is_eligible_destination(&self, source_size_bytes: u64) -> bool {
        !self.is_system_disk && !self.is_mounted && self.size_bytes >= source_size_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_device(size_bytes: u64, is_system_disk: bool, is_mounted: bool) -> BlockDevice {
        BlockDevice {
            path: PathBuf::from("/dev/sdb"),
            sysfs_name: "sdb".to_string(),
            size_bytes,
            properties: DeviceProperties::default(),
            is_system_disk,
            is_mounted,
            partition_table: None,
        }
    }

    #[test]
    fn test_formatted_size_tb() {
        let d = make_device(2_000_000_000_000, false, false);
        assert_eq!(d.formatted_size(), "2.0 TB");
    }

    #[test]
    fn test_formatted_size_gb() {
        let d = make_device(500_100_000_000, false, false);
        assert_eq!(d.formatted_size(), "500.1 GB");
    }

    #[test]
    fn test_formatted_size_mb() {
        let d = make_device(2_097_152, false, false);
        assert_eq!(d.formatted_size(), "2.1 MB");
    }

    #[test]
    fn test_formatted_size_bytes() {
        let d = make_device(512, false, false);
        assert_eq!(d.formatted_size(), "512 B");
    }

    #[test]
    fn test_eligible_destination_ok() {
        let d = make_device(1_000_000_000_000, false, false);
        assert!(d.is_eligible_destination(500_000_000_000));
    }

    #[test]
    fn test_eligible_destination_same_size() {
        let d = make_device(1_000_000_000_000, false, false);
        assert!(d.is_eligible_destination(1_000_000_000_000));
    }

    #[test]
    fn test_not_eligible_system_disk() {
        let d = make_device(1_000_000_000_000, true, false);
        assert!(!d.is_eligible_destination(500_000_000_000));
    }

    #[test]
    fn test_not_eligible_mounted() {
        let d = make_device(1_000_000_000_000, false, true);
        assert!(!d.is_eligible_destination(500_000_000_000));
    }

    #[test]
    fn test_not_eligible_too_small() {
        let d = make_device(100_000_000_000, false, false);
        assert!(!d.is_eligible_destination(500_000_000_000));
    }
}
