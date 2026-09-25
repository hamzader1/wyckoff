//! Building the request we send to a model.
//!
//! `PROMPT_REV` participates in cache keys, so changing the wording invalidates
//! derived artifacts instead of silently mixing old and new outputs.

pub mod budget;
pub mod text;

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;

pub use budget::{RenderedDiff, render as render_diff};
pub use text::{system, user};

use crate::branch::BranchHint;
use crate::context::ProjectContext;
use crate::diff::ParsedDiff;
use crate::style::StyleProfile;

/// Bump when the prompt text changes in a way that should invalidate caches.
pub const PROMPT_REV: &str = "1";

pub struct Inputs<'a> {
    pub diff: &'a ParsedDiff,
    pub context: &'a ProjectContext,
    pub style: &'a StyleProfile,
    pub branch: &'a BranchHint,
    pub max_subject_len: usize,
    pub include_body: bool,
    pub user_intent: Option<&'a str>,
    pub extra_instructions: Option<&'a str>,
    pub rendered_diff: &'a RenderedDiff,
    /// `--style plain|conventional` beats what the history says.
    pub style_override: Option<crate::style::Style>,
}

#[derive(Debug, Clone)]
pub struct Options<'a> {
    pub max_subject_len: usize,
    pub include_body: bool,
    pub user_intent: Option<&'a str>,
    pub extra_instructions: Option<&'a str>,
    pub max_output_tokens: u32,
    pub style_override: Option<crate::style::Style>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            max_subject_len: crate::msg::DEFAULT_MAX_SUBJECT,
            include_body: false,
            user_intent: None,
            extra_instructions: None,
            max_output_tokens: 700,
            style_override: None,
        }
    }
}

/// Everything a provider request needs.
#[derive(Debug, Clone)]
pub struct Request {
    pub system: String,
    pub user: String,
    pub max_output_tokens: u32,
    pub wants_json: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn build<'a>(
    diff: &'a ParsedDiff,
    context: &'a ProjectContext,
    style: &'a StyleProfile,
    branch: &'a BranchHint,
    options: Options<'a>,
    rendered_diff: &'a RenderedDiff,
) -> Request {
    let inputs = Inputs {
        diff,
        context,
        style,
        branch,
        max_subject_len: options.max_subject_len,
        include_body: options.include_body,
        user_intent: options.user_intent,
        extra_instructions: options.extra_instructions,
        rendered_diff,
        style_override: options.style_override,
    };
    Request {
        system: system(&inputs),
        user: user(&inputs),
        max_output_tokens: options.max_output_tokens,
        wants_json: true,
    }
}

/// Second (and final) attempt: same request, plus what was wrong.
pub fn repair(request: &Request, bad_output: &str, problems: &[String]) -> Request {
    let mut user = String::new();
    user.push_str(&request.user);
    user.push_str("\n\n## Your previous answer was rejected\n");
    user.push_str(bad_output.trim());
    user.push_str("\n\nProblems with it:\n");
    for problem in problems {
        user.push_str(&format!("- {problem}\n"));
    }
    user.push_str(
        "\nWrite the message again, fixing exactly those problems and changing nothing else.\n\
         Reply with the same JSON object.\n",
    );
    Request {
        user,
        ..request.clone()
    }
}
