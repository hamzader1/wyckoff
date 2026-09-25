//! Finding the provider payload inside a response.
//!
//! Not every gateway returns the OpenAI shape at the top level. Cline's API, for
//! example, wraps it: `{"data": {"choices": [...]}}`. Rather than special-casing
//! each one, we look for the object that actually holds an answer and hand that
//! to the shape-specific parsers.

use serde_json::Value;

/// Keys that identify the object holding the model's answer.
const MARKERS: &[&str] = &["choices", "output", "output_text", "content", "candidates"];

const MAX_DEPTH: usize = 4;
const MAX_NODES: usize = 64;

/// Walk the response (bounded) and return the first object that looks like a
/// provider payload. Returns the input unchanged when nothing better is found.
pub fn unwrap_payload(value: &Value) -> &Value {
    let mut queue: Vec<(&Value, usize)> = vec![(value, 0)];
    let mut visited = 0;

    while let Some((node, depth)) = queue.pop() {
        visited += 1;
        if visited > MAX_NODES {
            break;
        }

        if let Some(object) = node.as_object() {
            if MARKERS.iter().any(|marker| object.contains_key(*marker)) {
                return node;
            }
            if depth < MAX_DEPTH {
                for child in object.values() {
                    if child.is_object() || child.is_array() {
                        queue.push((child, depth + 1));
                    }
                }
            }
        } else if let Some(array) = node.as_array()
            && depth < MAX_DEPTH
        {
            for child in array {
                if child.is_object() {
                    queue.push((child, depth + 1));
                }
            }
        }
    }

    value
}

/// Top-level keys, for error messages that actually help.
pub fn top_level_keys(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            let keys: Vec<&str> = object.keys().map(|key| key.as_str()).take(12).collect();
            keys.join(", ")
        }
        Value::Array(_) => "(an array)".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_normal_openai_response_is_returned_as_is() {
        let value = serde_json::json!({"choices": [{"message": {"content": "hi"}}]});
        assert_eq!(
            unwrap_payload(&value)["choices"][0]["message"]["content"],
            "hi"
        );
    }

    #[test]
    fn the_cline_envelope_is_opened() {
        let value = serde_json::json!({"data": {"choices": [{"message": {"content": "hi"}}]}});
        let payload = unwrap_payload(&value);
        assert_eq!(payload["choices"][0]["message"]["content"], "hi");
    }

    #[test]
    fn nested_envelopes_are_opened_too() {
        let value = serde_json::json!({
            "result": {"body": {"output": [{"content": [{"text": "deep"}]}]}}
        });
        let payload = unwrap_payload(&value);
        assert!(payload.get("output").is_some(), "{payload}");
    }

    #[test]
    fn native_anthropic_responses_are_found_by_content() {
        let value = serde_json::json!({"content": [{"type": "text", "text": "hi"}]});
        assert_eq!(unwrap_payload(&value)["content"][0]["text"], "hi");
    }

    #[test]
    fn an_error_body_does_not_panic_and_stays_itself() {
        let value = serde_json::json!({"error": {"message": "bad key", "code": 401}});
        assert!(unwrap_payload(&value).get("error").is_some());
        assert_eq!(top_level_keys(&value), "error");
    }
}
