//! Unit tests for system disk detection (Layer 0).
//!
//! These tests verify `SystemDiskDetector` and `SystemDiskChecker` on real
//! Linux `/proc` interfaces without requiring a real block device.

use diskclone::device::SystemDiskChecker;
use diskclone::device::SystemDiskDetector;
use std::io::Write;

// ── SystemDiskDetector ────────────────────────────────────────────────────────

#[test]
fn test_detect_boot_device_handles_no_root() {
    // On any Linux host /proc/cmdline exists; result must not panic.
    let result = SystemDiskDetector::detect_boot_device();
    assert!(
        result.is_ok(),
        "detect_boot_device returned Err: {result:?}"
    );
}

#[test]
fn test_detect_boot_device_parses_dev_path() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "BOOT_IMAGE=/vmlinuz root=/dev/sda2 ro quiet splash").unwrap();

    let boot = SystemDiskDetector::detect_boot_device_from(tmp.path()).unwrap();
    assert_eq!(boot, Some("/dev/sda2".to_string()));
}

#[test]
fn test_detect_boot_device_parses_uuid() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    // UUID that won't resolve on disk — detect_boot_device_from falls back to the
    // raw by-uuid path when canonicalize fails.
    writeln!(tmp, "root=UUID=dead-beef-1234 ro").unwrap();

    let boot = SystemDiskDetector::detect_boot_device_from(tmp.path()).unwrap();
    // Must return Some (even if by-uuid symlink doesn't exist on the test host).
    assert!(boot.is_some());
    let s = boot.unwrap();
    // Either the canonical /dev/... or the raw by-uuid path.
    assert!(s.starts_with("/dev/"), "Unexpected value: {s}");
}

#[test]
fn test_detect_boot_device_no_root_param() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "BOOT_IMAGE=/vmlinuz ro quiet splash").unwrap();

    let boot = SystemDiskDetector::detect_boot_device_from(tmp.path()).unwrap();
    assert_eq!(boot, None);
}

#[test]
fn test_is_system_disk_nonexistent_device() {
    // Should not panic; fail-closed for empty path.
    let result = SystemDiskDetector::is_system_disk("/dev/nonexistent_zzz");
    assert!(result.is_ok());
    // Non-existent device with no cmdline/mounts match → Ok(false)
}

#[test]
fn test_is_system_disk_empty_path_fail_closed() {
    let cmdline = tempfile::NamedTempFile::new().unwrap();
    let mounts = tempfile::NamedTempFile::new().unwrap();
    // Empty device path must fail-closed → Ok(true).
    let result =
        SystemDiskDetector::is_system_disk_internal(cmdline.path(), mounts.path(), "").unwrap();
    assert!(result, "Empty device path should fail closed");
}

#[test]
fn test_get_mount_point_nonexistent() {
    // Should not panic.
    let result = SystemDiskDetector::get_mount_point("/dev/nonexistent_zzz");
    assert!(result.is_ok());
}

#[test]
fn test_get_mount_point_from_mock() {
    let mut mounts = tempfile::NamedTempFile::new().unwrap();
    writeln!(mounts, "/dev/sda2 / ext4 rw,relatime 0 0").unwrap();
    writeln!(mounts, "/dev/sda1 /boot vfat rw 0 0").unwrap();

    let mp = SystemDiskDetector::get_mount_point_from(mounts.path(), "/dev/sda2").unwrap();
    assert_eq!(mp, Some("/".to_string()));

    let mp2 = SystemDiskDetector::get_mount_point_from(mounts.path(), "/dev/sda1").unwrap();
    assert_eq!(mp2, Some("/boot".to_string()));

    let mp3 = SystemDiskDetector::get_mount_point_from(mounts.path(), "/dev/sdb").unwrap();
    assert_eq!(mp3, None);
}

#[test]
fn test_is_system_disk_blocks_nvme_parent() {
    let mut cmdline = tempfile::NamedTempFile::new().unwrap();
    writeln!(cmdline, "root=/dev/nvme0n1p2 ro").unwrap();

    let mut mounts = tempfile::NamedTempFile::new().unwrap();
    writeln!(mounts, "/dev/nvme0n1p2 / ext4 rw 0 0").unwrap();

    // Querying parent disk (/dev/nvme0n1) → must also block.
    assert!(SystemDiskDetector::is_system_disk_internal(
        cmdline.path(),
        mounts.path(),
        "/dev/nvme0n1",
    )
    .unwrap());
}

#[test]
fn test_is_system_disk_allows_data_drive() {
    let mut cmdline = tempfile::NamedTempFile::new().unwrap();
    writeln!(cmdline, "root=/dev/sda2 ro").unwrap();

    let mut mounts = tempfile::NamedTempFile::new().unwrap();
    writeln!(mounts, "/dev/sda2 / ext4 rw 0 0").unwrap();
    writeln!(mounts, "/dev/sdb1 /data ext4 rw 0 0").unwrap();

    // /dev/sdb is a data drive, must be allowed.
    let result =
        SystemDiskDetector::is_system_disk_internal(cmdline.path(), mounts.path(), "/dev/sdb")
            .unwrap();
    assert!(!result, "/dev/sdb should NOT be flagged as system disk");
}

#[test]
fn test_is_system_disk_missing_files_fail_closed() {
    use std::path::Path;
    let result = SystemDiskDetector::is_system_disk_internal(
        Path::new("/nonexistent/cmdline"),
        Path::new("/nonexistent/mounts"),
        "/dev/sdc",
    )
    .unwrap();
    assert!(
        result,
        "Missing proc files should fail closed (block clone)"
    );
}

// ── SystemDiskChecker ─────────────────────────────────────────────────────────

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
}

#[test]
fn test_disk_from_partition_whole_disk_none() {
    assert_eq!(SystemDiskChecker::disk_from_partition("sda"), None);
    assert_eq!(SystemDiskChecker::disk_from_partition("nvme0n1"), None);
}

#[test]
#[ignore] // Requires actual root system with /proc/cmdline pointing to a real device
fn test_is_system_disk_real_root_partition() {
    use std::path::Path;
    if !Path::new("/proc/mounts").exists() {
        return;
    }
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
    if let Some(root_dev) = root_device {
        let root_name = root_dev.strip_prefix("/dev/").unwrap_or(&root_dev);
        let disk_name = SystemDiskChecker::disk_from_partition(root_name)
            .unwrap_or_else(|| root_name.to_string());
        let disk_path = std::path::PathBuf::from("/dev").join(&disk_name);
        let result = SystemDiskChecker::is_system_disk(&disk_path);
        assert!(result.is_ok());
        if disk_path.exists() {
            assert!(result.unwrap(), "{disk_path:?} should be system disk");
        }
    }
}
