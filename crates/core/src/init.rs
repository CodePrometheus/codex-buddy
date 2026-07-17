use std::fs::{self, File};
use std::path::{Path, PathBuf};

use crate::auth::load_auth_info;
use crate::config_check::ensure_file_store;
use crate::error::{Error, Result};
use crate::layout::{SWITCHED_ENTRIES, build_account_dir, point_switched_entries};
use crate::paths::{Paths, validate_alias};
use crate::registry::{self, AccountRecord, now_epoch};

/// The changes init will make (read-only, for display + confirmation).
#[derive(Debug, Clone)]
pub struct InitPlan {
    pub alias: String,
    pub account_key: String,
    pub email: Option<String>,
    pub plan: Option<String>,
    /// Currently a real ~/.codex/auth.json; becomes a symlink after migration.
    pub codex_auth: PathBuf,
    pub account_dir: PathBuf,
    pub account_auth: PathBuf,
    pub backup_path: PathBuf,
}

/// Collect the migration plan. Read-only, makes no changes.
pub fn plan(paths: &Paths, alias: &str) -> Result<InitPlan> {
    validate_alias(alias)?;
    ensure_file_store(&paths.codex_config())?;

    let codex_auth = paths.codex_auth();
    let meta = fs::symlink_metadata(&codex_auth).map_err(|_| {
        Error::Other(format!(
            "{} not found; run codex login first",
            codex_auth.display()
        ))
    })?;
    if meta.file_type().is_symlink() {
        return Err(Error::Other(
            "~/.codex/auth.json is already a symlink (already initialized; use list / switch)"
                .into(),
        ));
    }

    let reg = registry::load(&paths.registry_file())?;
    if !reg.accounts.is_empty() {
        return Err(Error::Other(
            "codex-buddy is already initialized; use `add` for a new account".into(),
        ));
    }

    let account_dir = paths.account_dir(alias);
    if account_dir.exists() {
        return Err(Error::AccountExists(alias.to_string()));
    }

    let info = load_auth_info(&codex_auth)?;
    let backup_path = paths
        .backup_dir()
        .join(format!("auth.json.{}.bak", now_epoch()));

    Ok(InitPlan {
        alias: alias.to_string(),
        account_key: info.account_key,
        email: info.email,
        plan: info.plan,
        codex_auth,
        account_dir,
        account_auth: paths.account_auth(alias),
        backup_path,
    })
}

/// Execute the migration, rolling back on any failure. The only operation that modifies existing
/// ~/.codex data: moves auth.json / sessions / history.jsonl into the account dir and replaces
/// ~/.codex/<each> with a symlink to it.
pub fn apply(paths: &Paths, plan: &InitPlan) -> Result<()> {
    ensure_file_store(&paths.codex_config())?;
    let meta = fs::symlink_metadata(&plan.codex_auth)
        .map_err(|_| Error::Other("~/.codex/auth.json not found; cannot migrate".into()))?;
    if meta.file_type().is_symlink() {
        return Err(Error::Other(
            "~/.codex/auth.json is already a symlink (already initialized)".into(),
        ));
    }
    if plan.account_dir.exists() {
        return Err(Error::AccountExists(plan.alias.clone()));
    }

    match apply_inner(paths, plan) {
        Ok(()) => Ok(()),
        Err(e) => {
            rollback(paths, plan);
            Err(e)
        }
    }
}

fn apply_inner(paths: &Paths, plan: &InitPlan) -> Result<()> {
    fs::create_dir_all(paths.backup_dir())?;
    copy_file_synced(&plan.codex_auth, &plan.backup_path)?;

    fs::create_dir_all(&plan.account_dir)?;
    build_account_dir(paths, &plan.alias)?;

    // Move the switched entries (auth.json / sessions / history.jsonl) into the account dir.
    for &entry in SWITCHED_ENTRIES {
        let src = paths.codex_home().join(entry);
        let dst = plan.account_dir.join(entry);
        if src.exists() && !dst.exists() {
            fs::rename(&src, &dst)?;
        }
    }

    // Replace ~/.codex/{auth,sessions,history} with symlinks to this account.
    point_switched_entries(paths, &plan.alias)?;

    let now = now_epoch();
    let record = AccountRecord {
        alias: plan.alias.clone(),
        account_key: plan.account_key.clone(),
        email: plan.email.clone(),
        plan: plan.plan.clone(),
        dir: plan.alias.clone(),
        added_at: now,
        last_used_at: Some(now),
    };
    registry::update(paths, |reg| {
        reg.add(record)?;
        reg.set_active(&plan.alias);
        Ok(())
    })?;
    Ok(())
}

/// Restore ~/.codex/{auth,sessions,history} to real entries and remove the account dir.
///
/// Idempotent; only restores an entry whose ~/.codex/<entry> became a symlink or vanished, so
/// the shared ~/.codex data is never touched. auth comes from the backup; sessions/history move
/// back out of the account dir.
fn rollback(paths: &Paths, plan: &InitPlan) {
    for &entry in SWITCHED_ENTRIES {
        let live = paths.codex_home().join(entry);
        let needs_restore = match fs::symlink_metadata(&live) {
            Ok(m) => m.file_type().is_symlink(),
            Err(_) => true,
        };
        if !needs_restore {
            continue;
        }
        let _ = fs::remove_file(&live);
        if entry == "auth.json" {
            let _ = fs::copy(&plan.backup_path, &live);
        } else {
            let moved = plan.account_dir.join(entry);
            if moved.exists() {
                let _ = fs::rename(&moved, &live);
            }
        }
    }
    let _ = fs::remove_dir_all(&plan.account_dir);
}

fn copy_file_synced(src: &Path, dst: &Path) -> Result<()> {
    fs::copy(src, dst)?;
    if let Ok(f) = File::open(dst) {
        let _ = f.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests;
