//! System disk protection rules.

use crate::device::BlockDevice;
use crate::error::{Result, SafetyError};

pub struct SystemDiskProtection;

impl SystemDiskProtection {
    /// Enforces that the destination is not the host OS disk.
    pub fn assert_not_system_disk(destination: &BlockDevice) -> Result<()> {
        if destination.is_system_disk {
            Err(SafetyError::DestinationIsSystemDisk(destination.path.display().to_string()).into())
        } else {
            Ok(())
        }
    }
}
