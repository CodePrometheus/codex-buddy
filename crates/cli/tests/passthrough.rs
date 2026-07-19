use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codex-buddy"))
}

fn combined(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

/// `run <alias> -- ... --help` must forward -h/--help to codex, not print codex-buddy's own help.
/// An unknown alias makes it stop at account resolution before ever launching codex.
#[test]
fn run_forwards_help_flag_to_codex() {
    let tmp = tempfile::tempdir().unwrap();
    let out = bin()
        .args(["run", "no-such-account", "--", "--help"])
        .env("HOME", tmp.path())
        .env_remove("CODEX_HOME")
        .output()
        .unwrap();
    let text = combined(&out);
    assert!(
        text.contains("account not found"),
        "expected to reach account resolution, got: {text}"
    );
    assert!(
        !text.contains("multi-account switcher"),
        "codex-buddy help leaked into passthrough: {text}"
    );
}

/// A top-level -h/--help (no `run`) still prints our help.
#[test]
fn top_level_help_prints_help() {
    let out = bin().arg("--help").output().unwrap();
    let text = combined(&out);
    assert!(text.contains("multi-account switcher"), "got: {text}");
}

/// A top-level -V/--version prints our version, not codex's.
#[test]
fn top_level_version_prints_version() {
    let out = bin().arg("--version").output().unwrap();
    let text = combined(&out);
    assert!(
        text.trim() == format!("codex-buddy {}", env!("CARGO_PKG_VERSION")),
        "got: {text}"
    );
}

/// `run <alias> -- ... --version` must forward --version to codex, not print codex-buddy's own.
#[test]
fn run_forwards_version_flag_to_codex() {
    let tmp = tempfile::tempdir().unwrap();
    let out = bin()
        .args(["run", "no-such-account", "--", "--version"])
        .env("HOME", tmp.path())
        .env_remove("CODEX_HOME")
        .output()
        .unwrap();
    let text = combined(&out);
    assert!(
        text.contains("account not found"),
        "expected to reach account resolution, got: {text}"
    );
    assert!(
        !text.starts_with("codex-buddy "),
        "codex-buddy version leaked into passthrough: {text}"
    );
}
