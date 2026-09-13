//! Application configuration and runtime tunable parameters.

use serde::{Deserialize, Serialize};

/// Global configuration options for DiskClone operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Default I/O chunk size in bytes (e.g. 4MB).
    pub default_chunk_size: usize,
    /// Maximum allowable chunk size for adaptive sizing (e.g. 16MB).
    pub max_chunk_size: usize,
    /// Minimum allowable chunk size (e.g. 512KB).
    pub min_chunk_size: usize,
    /// Number of pre-allocated buffers in the BufferPool.
    pub buffer_pool_capacity: usize,
    /// Checkpoint interval in transferred bytes (e.g. 500MB).
    pub checkpoint_interval_bytes: u64,
    /// Enable post-clone block verification pass.
    pub verify_after_clone: bool,
    /// Verification mode (xxHash64 or SHA256).
    pub verification_mode: VerificationMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMode {
    XxHash64,
    Sha256,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_chunk_size: 4 * 1024 * 1024,
            max_chunk_size: 16 * 1024 * 1024,
            min_chunk_size: 512 * 1024,
            buffer_pool_capacity: 8,
            checkpoint_interval_bytes: 500 * 1024 * 1024,
            verify_after_clone: true,
            verification_mode: VerificationMode::XxHash64,
        }
    }
}
