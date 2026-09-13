//! Discrete states of the cloning workflow.

use serde::{Deserialize, Serialize};

/// Enumeration of all valid operational states in the DiskClone workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CloneState {
    /// Initial idle state. Ready to initiate device discovery.
    Idle,
    /// Actively probing sysfs, udev, and mount info for available devices.
    ScanningDevices,
    /// Device scan completed; awaiting user selection of source drive.
    AwaitingSourceSelection,
    /// Source drive selected; awaiting user selection of destination drive.
    AwaitingDestinationSelection,
    /// Evaluating safety rules (capacity, mount status, system disk check).
    ValidatingSelection,
    /// Presenting visual warning; requiring explicit typed confirmation string.
    AwaitingConfirmation,
    /// Opening file handles, pre-allocating buffer pools, initializing journal.
    Preparing,
    /// Actively reading, checksumming, writing, and streaming progress.
    Cloning,
    /// Flushing disk caches and syncing buffers to physical media.
    Flush,
    /// Reading back blocks and validating checksums between source and target.
    Verifying,
    /// Clone and verification completed successfully.
    Completed,
    /// User requested cancellation; gracefully stopping writes and syncing state.
    Cancelling,
    /// Error encountered; diagnostic captured and devices safely closed.
    Error,
}

impl std::fmt::Display for CloneState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
