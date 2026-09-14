//! Detection of operating system disks and protected system partitions.

use crate::error::{DeviceError, Result};
use std::path::{Path, PathBuf};

/// Safety utility verifying whether a path or device corresponds to the host operating system.
pub struct SystemDiskChecker;

impl SystemDiskChecker {
    // ── Root device resolution ────────────────────────────────────────────

    /// Identifies the device backing the root filesystem `/`.
    ///
    /// Resolution order:
    /// 1. Read `/proc/cmdline` → extract `root=` parameter
    ///    - `/dev/sdX`   → return as-is
    ///    - `UUID=...`   → resolve via `/dev/disk/by-uuid/`
    ///    - `LABEL=...`  → resolve via `/dev/disk/by-label/`
    /// 2. Fallback: parse `/proc/mounts` for the `/` mount point
    /// 3. Return `None` if both steps fail
    pub fn get_root_device() -> Result<Option<String>> {
        // Step 1: /proc/cmdline
        if let Ok(Some(device)) = Self::root_from_cmdline() {
            return Ok(Some(device));
        }
        // Step 2: /proc/mounts fallback
        Self::root_from_mounts()
    }

    /// Parses `/proc/cmdline` to find the `root=` parameter and resolves it to a
    /// `/dev/…` path.
    fn root_from_cmdline() -> Result<Option<String>> {
        let cmdline = match std::fs::read_to_string("/proc/cmdline") {
            Ok(s) => s,
            Err(_) => return Ok(None),
        };

        for token in cmdline.split_whitespace() {
            if let Some(value) = token.strip_prefix("root=") {
                if let Ok(resolved) = Self::resolve_root_value(value) {
                    return Ok(Some(resolved));
                }
            }
        }
        Ok(None)
    }

    /// Resolves a `root=` value to an absolute `/dev/` path.
    ///
    /// Handles three formats:
    /// - `/dev/sdX`   → returned as-is
    /// - `UUID=<id>`  → symlink resolution from `/dev/disk/by-uuid/<id>`
    /// - `LABEL=<l>`  → symlink resolution from `/dev/disk/by-label/<l>`
    fn resolve_root_value(value: &str) -> Result<String> {
        if value.starts_with("/dev/") {
            return Ok(value.to_string());
        }

        if let Some(uuid) = value.strip_prefix("UUID=") {
            let link = PathBuf::from("/dev/disk/by-uuid").join(uuid);
            return Self::resolve_symlink(&link);
        }

        if let Some(label) = value.strip_prefix("LABEL=") {
            let link = PathBuf::from("/dev/disk/by-label").join(label);
            return Self::resolve_symlink(&link);
        }

        // Unknown format — return as-is and let the caller decide
        Ok(value.to_string())
    }

    /// Resolves a symlink in `/dev/disk/by-uuid/` or `/dev/disk/by-label/` to a
    /// canonical `/dev/…` path.
    fn resolve_symlink(link: &Path) -> Result<String> {
        let canonical = std::fs::canonicalize(link).map_err(|e| {
            DeviceError::PropertyQueryFailed(format!(
                "Cannot resolve symlink {}: {e}",
                link.display()
            ))
        })?;
        Ok(canonical.to_string_lossy().to_string())
    }

    /// Parses `/proc/mounts` to find the device mounted at `/`.
    fn root_from_mounts() -> Result<Option<String>> {
        let content = match std::fs::read_to_string("/proc/mounts") {
            Ok(s) => s,
            Err(_) => return Ok(None),
        };

        for line in content.lines() {
            let mut parts = line.split_whitespace();
            let device = match parts.next() {
                Some(d) => d,
                None => continue,
            };
            let mount_point = match parts.next() {
                Some(m) => m,
                None => continue,
            };
            if mount_point == "/" {
                return Ok(Some(device.to_string()));
            }
        }
        Ok(None)
    }

    // ── Partition → disk name extraction ─────────────────────────────────

    /// Extracts the parent disk name from a partition device name.
    ///
    /// Handles the following naming schemes:
    ///
    /// | Scheme | Example input | Output |
    /// |--------|---------------|--------|
    /// | NVMe   | `nvme0n1p2`   | `nvme0n1` |
    /// | MMC    | `mmcblk0p1`   | `mmcblk0` |
    /// | SATA   | `sda3`        | `sda` |
    /// | virtio | `vda2`        | `vda` |
    ///
    /// Returns `None` if the name has no recognisable partition suffix.
    pub fn disk_from_partition(partition: &str) -> Option<String> {
        // NVMe / MMC pattern: disk name ends with a digit, then 'p', then partition digits.
        // e.g. nvme0n1p1  →  idx of last 'p' = 7, suffix = "1", char before 'p' = '1' ✓
        // e.g. nvme0n1    →  idx of last 'p' = 5 ("n"), suffix = "1", char before 'p' = '0' –
        //                    BUT stripping suffix "1" would give "nvme0n" which has no
        //                    alphabetic run after a digit, so we additionally require the
        //                    remaining prefix itself ends with a digit ("n1").
        if let Some(idx) = partition.rfind('p') {
            let suffix = &partition[idx + 1..];
            let prefix = &partition[..idx];
            if !suffix.is_empty()
                && suffix.chars().all(|c| c.is_ascii_digit())
                && idx > 0
                && partition.as_bytes()[idx - 1].is_ascii_digit()
                // The prefix must itself end with a digit (true for nvme0n1, mmcblk0)
                // This rejects bare names like "nvme0n" that have no trailing digit.
                && prefix.chars().next_back().is_some_and(|c| c.is_ascii_digit())
            {
                return Some(prefix.to_string());
            }
        }

        // SATA / virtio: strip trailing decimal digits.
        // Guards:
        //   1. Digits were actually stripped (input had a partition number)
        //   2. Remainder contains at least one letter (not a pure number)
        //   3. Remainder is not a bare NVMe/MMC whole-disk name.
        //      NVMe whole-disk names end with the namespace suffix "n<digits>":
        //      e.g. "nvme0n1" → stripped to "nvme0n" which ends with 'n' preceded
        //      by a digit.  SATA names "sda", "vda" end with a non-digit letter
        //      that is NOT preceded by a digit, so they pass.
        let disk = partition.trim_end_matches(|c: char| c.is_ascii_digit());
        let looks_like_nvme_namespace = disk.ends_with('n')
            && disk.len() >= 2
            && disk.as_bytes()[disk.len() - 2].is_ascii_digit();
        if disk.len() < partition.len()
            && disk.chars().any(|c| c.is_ascii_alphabetic())
            && !looks_like_nvme_namespace
        {
            Some(disk.to_string())
        } else {
            None
        }
    }

    // ── System disk check ─────────────────────────────────────────────────

    /// Determines whether the given block device path hosts the root filesystem `/`,
    /// `/boot`, or `/boot/efi`.
    ///
    /// A device is considered the system disk when:
    /// - the root partition (from `/proc/cmdline` / `/proc/mounts`) resolves to
    ///   the same physical disk after stripping the partition suffix, **or**
    /// - any partition of this disk appears in `/proc/mounts` for a critical
    ///   mount point (`/`, `/boot`, `/boot/efi`) — this covers LVM / device-mapper
    ///   setups where the "root" device is e.g. `/dev/mapper/root`.
    pub fn is_system_disk(device_path: &Path) -> Result<bool> {
        // Fast path: direct name match via cmdline / mounts root
        if let Some(root_partition) = Self::get_root_device()? {
            let root_name = root_partition
                .strip_prefix("/dev/")
                .unwrap_or(&root_partition);
            let root_disk =
                Self::disk_from_partition(root_name).unwrap_or_else(|| root_name.to_string());
            let test_name = device_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if test_name == root_disk || test_name == root_name {
                return Ok(true);
            }
        }

        // Slow path: check whether any partition of this disk is mounted at a
        // critical mount point.  This handles LVM, dm-crypt, btrfs subvols, etc.
        let device_prefix = device_path.to_string_lossy().to_string();
        let content = match std::fs::read_to_string("/proc/mounts") {
            Ok(s) => s,
            Err(_) => return Ok(false),
        };
        for line in content.lines() {
            let mut parts = line.split_whitespace();
            let _device = parts.next().unwrap_or("");
            let mount_point = parts.next().unwrap_or("");
            // We care about partitions of THIS disk being at critical mounts
            // A partition of /dev/sda would be /dev/sda1, /dev/sda2, etc.
            if matches!(mount_point, "/" | "/boot" | "/boot/efi") {
                // Check if this line's device starts with our disk path prefix
                if _device.starts_with(&device_prefix) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── disk_from_partition — Tier-1 (pure logic) ────────────────────────

    #[test]
    fn test_disk_from_partition_sata() {
        assert_eq!(
            SystemDiskChecker::disk_from_partition("sda1"),
            Some("sda".to_string())
        );
        assert_eq!(
            SystemDiskChecker::disk_from_partition("sdb3"),
            Some("sdb".to_string())
        );
    }

    #[test]
    fn test_disk_from_partition_nvme() {
        assert_eq!(
            SystemDiskChecker::disk_from_partition("nvme0n1p1"),
            Some("nvme0n1".to_string())
        );
        assert_eq!(
            SystemDiskChecker::disk_from_partition("nvme1n2p3"),
            Some("nvme1n2".to_string())
        );
    }

    #[test]
    fn test_disk_from_partition_mmc() {
        assert_eq!(
            SystemDiskChecker::disk_from_partition("mmcblk0p1"),
            Some("mmcblk0".to_string())
        );
    }

    #[test]
    fn test_disk_from_partition_virtio() {
        assert_eq!(
            SystemDiskChecker::disk_from_partition("vda2"),
            Some("vda".to_string())
        );
    }

    #[test]
    fn test_disk_from_partition_whole_disk_returns_none() {
        // Whole disk names have no partition suffix
        assert_eq!(SystemDiskChecker::disk_from_partition("sda"), None);
        assert_eq!(SystemDiskChecker::disk_from_partition("nvme0n1"), None);
    }

    // ── get_root_device — Tier-2 (reads /proc) ───────────────────────────

    #[test]
    fn test_get_root_device_returns_some_or_none() {
        // On any Linux host /proc/cmdline exists; result should be Ok(…)
        if !Path::new("/proc/cmdline").exists() {
            return;
        }
        let result = SystemDiskChecker::get_root_device();
        assert!(result.is_ok(), "get_root_device returned Err: {result:?}");
        // Value (if present) must look like a /dev/ path
        if let Some(dev) = result.unwrap() {
            assert!(
                dev.starts_with("/dev/") || dev.is_empty(),
                "Unexpected root device: {dev}"
            );
        }
    }

    // ── is_system_disk — Tier-2 ──────────────────────────────────────────

    #[test]
    fn test_is_system_disk_on_real_system() {
        if !Path::new("/proc/mounts").exists() {
            return;
        }
        // Find what is actually mounted at "/" and use that device's parent disk.
        let content = std::fs::read_to_string("/proc/mounts").unwrap();
        let root_device = content.lines().find_map(|line| {
            let mut parts = line.split_whitespace();
            let dev = parts.next()?;
            let mnt = parts.next()?;
            if mnt == "/" {
                Some(dev.to_string())
            } else {
                None
            }
        });
        let Some(root_dev) = root_device else { return };

        // For dm / mapper devices the disk is the mapper itself; for regular
        // block devices strip the partition suffix.
        let root_name = root_dev.strip_prefix("/dev/").unwrap_or(&root_dev);
        let disk_name = SystemDiskChecker::disk_from_partition(root_name)
            .unwrap_or_else(|| root_name.to_string());
        let disk_path = PathBuf::from("/dev").join(&disk_name);

        let result = SystemDiskChecker::is_system_disk(&disk_path);
        assert!(result.is_ok(), "is_system_disk returned Err: {result:?}");
        // Only assert system-disk if the disk path actually exists on this machine
        if disk_path.exists() {
            assert!(
                result.unwrap(),
                "Expected {disk_path:?} to be the system disk"
            );
        }
    }
}
