//! Providers: one trait, three wire shapes, one mock.

pub mod anthropic;
pub mod envelope;
#[path = "http_client.rs"]
pub mod http;
pub mod mock;
pub mod openai;

use crate::config::{ResolvedProvider, Shape};
use crate::error::Result;
use crate::prompt::Request;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub text: String,
    pub usage: Usage,
    /// The model hit its output limit, so the answer may be cut short.
    pub truncated: bool,
}

impl LlmResponse {
    pub fn summary(&self) -> String {
        match (self.usage.input_tokens, self.usage.output_tokens) {
            (Some(input), Some(output)) => format!("{input} in / {output} out tokens"),
            _ => "usage not reported".to_string(),
        }
    }
}

pub trait Provider {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    /// Send the prompt and return raw model text.
    fn complete(&self, request: &Request) -> Result<LlmResponse>;
    /// Ask the endpoint what it serves; used by `wyckoff models` and `doctor`.
    fn list_models(&self) -> Result<Vec<String>>;
}

/// Build the provider for a resolved config.
pub fn build(provider: &ResolvedProvider) -> Box<dyn Provider> {
    match provider.shape {
        Shape::AnthropicMessages => Box::new(anthropic::Anthropic::new(provider.clone())),
        Shape::Mock => Box::new(mock::Mock::new(provider.clone())),
        // Both OpenAI surfaces live in one implementation: same auth, same
        // response shapes to dig through, different request body.
        Shape::ChatCompletions | Shape::Responses => {
            Box::new(openai::OpenAi::new(provider.clone()))
        }
    }
}

/// Headers every OpenAI-shaped request needs.
pub(crate) fn bearer_headers(provider: &ResolvedProvider) -> Vec<(String, String)> {
    let mut headers = vec![
        ("content-type".to_string(), "application/json".to_string()),
        ("accept".to_string(), "application/json".to_string()),
    ];
    if let Some(key) = &provider.api_key {
        headers.push(("authorization".to_string(), format!("Bearer {key}")));
    }
    for (name, value) in &provider.headers {
        headers.push((name.clone(), value.clone()));
    }
    headers
}

/// Merge the user's `params` table into a request body.
pub(crate) fn apply_params(body: &mut serde_json::Value, provider: &ResolvedProvider) {
    let Some(object) = body.as_object_mut() else {
        return;
    };
    for (key, value) in &provider.params {
        object.insert(key.clone(), value.clone());
    }
}
