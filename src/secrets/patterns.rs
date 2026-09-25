//! The actual secret detectors.
//!
//! Deliberately regex-free: a handful of prefix checks and one assignment
//! heuristic cover the cases that matter, cost nothing at runtime, and keep the
//! dependency tree at zero for this module.

use super::{PLACEHOLDERS, SENSITIVE_NAMES};

/// Known credential prefixes. `min_len` guards against matching prose that
/// happens to contain the prefix.
const MARKERS: &[(&str, &str, usize)] = &[
    ("ghp_", "a GitHub token", 20),
    ("gho_", "a GitHub token", 20),
    ("ghu_", "a GitHub token", 20),
    ("github_pat_", "a GitHub token", 20),
    ("AKIA", "an AWS access key", 16),
    ("ASIA", "an AWS access key", 16),
    ("xoxb-", "a Slack token", 20),
    ("xoxp-", "a Slack token", 20),
    ("nvapi-", "an NVIDIA API key", 20),
    ("sk-ant-", "an Anthropic API key", 20),
    ("sk-proj-", "an OpenAI API key", 20),
    ("AIza", "a Google API key", 30),
    ("glpat-", "a GitLab token", 20),
    ("hf_", "a Hugging Face token", 20),
];

/// High-confidence key material: no name/value heuristics needed.
pub(super) fn detects_key_material(line: &str) -> Option<&'static str> {
    if line.contains("PRIVATE KEY-----") {
        return Some("a private key block");
    }
    for (marker, kind, min_len) in MARKERS {
        if let Some(rest) = find_after(line, marker)
            && has_token_chars(rest, *min_len)
        {
            return Some(kind);
        }
    }
    // Bare `sk-...` last: it collides with ordinary words more often.
    if let Some(rest) = find_after(line, "sk-")
        && has_token_chars(rest, 24)
    {
        return Some("an API key");
    }
    None
}

/// `api_key = "..."` style assignments whose value looks like a real literal.
pub(super) fn detects_assignment(line: &str) -> Option<&'static str> {
    let (name, value) = split_assignment(line)?;
    let name_lower = name.to_lowercase();
    if !SENSITIVE_NAMES
        .iter()
        .any(|needle| name_lower.contains(needle))
    {
        return None;
    }
    if looks_like_literal(value) {
        Some("a hardcoded credential")
    } else {
        None
    }
}

/// Any `KEY=value` line with a substantial value — used inside `.env` files,
/// where even a plainly named variable should not leave the machine.
pub(super) fn looks_like_assignment(line: &str) -> bool {
    split_assignment(line)
        .map(|(_, value)| looks_like_literal(value))
        .unwrap_or(false)
}

/// The variable name is the last word before `=`, which covers `KEY=`,
/// `export KEY=`, `let key =` and `const key =`.
fn split_assignment(line: &str) -> Option<(&str, &str)> {
    let (before, value) = line.split_once('=')?;
    let name = before.split_whitespace().next_back()?;
    let name = name.trim_matches(|c| c == '"' || c == '\'' || c == ';');
    if name.is_empty() || name.len() > 64 {
        return None;
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return None;
    }
    Some((name, value))
}

/// The value has to look like a literal, not an expression: this is what stops
/// `let password = user_input.to_string();` from being flagged.
fn looks_like_literal(value: &str) -> bool {
    let trimmed = value.trim();
    let quoted = trimmed.len() > 1
        && ((trimmed.starts_with('"') && trimmed.contains('"'))
            || (trimmed.starts_with('\'') && trimmed.contains('\'')));
    let inner = strip_quotes(trimmed);
    if inner.len() < 12 || looks_like_placeholder(&inner) {
        return false;
    }
    if quoted {
        return true;
    }
    inner
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && inner.chars().any(|c| c.is_ascii_digit())
}

fn looks_like_placeholder(value: &str) -> bool {
    let lower = value.to_lowercase();
    PLACEHOLDERS.iter().any(|p| lower.contains(p))
        || value.chars().all(|c| c == '*' || c == 'x' || c == '-')
}

fn strip_quotes(value: &str) -> String {
    value
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .trim_end_matches([';', ','])
        .to_string()
}

fn find_after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    line.find(marker).map(|i| &line[i + marker.len()..])
}

/// Does `rest` start with at least `min_len` token characters, including at
/// least one digit? Random-looking strings almost always contain a digit, which
/// keeps us from flagging identifiers like `ghp_token_name_placeholder`.
fn has_token_chars(rest: &str, min_len: usize) -> bool {
    let token: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    token.len() >= min_len && token.chars().any(|c| c.is_ascii_digit())
}
