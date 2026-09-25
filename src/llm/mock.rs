//! The mock provider: canned output, no network.
//!
//! Everything in the test suite and every `--dry-run` goes through here, which
//! means the whole pipeline (prompt building, JSON parsing, validation, repair,
//! rendering, output) is exercised exactly as it is in production.

use super::{LlmResponse, Provider, Usage};
use crate::config::ResolvedProvider;
use crate::error::Result;
use crate::prompt::Request;

pub struct Mock {
    provider: ResolvedProvider,
}

impl Mock {
    pub fn new(provider: ResolvedProvider) -> Self {
        Self { provider }
    }

    /// `$WYCKOFF_MOCK_RESPONSE` may be a path to a file or raw text.
    fn canned(&self, request: &Request) -> String {
        match std::env::var("WYCKOFF_MOCK_RESPONSE") {
            Ok(value) if !value.trim().is_empty() => {
                let path = std::path::Path::new(value.trim());
                if path.is_file() {
                    std::fs::read_to_string(path).unwrap_or_default()
                } else {
                    value
                }
            }
            _ => {
                // A plausible answer derived from the prompt, so dry runs and
                // integration tests get something stable and inspectable.
                let subject = derive_subject(request);
                serde_json::json!({
                    "subject": subject,
                    "body": [],
                    "breaking": false,
                    "footers": [],
                    "confidence": 0.5,
                    "notes_to_user": "mock provider: not a real model"
                })
                .to_string()
            }
        }
    }
}

/// Pull a rough subject out of the prompt so the mock output is not useless.
fn derive_subject(request: &Request) -> String {
    let mut files: Vec<String> = Vec::new();
    for line in request.user.lines() {
        if let Some(rest) = line.strip_prefix("- modified ") {
            files.push(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("- added ") {
            files.push(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("diff --git ")
            && let Some(path) = rest.split_whitespace().last()
        {
            files.push(path.trim_start_matches("b/").to_string());
        }
    }

    match files.first() {
        Some(path) => format!(
            "Update {} and the code around it",
            crate::util::short_name(path)
        ),
        None => "Update the staged changes".to_string(),
    }
}

impl Provider for Mock {
    fn name(&self) -> &str {
        &self.provider.name
    }

    fn model(&self) -> &str {
        &self.provider.model
    }

    fn complete(&self, request: &Request) -> Result<LlmResponse> {
        if std::env::var("WYCKOFF_MOCK_FAIL").is_ok() {
            return Err(crate::error::Error::ProviderHttp {
                provider: "mock".into(),
                status: 500,
                body: "mock failure requested by WYCKOFF_MOCK_FAIL".into(),
            });
        }
        Ok(LlmResponse {
            text: self.canned(request),
            usage: Usage {
                input_tokens: Some((request.system.len() / 4) as u64),
                output_tokens: Some(20),
            },
            truncated: false,
        })
    }

    fn list_models(&self) -> Result<Vec<String>> {
        Ok(vec!["mock".to_string()])
    }
}

/// Used by providers to keep error messages readable.
pub fn clip(text: &str) -> String {
    let trimmed = text.trim();
    let cut: String = trimmed.chars().take(600).collect();
    if trimmed.chars().count() > 600 {
        format!("{cut}…")
    } else {
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Request {
        Request {
            system: "rules".into(),
            user: "## Staged change\n1 file, +1 -0\n- added src/diff/parse.rs (+1 -0)\n".into(),
            max_output_tokens: 700,
            wants_json: true,
        }
    }

    #[test]
    fn default_output_is_valid_json_with_a_plausible_subject() {
        let mock = Mock::new(mock_provider());
        let response = mock.complete(&request()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&response.text).unwrap();
        let subject = value["subject"].as_str().unwrap();
        assert!(subject.contains("parse"), "{subject}");
        assert!(!subject.contains(':'), "{subject}");
    }

    #[test]
    fn lists_itself() {
        let mock = Mock::new(mock_provider());
        assert_eq!(mock.list_models().unwrap(), vec!["mock"]);
    }

    fn mock_provider() -> ResolvedProvider {
        ResolvedProvider {
            name: "mock".into(),
            label: "Mock".into(),
            shape: crate::config::Shape::Mock,
            base: String::new(),
            model: "mock".into(),
            api_key: None,
            key_env: "WYCKOFF_MOCK_RESPONSE".into(),
            api_version: String::new(),
            json_mode: false,
            max_output_tokens: None,
            timeout_secs: 5,
            headers: Default::default(),
            params: Default::default(),
        }
    }
}
