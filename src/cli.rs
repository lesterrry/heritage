use std::path::PathBuf;

use clap::Parser;

/// Copy configured files into named target directories, or clear them.
#[derive(Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    /// TOML configuration; relative patterns and targets/ are relative to this file.
    #[arg(short, long, default_value = "config.toml")]
    pub(crate) config: PathBuf,

    /// Process only these targets (repeat the option to select several).
    #[arg(short, long)]
    pub(crate) target: Vec<String>,

    /// Remove all contents of the selected target directories instead of copying.
    #[arg(long)]
    pub(crate) clear: bool,

    /// Disable animated progress output.
    #[arg(long)]
    pub(crate) no_progress: bool,
}
