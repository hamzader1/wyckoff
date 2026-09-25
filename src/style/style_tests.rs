//! Tests for the style engine.

use super::*;
use crate::git::CommitRecord;

fn record(sha: &str, subject: &str, paths: &[&str]) -> CommitRecord {
    CommitRecord {
        sha: sha.to_string(),
        subject: subject.to_string(),
        paths: paths.iter().map(|p| p.to_string()).collect(),
    }
}

fn untemplated_history() -> Vec<CommitRecord> {
    vec![
        record(
            "a",
            "Refactor B-Tree page access to use failable cell count retrieval",
            &["src/btree/page.rs"],
        ),
        record(
            "b",
            "Remake Btree page implementation to use generic byte access",
            &["src/btree/page.rs", "src/btree/mod.rs"],
        ),
        record(
            "c",
            "Add cell replacement support to the page layer",
            &["src/btree/page.rs"],
        ),
        record(
            "d",
            "Fix off by one in the free list walker",
            &["src/btree/freelist.rs"],
        ),
        record("e", "Simplify the cache key derivation", &["src/cache.rs"]),
        record(
            "f",
            "Drop the unused page type alias",
            &["src/btree/types.rs"],
        ),
    ]
}

#[test]
fn plain_repos_stay_plain() {
    let profile = StyleProfile::build(&untemplated_history(), &["src/btree/page.rs".into()], 3);
    assert_eq!(profile.decision(), Style::Plain);
    assert_eq!(profile.sample, 6);
    assert!(profile.conventional_ratio < 0.1);
    assert!(profile.non_imperative_ratio < 0.5);
    assert_eq!(profile.language, Language::English);
    assert!(profile.top_verbs.iter().any(|(v, _)| v == "refactor"));
}

#[test]
fn templated_repos_are_detected() {
    let history: Vec<CommitRecord> = (0..8)
        .map(|i| {
            record(
                &format!("{i}"),
                "feat(parser): add a thing",
                &["src/parse.rs"],
            )
        })
        .chain((0..2).map(|i| record(&format!("x{i}"), "Fix the parser", &["src/parse.rs"])))
        .collect();
    let profile = StyleProfile::build(&history, &["src/parse.rs".into()], 3);
    assert_eq!(profile.decision(), Style::Conventional);
    assert!(profile.conventional_ratio > 0.7);
}

#[test]
fn a_single_templated_commit_is_not_a_convention() {
    let history = vec![
        record("a", "feat: add cache", &["src/cache.rs"]),
        record("b", "Fix the parser", &["src/parse.rs"]),
    ];
    let profile = StyleProfile::build(&history, &[], 3);
    assert_eq!(
        profile.decision(),
        Style::Plain,
        "too small a sample to infer"
    );
}

#[test]
fn exemplars_prefer_commits_that_touched_the_same_files() {
    let profile = StyleProfile::build(&untemplated_history(), &["src/btree/page.rs".into()], 2);
    assert_eq!(profile.exemplars.len(), 2);
    assert!(profile.exemplars[0].overlap > 0);
    assert!(
        profile.exemplars.iter().all(|e| e.subject.contains("page")),
        "{:?}",
        profile.exemplars
    );
}

#[test]
fn exemplars_fall_back_to_recent_messages_without_overlap() {
    let profile = StyleProfile::build(&untemplated_history(), &["docs/notes.md".into()], 2);
    assert_eq!(profile.exemplars.len(), 2);
    assert!(
        profile.exemplars.iter().all(|e| e.overlap == 0),
        "{:?}",
        profile.exemplars
    );
    assert_eq!(
        profile.exemplars[0].subject,
        "Refactor B-Tree page access to use failable cell count retrieval"
    );
}

#[test]
fn empty_history_is_survivable() {
    let profile = StyleProfile::build(&[], &["src/a.rs".into()], 3);
    assert_eq!(profile.sample, 0);
    assert_eq!(profile.decision(), Style::Plain);
    assert!(profile.exemplars.is_empty());
    assert!(profile.guide().contains("last 0 commits"));
}

#[test]
fn guide_mentions_the_decided_style_and_language() {
    let profile = StyleProfile::build(&untemplated_history(), &[], 0);
    let guide = profile.guide();
    assert!(guide.contains("prefixes"), "{guide}");
    assert!(guide.contains("English"), "{guide}");
    assert!(guide.contains("imperative"), "{guide}");
    assert!(profile.summary_line().contains("plain"));
}
