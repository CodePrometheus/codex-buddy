use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use codex_buddy_core::doctor;
use codex_buddy_core::init::{self, InitPlan};
use codex_buddy_core::ops;
use codex_buddy_core::paths::{Paths, suggest_alias};
use codex_buddy_core::registry::now_epoch;
use codex_buddy_core::usage;
use pico_args::Arguments;

mod commands;
mod output;

type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

/// A command-line usage mistake (missing/extra arguments), exiting 2 like an unknown command —
/// distinct from an operation that failed (exit 1).
#[derive(Debug)]
struct UsageError(String);

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for UsageError {}

fn usage_err<T>(msg: impl Into<String>) -> CliResult<T> {
    Err(Box::new(UsageError(msg.into())))
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(e) => {
            eprintln!("codex-buddy: {e}");
            if e.downcast_ref::<UsageError>().is_some() {
                ExitCode::from(2)
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

fn run() -> CliResult<i32> {
    let mut args = Arguments::from_env();
    let sub = args.subcommand()?;
    // `run` passes everything after the alias through to codex verbatim, so -h/--help/-V/--version
    // there must reach codex; for every other command these are handled at the top level.
    if sub.as_deref() != Some("run") {
        if args.contains(["-h", "--help"]) {
            print_help();
            return Ok(0);
        }
        if args.contains(["-V", "--version"]) {
            println!("codex-buddy {}", env!("CARGO_PKG_VERSION"));
            return Ok(0);
        }
    }
    match sub.as_deref() {
        Some("init") => {
            cmd_init(args)?;
            Ok(0)
        }
        Some("add") => {
            cmd_add(args)?;
            Ok(0)
        }
        Some("switch") => {
            cmd_switch(args)?;
            Ok(0)
        }
        Some("list") => cmd_list(args),
        Some("current") => cmd_current(args),
        Some("usage") => commands::insights::usage(args),
        Some("recommend") => commands::insights::recommend(args),
        Some("report") => commands::insights::report(args),
        Some("run") => cmd_run(args),
        Some("rename") => {
            cmd_rename(args)?;
            Ok(0)
        }
        Some("remove") => {
            cmd_remove(args)?;
            Ok(0)
        }
        Some("import") => commands::transfer::import(args),
        Some("export") => {
            commands::transfer::export(args)?;
            Ok(0)
        }
        Some("relogin") => {
            cmd_relogin(args)?;
            Ok(0)
        }
        Some("doctor") => cmd_doctor(args),
        Some("path") => {
            cmd_path(args)?;
            Ok(0)
        }
        Some(other) => {
            eprintln!("unknown command: {other}\n");
            print_help();
            Ok(2)
        }
        None => {
            print_help();
            Ok(0)
        }
    }
}

/// Up to `max` positional args; anything beyond (an extra word, a typo'd flag) errors instead
/// of being silently ignored — `remove alias --ys` must not quietly fall back to interactive.
fn positionals(args: Arguments, max: usize, usage: &str) -> CliResult<Vec<String>> {
    let rest = args.finish();
    if let Some(extra) = rest.get(max) {
        return usage_err(format!(
            "unexpected argument `{}`; usage: {usage}",
            extra.to_string_lossy()
        ));
    }
    Ok(rest
        .into_iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect())
}

fn cmd_init(mut args: Arguments) -> CliResult<()> {
    let yes = args.contains("--yes");
    let rest = positionals(args, 1, "codex-buddy init [alias] [--yes]")?;
    let paths = Paths::from_env()?;

    let alias = match rest.into_iter().next() {
        Some(s) => s,
        None => prompt_alias_for_init(&paths)?,
    };

    let plan = init::plan(&paths, &alias)?;

    print_init_plan(&plan);
    if !yes && !confirm("Proceed with the migration above?")? {
        println!("Cancelled; nothing changed.");
        return Ok(());
    }

    init::apply(&paths, &plan)?;
    println!(
        "Done: account '{}' is managed and set as current.",
        plan.alias
    );
    Ok(())
}

/// When init is run without an alias: run the alias-independent checks first (so "not logged
/// in" / "already initialized" fail before any prompt), then show the detected account and ask
/// for a name, suggesting the email's local part.
fn prompt_alias_for_init(paths: &Paths) -> CliResult<String> {
    let info = init::preflight(paths)?;
    println!("Detected current account:");
    println!("  email : {}", info.email.as_deref().unwrap_or("-"));
    println!("  plan  : {}", info.plan.as_deref().unwrap_or("-"));
    println!();
    let suggested = suggest_alias(info.email.as_deref());
    Ok(prompt_with_default("Alias for this account", &suggested)?)
}

fn cmd_add(args: Arguments) -> CliResult<()> {
    let rest = positionals(args, 1, "codex-buddy add [alias]")?;
    let alias = match rest.into_iter().next() {
        Some(s) => s,
        None => {
            let a = prompt_line("Alias for the new account: ")?;
            if a.is_empty() {
                return usage_err("an account alias is required");
            }
            a
        }
    };
    let paths = Paths::from_env()?;
    println!("Opening codex login for '{alias}'; complete the login in your browser...");
    ops::add(&paths, &alias)?;
    println!(
        "Account '{alias}' added. Use `codex-buddy switch {alias}`, or `codex-buddy run {alias} -- ...` to run it in parallel."
    );
    Ok(())
}

fn cmd_switch(mut args: Arguments) -> CliResult<()> {
    let next = args.contains("--next");
    let target = positionals(
        args,
        1,
        "codex-buddy switch <alias | -> | codex-buddy switch --next",
    )?
    .into_iter()
    .next();
    if next && target.is_some() {
        return usage_err("switch accepts either an account alias or --next, not both");
    }
    let paths = Paths::from_env()?;
    match target.as_deref() {
        None if next => {
            ops::switch_next(&paths)?;
        }
        Some("-") => ops::switch_previous(&paths)?,
        Some(alias) => ops::switch(&paths, alias)?,
        None => {
            return usage_err(
                "switch needs an account alias, - for the previous account, or --next",
            );
        }
    }
    match ops::current(&paths)? {
        Some(v) => println!("Switched to: {}", fmt_account(&v)),
        None => println!("Switched."),
    }
    Ok(())
}

fn cmd_list(mut args: Arguments) -> CliResult<i32> {
    let json = args.contains("--json");
    positionals(args, 0, "codex-buddy list")?;
    let paths = Paths::from_env()?;
    let views = ops::list(&paths)?;
    if json {
        output::print_json(&views)?;
        return Ok(0);
    }
    if views.is_empty() {
        println!("No accounts yet; run `codex-buddy init`.");
        return Ok(0);
    }

    let s = Style::detect();
    let now = now_epoch();
    let email_of = |v: &ops::AccountView| v.email.clone().unwrap_or_else(|| "-".into());
    let plan_of = |v: &ops::AccountView| v.plan.clone().unwrap_or_else(|| "-".into());

    // Usage columns follow the windows present in the data; codex's window set has changed
    // upstream before (the 5h window disappeared), so none are hardcoded.
    let mut minutes: Vec<i64> = views
        .iter()
        .flat_map(|v| v.usage.iter().flat_map(|u| u.windows.iter()))
        .map(|w| w.window_minutes)
        .collect();
    minutes.sort_unstable();
    minutes.dedup();

    let mut columns: Vec<(String, Vec<String>)> = vec![
        (
            "ALIAS".into(),
            views.iter().map(|v| v.alias.clone()).collect(),
        ),
        ("EMAIL".into(), views.iter().map(email_of).collect()),
        ("PLAN".into(), views.iter().map(plan_of).collect()),
    ];
    for mins in minutes {
        columns.push((
            fmt_window_label(mins).to_uppercase(),
            views
                .iter()
                .map(|v| fmt_window(&v.usage, mins, now))
                .collect(),
        ));
    }

    let widths: Vec<usize> = columns
        .iter()
        .map(|(head, vals)| {
            vals.iter()
                .map(|s| s.chars().count())
                .chain([head.chars().count()])
                .max()
                .unwrap()
        })
        .collect();
    let padded = |cells: Vec<&str>| {
        cells
            .iter()
            .zip(&widths)
            .map(|(cell, w)| format!("{cell:<w$}"))
            .collect::<Vec<_>>()
            .join("  ")
    };

    let heads = padded(columns.iter().map(|(head, _)| head.as_str()).collect());
    println!("{}", s.dim(&format!("  {heads}  LAST USED")));
    for (i, v) in views.iter().enumerate() {
        let mark = if v.is_active { "*" } else { " " };
        let cells = padded(columns.iter().map(|(_, vals)| vals[i].as_str()).collect());
        let line = format!("{mark} {cells}  {}", fmt_ago(v.last_used_at, now));
        if v.is_active {
            println!("{line}");
        } else {
            println!("{}", s.dim(&line));
        }
    }
    Ok(0)
}

fn cmd_current(mut args: Arguments) -> CliResult<i32> {
    let json = args.contains("--json");
    positionals(args, 0, "codex-buddy current")?;
    let paths = Paths::from_env()?;
    let current = ops::current(&paths)?;
    if json {
        output::print_json(&current)?;
        return Ok(0);
    }
    match current {
        Some(v) => println!("{}", fmt_account(&v)),
        None => println!("No active account."),
    }
    Ok(0)
}

fn cmd_run(args: Arguments) -> CliResult<i32> {
    // Everything after the alias is codex's, verbatim — no positional cap, no UTF-8 conversion.
    let mut rest = args.finish().into_iter();
    let Some(alias) = rest.next() else {
        return usage_err("run needs an account alias: codex-buddy run <alias> -- <codex args>");
    };
    let mut passthrough: Vec<OsString> = rest.collect();
    if passthrough.first().map(OsString::as_os_str) == Some(OsStr::new("--")) {
        passthrough.remove(0);
    }
    let paths = Paths::from_env()?;
    Ok(ops::run(&paths, &alias.to_string_lossy(), &passthrough)?)
}

fn cmd_rename(args: Arguments) -> CliResult<()> {
    let usage = "codex-buddy rename <old> <new>";
    let mut rest = positionals(args, 2, usage)?.into_iter();
    let (Some(old), Some(new)) = (rest.next(), rest.next()) else {
        return usage_err(format!("rename needs: {usage}"));
    };
    let paths = Paths::from_env()?;
    ops::rename(&paths, &old, &new)?;
    println!("Renamed: {old} -> {new}");
    Ok(())
}

fn cmd_remove(mut args: Arguments) -> CliResult<()> {
    let yes = args.contains("--yes");
    let Some(alias) = positionals(args, 1, "codex-buddy remove <alias> [--yes]")?
        .into_iter()
        .next()
    else {
        return usage_err("remove needs an account alias");
    };
    let paths = Paths::from_env()?;
    // Resolve before prompting: confirming deletion of a nonexistent account is noise.
    ops::account_home(&paths, &alias)?;
    if !yes
        && !confirm(&format!(
            "Remove account '{alias}' and delete its credentials? This cannot be undone."
        ))?
    {
        println!("Cancelled; nothing changed.");
        return Ok(());
    }
    ops::remove(&paths, &alias)?;
    println!("Removed account '{alias}'.");
    Ok(())
}

fn cmd_relogin(args: Arguments) -> CliResult<()> {
    let Some(alias) = positionals(args, 1, "codex-buddy relogin <alias>")?
        .into_iter()
        .next()
    else {
        return usage_err("relogin needs an account alias");
    };
    let paths = Paths::from_env()?;
    println!("Opening codex login for '{alias}'; complete the login in your browser...");
    ops::relogin(&paths, &alias)?;
    println!("Re-logged in '{alias}'.");
    Ok(())
}

fn cmd_doctor(mut args: Arguments) -> CliResult<i32> {
    let json = args.contains("--json");
    positionals(args, 0, "codex-buddy doctor")?;
    let paths = Paths::from_env()?;
    let checks = doctor::diagnose(&paths);
    if json {
        output::print_json(&checks)?;
        return Ok(
            if checks
                .iter()
                .any(|check| check.level == doctor::Level::Fail)
            {
                1
            } else {
                0
            },
        );
    }
    let mut has_fail = false;
    for c in &checks {
        print_check(c);
        if c.level == doctor::Level::Fail {
            has_fail = true;
        }
    }
    Ok(if has_fail { 1 } else { 0 })
}

fn print_check(check: &doctor::Check) {
    let tag = match check.level {
        doctor::Level::Pass => "ok  ",
        doctor::Level::Warn => "warn",
        doctor::Level::Fail => "fail",
    };
    println!("[{tag}] {}", check.message);
}

fn cmd_path(args: Arguments) -> CliResult<()> {
    let Some(alias) = positionals(args, 1, "codex-buddy path <alias>")?
        .into_iter()
        .next()
    else {
        return usage_err("path needs an account alias");
    };
    let paths = Paths::from_env()?;
    println!("{}", ops::account_home(&paths, &alias)?.display());
    Ok(())
}

fn fmt_account(v: &ops::AccountView) -> String {
    format!(
        "{}  {}  [{}]",
        v.alias,
        v.email.as_deref().unwrap_or("-"),
        v.plan.as_deref().unwrap_or("-"),
    )
}

/// One window's usage as `34% (3h)` (used percent + countdown to reset), or `-` when the window
/// has no data or its reset is already in the past.
fn fmt_window(u: &Option<usage::Usage>, mins: i64, now: i64) -> String {
    let Some(u) = u else {
        return "-".to_string();
    };
    match u.windows.iter().find(|w| w.window_minutes == mins) {
        Some(w) if !w.is_expired(now) => fmt_window_value(w.used_percent, w.resets_at, now),
        _ => "-".to_string(),
    }
}

fn fmt_window_value(used_percent: f64, resets_at: Option<i64>, now: i64) -> String {
    let used = used_percent.clamp(0.0, 100.0);
    match resets_at {
        Some(reset) => format!("{used:.0}% ({})", fmt_duration(reset - now)),
        None => format!("{used:.0}%"),
    }
}

/// Short name for a rate-limit window: `5h`, `1w`, then generic `3d` / `90m` fallbacks.
fn fmt_window_label(minutes: i64) -> String {
    match minutes {
        usage::FIVE_HOUR_MINUTES => "5h".into(),
        usage::WEEKLY_MINUTES => "1w".into(),
        m if m % 1440 == 0 => format!("{}d", m / 1440),
        m if m % 60 == 0 => format!("{}h", m / 60),
        m => format!("{m}m"),
    }
}

/// Coarse duration like `12m` / `3h` / `5d`.
fn fmt_duration(secs: i64) -> String {
    let s = secs.max(0);
    if s < 3600 {
        format!("{}m", s / 60)
    } else if s < 86400 {
        format!("{}h", s / 3600)
    } else {
        format!("{}d", s / 86400)
    }
}

/// Time since an epoch as `just now` / `6m ago` / `3h ago` / `2d ago`.
fn fmt_ago(t: Option<i64>, now: i64) -> String {
    let Some(t) = t else {
        return "-".to_string();
    };
    let s = (now - t).max(0);
    if s < 60 {
        "just now".to_string()
    } else if s < 3600 {
        format!("{}m ago", s / 60)
    } else if s < 86400 {
        format!("{}h ago", s / 3600)
    } else {
        format!("{}d ago", s / 86400)
    }
}

/// Minimal ANSI styling, disabled when stdout is not a terminal, `NO_COLOR` is set
/// (https://no-color.org), or the terminal declares itself dumb.
struct Style {
    on: bool,
}

impl Style {
    fn detect() -> Self {
        Self {
            on: io::stdout().is_terminal()
                && env::var_os("NO_COLOR").is_none()
                && env::var_os("TERM").is_none_or(|t| t != "dumb"),
        }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.on {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn dim(&self, text: &str) -> String {
        self.paint("2", text)
    }
}

fn print_init_plan(plan: &InitPlan) {
    println!("About to run the first-time migration:\n");
    println!("  alias   : {}", plan.alias);
    println!("  account : {}", plan.account_key);
    println!("  email   : {}", plan.email.as_deref().unwrap_or("-"));
    println!("  plan    : {}", plan.plan.as_deref().unwrap_or("-"));
    println!();
    let codex_home = plan
        .codex_auth
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.codex".into());
    println!("  move    {} (become per-account)", plan.moves.join(", "));
    println!("  from    {codex_home}");
    println!("  into    {}", plan.account_dir.display());
    println!("  each is replaced by a symlink to this account's copy");
    println!("  backup  auth.json -> {}", plan.backup_path.display());
    println!();
    println!("  unchanged: config.toml / sqlite / everything else (stays shared)");
    println!();
}

fn confirm(prompt: &str) -> io::Result<bool> {
    print!("{prompt} [y/N] ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let a = line.trim().to_ascii_lowercase();
    Ok(a == "y" || a == "yes")
}

fn prompt_with_default(prompt: &str, default: &str) -> io::Result<String> {
    print!("{prompt} [{default}]: ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let s = line.trim();
    Ok(if s.is_empty() {
        default.to_string()
    } else {
        s.to_string()
    })
}

fn prompt_line(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn print_help() {
    println!(
        "codex-buddy — multi-account switcher & parallel runner for the Codex CLI\n\n\
Setup\n  \
init [alias] [--yes]        adopt the current ~/.codex account\n  \
add <alias>                 log in and adopt a new account\n  \
import <path> [options]     import one auth.json or an <alias>/auth.json directory\n  \
export <alias> <path>       export one auth.json (--force to overwrite)\n  \
relogin <alias>             re-login an existing account\n  \
rename <old> <new>          rename an account\n  \
remove <alias> [--yes]      remove an account\n\n\
Use\n  \
list [--json]               list accounts with usage\n  \
current [--json]            show the active account\n  \
usage [alias] [options]     show usage; --remote fetches live via codex\n  \
recommend [options]         recommend from comparable usage windows\n  \
switch <alias> | - | --next switch account (- = previous)\n  \
run <alias> -- <args>       run codex under an account (parallel)\n  \
path <alias>                print an account's CODEX_HOME\n  \
doctor [--json]             check setup health\n  \
report [--json]             summarize accounts and health\n\n  \
-h, --help                  show this help\n  \
-V, --version               show the version"
    );
}
