//! Tests for symbol extraction.

use super::*;
use crate::diff::ParsedDiff;

fn file_from(added: &[&str], removed: &[&str], path: &str) -> crate::diff::FileDiff {
    let mut text = format!(
        "diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1 +1,{} @@\n",
        added.len() + 1
    );
    text.push_str(" context\n");
    for line in removed {
        text.push('-');
        text.push_str(line);
        text.push('\n');
    }
    for line in added {
        text.push('+');
        text.push_str(line);
        text.push('\n');
    }
    ParsedDiff::parse(&text).files.remove(0)
}

fn names(symbols: Vec<Symbol>) -> Vec<String> {
    symbols.into_iter().map(|s| s.name).collect()
}

#[test]
fn finds_rust_definitions() {
    let file = file_from(
        &[
            "pub fn replace_cell(&mut self, index: usize) -> Result<Cell> {",
            "    // a struct is not defined in a comment like this",
            "pub struct PageHeader {",
            "let x = self.fn_pointer();",
        ],
        &[],
        "src/btree/page.rs",
    );
    let found = names(added_symbols(&file));
    assert!(found.contains(&"replace_cell".to_string()), "{found:?}");
    assert!(found.contains(&"PageHeader".to_string()), "{found:?}");
    assert_eq!(found.len(), 2, "{found:?}");
}

#[test]
fn finds_python_go_and_js_definitions() {
    let py = file_from(
        &["def parse_diff(text):", "class Hunk:"],
        &[],
        "src/parse.py",
    );
    assert_eq!(names(added_symbols(&py)), vec!["parse_diff", "Hunk"]);

    let go = file_from(
        &[
            "func (r *Reader) Read(p []byte) (int, error) {",
            "func NewReader(path string) *Reader {",
        ],
        &[],
        "reader.go",
    );
    assert_eq!(names(added_symbols(&go)), vec!["Read", "NewReader"]);

    let ts = file_from(
        &[
            "export function buildPrompt(ctx: Ctx) {",
            "const renderDiff = (diff) => {",
        ],
        &[],
        "src/prompt.ts",
    );
    assert_eq!(names(added_symbols(&ts)), vec!["buildPrompt", "renderDiff"]);
}

#[test]
fn removal_side_is_read_too() {
    let file = file_from(&[], &["pub fn old_helper() {"], "src/lib.rs");
    assert_eq!(names(removed_symbols(&file)), vec!["old_helper"]);
}

#[test]
fn calls_and_lists_are_not_definitions() {
    let file = file_from(
        &[
            "let page = open_page(fn_offset);",
            "    type, enum, struct,",
        ],
        &[],
        "src/a.rs",
    );
    assert!(
        added_symbols(&file).is_empty(),
        "{:?}",
        added_symbols(&file)
    );
}
