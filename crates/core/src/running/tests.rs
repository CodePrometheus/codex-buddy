use super::*;
use crate::registry::AccountRecord;
use std::fs;
use std::process::{Child, Command};
use std::time::Duration;
use tempfile::tempdir;

fn account(alias: &str) -> AccountRecord {
    AccountRecord {
        alias: alias.into(),
        account_key: format!("key-{alias}"),
        email: None,
        plan: None,
        dir: alias.into(),
        added_at: 0,
        last_used_at: None,
    }
}

fn blob(exec_path: &str, argv: &[&str], env: &[&str]) -> Vec<u8> {
    let mut b = (argv.len() as i32).to_ne_bytes().to_vec();
    b.extend_from_slice(exec_path.as_bytes());
    b.extend_from_slice(&[0, 0, 0]);
    for arg in argv {
        b.extend_from_slice(arg.as_bytes());
        b.push(0);
    }
    for entry in env {
        b.extend_from_slice(entry.as_bytes());
        b.push(0);
    }
    b
}

/// Copies /bin/sleep to `dir/codex` and spawns it, so a real process named `codex` with the
/// given environment exists while the caller probes.
fn spawn_fake_codex(dir: &Path, env: &[(&str, &Path)]) -> Child {
    let fake = dir.join("codex");
    fs::copy("/bin/sleep", &fake).unwrap();
    let mut cmd = Command::new(&fake);
    cmd.arg("30").env_remove("CODEX_HOME");
    for (key, value) in env {
        cmd.env(key, value);
    }
    spawn_and_wait_for_name(cmd, "codex")
}

/// Spawns and blocks until the process's exec completed (its name is visible as `name`) —
/// a fixed sleep is flaky because first exec of a freshly copied binary can stall on
/// signature / XProtect checks.
fn spawn_and_wait_for_name(mut cmd: Command, name: &str) -> Child {
    let mut child = cmd.spawn().unwrap();
    let pid = child.id() as c_int;
    for _ in 0..200 {
        if process_name(pid).as_deref() == Some(name) {
            return child;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("process never showed up as {name}");
}

#[test]
fn env_var_finds_the_key() {
    let b = blob(
        "/x/codex",
        &["codex", "resume", "abc"],
        &["TERM=xterm", "CODEX_HOME=/tmp/acct", "HOME=/Users/me"],
    );
    assert_eq!(env_var(&b, "CODEX_HOME"), Some("/tmp/acct".into()));
    assert_eq!(env_var(&b, "HOME"), Some("/Users/me".into()));
}

#[test]
fn env_var_missing_key_is_none() {
    let b = blob("/x/codex", &["codex"], &["TERM=xterm"]);
    assert_eq!(env_var(&b, "CODEX_HOME"), None);
}

#[test]
fn env_var_does_not_match_an_argv_lookalike() {
    let b = blob("/x/codex", &["codex", "CODEX_HOME=/evil"], &["HOME=/h"]);
    assert_eq!(env_var(&b, "CODEX_HOME"), None);
}

#[test]
fn env_var_requires_the_full_key() {
    let b = blob(
        "/x/codex",
        &["codex"],
        &["XCODEX_HOME=/a", "CODEX_HOMEX=/b"],
    );
    assert_eq!(env_var(&b, "CODEX_HOME"), None);
}

#[test]
fn env_var_value_keeps_equals_signs() {
    let b = blob("/x/codex", &["codex"], &["CODEX_HOME=/a=b"]);
    assert_eq!(env_var(&b, "CODEX_HOME"), Some("/a=b".into()));
}

#[test]
fn env_var_handles_empty_argv() {
    let b = blob("/x/codex", &[], &["CODEX_HOME=/tmp/acct"]);
    assert_eq!(env_var(&b, "CODEX_HOME"), Some("/tmp/acct".into()));
}

#[test]
fn env_var_truncated_blob_is_none() {
    assert_eq!(env_var(&[], "CODEX_HOME"), None);
    assert_eq!(env_var(&[2, 0, 0, 0], "CODEX_HOME"), None);
    assert_eq!(
        env_var(&blob("/x/codex", &["codex"], &[]), "CODEX_HOME"),
        None
    );
}

#[test]
fn account_with_no_dir_is_not_running() {
    let d = tempdir().unwrap();
    let paths = Paths::with_roots(d.path().join("codex"), d.path().join("buddy"));
    let mut reg = Registry::default();
    reg.accounts.push(account("work"));
    assert!(running_accounts(&paths, &reg).is_empty());
}

#[test]
fn a_codex_process_with_codex_home_is_attributed_to_its_account() {
    let d = tempdir().unwrap();
    let paths = Paths::with_roots(d.path().join("codex"), d.path().join("buddy"));
    let acct = paths.account_dir("work");
    fs::create_dir_all(&acct).unwrap();
    fs::create_dir_all(paths.account_dir("other")).unwrap();
    let mut reg = Registry::default();
    reg.accounts.push(account("work"));
    reg.accounts.push(account("other"));

    let mut child = spawn_fake_codex(d.path(), &[("CODEX_HOME", acct.as_path())]);
    let running = running_accounts(&paths, &reg);
    let _ = child.kill();
    let _ = child.wait();

    assert_eq!(running, BTreeSet::from(["work".to_string()]));
}

#[test]
fn a_codex_process_without_codex_home_counts_as_the_active_account() {
    let d = tempdir().unwrap();
    let home = d.path().join("home");
    let codex_home = home.join(".codex");
    fs::create_dir_all(&codex_home).unwrap();
    let paths = Paths::with_roots(codex_home, d.path().join("buddy"));
    let mut reg = Registry::default();
    reg.accounts.push(account("work"));
    reg.active_account = Some("work".into());

    let mut child = spawn_fake_codex(d.path(), &[("HOME", home.as_path())]);
    let running = running_accounts(&paths, &reg);
    let _ = child.kill();
    let _ = child.wait();

    assert_eq!(running, BTreeSet::from(["work".to_string()]));
}

#[test]
fn a_non_codex_process_with_codex_home_is_ignored() {
    let d = tempdir().unwrap();
    let paths = Paths::with_roots(d.path().join("codex"), d.path().join("buddy"));
    let acct = paths.account_dir("work");
    fs::create_dir_all(&acct).unwrap();
    let mut reg = Registry::default();
    reg.accounts.push(account("work"));

    let mut cmd = Command::new("/bin/sleep");
    cmd.arg("30").env("CODEX_HOME", &acct);
    let mut child = spawn_and_wait_for_name(cmd, "sleep");
    let running = running_accounts(&paths, &reg);
    let _ = child.kill();
    let _ = child.wait();

    assert!(running.is_empty());
}
