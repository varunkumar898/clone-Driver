//! Checkpoint journal persistence and crash recovery.

use crate::error::StorageError;
use crate::storage::checksum::ChecksumBlock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JournalEntry {
    pub timestamp: u64,
    pub offset: u64,
    pub bytes_copied: u64,
    pub checksum_block: ChecksumBlock,
}

pub struct CloneJournal {
    pub path: String,
    pub entries: Vec<JournalEntry>,
}

impl CloneJournal {
    /// Create new journal at ~/.cache/diskclone/clone_TIMESTAMP.json
    pub fn create(
        _source_path: &str,
        _dest_path: &str,
        _source_size: u64,
    ) -> Result<Self, StorageError> {
        // Get cache directory (CLARIFICATION 3)
        let cache_dir = dirs::cache_dir()
            .ok_or(StorageError::NoCacheDirectory)?
            .join("diskclone");

        // Create directory if needed
        fs::create_dir_all(&cache_dir).map_err(StorageError::IoError)?;

        // Generate filename with timestamp and monotonic sequence to prevent test collision
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| StorageError::TimeError)?
            .as_secs();

        let filename = if seq == 0 {
            format!("clone_{}.json", timestamp)
        } else {
            format!("clone_{}_{}.json", timestamp, seq)
        };
        let path = cache_dir.join(filename);

        // Create journal
        let journal = CloneJournal {
            path: path.to_string_lossy().to_string(),
            entries: Vec::new(),
        };

        journal.save()?;

        Ok(journal)
    }

    /// Append entry to journal and save immediately
    pub fn append(&mut self, entry: JournalEntry) -> Result<(), StorageError> {
        self.entries.push(entry);
        self.save()
    }

    /// Save journal to disk
    pub fn save(&self) -> Result<(), StorageError> {
        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        let path = Path::new(&self.path);
        let temp_path = path.parent().unwrap().join(format!(
            "{}.tmp",
            path.file_name().unwrap().to_string_lossy()
        ));

        // Write to temp file first
        fs::write(&temp_path, &json).map_err(StorageError::IoError)?;

        // Atomic rename
        fs::rename(&temp_path, path).map_err(StorageError::IoError)?;

        Ok(())
    }

    /// Load journal from file
    pub fn load(journal_path: &str) -> Result<Self, StorageError> {
        let content = fs::read_to_string(journal_path).map_err(StorageError::IoError)?;

        let entries: Vec<JournalEntry> = serde_json::from_str(&content)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;

        Ok(CloneJournal {
            path: journal_path.to_string(),
            entries,
        })
    }

    /// Get last checkpoint entry
    pub fn last_checkpoint(&self) -> Option<&JournalEntry> {
        self.entries.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_create_and_save() {
        let journal = CloneJournal::create("/dev/sda", "/dev/sdb", 1000000).unwrap();

        // File should exist
        assert!(std::path::Path::new(&journal.path).exists());

        // Clean up
        let _ = std::fs::remove_file(&journal.path);
    }

    #[test]
    fn test_journal_append_and_load() {
        let mut journal = CloneJournal::create("/dev/sda", "/dev/sdb", 1000000).unwrap();

        let entry = JournalEntry {
            timestamp: 12345,
            offset: 0,
            bytes_copied: 1000,
            checksum_block: ChecksumBlock {
                offset: 0,
                size: 1000,
                xxhash: 12345,
                sha256: None,
            },
        };

        journal.append(entry.clone()).unwrap();

        // Load and verify
        let loaded = CloneJournal::load(&journal.path).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.last_checkpoint().unwrap().offset, 0);

        // Clean up
        let _ = std::fs::remove_file(&journal.path);
    }
}
