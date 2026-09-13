//! Operational context passed through state transitions.

use crate::device::BlockDevice;
use serde::{Deserialize, Serialize};

/// Contextual payload maintaining current job state, device selections, and metrics.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StateMachineContext {
    pub source_device: Option<BlockDevice>,
    pub destination_device: Option<BlockDevice>,
    pub confirmation_token: Option<String>,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub error_message: Option<String>,
}

impl StateMachineContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        todo!("Phase 3: implement context reset")
    }
}
