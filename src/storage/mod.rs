//! Core storage I/O and verification engine.

pub mod adaptive;
pub mod buffer;
pub mod checksum;
pub mod engine;
pub mod io_ops;
pub mod progress;
pub mod verification;

pub use adaptive::AdaptiveChunker;
pub use buffer::BufferPool;
pub use checksum::{BlockChecksum, ChecksumAlgorithm};
pub use engine::CloneEngine;
pub use io_ops::BlockIo;
pub use progress::{ProgressTracker, TransferMetrics};
pub use verification::{VerificationEngine, VerificationReport};
