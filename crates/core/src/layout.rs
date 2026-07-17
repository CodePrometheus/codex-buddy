use std::fs;
use std::os::unix::fs as unixfs;
use std::path::Path;

use crate::error::{Error, Result};
use crate::paths::Paths;

/// Whether a ~/.codex entry must be isolated (never symlinked into an account dir).
///
/// `auth.json` is the per-account credential; `sessions` and `history.jsonl` are per-account so
/// usage/history don't mix across accounts; `sqlite` dirs and `*.sqlite` files are databases,
/// unsafe to share under symlink + concurrent access.
pub fn is_isolated_entry(name: &str) -> bool {
    matches!(name, "auth.json" | "sqlite" | "sessions" | "history.jsonl")
        || name.contains(".sqlite")
}

/// Entries kept per-account and switched by repointing `~/.codex/<entry>` (like auth.json).
pub const SWITCHED_ENTRIES: &[&str] = &["auth.json", "sessions", "history.jsonl"];

/// Point `~/.codex/{auth.json, sessions, history.jsonl}` at `alias`, creating a missing sessions
/// dir / history file so no symlink dangles. The account's auth.json must already exist.
pub fn point_switched_entries(paths: &Paths, alias: &str) -> Result<()> {
    let acct = paths.account_dir(alias);
    for &entry in SWITCHED_ENTRIES {
        let target = acct.join(entry);
        // The account side must be a real entity. A symlink here (e.g. a stale reverse link
        // back into ~/.codex from an older layout) would make the repoint below point at itself.
        if fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
            fs::remove_file(&target)?;
        }
        ensure_switched_target(&target, entry)?;
        atomic_symlink(&paths.codex_home().join(entry), &target)?;
    }
    Ok(())
}

fn ensure_switched_target(target: &Path, entry: &str) -> Result<()> {
    if target.exists() {
        return Ok(());
    }
    match entry {
        "auth.json" => Err(Error::Other(format!(
            "account is missing auth.json: {}",
            target.display()
        ))),
        "sessions" => {
            fs::create_dir_all(target)?;
            Ok(())
        }
        _ => {
            if let Some(p) = target.parent() {
                fs::create_dir_all(p)?;
            }
            fs::File::create(target)?;
            Ok(())
        }
    }
}

/// Atomically make `link` a symlink to `target` (temp symlink + rename).
///
/// Used to repoint ~/.codex/auth.json on switch, and to rebuild shared links.
pub fn atomic_symlink(link: &Path, target: &Path) -> Result<()> {
    let dir = link
        .parent()
        .ok_or_else(|| Error::Other(format!("symlink path has no parent: {}", link.display())))?;
    fs::create_dir_all(dir)?;
    let file_name = link
        .file_name()
        .ok_or_else(|| Error::Other(format!("symlink path has no file name: {}", link.display())))?
        .to_string_lossy()
        .into_owned();
    let tmp = dir.join(format!(".{file_name}.tmp.{}", std::process::id()));
    let _ = fs::remove_file(&tmp);
    unixfs::symlink(target, &tmp)?;
    fs::rename(&tmp, link).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        Error::Io(e)
    })?;
    Ok(())
}

/// Build (idempotently) an account dir's shared symlinks.
///
/// Symlinks every non-isolated ~/.codex entry back into the account dir, then self-heals by
/// dropping any leftover symlink for an entry that should now be isolated.
pub fn build_account_dir(paths: &Paths, alias: &str) -> Result<()> {
    let acct = paths.account_dir(alias);
    fs::create_dir_all(&acct)?;
    let codex = paths.codex_home();

    for entry in fs::read_dir(codex)? {
        let entry = entry?;
        let name = entry.file_name();
        if is_isolated_entry(&name.to_string_lossy()) {
            continue;
        }
        ensure_shared_symlink(&acct.join(&name), &codex.join(&name))?;
    }

    for entry in fs::read_dir(&acct)? {
        let entry = entry?;
        let name = entry.file_name();
        if !is_isolated_entry(&name.to_string_lossy()) {
            continue;
        }
        let path = entry.path();
        let is_symlink = fs::symlink_metadata(&path)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if is_symlink {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn ensure_shared_symlink(link: &Path, target: &Path) -> Result<()> {
    if let Ok(meta) = fs::symlink_metadata(link) {
        if meta.file_type().is_symlink() {
            if let Ok(cur) = fs::read_link(link)
                && cur == target
            {
                return Ok(());
            }
            fs::remove_file(link)?;
        } else {
            return Err(Error::Other(format!(
                "account dir entry is not a symlink, refusing to overwrite: {}",
                link.display()
            )));
        }
    }
    atomic_symlink(link, target)
}

#[cfg(test)]
mod tests;
