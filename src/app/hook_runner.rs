//! The `hook-fill` path, used by the installed `prepare-commit-msg` hook.
//!
//! This must never fail a commit: every error becomes a note on stderr and the
//! exit code stays zero.

use std::path::Path;

use crate::cli::GenerateArgs;
use crate::config::{Config, Settings};
use crate::error::Result;
use crate::generate::{self, Options};
use crate::git::Git;
use crate::hook;

pub fn hook_fill(file: &Path, config_path: Option<&Path>, verbose: bool) -> Result<i32> {
    if !hook::should_fill(file) {
        return Ok(0);
    }

    if let Err(error) = fill(file, config_path) {
        // Never fail the commit, but never hide the reason either: an empty
        // editor plus one clear line is the honest outcome.
        eprintln!("wyckoff: no message generated: {error}");
        if verbose {
            eprintln!(
                "         (`wyckoff doctor` checks the provider; `wyckoff --dry-run` shows the request)"
            );
        }
    }
    Ok(0)
}

fn fill(file: &Path, config_path: Option<&Path>) -> Result<()> {
    let git = Git::discover()?;

    // A merge, rebase or cherry-pick has its own message and its own rules.
    if let Some(operation) = git.operation_in_progress() {
        eprintln!("wyckoff: leaving the {operation} message alone");
        return Ok(());
    }

    let (config, _) = Config::load(config_path)?;
    let settings = Settings::from_config(&config);
    let options = Options::from_args(&GenerateArgs::default(), &settings);
    let outcome = generate::run(&git, &config, &options)?;

    if outcome.text.trim().is_empty() {
        return Ok(());
    }
    hook::write_message(file, &outcome.text)?;
    eprintln!("wyckoff: wrote a message you can edit, or clear it to write your own:");
    eprintln!("  {}", outcome.text.replace('\n', "\n  "));
    Ok(())
}
