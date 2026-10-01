//! Where the message goes: stdout, the clipboard, or straight into git.

use std::io::Write;

use crate::error::{Error, Result};
use crate::msg::CommitMsg;
use crate::validate::Issue;

/// Everything worth showing alongside the message with `-v`.
#[derive(Debug, Default, Clone)]
pub struct Meta {
    pub provider: String,
    pub model: String,
    pub source: String,
    pub files: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub prompt_tokens: usize,
    pub usage: String,
    /// One line describing the measured commit style.
    pub style: String,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub exemplars: usize,
    pub diff_truncated: bool,
    pub diff_included: usize,
    pub diff_omitted: usize,
    pub diff_summarized: usize,
    /// Seconds spent waiting on the provider.
    pub analysis_secs: f64,
}

pub fn print_message(text: &str, quiet: bool) {
    if quiet {
        return;
    }
    println!("{text}");
}

pub fn print_notes(msg: &CommitMsg, issues: &[Issue], verbose: bool) {
    for warning in crate::validate::warnings(issues) {
        eprintln!("warning: {warning}");
    }
    if verbose && !msg.notes.trim().is_empty() {
        eprintln!("note: {}", msg.notes.trim());
    }
    if verbose {
        match msg.confidence {
            Some(value) => eprintln!("confidence: {:.2}", value),
            None => eprintln!("confidence: not reported"),
        }
    }
}

pub fn print_meta(meta: &Meta) {
    eprintln!("provider: {} ({})", meta.provider, meta.model);
    eprintln!("source: {} diff", meta.source);
    eprintln!(
        "diff: {} files, +{} -{}",
        meta.files, meta.insertions, meta.deletions
    );
    eprintln!("style: {}", meta.style);
    eprintln!(
        "prompt: ~{} tokens, {} exemplars{}",
        meta.prompt_tokens,
        meta.exemplars,
        if meta.diff_truncated {
            format!(
                ", condensed ({} full files, {} summarised, {} left out)",
                meta.diff_included, meta.diff_summarized, meta.diff_omitted
            )
        } else {
            format!(", {} files in full", meta.diff_included)
        }
    );
    eprintln!(
        "cache: {} hit(s), {} miss(es)",
        meta.cache_hits, meta.cache_misses
    );
    eprintln!(
        "analysis: {:.1}s (provider call + repair)",
        meta.analysis_secs
    );
    if !meta.usage.is_empty() {
        eprintln!("usage: {}", meta.usage);
    }
}

pub fn json(msg: &CommitMsg, meta: &Meta) -> Result<String> {
    let value = serde_json::json!({
        "subject": msg.subject,
        "body": msg.body,
        "breaking": msg.breaking,
        "footers": msg.footers,
        "confidence": msg.confidence,
        "notes": msg.notes,
        "message": msg.render(true),
        "meta": {
            "provider": meta.provider,
            "model": meta.model,
            "source": meta.source,
            "files": meta.files,
            "insertions": meta.insertions,
            "deletions": meta.deletions,
            "prompt_tokens": meta.prompt_tokens,
            "usage": meta.usage,
            "style": meta.style
        }
    });
    Ok(serde_json::to_string_pretty(&value)?)
}

/// Copy to the system clipboard. macOS, Wayland, X11 and Windows.
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(windows) {
        &[("clip", &[])]
    } else {
        &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    };

    let mut last_error = String::from("no clipboard tool found");
    for (command, args) in candidates {
        match std::process::Command::new(command)
            .args(*args)
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin
                        .write_all(text.as_bytes())
                        .map_err(|e| Error::msg(e.to_string()))?;
                }
                drop(child.stdin.take());
                if child.wait().map(|s| s.success()).unwrap_or(false) {
                    return Ok(());
                }
                last_error = format!("{command} failed");
            }
            Err(e) => last_error = format!("{command}: {e}"),
        }
    }
    Err(Error::msg(format!(
        "could not copy to the clipboard: {last_error}"
    )))
}

/// `git commit -F -`, with anything from `extra_commit_args`.
pub fn commit(git: &crate::git::Git, message: &str, extra_args: &[String]) -> Result<()> {
    let mut args = extra_args.to_vec();
    if !args.iter().any(|a| a == "--no-verify") {
        // Nothing to add by default; this is where --signoff etc. lands.
    }
    args.retain(|a| !a.is_empty());
    git.commit(message, &args)
}
