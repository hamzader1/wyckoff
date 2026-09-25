//! Defensive cleaning of generated subjects.
//!
//! The tool exists to produce untemplated, human-sounding messages, so a model
//! that answers with `feat(parser): ...` anyway must never reach the user.
//! Everything a model produces passes through here.

use crate::util;

pub const DEFAULT_MAX_SUBJECT: usize = 120;

/// Words a model might use as a "type" prefix. If a subject starts with one of
/// these followed by a colon, the prefix is stripped.
const CONVENTIONAL_HEADS: &[&str] = &[
    "feat", "feature", "fix", "bugfix", "hotfix", "chore", "docs", "doc", "style", "refactor",
    "refact", "perf", "test", "tests", "build", "ci", "revert", "wip", "breaking", "deps",
];

/// Strip prefixes, bullets, quotes, trailing periods; fix capitalisation.
/// Idempotent.
pub fn clean_subject(raw: &str) -> String {
    let mut text = raw.trim().to_string();

    // Multi-line output: the first line is the subject.
    if let Some((first, rest)) = text.split_once('\n')
        && !rest.trim().is_empty()
        && first.trim().len() <= DEFAULT_MAX_SUBJECT
    {
        text = first.trim().to_string();
    }

    text = text
        .trim_matches(|c: char| {
            c == '`' || c == '"' || c == '\'' || c == '*' || c == '#' || c.is_whitespace()
        })
        .to_string();
    text = text
        .trim_start_matches(['-', '•', '>'])
        .trim_start()
        .to_string();

    if let Some(rest) = text.strip_prefix("Subject:") {
        text = rest.trim().to_string();
    }

    text = strip_type_prefix(&text);

    // Collapse whitespace: models love double spaces.
    text = text.split_whitespace().collect::<Vec<_>>().join(" ");

    text = text
        .trim_end_matches(['.', ':', ';', ','])
        .trim_end()
        .to_string();

    // Drop a dangling conjunction left behind by truncation.
    if text.ends_with(" and") {
        text.truncate(text.len() - 4);
    }

    util::sentence_case(&text)
}

/// `feat(parser): add X` -> `add X`.
fn strip_type_prefix(text: &str) -> String {
    let Some((head, rest)) = text.split_once(':') else {
        return text.to_string();
    };
    let head = head.trim().to_lowercase();
    let bare = match head.split_once('(') {
        Some((kind, scope)) => {
            if !scope.trim_end().ends_with(')') {
                return text.to_string();
            }
            kind.trim().to_string()
        }
        None => head.clone(),
    };
    let bare = bare.trim_end_matches('!').to_string();
    if CONVENTIONAL_HEADS.contains(&bare.as_str()) {
        let rest = rest.trim();
        if !rest.is_empty() {
            return rest.to_string();
        }
    }
    text.to_string()
}

/// Does the subject start with a template prefix (used by the validator to
/// flag prefix-y input before we silently fix it)?
pub fn has_type_prefix(text: &str) -> bool {
    let trimmed = text.trim();
    let Some((head, _)) = trimmed.split_once(':') else {
        return false;
    };
    let head = head.trim().to_lowercase();
    let bare = match head.split_once('(') {
        Some((kind, _)) => kind.trim().to_string(),
        None => head.clone(),
    };
    CONVENTIONAL_HEADS.contains(&bare.trim_end_matches('!').to_string().as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_every_flavour_of_prefix() {
        assert_eq!(clean_subject("feat: add cache"), "Add cache");
        assert_eq!(clean_subject("fix(parser): handle rename"), "Handle rename");
        assert_eq!(clean_subject("chore!: bump deps"), "Bump deps");
        assert_eq!(clean_subject("refactor:  tidy up  "), "Tidy up");
        // A prefix with nothing after it leaves the bare word.
        assert_eq!(clean_subject("feat: "), "Feat");
        // not a known type -> left alone
        assert_eq!(
            clean_subject("Parser: handle rename"),
            "Parser: handle rename"
        );
    }

    #[test]
    fn strips_noise_around_a_subject() {
        assert_eq!(clean_subject("- Add a thing."), "Add a thing");
        assert_eq!(clean_subject("`Add a thing`"), "Add a thing");
        assert_eq!(clean_subject("Subject: Add a thing"), "Add a thing");
        assert_eq!(clean_subject("\"Add a thing\""), "Add a thing");
        assert_eq!(clean_subject("add a thing and"), "Add a thing");
        assert_eq!(clean_subject("Add    a     thing"), "Add a thing");
        assert_eq!(clean_subject("Add a thing:\n\n- because"), "Add a thing");
    }

    #[test]
    fn is_idempotent() {
        let once = clean_subject("feat: Add a thing.");
        assert_eq!(clean_subject(&once), once);
        assert_eq!(once, "Add a thing");
    }

    #[test]
    fn detects_template_prefixes() {
        assert!(has_type_prefix("feat: x"));
        assert!(has_type_prefix("fix(scope): x"));
        assert!(!has_type_prefix("Refactor the parser"));
        assert!(!has_type_prefix("Parser: handle rename"));
    }
}
