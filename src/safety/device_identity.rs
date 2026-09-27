//! Device serial number and identity verification.

use crate::device::BlockDevice;
use crate::error::{Result, SafetyError};

pub struct DeviceIdentityMatcher;

impl DeviceIdentityMatcher {
    /// Compares expected and found serial numbers to prevent accidental target swapping.
    pub fn match_serials(expected: &str, found: &str) -> Result<()> {
        if !expected.is_empty() && expected == found {
            Ok(())
        } else {
            Err(SafetyError::SerialMismatch {
                expected: expected.to_string(),
                found: found.to_string(),
            }
            .into())
        }
    }

    /// Verifies that a device's serial number matches the one recorded in the resume journal.
    ///
    /// Returns `Ok(true)` when the serial matches, `Ok(false)` when the serial is empty or
    /// unknowable (no serial in sysfs), and `Err(SafetyError::SerialMismatch)` on a definitive
    /// mismatch — which should block resume to prevent cloning to the wrong disk.
    pub fn verify_resume_identity(expected_serial: &str, device: &BlockDevice) -> Result<bool> {
        let found_serial = &device.properties.serial_number;

        // If either side is unknown/empty we can't verify — treat as "not confirmed but not
        // a hard failure"; let the caller decide whether to block or warn.
        if expected_serial.is_empty() || found_serial.is_empty() {
            return Ok(false);
        }

        if expected_serial == found_serial.as_str() {
            Ok(true)
        } else {
            Err(SafetyError::SerialMismatch {
                expected: expected_serial.to_string(),
                found: found_serial.clone(),
            }
            .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::properties::DeviceProperties;
    use std::path::PathBuf;

    fn make_device(serial: &str) -> BlockDevice {
        BlockDevice {
            path: PathBuf::from("/dev/sdc"),
            sysfs_name: "sdc".to_string(),
            size_bytes: 1_000_000_000,
            properties: DeviceProperties {
                serial_number: serial.to_string(),
                ..Default::default()
            },
            is_system_disk: false,
            is_mounted: false,
            partition_table: None,
        }
    }

    #[test]
    fn test_match_serials_exact() {
        assert!(DeviceIdentityMatcher::match_serials("S1234", "S1234").is_ok());
    }

    #[test]
    fn test_match_serials_mismatch() {
        let err = DeviceIdentityMatcher::match_serials("S1234", "S9999");
        assert!(err.is_err());
    }

    #[test]
    fn test_match_serials_empty_expected_fails() {
        // Empty expected serial is treated as "no serial known" → fails match.
        let err = DeviceIdentityMatcher::match_serials("", "S1234");
        assert!(err.is_err());
    }

    #[test]
    fn test_verify_resume_identity_match() {
        let dev = make_device("S1234");
        let result = DeviceIdentityMatcher::verify_resume_identity("S1234", &dev);
        assert!(result.unwrap());
    }

    #[test]
    fn test_verify_resume_identity_mismatch_returns_err() {
        let dev = make_device("S9999");
        let result = DeviceIdentityMatcher::verify_resume_identity("S1234", &dev);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_resume_identity_empty_serial_returns_false() {
        // Device has no serial (empty) → cannot confirm but not a hard fail.
        let dev = make_device("");
        let result = DeviceIdentityMatcher::verify_resume_identity("S1234", &dev);
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_resume_identity_empty_expected_returns_false() {
        let dev = make_device("S1234");
        let result = DeviceIdentityMatcher::verify_resume_identity("", &dev);
        assert!(!result.unwrap());
    }
}
