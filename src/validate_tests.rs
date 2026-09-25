//! Tests for the validator.

use super::*;
use crate::msg::CommitMsg;
use crate::style::Language;
use crate::validate::rules::warnings;

const DIFF: &str = "\
diff --git a/src/btree/page.rs b/src/btree/page.rs
--- a/src/btree/page.rs
+++ b/src/btree/page.rs
@@ -1,3 +1,4 @@
 fn cell_count(&self) -> usize {
+    BTreePage::cell_count(self)
 }
";

fn context(diff: &ParsedDiff) -> Context<'_> {
    Context {
        diff,
        max_subject_len: 120,
        allow_prefix: false,
        language: Language::English,
    }
}

fn check(subject: &str) -> Vec<Issue> {
    let diff = ParsedDiff::parse(DIFF);
    validate(&CommitMsg::new(subject), &context(&diff))
}

#[test]
fn a_good_subject_passes_clean() {
    let issues = check("Refactor Btree page access to use failable cell count retrieval");
    assert!(issues.is_empty(), "{issues:?}");
}

#[test]
fn long_subjects_are_errors() {
    let long = "Refactor ".to_string() + &"very long detail ".repeat(12);
    let issues = check(&long);
    assert!(blocking(&issues).iter().any(|m| m.contains("characters")));
}

#[test]
fn prefixes_are_errors_unless_the_repo_uses_them() {
    let issues = check("feat(parser): add rename support");
    assert!(
        blocking(&issues).iter().any(|m| m.contains("type prefix")),
        "{issues:?}"
    );

    let diff = ParsedDiff::parse(DIFF);
    let config = Context {
        allow_prefix: true,
        ..context(&diff)
    };
    let issues = validate(&CommitMsg::new("feat: add rename support"), &config);
    assert!(blocking(&issues).is_empty(), "{issues:?}");
}

#[test]
fn lowercase_starts_are_errors() {
    let issues = check("refactor the page access");
    assert!(blocking(&issues).iter().any(|m| m.contains("lowercase")));
}

#[test]
fn invented_paths_are_errors() {
    let issues = check("Refactor src/btree/freelist.rs access");
    assert!(
        blocking(&issues).iter().any(|m| m.contains("freelist.rs")),
        "{issues:?}"
    );

    let issues = check("Refactor src/btree/page.rs access");
    assert!(blocking(&issues).is_empty(), "{issues:?}");
}

#[test]
fn bare_filenames_are_recognised() {
    let issues = check("Extend page.rs with a safer cell count");
    assert!(blocking(&issues).is_empty(), "{issues:?}");
}

#[test]
fn filler_and_tenses_are_warnings() {
    let issues = check("This commit makes various improvements");
    let list = warnings(&issues);
    assert!(list.iter().any(|m| m.contains("this commit")), "{list:?}");

    let issues = check("Fixed the page access");
    assert!(
        warnings(&issues).iter().any(|m| m.contains("imperative")),
        "{issues:?}"
    );
}

#[test]
fn hallucinated_vocabulary_is_flagged_without_blocking() {
    let issues = check("Refactor WidgetFactoryBroker caching layer");
    assert!(
        warnings(&issues)
            .iter()
            .any(|m| m.contains("shares no identifier")),
        "{issues:?}"
    );
    assert!(blocking(&issues).is_empty(), "{issues:?}");
}

#[test]
fn real_identifiers_are_accepted() {
    let issues = check("Refactor BTreePage cell count lookup");
    assert!(
        !warnings(&issues)
            .iter()
            .any(|m| m.contains("shares no identifier")),
        "{issues:?}"
    );
}

#[test]
fn language_mismatch_is_a_warning() {
    let diff = ParsedDiff::parse(DIFF);
    let config = Context {
        language: Language::Arabic,
        ..context(&diff)
    };
    let issues = validate(&CommitMsg::new("Refactor the page access"), &config);
    assert!(
        warnings(&issues).iter().any(|m| m.contains("Arabic")),
        "{issues:?}"
    );
}

#[test]
fn empty_and_period_subjects() {
    assert!(
        blocking(&validate(
            &CommitMsg::default(),
            &context(&ParsedDiff::default())
        ))
        .len()
            == 1
    );

    let issues = check("Refactor the page access.");
    assert!(
        warnings(&issues).iter().any(|m| m.contains("period")),
        "{issues:?}"
    );
}

#[test]
fn breaking_without_explanation_is_a_warning() {
    let diff = ParsedDiff::parse(DIFF);
    let msg = CommitMsg {
        subject: "Rework the page access API".into(),
        breaking: true,
        ..Default::default()
    };
    let issues = validate(&msg, &context(&diff));
    assert!(
        warnings(&issues).iter().any(|m| m.contains("breaking")),
        "{issues:?}"
    );
}
