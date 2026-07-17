use std::fs;
use std::path::Path;

use crate::error::{Error, Result};

/// codex's credential storage mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialStore {
    File,
    Keyring,
    Auto,
    Ephemeral,
    Unknown(String),
}

impl CredentialStore {
    pub fn as_str(&self) -> String {
        match self {
            CredentialStore::File => "file".into(),
            CredentialStore::Keyring => "keyring".into(),
            CredentialStore::Auto => "auto".into(),
            CredentialStore::Ephemeral => "ephemeral".into(),
            CredentialStore::Unknown(s) => s.clone(),
        }
    }

    fn parse(v: &str) -> Self {
        match v {
            "file" => CredentialStore::File,
            "keyring" => CredentialStore::Keyring,
            "auto" => CredentialStore::Auto,
            "ephemeral" => CredentialStore::Ephemeral,
            other => CredentialStore::Unknown(other.to_string()),
        }
    }
}

/// Read `cli_auth_credentials_store` from config.toml; a missing file or key means `file`.
pub fn credential_store(config_path: &Path) -> Result<CredentialStore> {
    let text = match fs::read_to_string(config_path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(CredentialStore::File),
        Err(e) => return Err(Error::Io(e)),
    };
    for line in text.lines() {
        if let Some(v) = extract_value(line, "cli_auth_credentials_store") {
            return Ok(CredentialStore::parse(&v));
        }
    }
    Ok(CredentialStore::File)
}

/// Ensure the credential store is `file`, else return [`Error::UnsupportedCredentialStore`].
pub fn ensure_file_store(config_path: &Path) -> Result<()> {
    match credential_store(config_path)? {
        CredentialStore::File => Ok(()),
        other => Err(Error::UnsupportedCredentialStore(other.as_str())),
    }
}

/// Extract the value of `key = "value"` from a TOML line. Comment lines and keys that merely
/// share `key` as a prefix are ignored.
fn extract_value(line: &str, key: &str) -> Option<String> {
    let rest = line.trim().strip_prefix(key)?;
    let rest = rest.trim_start().strip_prefix('=')?;
    let mut val = rest.trim();
    if let Some(idx) = val.find('#') {
        val = val[..idx].trim();
    }
    let val = val.trim_matches(|c| c == '"' || c == '\'').trim();
    if val.is_empty() {
        None
    } else {
        Some(val.to_string())
    }
}

#[cfg(test)]
mod tests;
