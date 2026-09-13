use diskclone::storage::buffer::BufferPool;

#[test]
fn test_buffer_pool_acquire_and_release() {
    let pool = BufferPool::new(2, 1024);
    assert_eq!(pool.chunk_size(), 1024);

    let b1 = pool.acquire().expect("acquire buffer 1");
    let b2 = pool.acquire().expect("acquire buffer 2");
    assert_eq!(b1.capacity(), 1024);
    assert_eq!(b2.capacity(), 1024);

    // Pool is now empty, next acquire must fail
    assert!(pool.acquire().is_err());

    // Release b1 and re-acquire
    pool.release(b1);
    let b3 = pool.acquire().expect("acquire recycled buffer");
    assert_eq!(b3.capacity(), 1024);
}
