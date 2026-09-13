//! Comprehensive safety validation suite.

use super::confirmation::ConfirmationManager;
use super::protection::SystemDiskProtection;
use crate::device::BlockDevice;
use crate::error::{Result, SafetyError};

pub struct SafetyValidator;

impl SafetyValidator {
    /// Validates complete selection prior to cloning initiation.
    pub fn validate_selection(
        source: &BlockDevice,
        destination: &BlockDevice,
        confirmation_token: &str,
    ) -> Result<()> {
        // 1. Identical device check
        if source.path == destination.path {
            return Err(SafetyError::IdenticalSourceAndDestination(
                source.path.display().to_string(),
            )
            .into());
        }

        // 2. System disk protection
        SystemDiskProtection::assert_not_system_disk(destination)?;

        // 3. Mount check
        if destination.is_mounted {
            return Err(
                SafetyError::DestinationIsMounted(destination.path.display().to_string()).into(),
            );
        }

        // 4. Capacity check
        if destination.size_bytes < source.size_bytes {
            return Err(SafetyError::DestinationTooSmall {
                source_bytes: source.size_bytes,
                dest_bytes: destination.size_bytes,
            }
            .into());
        }

        // 5. Confirmation token check
        ConfirmationManager::validate_token(confirmation_token)?;

        Ok(())
    }
}
