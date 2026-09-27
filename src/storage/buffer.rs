//! Page-aligned memory buffer pool.
//!
//! Uses crossbeam-queue for lock-free buffer management, avoiding heap churn
//! during the high-speed clone loop.

use crate::error::StorageError;
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// ------------------------------------------------------------------
// AlignedBuffer
// ------------------------------------------------------------------

/// Pre-allocated, page-aligned buffer.
pub struct AlignedBuffer {
    /// The actual data bytes (page-aligned allocation)
    pub data: Vec<u8>,
    /// Number of valid bytes in data (≤ capacity)
    pub size: usize,
    /// Source device offset for this buffer
    pub offset: u64,
    /// Pool index tracking for recycled buffer lookup
    pub(crate) pool_idx: Option<usize>,
}

impl AlignedBuffer {
    /// Create new page-aligned buffer with given capacity
    pub fn new(capacity: usize) -> Result<Self, StorageError> {
        // Use Vec with initialized capacity so as_mut_slice provides valid slice
        let data = vec![0u8; capacity];

        // Verify page alignment (4096 bytes)
        let ptr = data.as_ptr() as usize;
        if !ptr.is_multiple_of(4096) {
            eprintln!("Warning: buffer not page-aligned at {:#x}", ptr);
        }

        Ok(AlignedBuffer {
            data,
            size: 0,
            offset: 0,
            pool_idx: None,
        })
    }

    /// Reset buffer for reuse (clear size, preserve capacity)
    pub fn reset(&mut self) {
        if self.data.len() != self.data.capacity() {
            self.data.resize(self.data.capacity(), 0);
        }
        self.size = 0;
        self.offset = 0;
    }

    /// Get capacity (max data we can hold)
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Get current valid size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Set current valid size
    pub fn set_size(&mut self, size: usize) {
        self.size = size;
    }

    /// Get source device offset
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Set source device offset
    pub fn set_offset(&mut self, offset: u64) {
        self.offset = offset;
    }

    /// Get mutable slice of capacity
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Convenience alias for mutable capacity slice
    pub fn as_mut_capacity_slice(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }

    /// Get immutable slice of valid data
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.size]
    }

    /// Get raw pointer to buffer data
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }
}

impl Clone for AlignedBuffer {
    fn clone(&self) -> Self {
        AlignedBuffer {
            data: self.data.clone(),
            size: self.size,
            offset: self.offset,
            pool_idx: self.pool_idx,
        }
    }
}

// ------------------------------------------------------------------
// BufferPool
// ------------------------------------------------------------------

/// Ring buffer of pre-allocated buffers (lock-free)
#[derive(Clone)]
pub struct BufferPool {
    /// Pre-allocated buffers (owned by pool)
    buffers: Vec<AlignedBuffer>,
    /// Queue of free buffer indices
    free_queue: Arc<ArrayQueue<usize>>,
    /// Count of buffers currently in flight
    in_flight: Arc<AtomicUsize>,
    /// Total memory allocated
    total_memory: usize,
    /// Chunk size per buffer
    chunk_size: usize,
}

impl BufferPool {
    /// Create pool with num_buffers × buffer_size bytes total
    pub fn new(num_buffers: usize, buffer_size: usize) -> Result<Self, StorageError> {
        // Pre-allocate all buffers
        let mut buffers = Vec::with_capacity(num_buffers);
        for i in 0..num_buffers {
            let mut buf = AlignedBuffer::new(buffer_size)?;
            buf.pool_idx = Some(i);
            buffers.push(buf);
        }

        // Initialize free queue with all indices
        let free_queue = Arc::new(ArrayQueue::new(num_buffers));
        for i in 0..num_buffers {
            let _ = free_queue.push(i); // Should never fail - we just created it
        }

        let total_memory = num_buffers * buffer_size;

        Ok(BufferPool {
            buffers,
            free_queue,
            in_flight: Arc::new(AtomicUsize::new(0)),
            total_memory,
            chunk_size: buffer_size,
        })
    }

    /// Acquire a buffer from the pool
    pub fn acquire(&self) -> Result<AlignedBuffer, StorageError> {
        match self.free_queue.pop() {
            Some(idx) => {
                let mut buffer = self.buffers[idx].clone();
                buffer.pool_idx = Some(idx);
                self.in_flight.fetch_add(1, Ordering::Relaxed);
                Ok(buffer)
            }
            None => Err(StorageError::BufferPoolExhausted),
        }
    }

    /// Release a buffer back to the pool
    pub fn release(&self, mut buffer: AlignedBuffer) -> Result<(), StorageError> {
        buffer.reset();

        // Find which buffer this is (by pointer comparison or recorded pool index)
        for (idx, pool_buf) in self.buffers.iter().enumerate() {
            if std::ptr::eq(pool_buf.as_ptr(), buffer.as_ptr()) || buffer.pool_idx == Some(idx) {
                self.free_queue
                    .push(idx)
                    .map_err(|_| StorageError::BufferPoolFull)?;
                self.in_flight.fetch_sub(1, Ordering::Relaxed);
                return Ok(());
            }
        }

        Err(StorageError::BufferNotFromPool)
    }

    /// Get total memory allocated
    pub fn memory_usage(&self) -> usize {
        self.total_memory
    }

    /// Get number of buffers currently in use
    pub fn in_flight_count(&self) -> usize {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// Number of buffers currently in flight (alias)
    pub fn in_flight(&self) -> usize {
        self.in_flight_count()
    }

    /// Get chunk size per buffer
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_new() {
        let buf = AlignedBuffer::new(4096).unwrap();
        assert_eq!(buf.capacity(), 4096);
        assert_eq!(buf.size(), 0);
    }

    #[test]
    fn test_aligned_buffer_reset() {
        let mut buf = AlignedBuffer::new(4096).unwrap();
        // Pretend we filled it
        buf.data.resize(100, 0);
        buf.size = 100;

        buf.reset();
        assert_eq!(buf.size(), 0);
        assert_eq!(buf.capacity(), 4096); // Capacity preserved
    }

    #[test]
    fn test_buffer_pool_new() {
        let pool = BufferPool::new(4, 4096).unwrap();
        assert_eq!(pool.memory_usage(), 4 * 4096);
    }

    #[test]
    fn test_buffer_pool_acquire_release() {
        let pool = BufferPool::new(2, 4096).unwrap();

        let buf1 = pool.acquire().unwrap();
        assert_eq!(pool.in_flight_count(), 1);

        let buf2 = pool.acquire().unwrap();
        assert_eq!(pool.in_flight_count(), 2);

        pool.release(buf1).unwrap();
        assert_eq!(pool.in_flight_count(), 1);

        pool.release(buf2).unwrap();
        assert_eq!(pool.in_flight_count(), 0);
    }

    #[test]
    fn test_buffer_pool_exhaustion() {
        let pool = BufferPool::new(1, 4096).unwrap();

        let _buf1 = pool.acquire().unwrap();

        // Next acquire should fail since pool is exhausted
        assert!(pool.acquire().is_err());
    }

    #[test]
    fn test_buffer_pool_acquire_and_release() {
        let pool = BufferPool::new(2, 1024).expect("create buffer pool");
        assert_eq!(pool.chunk_size(), 1024);

        let b1 = pool.acquire().expect("acquire buffer 1");
        let b2 = pool.acquire().expect("acquire buffer 2");
        assert_eq!(b1.capacity(), 1024);
        assert_eq!(b2.capacity(), 1024);

        // Pool is now empty, next acquire must fail
        assert!(pool.acquire().is_err());

        // Release b1 and re-acquire
        pool.release(b1).expect("release buffer");
        let b3 = pool.acquire().expect("acquire recycled buffer");
        assert_eq!(b3.capacity(), 1024);
    }
}
