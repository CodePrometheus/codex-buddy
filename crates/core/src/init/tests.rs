use super::*;
use crate::layout::point_switched_entries;
use crate::testutil::auth_json;
use tempfile::tempdir;

fn setup_codex(dir: &Path) -> Paths {
    let codex = dir.join("codex");
    let buddy = dir.join("buddy");
    fs::create_dir_all(codex.join("sessions")).unwrap();
    fs::write(codex.join("auth.json"), auth_json("u", "a")).unwrap();
    fs::write(codex.join("config.toml"), "model = \"x\"\n").unwrap();
    fs::write(codex.join("sessions").join("s.jsonl"), "x").unwrap();
    Paths::with_roots(codex, buddy)
}

#[test]
fn plan_and_apply_migrates_first_account() {
    let d = tempdir().unwrap();
    let paths = setup_codex(d.path());
    let original_auth = fs::read_to_string(paths.codex_auth()).unwrap();

    let p = plan(&paths, "work").unwrap();
    assert_eq!(p.account_key, "u::a");
    apply(&paths, &p).unwrap();

    // ~/.codex/{auth.json, sessions} are now symlinks into the account dir.
    for entry in ["auth.json", "sessions"] {
        let live = paths.codex_home().join(entry);
        let m = fs::symlink_metadata(&live).unwrap();
        assert!(m.file_type().is_symlink(), "{entry} should be a symlink");
        assert_eq!(
            fs::read_link(&live).unwrap(),
            paths.account_dir("work").join(entry)
        );
    }
    // Account dir holds the real auth + the moved session file.
    assert!(
        fs::symlink_metadata(paths.account_auth("work"))
            .unwrap()
            .file_type()
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(paths.account_auth("work")).unwrap(),
        original_auth
    );
    assert!(
        paths
            .account_dir("work")
            .join("sessions")
            .join("s.jsonl")
            .exists()
    );
    // Shared config is still a symlink.
    assert!(
        fs::symlink_metadata(paths.account_dir("work").join("config.toml"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let reg = registry::load(&paths.registry_file()).unwrap();
    assert_eq!(reg.active(), Some("work"));
    assert!(p.backup_path.exists());
}

#[test]
fn plan_rejects_when_already_initialized() {
    let d = tempdir().unwrap();
    let paths = setup_codex(d.path());
    let p = plan(&paths, "work").unwrap();
    apply(&paths, &p).unwrap();
    assert!(plan(&paths, "work2").is_err());
}

#[test]
fn plan_rejects_keyring_store() {
    let d = tempdir().unwrap();
    let paths = setup_codex(d.path());
    fs::write(
        paths.codex_config(),
        "cli_auth_credentials_store = \"keyring\"\n",
    )
    .unwrap();
    assert!(plan(&paths, "work").is_err());
}

#[test]
fn rollback_restores_switched_entries() {
    let d = tempdir().unwrap();
    let paths = setup_codex(d.path());
    let original_auth = fs::read_to_string(paths.codex_auth()).unwrap();

    // Simulate a mid-migration state: backup auth, move entries, point symlinks.
    fs::create_dir_all(paths.backup_dir()).unwrap();
    let backup = paths.backup_dir().join("auth.json.bak");
    fs::copy(paths.codex_auth(), &backup).unwrap();
    fs::create_dir_all(paths.account_dir("work")).unwrap();
    build_account_dir(&paths, "work").unwrap();
    for entry in ["auth.json", "sessions"] {
        fs::rename(
            paths.codex_home().join(entry),
            paths.account_dir("work").join(entry),
        )
        .unwrap();
    }
    point_switched_entries(&paths, "work").unwrap();

    let plan = InitPlan {
        alias: "work".into(),
        account_key: "u::a".into(),
        email: None,
        plan: None,
        codex_auth: paths.codex_auth(),
        account_dir: paths.account_dir("work"),
        account_auth: paths.account_auth("work"),
        backup_path: backup,
        moves: vec!["auth.json".into(), "sessions".into()],
    };
    rollback(&paths, &plan).unwrap();

    // ~/.codex/auth.json + sessions restored to real entries; account dir gone.
    assert!(
        fs::symlink_metadata(paths.codex_auth())
            .unwrap()
            .file_type()
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(paths.codex_auth()).unwrap(),
        original_auth
    );
    assert!(
        fs::symlink_metadata(paths.codex_home().join("sessions"))
            .unwrap()
            .file_type()
            .is_dir()
    );
    assert!(!paths.account_dir("work").exists());
    assert!(paths.codex_config().exists());
}

#[test]
fn rollback_keeps_the_account_dir_when_a_restore_fails() {
    let d = tempdir().unwrap();
    let paths = setup_codex(d.path());

    // Mid-migration state whose auth backup is missing, so the auth restore must fail.
    fs::create_dir_all(paths.account_dir("work")).unwrap();
    for entry in ["auth.json", "sessions"] {
        fs::rename(
            paths.codex_home().join(entry),
            paths.account_dir("work").join(entry),
        )
        .unwrap();
    }

    let plan = InitPlan {
        alias: "work".into(),
        account_key: "u::a".into(),
        email: None,
        plan: None,
        codex_auth: paths.codex_auth(),
        account_dir: paths.account_dir("work"),
        account_auth: paths.account_auth("work"),
        backup_path: paths.backup_dir().join("missing.bak"),
        moves: vec!["auth.json".into(), "sessions".into()],
    };
    let err = rollback(&paths, &plan).unwrap_err();

    // The account dir survives as the sole copy of the unrestored auth.json.
    assert!(err.contains("auth.json"));
    assert!(paths.account_dir("work").join("auth.json").exists());
    // Entries that could be restored were: sessions moved back out.
    assert!(paths.codex_home().join("sessions").is_dir());
}

#[test]
fn apply_rolls_back_on_registry_failure() {
    let d = tempdir().unwrap();
    let paths = setup_codex(d.path());
    let original_auth = fs::read_to_string(paths.codex_auth()).unwrap();

    let p = plan(&paths, "work").unwrap();
    // Make the final registry write fail: pre-write a registry with an unsupported schema, so
    // apply_inner runs to completion then errors, exercising the automatic rollback path.
    fs::create_dir_all(paths.buddy_home()).unwrap();
    fs::write(
        paths.registry_file(),
        r#"{"schema_version":999,"accounts":[]}"#,
    )
    .unwrap();

    assert!(apply(&paths, &p).is_err());

    // Rolled back: ~/.codex/auth.json is a real file again with the original contents.
    let m = fs::symlink_metadata(paths.codex_auth()).unwrap();
    assert!(m.file_type().is_file());
    assert_eq!(
        fs::read_to_string(paths.codex_auth()).unwrap(),
        original_auth
    );
    // sessions restored as a real dir with its original file; account dir gone.
    assert!(
        fs::symlink_metadata(paths.codex_home().join("sessions"))
            .unwrap()
            .file_type()
            .is_dir()
    );
    assert!(paths.codex_home().join("sessions").join("s.jsonl").exists());
    assert!(!paths.account_dir("work").exists());
}
