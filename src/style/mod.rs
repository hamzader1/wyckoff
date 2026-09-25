//! The commit-style engine.
//!
//! Two jobs, both deterministic and both cheap:
//!
//! 1. Measure how this repository (really: this author) writes commit subjects —
//!    length, capitalisation, mood, whether prefixes are used *at all*, language.
//! 2. Pick a few *examples* for the prompt. Not fifty messages: three, chosen by
//!    which past commits touched the same paths as the staged diff. That is the
//!    trick — the model gets the author's voice in ~120 tokens instead of a wall
//!    of history that dilutes its attention.

pub mod exemplars;
pub mod guide;
pub mod metrics;

#[cfg(test)]
#[path = "style_tests.rs"]
mod style_tests;

pub use metrics::Language;

use crate::git::CommitRecord;
use serde::{Deserialize, Serialize};

/// Whether to write a bare subject, or imitate a templated convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// One plain sentence. What this tool is for.
    Plain,
    /// The repo overwhelmingly uses `type(scope): ...`, so match it.
    Conventional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Exemplar {
    pub subject: String,
    pub overlap: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleProfile {
    pub sample: usize,
    /// Newest commit in the window; useful for debugging a stale cache.
    pub newest_sha: String,
    pub conventional_ratio: f32,
    pub avg_len: usize,
    pub max_len: usize,
    pub sentence_case_ratio: f32,
    pub trailing_period_ratio: f32,
    pub non_imperative_ratio: f32,
    pub ticket_ratio: f32,
    pub emoji_count: usize,
    pub language: Language,
    pub top_verbs: Vec<(String, usize)>,
    pub exemplars: Vec<Exemplar>,
}

impl StyleProfile {
    pub fn build(records: &[CommitRecord], staged_paths: &[String], exemplar_count: usize) -> Self {
        let subjects: Vec<String> = records
            .iter()
            .map(|r| r.subject.trim().to_string())
            .collect();

        let usable: Vec<&CommitRecord> = records
            .iter()
            .filter(|r| !r.subject.trim().is_empty() && r.subject.chars().count() <= 200)
            .collect();
        let divisor = usable.len().max(1) as f32;

        let ratio = |predicate: fn(&str) -> bool| -> f32 {
            usable.iter().filter(|r| predicate(&r.subject)).count() as f32 / divisor
        };

        let lengths: Vec<usize> = usable.iter().map(|r| r.subject.chars().count()).collect();
        let avg_len = if lengths.is_empty() {
            0
        } else {
            lengths.iter().sum::<usize>() / lengths.len()
        };

        let mut verbs: std::collections::BTreeMap<String, usize> = Default::default();
        for record in &usable {
            if let Some(word) = metrics::opening_word(&record.subject) {
                *verbs.entry(word).or_insert(0) += 1;
            }
        }
        let mut top_verbs: Vec<(String, usize)> = verbs.into_iter().collect();
        top_verbs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        top_verbs.truncate(6);

        Self {
            sample: records.len(),
            newest_sha: records
                .first()
                .map(|record| record.sha.chars().take(8).collect())
                .unwrap_or_default(),
            conventional_ratio: ratio(metrics::is_conventional),
            avg_len,
            max_len: lengths.iter().copied().max().unwrap_or(0),
            sentence_case_ratio: ratio(metrics::starts_sentence_case),
            trailing_period_ratio: ratio(metrics::has_trailing_period),
            non_imperative_ratio: ratio(metrics::looks_non_imperative),
            ticket_ratio: ratio(metrics::has_ticket),
            emoji_count: usable
                .iter()
                .filter(|r| metrics::has_emoji(&r.subject))
                .count(),
            language: metrics::detect_language(&subjects),
            top_verbs,
            exemplars: exemplars::pick(records, staged_paths, exemplar_count),
        }
    }

    /// Prefixes only when the repo is *clearly* templated. A couple of stray
    /// `fix:` commits from three years ago are not a convention.
    pub fn decision(&self) -> Style {
        if self.sample >= 5 && self.conventional_ratio >= 0.6 {
            Style::Conventional
        } else {
            Style::Plain
        }
    }

    pub fn style_label(&self) -> &'static str {
        match self.decision() {
            Style::Plain => "plain",
            Style::Conventional => "conventional",
        }
    }
}
