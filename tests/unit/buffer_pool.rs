#[cfg(test)]
mod tests {
    use diskclone::storage::buffer::*;

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
