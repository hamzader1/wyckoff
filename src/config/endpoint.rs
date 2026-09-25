//! Endpoint helpers and the display form of a provider.

use super::{ResolvedProvider, Shape};

impl ResolvedProvider {
    /// `{base}/models` — every shape we support exposes it, which is what makes
    /// `wyckoff models` and `wyckoff doctor` provider-agnostic.
    pub fn models_url(&self) -> String {
        format!("{}/models", self.base)
    }

    /// The URL a completion request goes to, per shape.
    pub fn chat_url(&self) -> String {
        match self.shape {
            Shape::Responses => format!("{}/responses", self.base),
            Shape::AnthropicMessages => format!("{}/messages", self.base),
            _ => format!("{}/chat/completions", self.base),
        }
    }

    /// Safe for logs: never includes the key itself.
    pub fn describe(&self) -> String {
        format!(
            "{} ({}, shape {}, model {}, key {})",
            self.name,
            self.label,
            self.shape.as_str(),
            self.model,
            if self.api_key.is_some() {
                "set"
            } else {
                "missing"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ProviderConfig};

    fn provider(shape: &str, base: &str) -> ResolvedProvider {
        let mut config = Config::default();
        config.providers.insert(
            "x".into(),
            ProviderConfig {
                shape: Some(shape.into()),
                base: Some(base.into()),
                model: Some("m".into()),
                api_key: Some("secret-value-here".into()),
                ..Default::default()
            },
        );
        config.resolved("x", None).unwrap()
    }

    #[test]
    fn urls_per_shape() {
        assert_eq!(
            provider("chat_completions", "https://a/v1").chat_url(),
            "https://a/v1/chat/completions"
        );
        assert_eq!(
            provider("responses", "https://a/v1").chat_url(),
            "https://a/v1/responses"
        );
        assert_eq!(
            provider("anthropic_messages", "https://a/v1").chat_url(),
            "https://a/v1/messages"
        );
        assert_eq!(
            provider("chat_completions", "https://a/v1").models_url(),
            "https://a/v1/models"
        );
    }

    #[test]
    fn describe_never_leaks_the_key() {
        let described = provider("chat_completions", "https://a/v1").describe();
        assert!(!described.contains("secret-value-here"), "{described}");
        assert!(described.contains("key set"), "{described}");
    }
}
