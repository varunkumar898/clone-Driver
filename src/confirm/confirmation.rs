//! Two-step confirmation flow and validation.

use serde::{Deserialize, Serialize};

use crate::device::safety::SystemDiskDetector;
use crate::device::BlockDevice;
use crate::error::ConfirmationError;

pub const REQUIRED_CONFIRMATION_STRING: &str = "CLONE TO THIS DISK";

/// Structured payload presented for visual confirmation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfirmationRequest {
    pub source_model: String,
    pub source_capacity_gb: f64,
    pub source_serial: String,
    pub dest_model: String,
    pub dest_capacity_gb: f64,
    pub dest_serial: String,
}

impl ConfirmationRequest {
    pub fn new(
        source_model: String,
        source_capacity_gb: f64,
        source_serial: String,
        dest_model: String,
        dest_capacity_gb: f64,
        dest_serial: String,
    ) -> Self {
        ConfirmationRequest {
            source_model,
            source_capacity_gb,
            source_serial,
            dest_model,
            dest_capacity_gb,
            dest_serial,
        }
    }

    /// Format for display in UI
    pub fn format_summary(&self) -> String {
        format!(
            "Source: {} ({:.1} GB, S/N: {})\nDest: {} ({:.1} GB, S/N: {})",
            self.source_model,
            self.source_capacity_gb,
            self.source_serial,
            self.dest_model,
            self.dest_capacity_gb,
            self.dest_serial,
        )
    }
}

/// Confirmation response recorded from user inputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfirmationResponse {
    pub visual_confirmed: bool,
    pub text_confirmed: bool,
    pub text_provided: String,
}

/// Manager coordinating two-step confirmation state and safety gate.
#[derive(Debug, Clone, Default)]
pub struct ConfirmationManager {
    pub request: Option<ConfirmationRequest>,
    pub response: Option<ConfirmationResponse>,
}

impl ConfirmationManager {
    /// Creates a new empty confirmation manager.
    pub fn new() -> Self {
        Self {
            request: None,
            response: None,
        }
    }

    /// Set the confirmation request (what we're asking user to confirm)
    pub fn set_request(&mut self, request: ConfirmationRequest) {
        self.request = Some(request);
    }

    /// Get the current request
    pub fn get_request(&self) -> Option<&ConfirmationRequest> {
        self.request.as_ref()
    }

    /// Set the confirmation response (what user provided)
    pub fn set_response(&mut self, response: ConfirmationResponse) {
        self.response = Some(response);
    }

    /// Validate entire confirmation (both visual and text)
    pub fn validate_full_confirmation(&self) -> Result<(), String> {
        let response = self
            .response
            .as_ref()
            .ok_or_else(|| "No confirmation response set".to_string())?;

        if !response.visual_confirmed {
            return Err("Visual confirmation not provided".to_string());
        }

        if !response.text_confirmed {
            return Err("Text confirmation failed".to_string());
        }

        Ok(())
    }

    /// Check if confirmation is complete and valid
    pub fn is_confirmed(&self) -> bool {
        self.validate_full_confirmation().is_ok()
    }

    /// Extracts visual confirmation details from source and destination block devices.
    pub fn request_visual_confirmation(
        source: &BlockDevice,
        dest: &BlockDevice,
    ) -> ConfirmationRequest {
        ConfirmationRequest {
            source_model: if source.properties.model.is_empty() {
                "Unknown".to_string()
            } else {
                source.properties.model.clone()
            },
            source_capacity_gb: source.size_bytes as f64 / 1_000_000_000.0,
            source_serial: if source.properties.serial_number.is_empty() {
                "Unknown".to_string()
            } else {
                source.properties.serial_number.clone()
            },
            dest_model: if dest.properties.model.is_empty() {
                "Unknown".to_string()
            } else {
                dest.properties.model.clone()
            },
            dest_capacity_gb: dest.size_bytes as f64 / 1_000_000_000.0,
            dest_serial: if dest.properties.serial_number.is_empty() {
                "Unknown".to_string()
            } else {
                dest.properties.serial_number.clone()
            },
        }
    }

    /// Strictly verifies typed text confirmation (case-sensitive exact match).
    pub fn validate_text_confirmation(text: &str) -> Result<bool, ConfirmationError> {
        if text == REQUIRED_CONFIRMATION_STRING {
            Ok(true)
        } else {
            Err(ConfirmationError::TextMismatch {
                expected: REQUIRED_CONFIRMATION_STRING.to_string(),
                got: text.to_string(),
            })
        }
    }

    /// Defense-in-depth re-verification that destination path is not a system disk.
    pub fn final_check_system_disk(dest_path: &str) -> Result<(), ConfirmationError> {
        if SystemDiskDetector::is_system_disk(dest_path).unwrap_or(true) {
            return Err(ConfirmationError::SystemDiskProtected);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::properties::DeviceProperties;
    use std::path::PathBuf;

    fn make_mock_device(path: &str, model: &str, serial: &str, size_bytes: u64) -> BlockDevice {
        BlockDevice {
            path: PathBuf::from(path),
            sysfs_name: path.trim_start_matches("/dev/").to_string(),
            size_bytes,
            properties: DeviceProperties {
                model: model.to_string(),
                serial_number: serial.to_string(),
                ..Default::default()
            },
            is_system_disk: false,
            is_mounted: false,
            partition_table: None,
        }
    }

    #[test]
    fn test_confirmation_request_formatting() {
        let src = make_mock_device("/dev/sdb", "Samsung 970 EVO", "S12345", 500_000_000_000);
        let dst = make_mock_device("/dev/sdc", "Crucial MX500", "C67890", 1_000_000_000_000);

        let req = ConfirmationManager::request_visual_confirmation(&src, &dst);
        assert_eq!(req.source_model, "Samsung 970 EVO");
        assert_eq!(req.source_serial, "S12345");
        assert!((req.source_capacity_gb - 500.0).abs() < 0.01);

        assert_eq!(req.dest_model, "Crucial MX500");
        assert_eq!(req.dest_serial, "C67890");
        assert!((req.dest_capacity_gb - 1000.0).abs() < 0.01);
    }

    #[test]
    fn test_confirmation_text_exact_match() {
        let res = ConfirmationManager::validate_text_confirmation("CLONE TO THIS DISK");
        assert_eq!(res, Ok(true));
    }

    #[test]
    fn test_confirmation_text_rejects_typo() {
        // Case-sensitive check
        let res_case = ConfirmationManager::validate_text_confirmation("clone to this disk");
        assert!(matches!(
            res_case,
            Err(ConfirmationError::TextMismatch { .. })
        ));

        // Typo check
        let res_typo = ConfirmationManager::validate_text_confirmation("CLONE TO THIS DIKS");
        assert!(matches!(
            res_typo,
            Err(ConfirmationError::TextMismatch { .. })
        ));

        // Leading/trailing whitespace rejected under strict exact match
        let res_ws = ConfirmationManager::validate_text_confirmation(" CLONE TO THIS DISK ");
        assert!(matches!(
            res_ws,
            Err(ConfirmationError::TextMismatch { .. })
        ));
    }

    #[test]
    fn test_confirmation_defense_in_depth_blocks_system_disk() {
        // Safe mock device
        assert!(ConfirmationManager::final_check_system_disk("/dev/sdz").is_ok());

        // Empty string or system device triggers fail-closed protection
        assert_eq!(
            ConfirmationManager::final_check_system_disk(""),
            Err(ConfirmationError::SystemDiskProtected)
        );
    }
}
