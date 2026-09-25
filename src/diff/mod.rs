//! The staged change, parsed.

pub mod classify;
mod headers;
pub mod parse;
pub mod scan;
pub mod tables;

pub use classify::{Kind, classify};
pub use parse::FileDiff;

use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, Default)]
pub struct ParsedDiff {
    pub files: Vec<FileDiff>,
}

impl ParsedDiff {
    pub fn parse(text: &str) -> Self {
        Self {
            files: scan::parse(text),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn insertions(&self) -> usize {
        self.files.iter().map(|f| f.insertions).sum()
    }

    pub fn deletions(&self) -> usize {
        self.files.iter().map(|f| f.deletions).sum()
    }

    pub fn paths(&self) -> Vec<String> {
        self.files
            .iter()
            .flat_map(|f| {
                let mut v = vec![f.path.clone()];
                if let Some(old) = &f.old_path {
                    v.push(old.clone());
                }
                v
            })
            .filter(|p| !p.is_empty())
            .collect()
    }

    /// Files grouped by kind. Order inside a group follows the diff order.
    pub fn by_kind(&self) -> Vec<(Kind, Vec<&FileDiff>)> {
        let mut map: BTreeMap<Kind, Vec<&FileDiff>> = BTreeMap::new();
        for file in &self.files {
            map.entry(classify(&file.path)).or_default().push(file);
        }
        map.into_iter().collect()
    }

    /// `7 files, +412 -88` — the compact header line we show and send.
    pub fn stat_line(&self) -> String {
        let mut out = crate::util::plural(self.files.len(), "file", "files");
        let ins = self.insertions();
        let del = self.deletions();
        if ins > 0 || del > 0 {
            let _ = write!(out, ", +{ins} -{del}");
        }
        out
    }

    /// Short descriptor: `3 source, 1 test, 1 lockfile`.
    pub fn kind_summary(&self) -> String {
        let parts: Vec<String> = self
            .by_kind()
            .into_iter()
            .map(|(kind, files)| format!("{} {}", files.len(), kind.label()))
            .collect();
        parts.join(", ")
    }

    /// Every path mentioned anywhere in the diff (used to catch invented paths).
    pub fn path_set(&self) -> std::collections::HashSet<String> {
        self.paths().into_iter().collect()
    }

    /// Everything the diff actually contains, lowercased — used to check that a
    /// generated subject shares real identifiers with the change.
    pub fn text_corpus(&self) -> String {
        let mut out = String::new();
        for file in &self.files {
            out.push_str(&file.raw);
            out.push('\n');
        }
        out.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::parse::Status;

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
        let diff = ParsedDiff::parse(SAMPLE);
        assert_eq!(diff.files.len(), 4);

        let first = &diff.files[0];
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
        let diff = ParsedDiff::parse(SAMPLE);
        assert_eq!(classify(&diff.files[1].path), Kind::Lockfile);
        assert_eq!(diff.files[2].status, Status::Renamed);
        assert_eq!(diff.files[2].old_path.as_deref(), Some("src/lib.rs"));
        assert_eq!(diff.files[2].path, "src/diff/engine.rs");
        assert!(diff.files[3].binary);
        assert_eq!(diff.files[3].status, Status::Added);
    }

    #[test]
    fn aggregates_and_headers() {
        let diff = ParsedDiff::parse(SAMPLE);
        // +2 -1 in parse.rs, +1 in Cargo.lock, the rename and the binary add nothing.
        assert_eq!(diff.insertions(), 3);
        assert_eq!(diff.deletions(), 1);
        assert!(diff.stat_line().starts_with("4 files, +3 -1"));
        assert!(diff.kind_summary().contains("source"));
        assert!(diff.path_set().contains("src/lib.rs"));
    }

    #[test]
    fn handles_quoted_and_spaced_paths() {
        let text = "diff --git \"a/my file.rs\" \"b/my file.rs\"\n--- \"a/my file.rs\"\n+++ \"b/my file.rs\"\n@@ -1 +1 @@\n-old\n+new\n";
        let diff = ParsedDiff::parse(text);
        assert_eq!(diff.files.len(), 1);
        assert_eq!(diff.files[0].path, "my file.rs");
        assert_eq!(diff.files[0].insertions, 1);
    }

    #[test]
    fn empty_diff_is_empty() {
        assert!(ParsedDiff::parse("").is_empty());
    }
}
