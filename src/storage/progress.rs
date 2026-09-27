//! Real-time clone progress tracking and throughput estimation.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferMetrics {
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub percentage: f64,
    pub eta_seconds: u64,
    pub current_chunk: u64,
    pub total_chunks: u64,
}

pub struct ProgressTracker {
    total_bytes: u64,
    bytes_copied: AtomicU64,
    checksum_errors: AtomicU64,
    start_time: Instant,
    chunk_size: usize,
}

impl ProgressTracker {
    pub fn new(total_bytes: u64) -> Self {
        ProgressTracker {
            total_bytes,
            bytes_copied: AtomicU64::new(0),
            checksum_errors: AtomicU64::new(0),
            start_time: Instant::now(),
            chunk_size: 4 * 1024 * 1024,
        }
    }

    pub fn with_chunk_size(total_bytes: u64, chunk_size: usize) -> Self {
        ProgressTracker {
            total_bytes,
            bytes_copied: AtomicU64::new(0),
            checksum_errors: AtomicU64::new(0),
            start_time: Instant::now(),
            chunk_size,
        }
    }

    /// Report progress atomically
    pub fn report_progress(&self, bytes: u64) -> ProgressSnapshot {
        self.bytes_copied.fetch_add(bytes, Ordering::Relaxed);
        self.snapshot()
    }

    /// Update progress (alias for compatibility)
    pub fn update(&self, bytes_written: u64) {
        self.bytes_copied
            .fetch_add(bytes_written, Ordering::Relaxed);
    }

    /// Report checksum error
    pub fn report_checksum_error(&self) {
        self.checksum_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current progress snapshot
    pub fn snapshot(&self) -> ProgressSnapshot {
        let bytes_copied = self.bytes_copied.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();

        let percentage = if self.total_bytes > 0 {
            (bytes_copied as f64 / self.total_bytes as f64) * 100.0
        } else {
            0.0
        };

        let throughput_mbps = if elapsed.as_secs_f64() > 0.0 {
            (bytes_copied as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let remaining_bytes = self.total_bytes.saturating_sub(bytes_copied);
        let eta = if throughput_mbps > 0.0 {
            let remaining_secs = (remaining_bytes as f64 / (1024.0 * 1024.0)) / throughput_mbps;
            Duration::from_secs_f64(remaining_secs)
        } else {
            Duration::from_secs(0)
        };

        ProgressSnapshot {
            bytes_copied,
            total_bytes: self.total_bytes,
            percentage,
            throughput_mbps,
            eta,
            elapsed,
            checksum_errors: self.checksum_errors.load(Ordering::Relaxed),
        }
    }

    /// Compute TransferMetrics for backward compatibility
    pub fn metrics(&self) -> TransferMetrics {
        let snap = self.snapshot();
        let current_chunk = if self.chunk_size > 0 {
            snap.bytes_copied / self.chunk_size as u64
        } else {
            0
        };
        let total_chunks = if self.chunk_size > 0 {
            snap.total_bytes.div_ceil(self.chunk_size as u64)
        } else {
            0
        };

        TransferMetrics {
            bytes_processed: snap.bytes_copied,
            total_bytes: snap.total_bytes,
            speed_bytes_per_sec: (snap.throughput_mbps * 1024.0 * 1024.0) as u64,
            percentage: snap.percentage,
            eta_seconds: snap.eta.as_secs(),
            current_chunk,
            total_chunks,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProgressSnapshot {
    pub bytes_copied: u64,
    pub total_bytes: u64,
    pub percentage: f64,
    pub throughput_mbps: f64,
    pub eta: Duration,
    pub elapsed: Duration,
    pub checksum_errors: u64,
}

impl ProgressSnapshot {
    pub fn format_summary(&self) -> String {
        let eta_secs = self.eta.as_secs();
        let eta_mins = eta_secs / 60;
        let eta_secs = eta_secs % 60;

        let bytes_str = format_bytes(self.bytes_copied);
        let total_str = format_bytes(self.total_bytes);

        format!(
            "{:.1}% | {:.0} MB/s | ETA {}m {}s | {} / {}",
            self.percentage, self.throughput_mbps, eta_mins, eta_secs, bytes_str, total_str,
        )
    }
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_tracker_initial() {
        let tracker = ProgressTracker::new(1000);
        let snap = tracker.snapshot();
        assert_eq!(snap.bytes_copied, 0);
        assert_eq!(snap.total_bytes, 1000);
        assert_eq!(snap.percentage, 0.0);
        assert_eq!(snap.checksum_errors, 0);
    }

    #[test]
    fn test_progress_tracker_new() {
        let tracker = ProgressTracker::new(1000);
        let snap = tracker.snapshot();
        assert_eq!(snap.percentage, 0.0);
    }

    #[test]
    fn test_progress_report_progress() {
        let tracker = ProgressTracker::new(1000);
        let snap = tracker.report_progress(500);
        assert_eq!(snap.bytes_copied, 500);
        assert!(snap.percentage > 0.0);
    }

    #[test]
    fn test_progress_format_summary() {
        let snap = ProgressSnapshot {
            bytes_copied: 500_000_000,
            total_bytes: 1_000_000_000,
            percentage: 50.0,
            throughput_mbps: 250.0,
            eta: Duration::from_secs(120),
            elapsed: Duration::from_secs(120),
            checksum_errors: 0,
        };

        let summary = snap.format_summary();
        assert!(summary.contains("50.0%"));
        assert!(summary.contains("250"));
    }

    #[test]
    fn test_report_checksum_error() {
        let tracker = ProgressTracker::new(1000);
        tracker.report_checksum_error();
        tracker.report_checksum_error();
        assert_eq!(tracker.snapshot().checksum_errors, 2);
    }
}
