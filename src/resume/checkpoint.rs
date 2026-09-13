//! Periodic checkpoint committer.

use super::journal::ResumeRecord;
use crate::error::Result;
use std::path::PathBuf;

pub struct CheckpointManager {
    journal_path: PathBuf,
    record: ResumeRecord,
}

impl CheckpointManager {
    pub fn new(journal_path: PathBuf, record: ResumeRecord) -> Self {
        Self {
            journal_path,
            record,
        }
    }

    /// Commits an offset checkpoint.
    pub fn commit_offset(&mut self, _offset: u64) -> Result<()> {
        todo!("Phase 3: implement commit_offset")
    }
}
