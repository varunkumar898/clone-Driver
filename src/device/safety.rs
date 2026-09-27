//! Safety utilities for system disk protection and capacity validation.

use std::path::{Path, PathBuf};

use crate::device::BlockDevice;

/// Errors occurring during device capacity validation.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CapacityError {
    #[error("Destination device capacity ({available} bytes) is insufficient for source ({required} bytes)")]
    InsufficientSpace { required: u64, available: u64 },

    #[error("Device query failed: {0}")]
    QueryFailed(String),
}

/// Utility for detecting whether a device corresponds to the host operating system disk.
pub struct SystemDiskDetector;

impl SystemDiskDetector {
    /// Detects the boot/root device from `/proc/cmdline`.
    pub fn detect_boot_device() -> Result<Option<String>, std::io::Error> {
        Self::detect_boot_device_from(Path::new("/proc/cmdline"))
    }

    /// Reads and parses boot device parameter from the specified cmdline path.
    pub fn detect_boot_device_from(cmdline_path: &Path) -> Result<Option<String>, std::io::Error> {
        let content = match std::fs::read_to_string(cmdline_path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };

        for token in content.split_whitespace() {
            if let Some(val) = token.strip_prefix("root=") {
                if val.starts_with("/dev/") {
                    return Ok(Some(val.to_string()));
                } else if let Some(uuid) = val.strip_prefix("UUID=") {
                    let path = PathBuf::from("/dev/disk/by-uuid").join(uuid);
                    if let Ok(target) = std::fs::canonicalize(&path) {
                        return Ok(Some(target.to_string_lossy().to_string()));
                    }
                    return Ok(Some(format!("/dev/disk/by-uuid/{}", uuid)));
                } else if let Some(label) = val.strip_prefix("LABEL=") {
                    let path = PathBuf::from("/dev/disk/by-label").join(label);
                    if let Ok(target) = std::fs::canonicalize(&path) {
                        return Ok(Some(target.to_string_lossy().to_string()));
                    }
                    return Ok(Some(format!("/dev/disk/by-label/{}", label)));
                } else {
                    return Ok(Some(val.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Finds the mount point for a device from `/proc/mounts`.
    pub fn get_mount_point(device: &str) -> Result<Option<String>, std::io::Error> {
        Self::get_mount_point_from(Path::new("/proc/mounts"), device)
    }

    /// Reads and finds mount point for device in specified mounts file.
    pub fn get_mount_point_from(
        mounts_path: &Path,
        device: &str,
    ) -> Result<Option<String>, std::io::Error> {
        let content = std::fs::read_to_string(mounts_path)?;
        let dev_norm = normalize_dev(device);

        for line in content.lines() {
            let mut parts = line.split_whitespace();
            if let (Some(dev), Some(mount)) = (parts.next(), parts.next()) {
                if normalize_dev(dev) == dev_norm {
                    return Ok(Some(mount.to_string()));
                }
            }
        }

        Ok(None)
    }

    /// Determines if a device path is a system disk using live system status.
    ///
    /// Fail-closed: returns `Ok(true)` if detection is uncertain.
    pub fn is_system_disk(device_path: &str) -> Result<bool, std::io::Error> {
        Self::is_system_disk_internal(
            Path::new("/proc/cmdline"),
            Path::new("/proc/mounts"),
            device_path,
        )
    }

    /// Internal helper allowing path substitution for unit tests.
    pub fn is_system_disk_internal(
        cmdline_path: &Path,
        mounts_path: &Path,
        device_path: &str,
    ) -> Result<bool, std::io::Error> {
        if device_path.is_empty() {
            // FAIL CLOSED: Empty or invalid device path is blocked
            return Ok(true);
        }

        // Layer 1: Check root parameter from kernel cmdline
        let boot_device = match Self::detect_boot_device_from(cmdline_path) {
            Ok(b) => b,
            Err(_) => {
                // FAIL CLOSED: Cannot safely read cmdline
                return Ok(true);
            }
        };

        if let Some(ref boot) = boot_device {
            if devices_overlap(device_path, boot) {
                return Ok(true);
            }
        }

        // Layer 2: Check critical mounts in /proc/mounts (/, /boot, /boot/efi)
        let mounts_content = match std::fs::read_to_string(mounts_path) {
            Ok(c) => c,
            Err(_) => {
                // FAIL CLOSED: Cannot safely read mounts
                return Ok(true);
            }
        };

        for line in mounts_content.lines() {
            let mut parts = line.split_whitespace();
            if let (Some(dev), Some(mount)) = (parts.next(), parts.next()) {
                if matches!(mount, "/" | "/boot" | "/boot/efi") && devices_overlap(device_path, dev)
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

/// Normalizes device path or name to base format without `/dev/`.
fn normalize_dev(path: &str) -> &str {
    path.trim_start_matches("/dev/")
}

/// Returns the parent disk name of a partition name.
fn disk_from_partition(partition: &str) -> Option<&str> {
    let p = normalize_dev(partition);

    // NVMe / MMC pattern (e.g. nvme0n1p1 -> nvme0n1, mmcblk0p1 -> mmcblk0)
    if let Some(idx) = p.rfind('p') {
        let suffix = &p[idx + 1..];
        let prefix = &p[..idx];
        if !suffix.is_empty()
            && suffix.chars().all(|c| c.is_ascii_digit())
            && idx > 0
            && p.as_bytes()[idx - 1].is_ascii_digit()
            && prefix
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_digit())
        {
            return Some(prefix);
        }
    }

    // SATA / VirtIO pattern (e.g. sda1 -> sda, vda2 -> vda)
    let disk = p.trim_end_matches(|c: char| c.is_ascii_digit());
    let looks_like_nvme_namespace =
        disk.ends_with('n') && disk.len() >= 2 && disk.as_bytes()[disk.len() - 2].is_ascii_digit();
    if disk.len() < p.len()
        && disk.chars().any(|c| c.is_ascii_alphabetic())
        && !looks_like_nvme_namespace
    {
        Some(disk)
    } else {
        None
    }
}

/// Checks whether two device paths represent the same physical disk or partition relationship.
fn devices_overlap(dev1: &str, dev2: &str) -> bool {
    let d1 = normalize_dev(dev1);
    let d2 = normalize_dev(dev2);

    if d1 == d2 {
        return true;
    }

    let p1 = disk_from_partition(d1).unwrap_or(d1);
    let p2 = disk_from_partition(d2).unwrap_or(d2);

    p1 == p2
}

/// Capacity validation utility ensuring destination disk has sufficient space.
pub struct CapacityValidator;

impl CapacityValidator {
    /// Validates that destination device capacity is at least `source_size`.
    /// Emits a warning if destination has less than 5% slack.
    pub fn validate(source_size: u64, dest_device: &BlockDevice) -> Result<(), CapacityError> {
        let dest_capacity = dest_device.size_bytes;
        if dest_capacity < source_size {
            return Err(CapacityError::InsufficientSpace {
                required: source_size,
                available: dest_capacity,
            });
        }

        if (dest_capacity as f64) < (source_size as f64 * 1.05) {
            eprintln!(
                "Warning: destination capacity ({dest_capacity} B) has less than 5% slack over source ({source_size} B)"
            );
        }

        Ok(())
    }

    /// Returns `true` if destination has less than 5% slack over source size.
    pub fn has_tight_space(source_size: u64, dest_device: &BlockDevice) -> bool {
        let dest_capacity = dest_device.size_bytes;
        dest_capacity >= source_size && (dest_capacity as f64) < (source_size as f64 * 1.05)
    }

    /// Computes the remaining free capacity of a destination block device.
    /// Validates raw capacities directly (source bytes vs dest bytes).
    pub fn validate_capacity(source_bytes: u64, dest_bytes: u64) -> Result<(), CapacityError> {
        if dest_bytes < source_bytes {
            return Err(CapacityError::InsufficientSpace {
                required: source_bytes,
                available: dest_bytes,
            });
        }
        Ok(())
    }

    pub fn get_remaining_capacity(dest_device: &BlockDevice) -> Result<u64, CapacityError> {
        let total = dest_device.size_bytes;
        let used_by_partitions: u64 = dest_device
            .partition_table
            .as_ref()
            .map(|pt| pt.partitions.iter().map(|p| p.size_bytes).sum())
            .unwrap_or(0);

        Ok(total.saturating_sub(used_by_partitions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::properties::DeviceProperties;
    use crate::partition::table::{PartitionEntry, PartitionScheme, PartitionTable};
    use std::io::Write;

    fn make_test_device(size_bytes: u64) -> BlockDevice {
        BlockDevice {
            path: PathBuf::from("/dev/sdc"),
            sysfs_name: "sdc".to_string(),
            size_bytes,
            properties: DeviceProperties::default(),
            is_system_disk: false,
            is_mounted: false,
            partition_table: None,
        }
    }

    // ── Layer 0: System Disk Detection Tests ─────────────────────────────

    #[test]
    fn test_detect_boot_device_from_cmdline() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        writeln!(
            temp,
            "BOOT_IMAGE=/vmlinuz-linux root=/dev/sda2 ro quiet splash"
        )
        .unwrap();

        let boot = SystemDiskDetector::detect_boot_device_from(temp.path()).unwrap();
        assert_eq!(boot, Some("/dev/sda2".to_string()));
    }

    #[test]
    fn test_get_mount_point_root() {
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        writeln!(temp, "/dev/sda2 / ext4 rw,relatime 0 0").unwrap();
        writeln!(temp, "/dev/sda1 /boot ext4 rw,relatime 0 0").unwrap();

        let mount = SystemDiskDetector::get_mount_point_from(temp.path(), "/dev/sda2").unwrap();
        assert_eq!(mount, Some("/".to_string()));

        let mount_boot =
            SystemDiskDetector::get_mount_point_from(temp.path(), "/dev/sda1").unwrap();
        assert_eq!(mount_boot, Some("/boot".to_string()));
    }

    #[test]
    fn test_is_system_disk_blocks_os_drive() {
        let mut cmdline = tempfile::NamedTempFile::new().unwrap();
        writeln!(cmdline, "root=/dev/nvme0n1p2 ro quiet").unwrap();

        let mut mounts = tempfile::NamedTempFile::new().unwrap();
        writeln!(mounts, "/dev/nvme0n1p2 / ext4 rw 0 0").unwrap();
        writeln!(mounts, "/dev/nvme0n1p1 /boot/efi vfat rw 0 0").unwrap();

        // Testing partition
        assert!(SystemDiskDetector::is_system_disk_internal(
            cmdline.path(),
            mounts.path(),
            "/dev/nvme0n1p2"
        )
        .unwrap());

        // Testing whole parent disk
        assert!(SystemDiskDetector::is_system_disk_internal(
            cmdline.path(),
            mounts.path(),
            "/dev/nvme0n1"
        )
        .unwrap());
    }

    #[test]
    fn test_is_system_disk_allows_data_drive() {
        let mut cmdline = tempfile::NamedTempFile::new().unwrap();
        writeln!(cmdline, "root=/dev/sda2 ro quiet").unwrap();

        let mut mounts = tempfile::NamedTempFile::new().unwrap();
        writeln!(mounts, "/dev/sda2 / ext4 rw 0 0").unwrap();
        writeln!(mounts, "/dev/sdb1 /mnt/backup ext4 rw 0 0").unwrap();

        let allowed =
            SystemDiskDetector::is_system_disk_internal(cmdline.path(), mounts.path(), "/dev/sdb")
                .unwrap();
        assert!(!allowed);
    }

    #[test]
    fn test_is_system_disk_uncertain_blocks() {
        // Missing files or empty paths must fail closed (return Ok(true))
        let missing_cmdline = Path::new("/nonexistent/cmdline");
        let missing_mounts = Path::new("/nonexistent/mounts");

        assert!(SystemDiskDetector::is_system_disk_internal(
            missing_cmdline,
            missing_mounts,
            "/dev/sdc"
        )
        .unwrap());

        let temp_cmdline = tempfile::NamedTempFile::new().unwrap();
        let temp_mounts = tempfile::NamedTempFile::new().unwrap();
        assert!(SystemDiskDetector::is_system_disk_internal(
            temp_cmdline.path(),
            temp_mounts.path(),
            ""
        )
        .unwrap());
    }

    // ── Layer 1: Capacity Validation Tests ───────────────────────────────

    #[test]
    fn test_capacity_validates_sufficient_space() {
        let dev = make_test_device(200_000_000);
        let res = CapacityValidator::validate(100_000_000, &dev);
        assert!(res.is_ok());
        assert!(!CapacityValidator::has_tight_space(100_000_000, &dev));
    }

    #[test]
    fn test_capacity_rejects_insufficient_space() {
        let dev = make_test_device(100_000_000);
        let res = CapacityValidator::validate(200_000_000, &dev);
        assert_eq!(
            res,
            Err(CapacityError::InsufficientSpace {
                required: 200_000_000,
                available: 100_000_000
            })
        );
    }

    #[test]
    fn test_capacity_warns_on_tight_space() {
        // 102MB dest, 100MB src -> dest >= src, but < 105MB (5% slack)
        let dev = make_test_device(102_000_000);
        let res = CapacityValidator::validate(100_000_000, &dev);
        assert!(res.is_ok());
        assert!(CapacityValidator::has_tight_space(100_000_000, &dev));
    }

    #[test]
    fn test_get_remaining_capacity_with_partitions() {
        let mut dev = make_test_device(500_000_000);
        dev.partition_table = Some(PartitionTable {
            scheme: PartitionScheme::Gpt,
            sector_size: 512,
            total_sectors: 500_000_000 / 512,
            partitions: vec![
                PartitionEntry {
                    index: 1,
                    start_lba: 2048,
                    end_lba: 204800,
                    size_bytes: 100_000_000,
                    partition_type_guid: None,
                    mbr_type_byte: None,
                    name: None,
                    is_bootable: false,
                },
                PartitionEntry {
                    index: 2,
                    start_lba: 204801,
                    end_lba: 409600,
                    size_bytes: 100_000_000,
                    partition_type_guid: None,
                    mbr_type_byte: None,
                    name: None,
                    is_bootable: false,
                },
            ],
        });

        let remaining = CapacityValidator::get_remaining_capacity(&dev).unwrap();
        assert_eq!(remaining, 300_000_000);
    }
}
