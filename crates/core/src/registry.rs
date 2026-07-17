use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::paths::Paths;

/// Registry schema version supported by this tool.
pub const SCHEMA_VERSION: u32 = 1;

/// A single account record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecord {
    pub alias: String,
    pub account_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    /// Directory name under ~/.codex-buddy (usually equal to `alias`).
    pub dir: String,
    /// Unix epoch seconds.
    pub added_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<i64>,
}

/// The whole registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_account: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_account: Option<String>,
    #[serde(default)]
    pub accounts: Vec<AccountRecord>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            active_account: None,
            previous_account: None,
            accounts: Vec::new(),
        }
    }
}

impl Registry {
    pub fn find(&self, alias: &str) -> Option<&AccountRecord> {
        self.accounts.iter().find(|a| a.alias == alias)
    }

    pub fn find_mut(&mut self, alias: &str) -> Option<&mut AccountRecord> {
        self.accounts.iter_mut().find(|a| a.alias == alias)
    }

    pub fn find_by_key(&self, account_key: &str) -> Option<&AccountRecord> {
        self.accounts.iter().find(|a| a.account_key == account_key)
    }

    pub fn active(&self) -> Option<&str> {
        self.active_account.as_deref()
    }

    pub fn previous(&self) -> Option<&str> {
        self.previous_account.as_deref()
    }

    /// Add an account; errors if the alias or account key already exists.
    pub fn add(&mut self, record: AccountRecord) -> Result<()> {
        if self.find(&record.alias).is_some() {
            return Err(Error::AccountExists(record.alias));
        }
        if self.find_by_key(&record.account_key).is_some() {
            return Err(Error::AccountExists(format!(
                "account_key={}",
                record.account_key
            )));
        }
        self.accounts.push(record);
        Ok(())
    }

    /// Remove an account, clearing the active / previous pointers if they referred to it.
    pub fn remove(&mut self, alias: &str) -> Result<AccountRecord> {
        let idx = self
            .accounts
            .iter()
            .position(|a| a.alias == alias)
            .ok_or_else(|| Error::AccountNotFound(alias.to_string()))?;
        let removed = self.accounts.remove(idx);
        if self.active_account.as_deref() == Some(alias) {
            self.active_account = None;
        }
        if self.previous_account.as_deref() == Some(alias) {
            self.previous_account = None;
        }
        Ok(removed)
    }

    /// Make `alias` active, moving the old active into `previous` (for `switch -`).
    pub fn set_active(&mut self, alias: &str) {
        if self.active_account.as_deref() == Some(alias) {
            return;
        }
        self.previous_account = self.active_account.take();
        self.active_account = Some(alias.to_string());
    }
}

/// Current Unix time in seconds.
pub fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Load the registry; a missing file yields an empty registry; a newer schema is rejected.
pub fn load(path: &Path) -> Result<Registry> {
    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Registry::default()),
        Err(e) => return Err(Error::Io(e)),
    };
    let reg: Registry = serde_json::from_str(&data)?;
    if reg.schema_version > SCHEMA_VERSION {
        return Err(Error::RegistrySchemaTooNew {
            found: reg.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    Ok(reg)
}

/// Write the registry atomically (temp file + fsync + rename).
pub fn save(reg: &Registry, path: &Path) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::Other("registry path has no parent".into()))?;
    fs::create_dir_all(dir)?;
    let data = serde_json::to_vec_pretty(reg)?;
    let tmp = dir.join(format!(".registry.json.tmp.{}", std::process::id()));
    {
        let mut f = File::create(&tmp)?;
        f.write_all(&data)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Registry write lock. Exclusive while held; released when the `File` is dropped.
pub struct RegistryLock {
    _file: File,
}

/// Acquire the registry write lock, blocking until available.
pub fn acquire_lock(paths: &Paths) -> Result<RegistryLock> {
    fs::create_dir_all(paths.buddy_home())?;
    let lock_path = paths.buddy_home().join("registry.json.lock");
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)?;
    file.lock()?;
    Ok(RegistryLock { _file: file })
}

/// Load -> mutate -> save while holding the lock, so concurrent writers don't lose updates.
pub fn update<F, T>(paths: &Paths, f: F) -> Result<T>
where
    F: FnOnce(&mut Registry) -> Result<T>,
{
    let _lock = acquire_lock(paths)?;
    let path = paths.registry_file();
    let mut reg = load(&path)?;
    let out = f(&mut reg)?;
    save(&reg, &path)?;
    Ok(out)
}

#[cfg(test)]
mod tests;
