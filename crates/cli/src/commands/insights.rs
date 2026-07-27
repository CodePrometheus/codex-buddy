use codex_buddy_core::doctor;
use codex_buddy_core::ops;
use codex_buddy_core::paths::Paths;
use codex_buddy_core::recommend;
use codex_buddy_core::registry::now_epoch;
use codex_buddy_core::remote;
use pico_args::Arguments;

use crate::output;
use crate::{
    CliResult, fmt_duration, fmt_window_label, fmt_window_value, positionals, print_check,
};

pub fn usage(mut args: Arguments) -> CliResult<i32> {
    let json = args.contains("--json");
    let remote = args.contains("--remote");
    let alias = positionals(args, 1, "codex-buddy usage [alias] [--remote] [--json]")?
        .into_iter()
        .next();
    let paths = Paths::from_env()?;
    let views = match alias {
        Some(alias) => vec![ops::get(&paths, &alias)?],
        None => ops::list(&paths)?,
    };
    let now = now_epoch();
    let usages: Vec<output::AccountUsage> = if remote {
        views
            .iter()
            .map(|view| {
                output::AccountUsage::from_remote(
                    &view.alias,
                    remote::fetch_account(&paths, &view.alias),
                    now,
                )
            })
            .collect()
    } else {
        views
            .iter()
            .map(|view| output::AccountUsage::from_view(view, now))
            .collect()
    };
    let failed = usages
        .iter()
        .any(|account| account.status == output::UsageStatus::Error);

    if json {
        output::print_json(&usages)?;
        return Ok(if failed { 1 } else { 0 });
    }
    if usages.is_empty() {
        println!("No accounts yet; run `codex-buddy init`.");
        return Ok(0);
    }

    print_usage_table(&usages, now);
    Ok(if failed { 1 } else { 0 })
}

/// Columns are derived from the windows actually present — codex's window set has changed
/// before (the 5h window disappeared upstream), so nothing is hardcoded.
fn print_usage_table(usages: &[output::AccountUsage], now: i64) {
    let mut minutes: Vec<i64> = usages
        .iter()
        .flat_map(|account| account.windows.iter().map(|w| w.window_minutes))
        .collect();
    minutes.sort_unstable();
    minutes.dedup();

    let mut columns: Vec<(String, Vec<String>)> = Vec::with_capacity(2 + minutes.len());
    columns.push((
        "ALIAS".into(),
        usages.iter().map(|a| a.alias.clone()).collect(),
    ));
    columns.push((
        "STATUS".into(),
        usages
            .iter()
            .map(|a| usage_status(a.status).into())
            .collect(),
    ));
    for mins in minutes {
        columns.push((
            fmt_window_label(mins).to_uppercase(),
            usages
                .iter()
                .map(|account| fmt_usage_window(account, mins, now))
                .collect(),
        ));
    }

    let trailing: Vec<String> = usages
        .iter()
        .map(|a| a.error.clone().unwrap_or_default())
        .collect();
    print_table(&columns, &trailing);
}

fn print_table(columns: &[(String, Vec<String>)], trailing: &[String]) {
    let widths: Vec<usize> = columns
        .iter()
        .map(|(head, vals)| {
            vals.iter()
                .map(|s| s.chars().count())
                .chain([head.chars().count()])
                .max()
                .unwrap_or(0)
        })
        .collect();
    let render = |cells: Vec<&str>, trail: &str| {
        let body = cells
            .iter()
            .zip(&widths)
            .map(|(cell, width)| format!("{cell:<width$}"))
            .collect::<Vec<_>>()
            .join("  ");
        println!("{}", format!("  {body}  {trail}").trim_end());
    };
    render(columns.iter().map(|(head, _)| head.as_str()).collect(), "");
    let rows = columns.first().map_or(0, |(_, vals)| vals.len());
    for row in 0..rows {
        render(
            columns.iter().map(|(_, vals)| vals[row].as_str()).collect(),
            trailing.get(row).map(String::as_str).unwrap_or(""),
        );
    }
}

pub fn recommend(mut args: Arguments) -> CliResult<i32> {
    let json = args.contains("--json");
    let remote = args.contains("--remote");
    positionals(args, 0, "codex-buddy recommend [--remote] [--json]")?;
    let paths = Paths::from_env()?;
    let result = if remote {
        recommend::recommend_remote(&paths)?
    } else {
        recommend::recommend(&paths)?
    };
    if json {
        output::print_json(&result)?;
        return Ok(0);
    }

    let now = now_epoch();
    println!("Recommended: {}", result.alias);
    println!(
        "  bottleneck: {} with {:.0}% remaining",
        fmt_window_label(result.bottleneck_window_minutes),
        result.remaining_percent
    );
    for window in result.windows {
        println!(
            "  {}: {:.0}% used, {:.0}% remaining{}",
            fmt_window_label(window.window_minutes),
            window.used_percent,
            window.remaining_percent,
            window
                .resets_at
                .map(|reset| format!(", resets in {}", fmt_duration(reset - now)))
                .unwrap_or_default()
        );
    }
    Ok(0)
}

pub fn report(mut args: Arguments) -> CliResult<i32> {
    let json = args.contains("--json");
    positionals(args, 0, "codex-buddy report [--json]")?;
    let report = doctor::report(&Paths::from_env()?);
    if json {
        output::print_json(&report)?;
    } else {
        println!("Accounts: {}", report.account_count);
        println!(
            "Active: {}",
            report.active_account.as_deref().unwrap_or("-")
        );
        println!(
            "Checks: {} pass, {} warn, {} fail",
            report.pass_count, report.warn_count, report.fail_count
        );
        for check in &report.checks {
            print_check(check);
        }
    }
    Ok(if report.fail_count > 0 { 1 } else { 0 })
}

fn fmt_usage_window(account: &output::AccountUsage, mins: i64, now: i64) -> String {
    match account
        .windows
        .iter()
        .find(|window| window.window_minutes == mins)
    {
        Some(window) if window.expired => "expired".into(),
        Some(window) => fmt_window_value(window.used_percent, window.resets_at, now),
        None => "-".into(),
    }
}

fn usage_status(status: output::UsageStatus) -> &'static str {
    match status {
        output::UsageStatus::Fresh => "fresh",
        output::UsageStatus::Expired => "expired",
        output::UsageStatus::Missing => "missing",
        output::UsageStatus::Error => "error",
    }
}
