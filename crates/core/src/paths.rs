use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// The set of paths codex-buddy works with.
#[derive(Debug, Clone)]
pub struct Paths {
    codex_home: PathBuf,
    buddy_home: PathBuf,
}

impl Paths {
    /// Resolve from the environment: `CODEX_HOME` or `~/.codex`, and `~/.codex-buddy`.
    pub fn from_env() -> Result<Self> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or_else(|| Error::Other("HOME is not set".into()))?;
        let codex_home = match env::var_os("CODEX_HOME") {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => home.join(".codex"),
        };
        let buddy_home = home.join(".codex-buddy");
        Ok(Self {
            codex_home,
            buddy_home,
        })
    }

    /// Construct with explicit roots. Mainly for tests.
    pub fn with_roots(codex_home: impl Into<PathBuf>, buddy_home: impl Into<PathBuf>) -> Self {
        Self {
            codex_home: codex_home.into(),
            buddy_home: buddy_home.into(),
        }
    }

    pub fn codex_home(&self) -> &Path {
        &self.codex_home
    }

    pub fn buddy_home(&self) -> &Path {
        &self.buddy_home
    }

    /// codex's active credential; a symlink to the current account's auth.json after init.
    pub fn codex_auth(&self) -> PathBuf {
        self.codex_home.join("auth.json")
    }

    pub fn codex_config(&self) -> PathBuf {
        self.codex_home.join("config.toml")
    }

    pub fn registry_file(&self) -> PathBuf {
        self.buddy_home.join("registry.json")
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.buddy_home.join("backups")
    }

    /// An account's directory (its CODEX_HOME).
    pub fn account_dir(&self, alias: &str) -> PathBuf {
        self.buddy_home.join(alias)
    }

    pub fn account_auth(&self, alias: &str) -> PathBuf {
        self.account_dir(alias).join("auth.json")
    }

    /// Create `~/.codex-buddy` (if missing) and lock it down to owner-only (0700), so nothing
    /// beneath it — registry.json, backups, account credentials — is reachable by other local
    /// users regardless of their individual permissions. Idempotent; safe to call repeatedly.
    pub fn ensure_buddy_home(&self) -> Result<()> {
        fs::create_dir_all(&self.buddy_home)?;
        fs::set_permissions(&self.buddy_home, fs::Permissions::from_mode(0o700))?;
        Ok(())
    }
}

/// Validate that an alias is safe as a directory name: non-empty, no path separators,
/// not `.`/`..`, not a reserved name. Guards against path traversal.
pub fn validate_alias(alias: &str) -> Result<()> {
    if alias.is_empty() {
        return Err(Error::Other("account alias must not be empty".into()));
    }
    if alias.starts_with('.') {
        return Err(Error::Other(format!(
            "account alias must not start with '.': {alias}"
        )));
    }
    if alias.contains('/') || alias.contains('\\') || alias.contains('\0') {
        return Err(Error::Other(format!(
            "account alias must not contain path separators: {alias}"
        )));
    }
    if alias == "registry.json" || alias == "registry.json.lock" || alias == "backups" {
        return Err(Error::Other(format!("account alias is reserved: {alias}")));
    }
    Ok(())
}

/// Suggest an alias from an email: the local part before `@`, keeping only alphanumerics
/// and `-`/`_`; falls back to `default` when empty or invalid.
pub fn suggest_alias(email: Option<&str>) -> String {
    let local = email.and_then(|e| e.split('@').next()).unwrap_or("");
    let cleaned: String = local
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if cleaned.is_empty() || validate_alias(&cleaned).is_err() {
        "default".to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests;
