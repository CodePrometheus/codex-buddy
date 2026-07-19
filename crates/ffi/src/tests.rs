use super::*;
use codex_buddy_core::usage::{Usage, Window};

fn view(alias: &str, active: bool, usage: Option<Usage>) -> AccountView {
    AccountView {
        alias: alias.into(),
        email: Some(format!("{alias}@example.com")),
        plan: Some("pro".into()),
        account_key: "k".into(),
        is_active: active,
        usage,
        last_used_at: None,
    }
}

#[test]
fn to_account_carries_running_state_from_the_set() {
    let mut running = BTreeSet::new();
    running.insert("work".to_string());

    let running_acc = to_account(view("work", true, None), &running);
    let idle_acc = to_account(view("personal", false, None), &running);

    assert!(running_acc.is_running);
    assert!(!idle_acc.is_running);
}

#[test]
fn to_account_flattens_usage_windows() {
    let usage = Usage {
        windows: vec![
            Window {
                window_minutes: 300,
                used_percent: 46.0,
                resets_at: None,
            },
            Window {
                window_minutes: 10080,
                used_percent: 78.0,
                resets_at: None,
            },
        ],
    };
    let acc = to_account(view("work", true, Some(usage)), &BTreeSet::new());
    assert_eq!(acc.usage.len(), 2);
    assert_eq!(acc.usage[0].window_minutes, 300);
    assert_eq!(acc.usage[1].used_percent, 78.0);
}

#[test]
fn to_account_with_no_usage_is_an_empty_vec() {
    let acc = to_account(view("work", true, None), &BTreeSet::new());
    assert!(acc.usage.is_empty());
}

#[test]
fn to_doctor_check_maps_every_level() {
    let pass = to_doctor_check(doctor::Check {
        level: Level::Pass,
        message: "ok".into(),
    });
    let warn = to_doctor_check(doctor::Check {
        level: Level::Warn,
        message: "hm".into(),
    });
    let fail = to_doctor_check(doctor::Check {
        level: Level::Fail,
        message: "bad".into(),
    });
    assert!(matches!(pass.level, CheckLevel::Pass));
    assert!(matches!(warn.level, CheckLevel::Warn));
    assert!(matches!(fail.level, CheckLevel::Fail));
    assert_eq!(fail.message, "bad");
}

#[test]
fn core_error_becomes_a_readable_ffi_error() {
    let core_err = codex_buddy_core::error::Error::AccountNotFound("work".into());
    let ffi_err: FfiError = core_err.into();
    assert_eq!(ffi_err.to_string(), "account not found: work");
}
