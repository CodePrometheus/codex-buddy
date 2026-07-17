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
fn keeps_newest_per_window() {
    let d = tempdir().unwrap();
    let day = d.path().join("2026/07/16");
    fs::create_dir_all(&day).unwrap();
    // Older record carries both windows; a later record carries only a fresher weekly.
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
    assert_eq!(five.used_percent, 4.0); // only present in the older record
    assert_eq!(week.used_percent, 0.0); // newest weekly wins (resets_at 300 > 200)
    assert_eq!(week.resets_at, Some(300));
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
