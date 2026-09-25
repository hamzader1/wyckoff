//! Tests for provider resolution.

use super::{Config, ProviderConfig, Shape};

fn with_key(name: &str) -> Config {
    let mut config = Config::default();
    config.providers.insert(
        name.to_string(),
        ProviderConfig {
            api_key: Some("test-key".into()),
            ..Default::default()
        },
    );
    config
}

#[test]
fn presets_supply_the_rest_of_the_details() {
    let config = with_key("gemini");
    let gemini = config.resolved("gemini", None).unwrap();
    assert_eq!(gemini.shape, Shape::ChatCompletions);
    assert!(gemini.base.contains("generativelanguage"));
    assert!(gemini.json_mode);
    assert_eq!(
        gemini.chat_url(),
        format!("{}/chat/completions", gemini.base)
    );

    let claude = with_key("claude").resolved("claude", None).unwrap();
    assert_eq!(claude.shape, Shape::AnthropicMessages);
    assert_eq!(claude.chat_url(), "https://api.anthropic.com/v1/messages");
    assert!(!claude.json_mode, "asked for JSON in text instead");

    let openai = with_key("openai").resolved("openai", None).unwrap();
    assert_eq!(openai.shape, Shape::Responses);
    assert_eq!(openai.chat_url(), "https://api.openai.com/v1/responses");

    let nvidia = with_key("nvidia").resolved("nvidia", None).unwrap();
    assert_eq!(nvidia.base, "https://integrate.api.nvidia.com/v1");
}

#[test]
fn model_override_wins_over_config() {
    let config = with_key("gemini");
    let provider = config
        .resolved("gemini", Some("gemini-2.5-flash-lite"))
        .unwrap();
    assert_eq!(provider.model, "gemini-2.5-flash-lite");
}

#[test]
fn custom_openai_compatible_provider_needs_only_config() {
    let mut config = Config::default();
    config.providers.insert(
        "gw".into(),
        ProviderConfig {
            shape: Some("chat_completions".into()),
            base: Some("https://gw.internal/v1/".into()),
            model: Some("m".into()),
            api_key: Some("k".into()),
            ..Default::default()
        },
    );
    let provider = config.resolved("gw", None).unwrap();
    assert_eq!(
        provider.base, "https://gw.internal/v1",
        "trailing slash trimmed"
    );
    assert_eq!(
        provider.chat_url(),
        "https://gw.internal/v1/chat/completions"
    );
    assert!(config.known_providers().contains(&"gw".to_string()));
}

#[test]
fn unknown_provider_lists_the_known_ones() {
    let error = Config::default()
        .resolved("not-a-provider", None)
        .unwrap_err();
    let text = error.to_string();
    assert!(text.contains("unknown provider"), "{text}");
    assert!(text.contains("gemini"), "{text}");
    assert!(text.contains("nvidia"), "{text}");
}

#[test]
fn a_model_is_required_and_the_error_is_actionable() {
    let error = Config::default().resolved("ollama", None).unwrap_err();
    assert!(error.to_string().contains("needs a model"), "{error}");

    let provider = Config::default()
        .resolved("ollama", Some("qwen2.5-coder:7b"))
        .unwrap();
    assert_eq!(provider.model, "qwen2.5-coder:7b");
    assert_eq!(
        provider.chat_url(),
        "http://localhost:11434/v1/chat/completions"
    );
}

#[test]
fn missing_key_error_names_the_variable_and_the_file() {
    // This process has no GEMINI_API_KEY.
    let error = Config::default().resolved("gemini", None).unwrap_err();
    let text = error.to_string();
    assert!(text.contains("GEMINI_API_KEY"), "{text}");
    assert!(text.contains("key_cmd"), "{text}");
}

#[test]
fn key_resolution_prefers_config_then_env_then_command() {
    let mut config = Config::default();
    config.providers.insert(
        "claude".into(),
        ProviderConfig {
            api_key: Some("from-config".into()),
            key_env: Some("WYCKOFF_TEST_KEY".into()),
            key_cmd: Some("echo from-command".into()),
            ..Default::default()
        },
    );
    assert_eq!(
        config.resolved("claude", None).unwrap().api_key.as_deref(),
        Some("from-config")
    );

    let mut config = Config::default();
    config.providers.insert(
        "claude".into(),
        ProviderConfig {
            key_env: Some("WYCKOFF_TEST_KEY".into()),
            key_cmd: Some("echo from-command".into()),
            ..Default::default()
        },
    );
    assert_eq!(
        config
            .resolved("claude", Some("m"))
            .map(|p| p.api_key)
            .ok()
            .flatten()
            .as_deref(),
        Some("from-command"),
        "falls back to key_cmd when no env var is set"
    );
}
