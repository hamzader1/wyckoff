//! Anthropic's native Messages API.
//!
//! Used instead of their OpenAI compatibility layer on purpose: that layer
//! documents `response_format` as ignored, which would cost JSON-mode
//! reliability. Native also gives us `stop_reason`, so we can tell when the
//! model ran out of room instead of guessing.

#[cfg(test)]
#[path = "anthropic_tests.rs"]
mod anthropic_tests;

use super::{LlmResponse, Provider, Usage, apply_params, http};
use crate::config::ResolvedProvider;
use crate::error::{Error, Result};
use crate::prompt::Request;

pub struct Anthropic {
    provider: ResolvedProvider,
}

impl Anthropic {
    pub fn new(provider: ResolvedProvider) -> Self {
        Self { provider }
    }

    pub(crate) fn headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("accept".to_string(), "application/json".to_string()),
            (
                "anthropic-version".to_string(),
                self.provider.api_version.clone(),
            ),
        ];
        if let Some(key) = &self.provider.api_key {
            headers.push(("x-api-key".to_string(), key.clone()));
        }
        for (name, value) in &self.provider.headers {
            headers.push((name.clone(), value.clone()));
        }
        headers
    }

    pub(crate) fn body(&self, request: &Request) -> serde_json::Value {
        // `max_tokens` is required by this API, unlike the OpenAI surfaces.
        let limit = self
            .provider
            .max_output_tokens
            .unwrap_or(request.max_output_tokens);
        let mut body = serde_json::json!({
            "model": self.provider.model,
            "max_tokens": limit,
            "system": request.system,
            "messages": [{ "role": "user", "content": request.user }],
        });
        apply_params(&mut body, &self.provider);
        body
    }
}

impl Provider for Anthropic {
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
            &self.headers(),
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
        let payload = super::envelope::unwrap_payload(&value);

        let text: Vec<String> = payload
            .get("content")
            .and_then(|c| c.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
                    .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        if text.is_empty() {
            return Err(Error::ModelOutput(format!(
                "no text blocks in the response (top-level keys: [{}]).\n{}",
                super::envelope::top_level_keys(&value),
                super::mock::clip(&response.body)
            )));
        }

        Ok(LlmResponse {
            text: text.join("\n"),
            usage: payload
                .get("usage")
                .map(|u| Usage {
                    input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()),
                    output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()),
                })
                .unwrap_or_default(),
            truncated: payload.get("stop_reason").and_then(|v| v.as_str()) == Some("max_tokens"),
        })
    }

    fn list_models(&self) -> Result<Vec<String>> {
        let response = http::get_json(
            &self.provider.models_url(),
            &self.headers(),
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
        names.sort();
        Ok(names)
    }
}
