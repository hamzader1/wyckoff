//! Tests for the per-subject metrics.

use super::*;

#[test]
fn detects_imperative_vs_not() {
    assert!(!looks_non_imperative("Refactor the page access path"));
    assert!(!looks_non_imperative("Add cell replacement support"));
    assert!(!looks_non_imperative(
        "Remake the btree page implementation"
    ));
    assert!(looks_non_imperative("Fixed the parser"));
    assert!(looks_non_imperative("Fixing the parser"));
    assert!(looks_non_imperative("Various improvements"));
    assert!(looks_non_imperative("This commit adds caching"));
}

#[test]
fn detects_tickets() {
    assert!(has_ticket("Fix login race (#4512)"));
    assert!(has_ticket("Handle PROJ-4512 timeouts"));
    assert!(!has_ticket("Fix login race"));
    assert!(!has_ticket("Bump version to 1-2-3"));
}

#[test]
fn detects_language_from_script_then_stopwords() {
    assert_eq!(
        detect_language(&["إضافة دعم جديد للتحقق".to_string()]),
        Language::Arabic
    );
    assert_eq!(
        detect_language(&["Add support for the new parser".to_string()]),
        Language::English
    );
    assert_eq!(
        detect_language(&[
            "Ajouter la gestion des erreurs".to_string(),
            "Corrige la gestion des erreurs pour le parseur".to_string(),
        ]),
        Language::French
    );
}

#[test]
fn detects_emoji_case_and_punctuation() {
    assert!(has_emoji("Add cache ✨"));
    assert!(!has_emoji("Add cache"));
    assert!(starts_sentence_case("Add cache"));
    assert!(!starts_sentence_case("add cache"));
    assert!(has_trailing_period("Add cache."));
    assert!(!has_trailing_period("Add cache"));
    assert!(is_conventional("feat: add cache"));
    assert!(!is_conventional("Add cache"));
}

#[test]
fn finds_opening_words() {
    assert_eq!(opening_word("Refactor the parser"), Some("refactor".into()));
    assert_eq!(opening_word("`Add` cache"), Some("add".into()));
    assert_eq!(opening_word("   "), None);
}
