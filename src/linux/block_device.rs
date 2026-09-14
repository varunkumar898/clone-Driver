//! Safe block device file descriptor handle management.

use crate::error::{DeviceError, Result};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

/// Managed Linux block device file descriptor ensuring strict access modes.
pub struct LinuxBlockDeviceHandle {
    file: File,
    path: PathBuf,
    is_write: bool,
}

impl LinuxBlockDeviceHandle {
    /// Opens a block device strictly read-only (suitable for source cloning).
    ///
    /// Uses `O_RDONLY` with `O_NONBLOCK` to avoid blocking on devices like
    /// optical drives that might not have media inserted.
    pub fn open_read_only(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
            .map_err(|e| {
                DeviceError::InvalidPath(format!("Cannot open {} read-only: {e}", path.display()))
            })?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            is_write: false,
        })
    }

    /// Opens a block device write-only with synchronous write flags for destination
    /// cloning.
    ///
    /// Uses `O_WRONLY | O_SYNC` to guarantee that every write is committed to the
    /// physical medium before returning, which is critical for data integrity during
    /// a clone operation.
    pub fn open_write_only(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open(path)
            .map_err(|e| {
                DeviceError::InvalidPath(format!("Cannot open {} write-only: {e}", path.display()))
            })?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            is_write: true,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Tier-1: opening a regular temp file read-only should succeed
    #[test]
    fn test_open_read_only_regular_file() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let handle = LinuxBlockDeviceHandle::open_read_only(f.path());
        assert!(handle.is_ok());
        let h = handle.unwrap();
        assert!(!h.is_write());
        assert_eq!(h.path(), f.path());
    }

    /// Tier-1: opening a regular temp file write-only should succeed
    #[test]
    fn test_open_write_only_regular_file() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let handle = LinuxBlockDeviceHandle::open_write_only(f.path());
        assert!(handle.is_ok());
        assert!(handle.unwrap().is_write());
    }

    /// Tier-1: opening a non-existent path returns an error
    #[test]
    fn test_open_read_only_missing_returns_error() {
        let result = LinuxBlockDeviceHandle::open_read_only(Path::new("/nonexistent/device/xyz"));
        assert!(result.is_err());
    }
}
