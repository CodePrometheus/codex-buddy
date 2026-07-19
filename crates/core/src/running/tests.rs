use super::*;
use crate::registry::AccountRecord;
use std::fs;
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

#[test]
fn lsof_lists_command_matches_the_command_column() {
    let out = "COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\n\
               codex   123 me    3u   REG  1,15   0        1 /a/b\n";
    assert!(lsof_lists_command(out, "codex"));
}

#[test]
fn lsof_lists_command_does_not_match_other_processes() {
    let out = "COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\n\
               sleep   123 me    3u   REG  1,15   0        1 /a/b\n";
    assert!(!lsof_lists_command(out, "codex"));
}

#[test]
fn lsof_lists_command_skips_the_header_row() {
    let out = "codex   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME\n\
               sleep   123 me    3u   REG  1,15   0        1 /a/b\n";
    assert!(!lsof_lists_command(out, "codex"));
}

#[test]
fn lsof_lists_command_empty_output_is_false() {
    assert!(!lsof_lists_command("", "codex"));
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
fn an_unrelated_process_holding_a_file_open_is_not_mistaken_for_codex() {
    let d = tempdir().unwrap();
    let paths = Paths::with_roots(d.path().join("codex"), d.path().join("buddy"));
    let acct_dir = paths.account_dir("work");
    fs::create_dir_all(&acct_dir).unwrap();
    let probe = acct_dir.join("state_5.sqlite-shm");
    fs::write(&probe, b"").unwrap();

    let mut holder = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("exec 3<>'{}'; sleep 5", probe.display()))
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));

    assert!(!has_codex_handle(&acct_dir));

    let _ = holder.kill();
    let _ = holder.wait();
}
