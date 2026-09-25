//! The rules, enforced in code rather than hoped for in a prompt.
//!
//! A model that follows every instruction on the first try is not a thing that
//! exists. So the contract is checked here: errors go back for one repair
//! attempt, warnings go to the human, and none of it costs a token.

mod rules;

#[cfg(test)]
#[path = "../validate_tests.rs"]
mod validate_tests;

pub use rules::{blocking, warnings};

use crate::diff::ParsedDiff;
use crate::msg::CommitMsg;
use crate::msg::clean::has_type_prefix;
use crate::style::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub level: Level,
    pub message: String,
}

impl Issue {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            message: message.into(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            level: Level::Warning,
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.level == Level::Error
    }
}

pub struct Context<'a> {
    pub diff: &'a ParsedDiff,
    pub max_subject_len: usize,
    /// True when the repository really does use `type:` prefixes.
    pub allow_prefix: bool,
    pub language: Language,
}

pub fn validate(msg: &CommitMsg, ctx: &Context<'_>) -> Vec<Issue> {
    let mut issues = Vec::new();
    let subject = msg.subject.trim();

    if subject.is_empty() {
        issues.push(Issue::error("the subject is empty"));
        return issues;
    }

    let length = subject.chars().count();
    if length > ctx.max_subject_len {
        issues.push(Issue::error(format!(
            "the subject is {length} characters, the limit is {}",
            ctx.max_subject_len
        )));
    }

    // Belt and braces: `clean_subject` already strips these, so reaching here
    // means the prefix came back in an unexpected shape.
    if !ctx.allow_prefix && has_type_prefix(subject) {
        issues.push(Issue::error(
            "the subject starts with a type prefix (feat:, fix:, chore: ...); write a plain sentence instead",
        ));
    }

    if subject.ends_with('.') {
        issues.push(Issue::warning("the subject ends with a period"));
    }

    if !has_type_prefix(subject) && subject.chars().next().is_some_and(|c| c.is_lowercase()) {
        issues.push(Issue::error(format!(
            "the subject starts with a lowercase letter ({:?})",
            subject.chars().take(24).collect::<String>()
        )));
    }

    let lower = subject.to_lowercase();
    if let Some(phrase) = rules::FILLER.iter().find(|phrase| lower.contains(**phrase)) {
        issues.push(Issue::warning(format!(
            "\"{phrase}\" says nothing about the change; name what actually changed"
        )));
    }

    if crate::style::metrics::looks_non_imperative(subject) {
        issues.push(Issue::warning(
            "written in the past tense or as a gerund; use the imperative (Add, not Added)",
        ));
    }

    // Invented file paths are the clearest sign of a hallucination and the
    // cheapest thing to check exactly.
    let paths = ctx.diff.path_set();
    if let Some(issue) = rules::path_check(subject, &paths) {
        issues.push(issue);
    }

    if let Some(issue) = rules::identifier_check(subject, ctx.diff) {
        issues.push(issue);
    }

    if ctx.language == Language::Arabic && subject.is_ascii() {
        issues.push(Issue::warning(
            "this repository commits in Arabic, but the subject is ASCII",
        ));
    }

    if msg.breaking && msg.body.is_empty() && msg.footers.is_empty() {
        issues.push(Issue::warning(
            "marked as breaking, but the message does not say what breaks",
        ));
    }

    issues
}
