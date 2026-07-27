use codex_buddy_core::error::Error;
use codex_buddy_core::ops::AccountView;
use codex_buddy_core::remote::RemoteUsage;
use codex_buddy_core::transfer::{ImportItemResult, ImportStatus};
use codex_buddy_core::usage::Usage;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageStatus {
    Fresh,
    Expired,
    Missing,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageSource {
    Local,
    Remote,
}

#[derive(Debug, Serialize)]
pub struct UsageWindow {
    pub window_minutes: i64,
    pub used_percent: f64,
    pub resets_at: Option<i64>,
    pub expired: bool,
}

#[derive(Debug, Serialize)]
pub struct AccountUsage {
    pub alias: String,
    pub source: UsageSource,
    pub status: UsageStatus,
    pub windows: Vec<UsageWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AccountUsage {
    pub fn from_view(view: &AccountView, now: i64) -> Self {
        let windows = view
            .usage
            .as_ref()
            .map(|usage| usage_windows(usage, now))
            .unwrap_or_default();
        Self {
            alias: view.alias.clone(),
            source: UsageSource::Local,
            status: status_of(&windows),
            windows,
            error: None,
        }
    }

    pub fn from_remote(alias: &str, fetched: Result<RemoteUsage, Error>, now: i64) -> Self {
        match fetched {
            Ok(remote) => {
                let windows = usage_windows(&remote.usage, now);
                Self {
                    alias: alias.to_string(),
                    source: UsageSource::Remote,
                    status: status_of(&windows),
                    windows,
                    error: None,
                }
            }
            Err(e) => Self {
                alias: alias.to_string(),
                source: UsageSource::Remote,
                status: UsageStatus::Error,
                windows: Vec::new(),
                error: Some(e.to_string()),
            },
        }
    }
}

fn usage_windows(usage: &Usage, now: i64) -> Vec<UsageWindow> {
    usage
        .windows
        .iter()
        .map(|window| UsageWindow {
            window_minutes: window.window_minutes,
            used_percent: window.used_percent,
            resets_at: window.resets_at,
            expired: window.is_expired(now),
        })
        .collect()
}

fn status_of(windows: &[UsageWindow]) -> UsageStatus {
    if windows.is_empty() {
        UsageStatus::Missing
    } else if windows.iter().all(|window| window.expired) {
        UsageStatus::Expired
    } else {
        UsageStatus::Fresh
    }
}

#[derive(Debug, Serialize)]
pub struct ImportReport<'a> {
    pub imported: usize,
    pub skipped: usize,
    pub failed: usize,
    pub items: &'a [ImportItemResult],
}

impl<'a> ImportReport<'a> {
    pub fn new(items: &'a [ImportItemResult]) -> Self {
        Self {
            imported: count(items, ImportStatus::Imported),
            skipped: count(items, ImportStatus::Skipped),
            failed: count(items, ImportStatus::Failed),
            items,
        }
    }
}

pub fn print_json(value: &impl Serialize) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn count(items: &[ImportItemResult], status: ImportStatus) -> usize {
    items.iter().filter(|item| item.status == status).count()
}

#[cfg(test)]
mod tests {
    use codex_buddy_core::usage::{Usage, Window};

    use super::*;

    fn view(usage: Option<Usage>) -> AccountView {
        AccountView {
            alias: "work".into(),
            email: None,
            plan: None,
            is_active: true,
            usage,
            last_used_at: None,
        }
    }

    #[test]
    fn usage_status_distinguishes_zero_expired_and_missing() {
        let fresh_zero = AccountUsage::from_view(
            &view(Some(Usage {
                windows: vec![Window {
                    window_minutes: 300,
                    used_percent: 0.0,
                    resets_at: Some(101),
                }],
            })),
            100,
        );
        assert_eq!(fresh_zero.status, UsageStatus::Fresh);
        assert_eq!(fresh_zero.windows[0].used_percent, 0.0);

        let expired = AccountUsage::from_view(
            &view(Some(Usage {
                windows: vec![Window {
                    window_minutes: 300,
                    used_percent: 30.0,
                    resets_at: Some(100),
                }],
            })),
            100,
        );
        assert_eq!(expired.status, UsageStatus::Expired);
        assert!(expired.windows[0].expired);

        assert_eq!(
            AccountUsage::from_view(&view(None), 100).status,
            UsageStatus::Missing
        );
    }
}
