//! Tests for the Anthropic request shape.

use super::*;
use crate::config::{Config, ProviderConfig, Shape};

fn provider() -> ResolvedProvider {
    let mut config = Config::default();
    config.providers.insert(
        "claude".into(),
        ProviderConfig {
            api_key: Some("key-123".into()),
            ..Default::default()
        },
    );
    config.resolved("claude", None).unwrap()
}

fn request() -> Request {
    Request {
        system: "sys".into(),
        user: "usr".into(),
        max_output_tokens: 700,
        wants_json: true,
    }
}

#[test]
fn targets_the_messages_endpoint_with_x_api_key() {
    let provider = provider();
    assert_eq!(provider.shape, Shape::AnthropicMessages);
    assert_eq!(provider.chat_url(), "https://api.anthropic.com/v1/messages");

    let names: Vec<String> = Anthropic::new(provider)
        .headers()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert!(names.contains(&"x-api-key".to_string()));
    assert!(names.contains(&"anthropic-version".to_string()));
    assert!(!names.contains(&"authorization".to_string()));
}

#[test]
fn body_carries_system_separately_and_requires_max_tokens() {
    let body = Anthropic::new(provider()).body(&request());
    assert_eq!(body["system"], "sys");
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"], "usr");
    assert_eq!(body["max_tokens"], 700);
    assert!(body.get("response_format").is_none());
}

#[test]
fn configured_output_limit_wins() {
    let mut provider = provider();
    provider.max_output_tokens = Some(256);
    assert_eq!(Anthropic::new(provider).body(&request())["max_tokens"], 256);
}
