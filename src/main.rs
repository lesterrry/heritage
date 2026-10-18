mod clear;
mod cli;
mod config;
mod copy;
mod matching;

use std::fs;

use anyhow::{Context, Result};
use clap::Parser;

use crate::{clear::clear_target, cli::Cli, config::Config, copy::copy_target};

fn run(cli: Cli) -> Result<()> {
    let config_path = fs::canonicalize(&cli.config)
        .with_context(|| format!("opening config {}", cli.config.display()))?;
    let base = config_path
        .parent()
        .context("config has no parent directory")?;
    let config = Config::load(&config_path)?;
    config.validate_targets(&cli.target)?;
    if !cli.clear {
        config.validate_sources(base)?;
    }
    for (name, target) in &config.target {
        if cli.target.is_empty() || cli.target.contains(name) {
            let result = if cli.clear {
                clear_target(name, base)
            } else {
                copy_target(name, target, base, !cli.no_progress)
            };
            result.with_context(|| format!("target {name:?}"))?;
        }
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run(Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
