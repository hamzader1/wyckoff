//! Tests for the OpenAI-shaped request bodies.

use super::*;
use crate::config::ProviderConfig;

fn provider(shape: Shape) -> ResolvedProvider {
    let mut config = crate::config::Config::default();
    config.providers.insert(
        "x".into(),
        ProviderConfig {
            shape: Some(shape.as_str().into()),
            base: Some("https://example.test/v1".into()),
            model: Some("m".into()),
            api_key: Some("k".into()),
            json_mode: Some(true),
            ..Default::default()
        },
    );
    config.resolved("x", None).unwrap()
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
fn chat_completions_body_is_openai_shaped() {
    let body = OpenAi::new(provider(Shape::ChatCompletions)).body(&request());
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][0]["content"], "sys");
    assert_eq!(body["messages"][1]["content"], "usr");
    assert_eq!(body["response_format"]["type"], "json_object");
    assert_eq!(body["stream"], false);
    assert!(body.get("input").is_none());
}

#[test]
fn responses_body_uses_input_not_messages() {
    let body = OpenAi::new(provider(Shape::Responses)).body(&request());
    assert_eq!(body["input"][0]["content"], "sys");
    assert_eq!(body["input"][1]["content"], "usr");
    assert!(body.get("messages").is_none());
    assert!(
        body.get("response_format").is_none(),
        "the Responses equivalent is unstable, so we rely on the prompt instead"
    );
}

#[test]
fn json_mode_can_be_turned_off() {
    let mut config = crate::config::Config::default();
    config.providers.insert(
        "x".into(),
        ProviderConfig {
            shape: Some("chat_completions".into()),
            base: Some("https://example.test/v1".into()),
            model: Some("m".into()),
            api_key: Some("k".into()),
            json_mode: Some(false),
            ..Default::default()
        },
    );
    let body = OpenAi::new(config.resolved("x", None).unwrap()).body(&request());
    assert!(body.get("response_format").is_none());
}

#[test]
fn output_limit_uses_the_right_field_per_shape() {
    let mut chat_provider = provider(Shape::ChatCompletions);
    chat_provider.max_output_tokens = Some(700);
    let body = OpenAi::new(chat_provider).body(&request());
    assert_eq!(body["max_tokens"], 700);
    assert!(body.get("max_output_tokens").is_none());

    let mut responses_provider = provider(Shape::Responses);
    responses_provider.max_output_tokens = Some(700);
    let body = OpenAi::new(responses_provider).body(&request());
    assert_eq!(body["max_output_tokens"], 700);
    assert!(body.get("max_tokens").is_none());
}

#[test]
fn user_params_are_merged_last_and_win() {
    let mut configured = provider(Shape::ChatCompletions);
    configured
        .params
        .insert("temperature".into(), serde_json::json!(0.2));
    configured
        .params
        .insert("model".into(), serde_json::json!("override"));
    let body = OpenAi::new(configured).body(&request());
    assert_eq!(body["temperature"], 0.2);
    assert_eq!(body["model"], "override", "escape hatch beats our default");
}
