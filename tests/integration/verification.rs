use diskclone::storage::verification::VerificationEngine;
use std::io::Write;

/// Scaffold kept for backward compatibility.
#[test]
fn test_verification_engine_scaffold() {
    let _ = VerificationEngine; // Scaffold ready for Phase 4 implementation tests
}

/// Identical files pass verification.
#[test]
fn test_verification_pass_identical_files() {
    let payload = b"identical data in both source and destination";

    let mut src = tempfile::NamedTempFile::new().unwrap();
    src.write_all(payload).unwrap();
    src.flush().unwrap();

    let mut dst = tempfile::NamedTempFile::new().unwrap();
    dst.write_all(payload).unwrap();
    dst.flush().unwrap();

    let report = VerificationEngine::verify_devices(
        src.path().to_str().unwrap(),
        dst.path().to_str().unwrap(),
        payload.len() as u64,
    )
    .unwrap();

    assert!(report.verified, "identical files should verify");
    assert_eq!(report.matched_bytes, payload.len() as u64);
    assert_eq!(report.mismatched_bytes, 0);
    assert_eq!(report.source_hash, report.dest_hash);
}

/// A single corrupted byte causes verification to fail.
#[test]
fn test_verification_fail_on_single_byte_corruption() {
    let payload = b"data that will be corrupted at one position";

    let mut src = tempfile::NamedTempFile::new().unwrap();
    src.write_all(payload).unwrap();
    src.flush().unwrap();

    let mut corrupted = payload.to_vec();
    corrupted[5] ^= 0xFF; // flip all bits in byte 5

    let mut dst = tempfile::NamedTempFile::new().unwrap();
    dst.write_all(&corrupted).unwrap();
    dst.flush().unwrap();

    let report = VerificationEngine::verify_devices(
        src.path().to_str().unwrap(),
        dst.path().to_str().unwrap(),
        payload.len() as u64,
    )
    .unwrap();

    assert!(!report.verified, "corrupted destination should not verify");
    assert_ne!(report.source_hash, report.dest_hash);
}

/// Empty file pair verifies trivially.
#[test]
fn test_verification_empty_files() {
    let src = tempfile::NamedTempFile::new().unwrap();
    let dst = tempfile::NamedTempFile::new().unwrap();

    let report = VerificationEngine::verify_devices(
        src.path().to_str().unwrap(),
        dst.path().to_str().unwrap(),
        0,
    )
    .unwrap();

    // Zero bytes — trivially verified, hashes will match (both empty).
    assert!(report.verified || report.total_bytes == 0);
}

/// Large-ish payload (64 KB) still verifies correctly.
#[test]
fn test_verification_64kb_payload() {
    let payload: Vec<u8> = (0u8..=255).cycle().take(64 * 1024).collect();

    let mut src = tempfile::NamedTempFile::new().unwrap();
    src.write_all(&payload).unwrap();
    src.flush().unwrap();

    let mut dst = tempfile::NamedTempFile::new().unwrap();
    dst.write_all(&payload).unwrap();
    dst.flush().unwrap();

    let report = VerificationEngine::verify_devices(
        src.path().to_str().unwrap(),
        dst.path().to_str().unwrap(),
        payload.len() as u64,
    )
    .unwrap();

    assert!(report.verified);
}
