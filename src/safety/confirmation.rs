//! Explicit typed string confirmation validation.

use crate::error::{Result, SafetyError};

/// Exact string required to authorize destructive writes.
pub const REQUIRED_CONFIRMATION_STRING: &str = "CLONE TO THIS DISK";

pub struct ConfirmationManager;

impl ConfirmationManager {
    /// Validates that the provided token strictly matches the required confirmation string.
    pub fn validate_token(provided: &str) -> Result<()> {
        if provided.trim() == REQUIRED_CONFIRMATION_STRING {
            Ok(())
        } else {
            Err(SafetyError::InvalidConfirmationToken {
                expected: REQUIRED_CONFIRMATION_STRING.to_string(),
                provided: provided.to_string(),
            }
            .into())
        }
    }
}
