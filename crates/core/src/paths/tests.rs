use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn ensure_buddy_home_creates_dir_and_locks_it_down() {
    let d = tempfile::tempdir().unwrap();
    let buddy = d.path().join("buddy");
    let paths = Paths::with_roots(d.path().join("codex"), &buddy);
    paths.ensure_buddy_home().unwrap();
    let mode = fs::metadata(&buddy).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
}

#[test]
fn with_roots_derives_paths() {
    let p = Paths::with_roots("/tmp/codex", "/tmp/buddy");
    assert_eq!(p.codex_auth(), Path::new("/tmp/codex/auth.json"));
    assert_eq!(p.codex_config(), Path::new("/tmp/codex/config.toml"));
    assert_eq!(p.registry_file(), Path::new("/tmp/buddy/registry.json"));
    assert_eq!(p.backup_dir(), Path::new("/tmp/buddy/backups"));
    assert_eq!(p.account_dir("work"), Path::new("/tmp/buddy/work"));
    assert_eq!(
        p.account_auth("work"),
        Path::new("/tmp/buddy/work/auth.json")
    );
}

#[test]
fn alias_validation() {
    assert!(validate_alias("work").is_ok());
    assert!(validate_alias("personal-2").is_ok());
    assert!(validate_alias("").is_err());
    assert!(validate_alias(".").is_err());
    assert!(validate_alias("..").is_err());
    assert!(validate_alias("a/b").is_err());
    assert!(validate_alias("backups").is_err());
    assert!(validate_alias("registry.json").is_err());
    assert!(validate_alias("registry.json.lock").is_err());
    assert!(validate_alias(".hidden").is_err());
    assert!(validate_alias("-").is_err());
    assert!(validate_alias("-y").is_err());
    assert!(validate_alias("--force").is_err());
}

#[test]
fn suggests_alias_from_email() {
    assert_eq!(suggest_alias(Some("zhouzixin@apache.org")), "zhouzixin");
    assert_eq!(suggest_alias(Some("a.b+c@x.com")), "abc");
    assert_eq!(suggest_alias(None), "default");
    assert_eq!(suggest_alias(Some("@x.com")), "default");
}
