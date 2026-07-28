use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::error::{Error, Result};
use crate::ops;
use crate::paths::Paths;
use crate::usage::{Usage, Window};

/// A live rate-limit snapshot fetched through `codex app-server`.
///
/// The network request is made by the official codex binary itself (its own client, auth, and
/// token refresh) — codex-buddy never talks to the backend directly. This is the only sanctioned
/// way for codex-buddy to obtain remote data.
#[derive(Debug, Clone)]
pub struct RemoteUsage {
    pub usage: Usage,
    pub plan: Option<String>,
}

// Healthy round trips answer in a few seconds; long timeouts only stretch how long the UI's
// refresh state stays busy when the network is flaky.
const TIMEOUT: Duration = Duration::from_secs(15);

/// Fetch one account's live usage by running `codex app-server` with the account's dir as
/// `CODEX_HOME` — the same scoping trick `run` uses, so any managed account works, active or not.
pub fn fetch_account(paths: &Paths, alias: &str) -> Result<RemoteUsage> {
    let home = ops::account_home(paths, alias)?;
    let mut cmd = ops::codex_command();
    cmd.env("CODEX_HOME", &home);
    fetch_with(cmd, TIMEOUT)
}

/// Drive one `initialize` -> `account/rateLimits/read` round trip against `<cmd> app-server`,
/// killing the server after the answer (or the deadline) — it is a one-shot probe, not a daemon.
fn fetch_with(mut cmd: Command, timeout: Duration) -> Result<RemoteUsage> {
    let mut child = cmd
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(ops::codex_spawn_error)?;
    let stdin = child.stdin.take().expect("stdin is piped");
    let stdout = child.stdout.take().expect("stdout is piped");

    // The dialogue runs on a worker thread so a stuck server can't block forever: the main
    // thread waits with a deadline and then kills the child, which unblocks the worker via EOF.
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        let _ = sender.send(converse(stdin, stdout));
    });
    let outcome = receiver.recv_timeout(timeout);
    let _ = child.kill();
    let _ = child.wait();
    let _ = worker.join();
    match outcome {
        Ok(result) => result,
        Err(_) => Err(Error::Other(
            "timed out waiting for `codex app-server`".into(),
        )),
    }
}

fn converse(mut stdin: ChildStdin, stdout: ChildStdout) -> Result<RemoteUsage> {
    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "clientInfo": {
                "name": "codex-buddy",
                "title": "codex-buddy",
                "version": env!("CARGO_PKG_VERSION"),
            }
        }
    });
    writeln!(stdin, "{initialize}")?;
    stdin.flush()?;

    for line in BufReader::new(stdout).lines() {
        let Ok(message) = serde_json::from_str::<Value>(&line?) else {
            continue;
        };
        match message.get("id").and_then(Value::as_i64) {
            Some(0) => {
                check_rpc_error(&message)?;
                writeln!(
                    stdin,
                    "{}",
                    json!({"jsonrpc": "2.0", "method": "initialized"})
                )?;
                writeln!(
                    stdin,
                    "{}",
                    json!({"jsonrpc": "2.0", "id": 1, "method": "account/rateLimits/read", "params": {}})
                )?;
                stdin.flush()?;
            }
            Some(1) => return parse_rate_limits(&message),
            // Server notifications and unrelated ids are irrelevant to this one-shot probe.
            _ => {}
        }
    }
    Err(Error::Other(
        "`codex app-server` closed before answering; make sure codex is up to date".into(),
    ))
}

fn check_rpc_error(message: &Value) -> Result<()> {
    match message.get("error") {
        None => Ok(()),
        Some(error) => Err(Error::Other(format!(
            "codex app-server error: {}",
            error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ))),
    }
}

fn parse_rate_limits(message: &Value) -> Result<RemoteUsage> {
    check_rpc_error(message)?;
    let snapshot = message
        .pointer("/result/rateLimits")
        .ok_or_else(|| Error::Other("codex app-server returned no rate-limit snapshot".into()))?;
    let mut windows: Vec<Window> = ["primary", "secondary"]
        .iter()
        .filter_map(|slot| snapshot.get(*slot).and_then(parse_window))
        .collect();
    windows.sort_by_key(|w| w.window_minutes);
    if windows.is_empty() {
        return Err(Error::Other(
            "codex app-server returned no rate-limit windows".into(),
        ));
    }
    Ok(RemoteUsage {
        usage: Usage { windows },
        plan: snapshot
            .get("planType")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

fn parse_window(value: &Value) -> Option<Window> {
    Some(Window {
        window_minutes: value.get("windowDurationMins")?.as_i64()?,
        used_percent: value.get("usedPercent")?.as_f64()?,
        resets_at: value.get("resetsAt").and_then(Value::as_i64),
    })
}

#[cfg(test)]
mod tests;
