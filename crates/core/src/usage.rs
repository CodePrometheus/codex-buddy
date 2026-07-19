use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// One rate-limit window (e.g. the 5h or weekly window).
#[derive(Debug, Clone)]
pub struct Window {
    pub window_minutes: i64,
    pub used_percent: f64,
    pub resets_at: Option<i64>,
}

/// A usage snapshot: the newest state of each window (typically 5h + weekly).
#[derive(Debug, Clone)]
pub struct Usage {
    pub windows: Vec<Window>,
}

const FIVE_HOUR: i64 = 300;
const WEEKLY: i64 = 10080;

/// Newest usage snapshot from a sessions dir.
///
/// Codex logs a rate_limits record per event, but any one record may carry only a single window
/// (a fresh session often reports just the weekly one), so each window is tracked independently.
/// Recency comes from scan order, not `resets_at`: for a rolling window, `resets_at` tracks when
/// the *oldest* counted usage rolls off, which barely moves as a session's `used_percent` climbs
/// (confirmed against a real session: it held one value across 30+ points of rising usage,
/// wobbling by a second or two from API-side jitter) — comparing it as "bigger = newer" can pick
/// an earlier, lower reading over the true latest one. Rollouts are scanned newest-file-first;
/// within a file, `windows_in_file` returns records in the log's chronological order, so simply
/// keeping the last one seen per window is that file's most recent reading. A window already
/// found in a newer file is left alone — older files can't override it. Stops once both the 5h
/// and weekly windows are known. None if the account has no rollouts carrying rate_limits.
pub fn latest_usage(sessions_dir: &Path) -> Option<Usage> {
    let mut rollouts = Vec::new();
    collect_rollouts(sessions_dir, &mut rollouts);
    rollouts.sort();

    let mut best: BTreeMap<i64, Window> = BTreeMap::new();
    for path in rollouts.iter().rev().take(80) {
        let mut latest_in_file: BTreeMap<i64, Window> = BTreeMap::new();
        for w in windows_in_file(path) {
            latest_in_file.insert(w.window_minutes, w);
        }
        for (window_minutes, w) in latest_in_file {
            best.entry(window_minutes).or_insert(w);
        }
        if best.contains_key(&FIVE_HOUR) && best.contains_key(&WEEKLY) {
            break;
        }
    }
    (!best.is_empty()).then(|| Usage {
        windows: best.into_values().collect(),
    })
}

fn collect_rollouts(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rollouts(&p, out);
        } else if is_rollout(&p) {
            out.push(p);
        }
    }
}

fn is_rollout(p: &Path) -> bool {
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"))
}

fn windows_in_file(path: &Path) -> Vec<Window> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in content.lines() {
        if !line.contains("rate_limits") {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(line)
            && let Some(rl) = find_rate_limits(&v)
        {
            for key in ["primary", "secondary"] {
                if let Some(w) = rl.get(key).and_then(parse_window) {
                    out.push(w);
                }
            }
        }
    }
    out
}

fn find_rate_limits(v: &Value) -> Option<&Value> {
    match v {
        Value::Object(m) => {
            if let Some(rl) = m.get("rate_limits")
                && !rl.is_null()
            {
                return Some(rl);
            }
            m.values().find_map(find_rate_limits)
        }
        Value::Array(a) => a.iter().find_map(find_rate_limits),
        _ => None,
    }
}

fn parse_window(v: &Value) -> Option<Window> {
    let window_minutes = v.get("window_minutes")?.as_i64()?;
    let used_percent = v.get("used_percent")?.as_f64()?;
    let resets_at = v.get("resets_at").and_then(Value::as_i64);
    Some(Window {
        window_minutes,
        used_percent,
        resets_at,
    })
}

#[cfg(test)]
mod tests;
