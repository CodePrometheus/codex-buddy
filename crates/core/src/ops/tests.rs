use super::*;
use crate::testutil::auth_json;
use tempfile::tempdir;

fn add_account(paths: &Paths, alias: &str, user: &str, acct: &str) {
    fs::create_dir_all(paths.account_dir(alias)).unwrap();
    fs::write(paths.account_auth(alias), auth_json(user, acct)).unwrap();
    build_account_dir(paths, alias).unwrap();
    registry::update(paths, |r| {
        r.add(AccountRecord {
            alias: alias.into(),
            account_key: format!("{user}::{acct}"),
            email: None,
            plan: None,
            dir: alias.into(),
            added_at: 0,
            last_used_at: None,
        })?;
        Ok(())
    })
    .unwrap();
}

fn init_account_a(dir: &std::path::Path) -> Paths {
    let codex = dir.join("codex");
    let buddy = dir.join("buddy");
    fs::create_dir_all(&codex).unwrap();
    fs::write(codex.join("auth.json"), auth_json("usera", "accta")).unwrap();
    fs::write(codex.join("config.toml"), "x").unwrap();
    fs::create_dir_all(codex.join("sessions")).unwrap();
    let paths = Paths::with_roots(codex, buddy);
    let p = crate::init::plan(&paths, "a").unwrap();
    crate::init::apply(&paths, &p).unwrap();
    paths
}

fn setup(dir: &std::path::Path) -> Paths {
    let paths = init_account_a(dir);
    add_account(&paths, "b", "userb", "acctb");
    paths
}

#[test]
fn switch_repoints_and_tracks_previous() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    switch(&paths, "b").unwrap();
    assert_eq!(
        fs::read_link(paths.codex_auth()).unwrap(),
        paths.account_auth("b")
    );
    assert_eq!(
        fs::read_link(paths.codex_home().join("sessions")).unwrap(),
        paths.account_dir("b").join("sessions")
    );
    let reg = registry::load(&paths.registry_file()).unwrap();
    assert_eq!(reg.active(), Some("b"));
    assert_eq!(reg.previous(), Some("a"));
}

#[test]
fn switch_previous_goes_back() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    switch(&paths, "b").unwrap();
    switch_previous(&paths).unwrap();
    assert_eq!(
        fs::read_link(paths.codex_auth()).unwrap(),
        paths.account_auth("a")
    );
    let reg = registry::load(&paths.registry_file()).unwrap();
    assert_eq!(reg.active(), Some("a"));
    assert_eq!(reg.previous(), Some("b"));
}

#[test]
fn switch_unknown_errors() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    assert!(switch(&paths, "nope").is_err());
}

#[test]
fn switch_previous_without_history_errors() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    assert!(switch_previous(&paths).is_err());
}

#[test]
fn list_and_current_resolve_metadata() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    let views = list(&paths).unwrap();
    assert_eq!(views.len(), 2);
    let a = views.iter().find(|v| v.alias == "a").unwrap();
    assert!(a.is_active);
    assert_eq!(a.email.as_deref(), Some("usera@example.com"));
    assert_eq!(a.plan.as_deref(), Some("pro"));
    let cur = current(&paths).unwrap().unwrap();
    assert_eq!(cur.alias, "a");
}

#[test]
fn run_unknown_errors() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    assert!(run(&paths, "nope", &[]).is_err());
}

#[test]
fn add_prepare_requires_init() {
    let d = tempdir().unwrap();
    let codex = d.path().join("codex");
    let buddy = d.path().join("buddy");
    fs::create_dir_all(&codex).unwrap();
    fs::write(codex.join("auth.json"), auth_json("u", "a")).unwrap();
    let paths = Paths::with_roots(codex, buddy);
    assert!(add_prepare(&paths, "b").is_err());
}

#[test]
fn add_prepare_builds_dir_and_symlinks() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    let dir = add_prepare(&paths, "b").unwrap();
    assert_eq!(dir, paths.account_dir("b"));
    assert!(
        fs::symlink_metadata(paths.account_dir("b").join("config.toml"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn add_prepare_rejects_existing_alias() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    assert!(add_prepare(&paths, "a").is_err());
}

#[test]
fn add_finalize_writes_registry() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    add_prepare(&paths, "b").unwrap();
    fs::write(paths.account_auth("b"), auth_json("userb", "acctb")).unwrap();
    add_finalize(&paths, "b").unwrap();
    let reg = registry::load(&paths.registry_file()).unwrap();
    assert_eq!(reg.find("b").unwrap().account_key, "userb::acctb");
}

#[test]
fn add_finalize_rejects_duplicate_key_and_cleans() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    add_prepare(&paths, "dup").unwrap();
    fs::write(paths.account_auth("dup"), auth_json("usera", "accta")).unwrap();
    assert!(add_finalize(&paths, "dup").is_err());
    assert!(!paths.account_dir("dup").exists());
    assert!(
        registry::load(&paths.registry_file())
            .unwrap()
            .find("dup")
            .is_none()
    );
}

#[test]
fn add_finalize_cleans_up_on_missing_auth() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    add_prepare(&paths, "x").unwrap();
    assert!(add_finalize(&paths, "x").is_err());
    assert!(!paths.account_dir("x").exists());
}

#[test]
fn rename_updates_dir_registry_and_active_symlink() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    rename(&paths, "a", "primary").unwrap();
    assert!(!paths.account_dir("a").exists());
    assert!(paths.account_dir("primary").exists());
    let reg = registry::load(&paths.registry_file()).unwrap();
    assert!(reg.find("a").is_none());
    assert_eq!(reg.find("primary").unwrap().dir, "primary");
    assert_eq!(reg.active(), Some("primary"));
    assert_eq!(
        fs::read_link(paths.codex_auth()).unwrap(),
        paths.account_auth("primary")
    );
}

#[test]
fn rename_rejects_existing_target() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    assert!(rename(&paths, "a", "b").is_err());
}

#[test]
fn rename_unknown_errors() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    assert!(rename(&paths, "nope", "x").is_err());
}

#[test]
fn remove_deletes_dir_and_registry_entry() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    remove(&paths, "b").unwrap();
    assert!(!paths.account_dir("b").exists());
    assert!(
        registry::load(&paths.registry_file())
            .unwrap()
            .find("b")
            .is_none()
    );
}

#[test]
fn remove_refuses_active() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    assert!(remove(&paths, "a").is_err());
    assert!(paths.account_dir("a").exists());
}

#[test]
fn remove_clears_previous_pointer() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    switch(&paths, "b").unwrap();
    switch(&paths, "a").unwrap();
    remove(&paths, "b").unwrap();
    assert_eq!(
        registry::load(&paths.registry_file()).unwrap().previous(),
        None
    );
}

#[test]
fn remove_unknown_errors() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    assert!(remove(&paths, "nope").is_err());
}

#[test]
fn import_copies_auth_and_registers() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    let src = d.path().join("external-auth.json");
    fs::write(&src, auth_json("userc", "acctc")).unwrap();
    import(&paths, &src, "c").unwrap();
    assert!(paths.account_auth("c").exists());
    assert_eq!(
        registry::load(&paths.registry_file())
            .unwrap()
            .find("c")
            .unwrap()
            .account_key,
        "userc::acctc"
    );
}

#[test]
fn import_rejects_duplicate_key_and_cleans() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    let src = d.path().join("dup-auth.json");
    fs::write(&src, auth_json("usera", "accta")).unwrap();
    assert!(import(&paths, &src, "c").is_err());
    assert!(!paths.account_dir("c").exists());
}

#[test]
fn import_missing_src_errors_and_cleans() {
    let d = tempdir().unwrap();
    let paths = init_account_a(d.path());
    let src = d.path().join("nope.json");
    assert!(import(&paths, &src, "c").is_err());
    assert!(!paths.account_dir("c").exists());
}

#[test]
fn relogin_unknown_errors() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    assert!(relogin(&paths, "nope").is_err());
}

#[test]
fn account_home_returns_dir() {
    let d = tempdir().unwrap();
    let paths = setup(d.path());
    assert_eq!(account_home(&paths, "b").unwrap(), paths.account_dir("b"));
    assert!(account_home(&paths, "nope").is_err());
}
