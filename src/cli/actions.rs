//! The argument groups used by the smaller subcommands.

use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum HookAction {
    /// Install the hook (prefills the editor on a plain `git commit`).
    Install {
        #[arg(long)]
        force: bool,
    },
    Uninstall,
    Status,
}

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// Overwrite an existing .context.
    #[arg(long)]
    pub force: bool,

    /// Print the generated text instead of writing it.
    #[arg(long)]
    pub print: bool,

    /// Where to write it.
    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct ProviderArgs {
    /// Provider to inspect.
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Model to check.
    #[arg(short, long)]
    pub model: Option<String>,

    /// Config file to use.
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Print the config file path.
    Path,
    /// Write a commented starter config.
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Print the effective settings.
    Show,
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Show what is cached for this repo.
    Stat,
    /// Print the cache directory.
    Dir,
    /// Delete this repo's cache entries.
    Clean,
}
