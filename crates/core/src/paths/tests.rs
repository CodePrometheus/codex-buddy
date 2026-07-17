use super::*;

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
}

#[test]
fn suggests_alias_from_email() {
    assert_eq!(suggest_alias(Some("zhouzixin@apache.org")), "zhouzixin");
    assert_eq!(suggest_alias(Some("a.b+c@x.com")), "abc");
    assert_eq!(suggest_alias(None), "default");
    assert_eq!(suggest_alias(Some("@x.com")), "default");
}
