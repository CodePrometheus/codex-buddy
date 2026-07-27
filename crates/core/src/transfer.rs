use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::auth::load_auth_info;
use crate::error::{Error, Result};
use crate::ops;
use crate::paths::{Paths, validate_alias};
use crate::registry;

/// Outcome of one item in a directory import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportStatus {
    Imported,
    Skipped,
    Failed,
}

/// Machine-readable result for one directory import item.
#[derive(Debug, Clone, Serialize)]
pub struct ImportItemResult {
    pub source: PathBuf,
    pub alias: String,
    pub status: ImportStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

enum PreparedImport {
    Ready { source: PathBuf, alias: String },
    Finished(ImportItemResult),
}

/// Import each immediate `<root>/<alias>/auth.json` independently.
///
/// All inputs are inspected before the first write. Each valid account is then committed through
/// the existing single-account import path; failures do not roll back successful siblings.
pub fn import_directory(
    paths: &Paths,
    root: &Path,
    skip_existing: bool,
) -> Result<Vec<ImportItemResult>> {
    ops::ensure_account_changes_ready(paths)?;
    if !root.is_dir() {
        return Err(Error::Other(format!(
            "import source is not a directory: {}",
            root.display()
        )));
    }

    let sources = discover_imports(root)?;
    if sources.is_empty() {
        return Err(Error::Other(format!(
            "no <alias>/auth.json entries found in {}",
            root.display()
        )));
    }

    let reg = registry::load(&paths.registry_file())?;
    let mut batch_keys: BTreeMap<String, String> = BTreeMap::new();
    let mut prepared = Vec::with_capacity(sources.len());

    for (name, source) in sources {
        prepared.push(preflight_import(
            paths,
            &reg,
            &mut batch_keys,
            name,
            source,
            skip_existing,
        ));
    }

    Ok(prepared
        .into_iter()
        .map(|item| match item {
            PreparedImport::Finished(result) => result,
            PreparedImport::Ready { source, alias } => match ops::import(paths, &source, &alias) {
                Ok(()) => ImportItemResult {
                    source,
                    alias,
                    status: ImportStatus::Imported,
                    error: None,
                },
                Err(e) => ImportItemResult {
                    source,
                    alias,
                    status: ImportStatus::Failed,
                    error: Some(e.to_string()),
                },
            },
        })
        .collect())
}

fn discover_imports(root: &Path) -> Result<Vec<(std::ffi::OsString, PathBuf)>> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let auth = entry.path().join("auth.json");
            if auth.is_file() {
                sources.push((entry.file_name(), auth));
            }
        }
    }
    sources.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(sources)
}

fn preflight_import(
    paths: &Paths,
    reg: &registry::Registry,
    batch_keys: &mut BTreeMap<String, String>,
    name: std::ffi::OsString,
    source: PathBuf,
    skip_existing: bool,
) -> PreparedImport {
    let Some(alias) = name.to_str().map(str::to_owned) else {
        return failed(source, name.to_string_lossy(), "alias is not valid UTF-8");
    };
    if let Err(e) = validate_alias(&alias) {
        return failed(source, alias, e);
    }
    let info = match load_auth_info(&source) {
        Ok(info) => info,
        Err(e) => return failed(source, alias, e),
    };

    if let Some(existing) = reg.find(&alias) {
        if existing.account_key == info.account_key && skip_existing {
            return PreparedImport::Finished(finished(source, alias, ImportStatus::Skipped, None));
        }
        let message = if existing.account_key == info.account_key {
            format!("account `{alias}` already exists; use --skip-existing to skip it")
        } else {
            format!("alias `{alias}` belongs to a different existing account")
        };
        return failed(source, alias, message);
    }
    if let Some(existing) = reg.find_by_key(&info.account_key) {
        return failed(
            source,
            alias,
            format!(
                "account is already registered with alias `{}`",
                existing.alias
            ),
        );
    }
    if paths.account_dir(&alias).exists() {
        return failed(
            source,
            alias.clone(),
            format!("target account directory already exists for `{alias}`"),
        );
    }
    if let Some(first_alias) = batch_keys.get(&info.account_key) {
        return failed(
            source,
            alias,
            format!("same account also appears in this batch as `{first_alias}`"),
        );
    }

    batch_keys.insert(info.account_key, alias.clone());
    PreparedImport::Ready { source, alias }
}

/// Export one managed auth.json to an explicit external file.
pub fn export_account(paths: &Paths, alias: &str, destination: &Path, force: bool) -> Result<()> {
    let source = ops::account_home(paths, alias)?.join("auth.json");
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent)?;
    if !parent.is_dir() {
        return Err(Error::Other(format!(
            "export destination parent is not a directory: {}",
            parent.display()
        )));
    }

    let buddy_home = fs::canonicalize(paths.buddy_home())?;
    if parent.starts_with(&buddy_home) {
        return Err(Error::Other(
            "export destination must be outside the managed codex-buddy directory".into(),
        ));
    }
    let file_name = destination.file_name().ok_or_else(|| {
        Error::Other(format!(
            "export destination has no file name: {}",
            destination.display()
        ))
    })?;
    let destination = parent.join(file_name);

    if let Ok(meta) = fs::symlink_metadata(&destination) {
        if meta.file_type().is_symlink() {
            return Err(Error::Other(format!(
                "refusing to export over a symlink: {}",
                destination.display()
            )));
        }
        if !meta.is_file() {
            return Err(Error::Other(format!(
                "export destination is not a regular file: {}",
                destination.display()
            )));
        }
        if !force {
            return Err(Error::Other(format!(
                "export destination already exists; pass --force to overwrite: {}",
                destination.display()
            )));
        }
    }

    let (temporary, mut output) = create_temporary(&parent)?;
    let result = (|| {
        let mut input = fs::File::open(&source)?;
        io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        drop(output);

        if force {
            fs::rename(&temporary, &destination)?;
        } else {
            fs::hard_link(&temporary, &destination)?;
            fs::remove_file(&temporary)?;
        }
        fs::File::open(&parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn create_temporary(parent: &Path) -> Result<(PathBuf, fs::File)> {
    for attempt in 0..100 {
        let path = parent.join(format!(
            ".codex-buddy-export.tmp.{}.{}",
            std::process::id(),
            attempt
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(Error::Io(e)),
        }
    }
    Err(Error::Other(format!(
        "could not create an export temporary file in {}",
        parent.display()
    )))
}

fn failed(
    source: PathBuf,
    alias: impl Into<String>,
    error: impl std::fmt::Display,
) -> PreparedImport {
    PreparedImport::Finished(finished(
        source,
        alias,
        ImportStatus::Failed,
        Some(error.to_string()),
    ))
}

fn finished(
    source: PathBuf,
    alias: impl Into<String>,
    status: ImportStatus,
    error: Option<String>,
) -> ImportItemResult {
    ImportItemResult {
        source,
        alias: alias.into(),
        status,
        error,
    }
}

#[cfg(test)]
mod tests;
