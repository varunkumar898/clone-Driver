use diskclone::storage::adaptive::{AdaptiveBlockSize, AdaptiveChunker, DeviceType};

#[test]
fn test_adaptive_hdd_defaults() {
    let adaptive = AdaptiveBlockSize::new(DeviceType::HDD);
    assert_eq!(adaptive.get_size(), 4 * 1024 * 1024);
}

#[test]
fn test_adaptive_ssd_defaults() {
    let adaptive = AdaptiveBlockSize::new(DeviceType::SSD);
    assert_eq!(adaptive.get_size(), 8 * 1024 * 1024);
}

#[test]
fn test_adaptive_decrease_on_high_latency() {
    let mut adaptive = AdaptiveBlockSize::new(DeviceType::SSD);
    let initial = adaptive.get_size();

    adaptive.adjust(100.0, 100.0); // High latency
    assert!(adaptive.get_size() < initial);
}

#[test]
fn test_adaptive_chunker_scaffold() {
    let chunker = AdaptiveChunker::new(4 * 1024 * 1024, 1024 * 1024, 16 * 1024 * 1024);
    assert_eq!(chunker.current_chunk_size(), 4 * 1024 * 1024);
}
