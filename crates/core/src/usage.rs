use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// One rate-limit window (e.g. the 5h or weekly window).
#[derive(Debug, Clone)]
pub struct Window {
    pub window_minutes: i64,
    pub used_percent: f64,
    pub resets_at: Option<i64>,
}

impl Window {
    /// Whether this reading is stale: once the reset time has passed the window has rolled
    /// over, so the recorded percentage no longer describes anything real.
    pub fn is_expired(&self, now: i64) -> bool {
        self.resets_at.is_some_and(|r| r <= now)
    }
}

/// A usage snapshot: the newest state of each window (typically 5h + weekly).
#[derive(Debug, Clone)]
pub struct Usage {
    pub windows: Vec<Window>,
}

pub const FIVE_HOUR_MINUTES: i64 = 300;
pub const WEEKLY_MINUTES: i64 = 10080;

/// Bytes read per backwards step when scanning a rollout from its tail.
const TAIL_CHUNK: usize = 64 * 1024;

/// Per-file backstop on tail bytes scanned. Codex logs a rate_limits record every event, so a
/// window that hasn't appeared in the last few MB of a live rollout won't have a *current*
/// reading deeper in the file either — anything found there would fail `is_expired` anyway.
const MAX_TAIL_SCAN: u64 = 4 * 1024 * 1024;

/// Newest usage snapshot from a sessions dir, given the current epoch seconds.
///
/// Codex logs a rate_limits record per event, but any one record may carry only a single window
/// (this machine's plus plan currently reports just the weekly one), so each window is tracked
/// independently. Recency comes from scan order, not `resets_at`: for a rolling window,
/// `resets_at` tracks when the *oldest* counted usage rolls off, which barely moves as a
/// session's `used_percent` climbs (confirmed against a real session: it held one value across
/// 30+ points of rising usage, wobbling by a second or two from API-side jitter) — comparing it
/// as "bigger = newer" can pick an earlier, lower reading over the true latest one.
///
/// Sessions are date-sharded (`YYYY/MM/DD/rollout-<timestamp>-<id>.jsonl`) and both dir and file
/// names sort chronologically, so the tree is traversed newest-first without collecting it, and
/// each file is scanned backwards from its tail — the first reading seen per window is that
/// window's latest, and older files can only fill windows newer ones didn't have.
///
/// A window's reset period bounds how far back a still-valid reading can exist: a file whose
/// mtime predates `now - 5h` can't contain an unexpired 5h reading, and one older than
/// `now - 7d` can't contain any — so the hunt for each window ends at its own horizon instead
/// of trawling months of history for data that would only be discarded as expired.
pub fn latest_usage(sessions_dir: &Path, now: i64) -> Option<Usage> {
    let five_hour_horizon = now - FIVE_HOUR_MINUTES * 60;
    let weekly_horizon = now - WEEKLY_MINUTES * 60;

    let mut best: BTreeMap<i64, Window> = BTreeMap::new();
    let _ = scan_newest_first(sessions_dir, &mut |file| {
        let mtime = mtime_epoch(file).unwrap_or(now);
        if mtime < weekly_horizon {
            return ControlFlow::Break(());
        }
        let done = |best: &BTreeMap<i64, Window>| {
            best.contains_key(&WEEKLY_MINUTES)
                && (best.contains_key(&FIVE_HOUR_MINUTES) || mtime < five_hour_horizon)
        };
        let _ = scan_lines_backwards(file, TAIL_CHUNK, MAX_TAIL_SCAN, &mut |line| {
            merge_line(line, &mut best);
            if done(&best) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        if done(&best) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    });
    (!best.is_empty()).then(|| Usage {
        windows: best.into_values().collect(),
    })
}

/// Merge one log line into `best`; the scan is newest-first, so the first reading seen per
/// window wins.
fn merge_line(line: &str, best: &mut BTreeMap<i64, Window>) {
    if line.contains("rate_limits")
        && let Ok(v) = serde_json::from_str::<Value>(line)
        && let Some(rl) = find_rate_limits(&v)
    {
        for key in ["primary", "secondary"] {
            if let Some(w) = rl.get(key).and_then(parse_window) {
                best.entry(w.window_minutes).or_insert(w);
            }
        }
    }
}

fn mtime_epoch(p: &Path) -> Option<i64> {
    let t = fs::metadata(p).ok()?.modified().ok()?;
    let d = t.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(d.as_secs() as i64)
}

/// Visit rollout files newest-first, descending lazily so a break stops the walk without ever
/// collecting the tree. Sorting each level descending is chronological for the date shards and
/// rollout file names alike, and keeps any non-standard layout working.
fn scan_newest_first(
    dir: &Path,
    visit: &mut impl FnMut(&Path) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let Ok(entries) = fs::read_dir(dir) else {
        return ControlFlow::Continue(());
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort_unstable();
    for p in paths.iter().rev() {
        if p.is_dir() {
            scan_newest_first(p, visit)?;
        } else if is_rollout(p) {
            visit(p)?;
        }
    }
    ControlFlow::Continue(())
}

fn is_rollout(p: &Path) -> bool {
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"))
}

/// Lines a rollout is scanned for (rate_limits records) are small; anything larger mid-assembly
/// is some other event's huge payload and is skipped rather than accumulated, which would
/// otherwise degrade the backwards walk to quadratic copying (real rollouts have >1 MB lines).
const MAX_LINE: usize = 128 * 1024;

/// Feed a file's lines to `f` newest-first without reading the file whole: rollouts are
/// append-only logs whose newest lines are last, and a single file can grow to hundreds of MB,
/// so the file is walked backwards in `chunk_size` steps from the tail, visiting at most
/// `max_scan` bytes. Lines split across chunk boundaries are reassembled before being emitted;
/// lines longer than [`MAX_LINE`] are dropped.
fn scan_lines_backwards(
    path: &Path,
    chunk_size: usize,
    max_scan: u64,
    f: &mut impl FnMut(&str) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let Ok(mut file) = File::open(path) else {
        return ControlFlow::Continue(());
    };
    let Ok(len) = file.seek(SeekFrom::End(0)) else {
        return ControlFlow::Continue(());
    };
    let stop = len.saturating_sub(max_scan);
    let mut pos = len;
    // The not-yet-emitted prefix of the earliest line seen so far; grows leftwards each step.
    let mut tail: Vec<u8> = Vec::new();
    // Set while walking through the middle of an overlong line being dropped.
    let mut skipping = false;
    while pos > stop {
        let step = (chunk_size as u64).min(pos - stop);
        pos -= step;
        let mut buf = vec![0u8; step as usize];
        if file.seek(SeekFrom::Start(pos)).is_err() || file.read_exact(&mut buf).is_err() {
            return ControlFlow::Continue(());
        }
        buf.extend_from_slice(&tail);
        let complete_from = if pos == 0 {
            0
        } else {
            match buf.iter().position(|&b| b == b'\n') {
                Some(i) => i + 1,
                // No newline in the whole chunk: everything is one longer line's middle.
                None => {
                    if skipping {
                        continue;
                    }
                    if buf.len() > MAX_LINE {
                        tail.clear();
                        skipping = true;
                    } else {
                        tail = buf;
                    }
                    continue;
                }
            }
        };
        let mut lines = buf[complete_from..].split(|&b| b == b'\n').rev();
        if skipping {
            // The rightmost fragment belongs to the line being dropped.
            lines.next();
            skipping = false;
        }
        for line in lines {
            if line.is_empty() {
                continue;
            }
            if let Ok(s) = std::str::from_utf8(line)
                && f(s).is_break()
            {
                return ControlFlow::Break(());
            }
        }
        tail = buf[..complete_from.saturating_sub(1)].to_vec();
    }
    ControlFlow::Continue(())
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
