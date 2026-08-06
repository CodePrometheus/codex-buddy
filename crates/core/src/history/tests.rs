use tempfile::tempdir;

use super::*;
use crate::usage::Window;

fn usage(used: f64) -> Usage {
    Usage {
        windows: vec![Window {
            window_minutes: 10080,
            used_percent: used,
            resets_at: Some(9_999_999_999),
        }],
    }
}

#[test]
fn records_and_loads_in_order() {
    let d = tempdir().unwrap();
    record(d.path(), &usage(10.0), 1_000).unwrap();
    record(d.path(), &usage(20.0), 2_000).unwrap();
    let samples = load(d.path(), 0);
    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].windows[0].used_percent, 10.0);
    assert_eq!(samples[1].windows[0].used_percent, 20.0);
    assert_eq!(load(d.path(), 1_500).len(), 1);
}

#[test]
fn unchanged_samples_inside_the_throttle_window_are_skipped() {
    let d = tempdir().unwrap();
    record(d.path(), &usage(10.0), 1_000).unwrap();
    record(d.path(), &usage(10.0), 1_000 + 60).unwrap();
    assert_eq!(load(d.path(), 0).len(), 1);

    // A changed value goes through even inside the interval.
    record(d.path(), &usage(11.0), 1_000 + 120).unwrap();
    assert_eq!(load(d.path(), 0).len(), 2);

    // The same value goes through once the interval has passed.
    record(d.path(), &usage(11.0), 1_000 + 120 + MIN_INTERVAL_SECS).unwrap();
    assert_eq!(load(d.path(), 0).len(), 3);
}

#[test]
fn old_samples_are_pruned_on_write() {
    let d = tempdir().unwrap();
    record(d.path(), &usage(10.0), 1_000).unwrap();
    record(d.path(), &usage(20.0), 1_000 + RETENTION_SECS + 1).unwrap();
    let samples = load(d.path(), 0);
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].windows[0].used_percent, 20.0);
}

#[test]
fn garbage_lines_are_skipped_not_fatal() {
    let d = tempdir().unwrap();
    record(d.path(), &usage(10.0), 1_000).unwrap();
    let path = d.path().join(FILE_NAME);
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("not-json\n");
    std::fs::write(&path, text).unwrap();

    assert_eq!(load(d.path(), 0).len(), 1);
    record(d.path(), &usage(20.0), 2_000).unwrap();
    assert_eq!(load(d.path(), 0).len(), 2);
}
