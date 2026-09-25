//! The two prompts we send, and nothing else.
//!
//! Deliberately small. Everything expensive (history, project facts) was already
//! compressed by the layers above. Every rule here exists because of a specific
//! failure mode: prefixes sneaking in, invented identifiers, guessing at intent.

use super::Inputs;
use crate::diff::ParsedDiff;
use crate::style::Style;

pub fn system(inputs: &Inputs<'_>) -> String {
    let mut out = String::new();

    out.push_str(
        "You write git commit messages for the developer who made the change.\n\
         You are given a description of the project, the branch name, measurements of how\n\
         this repository writes commit messages, a few of its recent messages, and the\n\
         staged diff.\n\n\
         Reply with ONE JSON object and nothing else:\n\
         {\n\
         \x20 \"subject\": string,\n\
         \x20 \"body\": [string],\n\
         \x20 \"breaking\": boolean,\n\
         \x20 \"footers\": [string],\n\
         \x20 \"confidence\": number between 0 and 1,\n\
         \x20 \"notes_to_user\": string\n\
         }\n\n",
    );

    out.push_str(&format!(
        "Subject rules:\n\
         - one line, imperative mood, first letter capitalised, no trailing period\n\
         - at most {} characters; aim for 60-110 and use the space you need, no more\n\
         - NO type or scope prefixes. Never \"feat:\", \"fix(parser):\", \"chore:\",\n\
         \x20 \"refactor:\" or anything of that shape. It is a plain sentence a person\n\
         \x20 would type.\n\
         - name the real things that changed (module, type, function), spelled exactly as\n\
         \x20 they appear in the diff\n\
         - say what changed; add why when the diff and the project notes make it clear\n\
         - match the measured style below: same length, same mood, same language\n\
         - no filler (\"This commit\", \"Various improvements\", \"Update code\",\n\
         \x20 \"Minor fixes\"), no marketing, no emoji unless the author uses them\n",
        inputs.max_subject_len
    ));

    out.push_str(
        "\nRules for honesty:\n\
         - the diff is the only source of truth about what changed\n\
         - the branch name is a weak hint about intent; if it disagrees with the diff,\n\
         \x20 follow the diff and lower your confidence\n\
         - never mention a file, type, function, dependency or behaviour that is not in the diff\n\
         - if you cannot tell why the change was made, do not invent a reason: describe what\n\
         \x20 changed and set confidence low\n\n",
    );

    let decision = inputs
        .style_override
        .unwrap_or_else(|| inputs.style.decision());
    out.push_str(match (inputs.include_body, decision) {
        (false, Style::Plain) => {
            "body: always []. This project wants single-line messages.\n\
             footers: [] unless there is a ticket id worth recording.\n"
        }
        (false, Style::Conventional) => {
            "body: always []. Single-line messages only.\n\
             footers: [] unless there is a ticket id worth recording.\n\
             This repository *does* use `type(scope):` prefixes, so start the subject with\n\
             one, using the types and scopes seen in its history.\n"
        }
        (true, _) => {
            "body: short bullets giving the why, only when one line cannot carry it\n\
             (unrelated areas, a migration step). At most 3, each starting with a verb, no\n\
             trailing periods, no \"this commit\" preamble.\n\
             footers: [] unless there is a ticket id worth recording.\n"
        }
    });

    out.push_str("notes_to_user: anything you could not tell from the diff, or \"\".\n");

    if let Some(extra) = inputs.extra_instructions
        && !extra.trim().is_empty()
    {
        out.push_str("\nExtra instructions from this project's config:\n");
        out.push_str(extra.trim());
        out.push('\n');
    }

    out
}

pub fn user(inputs: &Inputs<'_>) -> String {
    let mut out = String::new();

    if !inputs.context.is_empty() {
        out.push_str("## Project\n");
        out.push_str(&inputs.context.render());
        out.push_str("\n\n");
    }

    out.push_str("## Branch\n");
    out.push_str(&inputs.branch.describe());
    out.push_str("\n(weak hint only — the diff decides what this change is)\n\n");

    out.push_str("## How this repo writes commit messages\n");
    out.push_str(&inputs.style.guide());
    if !inputs.style.exemplars.is_empty() {
        out.push_str("### Recent messages from this repo (imitate the voice, not the content)\n");
        for exemplar in &inputs.style.exemplars {
            out.push_str(&format!("- {}\n", exemplar.subject));
        }
    }
    out.push('\n');

    if let Some(intent) = inputs.user_intent
        && !intent.trim().is_empty()
    {
        out.push_str("## The author's own words about this change\n");
        out.push_str(intent.trim());
        out.push_str("\n(prefer this over anything you infer)\n\n");
    }

    out.push_str("## Staged change\n");
    out.push_str(&inputs.rendered_diff.text);

    // Hand the model the exact vocabulary of the change: names taken from the
    // added and removed lines. Deterministic, cheap, and it is the difference
    // between "improve the page layer" and "add `replace_cell` to page".
    let hints = symbol_hints(inputs.diff);
    if !hints.is_empty() {
        out.push_str("\n### Definitions this diff introduces or removes\n");
        out.push_str(&hints);
    }

    out
}

/// `+ `replace_cell` (function) in src/btree/page.rs`
fn symbol_hints(diff: &ParsedDiff) -> String {
    use crate::diff::Kind;
    use crate::symbols;

    let mut out = String::new();
    let mut count = 0;
    for file in &diff.files {
        if !matches!(crate::diff::classify(&file.path), Kind::Source | Kind::Test) {
            continue;
        }
        for (marker, found) in [
            ('+', symbols::added_symbols(file)),
            ('-', symbols::removed_symbols(file)),
        ] {
            for symbol in found {
                if count >= 12 {
                    return out;
                }
                out.push_str(&format!(
                    "{marker} `{}` ({}) in {}\n",
                    symbol.name, symbol.kind, file.path
                ));
                count += 1;
            }
        }
    }
    out
}
