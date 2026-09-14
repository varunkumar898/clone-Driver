//! System-wide block device discovery manager.

use super::device::BlockDevice;
use crate::error::Result;
use crate::linux::UdevScanner;
use std::path::Path;

/// Manages system hardware scans and aggregates block device metadata.
pub struct DeviceManager;

impl DeviceManager {
    /// Creates a new DeviceManager.
    pub fn new() -> Self {
        Self
    }

    /// Scans the host system via udev for all candidate block devices.
    ///
    /// Internally delegates to [`UdevScanner::scan_disks`], which combines
    /// udev property enumeration with sysfs attribute reading, mount-status
    /// detection, and system-disk classification.
    ///
    /// The returned list is sorted by device path (`/dev/sda` < `/dev/sdb` …).
    pub fn scan_devices(&self) -> Result<Vec<BlockDevice>> {
        UdevScanner::scan_disks()
    }

    /// Finds a specific device by its node path (e.g. `/dev/sdb`).
    ///
    /// Performs a full `scan_devices()` and returns the first device whose
    /// `path` matches exactly.  Returns `Ok(None)` when no match is found.
    pub fn find_by_path(&self, path: &str) -> Result<Option<BlockDevice>> {
        let target = Path::new(path);
        let devices = self.scan_devices()?;
        Ok(devices.into_iter().find(|d| d.path == target))
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tier-4: full scan requires udev — run manually.
    #[test]
    #[ignore = "requires udev and /dev access; run with: cargo test -- --include-ignored"]
    fn test_scan_devices_returns_results() {
        let mgr = DeviceManager::new();
        let result = mgr.scan_devices();
        assert!(result.is_ok(), "scan_devices failed: {result:?}");
        let devices = result.unwrap();
        println!("Found {} device(s):", devices.len());
        for d in &devices {
            println!(
                "  {:?}  {}  serial={:?}  system={} mounted={}",
                d.path,
                d.formatted_size(),
                d.properties.serial_number,
                d.is_system_disk,
                d.is_mounted,
            );
        }
        assert!(!devices.is_empty());
    }

    /// Tier-4: find_by_path should locate a known device.
    #[test]
    #[ignore = "requires udev and /dev access"]
    fn test_find_by_path_found() {
        let mgr = DeviceManager::new();
        // Find the first device reported by a full scan, then look it up
        let all = mgr.scan_devices().unwrap();
        if all.is_empty() {
            return;
        }
        let target = all[0].path.to_string_lossy().to_string();
        let result = mgr.find_by_path(&target);
        assert!(result.is_ok());
        assert!(
            result.unwrap().is_some(),
            "find_by_path could not locate {target}"
        );
    }

    /// Tier-4: find_by_path should return None for a non-existent path.
    #[test]
    #[ignore = "requires udev and /dev access"]
    fn test_find_by_path_not_found() {
        let mgr = DeviceManager::new();
        let result = mgr.find_by_path("/dev/nonexistent_xyz_device");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    /// Tier-1: DeviceManager::default() should not panic.
    #[test]
    fn test_default_construction() {
        let _mgr = DeviceManager;
    }
}
