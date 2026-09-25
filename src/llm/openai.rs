//! OpenAI-compatible endpoints: `/chat/completions` and `/responses`.
//!
//! One implementation covers OpenAI itself and every OpenAI-compatible gateway
//! (Gemini's compat endpoint, NVIDIA NIM, OpenCode Zen, Ollama, LM Studio, plus
//! anything a user adds in config): they only differ in the request body and in
//! where the assistant text hides in the response.

#[path = "openai_parse.rs"]
mod openai_parse;

#[cfg(test)]
#[path = "openai_tests.rs"]
mod openai_tests;

use super::{LlmResponse, Provider, Usage, apply_params, bearer_headers, envelope, http};
use crate::config::{ResolvedProvider, Shape};
use crate::error::{Error, Result};
use crate::prompt::Request;

use openai_parse::{extract_text, is_truncated};

pub struct OpenAi {
    provider: ResolvedProvider,
}

impl OpenAi {
    pub fn new(provider: ResolvedProvider) -> Self {
        Self { provider }
    }

    /// The request body for whichever OpenAI surface this provider uses.
    pub(crate) fn body(&self, request: &Request) -> serde_json::Value {
        let mut body = match self.provider.shape {
            Shape::Responses => serde_json::json!({
                "model": self.provider.model,
                "input": [
                    { "role": "system", "content": request.system },
                    { "role": "user", "content": request.user },
                ],
            }),
            _ => {
                let mut body = serde_json::json!({
                    "model": self.provider.model,
                    "stream": false,
                    "messages": [
                        { "role": "system", "content": request.system },
                        { "role": "user", "content": request.user },
                    ],
                });
                // Universally supported on the chat-completions surface. The
                // Responses equivalent keeps changing shape, so there we rely on
                // the prompt plus tolerant parsing instead of risking a 400 on
                // someone's first run.
                if self.provider.json_mode && request.wants_json {
                    body["response_format"] = serde_json::json!({ "type": "json_object" });
                }
                body
            }
        };

        if let Some(limit) = self.provider.max_output_tokens
            && let Some(object) = body.as_object_mut()
        {
            let field = match self.provider.shape {
                Shape::Responses => "max_output_tokens",
                _ => "max_tokens",
            };
            object.insert(field.to_string(), serde_json::json!(limit));
        }

        apply_params(&mut body, &self.provider);
        body
    }
}

impl Provider for OpenAi {
    fn name(&self) -> &str {
        &self.provider.name
    }

    fn model(&self) -> &str {
        &self.provider.model
    }

    fn complete(&self, request: &Request) -> Result<LlmResponse> {
        let body = self.body(request);
        let response = http::post_json(
            &self.provider.chat_url(),
            &bearer_headers(&self.provider),
            &body,
            self.provider.timeout_secs,
            &self.provider.name,
        )?;

        let value: serde_json::Value = serde_json::from_str(&response.body).map_err(|e| {
            Error::ModelOutput(format!(
                "the endpoint returned something that is not JSON: {e}\n{}",
                super::mock::clip(&response.body)
            ))
        })?;

        // Some gateways wrap the payload in an envelope (`{"data": {...}}`).
        let payload = envelope::unwrap_payload(&value);

        let text = extract_text(payload).ok_or_else(|| {
            Error::ModelOutput(format!(
                "no text found in the response (top-level keys: [{}], shape {}, model {}).\n{}",
                envelope::top_level_keys(&value),
                self.provider.shape.as_str(),
                self.provider.model,
                super::mock::clip(&response.body)
            ))
        })?;

        Ok(LlmResponse {
            text,
            usage: read_usage(payload),
            truncated: is_truncated(payload),
        })
    }

    fn list_models(&self) -> Result<Vec<String>> {
        let response = http::get_json(
            &self.provider.models_url(),
            &bearer_headers(&self.provider),
            self.provider.timeout_secs,
        )?;
        let value: serde_json::Value = serde_json::from_str(&response.body)?;

        let mut names: Vec<String> = value
            .get("data")
            .and_then(|d| d.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("id").and_then(|v| v.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if names.is_empty()
            && let Some(array) = value.get("models").and_then(|m| m.as_array())
        {
            names = array
                .iter()
                .filter_map(|item| item.get("id").and_then(|v| v.as_str()).map(String::from))
                .collect();
        }
        names.sort();
        Ok(names)
    }
}

fn read_usage(value: &serde_json::Value) -> Usage {
    let Some(usage) = value.get("usage") else {
        return Usage::default();
    };
    Usage {
        input_tokens: usage
            .get("prompt_tokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(|v| v.as_u64()),
        output_tokens: usage
            .get("completion_tokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(|v| v.as_u64()),
    }
}
