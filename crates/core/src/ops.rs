use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use serde::Serialize;

use crate::auth::load_auth_info;
use crate::config_check::ensure_file_store;
use crate::error::{Error, Result};
use crate::layout::{build_account_dir, point_switched_entries};
use crate::paths::{Paths, validate_alias};
use crate::registry::{self, AccountRecord, Registry, now_epoch};

/// Where `codex` was found, plus the `PATH` its process should inherit — npm-style launchers
/// re-resolve `node` and friends through `PATH`, so the absolute program path alone is not enough.
struct CodexLocation {
    program: PathBuf,
    path_var: Option<OsString>,
}

/// A `Command` for the `codex` binary.
///
/// A GUI app launched from Finder/Dock (as the tray is) only inherits the bare system `PATH`,
/// and version managers (nvm/volta/asdf) often extend `PATH` only in interactive rc files that
/// login shells never read. Resolution therefore walks a ladder — inherited `PATH`, login shell
/// `PATH`, interactive shell `PATH`, well-known install directories — and caches the answer for
/// the process lifetime. When nothing is found the bare name is used, so callers see a plain
/// not-found spawn error they can translate for the user.
pub(crate) fn codex_command() -> Command {
    static LOCATION: OnceLock<Option<CodexLocation>> = OnceLock::new();
    match LOCATION.get_or_init(find_codex) {
        Some(location) => {
            let mut cmd = Command::new(&location.program);
            if let Some(path) = &location.path_var {
                cmd.env("PATH", path);
            }
            cmd
        }
        None => Command::new("codex"),
    }
}

/// Translate a `codex` spawn failure into something a user can act on.
pub(crate) fn codex_spawn_error(e: std::io::Error) -> Error {
    if e.kind() == std::io::ErrorKind::NotFound {
        Error::Other("codex CLI not found; install codex and try again".into())
    } else {
        Error::Io(e)
    }
}

fn find_codex() -> Option<CodexLocation> {
    let current = env::var_os("PATH").unwrap_or_default();
    if let Some(program) = search_path_var(&current) {
        return Some(CodexLocation {
            program,
            path_var: None,
        });
    }
    for interactive in [false, true] {
        if let Some(shell_path) = shell_path_var(interactive) {
            let merged = merge_paths(&shell_path, &current);
            if let Some(program) = search_path_var(&merged) {
                return Some(CodexLocation {
                    program,
                    path_var: Some(merged),
                });
            }
        }
    }
    let dir = known_install_dirs()
        .into_iter()
        .find(|dir| dir.join("codex").is_file())?;
    Some(CodexLocation {
        program: dir.join("codex"),
        path_var: Some(merge_paths(dir.as_os_str(), &current)),
    })
}

fn search_path_var(path_var: &OsStr) -> Option<PathBuf> {
    env::split_paths(path_var)
        .map(|dir| dir.join("codex"))
        .find(|candidate| candidate.is_file())
}

fn merge_paths(front: &OsStr, back: &OsStr) -> OsString {
    let mut merged = front.to_os_string();
    if !back.is_empty() {
        merged.push(":");
        merged.push(back);
    }
    merged
}

/// The `PATH` the user's shell computes. Interactive startup files may print their own output
/// (themes, banners), so the value is emitted after a marker byte and everything before the
/// last marker is discarded.
fn shell_path_var(interactive: bool) -> Option<OsString> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let flag = if interactive { "-lic" } else { "-lc" };
    let output = Command::new(&shell)
        .args([flag, r#"printf '\037%s' "$PATH""#])
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let marker = output.stdout.iter().rposition(|&b| b == 0x1f)?;
    let bytes = output.stdout[marker + 1..].to_vec();
    use std::os::unix::ffi::OsStringExt;
    (!bytes.is_empty()).then(|| OsString::from_vec(bytes))
}

fn known_install_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ];
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".volta/bin"));
        dirs.push(home.join(".bun/bin"));
        dirs.extend(nvm_bin_dirs(&home.join(".nvm/versions/node")));
    }
    dirs
}

/// nvm keeps one bin dir per node version; try the newest first.
fn nvm_bin_dirs(versions: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(versions) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path().join("bin")).collect();
    dirs.sort_by_key(|dir| std::cmp::Reverse(nvm_version_key(dir)));
    dirs
}

fn nvm_version_key(bin_dir: &Path) -> Vec<i64> {
    bin_dir
        .parent()
        .and_then(|version| version.file_name())
        .and_then(|name| name.to_str())
        .map(|name| {
            name.trim_start_matches('v')
                .split('.')
                .map(|part| part.parse().unwrap_or(0))
                .collect()
        })
        .unwrap_or_default()
}

/// Look up an account and return its validated dir name. `dir` normally equals the alias, but
/// registry.json is user-editable — refuse anything that isn't a plain directory name before
/// any caller touches the filesystem with it (defense in depth against path traversal).
fn resolve_dir(reg: &Registry, alias: &str) -> Result<String> {
    let rec = reg
        .find(alias)
        .ok_or_else(|| Error::AccountNotFound(alias.to_string()))?;
    validate_alias(&rec.dir)?;
    Ok(rec.dir.clone())
}

/// Display view of an account.
#[derive(Debug, Clone, Serialize)]
pub struct AccountView {
    pub alias: String,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub is_active: bool,
    pub usage: Option<crate::usage::Usage>,
    pub last_used_at: Option<i64>,
}

/// Switch the active account by atomically repointing ~/.codex/auth.json.
///
/// Lookup, symlink repoint, and registry write all happen under the registry lock, so two
/// concurrent switches (tray + CLI) serialize instead of leaving the filesystem pointing at one
/// account while the registry records another.
pub fn switch(paths: &Paths, alias: &str) -> Result<()> {
    registry::update(paths, |r| switch_in_registry(paths, r, alias))
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

/// Switch to the next account in registry order, wrapping at the end.
///
/// Selection and switching happen under one registry lock so another CLI or the tray cannot
/// change the active account between those two steps.
pub fn switch_next(paths: &Paths) -> Result<String> {
    registry::update(paths, |r| {
        let next = match r.active() {
            Some(active) => {
                let index = r
                    .accounts
                    .iter()
                    .position(|a| a.alias == active)
                    .ok_or_else(|| {
                        Error::Other(format!(
                            "active account `{active}` is not present in the registry"
                        ))
                    })?;
                &r.accounts[(index + 1) % r.accounts.len()].alias
            }
            None => {
                &r.accounts
                    .first()
                    .ok_or_else(|| Error::Other("no accounts to switch to".into()))?
                    .alias
            }
        }
        .clone();
        switch_in_registry(paths, r, &next)?;
        Ok(next)
    })
}

fn switch_in_registry(paths: &Paths, reg: &mut Registry, alias: &str) -> Result<()> {
    let dir = resolve_dir(reg, alias)?;
    if !paths.account_auth(&dir).exists() {
        return Err(Error::MissingAuth(format!(
            "account {alias} has no auth.json"
        )));
    }
    // point_switched_entries refuses to clobber a real ~/.codex/auth.json (run `init` first).
    point_switched_entries(paths, &dir)?;
    reg.set_active(alias);
    if let Some(m) = reg.find_mut(alias) {
        m.last_used_at = Some(now_epoch());
    }
    Ok(())
}

/// Run codex under the given account (`CODEX_HOME=<account dir>`), returning its exit code.
/// Args are passed through as `OsString` so non-UTF8 arguments (e.g. filenames) survive intact.
pub fn run(paths: &Paths, alias: &str, args: &[OsString]) -> Result<i32> {
    let reg = registry::load(&paths.registry_file())?;
    let dir = resolve_dir(&reg, alias)?;
    if !paths.account_auth(&dir).exists() {
        return Err(Error::MissingAuth(format!(
            "account {alias} has no auth.json"
        )));
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
        .status()
        .map_err(codex_spawn_error)?;
    Ok(status.code().unwrap_or(1))
}

/// List accounts; email / plan are re-parsed from each id_token, falling back to the registry.
pub fn list(paths: &Paths) -> Result<Vec<AccountView>> {
    let reg = registry::load(&paths.registry_file())?;
    list_from(paths, &reg)
}

/// One account with the same metadata and local usage view returned by [`list`].
pub fn get(paths: &Paths, alias: &str) -> Result<AccountView> {
    let reg = registry::load(&paths.registry_file())?;
    let rec = reg
        .find(alias)
        .ok_or_else(|| Error::AccountNotFound(alias.to_string()))?;
    Ok(view_of(paths, rec, reg.active() == Some(alias), true))
}

/// Same as [`list`], but reuses an already-loaded registry — for callers (like the FFI layer)
/// that also need it for something else and shouldn't read `registry.json` twice.
pub fn list_from(paths: &Paths, reg: &Registry) -> Result<Vec<AccountView>> {
    let active = reg.active().map(str::to_owned);
    Ok(reg
        .accounts
        .iter()
        .map(|rec| {
            let is_active = active.as_deref() == Some(rec.alias.as_str());
            view_of(paths, rec, is_active, true)
        })
        .collect())
}

/// The active account, or None. Skips the usage scan — its callers (`current`, the post-switch
/// summary) print identity only, and shouldn't pay for a sessions walk.
pub fn current(paths: &Paths) -> Result<Option<AccountView>> {
    let reg = registry::load(&paths.registry_file())?;
    let Some(alias) = reg.active() else {
        return Ok(None);
    };
    Ok(reg.find(alias).map(|rec| view_of(paths, rec, true, false)))
}

/// Build one account's display view; email / plan are re-parsed from its id_token, falling back
/// to the registry copy.
fn view_of(paths: &Paths, rec: &AccountRecord, is_active: bool, with_usage: bool) -> AccountView {
    let (email, plan) = match load_auth_info(&paths.account_auth(&rec.dir)) {
        Ok(info) => (
            info.email.or_else(|| rec.email.clone()),
            info.plan.or_else(|| rec.plan.clone()),
        ),
        Err(_) => (rec.email.clone(), rec.plan.clone()),
    };
    let usage = with_usage
        .then(|| {
            crate::usage::latest_usage(&paths.account_dir(&rec.dir).join("sessions"), now_epoch())
        })
        .flatten();
    AccountView {
        alias: rec.alias.clone(),
        email,
        plan,
        is_active,
        usage,
        last_used_at: rec.last_used_at,
    }
}

/// Log in and adopt a new account. Runs interactive `codex login`.
pub fn add(paths: &Paths, alias: &str) -> Result<()> {
    let account_dir = add_prepare(paths, alias)?;
    let status = codex_command()
        .env("CODEX_HOME", &account_dir)
        .arg("login")
        .status()
        .map_err(codex_spawn_error)?;
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
    ensure_account_changes_ready(paths)?;
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

pub(crate) fn ensure_account_changes_ready(paths: &Paths) -> Result<()> {
    ensure_file_store(&paths.codex_config())?;
    let is_symlink = fs::symlink_metadata(paths.codex_auth())
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    if !is_symlink {
        return Err(Error::NotInitialized(
            "run init before adding or importing accounts".into(),
        ));
    }
    Ok(())
}

/// After login: parse the new auth, reject a duplicate account key, write the registry.
/// The duplicate checks run inside the locked registry update; the account dir is cleaned up
/// whenever the account didn't make it into the registry.
fn add_finalize(paths: &Paths, alias: &str) -> Result<()> {
    let info = match load_auth_info(&paths.account_auth(alias)) {
        Ok(i) => i,
        Err(e) => {
            let _ = fs::remove_dir_all(paths.account_dir(alias));
            return Err(Error::Other(format!("no valid auth.json after login: {e}")));
        }
    };
    let key = info.account_key.clone();
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
    let result = registry::update(paths, |r| {
        if let Some(existing) = r.find_by_key(&key) {
            return Err(Error::Other(format!(
                "account already exists as `{}`; not added again",
                existing.alias
            )));
        }
        r.add(record)?;
        Ok(())
    });
    if result.is_err() {
        let _ = fs::remove_dir_all(paths.account_dir(alias));
    }
    result
}

/// Remove an account: delete its dir (which holds the real auth.json) and drop it from the
/// registry. Refuses to remove the active account, which would leave ~/.codex/auth.json dangling.
/// Runs entirely under the registry lock so it can't interleave with a concurrent switch.
pub fn remove(paths: &Paths, alias: &str) -> Result<()> {
    registry::update(paths, |r| {
        let dir = resolve_dir(r, alias)?;
        if r.active() == Some(alias) {
            return Err(Error::Other(format!(
                "{alias} is the active account; switch to another account before removing it"
            )));
        }

        // Delete the dir first: if it fails the account stays intact and removable again.
        match fs::remove_dir_all(paths.account_dir(&dir)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::Io(e)),
        }
        r.remove(alias)?;
        Ok(())
    })
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
    let dir = resolve_dir(&reg, alias)?;
    ensure_file_store(&paths.codex_config())?;
    build_account_dir(paths, &dir)?;

    let status = codex_command()
        .env("CODEX_HOME", paths.account_dir(&dir))
        .arg("login")
        .status()
        .map_err(codex_spawn_error)?;
    if !status.success() {
        return Err(Error::Other("codex login did not succeed".into()));
    }

    let info = load_auth_info(&paths.account_auth(&dir))?;
    registry::update(paths, |r| {
        let m = r
            .find_mut(alias)
            .ok_or_else(|| Error::AccountNotFound(alias.to_string()))?;
        m.account_key = info.account_key;
        m.email = info.email;
        m.plan = info.plan;
        Ok(())
    })
}

/// The CODEX_HOME directory for an account, for manual `CODEX_HOME=… codex`.
pub fn account_home(paths: &Paths, alias: &str) -> Result<PathBuf> {
    let reg = registry::load(&paths.registry_file())?;
    Ok(paths.account_dir(&resolve_dir(&reg, alias)?))
}

/// Rename an account: update the registry alias + dir name, repointing ~/.codex/auth.json
/// when the account is active.
///
/// Runs under the registry lock; if anything fails after the dir moved (repoint or the registry
/// write itself), the move and links are undone so filesystem and registry stay consistent —
/// a dangling active ~/.codex/auth.json would force a re-login.
pub fn rename(paths: &Paths, old: &str, new: &str) -> Result<()> {
    if old == new {
        return Ok(());
    }
    validate_alias(new)?;
    let new_dir = paths.account_dir(new);
    let mut undo: Option<(String, bool)> = None;
    let result = registry::update(paths, |r| {
        let old_dir = resolve_dir(r, old)?;
        if r.find(new).is_some() || new_dir.exists() {
            return Err(Error::AccountExists(new.to_string()));
        }
        let was_active = r.active() == Some(old);

        fs::rename(paths.account_dir(&old_dir), &new_dir)?;
        undo = Some((old_dir, was_active));
        if was_active {
            point_switched_entries(paths, new)?;
        }

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
    });
    if let Err(e) = result {
        if let Some((old_dir, was_active)) = undo {
            let _ = fs::rename(&new_dir, paths.account_dir(&old_dir));
            if was_active {
                let _ = point_switched_entries(paths, &old_dir);
            }
        }
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
