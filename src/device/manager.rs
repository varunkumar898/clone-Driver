//! System-wide block device discovery manager.

use super::device::BlockDevice;
use crate::error::Result;

/// Manages system hardware scans and aggregates block device metadata.
pub struct DeviceManager;

impl DeviceManager {
    /// Creates a new DeviceManager.
    pub fn new() -> Self {
        Self
    }

    /// Scans the host system via `/sys/block` and `udev` for all candidate block devices.
    pub fn scan_devices(&self) -> Result<Vec<BlockDevice>> {
        todo!("Phase 2: implement full device scan")
    }

    /// Finds a specific device by node path (e.g. `/dev/sdb`).
    pub fn find_by_path(&self, _path: &str) -> Result<Option<BlockDevice>> {
        todo!("Phase 2: implement find_by_path")
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}
