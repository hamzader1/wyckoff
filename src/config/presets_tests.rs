//! Tests for the provider table.

use super::*;

#[test]
fn finds_presets_case_insensitively() {
    assert_eq!(find("Gemini").unwrap().model, "gemini-3.8-flash");
    assert_eq!(find("NVIDIA").unwrap().shape, Shape::ChatCompletions);
    assert_eq!(find("ZEN").unwrap().shape, Shape::Responses);
    assert!(find("nope").is_none());
    assert!(names().contains(&"claude"));
    assert!(names().contains(&"ollama"));
}

#[test]
fn every_preset_is_self_consistent() {
    for preset in PRESETS {
        assert!(!preset.name.is_empty());
        assert!(!preset.key_env.is_empty());
        assert!(!preset.label.is_empty());
        if preset.shape == Shape::Mock {
            continue;
        }
        assert!(
            preset.base.starts_with("http"),
            "{} has no usable base url",
            preset.name
        );
        assert!(
            !preset.base.ends_with('/'),
            "{} base url should not end with /",
            preset.name
        );
    }
}

#[test]
fn local_presets_need_no_key_and_remote_ones_do() {
    assert!(!find("ollama").unwrap().needs_key);
    assert!(!find("lmstudio").unwrap().needs_key);
    assert!(find("gemini").unwrap().needs_key);
    assert!(find("claude").unwrap().needs_key);
}

#[test]
fn json_mode_only_where_it_is_supported() {
    assert!(find("gemini").unwrap().json_mode);
    assert!(
        !find("claude").unwrap().json_mode,
        "Anthropic's OpenAI shim ignores response_format, so we ask for JSON in text instead"
    );
    assert!(!find("nvidia").unwrap().json_mode);
}

#[test]
fn preset_names_are_unique() {
    let mut seen = names();
    seen.sort();
    let before = seen.len();
    seen.dedup();
    assert_eq!(seen.len(), before);
}
