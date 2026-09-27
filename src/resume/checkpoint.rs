//! Periodic checkpoint committer.
//!
//! Wraps [`ResumeJournal`] to provide a simple API for updating and
//! persisting the resume offset during a running clone operation.

use super::journal::{ResumeJournal, ResumeRecord};
use crate::error::Result;
use std::path::PathBuf;

/// Manages checkpoint writes for an in-progress clone.
///
/// Call [`commit_offset`] every 500 MB (or at whatever interval the engine
/// chooses) to persist the latest progress so the operation can be resumed
/// after a crash or power failure.
pub struct CheckpointManager {
    journal_path: PathBuf,
    record: ResumeRecord,
}

impl CheckpointManager {
    /// Creates a new checkpoint manager.
    ///
    /// `journal_path` is the file that will be atomically updated on each
    /// commit.  `record` provides the initial state (source/dest serials,
    /// sizes, etc.).
    pub fn new(journal_path: PathBuf, record: ResumeRecord) -> Self {
        Self {
            journal_path,
            record,
        }
    }

    /// Commits the given byte offset as the last known-good position.
    ///
    /// Updates `record.last_completed_offset`, refreshes the timestamp, and
    /// atomically writes the journal to disk.
    pub fn commit_offset(&mut self, offset: u64) -> Result<()> {
        self.record.last_completed_offset = offset;
        self.record.touch();
        ResumeJournal::save(&self.journal_path, &self.record)
    }

    /// Returns the last committed offset (without I/O).
    pub fn last_committed_offset(&self) -> u64 {
        self.record.last_completed_offset
    }

    /// Returns an immutable reference to the current resume record.
    pub fn record(&self) -> &ResumeRecord {
        &self.record
    }

    /// Returns the path used for persistence.
    pub fn journal_path(&self) -> &PathBuf {
        &self.journal_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resume::journal::ResumeRecord;
    use tempfile::TempDir;

    fn make_record() -> ResumeRecord {
        ResumeRecord::new(
            "SRC001",
            "DST001",
            10 * 1024 * 1024 * 1024_u64,
            4 * 1024 * 1024,
            0xCAFEBABE,
        )
    }

    #[test]
    fn test_commit_offset_updates_record() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("checkpoint.json");
        let record = make_record();
        let mut manager = CheckpointManager::new(path.clone(), record);

        manager.commit_offset(500 * 1024 * 1024).unwrap();
        assert_eq!(manager.last_committed_offset(), 500 * 1024 * 1024);
    }

    #[test]
    fn test_commit_offset_persists_to_disk() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("checkpoint.json");
        let record = make_record();
        let mut manager = CheckpointManager::new(path.clone(), record);

        manager.commit_offset(1024 * 1024 * 1024).unwrap();

        // Journal file should exist and be valid JSON.
        let loaded = ResumeJournal::load(&path).unwrap();
        assert_eq!(loaded.last_completed_offset, 1024 * 1024 * 1024);
        assert_eq!(loaded.source_serial, "SRC001");
    }

    #[test]
    fn test_multiple_commits_overwrite_correctly() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("checkpoint.json");
        let record = make_record();
        let mut manager = CheckpointManager::new(path.clone(), record);

        manager.commit_offset(100 * 1024 * 1024).unwrap();
        manager.commit_offset(200 * 1024 * 1024).unwrap();
        manager.commit_offset(300 * 1024 * 1024).unwrap();

        let loaded = ResumeJournal::load(&path).unwrap();
        assert_eq!(loaded.last_completed_offset, 300 * 1024 * 1024);
    }

    #[test]
    fn test_last_committed_offset_initial_is_zero() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("checkpoint.json");
        let record = make_record();
        let manager = CheckpointManager::new(path, record);
        assert_eq!(manager.last_committed_offset(), 0);
    }
}
