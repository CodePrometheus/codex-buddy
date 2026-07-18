use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use codex_buddy_core::auth;
use codex_buddy_core::doctor;
use codex_buddy_core::init::{self, InitPlan};
use codex_buddy_core::ops;
use codex_buddy_core::paths::{Paths, suggest_alias};
use codex_buddy_core::usage;
use pico_args::Arguments;

type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(e) => {
            eprintln!("codex-buddy: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> CliResult<i32> {
    let mut args = Arguments::from_env();
    let sub = args.subcommand()?;
    // `run` passes everything after the alias through to codex verbatim, so a -h/--help there
    // must reach codex; for every other command a top-level -h/--help prints our help.
    if sub.as_deref() != Some("run") && args.contains(["-h", "--help"]) {
        print_help();
        return Ok(0);
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
        Some("list") => {
            cmd_list()?;
            Ok(0)
        }
        Some("current") => {
            cmd_current()?;
            Ok(0)
        }
        Some("run") => cmd_run(args),
        Some("rename") => {
            cmd_rename(args)?;
            Ok(0)
        }
        Some("remove") => {
            cmd_remove(args)?;
            Ok(0)
        }
        Some("import") => {
            cmd_import(args)?;
            Ok(0)
        }
        Some("relogin") => {
            cmd_relogin(args)?;
            Ok(0)
        }
        Some("doctor") => cmd_doctor(),
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

fn cmd_init(mut args: Arguments) -> CliResult<()> {
    let yes = args.contains("--yes");
    let rest = args.finish();
    let paths = Paths::from_env()?;

    let alias = match rest.first() {
        Some(s) => s.to_string_lossy().into_owned(),
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

/// When init is run without an alias: show the detected account and ask for one,
/// suggesting the email's local part.
fn prompt_alias_for_init(paths: &Paths) -> CliResult<String> {
    let suggested = match auth::load_auth_info(&paths.codex_auth()) {
        Ok(info) => {
            println!("Detected current account:");
            println!("  email : {}", info.email.as_deref().unwrap_or("-"));
            println!("  plan  : {}", info.plan.as_deref().unwrap_or("-"));
            println!();
            suggest_alias(info.email.as_deref())
        }
        Err(_) => "default".to_string(),
    };
    Ok(prompt_with_default("Alias for this account", &suggested)?)
}

fn cmd_add(args: Arguments) -> CliResult<()> {
    let rest = args.finish();
    let alias = match rest.first() {
        Some(s) => s.to_string_lossy().into_owned(),
        None => {
            let a = prompt_line("Alias for the new account: ")?;
            if a.is_empty() {
                return Err("an account alias is required".into());
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

fn cmd_switch(args: Arguments) -> CliResult<()> {
    let rest = args.finish();
    let target = rest
        .first()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or("switch needs an account alias (or - for the previous one)")?;

    let paths = Paths::from_env()?;
    if target == "-" {
        ops::switch_previous(&paths)?;
    } else {
        ops::switch(&paths, &target)?;
    }
    if let Some(v) = ops::current(&paths)? {
        println!("Switched to: {}", fmt_account(&v));
    }
    Ok(())
}

fn cmd_list() -> CliResult<()> {
    let paths = Paths::from_env()?;
    let views = ops::list(&paths)?;
    if views.is_empty() {
        println!("No accounts yet; run `codex-buddy init`.");
        return Ok(());
    }

    let s = Style::detect();
    let now = now_epoch();
    let email_of = |v: &ops::AccountView| v.email.clone().unwrap_or_else(|| "-".into());
    let plan_of = |v: &ops::AccountView| v.plan.clone().unwrap_or_else(|| "-".into());

    let aliases: Vec<String> = views.iter().map(|v| v.alias.clone()).collect();
    let emails: Vec<String> = views.iter().map(email_of).collect();
    let plans: Vec<String> = views.iter().map(plan_of).collect();
    let w5: Vec<String> = views
        .iter()
        .map(|v| fmt_window(&v.usage, 300, now))
        .collect();
    let w1: Vec<String> = views
        .iter()
        .map(|v| fmt_window(&v.usage, 10080, now))
        .collect();

    let width = |vals: &[String], head: &str| {
        vals.iter()
            .map(|s| s.chars().count())
            .chain([head.chars().count()])
            .max()
            .unwrap()
    };
    let alias_w = width(&aliases, "ALIAS");
    let email_w = width(&emails, "EMAIL");
    let plan_w = width(&plans, "PLAN");
    let w5_w = width(&w5, "5H");
    let w1_w = width(&w1, "1W");

    println!(
        "{}",
        s.dim(&format!(
            "  {:<alias_w$}  {:<email_w$}  {:<plan_w$}  {:<w5_w$}  {:<w1_w$}  ACTIVE",
            "ALIAS", "EMAIL", "PLAN", "5H", "1W"
        ))
    );
    for (i, v) in views.iter().enumerate() {
        let mark = if v.is_active { "*" } else { " " };
        let line = format!(
            "{mark} {:<alias_w$}  {:<email_w$}  {:<plan_w$}  {:<w5_w$}  {:<w1_w$}  {}",
            aliases[i],
            emails[i],
            plans[i],
            w5[i],
            w1[i],
            fmt_ago(v.last_used_at, now),
        );
        if v.is_active {
            println!("{line}");
        } else {
            println!("{}", s.dim(&line));
        }
    }
    Ok(())
}

fn cmd_current() -> CliResult<()> {
    let paths = Paths::from_env()?;
    match ops::current(&paths)? {
        Some(v) => println!("{}", fmt_account(&v)),
        None => println!("No active account."),
    }
    Ok(())
}

fn cmd_run(args: Arguments) -> CliResult<i32> {
    let rest = args.finish();
    let mut it = rest.into_iter().map(|s| s.to_string_lossy().into_owned());
    let alias = it
        .next()
        .ok_or("run needs an account alias: codex-buddy run <alias> -- <codex args>")?;
    let mut passthrough: Vec<String> = it.collect();
    if passthrough.first().map(String::as_str) == Some("--") {
        passthrough.remove(0);
    }
    let paths = Paths::from_env()?;
    Ok(ops::run(&paths, &alias, &passthrough)?)
}

fn cmd_rename(args: Arguments) -> CliResult<()> {
    let rest = args.finish();
    let old = rest
        .first()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or("rename needs: codex-buddy rename <old> <new>")?;
    let new = rest
        .get(1)
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or("rename needs: codex-buddy rename <old> <new>")?;
    let paths = Paths::from_env()?;
    ops::rename(&paths, &old, &new)?;
    println!("Renamed: {old} -> {new}");
    Ok(())
}

fn cmd_remove(mut args: Arguments) -> CliResult<()> {
    let yes = args.contains("--yes");
    let rest = args.finish();
    let alias = rest
        .first()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or("remove needs an account alias")?;
    let paths = Paths::from_env()?;
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

fn cmd_import(mut args: Arguments) -> CliResult<()> {
    let alias_opt: Option<String> = args.opt_value_from_str("--alias")?;
    let rest = args.finish();
    let src = rest
        .first()
        .map(PathBuf::from)
        .ok_or("import needs a path to an auth.json")?;
    let paths = Paths::from_env()?;
    let alias = match alias_opt {
        Some(a) => a,
        None => {
            let info = auth::load_auth_info(&src)?;
            let suggested = suggest_alias(info.email.as_deref());
            prompt_with_default("Alias for this account", &suggested)?
        }
    };
    ops::import(&paths, &src, &alias)?;
    println!("Imported account '{alias}'.");
    Ok(())
}

fn cmd_relogin(args: Arguments) -> CliResult<()> {
    let rest = args.finish();
    let alias = rest
        .first()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or("relogin needs an account alias")?;
    let paths = Paths::from_env()?;
    println!("Opening codex login for '{alias}'; complete the login in your browser...");
    ops::relogin(&paths, &alias)?;
    println!("Re-logged in '{alias}'.");
    Ok(())
}

fn cmd_doctor() -> CliResult<i32> {
    let paths = Paths::from_env()?;
    let checks = doctor::diagnose(&paths);
    let mut has_fail = false;
    for c in &checks {
        let tag = match c.level {
            doctor::Level::Pass => "ok  ",
            doctor::Level::Warn => "warn",
            doctor::Level::Fail => "fail",
        };
        println!("[{tag}] {}", c.message);
        if c.level == doctor::Level::Fail {
            has_fail = true;
        }
    }
    Ok(if has_fail { 1 } else { 0 })
}

fn cmd_path(args: Arguments) -> CliResult<()> {
    let rest = args.finish();
    let alias = rest
        .first()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or("path needs an account alias")?;
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

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// One window's usage as `34% (3h)` (used percent + countdown to reset), or `-` when the window
/// has no data or its reset is already in the past.
fn fmt_window(u: &Option<usage::Usage>, mins: i64, now: i64) -> String {
    let Some(u) = u else {
        return "-".to_string();
    };
    match u.windows.iter().find(|w| w.window_minutes == mins) {
        Some(w) if w.resets_at.is_none_or(|r| r > now) => {
            let used = w.used_percent.clamp(0.0, 100.0);
            match w.resets_at {
                Some(r) => format!("{used:.0}% ({})", fmt_duration(r - now)),
                None => format!("{used:.0}%"),
            }
        }
        _ => "-".to_string(),
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

/// Minimal ANSI styling, disabled when stdout is not a terminal.
struct Style {
    on: bool,
}

impl Style {
    fn detect() -> Self {
        Self {
            on: io::stdout().is_terminal(),
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
    println!("  move {}", plan.codex_auth.display());
    println!("  to   {}", plan.account_auth.display());
    println!(
        "  replace {} with a symlink to it",
        plan.codex_auth.display()
    );
    println!("  backup the original to {}", plan.backup_path.display());
    println!();
    println!("  unchanged: config.toml / sessions / history / sqlite / everything else");
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
import <path> [--alias a]   adopt an account from an existing auth.json\n  \
relogin <alias>             re-login an existing account\n  \
rename <old> <new>          rename an account\n  \
remove <alias> [--yes]      remove an account\n\n\
Use\n  \
list                        list accounts with usage\n  \
current                     show the active account\n  \
switch <alias> | -          switch account (- = previous)\n  \
run <alias> -- <args>       run codex under an account (parallel)\n  \
path <alias>                print an account's CODEX_HOME\n  \
doctor                      check setup health\n\n  \
-h, --help                  show this help"
    );
}
