use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

use tempfile::tempdir;

use super::*;
use crate::testutil::auth_json;

fn setup(dir: &Path) -> Paths {
    let codex = dir.join("codex");
    let buddy = dir.join("buddy");
    fs::create_dir_all(codex.join("sessions")).unwrap();
    fs::write(codex.join("auth.json"), auth_json("usera", "accta")).unwrap();
    fs::write(codex.join("config.toml"), "x").unwrap();
    let paths = Paths::with_roots(codex, buddy);
    let plan = crate::init::plan(&paths, "a").unwrap();
    crate::init::apply(&paths, &plan).unwrap();
    paths
}

fn import_source(root: &Path, alias: &str, user: &str, account: &str) -> PathBuf {
    let dir = root.join(alias);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auth.json");
    fs::write(&path, auth_json(user, account)).unwrap();
    path
}

#[test]
fn directory_import_keeps_successes_when_an_item_fails() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    let source = dir.path().join("imports");
    import_source(&source, "b", "userb", "acctb");
    let broken = source.join("broken");
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join("auth.json"), "{}").unwrap();

    let results = import_directory(&paths, &source, false).unwrap();
    assert_eq!(results.len(), 2);
    assert!(
        results
            .iter()
            .any(|item| item.alias == "b" && item.status == ImportStatus::Imported)
    );
    assert!(
        results
            .iter()
            .any(|item| item.alias == "broken" && item.status == ImportStatus::Failed)
    );
    assert!(
        registry::load(&paths.registry_file())
            .unwrap()
            .find("b")
            .is_some()
    );
}

#[test]
fn skip_existing_only_skips_the_same_alias_and_account() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    let source = dir.path().join("imports");
    import_source(&source, "a", "usera", "accta");
    import_source(&source, "other", "usera", "accta");

    let results = import_directory(&paths, &source, true).unwrap();
    let a = results.iter().find(|item| item.alias == "a").unwrap();
    let other = results.iter().find(|item| item.alias == "other").unwrap();
    assert_eq!(a.status, ImportStatus::Skipped);
    assert_eq!(other.status, ImportStatus::Failed);
}

#[test]
fn duplicate_accounts_inside_a_batch_import_only_once() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    let source = dir.path().join("imports");
    import_source(&source, "b", "userb", "acctb");
    import_source(&source, "same-b", "userb", "acctb");

    let results = import_directory(&paths, &source, false).unwrap();
    assert_eq!(
        results
            .iter()
            .filter(|item| item.status == ImportStatus::Imported)
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|item| item.status == ImportStatus::Failed)
            .count(),
        1
    );
}

#[test]
fn export_is_owner_only_and_requires_force_to_overwrite() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    let destination = dir.path().join("exported-auth.json");

    export_account(&paths, "a", &destination, false).unwrap();
    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        fs::read_to_string(paths.account_auth("a")).unwrap()
    );
    assert_eq!(
        fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
        0o600
    );

    fs::write(&destination, "old").unwrap();
    assert!(export_account(&paths, "a", &destination, false).is_err());
    export_account(&paths, "a", &destination, true).unwrap();
    assert_ne!(fs::read_to_string(&destination).unwrap(), "old");
    assert_eq!(
        fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn export_refuses_symlinks_and_managed_destinations() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    let external = dir.path().join("external");
    fs::write(&external, "keep").unwrap();
    let link = dir.path().join("auth-link.json");
    symlink(&external, &link).unwrap();

    assert!(export_account(&paths, "a", &link, true).is_err());
    assert_eq!(fs::read_to_string(&external).unwrap(), "keep");
    assert!(
        export_account(
            &paths,
            "a",
            &paths.buddy_home().join("should-not-write.json"),
            true,
        )
        .is_err()
    );
}
