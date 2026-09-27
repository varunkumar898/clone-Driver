//! Comprehensive Unit Tests for Phase 7: Resume/Recovery
//!
//! Validates:
//! - Checkpoint loading, validation, and planning
//! - Device serial verification (source and destination)
//! - 24-hour retention window enforcement
//! - Chunk alignment and progress calculation
//! - First-block xxHash64 hardware identity check
//! - Error handling for missing, corrupted, or invalid journals
//! - Atomic updates and cleanup

use diskclone::device::properties::DeviceProperties;
use diskclone::device::BlockDevice;
use diskclone::error::{CloneError, ResumeError};
use diskclone::resume::{
    CheckpointManager, RecoveryCoordinator, RecoveryPlan, ResumeJournal, ResumeRecord,
    MAX_CHECKPOINT_AGE_SECS,
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::TempDir;
use xxhash_rust::xxh64::xxh64;

fn get_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn create_mock_device(serial: &str, size_bytes: u64, path: PathBuf) -> BlockDevice {
    BlockDevice {
        path,
        sysfs_name: serial.to_string(),
        size_bytes,
        properties: DeviceProperties {
            serial_number: serial.to_string(),
            model: format!("Model-{serial}"),
            vendor: "AcmeDisk".to_string(),
            ..Default::default()
        },
        is_system_disk: false,
        is_mounted: false,
        partition_table: None,
    }
}

fn create_sample_record(
    src_serial: &str,
    dst_serial: &str,
    total_bytes: u64,
    offset: u64,
    age_secs: u64,
    hash: u64,
) -> ResumeRecord {
    let now = get_now_secs();
    ResumeRecord {
        source_serial: src_serial.to_string(),
        dest_serial: dst_serial.to_string(),
        source_size_bytes: total_bytes,
        last_completed_offset: offset,
        chunk_size: 4 * 1024 * 1024,
        first_block_xxhash: hash,
        timestamp_epoch_secs: now.saturating_sub(age_secs),
    }
}

// ── Test 1: Successful Recovery Plan Generation ─────────────────────────────
#[test]
fn test_recovery_valid_checkpoint_plan_creation() {
    let dir = TempDir::new().unwrap();
    let journal_path = dir.path().join("clone_journal.json");

    let record = create_sample_record(
        "WD-WCC4N1234567",
        "ST-9876543210AB",
        100 * 1024 * 1024 * 1024, // 100 GB
        40 * 1024 * 1024 * 1024,  // 40 GB
        1800,                     // 30 mins old
        0xFEEDFACECAFEBABE,
    );
    ResumeJournal::save(&journal_path, &record).unwrap();

    let src = create_mock_device(
        "WD-WCC4N1234567",
        100 * 1024 * 1024 * 1024,
        PathBuf::from("/dev/sdb"),
    );
    let dst = create_mock_device(
        "ST-9876543210AB",
        120 * 1024 * 1024 * 1024,
        PathBuf::from("/dev/sdc"),
    );

    let plan = RecoveryCoordinator::prepare_recovery_with_devices(&journal_path, &src, &dst)
        .expect("Valid checkpoint must produce a plan");

    assert_eq!(plan.source_serial, "WD-WCC4N1234567");
    assert_eq!(plan.dest_serial, "ST-9876543210AB");
    assert_eq!(plan.total_bytes, 100 * 1024 * 1024 * 1024);
    assert_eq!(plan.resumed_offset, 40 * 1024 * 1024 * 1024);
    assert_eq!(plan.bytes_remaining, 60 * 1024 * 1024 * 1024);
    assert!((plan.percent_complete - 40.0).abs() < 0.01);
    assert!(!plan.is_completed());
}

// ── Test 2: Hardware Serial Matching Success ────────────────────────────────
#[test]
fn test_recovery_device_serial_verification_success() {
    let record = create_sample_record("SRC_ABC", "DST_XYZ", 1_000_000, 500_000, 60, 0x1234);
    let result = RecoveryCoordinator::validate_device_serials(&record, "SRC_ABC", "DST_XYZ");
    assert!(result.is_ok());
}

// ── Test 3: Rejection on Source Device Serial Mismatch ───────────────────────
#[test]
fn test_recovery_fails_on_source_serial_mismatch() {
    let dir = TempDir::new().unwrap();
    let journal_path = dir.path().join("resume.json");
    let record = create_sample_record("SERIAL_A", "SERIAL_B", 10_000_000, 2_000_000, 120, 0x11);
    ResumeJournal::save(&journal_path, &record).unwrap();

    let wrong_src = create_mock_device("SERIAL_WRONG", 10_000_000, PathBuf::from("/dev/sdb"));
    let dst = create_mock_device("SERIAL_B", 10_000_000, PathBuf::from("/dev/sdc"));

    let err = RecoveryCoordinator::prepare_recovery_with_devices(&journal_path, &wrong_src, &dst)
        .unwrap_err();

    match err {
        CloneError::Resume(ResumeError::SerialMismatch(msg)) => {
            assert!(msg.contains("Source serial mismatch"));
            assert!(msg.contains("SERIAL_A"));
            assert!(msg.contains("SERIAL_WRONG"));
        }
        other => panic!("Expected SerialMismatch, got: {other:?}"),
    }
}

// ── Test 4: Rejection on Destination Device Serial Mismatch ─────────────────
#[test]
fn test_recovery_fails_on_dest_serial_mismatch() {
    let dir = TempDir::new().unwrap();
    let journal_path = dir.path().join("resume.json");
    let record = create_sample_record("SERIAL_A", "SERIAL_B", 10_000_000, 2_000_000, 120, 0x11);
    ResumeJournal::save(&journal_path, &record).unwrap();

    let src = create_mock_device("SERIAL_A", 10_000_000, PathBuf::from("/dev/sdb"));
    let wrong_dst = create_mock_device("SERIAL_TAMPERED", 10_000_000, PathBuf::from("/dev/sdc"));

    let err = RecoveryCoordinator::prepare_recovery_with_devices(&journal_path, &src, &wrong_dst)
        .unwrap_err();

    match err {
        CloneError::Resume(ResumeError::SerialMismatch(msg)) => {
            assert!(msg.contains("Destination serial mismatch"));
            assert!(msg.contains("SERIAL_B"));
            assert!(msg.contains("SERIAL_TAMPERED"));
        }
        other => panic!("Expected SerialMismatch, got: {other:?}"),
    }
}

// ── Test 5: Checkpoint Retention Valid Within 24 Hours ───────────────────────
#[test]
fn test_recovery_checkpoint_retention_valid_within_24h() {
    let record_1h = create_sample_record("S1", "D1", 1000, 500, 3600, 0);
    assert!(!RecoveryCoordinator::is_checkpoint_expired(
        &record_1h,
        MAX_CHECKPOINT_AGE_SECS,
        None
    ));

    let record_23h = create_sample_record("S1", "D1", 1000, 500, 23 * 3600, 0);
    assert!(!RecoveryCoordinator::is_checkpoint_expired(
        &record_23h,
        MAX_CHECKPOINT_AGE_SECS,
        None
    ));
}

// ── Test 6: Checkpoint Expiration After 24 Hours ────────────────────────────
#[test]
fn test_recovery_checkpoint_retention_expired_after_24h() {
    let dir = TempDir::new().unwrap();
    let journal_path = dir.path().join("expired.json");

    // 25 hours old (90,000 seconds)
    let record = create_sample_record("S1", "D1", 1000, 500, 90_000, 0);
    ResumeJournal::save(&journal_path, &record).unwrap();

    let src = create_mock_device("S1", 1000, PathBuf::from("/dev/sdb"));
    let dst = create_mock_device("D1", 2000, PathBuf::from("/dev/sdc"));

    let err =
        RecoveryCoordinator::prepare_recovery_with_devices(&journal_path, &src, &dst).unwrap_err();

    match err {
        CloneError::Resume(ResumeError::CheckpointExpired(msg)) => {
            assert!(msg.contains("Checkpoint is"));
            assert!(msg.contains("max allowed"));
        }
        other => panic!("Expected CheckpointExpired, got: {other:?}"),
    }
}

// ── Test 7: Exact Retention Boundary Check ──────────────────────────────────
#[test]
fn test_recovery_retention_boundary_exact_second() {
    let now = 1_000_000;
    let mut record = create_sample_record("S1", "D1", 1000, 500, 0, 0);

    // Exactly 86,400s (24h) -> NOT expired
    record.timestamp_epoch_secs = now - 86_400;
    assert!(!RecoveryCoordinator::is_checkpoint_expired(
        &record,
        MAX_CHECKPOINT_AGE_SECS,
        Some(now)
    ));

    // 86,401s (24h + 1s) -> EXPIRED
    record.timestamp_epoch_secs = now - 86_401;
    assert!(RecoveryCoordinator::is_checkpoint_expired(
        &record,
        MAX_CHECKPOINT_AGE_SECS,
        Some(now)
    ));
}

// ── Test 8: Atomic Progress Update and Persistence ──────────────────────────
#[test]
fn test_recovery_atomic_progress_update_and_persistence() {
    let dir = TempDir::new().unwrap();
    let journal_path = dir.path().join("atomic_checkpoint.json");

    let record = create_sample_record("SRC_DEV", "DST_DEV", 10_000_000, 0, 10, 0xABCD);
    let mut manager = CheckpointManager::new(journal_path.clone(), record);

    // Commit 1
    RecoveryCoordinator::update_checkpoint(&mut manager, 2_000_000).unwrap();
    let loaded1 = ResumeJournal::load(&journal_path).unwrap();
    assert_eq!(loaded1.last_completed_offset, 2_000_000);

    // Commit 2
    RecoveryCoordinator::update_checkpoint(&mut manager, 6_000_000).unwrap();
    let loaded2 = ResumeJournal::load(&journal_path).unwrap();
    assert_eq!(loaded2.last_completed_offset, 6_000_000);
}

// ── Test 9: Corrupted Journal JSON Rejection ─────────────────────────────────
#[test]
fn test_recovery_corrupted_journal_fails_gracefully() {
    let dir = TempDir::new().unwrap();
    let journal_path = dir.path().join("bad_journal.json");
    std::fs::write(&journal_path, b"{{INVALID_JSON_CORRUPT").unwrap();

    let src = create_mock_device("S1", 1000, PathBuf::from("/dev/sdb"));
    let dst = create_mock_device("D1", 2000, PathBuf::from("/dev/sdc"));

    let err =
        RecoveryCoordinator::prepare_recovery_with_devices(&journal_path, &src, &dst).unwrap_err();

    match err {
        CloneError::Resume(ResumeError::CorruptedJournal(_)) => {}
        other => panic!("Expected CorruptedJournal, got: {other:?}"),
    }
}

// ── Test 10: Missing Journal Rejection ───────────────────────────────────────
#[test]
fn test_recovery_missing_journal_fails_gracefully() {
    let dir = TempDir::new().unwrap();
    let missing_path = dir.path().join("does_not_exist.json");

    let src = create_mock_device("S1", 1000, PathBuf::from("/dev/sdb"));
    let dst = create_mock_device("D1", 2000, PathBuf::from("/dev/sdc"));

    let err =
        RecoveryCoordinator::prepare_recovery_with_devices(&missing_path, &src, &dst).unwrap_err();

    match err {
        CloneError::Resume(ResumeError::JournalNotFound(_)) => {}
        other => panic!("Expected JournalNotFound, got: {other:?}"),
    }
}

// ── Test 11: Source Device Size Mismatch ─────────────────────────────────────
#[test]
fn test_recovery_source_size_mismatch_fails() {
    let dir = TempDir::new().unwrap();
    let journal_path = dir.path().join("resume.json");
    let record = create_sample_record("S1", "D1", 50_000_000, 10_000_000, 10, 0);
    ResumeJournal::save(&journal_path, &record).unwrap();

    let altered_src = create_mock_device("S1", 60_000_000, PathBuf::from("/dev/sdb"));
    let dst = create_mock_device("D1", 100_000_000, PathBuf::from("/dev/sdc"));

    let err = RecoveryCoordinator::prepare_recovery_with_devices(&journal_path, &altered_src, &dst)
        .unwrap_err();

    match err {
        CloneError::Resume(ResumeError::SizeMismatch { expected, found }) => {
            assert_eq!(expected, 50_000_000);
            assert_eq!(found, 60_000_000);
        }
        other => panic!("Expected SizeMismatch, got: {other:?}"),
    }
}

// ── Test 12: Destination Device Capacity Insufficient ───────────────────────
#[test]
fn test_recovery_dest_capacity_insufficient_fails() {
    let dir = TempDir::new().unwrap();
    let journal_path = dir.path().join("resume.json");
    let record = create_sample_record("S1", "D1", 50_000_000, 10_000_000, 10, 0);
    ResumeJournal::save(&journal_path, &record).unwrap();

    let src = create_mock_device("S1", 50_000_000, PathBuf::from("/dev/sdb"));
    let undersized_dst = create_mock_device("D1", 30_000_000, PathBuf::from("/dev/sdc"));

    let err =
        RecoveryCoordinator::prepare_recovery_with_devices(&journal_path, &src, &undersized_dst)
            .unwrap_err();

    match err {
        CloneError::Resume(ResumeError::DeviceTooSmall {
            required,
            available,
        }) => {
            assert_eq!(required, 50_000_000);
            assert_eq!(available, 30_000_000);
        }
        other => panic!("Expected DeviceTooSmall, got: {other:?}"),
    }
}

// ── Test 13: Checkpoint Offset Exceeds Source Size ──────────────────────────
#[test]
fn test_recovery_offset_exceeds_total_size_fails() {
    let mut record = create_sample_record("S1", "D1", 1_000_000, 2_000_000, 10, 0);
    record.last_completed_offset = 2_000_000; // Greater than total_bytes

    let err = RecoveryCoordinator::validate_record(&record, None).unwrap_err();
    match err {
        CloneError::Resume(ResumeError::InvalidOffset(_)) => {}
        other => panic!("Expected InvalidOffset, got: {other:?}"),
    }
}

// ── Test 14: First-Block xxHash64 Identity Verification Passes ───────────────
#[test]
fn test_recovery_first_block_xxhash_verification_success() {
    let dir = TempDir::new().unwrap();
    let test_file = dir.path().join("mock_block_device.bin");

    let chunk_size = 4096;
    let data = vec![0xABu8; chunk_size];
    let mut file = File::create(&test_file).unwrap();
    file.write_all(&data).unwrap();

    let expected_hash = xxh64(&data, 0);

    let matches = RecoveryCoordinator::verify_first_block(&test_file, expected_hash, chunk_size)
        .expect("Block read should succeed");
    assert!(matches, "Hash of first block should match");
}

// ── Test 15: First-Block xxHash64 Identity Verification Fails ────────────────
#[test]
fn test_recovery_first_block_xxhash_verification_failure() {
    let dir = TempDir::new().unwrap();
    let test_file = dir.path().join("mock_block_device.bin");

    let chunk_size = 4096;
    let data = vec![0xCDu8; chunk_size];
    let mut file = File::create(&test_file).unwrap();
    file.write_all(&data).unwrap();

    let expected_hash = 0x12345678_87654321; // intentionally wrong
    let matches = RecoveryCoordinator::verify_first_block(&test_file, expected_hash, chunk_size)
        .expect("Block read should succeed");
    assert!(!matches, "Hash must not match corrupted/changed block");
}

// ── Test 16: Journal Cleanup Removes Checkpoint and Temp Files ──────────────
#[test]
fn test_recovery_cleanup_journal_removes_checkpoint() {
    let dir = TempDir::new().unwrap();
    let journal_path = dir.path().join("cleanup_journal.json");
    let tmp_path = journal_path.with_extension("tmp");

    std::fs::write(&journal_path, b"test").unwrap();
    std::fs::write(&tmp_path, b"temp").unwrap();

    assert!(journal_path.exists());
    assert!(tmp_path.exists());

    RecoveryCoordinator::cleanup_journal(&journal_path).unwrap();

    assert!(!journal_path.exists());
    assert!(!tmp_path.exists());
}

// ── Test 17: Chunk Alignment and Rounding ───────────────────────────────────
#[test]
fn test_recovery_chunk_alignment_and_rounding() {
    let mut record = create_sample_record("S", "D", 20_000_000, 15_000_000, 10, 0);
    record.chunk_size = 4 * 1024 * 1024; // 4,194,304 bytes

    // 15,000,000 / 4,194,304 = 3 chunks (12,582,912 bytes)
    let offset = RecoveryCoordinator::calculate_resume_offset(&record).unwrap();
    assert_eq!(offset, 3 * 4 * 1024 * 1024);
}

// ── Test 18: Percentage and Chunks Progress Calculations ────────────────────
#[test]
fn test_recovery_percentage_and_chunks_calculation() {
    let plan = RecoveryPlan {
        source_serial: "S".into(),
        dest_serial: "D".into(),
        total_bytes: 10 * 1024 * 1024,
        resumed_offset: 7 * 1024 * 1024,
        bytes_remaining: 3 * 1024 * 1024,
        percent_complete: 70.0,
        chunk_size: 1024 * 1024,
        checkpoint_age_secs: 120,
        first_block_xxhash: 0,
    };

    assert_eq!(plan.chunks_completed(), 7);
    assert_eq!(plan.chunks_remaining(), 3);
    assert!((plan.percent_complete - 70.0).abs() < f64::EPSILON);
    assert!(!plan.is_completed());
}
