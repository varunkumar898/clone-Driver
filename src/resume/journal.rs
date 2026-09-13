//! Resume journal data structures and serialization.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeRecord {
    pub source_serial: String,
    pub dest_serial: String,
    pub source_size_bytes: u64,
    pub last_completed_offset: u64,
    pub chunk_size: usize,
    pub first_block_xxhash: u64,
    pub timestamp_epoch_secs: u64,
}

pub struct ResumeJournal;

impl ResumeJournal {
    /// Loads an existing journal from disk.
    pub fn load(_path: &Path) -> Result<ResumeRecord> {
        todo!("Phase 3: implement journal loading")
    }

    /// Persists a journal checkpoint to disk atomically.
    pub fn save(_path: &Path, _record: &ResumeRecord) -> Result<()> {
        todo!("Phase 3: implement atomic journal save")
    }
}
