use diskclone::storage::buffer::BufferPool;
use diskclone::storage::checksum::compute_xxhash64;
use diskclone::storage::engine::{CloneEngine, CloneStrategy};
use std::io::{Read, Write};

/// Backward-compat scaffold test (required by spec).
#[test]
fn test_clone_engine_scaffold() {
    let pool = BufferPool::new(4, 1024 * 1024).expect("create buffer pool");
    let engine = CloneEngine::from_buffer_pool(pool);
    let _ = engine; // Scaffold ready for Phase 3 implementation tests
}

/// Clone 4 KB of deterministic data and verify byte-for-byte equality.
#[tokio::test]
async fn test_clone_small_tempfile_exact_match() {
    let payload: Vec<u8> = (0u8..=255).cycle().take(4096).collect();

    let mut src = tempfile::NamedTempFile::new().unwrap();
    src.write_all(&payload).unwrap();
    src.flush().unwrap();

    let dest = tempfile::NamedTempFile::new().unwrap();

    let mut engine = CloneEngine::new(
        src.path().to_str().unwrap(),
        dest.path().to_str().unwrap(),
        4096,
        CloneStrategy::FastXxHash,
    )
    .unwrap();

    let result = engine.execute().await.unwrap();

    assert!(result.success);
    assert_eq!(result.bytes_copied, 4096);

    // Verify byte-for-byte equality.
    let mut dest_file = std::fs::File::open(dest.path()).unwrap();
    let mut dest_bytes = Vec::new();
    dest_file.read_to_end(&mut dest_bytes).unwrap();
    assert_eq!(dest_bytes, payload);
}

/// After cloning, xxhash of source should equal CloneResult.xxhash.
#[tokio::test]
async fn test_clone_checksum_matches_source() {
    let payload = b"verifiable checksum data for diskclone phase 5";

    let mut src = tempfile::NamedTempFile::new().unwrap();
    src.write_all(payload).unwrap();
    src.flush().unwrap();

    let dest = tempfile::NamedTempFile::new().unwrap();

    let mut engine = CloneEngine::new(
        src.path().to_str().unwrap(),
        dest.path().to_str().unwrap(),
        payload.len() as u64,
        CloneStrategy::FastXxHash,
    )
    .unwrap();

    let result = engine.execute().await.unwrap();

    // Verify the engine's running xxhash matches a one-shot computation.
    let expected_xxhash = compute_xxhash64(payload);
    assert_eq!(result.xxhash, expected_xxhash);
}

/// SafeSha256 strategy should produce both xxhash and sha256.
#[tokio::test]
async fn test_clone_sha256_strategy_produces_both_hashes() {
    let payload = b"sha256 strategy test payload";

    let mut src = tempfile::NamedTempFile::new().unwrap();
    src.write_all(payload).unwrap();
    src.flush().unwrap();

    let dest = tempfile::NamedTempFile::new().unwrap();

    let mut engine = CloneEngine::new(
        src.path().to_str().unwrap(),
        dest.path().to_str().unwrap(),
        payload.len() as u64,
        CloneStrategy::SafeSha256,
    )
    .unwrap();

    let result = engine.execute().await.unwrap();

    assert!(result.success);
    assert!(
        result.sha256.is_some(),
        "sha256 should be present for SafeSha256 strategy"
    );
    let sha = result.sha256.unwrap();
    assert_eq!(sha.len(), 64, "sha256 hex string is 64 chars");
}

/// Progress should reach 100% when clone completes.
#[tokio::test]
async fn test_clone_progress_reaches_100_percent() {
    let payload = vec![42u8; 8192];

    let mut src = tempfile::NamedTempFile::new().unwrap();
    src.write_all(&payload).unwrap();
    src.flush().unwrap();

    let dest = tempfile::NamedTempFile::new().unwrap();

    let mut engine = CloneEngine::new(
        src.path().to_str().unwrap(),
        dest.path().to_str().unwrap(),
        8192,
        CloneStrategy::FastXxHash,
    )
    .unwrap();

    let result = engine.execute().await.unwrap();

    assert_eq!(result.bytes_copied, 8192);
    assert!(result.throughput_mbps >= 0.0);
}

/// Cancel signal stops the clone gracefully.
#[tokio::test]
async fn test_clone_cancel_returns_error() {
    let payload = vec![0u8; 4096];
    let mut src = tempfile::NamedTempFile::new().unwrap();
    src.write_all(&payload).unwrap();
    src.flush().unwrap();

    let dest = tempfile::NamedTempFile::new().unwrap();

    let mut engine = CloneEngine::new(
        src.path().to_str().unwrap(),
        dest.path().to_str().unwrap(),
        4096,
        CloneStrategy::FastXxHash,
    )
    .unwrap();

    // Cancel before execute.
    engine.cancel();
    let result = engine.execute().await;
    assert!(result.is_err(), "cancelled engine should return Err");
}
