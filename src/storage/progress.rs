//! Real-time clone progress tracking and throughput estimation.

use serde::{Deserialize, Serialize};
use std::time::Instant;

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
    bytes_processed: u64,
    start_time: Instant,
    last_update: Instant,
    last_bytes: u64,
    chunk_size: usize,
}

impl ProgressTracker {
    pub fn new(total_bytes: u64, chunk_size: usize) -> Self {
        let now = Instant::now();
        Self {
            total_bytes,
            bytes_processed: 0,
            start_time: now,
            last_update: now,
            last_bytes: 0,
            chunk_size,
        }
    }

    pub fn update(&mut self, bytes_written: u64) {
        self.bytes_processed += bytes_written;
    }

    pub fn metrics(&self) -> TransferMetrics {
        todo!("Phase 3: implement metrics calculation with smoothing")
    }
}
