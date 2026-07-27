use serde::Serialize;

use crate::auth::load_auth_info;
use crate::error::{Error, Result};
use crate::paths::{Paths, validate_alias};
use crate::registry::{self, now_epoch};
use crate::usage;

/// One fresh usage window used to explain an account recommendation.
#[derive(Debug, Clone, Serialize)]
pub struct RecommendationWindow {
    pub window_minutes: i64,
    pub used_percent: f64,
    pub remaining_percent: f64,
    pub resets_at: Option<i64>,
}

/// The account with the most headroom in its tightest comparable usage window.
#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub alias: String,
    pub bottleneck_window_minutes: i64,
    pub remaining_percent: f64,
    pub windows: Vec<RecommendationWindow>,
    pub last_used_at: Option<i64>,
}

struct Candidate {
    recommendation: Recommendation,
    registry_index: usize,
}

/// Recommend an account from fresh local usage data without making any state change.
///
/// Every usable account must expose the same set of fresh windows. Treating a missing window as
/// empty or full would manufacture certainty, so mismatched coverage returns an explicit error.
pub fn recommend(paths: &Paths) -> Result<Recommendation> {
    let reg = load_registry(paths)?;
    let now = now_epoch();
    let mut inputs = Vec::with_capacity(reg.accounts.len());
    for rec in &reg.accounts {
        validate_alias(&rec.dir)?;
        load_auth_info(&paths.account_auth(&rec.dir)).map_err(|e| {
            Error::Other(format!(
                "cannot recommend: account `{}` has invalid auth.json: {e}",
                rec.alias
            ))
        })?;
        let usage = usage::latest_usage(&paths.account_dir(&rec.dir).join("sessions"), now)
            .ok_or_else(|| {
                Error::Other(format!(
                    "cannot recommend: account `{}` has no local usage data",
                    rec.alias
                ))
            })?;
        inputs.push((rec.alias.clone(), rec.last_used_at, usage));
    }
    pick(inputs, now)
}

/// Same recommendation, but from live usage fetched through `codex app-server` for every
/// account — useful when local session data is stale or missing.
pub fn recommend_remote(paths: &Paths) -> Result<Recommendation> {
    let reg = load_registry(paths)?;
    let now = now_epoch();
    let mut inputs = Vec::with_capacity(reg.accounts.len());
    for rec in &reg.accounts {
        let remote = crate::remote::fetch_account(paths, &rec.alias)
            .map_err(|e| Error::Other(format!("cannot recommend: account `{}`: {e}", rec.alias)))?;
        inputs.push((rec.alias.clone(), rec.last_used_at, remote.usage));
    }
    pick(inputs, now)
}

fn load_registry(paths: &Paths) -> Result<registry::Registry> {
    let reg = registry::load(&paths.registry_file())?;
    if reg.accounts.is_empty() {
        return Err(Error::Other("cannot recommend: no accounts".into()));
    }
    Ok(reg)
}

/// Shared scoring over per-account usage snapshots, in registry order.
fn pick(inputs: Vec<(String, Option<i64>, usage::Usage)>, now: i64) -> Result<Recommendation> {
    let mut candidates = Vec::with_capacity(inputs.len());
    let mut expected_windows: Option<Vec<i64>> = None;

    for (registry_index, (alias, last_used_at, usage)) in inputs.into_iter().enumerate() {
        let windows: Vec<RecommendationWindow> = usage
            .windows
            .into_iter()
            .filter(|window| !window.is_expired(now))
            .map(|window| RecommendationWindow {
                window_minutes: window.window_minutes,
                used_percent: window.used_percent.clamp(0.0, 100.0),
                remaining_percent: 100.0 - window.used_percent.clamp(0.0, 100.0),
                resets_at: window.resets_at,
            })
            .collect();
        if windows.is_empty() {
            return Err(Error::Other(format!(
                "cannot recommend: account `{alias}` has no fresh usage data"
            )));
        }

        let window_set: Vec<i64> = windows.iter().map(|w| w.window_minutes).collect();
        match &expected_windows {
            Some(expected) if expected != &window_set => {
                return Err(Error::Other(format!(
                    "cannot recommend: usage windows are not comparable; account `{alias}` has {window_set:?}, expected {expected:?}"
                )));
            }
            None => expected_windows = Some(window_set),
            _ => {}
        }

        let bottleneck = windows
            .iter()
            .min_by(|a, b| a.remaining_percent.total_cmp(&b.remaining_percent))
            .expect("windows is non-empty");
        candidates.push(Candidate {
            recommendation: Recommendation {
                alias,
                bottleneck_window_minutes: bottleneck.window_minutes,
                remaining_percent: bottleneck.remaining_percent,
                windows,
                last_used_at,
            },
            registry_index,
        });
    }

    let best = candidates
        .into_iter()
        .max_by(compare_candidates)
        .expect("inputs are non-empty");
    Ok(best.recommendation)
}

fn compare_candidates(a: &Candidate, b: &Candidate) -> std::cmp::Ordering {
    a.recommendation
        .remaining_percent
        .total_cmp(&b.recommendation.remaining_percent)
        // An account never used through codex-buddy is older than every timestamp.
        .then_with(|| {
            b.recommendation
                .last_used_at
                .unwrap_or(i64::MIN)
                .cmp(&a.recommendation.last_used_at.unwrap_or(i64::MIN))
        })
        // `max_by` chooses the later equal item; reverse the index so registry order wins.
        .then_with(|| b.registry_index.cmp(&a.registry_index))
}

#[cfg(test)]
mod tests;
