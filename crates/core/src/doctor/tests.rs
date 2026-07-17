use super::*;
use crate::testutil::auth_json;
use tempfile::tempdir;

fn init_a(dir: &std::path::Path) -> Paths {
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

#[test]
fn healthy_setup_has_no_failures() {
    let d = tempdir().unwrap();
    let paths = init_a(d.path());
    let checks = diagnose(&paths);
    assert!(checks.iter().all(|c| c.level != Level::Fail), "{checks:?}");
    assert!(checks.iter().any(|c| c.level == Level::Pass));
}

#[test]
fn keyring_store_is_fail() {
    let d = tempdir().unwrap();
    let paths = init_a(d.path());
    fs::write(
        paths.codex_config(),
        "cli_auth_credentials_store = \"keyring\"\n",
    )
    .unwrap();
    let checks = diagnose(&paths);
    assert!(checks.iter().any(|c| c.level == Level::Fail));
}

#[test]
fn no_accounts_warns() {
    let d = tempdir().unwrap();
    let codex = d.path().join("codex");
    let buddy = d.path().join("buddy");
    fs::create_dir_all(&codex).unwrap();
    fs::write(codex.join("config.toml"), "x").unwrap();
    let paths = Paths::with_roots(codex, buddy);
    let checks = diagnose(&paths);
    assert!(checks.iter().any(|c| c.level == Level::Warn));
}
