//! uniffi FFI surface for the Swift menu bar tray. Every function is a thin, synchronous
//! wrapper around `codex-buddy-core`; no business logic lives here. `add_account` blocks on
//! `codex login` (browser OAuth) — callers must dispatch it off the main thread.

use std::collections::BTreeSet;
use std::path::PathBuf;

use codex_buddy_core::doctor::{self, Level};
use codex_buddy_core::ops::{self, AccountView};
use codex_buddy_core::paths::Paths;
use codex_buddy_core::registry;
use codex_buddy_core::running::running_accounts;

uniffi::setup_scaffolding!();

#[derive(Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum FfiError {
    Failed(String),
}

impl std::fmt::Display for FfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FfiError::Failed(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for FfiError {}

impl From<codex_buddy_core::error::Error> for FfiError {
    fn from(e: codex_buddy_core::error::Error) -> Self {
        FfiError::Failed(e.to_string())
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct UsageWindow {
    pub window_minutes: i64,
    pub used_percent: f64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Account {
    pub alias: String,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub is_active: bool,
    pub is_running: bool,
    pub usage: Vec<UsageWindow>,
}

#[derive(Debug, Clone, Copy, uniffi::Enum)]
pub enum CheckLevel {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct DoctorCheck {
    pub level: CheckLevel,
    pub message: String,
}

/// All accounts, each carrying its active/running state and latest usage — everything a
/// single tray render pass needs, in one call.
#[uniffi::export]
pub fn list_accounts() -> Result<Vec<Account>, FfiError> {
    let paths = Paths::from_env()?;
    let views = ops::list(&paths)?;
    let reg = registry::load(&paths.registry_file())?;
    let running = running_accounts(&paths, &reg);
    Ok(views.into_iter().map(|v| to_account(v, &running)).collect())
}

#[uniffi::export]
pub fn switch_account(alias: String) -> Result<(), FfiError> {
    let paths = Paths::from_env()?;
    ops::switch(&paths, &alias)?;
    Ok(())
}

/// Adopts a new account. Blocks on interactive `codex login` (opens the system browser) —
/// call from a background thread.
#[uniffi::export]
pub fn add_account(alias: String) -> Result<(), FfiError> {
    let paths = Paths::from_env()?;
    ops::add(&paths, &alias)?;
    Ok(())
}

#[uniffi::export]
pub fn remove_account(alias: String) -> Result<(), FfiError> {
    let paths = Paths::from_env()?;
    ops::remove(&paths, &alias)?;
    Ok(())
}

#[uniffi::export]
pub fn rename_account(old_alias: String, new_alias: String) -> Result<(), FfiError> {
    let paths = Paths::from_env()?;
    ops::rename(&paths, &old_alias, &new_alias)?;
    Ok(())
}

#[uniffi::export]
pub fn import_account(auth_json_path: String, alias: String) -> Result<(), FfiError> {
    let paths = Paths::from_env()?;
    ops::import(&paths, &PathBuf::from(auth_json_path), &alias)?;
    Ok(())
}

/// The account's CODEX_HOME, for "copy path" / "run in Terminal" actions.
#[uniffi::export]
pub fn account_home(alias: String) -> Result<String, FfiError> {
    let paths = Paths::from_env()?;
    Ok(ops::account_home(&paths, &alias)?
        .to_string_lossy()
        .into_owned())
}

#[uniffi::export]
pub fn doctor() -> Result<Vec<DoctorCheck>, FfiError> {
    let paths = Paths::from_env()?;
    Ok(doctor::diagnose(&paths)
        .into_iter()
        .map(to_doctor_check)
        .collect())
}

fn to_account(v: AccountView, running: &BTreeSet<String>) -> Account {
    Account {
        is_running: running.contains(&v.alias),
        usage: v
            .usage
            .map(|u| {
                u.windows
                    .into_iter()
                    .map(|w| UsageWindow {
                        window_minutes: w.window_minutes,
                        used_percent: w.used_percent,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        alias: v.alias,
        email: v.email,
        plan: v.plan,
        is_active: v.is_active,
    }
}

fn to_doctor_check(c: doctor::Check) -> DoctorCheck {
    DoctorCheck {
        level: match c.level {
            Level::Pass => CheckLevel::Pass,
            Level::Warn => CheckLevel::Warn,
            Level::Fail => CheckLevel::Fail,
        },
        message: c.message,
    }
}

#[cfg(test)]
mod tests;
