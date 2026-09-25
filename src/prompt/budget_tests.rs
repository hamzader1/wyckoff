//! Tests for diff budgeting.

use super::*;

const DIFF: &str = "\
diff --git a/src/diff/parse.rs b/src/diff/parse.rs
--- a/src/diff/parse.rs
+++ b/src/diff/parse.rs
@@ -1,3 +1,4 @@
 fn parse() {
+    let more = 1;
 }
diff --git a/Cargo.lock b/Cargo.lock
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,3 +1,4 @@
 version = 4
+name = \"newdep\"
";

#[test]
fn lockfile_body_is_replaced_by_its_effect() {
    let diff = ParsedDiff::parse(DIFF);
    let rendered = render(&diff, 10_000, &[]);
    assert!(rendered.text.contains("src/diff/parse.rs"));
    assert!(rendered.text.contains("let more = 1;"));
    assert!(!rendered.text.contains("name = \"newdep\""));
    assert!(rendered.text.contains("modified Cargo.lock"));
    assert_eq!(rendered.included_files, 1);
    assert_eq!(rendered.summarized_files, 1);
    assert!(!rendered.truncated);
}

#[test]
fn a_small_budget_still_pays_for_the_source() {
    let diff = ParsedDiff::parse(DIFF);
    let rendered = render(&diff, 80, &[]);
    assert!(
        rendered.text.contains("let more = 1;"),
        "source should be paid for first: {}",
        rendered.text
    );
}

#[test]
fn oversized_files_are_condensed_and_declared() {
    let mut big = String::from(
        "diff --git a/src/big.rs b/src/big.rs\n--- a/src/big.rs\n+++ b/src/big.rs\n@@ -1,1 +1,400 @@\n",
    );
    for i in 0..400 {
        big.push_str(&format!("+line {i}\n"));
    }
    let diff = ParsedDiff::parse(&big);
    let rendered = render(&diff, 300, &[]);
    assert!(rendered.truncated);
    assert!(
        rendered.tokens <= 700,
        "budget respected: {}",
        rendered.tokens
    );
    assert!(
        rendered.text.contains("more lines in this hunk"),
        "{}",
        rendered.text
    );
}

#[test]
fn omitted_files_are_listed() {
    let diff = ParsedDiff::parse(DIFF);
    let rendered = render(&diff, 1, &[]);
    assert!(!rendered.omitted_files.is_empty());
    assert!(
        rendered.text.contains("not shown in full"),
        "{}",
        rendered.text
    );
}

#[test]
fn skip_patterns_replace_a_body_with_its_summary_line() {
    let diff = ParsedDiff::parse(DIFF);
    let rendered = render(&diff, 10_000, &["src/diff/*".to_string()]);
    assert!(
        !rendered.text.contains("let more = 1;"),
        "{}",
        rendered.text
    );
    assert!(
        rendered.text.contains("modified src/diff/parse.rs"),
        "{}",
        rendered.text
    );
}

#[test]
fn glob_lite_matching() {
    assert!(matches_pattern("Cargo.lock", "*.lock"));
    assert!(matches_pattern("src/gen/foo.rs", "gen/*"));
    assert!(matches_pattern("vendor/x/y.go", "vendor/"));
    assert!(!matches_pattern("src/a.rs", "*.lock"));
    assert!(!matches_pattern("src/a.rs", ""));
    assert!(!matches_pattern("web/src/a.ts", "vendor/*"));
}
