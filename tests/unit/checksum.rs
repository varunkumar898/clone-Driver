use diskclone::storage::checksum::{compute_sha256, compute_xxhash64};

#[test]
fn test_xxhash64_consistency() {
    let data = b"diskclone-test-data-stream";
    let h1 = compute_xxhash64(data);
    let h2 = compute_xxhash64(data);
    assert_eq!(h1, h2);
    assert_ne!(h1, 0);
}

#[test]
fn test_sha256_known_vector() {
    // SHA256 of empty string is e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    let empty = b"";
    let hash = compute_sha256(empty);
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_checksum_state_xxhash_only() {
    use diskclone::storage::checksum::ChecksumState;
    let mut state = ChecksumState::new(false);
    state.update(b"diskclone");
    state.update(b"-test-data");
    let (xxhash, sha256) = state.finalize();

    assert_eq!(xxhash, compute_xxhash64(b"diskclone-test-data"));
    assert_eq!(sha256, None);
}

#[test]
fn test_checksum_state_with_sha256() {
    use diskclone::storage::checksum::ChecksumState;
    let mut state = ChecksumState::new(true);
    state.update(b"diskclone");
    state.update(b"-test-data");
    let (xxhash, sha256) = state.finalize();

    assert_eq!(xxhash, compute_xxhash64(b"diskclone-test-data"));
    assert_eq!(sha256, Some(compute_sha256(b"diskclone-test-data")));
}

#[test]
fn test_checksum_block_compute_and_verify() {
    use diskclone::storage::checksum::ChecksumBlock;
    let data = b"hello world 4096 block data";
    let block = ChecksumBlock::compute(data, 1024, true);

    assert_eq!(block.offset, 1024);
    assert_eq!(block.size, data.len());
    assert_eq!(block.xxhash, compute_xxhash64(data));
    assert_eq!(block.sha256, Some(compute_sha256(data)));

    assert!(block.verify(data));
    assert!(!block.verify(b"short"));

    let mut corrupted = data.to_vec();
    corrupted[0] ^= 0xFF;
    assert!(!block.verify(&corrupted));
}
