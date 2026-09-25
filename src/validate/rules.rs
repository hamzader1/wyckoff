//! The individual checks, kept small enough to reason about separately.

use super::Issue;
use crate::diff::{Kind, ParsedDiff, classify};

/// Phrases that mean the model did not really look at the diff.
pub const FILLER: &[&str] = &[
    "this commit",
    "various improvements",
    "various changes",
    "minor changes",
    "minor fixes",
    "misc changes",
    "some changes",
    "update code",
    "code cleanup",
    "as requested",
    "as needed",
    "general cleanup",
];

pub const CODE_EXTS: &[&str] = &[
    "rs", "py", "ts", "tsx", "js", "jsx", "go", "rb", "java", "kt", "c", "h", "cpp", "hpp", "cs",
    "swift", "php", "sh", "toml", "json", "yaml", "yml", "md", "sql", "css", "html", "lock",
];

/// Messages of the blocking issues, for the repair prompt.
pub fn blocking(issues: &[Issue]) -> Vec<String> {
    issues
        .iter()
        .filter(|issue| issue.is_error())
        .map(|issue| issue.message.clone())
        .collect()
}

pub fn warnings(issues: &[Issue]) -> Vec<String> {
    issues
        .iter()
        .filter(|issue| !issue.is_error())
        .map(|issue| issue.message.clone())
        .collect()
}

pub fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '(' || c == ')')
        .filter(|token| !token.is_empty())
        .map(|token| {
            token
                .trim_matches(|c: char| c == '`' || c == '"' || c == '\'' || c == '.')
                .to_string()
        })
        .collect()
}

pub fn looks_like_path(token: &str) -> bool {
    if token.contains('/') && token.len() > 2 {
        return true;
    }
    match token.rsplit_once('.') {
        Some((stem, ext)) => !stem.is_empty() && CODE_EXTS.contains(&ext.to_lowercase().as_str()),
        None => false,
    }
}

/// Reject paths the diff does not contain. Exact, so it is an error, not a
/// warning: a message naming a file that was never touched is simply wrong.
pub fn path_check(subject: &str, paths: &std::collections::HashSet<String>) -> Option<Issue> {
    for token in tokens(subject) {
        if !looks_like_path(&token) {
            continue;
        }
        if !named_in_diff(&token, paths) {
            return Some(Issue::error(format!(
                "\"{token}\" looks like a file, but no such file is in the diff"
            )));
        }
    }
    None
}

fn named_in_diff(token: &str, paths: &std::collections::HashSet<String>) -> bool {
    let cleaned = token.trim_end_matches(['.', ':']).to_lowercase();
    let base = cleaned.rsplit('/').next().unwrap_or(&cleaned);
    let stem = base.rsplit_once('.').map(|(s, _)| s).unwrap_or(base);
    // A token that names an extension has to match that extension: `parse.tsx`
    // is not `src/parse.ts`, however similar the stem.
    let token_has_extension = base.contains('.');

    paths.iter().any(|path| {
        let path_lower = path.to_lowercase();
        let path_base = path_lower.rsplit('/').next().unwrap_or(&path_lower);
        let path_stem = path_base
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or(path_base);
        if path_lower == cleaned || path_lower.ends_with(&format!("/{cleaned}")) {
            return true;
        }
        if token_has_extension {
            path_base == base
        } else {
            path_stem == stem
        }
    })
}

/// Did the subject mention anything that is actually in the diff? A subject
/// built entirely from invented vocabulary is suspicious, not fatal.
pub fn identifier_check(subject: &str, diff: &ParsedDiff) -> Option<Issue> {
    let has_source = diff
        .files
        .iter()
        .any(|file| matches!(classify(&file.path), Kind::Source));
    if !has_source {
        return None;
    }

    let identifiers: Vec<String> = tokens(subject)
        .into_iter()
        .filter(|token| {
            token.chars().count() >= 4
                && (token.contains('_') || token.chars().skip(1).any(|c| c.is_uppercase()))
        })
        .collect();
    if identifiers.is_empty() {
        return None;
    }

    let corpus = diff.text_corpus();
    if identifiers
        .iter()
        .any(|token| corpus.contains(&token.to_lowercase()))
    {
        return None;
    }

    Some(Issue::warning(format!(
        "the subject shares no identifier with the diff ({}); double-check the names",
        identifiers.join(", ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_path_like_tokens() {
        assert!(looks_like_path("src/diff/parse.rs"));
        assert!(looks_like_path("parse.rs"));
        assert!(looks_like_path("Cargo.lock"));
        assert!(!looks_like_path("parser"));
        assert!(!looks_like_path("v1.2"));
    }

    #[test]
    fn splits_tokens_around_punctuation() {
        let got = tokens("Refactor `BTreePage` (page.rs, cell_count) now");
        assert!(got.contains(&"BTreePage".to_string()), "{got:?}");
        assert!(got.contains(&"page.rs".to_string()), "{got:?}");
        assert!(got.contains(&"cell_count".to_string()), "{got:?}");
    }

    #[test]
    fn matches_paths_by_full_name_or_basename() {
        let mut paths = std::collections::HashSet::new();
        paths.insert("src/btree/page.rs".to_string());
        assert!(named_in_diff("src/btree/page.rs", &paths));
        assert!(named_in_diff("page.rs", &paths));
        assert!(named_in_diff("page", &paths));
        assert!(!named_in_diff("freelist.rs", &paths));

        let mut js = std::collections::HashSet::new();
        js.insert("src/parse.ts".to_string());
        assert!(named_in_diff("parse.ts", &js));
        assert!(!named_in_diff("parse.tsx", &js));
    }
}
