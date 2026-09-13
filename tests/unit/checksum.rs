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
