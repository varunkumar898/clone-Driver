//! Safe block device file descriptor handle management.

use crate::error::Result;
use std::fs::File;
use std::path::Path;

/// Managed Linux block device file descriptor ensuring strict access modes.
pub struct LinuxBlockDeviceHandle {
    file: File,
    path: std::path::PathBuf,
    is_write: bool,
}

impl LinuxBlockDeviceHandle {
    /// Opens device strictly read-only for source cloning.
    pub fn open_read_only(_path: &Path) -> Result<Self> {
        todo!("Phase 2: implement open_read_only")
    }

    /// Opens device write-only with synchronous write flags for destination cloning.
    pub fn open_write_only(_path: &Path) -> Result<Self> {
        todo!("Phase 2: implement open_write_only")
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_write(&self) -> bool {
        self.is_write
    }
}
