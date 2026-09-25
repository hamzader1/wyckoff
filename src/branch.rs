//! Branch name -> weak prior.
//!
//! A branch name is a *hint*, never a source of truth: `remake/drop-old-parser`
//! can easily end up being a three-line doc fix. We extract structure from it
//! and hand it to the model explicitly labelled as a weak signal, so a
//! disagreement between branch and diff resolves in favour of the diff.

#[derive(Debug, Clone, Default)]
pub struct BranchHint {
    pub raw: String,
    /// `feat`, `fix`, `remake`, `refactor`, ... if the branch is prefixed.
    pub kind: Option<String>,
    /// `ABC-123`, `#42`, ... if the branch carries a ticket.
    pub ticket: Option<String>,
    pub slug: Option<String>,
    pub is_default: bool,
}

const KINDS: &[&str] = &[
    "feat",
    "feature",
    "fix",
    "bugfix",
    "hotfix",
    "chore",
    "refactor",
    "refact",
    "remake",
    "rework",
    "rewrite",
    "redo",
    "docs",
    "doc",
    "test",
    "tests",
    "perf",
    "ci",
    "build",
    "release",
    "wip",
    "experiment",
    "spike",
    "cleanup",
    "clean",
    "revert",
    "improve",
    "update",
    "add",
    "remove",
    "delete",
    "migrate",
    "port",
    "support",
    "try",
    "tmp",
    "scratch",
    "draft",
    "poc",
    "review",
];

const DEFAULT_BRANCHES: &[&str] = &[
    "main",
    "master",
    "dev",
    "develop",
    "trunk",
    "staging",
    "stage",
    "prod",
    "production",
    "next",
    "head",
    "default",
];

pub fn is_default(name: &str) -> bool {
    let lower = name.to_lowercase();
    DEFAULT_BRANCHES.contains(&lower.as_str())
}

pub fn parse(name: &str) -> BranchHint {
    let raw = name
        .trim()
        .trim_start_matches("refs/heads/")
        .trim_start_matches("origin/")
        .to_string();

    let mut hint = BranchHint {
        raw: raw.clone(),
        is_default: is_default(&raw),
        ..Default::default()
    };
    if raw.is_empty() || raw == "HEAD" {
        return hint;
    }

    // Ticket ids keep their prefix: split on `/` first so `PROJ-4512` stays whole.
    let segments: Vec<String> = raw
        .split(['/', '_', '.', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect();

    let mut rest: Vec<String> = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        // A segment that is exactly a ticket id, wherever it appears.
        if let Some(ticket) = looks_like_ticket(segment) {
            if hint.ticket.is_none() {
                hint.ticket = Some(ticket);
            }
            continue;
        }

        let mut words: Vec<String> = segment
            .split('-')
            .filter(|part| !part.is_empty())
            .map(|part| part.to_string())
            .collect();

        if index == 0
            && let Some(first) = words.first()
            && KINDS.contains(&first.to_lowercase().as_str())
        {
            hint.kind = Some(first.to_lowercase());
            words.remove(0);
        }

        if hint.ticket.is_none() {
            for word in words.iter() {
                if let Some(ticket) = looks_like_ticket(word) {
                    hint.ticket = Some(ticket);
                    break;
                }
            }
        }
        rest.extend(words);
    }

    if !rest.is_empty() {
        hint.slug = Some(rest.join(" "));
    }
    hint
}

/// `PROJ-4512` or `#4512` -> the normalised ticket id.
fn looks_like_ticket(token: &str) -> Option<String> {
    if let Some(digits) = token.strip_prefix('#')
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
    {
        return Some(format!("#{digits}"));
    }
    let (prefix, suffix) = token.split_once('-')?;
    let prefix_ok = prefix.len() >= 2 && prefix.chars().all(|c| c.is_ascii_alphabetic());
    let suffix_ok =
        !suffix.is_empty() && suffix.len() <= 6 && suffix.chars().all(|c| c.is_ascii_digit());
    if prefix_ok && suffix_ok {
        Some(token.to_uppercase())
    } else {
        None
    }
}

impl BranchHint {
    /// One-line description for the prompt, e.g.
    /// `remake/btree-generic-bytes -> kind=remake, slug="btree generic bytes"`.
    pub fn describe(&self) -> String {
        if self.raw.is_empty() {
            return "(detached HEAD, no branch)".to_string();
        }
        if self.is_default {
            return format!("{} (default branch, no hint)", self.raw);
        }
        let mut parts = Vec::new();
        if let Some(kind) = &self.kind {
            parts.push(format!("author's own label: {kind}"));
        }
        if let Some(ticket) = &self.ticket {
            parts.push(format!("ticket: {ticket}"));
        }
        if let Some(slug) = &self.slug {
            parts.push(format!("slug: \"{slug}\""));
        }
        if parts.is_empty() {
            self.raw.clone()
        } else {
            format!("{} ({})", self.raw, parts.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_kind_slug_and_ticket() {
        let hint = parse("remake/btree-generic-bytes");
        assert_eq!(hint.kind.as_deref(), Some("remake"));
        assert_eq!(hint.slug.as_deref(), Some("btree generic bytes"));
        assert!(hint.ticket.is_none());

        let hint = parse("fix/PROJ-4512/login-race");
        assert_eq!(hint.kind.as_deref(), Some("fix"));
        assert_eq!(hint.ticket.as_deref(), Some("PROJ-4512"));
        assert_eq!(hint.slug.as_deref(), Some("login race"));

        let hint = parse("feat/cache-warmup");
        assert_eq!(hint.kind.as_deref(), Some("feat"));
    }

    #[test]
    fn default_branches_carry_no_hint() {
        let hint = parse("main");
        assert!(hint.is_default);
        assert!(hint.kind.is_none());
        assert!(hint.describe().contains("default branch"));
        assert_eq!(parse("refs/heads/dev").kind, None);
        assert!(parse("refs/heads/dev").is_default);
    }

    #[test]
    fn plain_branch_names_still_give_a_slug() {
        let hint = parse("hamza/cleanup");
        assert!(hint.kind.is_none());
        assert_eq!(hint.slug.as_deref(), Some("hamza cleanup"));
    }

    #[test]
    fn described_hint_is_readable() {
        let text = parse("rework/page-types").describe();
        assert!(text.contains("author's own label: rework"), "{text}");
        assert!(text.contains("page types"), "{text}");
    }
}
