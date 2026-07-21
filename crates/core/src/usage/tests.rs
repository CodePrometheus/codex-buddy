use super::*;
use tempfile::tempdir;

fn rec(five_h: Option<(f64, i64)>, weekly: Option<(f64, i64)>) -> String {
    let mut rl = serde_json::Map::new();
    if let Some((used, resets)) = five_h {
        rl.insert(
            "primary".into(),
            serde_json::json!({"used_percent": used, "window_minutes": 300, "resets_at": resets}),
        );
    }
    if let Some((used, resets)) = weekly {
        rl.insert(
            "secondary".into(),
            serde_json::json!({"used_percent": used, "window_minutes": 10080, "resets_at": resets}),
        );
    }
    serde_json::json!({ "payload": { "rate_limits": Value::Object(rl) } }).to_string()
}

#[test]
fn keeps_the_last_line_per_window_within_a_file() {
    let d = tempdir().unwrap();
    let day = d.path().join("2026/07/16");
    fs::create_dir_all(&day).unwrap();
    // Earlier line carries both windows; a later line carries only a fresher weekly.
    let body = format!(
        "{}\n{}\n",
        rec(Some((4.0, 100)), Some((41.0, 200))),
        rec(None, Some((0.0, 300))),
    );
    fs::write(day.join("rollout-a.jsonl"), body).unwrap();

    let u = latest_usage(d.path(), 0).unwrap();
    let five = u.windows.iter().find(|w| w.window_minutes == 300).unwrap();
    let week = u
        .windows
        .iter()
        .find(|w| w.window_minutes == 10080)
        .unwrap();
    assert_eq!(five.used_percent, 4.0); // only present in the earlier line
    assert_eq!(week.used_percent, 0.0); // last line in the file wins
    assert_eq!(week.resets_at, Some(300));
}

#[test]
fn a_flat_or_jittery_resets_at_does_not_fool_recency() {
    // Regression test: a real session showed resets_at pinned at one value for a rolling
    // window while used_percent climbed steadily, with occasional 1-2s jitter — an earlier,
    // lower reading with a slightly *larger* resets_at must not beat the true latest one.
    let d = tempdir().unwrap();
    let day = d.path().join("2026/07/19");
    fs::create_dir_all(&day).unwrap();
    let body = format!(
        "{}\n{}\n{}\n",
        rec(None, Some((11.0, 1_785_041_970))), // earlier, higher resets_at (jitter)
        rec(None, Some((36.0, 1_785_041_968))),
        rec(None, Some((37.0, 1_785_041_968))), // true latest: lower resets_at, higher usage
    );
    fs::write(day.join("rollout-a.jsonl"), body).unwrap();

    let u = latest_usage(d.path(), 0).unwrap();
    let week = u
        .windows
        .iter()
        .find(|w| w.window_minutes == 10080)
        .unwrap();
    assert_eq!(week.used_percent, 37.0);
}

#[test]
fn a_window_found_in_a_newer_file_is_not_overridden_by_an_older_file() {
    let d = tempdir().unwrap();
    let older = d.path().join("2026/07/16");
    let newer = d.path().join("2026/07/19");
    fs::create_dir_all(&older).unwrap();
    fs::create_dir_all(&newer).unwrap();
    fs::write(
        older.join("rollout-a.jsonl"),
        rec(Some((90.0, 1)), Some((90.0, 1))) + "\n",
    )
    .unwrap();
    fs::write(
        newer.join("rollout-b.jsonl"),
        rec(Some((5.0, 1)), None) + "\n",
    )
    .unwrap();

    let u = latest_usage(d.path(), 0).unwrap();
    let five = u.windows.iter().find(|w| w.window_minutes == 300).unwrap();
    let week = u
        .windows
        .iter()
        .find(|w| w.window_minutes == 10080)
        .unwrap();
    assert_eq!(five.used_percent, 5.0); // newer file's reading
    assert_eq!(week.used_percent, 90.0); // only the older file has this window; still used
}

#[test]
fn none_when_no_rollouts() {
    let d = tempdir().unwrap();
    assert!(latest_usage(d.path(), 0).is_none());
}

#[test]
fn backwards_scan_reassembles_lines_across_chunk_boundaries() {
    let d = tempdir().unwrap();
    let p = d.path().join("f");
    fs::write(&p, "first-line\nsecond-longer-line\nlast\n").unwrap();
    let mut seen = Vec::new();
    let flow = scan_lines_backwards(&p, 7, u64::MAX, &mut |line| {
        seen.push(line.to_string());
        ControlFlow::Continue(())
    });
    assert!(flow.is_continue());
    assert_eq!(seen, ["last", "second-longer-line", "first-line"]);
}

#[test]
fn backwards_scan_stops_at_break() {
    let d = tempdir().unwrap();
    let p = d.path().join("f");
    fs::write(&p, "old\nnewest\n").unwrap();
    let mut seen = Vec::new();
    let flow = scan_lines_backwards(&p, 4, u64::MAX, &mut |line| {
        seen.push(line.to_string());
        ControlFlow::Break(())
    });
    assert!(flow.is_break());
    assert_eq!(seen, ["newest"]);
}

#[test]
fn backwards_scan_handles_a_file_without_trailing_newline() {
    let d = tempdir().unwrap();
    let p = d.path().join("f");
    fs::write(&p, "a\nb").unwrap();
    let mut seen = Vec::new();
    let _ = scan_lines_backwards(&p, 64, u64::MAX, &mut |line| {
        seen.push(line.to_string());
        ControlFlow::Continue(())
    });
    assert_eq!(seen, ["b", "a"]);
}

#[test]
fn an_expired_window_is_flagged() {
    let w = Window {
        window_minutes: 300,
        used_percent: 10.0,
        resets_at: Some(100),
    };
    assert!(w.is_expired(100));
    assert!(!w.is_expired(99));
    let unbounded = Window {
        window_minutes: 300,
        used_percent: 10.0,
        resets_at: None,
    };
    assert!(!unbounded.is_expired(i64::MAX));
}

#[test]
fn skips_null_rate_limits() {
    let d = tempdir().unwrap();
    fs::write(
        d.path().join("rollout-x.jsonl"),
        "{\"payload\":{\"rate_limits\":null}}\n",
    )
    .unwrap();
    assert!(latest_usage(d.path(), 0).is_none());
}

#[test]
fn files_older_than_the_weekly_horizon_are_not_scanned() {
    let d = tempdir().unwrap();
    let p = d.path().join("rollout-old.jsonl");
    fs::write(
        &p,
        rec(Some((50.0, i64::MAX)), Some((50.0, i64::MAX))) + "\n",
    )
    .unwrap();

    let now = mtime_epoch(&p).unwrap() + WEEKLY_MINUTES * 60 + 60;
    assert!(latest_usage(d.path(), now).is_none());
    // Just inside the horizon the same file is used.
    assert!(latest_usage(d.path(), now - 120).is_some());
}

#[test]
fn an_overlong_line_is_skipped_not_accumulated() {
    let d = tempdir().unwrap();
    let p = d.path().join("f");
    let huge = "x".repeat(MAX_LINE * 2);
    fs::write(&p, format!("first\n{huge}\nlast\n")).unwrap();
    let mut seen = Vec::new();
    let _ = scan_lines_backwards(&p, 1024, u64::MAX, &mut |line| {
        seen.push(line.to_string());
        ControlFlow::Continue(())
    });
    assert_eq!(seen, ["last", "first"]);
}
