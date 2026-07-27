use std::fs;

use serde::Serialize;

use crate::auth::load_auth_info;
use crate::config_check::{CredentialStore, credential_store};
use crate::layout::SWITCHED_ENTRIES;
use crate::paths::Paths;
use crate::registry;

/// Severity of a single diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Pass,
    Warn,
    Fail,
}

/// One diagnostic line.
#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub code: String,
    pub level: Level,
    pub message: String,
}

/// A compact, non-secret summary of the current setup.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub active_account: Option<String>,
    pub account_count: usize,
    pub pass_count: usize,
    pub warn_count: usize,
    pub fail_count: usize,
    pub checks: Vec<Check>,
}

/// Read-only health check of the codex-buddy setup. Never mutates anything.
pub fn diagnose(paths: &Paths) -> Vec<Check> {
    diagnose_state(paths).1
}

fn diagnose_state(paths: &Paths) -> (Option<registry::Registry>, Vec<Check>) {
    let mut out = Vec::new();

    // The whole scheme relies on codex storing credentials as a plain file.
    match credential_store(&paths.codex_config()) {
        Ok(CredentialStore::File) => {
            out.push(pass("credential_store", "credential store is `file`"))
        }
        Ok(other) => out.push(fail(
            "credential_store",
            format!(
                "credential store is `{}`; must be `file` (keyring/auto/ephemeral break switching)",
                other.as_str()
            ),
        )),
        Err(e) => out.push(fail(
            "credential_store",
            format!("cannot read config.toml: {e}"),
        )),
    }

    // ~/.codex/auth.json must be a managed symlink for switching to work.
    let auth = paths.codex_auth();
    if fs::symlink_metadata(&auth).is_ok_and(|m| m.file_type().is_symlink()) {
        out.push(pass(
            "active_auth_link",
            "~/.codex/auth.json is a managed symlink",
        ));
    } else if auth.exists() {
        out.push(warn(
            "active_auth_link",
            "~/.codex/auth.json is a real file; run `init` to adopt it",
        ));
    } else {
        out.push(warn(
            "active_auth_link",
            "~/.codex/auth.json is missing; run `init`",
        ));
    }

    let reg = match registry::load(&paths.registry_file()) {
        Ok(r) => r,
        Err(e) => {
            out.push(fail("registry", format!("cannot read registry: {e}")));
            return (None, out);
        }
    };
    if reg.accounts.is_empty() {
        out.push(warn("accounts", "no accounts yet; run `init`"));
        return (Some(reg), out);
    }

    // The active account must exist, and every switched entry must point into its dir.
    match reg.active() {
        Some(active) => match reg.find(active) {
            Some(rec) => {
                for &entry in SWITCHED_ENTRIES {
                    let link = paths.codex_home().join(entry);
                    let want = paths.account_dir(&rec.dir).join(entry);
                    match fs::read_link(&link) {
                        Ok(t) if t == want => out.push(pass(
                            format!("switched_entry.{entry}"),
                            format!("~/.codex/{entry} -> {active}"),
                        )),
                        Ok(t) => out.push(warn(
                            format!("switched_entry.{entry}"),
                            format!(
                                "~/.codex/{entry} points at {}, not active account `{active}`",
                                t.display()
                            ),
                        )),
                        Err(_) => out.push(warn(
                            format!("switched_entry.{entry}"),
                            format!(
                                "~/.codex/{entry} is not a symlink to active account `{active}`"
                            ),
                        )),
                    }
                }
            }
            None => out.push(fail(
                "active_account",
                format!("active account `{active}` is not in the registry"),
            )),
        },
        None => out.push(warn("active_account", "no active account; run `switch`")),
    }

    // Every account should have a parseable auth.json.
    for rec in &reg.accounts {
        match load_auth_info(&paths.account_auth(&rec.dir)) {
            Ok(_) => out.push(pass(
                "account_auth",
                format!("account `{}` auth.json is valid", rec.alias),
            )),
            Err(e) => out.push(warn(
                "account_auth",
                format!("account `{}` auth.json is unreadable: {e}", rec.alias),
            )),
        }
    }

    (Some(reg), out)
}

/// Build a compact report from the same checks used by `doctor`.
pub fn report(paths: &Paths) -> Report {
    let (reg, checks) = diagnose_state(paths);
    let pass_count = checks.iter().filter(|c| c.level == Level::Pass).count();
    let warn_count = checks.iter().filter(|c| c.level == Level::Warn).count();
    let fail_count = checks.iter().filter(|c| c.level == Level::Fail).count();
    Report {
        active_account: reg.as_ref().and_then(|r| r.active_account.clone()),
        account_count: reg.as_ref().map_or(0, |r| r.accounts.len()),
        pass_count,
        warn_count,
        fail_count,
        checks,
    }
}

fn pass(code: impl Into<String>, message: impl Into<String>) -> Check {
    Check {
        code: code.into(),
        level: Level::Pass,
        message: message.into(),
    }
}

fn warn(code: impl Into<String>, message: impl Into<String>) -> Check {
    Check {
        code: code.into(),
        level: Level::Warn,
        message: message.into(),
    }
}

fn fail(code: impl Into<String>, message: impl Into<String>) -> Check {
    Check {
        code: code.into(),
        level: Level::Fail,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests;
