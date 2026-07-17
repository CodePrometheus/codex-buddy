use super::*;
use tempfile::tempdir;

fn setup(dir: &Path) -> Paths {
    let codex = dir.join("codex");
    let buddy = dir.join("buddy");
    fs::create_dir_all(&codex).unwrap();
    fs::write(codex.join("config.toml"), "x").unwrap();
    fs::write(codex.join("AGENTS.md"), "x").unwrap();
    fs::create_dir_all(codex.join("sessions")).unwrap();
    fs::write(codex.join("auth.json"), "{}").unwrap();
    fs::write(codex.join("logs_2.sqlite"), "db").unwrap();
    fs::write(codex.join("logs_2.sqlite-wal"), "wal").unwrap();
    Paths::with_roots(codex, buddy)
}

#[test]
fn isolated_entry_matches() {
    assert!(is_isolated_entry("auth.json"));
    assert!(is_isolated_entry("sqlite"));
    assert!(is_isolated_entry("sessions"));
    assert!(is_isolated_entry("history.jsonl"));
    assert!(is_isolated_entry("logs_2.sqlite"));
    assert!(is_isolated_entry("state_5.sqlite-wal"));
    assert!(!is_isolated_entry("config.toml"));
    assert!(!is_isolated_entry("AGENTS.md"));
}

#[test]
fn builds_shared_symlinks_and_isolates_auth_sqlite() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    build_account_dir(&paths, "work").unwrap();
    let acct = paths.account_dir("work");

    for name in ["config.toml", "AGENTS.md"] {
        let link = acct.join(name);
        let meta = fs::symlink_metadata(&link).unwrap();
        assert!(meta.file_type().is_symlink(), "{name} should be a symlink");
        assert_eq!(fs::read_link(&link).unwrap(), paths.codex_home().join(name));
    }
    // isolated: no symlink created
    assert!(fs::symlink_metadata(acct.join("auth.json")).is_err());
    assert!(fs::symlink_metadata(acct.join("sessions")).is_err());
    assert!(fs::symlink_metadata(acct.join("logs_2.sqlite")).is_err());
    assert!(fs::symlink_metadata(acct.join("logs_2.sqlite-wal")).is_err());
}

#[test]
fn rebuild_is_idempotent_and_picks_up_new_entries() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    build_account_dir(&paths, "work").unwrap();
    fs::write(paths.codex_home().join("rules.md"), "x").unwrap();
    build_account_dir(&paths, "work").unwrap();
    let acct = paths.account_dir("work");
    assert!(
        fs::symlink_metadata(acct.join("rules.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_link(acct.join("config.toml")).unwrap(),
        paths.codex_home().join("config.toml")
    );
}

#[test]
fn atomic_symlink_replaces_existing() {
    let d = tempdir().unwrap();
    let base = d.path();
    let link = base.join("lnk");
    let a = base.join("a");
    let b = base.join("b");
    fs::write(&a, "a").unwrap();
    fs::write(&b, "b").unwrap();
    atomic_symlink(&link, &a).unwrap();
    assert_eq!(fs::read_link(&link).unwrap(), a);
    atomic_symlink(&link, &b).unwrap();
    assert_eq!(fs::read_link(&link).unwrap(), b);
}

#[test]
fn rebuild_removes_stale_isolated_symlink() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    fs::create_dir_all(paths.codex_home().join("sqlite")).unwrap();
    build_account_dir(&paths, "work").unwrap();
    let acct = paths.account_dir("work");
    assert!(fs::symlink_metadata(acct.join("sqlite")).is_err());

    unixfs::symlink(paths.codex_home().join("sqlite"), acct.join("sqlite")).unwrap();
    build_account_dir(&paths, "work").unwrap();
    assert!(fs::symlink_metadata(acct.join("sqlite")).is_err());
    assert!(paths.codex_home().join("sqlite").exists());
}

#[test]
fn point_switched_replaces_reverse_symlink() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    let codex = paths.codex_home().to_path_buf();
    let acct = paths.account_dir("work");
    fs::create_dir_all(&acct).unwrap();
    fs::write(acct.join("auth.json"), "{}").unwrap();

    // Older layout: the account side holds reverse symlinks back into ~/.codex.
    fs::remove_dir_all(codex.join("sessions")).unwrap();
    unixfs::symlink(codex.join("sessions"), acct.join("sessions")).unwrap();
    unixfs::symlink(codex.join("history.jsonl"), acct.join("history.jsonl")).unwrap();

    point_switched_entries(&paths, "work").unwrap();

    // Account side is now a real entity, not a symlink.
    let sm = fs::symlink_metadata(acct.join("sessions")).unwrap();
    assert!(!sm.file_type().is_symlink() && sm.is_dir());
    let hm = fs::symlink_metadata(acct.join("history.jsonl")).unwrap();
    assert!(!hm.file_type().is_symlink() && hm.is_file());
    // ~/.codex side points at the account entity, with no cycle.
    assert_eq!(
        fs::read_link(codex.join("sessions")).unwrap(),
        acct.join("sessions")
    );
    assert_eq!(
        fs::read_link(codex.join("auth.json")).unwrap(),
        acct.join("auth.json")
    );
}
