//! The commit message contract.

pub mod clean;
pub mod parse;

pub use clean::clean_subject;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::util;

pub const DEFAULT_MAX_SUBJECT: usize = 120;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommitMsg {
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub body: Vec<String>,
    #[serde(default)]
    pub breaking: bool,
    #[serde(default)]
    pub footers: Vec<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default, alias = "notes_to_user", alias = "notes")]
    pub notes: String,
}

impl CommitMsg {
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            ..Default::default()
        }
    }

    /// Read whatever the provider sent back. Models and gateways wrap JSON in
    /// fences, add prose around it, or answer in plain text, so this is
    /// deliberately forgiving; the validator is what enforces quality.
    pub fn from_provider_output(text: &str) -> Result<Self> {
        if let Some(value) = parse::extract_json(text) {
            return Ok(parse::from_value(&value));
        }
        if let Some(subject) = parse::plain_subject(text) {
            return Ok(Self::new(subject));
        }
        Err(crate::error::Error::ModelOutput(parse::truncate_for_error(
            text,
        )))
    }

    /// Ready-to-use commit message text.
    pub fn render(&self, include_body: bool) -> String {
        let mut out = self.subject.clone();

        if include_body && !self.body.is_empty() {
            out.push_str("\n\n");
            for line in &self.body {
                let cleaned = line
                    .trim()
                    .trim_start_matches(['-', '*', '•', ' '])
                    .trim_end();
                if cleaned.is_empty() {
                    continue;
                }
                out.push_str("- ");
                out.push_str(cleaned);
                out.push('\n');
            }
            while out.ends_with('\n') {
                out.pop();
            }
        }

        if !self.footers.is_empty() {
            out.push_str("\n\n");
            for footer in &self.footers {
                out.push_str(footer.trim());
                out.push('\n');
            }
            while out.ends_with('\n') {
                out.pop();
            }
        }

        out
    }

    /// Hard cap, used only when validation and repair could not fix an
    /// over-long subject. Losing a word beats producing a 200-character header
    /// that git will wrap in every log view forever.
    pub fn enforce_max_len(&mut self, max: usize) {
        self.subject = util::truncate(&clean_subject(&self.subject), max);
    }

    pub fn is_empty(&self) -> bool {
        self.subject.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_body_only_when_asked() {
        let msg = CommitMsg {
            subject: "Add cache warming".into(),
            body: vec!["- keeps the first run fast".into()],
            ..Default::default()
        };
        assert_eq!(msg.render(false), "Add cache warming");
        assert_eq!(
            msg.render(true),
            "Add cache warming\n\n- keeps the first run fast"
        );
    }

    #[test]
    fn renders_footers() {
        let msg = CommitMsg {
            subject: "Add cache warming".into(),
            footers: vec!["Refs: PROJ-4512".into()],
            ..Default::default()
        };
        assert_eq!(msg.render(false), "Add cache warming\n\nRefs: PROJ-4512");
    }

    #[test]
    fn enforce_max_len_keeps_it_readable() {
        let mut msg = CommitMsg::new(
            "Refactor the btree page access to use failable cell counts and unified types",
        );
        msg.enforce_max_len(40);
        assert!(msg.subject.chars().count() <= 40, "{}", msg.subject);
        assert!(!msg.subject.ends_with(" and"), "{}", msg.subject);
    }
}
