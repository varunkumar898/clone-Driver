use diskclone::error::{CloneError, DeviceError, SafetyError};

#[test]
fn test_error_conversion_and_display() {
    let safety_err = SafetyError::DestinationIsSystemDisk("/dev/sda".to_string());
    let clone_err: CloneError = safety_err.into();

    let msg = format!("{}", clone_err);
    assert!(msg.contains("Safety validation error"));
    assert!(msg.contains("/dev/sda"));

    let dev_err = DeviceError::NotFound("/dev/nonexistent".to_string());
    let clone_err2: CloneError = dev_err.into();
    assert!(format!("{}", clone_err2).contains("Device error"));
}
