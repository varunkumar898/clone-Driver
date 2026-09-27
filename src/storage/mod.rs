//! Core storage I/O and verification engine.

pub mod adaptive;
pub mod buffer;
pub mod checksum;
pub mod engine;
pub mod io_ops;
pub mod journal;
pub mod progress;
pub mod verification;

pub use adaptive::{AdaptiveBlockSize, AdaptiveChunker, DeviceType};
pub use buffer::{AlignedBuffer, BufferPool};
pub use checksum::{BlockChecksum, ChecksumAlgorithm, ChecksumBlock, ChecksumState};
pub use engine::{CloneEngine, CloneResult, CloneStrategy};
pub use io_ops::{BlockDevice, BlockIo, SequentialReader, SequentialWriter};
pub use journal::{CloneJournal, JournalEntry};
pub use progress::{ProgressSnapshot, ProgressTracker, TransferMetrics};
pub use verification::{VerificationEngine, VerificationReport};
