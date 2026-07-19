use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use crate::paths::Paths;
use crate::registry::Registry;

/// Aliases with a live `codex` process holding a file open under their CODEX_HOME.
///
/// Best-effort: shells out to `lsof` per account rather than reading process environments,
/// since macOS no longer exposes another process's env to `ps`/`sysctl` for a same-user,
/// non-root caller. `lsof +D` does not follow the symlinked shared entries (sessions,
/// config.toml, …) inside an account dir, so it only ever reports that account's own isolated
/// files — catches both `codex-buddy run` and a manual `CODEX_HOME=… codex`. Returns an empty
/// set on any failure; this only feeds an optional UI indicator, never switching or data
/// integrity.
pub fn running_accounts(paths: &Paths, reg: &Registry) -> BTreeSet<String> {
    reg.accounts
        .iter()
        .filter(|rec| has_codex_handle(&paths.account_dir(&rec.dir)))
        .map(|rec| rec.alias.clone())
        .collect()
}

/// Whether a process named exactly `codex` holds a file open under `dir`.
fn has_codex_handle(dir: &Path) -> bool {
    let Ok(output) = Command::new("lsof").arg("+D").arg(dir).output() else {
        return false;
    };
    lsof_lists_command(&String::from_utf8_lossy(&output.stdout), "codex")
}

/// Whether any row of `lsof` output (past the header line) has `name` as its COMMAND column.
fn lsof_lists_command(output: &str, name: &str) -> bool {
    output
        .lines()
        .skip(1)
        .any(|line| line.split_whitespace().next() == Some(name))
}

#[cfg(test)]
mod tests;
