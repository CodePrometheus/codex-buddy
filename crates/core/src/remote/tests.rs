use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use tempfile::tempdir;

use super::*;

fn fake_codex(dir: &Path, body: &str) -> Command {
    let path = dir.join("codex");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    Command::new(path)
}

const HAPPY_SERVER: &str = r#"
read init
echo '{"jsonrpc":"2.0","id":0,"result":{"userAgent":"fake"}}'
read initialized
read request
echo '{"jsonrpc":"2.0","method":"some/notification","params":{}}'
echo '{"jsonrpc":"2.0","id":1,"result":{"rateLimits":{"primary":{"usedPercent":15,"windowDurationMins":10080,"resetsAt":1785727368},"secondary":null,"planType":"plus"}}}'
"#;

#[test]
fn fetch_parses_windows_and_plan() {
    let d = tempdir().unwrap();
    let remote = fetch_with(fake_codex(d.path(), HAPPY_SERVER), Duration::from_secs(10)).unwrap();
    assert_eq!(remote.plan.as_deref(), Some("plus"));
    assert_eq!(remote.usage.windows.len(), 1);
    let window = &remote.usage.windows[0];
    assert_eq!(window.window_minutes, 10080);
    assert_eq!(window.used_percent, 15.0);
    assert_eq!(window.resets_at, Some(1785727368));
}

#[test]
fn fetch_sorts_both_windows_by_duration() {
    let d = tempdir().unwrap();
    let body = r#"
read init
echo '{"jsonrpc":"2.0","id":0,"result":{}}'
read initialized
read request
echo '{"jsonrpc":"2.0","id":1,"result":{"rateLimits":{"primary":{"usedPercent":50,"windowDurationMins":10080},"secondary":{"usedPercent":10,"windowDurationMins":300}}}}'
"#;
    let remote = fetch_with(fake_codex(d.path(), body), Duration::from_secs(10)).unwrap();
    let minutes: Vec<i64> = remote
        .usage
        .windows
        .iter()
        .map(|w| w.window_minutes)
        .collect();
    assert_eq!(minutes, vec![300, 10080]);
}

#[test]
fn a_stuck_server_times_out_and_is_killed() {
    let d = tempdir().unwrap();
    let error = fetch_with(
        fake_codex(d.path(), "read init\nsleep 5"),
        Duration::from_millis(200),
    )
    .unwrap_err();
    assert!(error.to_string().contains("timed out"), "{error}");
}

#[test]
fn garbage_or_early_exit_is_a_clear_error() {
    let d = tempdir().unwrap();
    let error = fetch_with(
        fake_codex(d.path(), "echo not-json\necho '[1,2]'"),
        Duration::from_secs(10),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("closed before answering"),
        "{error}"
    );
}

#[test]
fn rpc_errors_are_surfaced() {
    let d = tempdir().unwrap();
    let body = r#"
read init
echo '{"jsonrpc":"2.0","id":0,"result":{}}'
read initialized
read request
echo '{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"not logged in"}}'
"#;
    let error = fetch_with(fake_codex(d.path(), body), Duration::from_secs(10)).unwrap_err();
    assert!(error.to_string().contains("not logged in"), "{error}");
}

#[test]
fn a_missing_binary_fails_with_context() {
    let error = fetch_with(Command::new("/nonexistent/codex"), Duration::from_secs(1)).unwrap_err();
    assert!(error.to_string().contains("cannot run"), "{error}");
}
