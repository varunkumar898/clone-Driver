//! Resume journal data structures and serialization.
//!
//! Provides atomic persistence of resume records using a temp-file + rename
//! pattern to prevent partial writes from corrupting the journal.

use crate::error::{Result, ResumeError};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// A serializable snapshot of clone progress, persisted for crash-recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeRecord {
    /// Serial number / identifier of the source device.
    pub source_serial: String,
    /// Serial number / identifier of the destination device.
    pub dest_serial: String,
    /// Total size of source in bytes.
    pub source_size_bytes: u64,
    /// Last successfully completed byte offset.
    pub last_completed_offset: u64,
    /// Block size used for the clone operation (bytes).
    pub chunk_size: usize,
    /// xxHash64 of the very first block (used to verify same device on resume).
    pub first_block_xxhash: u64,
    /// Unix epoch seconds when this record was last written.
    pub timestamp_epoch_secs: u64,
}

impl ResumeRecord {
    /// Returns current Unix timestamp in seconds.
    fn now_epoch_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Creates a new record with the current timestamp.
    pub fn new(
        source_serial: impl Into<String>,
        dest_serial: impl Into<String>,
        source_size_bytes: u64,
        chunk_size: usize,
        first_block_xxhash: u64,
    ) -> Self {
        ResumeRecord {
            source_serial: source_serial.into(),
            dest_serial: dest_serial.into(),
            source_size_bytes,
            last_completed_offset: 0,
            chunk_size,
            first_block_xxhash,
            timestamp_epoch_secs: Self::now_epoch_secs(),
        }
    }

    /// Bumps the timestamp to now (called before each save).
    pub fn touch(&mut self) {
        self.timestamp_epoch_secs = Self::now_epoch_secs();
    }
}

/// Handles atomic persistence of [`ResumeRecord`] to disk.
pub struct ResumeJournal;

impl ResumeJournal {
    /// Loads an existing journal from disk.
    ///
    /// Returns a [`ResumeError::JournalNotFound`] if the file does not exist,
    /// or [`ResumeError::CorruptedJournal`] if the JSON is invalid.
    pub fn load(path: &Path) -> Result<ResumeRecord> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                crate::error::CloneError::Resume(ResumeError::JournalNotFound(
                    path.display().to_string(),
                ))
            } else {
                crate::error::CloneError::Resume(ResumeError::CorruptedJournal(format!(
                    "read error for {}: {e}",
                    path.display()
                )))
            }
        })?;

        serde_json::from_str::<ResumeRecord>(&content).map_err(|e| {
            crate::error::CloneError::Resume(ResumeError::CorruptedJournal(format!(
                "JSON parse error: {e}"
            )))
        })
    }

    /// Persists a journal checkpoint to disk atomically.
    ///
    /// Writes to a sibling `.tmp` file first, then uses `rename(2)` so the
    /// final path is never partially written.
    pub fn save(path: &Path, record: &ResumeRecord) -> Result<()> {
        let json = serde_json::to_string_pretty(record)
            .map_err(crate::error::CloneError::Serialization)?;

        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(crate::error::CloneError::Io)?;
        }

        // Write to a temp file in the same directory (same filesystem → rename is atomic).
        let tmp_path = path.with_extension("tmp");
        let mut tmp_file =
            std::fs::File::create(&tmp_path).map_err(crate::error::CloneError::Io)?;
        tmp_file
            .write_all(json.as_bytes())
            .map_err(crate::error::CloneError::Io)?;
        tmp_file.sync_all().map_err(crate::error::CloneError::Io)?;
        drop(tmp_file);

        // Atomic rename.
        std::fs::rename(&tmp_path, path).map_err(crate::error::CloneError::Io)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_record() -> ResumeRecord {
        ResumeRecord::new(
            "SRC_SERIAL",
            "DST_SERIAL",
            10 * 1024 * 1024 * 1024,
            4096,
            0xDEADBEEF,
        )
    }

    #[test]
    fn test_save_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("resume.json");
        let record = make_record();
        ResumeJournal::save(&path, &record).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("resume.json");
        let mut record = make_record();
        record.last_completed_offset = 512 * 1024 * 1024;

        ResumeJournal::save(&path, &record).unwrap();
        let loaded = ResumeJournal::load(&path).unwrap();

        assert_eq!(loaded.source_serial, record.source_serial);
        assert_eq!(loaded.last_completed_offset, 512 * 1024 * 1024);
        assert_eq!(loaded.chunk_size, 4096);
        assert_eq!(loaded.first_block_xxhash, 0xDEADBEEF);
    }

    #[test]
    fn test_load_missing_file_returns_not_found() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");
        let err = ResumeJournal::load(&path);
        assert!(err.is_err());
        let msg = format!("{}", err.unwrap_err());
        assert!(msg.contains("not found") || msg.contains("Resume journal not found"));
    }

    #[test]
    fn test_load_corrupt_json_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("corrupt.json");
        std::fs::write(&path, b"{ not valid json !!").unwrap();

        let err = ResumeJournal::load(&path);
        assert!(err.is_err());
    }

    #[test]
    fn test_no_tmp_file_left_after_save() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("resume.json");
        let record = make_record();
        ResumeJournal::save(&path, &record).unwrap();
        // .tmp file must be gone after atomic rename
        assert!(!path.with_extension("tmp").exists());
    }
}
