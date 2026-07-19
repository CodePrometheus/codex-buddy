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

    let u = latest_usage(d.path()).unwrap();
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

    let u = latest_usage(d.path()).unwrap();
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

    let u = latest_usage(d.path()).unwrap();
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
    assert!(latest_usage(d.path()).is_none());
}

#[test]
fn skips_null_rate_limits() {
    let d = tempdir().unwrap();
    fs::write(
        d.path().join("rollout-x.jsonl"),
        "{\"payload\":{\"rate_limits\":null}}\n",
    )
    .unwrap();
    assert!(latest_usage(d.path()).is_none());
}
