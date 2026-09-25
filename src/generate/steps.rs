//! Individual pipeline steps, each testable on its own.

use crate::branch::BranchHint;
use crate::cache::Cache;
use crate::context::ProjectContext;
use crate::diff::ParsedDiff;
use crate::error::Result;
use crate::git::Git;
use crate::prompt::{self, RenderedDiff, Request};
use crate::style::{Style, StyleProfile};
use crate::validate;

use super::Options;

pub const EXPLAIN_SYSTEM: &str = "\
You are a senior engineer explaining a change to a teammate who has five minutes.\n\
Write 2-4 short sentences of plain prose. Say what changed and why it matters.\n\
No bullet lists, no headings, no marketing, no \"this commit\". Do not invent\n\
anything that is not in the diff: if the reason is unclear, say what you can see\n\
and stop. Reply with the prose only.";

/// L1: project facts. Cached against a cheap stat-based signature so editing
/// `.context` is picked up immediately.
pub fn context(
    git: &Git,
    cache: &Cache,
    staged_paths: &[String],
    options: &Options,
    index_fingerprint: &str,
) -> Result<ProjectContext> {
    let signature = crate::context::signature::of(
        git.root(),
        staged_paths,
        options.context_file.as_deref(),
        index_fingerprint,
    );
    let context_options = crate::context::Options {
        max_chars_per_file: options.context_max_chars,
        extra_context_file: options.context_file.clone(),
    };

    cache.get_or_compute("context", &signature, || {
        ProjectContext::collect(git, staged_paths, &context_options)
    })
}

/// L2: the style profile. Depends on the log window and on which paths are
/// staged (exemplar selection), so both go in the key.
pub fn style(
    git: &Git,
    cache: &Cache,
    staged_paths: &[String],
    options: &Options,
) -> Result<StyleProfile> {
    let head = git.head_sha().unwrap_or_default();
    let material = format!(
        "{head}|{}|{}|{}|{}|{}",
        options.history_window,
        prompt::PROMPT_REV,
        options.language.clone().unwrap_or_default(),
        options.style.clone().unwrap_or_default(),
        staged_paths.join("\u{2}")
    );
    let key = crate::paths::short_hash(&material);

    cache.get_or_compute("style", &key, || {
        let records = git.recent_commits(options.history_window)?;
        Ok(StyleProfile::build(
            &records,
            staged_paths,
            options.exemplars,
        ))
    })
}

pub fn request(
    diff: &ParsedDiff,
    context: &ProjectContext,
    style: &StyleProfile,
    branch: &BranchHint,
    options: &Options,
    rendered: &RenderedDiff,
) -> Request {
    let mut request = prompt::build(
        diff,
        context,
        style,
        branch,
        prompt::Options {
            max_subject_len: options.max_subject_len,
            include_body: options.include_body,
            user_intent: options.intent.as_deref(),
            extra_instructions: options.prompt_extra.as_deref(),
            max_output_tokens: 700,
            style_override: options.forced_style(),
        },
        rendered,
    );
    if options.mode == super::Mode::Explain {
        request.system = EXPLAIN_SYSTEM.to_string();
        request.wants_json = false;
    }
    request
}

/// Everything the validator needs to judge an answer.
pub fn validation_context<'a>(
    diff: &'a ParsedDiff,
    options: &Options,
    style: &StyleProfile,
) -> validate::Context<'a> {
    let allow_prefix = match options.forced_style() {
        Some(Style::Conventional) => true,
        Some(Style::Plain) => false,
        None => style.decision() == Style::Conventional,
    };
    validate::Context {
        diff,
        max_subject_len: options.max_subject_len,
        allow_prefix,
        language: style.language,
    }
}
