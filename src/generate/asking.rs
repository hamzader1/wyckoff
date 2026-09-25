//! Asking the model, and holding its answer to the rules.
//!
//! A provider we cannot reach, or output we cannot read a message out of, is a
//! hard error. We do not invent a line to fill the gap: a wrong message quietly
//! committed is worse than a clear failure you can fix in ten seconds.

use crate::diff::ParsedDiff;
use crate::error::Result;
use crate::llm::Provider;
use crate::msg::CommitMsg;
use crate::prompt::{self, Request};
use crate::style::StyleProfile;
use crate::validate;

use super::Options;
use super::steps::validation_context;

pub struct ModelOutcome {
    pub msg: CommitMsg,
    pub issues: Vec<validate::Issue>,
    pub usage: String,
    /// The answer broke a rule and was rewritten once.
    pub repaired: bool,
    /// The answer was cut off by the output limit.
    pub truncated: bool,
}

pub fn ask_model(
    provider: &dyn Provider,
    request: &Request,
    diff: &ParsedDiff,
    options: &Options,
    style: &StyleProfile,
) -> Result<ModelOutcome> {
    let response = provider.complete(request)?;
    let usage = response.summary();
    let truncated = response.truncated;
    let mut msg = CommitMsg::from_provider_output(&response.text)?;

    let context = validation_context(diff, options, style);
    let mut issues = validate::validate(&msg, &context);
    let mut blocking = validate::blocking(&issues);
    let mut repaired = false;

    // One targeted repair attempt. If the repair request itself fails we keep
    // the model's own first answer rather than throwing it away: its blocking
    // issues are still reported, and `wyckoff commit` refuses to commit them.
    if !blocking.is_empty() && options.repair {
        let fixed = prompt::repair(request, &response.text, &blocking);
        if let Ok(second) = provider.complete(&fixed)
            && let Ok(candidate) = CommitMsg::from_provider_output(&second.text)
        {
            let candidate_issues = validate::validate(&candidate, &context);
            if validate::blocking(&candidate_issues).len() < blocking.len() {
                msg = candidate;
                issues = candidate_issues;
                blocking = validate::blocking(&issues);
                repaired = true;
            }
        }
    }

    // The one problem we can always fix locally: an over-long subject. Losing a
    // word beats a 200-character header in every log view forever.
    if blocking
        .iter()
        .any(|message| message.contains("characters"))
    {
        msg.enforce_max_len(options.max_subject_len);
        issues = validate::validate(&msg, &context);
    }

    Ok(ModelOutcome {
        msg,
        issues,
        usage,
        repaired,
        truncated,
    })
}
