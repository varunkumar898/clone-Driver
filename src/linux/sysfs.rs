//! /sys/block querying and attribute extraction.

use crate::device::DeviceProperties;
use crate::error::{DeviceError, Result};
use std::path::{Path, PathBuf};

pub struct SysfsInspector;

impl SysfsInspector {
    /// Sysfs root — overridable in tests by changing this constant or using helper methods.
    fn sysfs_root() -> PathBuf {
        PathBuf::from("/sys/block")
    }

    // ── Low-level file readers ──────────────────────────────────────────────

    /// Reads a sysfs file as a trimmed `String`.  Returns `None` if the file
    /// does not exist; propagates other I/O errors.
    fn read_optional_string(path: &Path) -> Result<Option<String>> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(Some(s.trim().to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(DeviceError::PropertyQueryFailed(format!(
                "Failed to read {}: {e}",
                path.display()
            ))
            .into()),
        }
    }

    /// Reads a required sysfs file, returning an error if it is absent.
    fn read_required_string(path: &Path) -> Result<String> {
        Self::read_optional_string(path)?.ok_or_else(|| {
            DeviceError::PropertyQueryFailed(format!(
                "Required sysfs file missing: {}",
                path.display()
            ))
            .into()
        })
    }

    fn read_optional_u64(path: &Path) -> Result<Option<u64>> {
        match Self::read_optional_string(path)? {
            None => Ok(None),
            Some(s) => s.parse::<u64>().map(Some).map_err(|_| {
                DeviceError::PropertyQueryFailed(format!(
                    "Cannot parse u64 from {}: {:?}",
                    path.display(),
                    s
                ))
                .into()
            }),
        }
    }

    fn read_required_u64(path: &Path) -> Result<u64> {
        Self::read_optional_u64(path)?.ok_or_else(|| {
            DeviceError::PropertyQueryFailed(format!(
                "Required sysfs file missing: {}",
                path.display()
            ))
            .into()
        })
    }

    fn read_optional_u32(path: &Path) -> Result<Option<u32>> {
        match Self::read_optional_string(path)? {
            None => Ok(None),
            Some(s) => s.parse::<u32>().map(Some).map_err(|_| {
                DeviceError::PropertyQueryFailed(format!(
                    "Cannot parse u32 from {}: {:?}",
                    path.display(),
                    s
                ))
                .into()
            }),
        }
    }

    // ── Public API ──────────────────────────────────────────────────────────

    /// Reads device size in sectors from `/sys/block/<name>/size`.
    ///
    /// This file is **required** — an error is returned if it is absent.
    pub fn read_size_sectors(dev_name: &str) -> Result<u64> {
        let path = Self::sysfs_root().join(dev_name).join("size");
        Self::read_required_u64(&path)
    }

    /// Checks whether the device is rotational (spinning HDD) from
    /// `/sys/block/<name>/queue/rotational`.
    ///
    /// Returns `false` (SSD / NVMe) if the file does not exist.
    pub fn is_rotational(dev_name: &str) -> Result<bool> {
        let path = Self::sysfs_root().join(dev_name).join("queue/rotational");
        Ok(Self::read_optional_u32(&path)?.unwrap_or(0) != 0)
    }

    /// Reads all available hardware and operational properties from sysfs.
    ///
    /// Fallback values are used for all optional files (see the plan table).
    pub fn read_properties(dev_name: &str) -> Result<DeviceProperties> {
        let base = Self::sysfs_root().join(dev_name);

        let logical_block_size =
            Self::read_optional_u32(&base.join("queue/logical_block_size"))?.unwrap_or(512);
        let physical_block_size =
            Self::read_optional_u32(&base.join("queue/physical_block_size"))?.unwrap_or(512);
        let is_rotational =
            Self::read_optional_u32(&base.join("queue/rotational"))?.unwrap_or(0) != 0;
        let is_read_only = Self::read_optional_u32(&base.join("ro"))?.unwrap_or(0) != 0;
        let is_removable = Self::read_optional_u32(&base.join("removable"))?.unwrap_or(0) != 0;

        let vendor = Self::read_optional_string(&base.join("device/vendor"))?.unwrap_or_default();
        let model = Self::read_optional_string(&base.join("device/model"))?.unwrap_or_default();
        let serial_number =
            Self::read_optional_string(&base.join("device/serial"))?.unwrap_or_default();

        Ok(DeviceProperties {
            vendor,
            model,
            serial_number,
            wwn: None, // populated from udev ID_WWN later
            logical_block_size,
            physical_block_size,
            is_rotational,
            is_read_only,
            is_removable,
        })
    }

    /// Lists partition names that exist under `/sys/block/<device_name>/`.
    ///
    /// Returns names of subdirectories that start with `device_name`, i.e. the
    /// partition nodes.
    ///
    /// Example: `/sys/block/sda/` → `["sda1", "sda2"]`
    /// Example: `/sys/block/nvme0n1/` → `["nvme0n1p1", "nvme0n1p2"]`
    pub fn get_partitions(device_name: &str) -> Result<Vec<String>> {
        let base = Self::sysfs_root().join(device_name);
        let entries = match std::fs::read_dir(&base) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => {
                return Err(DeviceError::PropertyQueryFailed(format!(
                    "Cannot list {}: {e}",
                    base.display()
                ))
                .into())
            }
        };

        let mut partitions = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Partition directories start with device_name, have more chars, and suffix starts with digit or 'p'
            if name.starts_with(device_name)
                && name.len() > device_name.len()
                && name[device_name.len()..].starts_with(|c: char| c.is_ascii_digit() || c == 'p')
                && entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
            {
                partitions.push(name);
            }
        }
        partitions.sort();
        Ok(partitions)
    }

    /// Resolves a device node (e.g. `/dev/sda`) to the canonical sysfs device name
    /// (e.g. `"sda"`) by canonicalising symlinks and returning the basename.
    pub fn canonicalize_device_node(node: &Path) -> Result<String> {
        let canonical = std::fs::canonicalize(node).map_err(|e| {
            DeviceError::InvalidPath(format!("Cannot canonicalise {}: {e}", node.display()))
        })?;
        canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| {
                DeviceError::InvalidPath(format!(
                    "No file name component in {}",
                    canonical.display()
                ))
                .into()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tier-1: pure logic — checks that the sysfs root path is /sys/block
    #[test]
    fn test_sysfs_root() {
        assert_eq!(SysfsInspector::sysfs_root(), PathBuf::from("/sys/block"));
    }

    /// Tier-2: reads real sysfs — safe on any Linux host
    #[test]
    fn test_read_size_sectors_real() {
        if !Path::new("/sys/block").exists() {
            return;
        }
        let Ok(mut entries) = std::fs::read_dir("/sys/block") else {
            return;
        };
        if let Some(Ok(entry)) = entries.next() {
            let name = entry.file_name().to_string_lossy().to_string();
            // size file must exist
            let result = SysfsInspector::read_size_sectors(&name);
            assert!(
                result.is_ok(),
                "read_size_sectors failed for {name}: {result:?}"
            );
            assert!(result.unwrap() > 0, "Device {name} reported 0 sectors");
        }
    }

    /// Tier-2: checks rotational flag is readable without error
    #[test]
    fn test_is_rotational_no_panic() {
        if !Path::new("/sys/block").exists() {
            return;
        }
        let Ok(mut entries) = std::fs::read_dir("/sys/block") else {
            return;
        };
        if let Some(Ok(entry)) = entries.next() {
            let name = entry.file_name().to_string_lossy().to_string();
            let result = SysfsInspector::is_rotational(&name);
            assert!(
                result.is_ok(),
                "is_rotational failed for {name}: {result:?}"
            );
        }
    }

    /// Tier-2: read_properties should not panic for real devices
    #[test]
    fn test_read_properties_real() {
        if !Path::new("/sys/block").exists() {
            return;
        }
        let Ok(mut entries) = std::fs::read_dir("/sys/block") else {
            return;
        };
        if let Some(Ok(entry)) = entries.next() {
            let name = entry.file_name().to_string_lossy().to_string();
            let result = SysfsInspector::read_properties(&name);
            assert!(
                result.is_ok(),
                "read_properties failed for {name}: {result:?}"
            );
            let props = result.unwrap();
            // Logical block size should be a power of two ≥ 512
            assert!(props.logical_block_size >= 512);
            assert!(props.logical_block_size.is_power_of_two());
        }
    }

    /// Tier-2: get_partitions returns a sorted list (possibly empty for loop devices)
    #[test]
    fn test_get_partitions_real() {
        if !Path::new("/sys/block").exists() {
            return;
        }
        let Ok(mut entries) = std::fs::read_dir("/sys/block") else {
            return;
        };
        if let Some(Ok(entry)) = entries.next() {
            let name = entry.file_name().to_string_lossy().to_string();
            let result = SysfsInspector::get_partitions(&name);
            assert!(result.is_ok());
            // All returned names must start with device name
            for part in result.unwrap() {
                assert!(part.starts_with(&name), "{part} does not start with {name}");
            }
        }
    }

    /// Tier-1: get_partitions on a non-existent device returns Ok(vec![])
    #[test]
    fn test_get_partitions_missing_device() {
        let result = SysfsInspector::get_partitions("nonexistent_device_xyz");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
