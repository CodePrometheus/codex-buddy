use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};
use tempfile::TempDir;

struct TestHome {
    _temp: TempDir,
    home: PathBuf,
    codex: PathBuf,
}

impl TestHome {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let codex = home.join(".codex");
        fs::create_dir_all(codex.join("sessions")).unwrap();
        fs::write(codex.join("auth.json"), auth_json("usera", "accta")).unwrap();
        fs::write(codex.join("config.toml"), "x").unwrap();
        Self {
            _temp: temp,
            home,
            codex,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_codex-buddy"))
            .args(args)
            .env("HOME", &self.home)
            .env("CODEX_HOME", &self.codex)
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    }

    fn init(&self) {
        let output = self.run(&["init", "a", "--yes"]);
        assert_success(&output);
    }

    fn run_with_path(&self, args: &[&str], extra_path: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_codex-buddy"))
            .args(args)
            .env("HOME", &self.home)
            .env("CODEX_HOME", &self.codex)
            .env("NO_COLOR", "1")
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    extra_path.display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .output()
            .unwrap()
    }
}

#[test]
fn json_commands_and_next_use_the_public_core_views() {
    let test = TestHome::new();
    test.init();
    let source = test.home.join("b-auth.json");
    fs::write(&source, auth_json("userb", "acctb")).unwrap();
    let source_arg = source.to_string_lossy();
    assert_success(&test.run(&["import", &source_arg, "--alias", "b"]));

    let list = test.run(&["list", "--json"]);
    assert_success(&list);
    let accounts: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(accounts.as_array().unwrap().len(), 2);
    assert!(accounts[0].get("account_key").is_none());

    assert_success(&test.run(&["switch", "--next"]));
    let current = test.run(&["current", "--json"]);
    assert_success(&current);
    assert_eq!(
        serde_json::from_slice::<Value>(&current.stdout).unwrap()["alias"],
        "b"
    );

    let usage = test.run(&["usage", "--json"]);
    assert_success(&usage);
    assert_eq!(
        serde_json::from_slice::<Value>(&usage.stdout).unwrap()[0]["status"],
        "missing"
    );

    let doctor = test.run(&["doctor", "--json"]);
    assert_success(&doctor);
    let checks: Value = serde_json::from_slice(&doctor.stdout).unwrap();
    assert!(checks[0].get("code").is_some());

    let report = test.run(&["report", "--json"]);
    assert_success(&report);
    assert_eq!(
        serde_json::from_slice::<Value>(&report.stdout).unwrap()["account_count"],
        2
    );
}

#[test]
fn directory_import_reports_partial_success_and_export_is_safe() {
    let test = TestHome::new();
    test.init();
    let imports = test.home.join("imports");
    write_import(&imports, "b", &auth_json("userb", "acctb"));
    write_import(&imports, "broken", "{}");

    let imports_arg = imports.to_string_lossy();
    let imported = test.run(&["import", &imports_arg, "--json"]);
    assert_eq!(imported.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&imported.stdout).unwrap();
    assert_eq!(report["imported"], 1);
    assert_eq!(report["failed"], 1);

    let destination = test.home.join("exported-auth.json");
    let destination_arg = destination.to_string_lossy();
    assert_success(&test.run(&["export", "b", &destination_arg]));
    assert_eq!(
        fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
        0o600
    );

    fs::write(&destination, "old").unwrap();
    assert_eq!(
        test.run(&["export", "b", &destination_arg]).status.code(),
        Some(1)
    );
    assert_success(&test.run(&["export", "b", &destination_arg, "--force"]));
    assert_ne!(fs::read_to_string(destination).unwrap(), "old");
}

#[test]
fn remote_usage_and_recommend_ask_codex_per_account() {
    let test = TestHome::new();
    test.init();
    let source = test.home.join("b-auth.json");
    fs::write(&source, auth_json("userb", "acctb")).unwrap();
    let source_arg = source.to_string_lossy();
    assert_success(&test.run(&["import", &source_arg, "--alias", "b"]));

    // A fake `codex` that answers the app-server RPC, with usage keyed off CODEX_HOME so each
    // account gets distinct numbers: `a` is nearly drained, `b` has headroom.
    let bin = test.home.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let script = r#"#!/bin/sh
case "$CODEX_HOME" in
  */a) used=80 ;;
  *) used=20 ;;
esac
read line
echo '{"jsonrpc":"2.0","id":0,"result":{}}'
read line
read line
echo "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"rateLimits\":{\"primary\":{\"usedPercent\":$used,\"windowDurationMins\":10080,\"resetsAt\":9999999999},\"secondary\":null,\"planType\":\"pro\"}}}"
"#;
    fs::write(bin.join("codex"), script).unwrap();
    fs::set_permissions(bin.join("codex"), fs::Permissions::from_mode(0o755)).unwrap();

    let usage = test.run_with_path(&["usage", "--remote", "--json"], &bin);
    assert_success(&usage);
    let accounts: Value = serde_json::from_slice(&usage.stdout).unwrap();
    assert_eq!(accounts[0]["source"], "remote");
    assert_eq!(accounts[0]["status"], "fresh");
    assert_eq!(accounts[0]["windows"][0]["used_percent"], 80.0);

    let recommend = test.run_with_path(&["recommend", "--remote", "--json"], &bin);
    assert_success(&recommend);
    assert_eq!(
        serde_json::from_slice::<Value>(&recommend.stdout).unwrap()["alias"],
        "b"
    );
}

fn write_import(root: &Path, alias: &str, auth: &str) {
    let dir = root.join(alias);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("auth.json"), auth).unwrap();
}

fn auth_json(user: &str, account: &str) -> String {
    let claims = json!({
        "email": format!("{user}@example.com"),
        "https://api.openai.com/auth": {
            "chatgpt_user_id": user,
            "chatgpt_account_id": account,
            "chatgpt_plan_type": "pro"
        }
    });
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
    json!({
        "tokens": {
            "id_token": format!("x.{payload}.x"),
            "account_id": account
        }
    })
    .to_string()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
