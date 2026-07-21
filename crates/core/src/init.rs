use std::fs::{self, File};
use std::path::{Path, PathBuf};

use crate::auth::{AuthInfo, load_auth_info};
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
    /// ~/.codex entries that will move into the account dir and become symlinks — derived from
    /// [`SWITCHED_ENTRIES`] so what's displayed can never drift from what `apply` does.
    pub moves: Vec<String>,
}

/// The alias-independent checks `plan` runs, exposed so interactive callers can fail fast
/// (nothing to adopt, already initialized) before prompting the user for a name. Returns the
/// identity parsed from the auth.json that would be adopted.
pub fn preflight(paths: &Paths) -> Result<AuthInfo> {
    ensure_file_store(&paths.codex_config())?;

    let codex_auth = paths.codex_auth();
    let meta = match fs::symlink_metadata(&codex_auth) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::MissingAuth(format!(
                "{} not found; run codex login first",
                codex_auth.display()
            )));
        }
        Err(e) => return Err(Error::Io(e)),
    };
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

    load_auth_info(&codex_auth)
}

/// Collect the migration plan. Read-only, makes no changes.
pub fn plan(paths: &Paths, alias: &str) -> Result<InitPlan> {
    validate_alias(alias)?;
    let info = preflight(paths)?;

    let codex_auth = paths.codex_auth();
    let account_dir = paths.account_dir(alias);
    if account_dir.exists() {
        return Err(Error::AccountExists(alias.to_string()));
    }

    let backup_path = paths
        .backup_dir()
        .join(format!("auth.json.{}.bak", now_epoch()));

    let moves = SWITCHED_ENTRIES
        .iter()
        .filter(|e| paths.codex_home().join(e).exists())
        .map(|e| e.to_string())
        .collect();

    Ok(InitPlan {
        alias: alias.to_string(),
        account_key: info.account_key,
        email: info.email,
        plan: info.plan,
        codex_auth,
        account_dir,
        account_auth: paths.account_auth(alias),
        backup_path,
        moves,
    })
}

/// Execute the migration, rolling back on any failure. The only operation that modifies existing
/// ~/.codex data: moves auth.json / sessions / history.jsonl into the account dir and replaces
/// ~/.codex/<each> with a symlink to it.
pub fn apply(paths: &Paths, plan: &InitPlan) -> Result<()> {
    ensure_file_store(&paths.codex_config())?;
    let meta = match fs::symlink_metadata(&plan.codex_auth) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::MissingAuth(
                "~/.codex/auth.json not found; cannot migrate".into(),
            ));
        }
        Err(e) => return Err(Error::Io(e)),
    };
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
        Err(e) => Err(match rollback(paths, plan) {
            Ok(()) => e,
            Err(detail) => Error::Other(format!("{e}; rollback incomplete: {detail}")),
        }),
    }
}

fn apply_inner(paths: &Paths, plan: &InitPlan) -> Result<()> {
    paths.ensure_buddy_home()?;
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
/// back out of the account dir. The account dir is deleted only when every entry restored:
/// sessions/history have no backup copy, so after a partial restore the account dir is their
/// sole surviving copy and must be left in place for manual recovery.
fn rollback(paths: &Paths, plan: &InitPlan) -> std::result::Result<(), String> {
    let mut failed = Vec::new();
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
        let restored = if entry == "auth.json" {
            fs::copy(&plan.backup_path, &live).is_ok()
        } else {
            let moved = plan.account_dir.join(entry);
            !moved.exists() || fs::rename(&moved, &live).is_ok()
        };
        if !restored {
            failed.push(entry);
        }
    }
    if failed.is_empty() {
        let _ = fs::remove_dir_all(&plan.account_dir);
        Ok(())
    } else {
        Err(format!(
            "could not restore {} into {}; the moved data is preserved in {} — restore it manually",
            failed.join(", "),
            paths.codex_home().display(),
            plan.account_dir.display()
        ))
    }
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
