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
///
/// Phase 1 validates every ~/.codex entry is safe to repoint (a managed symlink, or absent) before
/// touching anything, so a real file/dir is never clobbered and a failure can't leave only some
/// entries repointed. The repoints in phase 2 are each atomic but not atomic as a group: a crash
/// mid-loop can leave auth.json switched while sessions/history are not. auth.json is the
/// login-critical entry and is repointed first; `doctor` flags the mismatch and re-running switch
/// heals it.
pub fn point_switched_entries(paths: &Paths, alias: &str) -> Result<()> {
    let acct = paths.account_dir(alias);

    // Phase 1: refuse to overwrite a real ~/.codex entry; only `init` may adopt real data.
    for &entry in SWITCHED_ENTRIES {
        refuse_real_entry(paths, entry)?;
    }

    // Phase 2: repoint each entry (each atomic; see the group-atomicity note above).
    for &entry in SWITCHED_ENTRIES {
        let target = acct.join(entry);
        // The account side must be a real entity. A symlink here (e.g. a stale reverse link
        // back into ~/.codex from an older layout) would make the repoint below point at itself.
        if fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
            fs::remove_file(&target)?;
        }
        ensure_switched_target(&target, entry)?;
        // Re-check just before the swap: a codex token refresh may have replaced the entry with
        // a real file since phase 1 looked (fully closing the race needs renamex_np(RENAME_EXCL)).
        refuse_real_entry(paths, entry)?;
        atomic_symlink(&paths.codex_home().join(entry), &target)?;
    }
    Ok(())
}

/// Error unless `~/.codex/<entry>` is a managed symlink or absent — a real file/dir there is
/// user data only `init` may adopt.
fn refuse_real_entry(paths: &Paths, entry: &str) -> Result<()> {
    let live = paths.codex_home().join(entry);
    if let Ok(m) = fs::symlink_metadata(&live)
        && !m.file_type().is_symlink()
    {
        let kind = if m.file_type().is_dir() {
            "directory"
        } else {
            "file"
        };
        return Err(Error::NotInitialized(format!(
            "~/.codex/{entry} is a real {kind}, not a managed symlink; run `init` first"
        )));
    }
    Ok(())
}

fn ensure_switched_target(target: &Path, entry: &str) -> Result<()> {
    if target.exists() {
        return Ok(());
    }
    match entry {
        "auth.json" => Err(Error::MissingAuth(format!(
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
