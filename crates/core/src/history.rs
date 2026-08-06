use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::usage::Usage;

/// One recorded observation of an account's rate-limit windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub ts: i64,
    pub windows: Vec<crate::usage::Window>,
}

/// Samples older than this are pruned on write.
pub const RETENTION_SECS: i64 = 30 * 24 * 3600;
/// A repeat observation inside this interval is skipped unless the values changed.
const MIN_INTERVAL_SECS: i64 = 600;

/// Lives inside the account dir (its CODEX_HOME) so remove/rename carry it automatically. The
/// codex-buddy-specific name keeps it out of `build_account_dir`'s way: a plain name that also
/// existed under `~/.codex` would be mirrored as a shared symlink, and the real file already
/// sitting there would make that rebuild fail.
const FILE_NAME: &str = ".codex-buddy-usage-history.jsonl";

/// Append `usage` to the account's local history, dedup-throttled and pruned to retention.
///
/// Best-effort observation: callers are read paths (list, usage, live fetches) and must swallow
/// errors rather than let bookkeeping break them. The rewrite is atomic (temp + rename); two
/// processes racing can lose one sample, which is acceptable for a trend line.
pub fn record(account_dir: &Path, usage: &Usage, now: i64) -> Result<()> {
    let path = account_dir.join(FILE_NAME);
    let mut samples = read_samples(&path);
    if let Some(last) = samples.last()
        && now - last.ts < MIN_INTERVAL_SECS
        && last.windows == usage.windows
    {
        return Ok(());
    }
    samples.push(Sample {
        ts: now,
        windows: usage.windows.clone(),
    });
    samples.retain(|sample| now - sample.ts <= RETENTION_SECS);

    let mut out = String::new();
    for sample in &samples {
        out.push_str(&serde_json::to_string(sample)?);
        out.push('\n');
    }
    let tmp = account_dir.join(format!("{FILE_NAME}.tmp"));
    fs::write(&tmp, out)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Samples recorded at or after `since`, oldest first. Unreadable lines are skipped.
pub fn load(account_dir: &Path, since: i64) -> Vec<Sample> {
    read_samples(&account_dir.join(FILE_NAME))
        .into_iter()
        .filter(|sample| sample.ts >= since)
        .collect()
}

fn read_samples(path: &Path) -> Vec<Sample> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[cfg(test)]
mod tests;
