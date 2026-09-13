//! libudev discovery and property extraction.

use crate::device::BlockDevice;
use crate::error::Result;

pub struct UdevScanner;

impl UdevScanner {
    /// Queries udev for all disk subsystem block devices.
    pub fn scan_disks() -> Result<Vec<BlockDevice>> {
        todo!("Phase 2: implement udev disk enumeration")
    }
}
