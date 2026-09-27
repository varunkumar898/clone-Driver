use diskclone::storage::checksum::ChecksumBlock;
use diskclone::storage::journal::{CloneJournal, JournalEntry};

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
