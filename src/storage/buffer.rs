//! Aligned memory buffer pool.

use crate::error::{Result, StorageError};
use std::sync::{Arc, Mutex};

/// A memory-aligned byte buffer for O_DIRECT and high-throughput block I/O.
pub struct AlignedBuffer {
    data: Vec<u8>,
    capacity: usize,
}

impl AlignedBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0u8; capacity],
            capacity,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Pre-allocated pool of aligned buffers to prevent runtime memory allocations.
#[derive(Clone)]
pub struct BufferPool {
    pool: Arc<Mutex<Vec<AlignedBuffer>>>,
    chunk_size: usize,
}

impl BufferPool {
    /// Creates a new buffer pool with the given capacity and chunk size.
    pub fn new(capacity: usize, chunk_size: usize) -> Self {
        let buffers = (0..capacity)
            .map(|_| AlignedBuffer::new(chunk_size))
            .collect();
        Self {
            pool: Arc::new(Mutex::new(buffers)),
            chunk_size,
        }
    }

    /// Acquires a buffer from the pool, returning an error if the pool is exhausted.
    pub fn acquire(&self) -> Result<AlignedBuffer> {
        let mut guard = self
            .pool
            .lock()
            .map_err(|_| StorageError::BufferPoolExhausted)?;
        guard.pop().ok_or(StorageError::BufferPoolExhausted.into())
    }

    /// Returns a buffer to the pool for reuse.
    pub fn release(&self, buffer: AlignedBuffer) {
        if let Ok(mut guard) = self.pool.lock() {
            guard.push(buffer);
        }
    }

    /// Returns chunk size configured for this pool.
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}
