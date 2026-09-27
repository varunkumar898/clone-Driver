//! Core cloning engine orchestrating reads, writes, checksumming, and checkpoints.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::StorageError;
use crate::storage::buffer::BufferPool;
use crate::storage::checksum::{ChecksumBlock, ChecksumState};
use crate::storage::io_ops::{BlockDevice, SequentialReader, SequentialWriter};
use crate::storage::journal::{CloneJournal, JournalEntry};
use crate::storage::progress::ProgressTracker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneStrategy {
    FastXxHash,
    SafeSha256,
}

#[derive(Debug, Clone)]
pub struct CloneResult {
    pub bytes_copied: u64,
    pub duration: std::time::Duration,
    pub throughput_mbps: f64,
    pub xxhash: u64,
    pub sha256: Option<String>,
    pub checksum_errors: u64,
    pub success: bool,
}

pub struct CloneEngine {
    pub source_reader: SequentialReader,
    pub dest_writer: SequentialWriter,
    pub buffer_pool: BufferPool,
    pub checksum_state: ChecksumState,
    pub progress: ProgressTracker,
    pub journal: CloneJournal,
    pub pause_flag: Arc<AtomicBool>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl CloneEngine {
    /// PRIMARY CONSTRUCTOR for production use (CLARIFICATION 2)
    pub fn new(
        source_path: &str,
        dest_path: &str,
        total_bytes: u64,
        strategy: CloneStrategy,
    ) -> Result<Self, StorageError> {
        let source_reader = SequentialReader::new(BlockDevice::open_read(source_path)?, 0);

        let dest_writer = SequentialWriter::new(BlockDevice::open_write(dest_path)?, 0);

        let buffer_pool = BufferPool::new(64, 4 * 1024 * 1024)?;

        let checksum_state = ChecksumState::new(strategy == CloneStrategy::SafeSha256);

        let progress = ProgressTracker::new(total_bytes);

        let journal = CloneJournal::create(source_path, dest_path, total_bytes)?;

        Ok(CloneEngine {
            source_reader,
            dest_writer,
            buffer_pool,
            checksum_state,
            progress,
            journal,
            pause_flag: Arc::new(AtomicBool::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    /// BACKWARD COMPAT CONSTRUCTOR for tests (CLARIFICATION 2)
    pub fn from_buffer_pool(pool: BufferPool) -> Self {
        // Minimal CloneEngine for testing only
        // Uses dummy devices and paths
        CloneEngine {
            source_reader: SequentialReader::new(BlockDevice::open_read("/dev/null").unwrap(), 0),
            dest_writer: SequentialWriter::new(BlockDevice::open_read("/dev/null").unwrap(), 0),
            buffer_pool: pool,
            checksum_state: ChecksumState::new(false),
            progress: ProgressTracker::new(0),
            journal: CloneJournal::create("/dev/null", "/dev/null", 0).unwrap(),
            pause_flag: Arc::new(AtomicBool::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Execute the clone operation
    pub async fn execute(&mut self) -> Result<CloneResult, StorageError> {
        let start_time = std::time::Instant::now();

        loop {
            // Check for cancel
            if self.cancel_flag.load(Ordering::Relaxed) {
                return Err(StorageError::CloneCancelled);
            }

            // Check for pause
            while self.pause_flag.load(Ordering::Relaxed) {
                std::hint::spin_loop();
            }

            // Acquire buffer
            let mut buffer = match self.buffer_pool.acquire() {
                Ok(buf) => buf,
                Err(_) => {
                    // Backpressure - spin
                    std::hint::spin_loop();
                    continue;
                }
            };

            // Read from source
            let bytes_read = self.source_reader.read_into(&mut buffer)?;

            if bytes_read == 0 {
                // End of file
                self.buffer_pool.release(buffer)?;
                break;
            }

            // Update checksum
            self.checksum_state.update(buffer.as_slice());

            // Write to destination
            let bytes_written = self.dest_writer.write_from(&buffer)?;

            if bytes_written != bytes_read {
                return Err(StorageError::ShortWrite);
            }

            // Release buffer
            self.buffer_pool.release(buffer)?;

            // Report progress
            let _snapshot = self.progress.report_progress(bytes_written as u64);

            // Checkpoint every 500 MB
            const CHECKPOINT_INTERVAL: u64 = 500 * 1024 * 1024;
            let bytes_copied = self.progress.snapshot().bytes_copied;

            if bytes_copied.is_multiple_of(CHECKPOINT_INTERVAL) && bytes_copied > 0 {
                let (xxhash, sha256) = self.checksum_state.clone().finalize();

                let entry = JournalEntry {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|_| StorageError::TimeError)?
                        .as_secs(),
                    offset: bytes_copied,
                    bytes_copied,
                    checksum_block: ChecksumBlock {
                        offset: bytes_copied,
                        size: 0,
                        xxhash,
                        sha256,
                    },
                };

                self.journal.append(entry)?;
                self.dest_writer.device.sync_buffers()?;
            }
        }

        // Finalize checksums
        let (xxhash, sha256) = self.checksum_state.clone().finalize();
        let duration = start_time.elapsed();
        let snapshot = self.progress.snapshot();

        Ok(CloneResult {
            bytes_copied: snapshot.bytes_copied,
            duration,
            throughput_mbps: snapshot.throughput_mbps,
            xxhash,
            sha256,
            checksum_errors: snapshot.checksum_errors,
            success: true,
        })
    }

    pub fn pause(&mut self) {
        self.pause_flag.store(true, Ordering::Release);
    }

    pub fn resume(&mut self) {
        self.pause_flag.store(false, Ordering::Release);
    }

    pub fn cancel(&mut self) {
        self.cancel_flag.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_engine_from_buffer_pool() {
        let pool = BufferPool::new(4, 4096).unwrap();
        let engine = CloneEngine::from_buffer_pool(pool);
        // Just verify it creates without panic
        assert_eq!(engine.progress.snapshot().bytes_copied, 0);
    }

    #[tokio::test]
    async fn test_clone_small_tempfile() {
        use std::io::Write;

        // Create source file
        let mut src = tempfile::NamedTempFile::new().unwrap();
        src.write_all(&[0u8; 1024]).unwrap();
        src.flush().unwrap();

        // Create dest file
        let dest = tempfile::NamedTempFile::new().unwrap();

        // Clone
        let mut engine = CloneEngine::new(
            src.path().to_str().unwrap(),
            dest.path().to_str().unwrap(),
            1024,
            CloneStrategy::FastXxHash,
        )
        .unwrap();

        let result = engine.execute().await.unwrap();

        assert_eq!(result.bytes_copied, 1024);
        assert!(result.success);
    }
}
