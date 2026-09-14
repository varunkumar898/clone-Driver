//! libudev discovery and property extraction.

use crate::device::{BlockDevice, SystemDiskChecker};
use crate::error::{DeviceError, Result};
use crate::linux::{MountInspector, SysfsInspector};
use std::path::PathBuf;

pub struct UdevScanner;

impl UdevScanner {
    /// Queries udev for all block devices with `DEVTYPE=disk` and enriches each
    /// with sysfs properties, mount status, and system-disk classification.
    ///
    /// Returns a `Vec<BlockDevice>` sorted by device path for deterministic ordering.
    pub fn scan_disks() -> Result<Vec<BlockDevice>> {
        let mut enumerator = udev::Enumerator::new().map_err(|e| {
            DeviceError::UdevError(format!("Failed to create udev enumerator: {e}"))
        })?;

        // Filter to block subsystem, whole-disk devices only
        enumerator
            .match_subsystem("block")
            .map_err(|e| DeviceError::UdevError(format!("match_subsystem failed: {e}")))?;
        enumerator
            .match_property("DEVTYPE", "disk")
            .map_err(|e| DeviceError::UdevError(format!("match_property DEVTYPE failed: {e}")))?;

        let udev_devices = enumerator
            .scan_devices()
            .map_err(|e| DeviceError::UdevError(format!("scan_devices failed: {e}")))?;

        let mut devices: Vec<BlockDevice> = Vec::new();

        for udev_device in udev_devices {
            // Skip devices without a /dev/ node
            let devnode = match udev_device.devnode() {
                Some(n) => n.to_path_buf(),
                None => continue,
            };

            let sysfs_name = udev_device.sysname().to_string_lossy().to_string();

            // ── Size ──────────────────────────────────────────────────────
            let size_bytes = Self::read_size_bytes(&devnode, &sysfs_name);

            // ── Properties ────────────────────────────────────────────────
            // Base from sysfs; overlay udev DB values where available.
            let mut props = SysfsInspector::read_properties(&sysfs_name).unwrap_or_default();

            // Helper: extract a udev property as a non-empty String
            let udev_string = |key: &str| -> Option<String> {
                udev_device
                    .property_value(key)
                    .map(|v| v.to_string_lossy().trim().to_string())
                    .filter(|s| !s.is_empty())
            };

            if let Some(serial) =
                udev_string("ID_SERIAL_SHORT").or_else(|| udev_string("ID_SERIAL"))
            {
                props.serial_number = serial;
            }
            if let Some(model) = udev_string("ID_MODEL") {
                props.model = model;
            }
            if let Some(vendor) = udev_string("ID_VENDOR") {
                props.vendor = vendor;
            }
            if let Some(wwn) = udev_string("ID_WWN") {
                props.wwn = Some(wwn);
            }

            // ── Mount status ──────────────────────────────────────────────
            let is_mounted =
                MountInspector::is_device_or_partition_mounted(&devnode).unwrap_or(false);

            // ── System disk ───────────────────────────────────────────────
            let is_system_disk = SystemDiskChecker::is_system_disk(&devnode).unwrap_or(false);

            devices.push(BlockDevice {
                path: devnode,
                sysfs_name,
                size_bytes,
                properties: props,
                is_system_disk,
                is_mounted,
                partition_table: None,
            });
        }

        // Stable, deterministic ordering
        devices.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(devices)
    }

    /// Reads device size in bytes.
    ///
    /// Prefers an ioctl on the open device node (requires read permission);
    /// falls back to `sysfs_sectors × logical_block_size` for unprivileged contexts.
    fn read_size_bytes(devnode: &PathBuf, sysfs_name: &str) -> u64 {
        if let Ok(f) = std::fs::File::open(devnode) {
            if let Ok(size) = crate::linux::ffi::get_block_device_size(&f) {
                if size > 0 {
                    return size;
                }
            }
        }
        // Sysfs fallback
        let sectors = SysfsInspector::read_size_sectors(sysfs_name).unwrap_or(0);
        let props = SysfsInspector::read_properties(sysfs_name).unwrap_or_default();
        sectors * props.logical_block_size as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tier-4: requires udev daemon and /dev access — run manually on real hardware.
    #[test]
    #[ignore = "requires udev and /dev access; run with: cargo test -- --include-ignored"]
    fn test_udev_scan_finds_devices() {
        let result = UdevScanner::scan_disks();
        assert!(result.is_ok(), "scan_disks failed: {result:?}");
        let devices = result.unwrap();
        assert!(
            !devices.is_empty(),
            "No block devices found — is this a real machine?"
        );
        for d in &devices {
            println!(
                "{:?}  {}  system={} mounted={}",
                d.path,
                d.formatted_size(),
                d.is_system_disk,
                d.is_mounted,
            );
            assert!(d.size_bytes > 0, "Device {:?} reported 0 bytes", d.path);
        }
    }

    /// Tier-4: exactly one device should be flagged as the system disk.
    #[test]
    #[ignore = "requires udev and /dev access"]
    fn test_exactly_one_system_disk() {
        let devices = UdevScanner::scan_disks().unwrap();
        let system_disks: Vec<_> = devices.iter().filter(|d| d.is_system_disk).collect();
        assert!(
            !system_disks.is_empty(),
            "No system disk detected — boot device missing from udev scan?"
        );
        if system_disks.len() > 1 {
            eprintln!(
                "Warning: {} devices flagged as system disk: {:?}",
                system_disks.len(),
                system_disks.iter().map(|d| &d.path).collect::<Vec<_>>()
            );
        }
    }

    /// Tier-4: devices are returned in sorted order.
    #[test]
    #[ignore = "requires udev and /dev access"]
    fn test_scan_result_is_sorted() {
        let devices = UdevScanner::scan_disks().unwrap();
        let paths: Vec<_> = devices.iter().map(|d| d.path.clone()).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted, "Scan result is not sorted by device path");
    }
}
