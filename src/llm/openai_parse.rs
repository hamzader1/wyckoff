//! Digging the assistant text out of either OpenAI response shape.

/// `/responses` and `/chat/completions` put the answer in different places, and
/// gateways disagree about the convenience fields, so we try each in turn.
pub(super) fn extract_text(value: &serde_json::Value) -> Option<String> {
    // Responses API convenience field: present on some gateways, not all.
    if let Some(text) = value.get("output_text").and_then(|v| v.as_str())
        && !text.trim().is_empty()
    {
        return Some(text.to_string());
    }

    // /responses -> output[].content[].text
    if let Some(output) = value.get("output").and_then(|v| v.as_array()) {
        let mut parts: Vec<String> = Vec::new();
        for item in output {
            if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
                for part in content {
                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                        parts.push(text.to_string());
                    }
                }
            }
        }
        if !parts.is_empty() {
            return Some(parts.join("\n"));
        }
    }

    // /chat/completions -> choices[].message.content (string or parts array)
    let choices = value.get("choices")?.as_array()?;
    let message = choices.first()?.get("message")?;
    match message.get("content") {
        Some(serde_json::Value::String(text)) => Some(text.clone()),
        Some(serde_json::Value::Array(parts)) => {
            let text: Vec<String> = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(|t| t.as_str()).map(String::from))
                .collect();
            if text.is_empty() {
                None
            } else {
                Some(text.join("\n"))
            }
        }
        // Reasoning models occasionally leave `content` empty; do not lose the
        // answer entirely if the text is sitting next door.
        _ => message
            .get("reasoning_content")
            .and_then(|v| v.as_str())
            .map(String::from),
    }
}

/// Did the model run out of output room?
pub(super) fn is_truncated(value: &serde_json::Value) -> bool {
    let reason = value
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("finish_reason"))
        .and_then(|r| r.as_str())
        .or_else(|| {
            value
                .get("incomplete_details")
                .and_then(|d| d.get("reason"))
                .and_then(|r| r.as_str())
        });
    matches!(
        reason,
        Some("length") | Some("max_tokens") | Some("max_output_tokens")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_chat_completions_text() {
        let value = serde_json::json!({
            "choices": [{"message": {"content": "{\"subject\": \"Add cache\"}"}}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 4}
        });
        assert_eq!(
            extract_text(&value).unwrap(),
            "{\"subject\": \"Add cache\"}"
        );
    }

    #[test]
    fn reads_responses_text() {
        let value = serde_json::json!({
            "output": [
                {"type": "reasoning", "content": []},
                {"type": "message", "content": [{"type": "output_text", "text": "hello"}]}
            ]
        });
        assert_eq!(extract_text(&value).unwrap(), "hello");

        assert_eq!(
            extract_text(&serde_json::json!({"output_text": "hi"})).unwrap(),
            "hi"
        );
    }

    #[test]
    fn reads_parts_array_content() {
        let value = serde_json::json!({"choices": [{"message": {"content": [{"text": "a"}, {"text": "b"}]}}]});
        assert_eq!(extract_text(&value).unwrap(), "a\nb");
    }

    #[test]
    fn detects_truncation_from_either_shape() {
        assert!(is_truncated(&serde_json::json!({
            "choices": [{"finish_reason": "length"}]
        })));
        assert!(is_truncated(&serde_json::json!({
            "incomplete_details": {"reason": "max_output_tokens"}
        })));
        assert!(!is_truncated(&serde_json::json!({
            "choices": [{"finish_reason": "stop"}]
        })));
    }

    #[test]
    fn missing_text_is_none_not_a_panic() {
        assert!(extract_text(&serde_json::json!({"choices": []})).is_none());
        assert!(extract_text(&serde_json::json!({})).is_none());
    }

    /// The exact shape that broke the Cline gateway: the OpenAI payload nested
    /// under `data`, the answer as a fenced JSON block, plus a `reasoning` field.
    #[test]
    fn reads_a_payload_wrapped_in_an_envelope() {
        let value = serde_json::json!({
            "data": {
                "choices": [{
                    "finish_reason": "stop",
                    "index": 0,
                    "logprobs": null,
                    "message": {
                        "content": "```json\n{\n  \"subject\": \"Add Send Sync marker\",\n  \"body\": [],\n  \"breaking\": false,\n  \"footers\": [],\n  \"confidence\": 0.9,\n  \"notes_to_user\": \"\"\n}\n```",
                        "reasoning": "Let me analyze this commit: the diff adds a marker function…"
                    }
                }]
            }
        });

        let payload = crate::llm::envelope::unwrap_payload(&value);
        let text = extract_text(payload).expect("the fenced answer should be found");
        let msg = crate::msg::CommitMsg::from_provider_output(&text).unwrap();
        assert_eq!(msg.subject, "Add Send Sync marker");
        assert_eq!(msg.confidence, Some(0.9));
        assert!(!is_truncated(payload));
    }
}
