use std::path::PathBuf;

use codex_buddy_core::auth;
use codex_buddy_core::ops;
use codex_buddy_core::paths::{Paths, suggest_alias};
use codex_buddy_core::transfer::{self as core_transfer, ImportStatus};
use pico_args::Arguments;

use crate::output;
use crate::{CliResult, positionals, prompt_with_default, usage_err};

pub fn import(mut args: Arguments) -> CliResult<i32> {
    let alias_opt: Option<String> = args.opt_value_from_str("--alias")?;
    let skip_existing = args.contains("--skip-existing");
    let json = args.contains("--json");
    let Some(src) = positionals(
        args,
        1,
        "codex-buddy import <auth.json | directory> [--alias <name>] [--skip-existing] [--json]",
    )?
    .into_iter()
    .next()
    .map(PathBuf::from) else {
        return usage_err("import needs a path to an auth.json");
    };
    let paths = Paths::from_env()?;
    if src.is_dir() {
        if alias_opt.is_some() {
            return usage_err("--alias is only valid when importing one auth.json");
        }
        let results = core_transfer::import_directory(&paths, &src, skip_existing)?;
        let report = output::ImportReport::new(&results);
        if json {
            output::print_json(&report)?;
        } else {
            print_import_report(&report);
        }
        return Ok(if report.failed > 0 { 1 } else { 0 });
    }
    if skip_existing {
        return usage_err("--skip-existing is only valid for directory imports");
    }
    if json && alias_opt.is_none() {
        return usage_err("--json requires --alias when importing one auth.json");
    }
    let alias = match alias_opt {
        Some(alias) => alias,
        None => {
            let info = auth::load_auth_info(&src)?;
            let suggested = suggest_alias(info.email.as_deref());
            prompt_with_default("Alias for this account", &suggested)?
        }
    };
    ops::import(&paths, &src, &alias)?;
    if json {
        output::print_json(&core_transfer::ImportItemResult {
            source: src,
            alias,
            status: ImportStatus::Imported,
            error: None,
        })?;
    } else {
        println!("Imported account '{alias}'.");
    }
    Ok(0)
}

pub fn export(mut args: Arguments) -> CliResult<()> {
    let force = args.contains("--force");
    let usage = "codex-buddy export <alias> <destination> [--force]";
    let mut rest = positionals(args, 2, usage)?.into_iter();
    let (Some(alias), Some(destination)) = (rest.next(), rest.next()) else {
        return usage_err(format!("export needs: {usage}"));
    };
    let destination = PathBuf::from(destination);
    core_transfer::export_account(&Paths::from_env()?, &alias, &destination, force)?;
    println!("Exported account '{alias}' to {}", destination.display());
    Ok(())
}

fn print_import_report(report: &output::ImportReport<'_>) {
    for item in report.items {
        let status = match item.status {
            ImportStatus::Imported => "imported",
            ImportStatus::Skipped => "skipped ",
            ImportStatus::Failed => "failed  ",
        };
        println!(
            "[{status}] {}{}",
            item.alias,
            item.error
                .as_ref()
                .map(|error| format!(": {error}"))
                .unwrap_or_default()
        );
    }
    println!(
        "Imported: {}, skipped: {}, failed: {}",
        report.imported, report.skipped, report.failed
    );
}
