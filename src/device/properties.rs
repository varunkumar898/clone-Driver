//! Hardware properties of block devices.

use serde::{Deserialize, Serialize};

/// Detailed hardware and operational properties of a block device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProperties {
    pub vendor: String,
    pub model: String,
    pub serial_number: String,
    pub wwn: Option<String>,
    pub logical_block_size: u32,
    pub physical_block_size: u32,
    pub is_rotational: bool,
    pub is_read_only: bool,
    pub is_removable: bool,
}

impl Default for DeviceProperties {
    fn default() -> Self {
        Self {
            vendor: String::new(),
            model: String::new(),
            serial_number: String::new(),
            wwn: None,
            logical_block_size: 512,
            physical_block_size: 512,
            is_rotational: false,
            is_read_only: false,
            is_removable: false,
        }
    }
}
