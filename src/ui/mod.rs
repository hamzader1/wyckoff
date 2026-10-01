//! Terminal presentation: the report, the confirmation prompt, the spinner.
//!
//! Everything here is skipped when stdout is not a terminal, so piping wyckoff
//! into a file, a script or a test still produces exactly one line on stdout and
//! no decoration. That is also what keeps the git hook safe.

mod spinner;

pub use spinner::Spinner;

use std::io::{self, IsTerminal};
use std::time::Duration;

use crate::output::Meta;
use crate::validate::{Issue, Level};

pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";
pub const BOLD: &str = "\x1b[1m";
pub const RED: &str = "\x1b[31m";
pub const YELLOW: &str = "\x1b[33m";
pub const CYAN: &str = "\x1b[36m";

/// True when there is a real terminal on both ends, i.e. when drawing a menu and
/// a spinner makes sense.
pub fn interactive() -> bool {
    io::stdout().is_terminal() && io::stderr().is_terminal()
}

fn step(text: &str) {
    eprintln!("{CYAN}◇{RESET}  {text}");
}

fn gutter() {
    eprintln!("{DIM}│{RESET}");
}

/// `◇  📁 Detected 3 staged files:` followed by the paths.
pub fn report_staged(paths: &[String]) {
    const SHOWN: usize = 12;
    let count = paths.len();
    step(&format!(
        "📁  {BOLD}Detected {} staged {}{RESET}",
        count,
        if count == 1 { "file" } else { "files" }
    ));
    for path in paths.iter().take(SHOWN) {
        eprintln!("      {DIM}{path}{RESET}");
    }
    if count > SHOWN {
        eprintln!("      {DIM}… and {} more{RESET}", count - SHOWN);
    }
    gutter();
}

/// `◇  ✅ Changes analyzed in 7.8s`
pub fn report_timing(elapsed: Duration, meta: &Meta) {
    step(&format!(
        "✅  {BOLD}Changes analyzed in {:.1}s{RESET}",
        elapsed.as_secs_f64()
    ));
    eprintln!(
        "      {DIM}{} · {} · ~{} tokens{RESET}",
        meta.provider, meta.model, meta.prompt_tokens
    );
    gutter();
}

/// The message itself, in the middle of the report.
pub fn print_message(text: &str) {
    println!();
    for line in text.lines() {
        println!("  {BOLD}{line}{RESET}");
    }
    println!();
}

pub fn report_issues(issues: &[Issue]) {
    for issue in issues {
        let (label, colour) = match issue.level {
            Level::Error => ("error", RED),
            Level::Warning => ("warn ", YELLOW),
        };
        eprintln!("{colour}│  {label}{RESET}  {DIM}{}{RESET}", issue.message);
    }
}

pub fn report_note(text: &str) {
    eprintln!("{DIM}│  {text}{RESET}");
}

pub fn report_aborted() {
    step(&format!("○  {DIM}nothing committed{RESET}"));
}

pub fn close() {
    eprintln!("{DIM}└{RESET}");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Commit with this message.
    Yes,
    /// Abort: nothing is committed.
    No,
    /// Let the author edit the message in their own editor.
    Edit,
    /// Ask the model again, for a different message.
    Retry,
}

const ITEMS: &[&str] = &[
    "Yes, commit this",
    "No, abort",
    "Edit in $EDITOR",
    "Retry, ask again",
];

pub fn ask() -> io::Result<Choice> {
    // ColorfulTheme ships with dialoguer; swapping in a custom theme (for ●/○
    // bullets, say) is a one-line change here.
    let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Use this commit message?")
        .items(ITEMS)
        .default(0)
        .interact()
        .map_err(|error| io::Error::other(error.to_string()))?;

    Ok(match selection {
        0 => Choice::Yes,
        1 => Choice::No,
        2 => Choice::Edit,
        _ => Choice::Retry,
    })
}

/// Open `$VISUAL`/`$EDITOR` (vim by default) on the current message.
pub fn edit(initial: &str) -> io::Result<Option<String>> {
    let edited = dialoguer::Editor::new()
        .require_save(true)
        .edit(initial)
        .map_err(|error| io::Error::other(error.to_string()))?;

    Ok(match edited {
        Some(text) if !text.trim().is_empty() => Some(text),
        _ => None,
    })
}
