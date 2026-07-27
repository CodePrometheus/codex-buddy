use std::fs;
use std::path::Path;

use serde_json::json;
use tempfile::tempdir;

use super::*;
use crate::ops;
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

    let source = dir.join("b-auth.json");
    fs::write(&source, auth_json("userb", "acctb")).unwrap();
    ops::import(&paths, &source, "b").unwrap();
    paths
}

fn write_usage(paths: &Paths, alias: &str, five_hour: Option<f64>, weekly: Option<f64>) {
    let mut limits = serde_json::Map::new();
    if let Some(used) = five_hour {
        limits.insert(
            "primary".into(),
            json!({
                "used_percent": used,
                "window_minutes": usage::FIVE_HOUR_MINUTES,
                "resets_at": i64::MAX
            }),
        );
    }
    if let Some(used) = weekly {
        limits.insert(
            "secondary".into(),
            json!({
                "used_percent": used,
                "window_minutes": usage::WEEKLY_MINUTES,
                "resets_at": i64::MAX
            }),
        );
    }
    let day = paths.account_dir(alias).join("sessions/2026/07/24");
    fs::create_dir_all(&day).unwrap();
    fs::write(
        day.join("rollout-test.jsonl"),
        json!({"payload": {"rate_limits": limits}}).to_string() + "\n",
    )
    .unwrap();
}

#[test]
fn recommends_the_account_with_the_best_bottleneck_headroom() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    write_usage(&paths, "a", Some(10.0), Some(80.0));
    write_usage(&paths, "b", Some(40.0), Some(40.0));

    let result = recommend(&paths).unwrap();
    assert_eq!(result.alias, "b");
    assert_eq!(result.remaining_percent, 60.0);
    assert_eq!(result.bottleneck_window_minutes, usage::FIVE_HOUR_MINUTES);
}

#[test]
fn ties_prefer_the_least_recently_used_account() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    write_usage(&paths, "a", None, Some(50.0));
    write_usage(&paths, "b", None, Some(50.0));
    registry::update(&paths, |reg| {
        reg.find_mut("a").unwrap().last_used_at = Some(1);
        reg.find_mut("b").unwrap().last_used_at = None;
        Ok(())
    })
    .unwrap();

    assert_eq!(recommend(&paths).unwrap().alias, "b");
}

#[test]
fn exact_ties_keep_registry_order() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    write_usage(&paths, "a", None, Some(50.0));
    write_usage(&paths, "b", None, Some(50.0));
    registry::update(&paths, |reg| {
        reg.find_mut("a").unwrap().last_used_at = None;
        reg.find_mut("b").unwrap().last_used_at = None;
        Ok(())
    })
    .unwrap();

    assert_eq!(recommend(&paths).unwrap().alias, "a");
}

#[test]
fn mismatched_window_sets_are_not_guessed_across() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    write_usage(&paths, "a", Some(10.0), Some(10.0));
    write_usage(&paths, "b", None, Some(10.0));

    let error = recommend(&paths).unwrap_err().to_string();
    assert!(error.contains("not comparable"), "{error}");
}

#[test]
fn missing_usage_returns_an_explicit_error() {
    let dir = tempdir().unwrap();
    let paths = setup(dir.path());
    write_usage(&paths, "a", None, Some(10.0));

    let error = recommend(&paths).unwrap_err().to_string();
    assert!(
        error.contains("account `b` has no local usage data"),
        "{error}"
    );
}
