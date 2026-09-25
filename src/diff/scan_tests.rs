//! Fixtures and tests for the diff scanner.

use super::super::parse::Status;
use super::parse;

const SAMPLE: &str = "\
diff --git a/src/diff/parse.rs b/src/diff/parse.rs
index 1111111..2222222 100644
--- a/src/diff/parse.rs
+++ b/src/diff/parse.rs
@@ -12,7 +12,9 @@ pub fn parse(text: &str) -> Vec<FileDiff> {
     let mut files = Vec::new();
-    let old = 1;
+    let new = 2;
+    let extra = 3;
     files
diff --git a/Cargo.lock b/Cargo.lock
index 3333333..4444444 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,3 +1,4 @@
 version = 4
+name = \"newdep\"
diff --git a/src/lib.rs b/src/diff/engine.rs
similarity index 96%
rename from src/lib.rs
rename to src/diff/engine.rs
diff --git a/logo.png b/logo.png
new file mode 100644
index 0000000..5555555
Binary files /dev/null and b/logo.png differ
";

#[test]
fn parses_files_hunks_and_counts() {
    let files = parse(SAMPLE);
    assert_eq!(files.len(), 4);

    let first = &files[0];
    assert_eq!(first.path, "src/diff/parse.rs");
    assert_eq!(first.status, Status::Modified);
    assert_eq!(first.hunks.len(), 1);
    assert_eq!(first.hunks[0].new_start, 12);
    assert_eq!(first.insertions, 2);
    assert_eq!(first.deletions, 1);
    assert_eq!(
        first.added_lines(),
        vec!["    let new = 2;", "    let extra = 3;"]
    );
    assert!(first.raw.contains("extra"));
    assert!(!first.raw.contains("Cargo.lock"));
}

#[test]
fn detects_lockfiles_renames_and_binaries() {
    let files = parse(SAMPLE);
    assert_eq!(files[1].path, "Cargo.lock");
    assert_eq!(files[2].status, Status::Renamed);
    assert_eq!(files[2].old_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(files[2].path, "src/diff/engine.rs");
    assert_eq!(files[2].display_name(), "src/lib.rs -> src/diff/engine.rs");
    assert!(files[3].binary);
    assert_eq!(files[3].status, Status::Added);
}

#[test]
fn handles_quoted_and_spaced_paths() {
    let text = "diff --git \"a/my file.rs\" \"b/my file.rs\"\n--- \"a/my file.rs\"\n+++ \"b/my file.rs\"\n@@ -1 +1 @@\n-old\n+new\n";
    let files = parse(text);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "my file.rs");
    assert_eq!(files[0].insertions, 1);
    assert_eq!(files[0].hunks[0].old_start, 1);
}

#[test]
fn empty_diff_is_empty() {
    assert!(parse("").is_empty());
}

#[test]
fn new_file_and_summary_line() {
    let text = "diff --git a/src/new.rs b/src/new.rs\nnew file mode 100644\nindex 0000000..1111111\n--- /dev/null\n+++ b/src/new.rs\n@@ -0,0 +1,2 @@\n+one\n+two\n";
    let files = parse(text);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].status, Status::Added);
    assert_eq!(files[0].summary_line(), "added src/new.rs (+2 -0)");
}

#[test]
fn hunk_content_that_looks_like_a_header_is_not_treated_as_one() {
    let text = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,2 +1,3 @@\n alpha\n--- not a header\n+++ also not\n";
    let files = parse(text);
    assert_eq!(
        files[0].insertions, 1,
        "the +++ line is content, not a header"
    );
    assert_eq!(files[0].deletions, 1);
    assert_eq!(files[0].hunks.len(), 1);
    assert_eq!(files.len(), 1, "no phantom second file");
}
