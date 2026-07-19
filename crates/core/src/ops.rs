use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::auth::load_auth_info;
use crate::config_check::ensure_file_store;
use crate::error::{Error, Result};
use crate::layout::{build_account_dir, point_switched_entries};
use crate::paths::{Paths, validate_alias};
use crate::registry::{self, AccountRecord, Registry, now_epoch};

/// A `Command` for the `codex` binary, with `PATH` widened to the user's login shell if `codex`
/// isn't already reachable through the inherited one.
///
/// A GUI app launched from Finder/Dock (as the tray will be) only inherits the bare system
/// `PATH` (`/usr/bin:/bin:/usr/sbin:/sbin`) — none of the Homebrew/nvm/asdf directories a
/// terminal session picks up from shell profile files. `codex-buddy` itself runs fine either
/// way; it's specifically this child process that needs the fuller PATH to find `codex`.
fn codex_command() -> Command {
    let mut cmd = Command::new("codex");
    if !is_on_path("codex")
        && let Some(path) = login_shell_path()
    {
        cmd.env("PATH", path);
    }
    cmd
}

fn is_on_path(bin: &str) -> bool {
    is_on(bin, &env::var_os("PATH").unwrap_or_default())
}

fn is_on(bin: &str, path_var: &OsStr) -> bool {
    env::split_paths(path_var).any(|dir| dir.join(bin).is_file())
}

/// The `PATH` a login shell would compute (sourcing `.zprofile`/`.profile` etc.), prepended to
/// the current one. None if the shell can't be run or reports nothing.
fn login_shell_path() -> Option<String> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let output = Command::new(&shell)
        .args(["-lc", "echo -n \"$PATH\""])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let shell_path = String::from_utf8(output.stdout).ok()?;
    let shell_path = shell_path.trim();
    if shell_path.is_empty() {
        return None;
    }
    let current = env::var("PATH").unwrap_or_default();
    Some(format!("{shell_path}:{current}"))
}

/// Display view of an account.
#[derive(Debug, Clone)]
pub struct AccountView {
    pub alias: String,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub account_key: String,
    pub is_active: bool,
    pub usage: Option<crate::usage::Usage>,
    pub last_used_at: Option<i64>,
}

/// Switch the active account by atomically repointing ~/.codex/auth.json.
pub fn switch(paths: &Paths, alias: &str) -> Result<()> {
    let reg = registry::load(&paths.registry_file())?;
    let rec = reg
        .find(alias)
        .ok_or_else(|| Error::AccountNotFound(alias.to_string()))?;
    let dir = rec.dir.clone();
    if !paths.account_auth(&dir).exists() {
        return Err(Error::Other(format!("account {alias} has no auth.json")));
    }

    // point_switched_entries refuses to clobber a real ~/.codex/auth.json (run `init` first).
    point_switched_entries(paths, &dir)?;

    registry::update(paths, |r| {
        r.set_active(alias);
        if let Some(m) = r.find_mut(alias) {
            m.last_used_at = Some(now_epoch());
        }
        Ok(())
    })?;
    Ok(())
}

/// Switch back to the previous account (`switch -`).
pub fn switch_previous(paths: &Paths) -> Result<()> {
    let reg = registry::load(&paths.registry_file())?;
    let prev = reg
        .previous()
        .ok_or_else(|| Error::Other("no previous account to switch back to".into()))?
        .to_string();
    switch(paths, &prev)
}

/// Run codex under the given account (`CODEX_HOME=<account dir>`), returning its exit code.
pub fn run(paths: &Paths, alias: &str, args: &[String]) -> Result<i32> {
    let reg = registry::load(&paths.registry_file())?;
    let rec = reg
        .find(alias)
        .ok_or_else(|| Error::AccountNotFound(alias.to_string()))?;
    let dir = rec.dir.clone();
    if !paths.account_auth(&dir).exists() {
        return Err(Error::Other(format!("account {alias} has no auth.json")));
    }

    build_account_dir(paths, &dir)?;

    let _ = registry::update(paths, |r| {
        if let Some(m) = r.find_mut(alias) {
            m.last_used_at = Some(now_epoch());
        }
        Ok(())
    });

    let status = codex_command()
        .env("CODEX_HOME", paths.account_dir(&dir))
        .args(args)
        .status()?;
    Ok(status.code().unwrap_or(1))
}

/// List accounts; email / plan are re-parsed from each id_token, falling back to the registry.
pub fn list(paths: &Paths) -> Result<Vec<AccountView>> {
    let reg = registry::load(&paths.registry_file())?;
    list_from(paths, &reg)
}

/// Same as [`list`], but reuses an already-loaded registry — for callers (like the FFI layer)
/// that also need it for something else and shouldn't read `registry.json` twice.
pub fn list_from(paths: &Paths, reg: &Registry) -> Result<Vec<AccountView>> {
    let active = reg.active().map(str::to_owned);
    let mut out = Vec::with_capacity(reg.accounts.len());
    for rec in &reg.accounts {
        let (email, plan) = match load_auth_info(&paths.account_auth(&rec.dir)) {
            Ok(info) => (
                info.email.or_else(|| rec.email.clone()),
                info.plan.or_else(|| rec.plan.clone()),
            ),
            Err(_) => (rec.email.clone(), rec.plan.clone()),
        };
        let usage = crate::usage::latest_usage(&paths.account_dir(&rec.dir).join("sessions"));
        out.push(AccountView {
            alias: rec.alias.clone(),
            email,
            plan,
            account_key: rec.account_key.clone(),
            is_active: active.as_deref() == Some(rec.alias.as_str()),
            usage,
            last_used_at: rec.last_used_at,
        });
    }
    Ok(out)
}

/// The active account, or None.
pub fn current(paths: &Paths) -> Result<Option<AccountView>> {
    Ok(list(paths)?.into_iter().find(|v| v.is_active))
}

/// Log in and adopt a new account. Runs interactive `codex login`.
pub fn add(paths: &Paths, alias: &str) -> Result<()> {
    let account_dir = add_prepare(paths, alias)?;
    let status = codex_command()
        .env("CODEX_HOME", &account_dir)
        .arg("login")
        .status()?;
    if !status.success() {
        let _ = fs::remove_dir_all(&account_dir);
        return Err(Error::Other(
            "codex login did not succeed; add cancelled".into(),
        ));
    }
    add_finalize(paths, alias)
}

/// Validate and build the account dir (no login yet). Returns the dir to use as CODEX_HOME.
fn add_prepare(paths: &Paths, alias: &str) -> Result<PathBuf> {
    validate_alias(alias)?;
    ensure_file_store(&paths.codex_config())?;
    // Require init: ~/.codex/auth.json must already be a managed symlink.
    let is_symlink = fs::symlink_metadata(paths.codex_auth())
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    if !is_symlink {
        return Err(Error::Other("run init before add".into()));
    }
    let reg = registry::load(&paths.registry_file())?;
    if reg.find(alias).is_some() {
        return Err(Error::AccountExists(alias.to_string()));
    }
    let account_dir = paths.account_dir(alias);
    if account_dir.exists() {
        return Err(Error::AccountExists(alias.to_string()));
    }
    paths.ensure_buddy_home()?;
    fs::create_dir_all(&account_dir)?;
    build_account_dir(paths, alias)?;
    Ok(account_dir)
}

/// After login: parse the new auth, reject a duplicate account key, write the registry.
/// Cleans up the account dir on failure.
fn add_finalize(paths: &Paths, alias: &str) -> Result<()> {
    let info = match load_auth_info(&paths.account_auth(alias)) {
        Ok(i) => i,
        Err(e) => {
            let _ = fs::remove_dir_all(paths.account_dir(alias));
            return Err(Error::Other(format!("no valid auth.json after login: {e}")));
        }
    };
    let reg = registry::load(&paths.registry_file())?;
    if let Some(existing) = reg.find_by_key(&info.account_key) {
        let existing_alias = existing.alias.clone();
        let _ = fs::remove_dir_all(paths.account_dir(alias));
        return Err(Error::Other(format!(
            "account already exists as `{existing_alias}`; not added again"
        )));
    }
    let now = now_epoch();
    let record = AccountRecord {
        alias: alias.to_string(),
        account_key: info.account_key,
        email: info.email,
        plan: info.plan,
        dir: alias.to_string(),
        added_at: now,
        last_used_at: None,
    };
    registry::update(paths, |r| {
        r.add(record)?;
        Ok(())
    })?;
    Ok(())
}

/// Remove an account: delete its dir (which holds the real auth.json) and drop it from the
/// registry. Refuses to remove the active account, which would leave ~/.codex/auth.json dangling.
pub fn remove(paths: &Paths, alias: &str) -> Result<()> {
    let reg = registry::load(&paths.registry_file())?;
    let rec = reg
        .find(alias)
        .ok_or_else(|| Error::AccountNotFound(alias.to_string()))?;
    if reg.active() == Some(alias) {
        return Err(Error::Other(format!(
            "{alias} is the active account; switch to another account before removing it"
        )));
    }
    let dir = rec.dir.clone();
    // Defense in depth: `dir` only ever comes from a validated alias in normal operation, but
    // registry.json is user-editable, and remove_dir_all is destructive — refuse anything that
    // isn't a plain directory name before touching the filesystem.
    validate_alias(&dir)?;

    // Delete the dir first: if it fails the account stays intact and removable again.
    match fs::remove_dir_all(paths.account_dir(&dir)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(Error::Io(e)),
    }
    registry::update(paths, |r| {
        r.remove(alias)?;
        Ok(())
    })?;
    Ok(())
}

/// Import an account from an existing auth.json file: copy it into a fresh account dir and
/// register it (no login). Shares the build / parse / cleanup path with `add`.
pub fn import(paths: &Paths, src: &Path, alias: &str) -> Result<()> {
    let account_dir = add_prepare(paths, alias)?;
    // fs::copy carries over src's permission bits, which may be looser than we want for a
    // credential file; pin it to owner-only regardless of how the source file was created.
    if let Err(e) = fs::copy(src, paths.account_auth(alias)).and_then(|_| {
        fs::set_permissions(paths.account_auth(alias), fs::Permissions::from_mode(0o600))
    }) {
        let _ = fs::remove_dir_all(&account_dir);
        return Err(Error::Io(e));
    }
    add_finalize(paths, alias)
}

/// Re-login an existing account (e.g. after its token expired): run `codex login` with
/// CODEX_HOME set to the account dir, then refresh the registry metadata from the new auth.json.
pub fn relogin(paths: &Paths, alias: &str) -> Result<()> {
    let reg = registry::load(&paths.registry_file())?;
    let rec = reg
        .find(alias)
        .ok_or_else(|| Error::AccountNotFound(alias.to_string()))?;
    let dir = rec.dir.clone();
    ensure_file_store(&paths.codex_config())?;
    build_account_dir(paths, &dir)?;

    let status = codex_command()
        .env("CODEX_HOME", paths.account_dir(&dir))
        .arg("login")
        .status()?;
    if !status.success() {
        return Err(Error::Other("codex login did not succeed".into()));
    }

    let info = load_auth_info(&paths.account_auth(&dir))?;
    registry::update(paths, |r| {
        if let Some(m) = r.find_mut(alias) {
            m.account_key = info.account_key;
            m.email = info.email;
            m.plan = info.plan;
        }
        Ok(())
    })?;
    Ok(())
}

/// The CODEX_HOME directory for an account, for manual `CODEX_HOME=… codex`.
pub fn account_home(paths: &Paths, alias: &str) -> Result<PathBuf> {
    let reg = registry::load(&paths.registry_file())?;
    let rec = reg
        .find(alias)
        .ok_or_else(|| Error::AccountNotFound(alias.to_string()))?;
    Ok(paths.account_dir(&rec.dir))
}

/// Rename an account: update the registry alias + dir name, repointing ~/.codex/auth.json
/// when the account is active.
pub fn rename(paths: &Paths, old: &str, new: &str) -> Result<()> {
    if old == new {
        return Ok(());
    }
    validate_alias(new)?;
    let reg = registry::load(&paths.registry_file())?;
    if reg.find(old).is_none() {
        return Err(Error::AccountNotFound(old.to_string()));
    }
    if reg.find(new).is_some() {
        return Err(Error::AccountExists(new.to_string()));
    }
    let new_dir = paths.account_dir(new);
    if new_dir.exists() {
        return Err(Error::AccountExists(new.to_string()));
    }
    let was_active = reg.active() == Some(old);

    fs::rename(paths.account_dir(old), &new_dir)?;

    // If repointing the active links at the new dir fails, undo the move so the active account's
    // ~/.codex/auth.json is never left dangling (which would force a re-login).
    if was_active && let Err(e) = point_switched_entries(paths, new) {
        let _ = fs::rename(&new_dir, paths.account_dir(old));
        let _ = point_switched_entries(paths, old);
        return Err(e);
    }

    registry::update(paths, |r| {
        if let Some(rec) = r.find_mut(old) {
            rec.alias = new.to_string();
            rec.dir = new.to_string();
        }
        if r.active_account.as_deref() == Some(old) {
            r.active_account = Some(new.to_string());
        }
        if r.previous_account.as_deref() == Some(old) {
            r.previous_account = Some(new.to_string());
        }
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests;
