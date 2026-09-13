//! Mount status detection via /proc/mounts and /proc/self/mountinfo.

use crate::error::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MountEntry {
    pub device_path: PathBuf,
    pub mount_point: PathBuf,
    pub fs_type: String,
    pub options: Vec<String>,
}

pub struct MountInspector;

impl MountInspector {
    /// Reads and parses all active mounts from `/proc/mounts`.
    pub fn list_mounts() -> Result<Vec<MountEntry>> {
        todo!("Phase 2: implement list_mounts")
    }

    /// Checks if a given device or any of its partition nodes is mounted.
    pub fn is_device_or_partition_mounted(_device_path: &Path) -> Result<bool> {
        todo!("Phase 2: implement is_device_or_partition_mounted")
    }
}
