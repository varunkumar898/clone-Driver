//! Mount status detection via /proc/mounts and /proc/self/mountinfo.

use crate::error::{DeviceError, Result};
use std::path::{Path, PathBuf};

/// A single entry from `/proc/mounts`.
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
    ///
    /// Each line has the format:
    /// ```text
    /// <device> <mountpoint> <fstype> <options> <dump> <pass>
    /// ```
    /// Lines beginning with `#` are skipped.
    pub fn list_mounts() -> Result<Vec<MountEntry>> {
        Self::parse_mounts_file(Path::new("/proc/mounts"))
    }

    /// Internal helper — accepts a path so unit tests can supply a fixture file.
    fn parse_mounts_file(path: &Path) -> Result<Vec<MountEntry>> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            DeviceError::PropertyQueryFailed(format!("Failed to read {}: {e}", path.display()))
        })?;

        let mut entries = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let device = match parts.next() {
                Some(d) => d,
                None => continue,
            };
            let mount_point = match parts.next() {
                Some(m) => m,
                None => continue,
            };
            let fs_type = parts.next().unwrap_or("").to_string();
            let options_str = parts.next().unwrap_or("");
            let options: Vec<String> = options_str.split(',').map(|s| s.to_string()).collect();

            entries.push(MountEntry {
                device_path: PathBuf::from(device),
                mount_point: PathBuf::from(mount_point),
                fs_type,
                options,
            });
        }
        Ok(entries)
    }

    /// Checks if a given device or any of its partition nodes is mounted.
    ///
    /// Matches any `MountEntry::device_path` that begins with `device_path` as a
    /// prefix, and canonicalises symlinks (e.g. `/dev/disk/by-uuid/...` -> `/dev/sda1`).
    pub fn is_device_or_partition_mounted(device_path: &Path) -> Result<bool> {
        let mounts = Self::list_mounts()?;
        let prefix = device_path.to_string_lossy();
        let canon_device = std::fs::canonicalize(device_path).ok();

        Ok(mounts.iter().any(|m| {
            // Fast path: direct prefix match
            if m.device_path.to_string_lossy().starts_with(prefix.as_ref()) {
                return true;
            }

            // Symlink resolution path (/dev/disk/by-uuid/... -> /dev/sda1)
            if let Some(ref canon) = canon_device {
                if let Ok(canon_mount) = std::fs::canonicalize(&m.device_path) {
                    if canon_mount.starts_with(canon) {
                        return true;
                    }
                }
            }

            false
        }))
    }

    /// Returns all mount points for a given device (or any of its partitions).
    ///
    /// `device` should be a bare device name such as `"sda"` or a full path
    /// such as `"/dev/sda"`. Canonicalises symlinks to match UUID/label mounts.
    pub fn get_mount_points(device: &str) -> Result<Vec<String>> {
        let mounts = Self::list_mounts()?;
        let dev_path = if device.starts_with("/dev/") {
            PathBuf::from(device)
        } else {
            PathBuf::from(format!("/dev/{device}"))
        };
        let prefix = dev_path.to_string_lossy();
        let canon_device = std::fs::canonicalize(&dev_path).ok();

        let mut points = Vec::new();
        for m in mounts {
            let matches_prefix = m.device_path.to_string_lossy().starts_with(prefix.as_ref());
            let matches_canon = if let Some(ref canon) = canon_device {
                if let Ok(canon_mount) = std::fs::canonicalize(&m.device_path) {
                    canon_mount.starts_with(canon)
                } else {
                    false
                }
            } else {
                false
            };

            if matches_prefix || matches_canon {
                points.push(m.mount_point.to_string_lossy().to_string());
            }
        }

        Ok(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_fixture(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn test_parse_mounts_basic() {
        let fixture = write_fixture(
            "/dev/sda1 / ext4 rw,relatime 0 0\n\
             /dev/sda2 /boot ext4 rw,relatime 0 0\n\
             tmpfs /tmp tmpfs rw,nosuid 0 0\n",
        );
        let entries = MountInspector::parse_mounts_file(fixture.path()).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].device_path, PathBuf::from("/dev/sda1"));
        assert_eq!(entries[0].mount_point, PathBuf::from("/"));
        assert_eq!(entries[1].mount_point, PathBuf::from("/boot"));
    }

    #[test]
    fn test_parse_mounts_skips_comments_and_blanks() {
        let fixture = write_fixture(
            "# this is a comment\n\
             \n\
             /dev/sdb1 /mnt ext4 rw 0 0\n",
        );
        let entries = MountInspector::parse_mounts_file(fixture.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].device_path, PathBuf::from("/dev/sdb1"));
    }

    #[test]
    fn test_get_mount_points() {
        // Tier-2: reads real /proc/mounts — safe as long as /proc is available
        if !Path::new("/proc/mounts").exists() {
            return;
        }
        // Root must be mounted somewhere — we just check the call doesn't panic
        let pts = MountInspector::get_mount_points("sda");
        assert!(pts.is_ok());
    }

    #[test]
    fn test_is_mounted_prefix_matching() {
        let fixture = write_fixture(
            "/dev/nvme0n1p1 / ext4 rw 0 0\n\
             /dev/nvme0n1p2 /boot vfat rw 0 0\n",
        );
        let entries = MountInspector::parse_mounts_file(fixture.path()).unwrap();
        // Simulate prefix matching manually (list_mounts reads /proc/mounts, so we test
        // the parsing half here)
        let prefix = "/dev/nvme0n1";
        let matched: Vec<_> = entries
            .iter()
            .filter(|e| e.device_path.to_string_lossy().starts_with(prefix))
            .collect();
        assert_eq!(matched.len(), 2);
    }
}
