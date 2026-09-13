use diskclone::device::{BlockDevice, DeviceProperties};
use diskclone::safety::SafetyValidator;

fn create_mock_device(
    path: &str,
    size_bytes: u64,
    is_system: bool,
    is_mounted: bool,
) -> BlockDevice {
    BlockDevice {
        path: std::path::PathBuf::from(path),
        sysfs_name: path.trim_start_matches("/dev/").to_string(),
        size_bytes,
        properties: DeviceProperties::default(),
        is_system_disk: is_system,
        is_mounted,
        partition_table: None,
    }
}

#[test]
fn test_reject_identical_devices() {
    let dev = create_mock_device("/dev/sdb", 100_000_000, false, false);
    let result = SafetyValidator::validate_selection(&dev, &dev, "CLONE TO THIS DISK");
    assert!(result.is_err());
}

#[test]
fn test_reject_system_disk_destination() {
    let src = create_mock_device("/dev/sdb", 100_000_000, false, false);
    let dst = create_mock_device("/dev/nvme0n1", 500_000_000, true, false);
    let result = SafetyValidator::validate_selection(&src, &dst, "CLONE TO THIS DISK");
    assert!(result.is_err());
}

#[test]
fn test_reject_mounted_destination() {
    let src = create_mock_device("/dev/sdb", 100_000_000, false, false);
    let dst = create_mock_device("/dev/sdc", 500_000_000, false, true);
    let result = SafetyValidator::validate_selection(&src, &dst, "CLONE TO THIS DISK");
    assert!(result.is_err());
}

#[test]
fn test_reject_undersized_destination() {
    let src = create_mock_device("/dev/sdb", 500_000_000, false, false);
    let dst = create_mock_device("/dev/sdc", 100_000_000, false, false);
    let result = SafetyValidator::validate_selection(&src, &dst, "CLONE TO THIS DISK");
    assert!(result.is_err());
}
