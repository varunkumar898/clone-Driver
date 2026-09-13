//! /sys/block querying and attribute extraction.

use crate::device::DeviceProperties;
use crate::error::Result;
use std::path::Path;

pub struct SysfsInspector;

impl SysfsInspector {
    /// Reads device size in sectors from `/sys/block/<name>/size`.
    pub fn read_size_sectors(_dev_name: &str) -> Result<u64> {
        todo!("Phase 2: implement read_size_sectors")
    }

    /// Reads device vendor/model/serial from sysfs.
    pub fn read_properties(_dev_name: &str) -> Result<DeviceProperties> {
        todo!("Phase 2: implement read_properties")
    }

    /// Checks if device is rotational (HDD) or non-rotational (SSD/NVMe).
    pub fn is_rotational(_dev_name: &str) -> Result<bool> {
        todo!("Phase 2: implement is_rotational")
    }

    /// Resolves canonical sysfs path for a device node.
    pub fn canonicalize_device_node(_node: &Path) -> Result<String> {
        todo!("Phase 2: implement canonicalize_device_node")
    }
}
