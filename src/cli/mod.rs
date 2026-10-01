//! Command line surface.
//!
//! Bare `wyckoff` is the tool: it reads the staged diff and prints a message.
//! Everything else is a subcommand, so the common case needs no typing.

pub mod actions;

#[cfg(test)]
#[path = "cli_tests.rs"]
mod cli_tests;

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

pub use actions::{CacheAction, ConfigAction, HookAction, InitArgs, ProviderArgs};

#[derive(Parser, Debug)]
#[command(
    name = "wyckoff",
    version,
    about = "Commit messages written from your staged diff, in your own voice.",
    long_about = "Reads your staged change, your project's .context, your branch name and how you \
                  have been writing commit messages, then writes one line that sounds like you \
                  wrote it. No prefixes, no templates.",
    max_term_width = 100
)]
pub struct Cli {
    #[command(flatten)]
    pub generate: GenerateArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Args, Debug, Default, Clone)]
pub struct GenerateArgs {
    /// Provider to use (gemini, claude, openai, zen, nvidia, ollama, ...).
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Model id; overrides the provider default.
    #[arg(short, long)]
    pub model: Option<String>,

    /// Follow the repo's history (auto), always write a bare sentence (plain),
    /// or always use type prefixes (conventional).
    #[arg(long, value_enum)]
    pub style: Option<StyleChoice>,

    /// Language of the message, or `auto` to detect it from history.
    #[arg(long)]
    pub language: Option<String>,

    /// Tell wyckoff what you were doing ("fix the login race"). The single
    /// biggest quality lever there is.
    #[arg(short = 'I', long)]
    pub intent: Option<String>,

    /// Allow a short bullet body under the subject.
    #[arg(short, long)]
    pub body: bool,

    /// Maximum subject length.
    #[arg(long)]
    pub max_len: Option<usize>,

    /// Use the working tree instead of the index (`git diff`).
    #[arg(long)]
    pub unstaged: bool,

    /// Use everything since HEAD (`git diff HEAD`).
    #[arg(long = "all")]
    pub everything: bool,

    /// Extra `.context`-style file to read.
    #[arg(long)]
    pub context_file: Option<PathBuf>,

    /// Never read or write the cache.
    #[arg(long)]
    pub no_cache: bool,

    /// Recompute cached values.
    #[arg(long)]
    pub refresh: bool,

    /// Copy the message to the clipboard.
    #[arg(long)]
    pub copy: bool,

    /// Print JSON instead of plain text.
    #[arg(long)]
    pub json: bool,

    /// Show the request that would be sent, and send nothing.
    #[arg(long)]
    pub dry_run: bool,

    /// Send the diff even if it looks like it contains secrets.
    #[arg(long)]
    pub allow_secrets: bool,

    /// Commit without asking: skips the interactive review prompt.
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// HTTP timeout in seconds for the provider call.
    #[arg(long)]
    pub timeout: Option<u64>,

    /// Extra paths whose diffs are never sent (glob-lite).
    #[arg(long = "exclude")]
    pub exclude: Vec<String>,

    /// Show provider, token and cache details.
    #[arg(long)]
    pub stats: bool,

    /// Commit the change with the generated message.
    #[arg(long)]
    pub commit: bool,

    /// Amend the previous commit instead of creating a new one (implies --commit).
    #[arg(long)]
    pub amend: bool,

    /// Do not print the message (used by the hook).
    #[arg(short, long)]
    pub quiet: bool,

    #[arg(short, long)]
    pub verbose: bool,

    /// Config file to use instead of the default location.
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Limit the diff to these paths (after `--`).
    #[arg(last = true)]
    pub paths: Vec<String>,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleChoice {
    Auto,
    Plain,
    Conventional,
}

impl StyleChoice {
    pub fn as_str(self) -> &'static str {
        match self {
            StyleChoice::Auto => "auto",
            StyleChoice::Plain => "plain",
            StyleChoice::Conventional => "conventional",
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Generate a message and commit with it.
    Commit(GenerateArgs),

    /// Describe the staged change in prose instead of a commit message.
    Explain(GenerateArgs),

    /// Install, remove or inspect the prepare-commit-msg hook.
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },

    /// Internal: called by the installed hook.
    #[command(name = "hook-fill", hide = true)]
    HookFill {
        file: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(short, long)]
        verbose: bool,
    },

    /// Write a starter .context for this repository.
    Init(InitArgs),

    /// List the models the provider offers.
    Models(ProviderArgs),

    /// Check the repo, the config, the key and the model.
    Doctor(ProviderArgs),

    /// Show or create the config file.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Inspect the cache.
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}
