use super::*;
use crate::paths::Paths;
use tempfile::tempdir;

fn rec(alias: &str, key: &str) -> AccountRecord {
    AccountRecord {
        alias: alias.into(),
        account_key: key.into(),
        email: None,
        plan: None,
        dir: alias.into(),
        added_at: 0,
        last_used_at: None,
    }
}

#[test]
fn set_active_tracks_previous() {
    let mut r = Registry::default();
    r.set_active("a");
    assert_eq!(r.active(), Some("a"));
    assert_eq!(r.previous(), None);
    r.set_active("b");
    assert_eq!(r.active(), Some("b"));
    assert_eq!(r.previous(), Some("a"));
    r.set_active("b");
    assert_eq!(r.previous(), Some("a"));
    r.set_active("c");
    assert_eq!(r.previous(), Some("b"));
}

#[test]
fn add_duplicate_is_error() {
    let mut r = Registry::default();
    r.add(rec("work", "u::a")).unwrap();
    assert!(r.add(rec("work", "u::b")).is_err());
    assert!(r.add(rec("other", "u::a")).is_err());
}

#[test]
fn remove_clears_pointers() {
    let mut r = Registry::default();
    r.add(rec("a", "u::a")).unwrap();
    r.add(rec("b", "u::b")).unwrap();
    r.set_active("a");
    r.set_active("b");
    r.remove("a").unwrap();
    assert_eq!(r.previous(), None);
    assert_eq!(r.active(), Some("b"));
}

#[test]
fn save_load_roundtrip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("registry.json");
    let mut r = Registry::default();
    r.add(rec("work", "u::a")).unwrap();
    r.set_active("work");
    save(&r, &path).unwrap();
    let loaded = load(&path).unwrap();
    assert_eq!(loaded.active(), Some("work"));
    assert_eq!(loaded.accounts.len(), 1);
}

#[test]
fn load_missing_returns_empty() {
    let dir = tempdir().unwrap();
    let r = load(&dir.path().join("nope.json")).unwrap();
    assert!(r.accounts.is_empty());
    assert_eq!(r.schema_version, SCHEMA_VERSION);
}

#[test]
fn rejects_newer_schema() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("registry.json");
    fs::write(&path, r#"{"schema_version":999,"accounts":[]}"#).unwrap();
    assert!(matches!(
        load(&path),
        Err(Error::RegistrySchemaTooNew { .. })
    ));
}

#[test]
fn update_persists_under_lock() {
    let dir = tempdir().unwrap();
    let paths = Paths::with_roots(dir.path().join("codex"), dir.path().join("buddy"));
    update(&paths, |r| {
        r.add(rec("work", "u::a"))?;
        r.set_active("work");
        Ok(())
    })
    .unwrap();
    let reg = load(&paths.registry_file()).unwrap();
    assert_eq!(reg.active(), Some("work"));
    assert_eq!(reg.accounts.len(), 1);
}

#[test]
fn concurrent_updates_do_not_lose() {
    let dir = tempdir().unwrap();
    let paths = Paths::with_roots(dir.path().join("codex"), dir.path().join("buddy"));
    let n = 8;
    std::thread::scope(|s| {
        for i in 0..n {
            let paths = paths.clone();
            s.spawn(move || {
                update(&paths, |r| {
                    r.add(rec(&format!("a{i}"), &format!("u::{i}")))?;
                    Ok(())
                })
                .unwrap();
            });
        }
    });
    let reg = load(&paths.registry_file()).unwrap();
    assert_eq!(reg.accounts.len(), n);
}
