use diskclone::storage::progress::{ProgressSnapshot, ProgressTracker};
use std::time::Duration;

#[test]
fn test_progress_tracker_new() {
    let tracker = ProgressTracker::new(1000);
    let snap = tracker.snapshot();
    assert_eq!(snap.percentage, 0.0);
}

#[test]
fn test_progress_report_progress() {
    let tracker = ProgressTracker::new(1000);
    let snap = tracker.report_progress(500);
    assert_eq!(snap.bytes_copied, 500);
    assert!(snap.percentage > 0.0);
}

#[test]
fn test_progress_format_summary() {
    let snap = ProgressSnapshot {
        bytes_copied: 500_000_000,
        total_bytes: 1_000_000_000,
        percentage: 50.0,
        throughput_mbps: 250.0,
        eta: Duration::from_secs(120),
        elapsed: Duration::from_secs(120),
        checksum_errors: 0,
    };

    let summary = snap.format_summary();
    assert!(summary.contains("50.0%"));
    assert!(summary.contains("250"));
}

#[test]
fn test_report_checksum_error() {
    let tracker = ProgressTracker::new(1000);
    tracker.report_checksum_error();
    tracker.report_checksum_error();
    assert_eq!(tracker.snapshot().checksum_errors, 2);
}
