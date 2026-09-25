//! Tests for prompt assembly.

use super::*;
use crate::branch;
use crate::context::{ContextFile, ProjectContext};
use crate::diff::ParsedDiff;
use crate::style::StyleProfile;

fn fixture() -> (
    ParsedDiff,
    ProjectContext,
    StyleProfile,
    BranchHint,
    RenderedDiff,
) {
    let diff = ParsedDiff::parse(
        "diff --git a/src/diff/parse.rs b/src/diff/parse.rs\n--- a/src/diff/parse.rs\n+++ b/src/diff/parse.rs\n@@ -1,2 +1,4 @@\n fn parse() {\n+    let more = 1;\n+pub fn extra_parser() {}\n }\n",
    );
    let context = ProjectContext {
        context_files: vec![ContextFile {
            path: ".context".into(),
            body: "wyckoff writes commit messages from the staged diff.".into(),
        }],
        manifests: vec![("Cargo.toml".into(), "crate `wyckoff`".into())],
        layout: "12 tracked files. Top-level: src/ (8).".into(),
        file_count: 12,
        extra: None,
    };
    let style = StyleProfile::build(&[], &diff.paths(), 3);
    let branch = branch::parse("main");
    let rendered = render_diff(&diff, 4000, &[]);
    (diff, context, style, branch, rendered)
}

#[test]
fn system_prompt_forbids_prefixes_and_demands_json() {
    let (diff, context, style, branch, rendered) = fixture();
    let request = build(
        &diff,
        &context,
        &style,
        &branch,
        Options::default(),
        &rendered,
    );
    assert!(request.system.contains("NO type or scope prefixes"));
    assert!(request.system.contains("\"subject\""));
    assert!(request.system.contains("120 characters"));
    assert!(request.system.contains("body: always []"));
    assert!(request.wants_json);
}

#[test]
fn system_prompt_follows_a_templated_repo() {
    let (diff, context, _, branch, rendered) = fixture();
    let history: Vec<crate::git::CommitRecord> = (0..8)
        .map(|i| crate::git::CommitRecord {
            sha: i.to_string(),
            subject: "feat(parser): add a thing".into(),
            paths: vec!["src/diff/parse.rs".into()],
        })
        .collect();
    let style = StyleProfile::build(&history, &diff.paths(), 3);
    let request = build(
        &diff,
        &context,
        &style,
        &branch,
        Options::default(),
        &rendered,
    );
    assert!(
        request.system.contains("does* use `type(scope):` prefixes"),
        "{}",
        request.system
    );
}

#[test]
fn body_instructions_follow_the_flag() {
    let (diff, context, style, branch, rendered) = fixture();
    let request = build(
        &diff,
        &context,
        &style,
        &branch,
        Options {
            include_body: true,
            ..Options::default()
        },
        &rendered,
    );
    assert!(request.system.contains("short bullets"));
}

#[test]
fn user_prompt_carries_context_style_branch_and_diff() {
    let (diff, context, style, branch, rendered) = fixture();
    let request = build(
        &diff,
        &context,
        &style,
        &branch,
        Options {
            user_intent: Some("fix the login race"),
            ..Options::default()
        },
        &rendered,
    );
    assert!(request.user.contains("## Project"));
    assert!(request.user.contains("wyckoff writes commit messages"));
    assert!(request.user.contains("## Branch"));
    assert!(request.user.contains("weak hint only"));
    assert!(
        request
            .user
            .contains("## How this repo writes commit messages")
    );
    assert!(request.user.contains("## Staged change"));
    assert!(request.user.contains("let more = 1;"));
    assert!(
        request
            .user
            .contains("Definitions this diff introduces or removes")
    );
    assert!(request.user.contains("`extra_parser`"), "{}", request.user);
    assert!(request.user.contains("fix the login race"));
}

#[test]
fn exemplars_and_extra_instructions_are_included() {
    let (diff, context, _, branch, rendered) = fixture();
    let history = vec![
        crate::git::CommitRecord {
            sha: "a".into(),
            subject: "Refactor the page access path".into(),
            paths: vec!["src/diff/parse.rs".into()],
        },
        crate::git::CommitRecord {
            sha: "b".into(),
            subject: "Add cell replacement".into(),
            paths: vec!["src/diff/parse.rs".into()],
        },
    ];
    let style = StyleProfile::build(&history, &diff.paths(), 2);
    let request = build(
        &diff,
        &context,
        &style,
        &branch,
        Options {
            extra_instructions: Some("always mention the ticket"),
            ..Options::default()
        },
        &rendered,
    );
    assert!(request.user.contains("imitate the voice"));
    assert!(request.user.contains("Refactor the page access path"));
    assert!(request.system.contains("always mention the ticket"));
}

#[test]
fn repair_prompt_repeats_the_problems() {
    let (diff, context, style, branch, rendered) = fixture();
    let request = build(
        &diff,
        &context,
        &style,
        &branch,
        Options::default(),
        &rendered,
    );
    let fixed = repair(
        &request,
        "{\"subject\": \"feat: something\"}",
        &["subject is 128 characters, max is 120".to_string()],
    );
    assert!(fixed.user.contains("rejected"));
    assert!(fixed.user.contains("max is 120"));
    assert_eq!(fixed.system, request.system);
}
