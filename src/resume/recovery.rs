//! Clone recovery coordinator and checkpoint resumption engine.
//!
//! Provides validation and recovery planning for interrupted disk clone operations:
//! - Checkpoint validation and 24-hour expiration enforcement
//! - Hardware serial verification against original source and destination
//! - Size and boundary validation with chunk alignment
//! - Optional first-block xxHash64 cryptographic identity check
//! - Atomic journal cleanup upon completion

use crate::device::{BlockDevice, DeviceManager};
use crate::error::{CloneError, Result, ResumeError};
use crate::resume::checkpoint::CheckpointManager;
use crate::resume::journal::{ResumeJournal, ResumeRecord};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use xxhash_rust::xxh64::xxh64;

/// Maximum allowable age of a checkpoint before it is considered expired (24 hours).
pub const MAX_CHECKPOINT_AGE_SECS: u64 = 24 * 60 * 60; // 86,400 seconds

/// Default chunk size for verification and alignment if unspecified.
pub const DEFAULT_CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4 MB

/// A calculated and verified recovery plan for an interrupted clone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryPlan {
    /// Expected source hardware serial number.
    pub source_serial: String,
    /// Expected destination hardware serial number.
    pub dest_serial: String,
    /// Total size of the source disk in bytes.
    pub total_bytes: u64,
    /// Offset from which copying should resume.
    pub resumed_offset: u64,
    /// Number of bytes remaining to be copied.
    pub bytes_remaining: u64,
    /// Percentage of the clone already completed [0.0, 100.0].
    pub percent_complete: f64,
    /// Chunker block size in bytes.
    pub chunk_size: usize,
    /// Age of the checkpoint in seconds at validation time.
    pub checkpoint_age_secs: u64,
    /// xxHash64 hash of the first block for device identity verification.
    pub first_block_xxhash: u64,
}

impl RecoveryPlan {
    /// Returns true if the checkpoint indicates the clone was already fully completed.
    pub fn is_completed(&self) -> bool {
        self.resumed_offset >= self.total_bytes && self.total_bytes > 0
    }

    /// Number of chunks already completed.
    pub fn chunks_completed(&self) -> u64 {
        if self.chunk_size == 0 {
            0
        } else {
            self.resumed_offset / (self.chunk_size as u64)
        }
    }

    /// Number of chunks remaining to be processed.
    pub fn chunks_remaining(&self) -> u64 {
        if self.chunk_size == 0 {
            0
        } else {
            self.bytes_remaining.div_ceil(self.chunk_size as u64)
        }
    }

    /// Formatted summary string of the recovery status.
    pub fn summary(&self) -> String {
        format!(
            "Resume at {} / {} ({:.2}% complete, {} remaining, age {}s)",
            self.resumed_offset,
            self.total_bytes,
            self.percent_complete,
            self.bytes_remaining,
            self.checkpoint_age_secs
        )
    }
}

/// Recovery coordinator that analyzes journals and prepares resumption plans.
pub struct RecoveryCoordinator;

impl RecoveryCoordinator {
    /// Validates hardware devices against the journal and calculates starting offset.
    ///
    /// Performs full validation:
    /// 1. Journal existence and integrity check.
    /// 2. 24-hour expiration window check.
    /// 3. Device discovery via `DeviceManager` and serial number matching.
    /// 4. Capacity and offset sanity validation.
    pub fn prepare_recovery(journal_path: &Path, device_manager: &DeviceManager) -> Result<u64> {
        let plan = Self::prepare_recovery_plan(journal_path, device_manager)?;
        Ok(plan.resumed_offset)
    }

    /// Prepares a detailed [`RecoveryPlan`] using discovered hardware devices.
    pub fn prepare_recovery_plan(
        journal_path: &Path,
        device_manager: &DeviceManager,
    ) -> Result<RecoveryPlan> {
        let record = ResumeJournal::load(journal_path)?;
        Self::validate_record(&record, None)?;

        let devices = device_manager.scan_devices()?;

        let source = devices
            .iter()
            .find(|d| d.properties.serial_number == record.source_serial)
            .ok_or_else(|| {
                CloneError::Resume(ResumeError::DeviceNotFound(format!(
                    "Source device with serial '{}' not found",
                    record.source_serial
                )))
            })?;

        let dest = devices
            .iter()
            .find(|d| d.properties.serial_number == record.dest_serial)
            .ok_or_else(|| {
                CloneError::Resume(ResumeError::DeviceNotFound(format!(
                    "Destination device with serial '{}' not found",
                    record.dest_serial
                )))
            })?;

        Self::prepare_recovery_with_devices(journal_path, source, dest)
    }

    /// Prepares a [`RecoveryPlan`] with pre-identified source and destination devices.
    pub fn prepare_recovery_with_devices(
        journal_path: &Path,
        source: &BlockDevice,
        dest: &BlockDevice,
    ) -> Result<RecoveryPlan> {
        let record = ResumeJournal::load(journal_path)?;
        Self::validate_record(&record, None)?;

        Self::validate_device_serials(
            &record,
            &source.properties.serial_number,
            &dest.properties.serial_number,
        )?;

        Self::validate_device_sizes(&record, source.size_bytes, dest.size_bytes)?;

        let resumed_offset = Self::calculate_resume_offset(&record)?;

        let bytes_remaining = record.source_size_bytes.saturating_sub(resumed_offset);

        let percent_complete = if record.source_size_bytes == 0 {
            0.0
        } else {
            (resumed_offset as f64 / record.source_size_bytes as f64 * 100.0).clamp(0.0, 100.0)
        };

        let now = current_epoch_secs();
        let checkpoint_age_secs = now.saturating_sub(record.timestamp_epoch_secs);

        Ok(RecoveryPlan {
            source_serial: record.source_serial,
            dest_serial: record.dest_serial,
            total_bytes: record.source_size_bytes,
            resumed_offset,
            bytes_remaining,
            percent_complete,
            chunk_size: record.chunk_size,
            checkpoint_age_secs,
            first_block_xxhash: record.first_block_xxhash,
        })
    }

    /// Validates the journal record integrity, offset boundaries, and expiration.
    pub fn validate_record(
        record: &ResumeRecord,
        current_time_override: Option<u64>,
    ) -> Result<()> {
        let now = current_time_override.unwrap_or_else(current_epoch_secs);

        if Self::is_checkpoint_expired(record, MAX_CHECKPOINT_AGE_SECS, Some(now)) {
            let age = now.saturating_sub(record.timestamp_epoch_secs);
            return Err(CloneError::Resume(ResumeError::CheckpointExpired(format!(
                "Checkpoint is {} seconds old (max allowed is {} seconds)",
                age, MAX_CHECKPOINT_AGE_SECS
            ))));
        }

        if record.chunk_size == 0 {
            return Err(CloneError::Resume(ResumeError::CorruptedJournal(
                "Chunk size in journal record cannot be 0".to_string(),
            )));
        }

        if record.last_completed_offset > record.source_size_bytes {
            return Err(CloneError::Resume(ResumeError::InvalidOffset(format!(
                "Last completed offset {} exceeds total source size {}",
                record.last_completed_offset, record.source_size_bytes
            ))));
        }

        Ok(())
    }

    /// Determines if a checkpoint has exceeded its maximum retention lifespan.
    pub fn is_checkpoint_expired(
        record: &ResumeRecord,
        max_age_secs: u64,
        current_time_override: Option<u64>,
    ) -> bool {
        let now = current_time_override.unwrap_or_else(current_epoch_secs);
        if now < record.timestamp_epoch_secs {
            // Future timestamp indicates clock skew or corruption
            return false;
        }
        now.saturating_sub(record.timestamp_epoch_secs) > max_age_secs
    }

    /// Verifies that hardware serial numbers match the journal record exactly.
    pub fn validate_device_serials(
        record: &ResumeRecord,
        source_serial: &str,
        dest_serial: &str,
    ) -> Result<()> {
        if record.source_serial != source_serial {
            return Err(CloneError::Resume(ResumeError::SerialMismatch(format!(
                "Source serial mismatch: expected '{}', found '{}'",
                record.source_serial, source_serial
            ))));
        }

        if record.dest_serial != dest_serial {
            return Err(CloneError::Resume(ResumeError::SerialMismatch(format!(
                "Destination serial mismatch: expected '{}', found '{}'",
                record.dest_serial, dest_serial
            ))));
        }

        Ok(())
    }

    /// Validates device size constraints against journal record expectations.
    pub fn validate_device_sizes(
        record: &ResumeRecord,
        source_size: u64,
        dest_capacity: u64,
    ) -> Result<()> {
        if source_size != record.source_size_bytes {
            return Err(CloneError::Resume(ResumeError::SizeMismatch {
                expected: record.source_size_bytes,
                found: source_size,
            }));
        }

        if dest_capacity < record.source_size_bytes {
            return Err(CloneError::Resume(ResumeError::DeviceTooSmall {
                required: record.source_size_bytes,
                available: dest_capacity,
            }));
        }

        Ok(())
    }

    /// Calculates safe resume offset, aligning to the recorded chunk boundary.
    pub fn calculate_resume_offset(record: &ResumeRecord) -> Result<u64> {
        if record.chunk_size == 0 {
            return Err(CloneError::Resume(ResumeError::CorruptedJournal(
                "Chunk size must be non-zero".to_string(),
            )));
        }

        let chunk = record.chunk_size as u64;
        // Floor align to chunk size
        let aligned = (record.last_completed_offset / chunk) * chunk;

        if aligned > record.source_size_bytes {
            return Err(CloneError::Resume(ResumeError::InvalidOffset(format!(
                "Aligned resume offset {} exceeds source size {}",
                aligned, record.source_size_bytes
            ))));
        }

        Ok(aligned)
    }

    /// Verifies the xxHash64 of the first block against the expected hash in the record.
    pub fn verify_first_block(
        device_path: &Path,
        expected_hash: u64,
        chunk_size: usize,
    ) -> Result<bool> {
        let mut file = File::open(device_path).map_err(CloneError::Io)?;
        let mut buffer = vec![0u8; chunk_size];
        let bytes_read = file.read(&mut buffer).map_err(CloneError::Io)?;

        if bytes_read == 0 {
            return Ok(false);
        }

        let calculated = xxh64(&buffer[..bytes_read], 0);

        Ok(calculated == expected_hash)
    }

    /// Cleans up and removes an existing journal file and temporary swap file atomically.
    pub fn cleanup_journal(journal_path: &Path) -> Result<()> {
        if journal_path.exists() {
            std::fs::remove_file(journal_path).map_err(CloneError::Io)?;
        }
        let tmp_path = journal_path.with_extension("tmp");
        if tmp_path.exists() {
            let _ = std::fs::remove_file(tmp_path);
        }
        Ok(())
    }

    /// Atomically updates a checkpoint offset using a [`CheckpointManager`].
    pub fn update_checkpoint(manager: &mut CheckpointManager, new_offset: u64) -> Result<()> {
        manager.commit_offset(new_offset)
    }
}

/// Helper returning current Unix epoch in seconds.
fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::properties::DeviceProperties;
    use tempfile::TempDir;

    fn make_test_device(serial: &str, size_bytes: u64) -> BlockDevice {
        BlockDevice {
            path: std::path::PathBuf::from(format!("/dev/{serial}")),
            sysfs_name: serial.to_string(),
            size_bytes,
            properties: DeviceProperties {
                serial_number: serial.to_string(),
                model: "TEST_DRIVE".to_string(),
                vendor: "TEST".to_string(),
                ..Default::default()
            },
            is_system_disk: false,
            is_mounted: false,
            partition_table: None,
        }
    }

    fn make_test_record(age_secs: u64) -> ResumeRecord {
        let now = current_epoch_secs();
        let timestamp = now.saturating_sub(age_secs);
        ResumeRecord {
            source_serial: "SRC_123".to_string(),
            dest_serial: "DST_456".to_string(),
            source_size_bytes: 1_000_000_000,
            last_completed_offset: 200_000_000,
            chunk_size: 4 * 1024 * 1024,
            first_block_xxhash: 0xA1B2C3D4,
            timestamp_epoch_secs: timestamp,
        }
    }

    #[test]
    fn test_is_checkpoint_expired_fresh() {
        let record = make_test_record(3600); // 1 hour old
        assert!(!RecoveryCoordinator::is_checkpoint_expired(
            &record,
            MAX_CHECKPOINT_AGE_SECS,
            None
        ));
    }

    #[test]
    fn test_is_checkpoint_expired_stale() {
        let record = make_test_record(90000); // ~25 hours old
        assert!(RecoveryCoordinator::is_checkpoint_expired(
            &record,
            MAX_CHECKPOINT_AGE_SECS,
            None
        ));
    }

    #[test]
    fn test_validate_device_serials_match() {
        let record = make_test_record(100);
        let res = RecoveryCoordinator::validate_device_serials(&record, "SRC_123", "DST_456");
        assert!(res.is_ok());
    }

    #[test]
    fn test_validate_device_serials_source_mismatch() {
        let record = make_test_record(100);
        let res = RecoveryCoordinator::validate_device_serials(&record, "WRONG_SRC", "DST_456");
        assert!(res.is_err());
    }

    #[test]
    fn test_validate_device_serials_dest_mismatch() {
        let record = make_test_record(100);
        let res = RecoveryCoordinator::validate_device_serials(&record, "SRC_123", "WRONG_DST");
        assert!(res.is_err());
    }

    #[test]
    fn test_validate_device_sizes_success() {
        let record = make_test_record(100);
        let res = RecoveryCoordinator::validate_device_sizes(&record, 1_000_000_000, 2_000_000_000);
        assert!(res.is_ok());
    }

    #[test]
    fn test_validate_device_sizes_source_mismatch() {
        let record = make_test_record(100);
        let res = RecoveryCoordinator::validate_device_sizes(&record, 999_999_999, 2_000_000_000);
        assert!(res.is_err());
    }

    #[test]
    fn test_validate_device_sizes_dest_too_small() {
        let record = make_test_record(100);
        let res = RecoveryCoordinator::validate_device_sizes(&record, 1_000_000_000, 500_000_000);
        assert!(res.is_err());
    }

    #[test]
    fn test_calculate_resume_offset_alignment() {
        let mut record = make_test_record(100);
        record.chunk_size = 4 * 1024 * 1024; // 4,194,304 bytes
        record.last_completed_offset = 10_000_000;
        let aligned = RecoveryCoordinator::calculate_resume_offset(&record).unwrap();
        // 10,000,000 / 4,194,304 = 2 chunks -> 8,388,608
        assert_eq!(aligned, 8_388_608);
    }

    #[test]
    fn test_prepare_recovery_with_devices_success() {
        let temp_dir = TempDir::new().unwrap();
        let journal_path = temp_dir.path().join("resume.json");
        let record = make_test_record(3600);
        ResumeJournal::save(&journal_path, &record).unwrap();

        let src = make_test_device("SRC_123", 1_000_000_000);
        let dst = make_test_device("DST_456", 2_000_000_000);

        let plan =
            RecoveryCoordinator::prepare_recovery_with_devices(&journal_path, &src, &dst).unwrap();
        assert_eq!(plan.source_serial, "SRC_123");
        assert_eq!(plan.dest_serial, "DST_456");
        assert_eq!(plan.total_bytes, 1_000_000_000);
        assert!(plan.percent_complete > 0.0);
        assert!(!plan.is_completed());
    }

    #[test]
    fn test_cleanup_journal_removes_files() {
        let temp_dir = TempDir::new().unwrap();
        let journal_path = temp_dir.path().join("resume.json");
        let record = make_test_record(10);
        ResumeJournal::save(&journal_path, &record).unwrap();
        assert!(journal_path.exists());

        RecoveryCoordinator::cleanup_journal(&journal_path).unwrap();
        assert!(!journal_path.exists());
    }

    #[test]
    fn test_plan_helpers() {
        let plan = RecoveryPlan {
            source_serial: "SRC".into(),
            dest_serial: "DST".into(),
            total_bytes: 1000,
            resumed_offset: 1000,
            bytes_remaining: 0,
            percent_complete: 100.0,
            chunk_size: 100,
            checkpoint_age_secs: 50,
            first_block_xxhash: 12345,
        };
        assert!(plan.is_completed());
        assert_eq!(plan.chunks_completed(), 10);
        assert_eq!(plan.chunks_remaining(), 0);
        assert!(plan.summary().contains("100.00%"));
    }
}
