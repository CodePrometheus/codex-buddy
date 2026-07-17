use std::fs;

use crate::auth::load_auth_info;
use crate::config_check::{CredentialStore, credential_store};
use crate::layout::SWITCHED_ENTRIES;
use crate::paths::Paths;
use crate::registry;

/// Severity of a single diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Pass,
    Warn,
    Fail,
}

/// One diagnostic line.
#[derive(Debug, Clone)]
pub struct Check {
    pub level: Level,
    pub message: String,
}

/// Read-only health check of the codex-buddy setup. Never mutates anything.
pub fn diagnose(paths: &Paths) -> Vec<Check> {
    let mut out = Vec::new();

    // The whole scheme relies on codex storing credentials as a plain file.
    match credential_store(&paths.codex_config()) {
        Ok(CredentialStore::File) => out.push(pass("credential store is `file`")),
        Ok(other) => out.push(fail(format!(
            "credential store is `{}`; must be `file` (keyring/auto/ephemeral break switching)",
            other.as_str()
        ))),
        Err(e) => out.push(fail(format!("cannot read config.toml: {e}"))),
    }

    // ~/.codex/auth.json must be a managed symlink for switching to work.
    let auth = paths.codex_auth();
    if fs::symlink_metadata(&auth).is_ok_and(|m| m.file_type().is_symlink()) {
        out.push(pass("~/.codex/auth.json is a managed symlink"));
    } else if auth.exists() {
        out.push(warn(
            "~/.codex/auth.json is a real file; run `init` to adopt it",
        ));
    } else {
        out.push(warn("~/.codex/auth.json is missing; run `init`"));
    }

    let reg = match registry::load(&paths.registry_file()) {
        Ok(r) => r,
        Err(e) => {
            out.push(fail(format!("cannot read registry: {e}")));
            return out;
        }
    };
    if reg.accounts.is_empty() {
        out.push(warn("no accounts yet; run `init`"));
        return out;
    }

    // The active account must exist, and every switched entry must point into its dir.
    match reg.active() {
        Some(active) => match reg.find(active) {
            Some(rec) => {
                for &entry in SWITCHED_ENTRIES {
                    let link = paths.codex_home().join(entry);
                    let want = paths.account_dir(&rec.dir).join(entry);
                    match fs::read_link(&link) {
                        Ok(t) if t == want => {
                            out.push(pass(format!("~/.codex/{entry} -> {active}")))
                        }
                        Ok(t) => out.push(warn(format!(
                            "~/.codex/{entry} points at {}, not active account `{active}`",
                            t.display()
                        ))),
                        Err(_) => out.push(warn(format!(
                            "~/.codex/{entry} is not a symlink to active account `{active}`"
                        ))),
                    }
                }
            }
            None => out.push(fail(format!(
                "active account `{active}` is not in the registry"
            ))),
        },
        None => out.push(warn("no active account; run `switch`")),
    }

    // Every account should have a parseable auth.json.
    for rec in &reg.accounts {
        match load_auth_info(&paths.account_auth(&rec.dir)) {
            Ok(_) => out.push(pass(format!("account `{}` auth.json is valid", rec.alias))),
            Err(e) => out.push(warn(format!(
                "account `{}` auth.json is unreadable: {e}",
                rec.alias
            ))),
        }
    }

    out
}

fn pass(message: impl Into<String>) -> Check {
    Check {
        level: Level::Pass,
        message: message.into(),
    }
}

fn warn(message: impl Into<String>) -> Check {
    Check {
        level: Level::Warn,
        message: message.into(),
    }
}

fn fail(message: impl Into<String>) -> Check {
    Check {
        level: Level::Fail,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests;
