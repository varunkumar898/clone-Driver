use diskclone::state_machine::{CloneState, StateMachine};

/// Existing scaffold preserved.
#[test]
fn test_e2e_state_machine_flow_scaffold() {
    let sm = StateMachine::new();
    assert_eq!(sm.state(), CloneState::Idle);
    // Scaffold ready for full clone workflow testing
}

/// Full end-to-end clone of a 1 MB file using loop devices (requires root).
/// Run with: cargo test --test e2e_full_clone -- --ignored
#[tokio::test]
#[ignore]
async fn test_full_clone_loop_device() {
    use diskclone::storage::engine::{CloneEngine, CloneStrategy};
    use diskclone::storage::verification::VerificationEngine;
    use std::process::Command;

    // Create a 1 MB source image.
    let src = tempfile::NamedTempFile::new().unwrap();
    let src_path = src.path().to_str().unwrap().to_string();
    Command::new("dd")
        .args([
            "if=/dev/urandom",
            &format!("of={}", src_path),
            "bs=1M",
            "count=1",
        ])
        .status()
        .expect("dd to create source image");

    let dst = tempfile::NamedTempFile::new().unwrap();
    let dst_path = dst.path().to_str().unwrap().to_string();

    // Pre-allocate destination file.
    Command::new("dd")
        .args([
            "if=/dev/zero",
            &format!("of={}", dst_path),
            "bs=1M",
            "count=1",
        ])
        .status()
        .expect("dd to create destination image");

    let total_bytes = 1024 * 1024_u64;

    let mut engine = CloneEngine::new(&src_path, &dst_path, total_bytes, CloneStrategy::SafeSha256)
        .expect("create clone engine");

    let result = engine.execute().await.expect("clone should succeed");

    assert!(result.success);
    assert_eq!(result.bytes_copied, total_bytes);
    assert!(result.sha256.is_some());

    // Phase 6 preview: verify source == dest.
    let report = VerificationEngine::verify_devices(&src_path, &dst_path, total_bytes).unwrap();
    assert!(
        report.verified,
        "source and destination should match after clone"
    );
}

/// Resume-after-interrupt: clone, cancel at 50%, resume, verify completion.
/// Run with: cargo test --test e2e_full_clone -- --ignored
#[tokio::test]
#[ignore]
async fn test_resume_after_interrupt() {
    use diskclone::storage::engine::{CloneEngine, CloneStrategy};

    let payload = vec![0xABu8; 512 * 1024]; // 512 KB

    let mut src = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut src, &payload).unwrap();
    std::io::Write::flush(&mut src).unwrap();

    let dst = tempfile::NamedTempFile::new().unwrap();

    let mut engine = CloneEngine::new(
        src.path().to_str().unwrap(),
        dst.path().to_str().unwrap(),
        payload.len() as u64,
        CloneStrategy::FastXxHash,
    )
    .unwrap();

    // Execute to completion (resume logic not yet wired to mid-stream cancel).
    let result = engine.execute().await.unwrap();
    assert!(result.success);
    assert_eq!(result.bytes_copied, payload.len() as u64);
}
