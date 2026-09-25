//! Fitting a diff into a token budget without losing meaning.
//!
//! The naive approach (paste the whole diff) breaks exactly when it matters: a
//! branch that bumps dependencies and regenerates a bundle produces a 40k-token
//! diff whose entire signal is "bump deps". So we spend the budget in priority
//! order, replace noise with a one-line effect, and — crucially — always state
//! what was left out, so the model never claims completeness it does not have.

use std::fmt::Write as _;

use crate::diff::{FileDiff, Kind, ParsedDiff, classify};
use crate::tokens;

#[cfg(test)]
#[path = "budget_tests.rs"]
mod budget_tests;

#[derive(Debug, Clone)]
pub struct RenderedDiff {
    pub text: String,
    pub tokens: usize,
    pub included_files: usize,
    pub summarized_files: usize,
    pub omitted_files: Vec<String>,
    pub truncated: bool,
}

/// Rank for spending: source first, noise last.
fn rank(file: &FileDiff, skip: &[String]) -> u8 {
    if skip
        .iter()
        .any(|pattern| matches_pattern(&file.path, pattern))
    {
        return 100;
    }
    match classify(&file.path) {
        Kind::Source => 0,
        Kind::Test => 1,
        Kind::Docs => 2,
        Kind::Config => 3,
        Kind::Ci => 4,
        Kind::Other => 5,
        Kind::Asset => 6,
        Kind::Lockfile => 7,
        Kind::Generated => 8,
        Kind::Vendored => 9,
    }
}

pub fn render(diff: &ParsedDiff, max_tokens: usize, skip: &[String]) -> RenderedDiff {
    let mut ordered: Vec<&FileDiff> = diff.files.iter().collect();
    // Stable sort keeps the diff's own order inside a rank.
    ordered.sort_by_key(|file| rank(file, skip));

    let mut text = String::new();
    let _ = writeln!(text, "{} — {}", diff.stat_line(), diff.kind_summary());
    let mut remaining = max_tokens.saturating_sub(tokens::estimate(&text));

    let mut included = 0;
    let mut summarized = 0;
    let mut omitted: Vec<String> = Vec::new();
    let mut truncated = false;

    for file in &ordered {
        let skipped = skip
            .iter()
            .any(|pattern| matches_pattern(&file.path, pattern));

        if classify(&file.path).body_is_noise() || skipped || file.binary || file.hunks.is_empty() {
            let line = format!("- {}\n", file.summary_line());
            let cost = tokens::estimate(&line);
            if cost <= remaining {
                text.push_str(&line);
                remaining -= cost;
                summarized += 1;
            } else {
                omitted.push(file.path.clone());
            }
            continue;
        }

        let full_cost = tokens::estimate_block(&file.raw);
        if full_cost <= remaining {
            text.push_str(&file.raw);
            if !file.raw.ends_with('\n') {
                text.push('\n');
            }
            remaining = remaining.saturating_sub(full_cost);
            included += 1;
            continue;
        }

        // Too big in full: keep the header and the first hunks, and say so.
        let condensed = condense(file, remaining);
        let cost = tokens::estimate_block(&condensed);
        if cost > 0 && cost <= remaining {
            text.push_str(&condensed);
            remaining = remaining.saturating_sub(cost);
            included += 1;
        } else {
            omitted.push(file.path.clone());
        }
        truncated = true;
    }

    if !omitted.is_empty() {
        let _ = writeln!(
            text,
            "\n[{} not shown in full because of the size limit: {}]",
            crate::util::plural(omitted.len(), "file was", "files were"),
            omitted
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    RenderedDiff {
        tokens: tokens::estimate_block(&text),
        text,
        included_files: included,
        summarized_files: summarized,
        omitted_files: omitted,
        truncated,
    }
}

/// Header plus as many leading hunks (trimmed) as the budget allows.
fn condense(file: &FileDiff, budget: usize) -> String {
    const MAX_HUNKS: usize = 3;
    const MAX_LINES_PER_HUNK: usize = 30;

    let mut out = String::new();
    let _ = writeln!(out, "{}", file.raw.lines().next().unwrap_or("diff --git"));
    let _ = writeln!(
        out,
        "{} (showing part of it: {} hunks)",
        file.summary_line(),
        file.hunks.len()
    );

    for hunk in file.hunks.iter().take(MAX_HUNKS) {
        let _ = writeln!(out, "{}", hunk.header);
        for line in hunk.lines.iter().take(MAX_LINES_PER_HUNK) {
            let _ = writeln!(out, "{line}");
        }
        if hunk.lines.len() > MAX_LINES_PER_HUNK {
            let _ = writeln!(
                out,
                "… ({} more lines in this hunk)",
                hunk.lines.len() - MAX_LINES_PER_HUNK
            );
        }
        if tokens::estimate_block(&out) > budget {
            break;
        }
    }
    if file.hunks.len() > MAX_HUNKS {
        let _ = writeln!(out, "… ({} more hunks)", file.hunks.len() - MAX_HUNKS);
    }
    out
}

/// Glob-lite: `*` matches anything, everything else is literal.
pub fn matches_pattern(path: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    if !pattern.contains('*') {
        return path.contains(pattern);
    }
    let mut rest = path;
    for (index, part) in pattern.split('*').enumerate() {
        if part.is_empty() {
            continue;
        }
        // The first segment must start a path component, so `gen/*` matches
        // `src/gen/x.rs` but `vendor/*` does not match `web/src/a.ts`.
        let found = if index == 0 {
            match_at_boundary(rest, part)
        } else {
            rest.find(part)
        };
        match found {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    true
}

fn match_at_boundary(haystack: &str, needle: &str) -> Option<usize> {
    if haystack.starts_with(needle) {
        return Some(0);
    }
    haystack.find(&format!("/{needle}")).map(|at| at + 1)
}
