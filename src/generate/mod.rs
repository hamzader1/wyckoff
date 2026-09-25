//! The pipeline: gather, ask, check, deliver.
//!
//! Order matters. Everything cheap and local happens first (diff, secrets,
//! context, style), so a misconfigured provider or a staged `.env` fails before
//! a single token is spent.

pub mod asking;
pub mod options;
pub mod steps;

pub use options::Options;

use crate::cache::Cache;
use crate::config::Config;
use crate::diff::ParsedDiff;
use crate::error::{Error, Result};
use crate::git::{DiffSource, Git};
use crate::msg::CommitMsg;
use crate::output::Meta;
use crate::prompt::{self, Request};
use crate::validate::Issue;

use asking::ModelOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// One-line commit message.
    Message,
    /// A short prose explanation of the change.
    Explain,
}

pub struct Outcome {
    pub msg: CommitMsg,
    pub text: String,
    pub meta: Meta,
    pub issues: Vec<Issue>,
    /// Present for `--dry-run`: the request that would have been sent.
    pub request: Option<Request>,
    /// The first answer broke a rule and was rewritten once.
    pub repaired: bool,
    /// The model ran out of output room.
    pub truncated: bool,
}

fn nothing_to_do(source: DiffSource) -> Error {
    match source {
        DiffSource::Staged => Error::NothingStaged,
        DiffSource::Unstaged => Error::msg(
            "no unstaged changes (try `wyckoff --all`, or stage something with `git add`)",
        ),
        DiffSource::Head => Error::msg("nothing has changed since HEAD"),
    }
}

pub fn run(git: &Git, config: &Config, options: &Options) -> Result<Outcome> {
    let diff_text = git.diff_for(
        options.diff_source,
        options.diff_context_lines,
        &options.paths,
    )?;
    let diff = ParsedDiff::parse(&diff_text);
    if diff.is_empty() {
        return Err(nothing_to_do(options.diff_source));
    }

    let findings = crate::secrets::scan(&diff);
    if !findings.is_empty() && !options.allow_secrets {
        let list = findings
            .iter()
            .map(|finding| finding.describe())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(Error::Secrets(list));
    }

    let cache = Cache::new(&git.root().display().to_string(), options.cache);
    let staged_paths = diff.paths();
    let index_fingerprint = git.index_fingerprint().unwrap_or_default();

    let context = steps::context(git, &cache, &staged_paths, options, &index_fingerprint)?;
    let style = steps::style(git, &cache, &staged_paths, options)?;
    let branch = crate::branch::parse(&git.branch()?);

    let mut skip: Vec<String> = config.skip.clone();
    skip.extend(options.exclude.iter().cloned());
    let rendered = prompt::render_diff(&diff, options.max_diff_tokens, &skip);

    let mut meta = Meta {
        provider: options.provider.clone(),
        model: options.model.clone().unwrap_or_default(),
        source: options.diff_source.describe().to_string(),
        files: diff.files.len(),
        insertions: diff.insertions(),
        deletions: diff.deletions(),
        prompt_tokens: rendered.tokens,
        style: style.summary_line(),
        cache_hits: cache.hits(),
        cache_misses: cache.misses(),
        exemplars: style.exemplars.len(),
        diff_truncated: rendered.truncated,
        diff_included: rendered.included_files,
        diff_omitted: rendered.omitted_files.len(),
        diff_summarized: rendered.summarized_files,
        ..Default::default()
    };

    let request = steps::request(&diff, &context, &style, &branch, options, &rendered);

    if options.dry_run {
        return Ok(Outcome {
            msg: CommitMsg::default(),
            text: String::new(),
            meta,
            issues: Vec::new(),
            request: Some(request),
            repaired: false,
            truncated: false,
        });
    }

    let resolved = config.resolved(&options.provider, options.model.as_deref())?;
    meta.provider = resolved.name.clone();
    meta.model = resolved.model.clone();
    let provider = crate::llm::build(&resolved);
    meta.model = provider.model().to_string();
    meta.provider = provider.name().to_string();

    let ModelOutcome {
        msg,
        issues,
        usage,
        repaired,
        truncated,
    } = asking::ask_model(provider.as_ref(), &request, &diff, options, &style)?;

    meta.usage = usage;

    Ok(Outcome {
        text: msg.render(options.include_body),
        msg,
        meta,
        issues,
        request: None,
        repaired,
        truncated,
    })
}
