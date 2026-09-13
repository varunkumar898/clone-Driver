//! Clone recovery coordinator.

use crate::device::DeviceManager;
use crate::error::Result;
use std::path::Path;

pub struct RecoveryCoordinator;

impl RecoveryCoordinator {
    /// Validates hardware devices against the journal and calculates starting offset.
    pub fn prepare_recovery(_journal_path: &Path, _device_manager: &DeviceManager) -> Result<u64> {
        todo!("Phase 3: implement prepare_recovery")
    }
}
