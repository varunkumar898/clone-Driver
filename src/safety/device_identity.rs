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

    /// Verifies device identity for resume operation.
    pub fn verify_resume_identity(_expected_serial: &str, _device: &BlockDevice) -> Result<bool> {
        todo!("Phase 3: implement verify_resume_identity")
    }
}
