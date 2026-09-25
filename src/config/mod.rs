//! User configuration.
//!
//! Precedence, highest first: command-line flags, environment variables, this
//! file, built-in presets. Every field is optional so a config file only names
//! what it wants to change.

pub mod endpoint;
pub mod load;
pub mod presets;
pub mod resolve;
pub mod shape;
pub mod starter;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use resolve::ResolvedProvider;
pub use shape::Shape;
pub use starter::write_starter;

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod resolve_tests;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Provider used when `--provider` is not given.
    pub default_provider: Option<String>,
    /// `plain` or `conventional`. Omit to follow the repo's own history.
    pub style: Option<String>,
    /// `auto`, `en`, `ar`, `fr` ...
    pub language: Option<String>,
    /// How many recent commits to read for the style profile.
    pub history_window: Option<usize>,
    /// How many example messages to put in the prompt.
    pub exemplars: Option<usize>,
    pub max_subject_len: Option<usize>,
    /// Add a bullet body to the message. Off by default: single line.
    pub include_body: Option<bool>,
    pub max_diff_tokens: Option<usize>,
    pub context_max_chars: Option<usize>,
    /// Let the model fix its own output once if the validator rejects it.
    pub repair: Option<bool>,
    pub diff_context_lines: Option<u32>,
    pub cache: Option<bool>,
    /// Appended to the system prompt.
    pub prompt_extra: Option<String>,
    /// Extra arguments for `git commit` (e.g. `--signoff`).
    #[serde(default)]
    pub extra_commit_args: Vec<String>,
    /// Glob-lite patterns whose diffs are never sent (`*.lock`, `dist/*`).
    #[serde(default)]
    pub skip: Vec<String>,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub shape: Option<String>,
    pub base: Option<String>,
    pub model: Option<String>,
    /// Prefer `key_env` or `key_cmd`; this exists for throwaway setups.
    pub api_key: Option<String>,
    pub key_env: Option<String>,
    /// Shell command whose stdout is the key, e.g.
    /// `security find-generic-password -w -s wyckoff-claude`
    pub key_cmd: Option<String>,
    /// Anthropic API version header.
    pub api_version: Option<String>,
    pub json_mode: Option<bool>,
    pub max_output_tokens: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub label: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// Raw fields merged into the request body (temperature, top_p, ...).
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
}

pub const DEFAULT_HISTORY_WINDOW: usize = 40;
pub const DEFAULT_EXEMPLARS: usize = 3;
pub const DEFAULT_MAX_DIFF_TOKENS: usize = 24_000;

/// Effective settings after defaults are applied.
#[derive(Debug, Clone)]
pub struct Settings {
    pub default_provider: String,
    pub style: String,
    pub language: String,
    pub history_window: usize,
    pub exemplars: usize,
    pub max_subject_len: usize,
    pub include_body: bool,
    pub max_diff_tokens: usize,
    pub context_max_chars: usize,
    pub repair: bool,
    pub diff_context_lines: u32,
    pub cache: bool,
    pub prompt_extra: Option<String>,
    pub extra_commit_args: Vec<String>,
    pub skip: Vec<String>,
}

impl Settings {
    pub fn from_config(config: &Config) -> Self {
        let provider = config
            .default_provider
            .clone()
            .or_else(|| std::env::var("WYCKOFF_PROVIDER").ok())
            .unwrap_or_else(|| "gemini".to_string());

        Self {
            default_provider: provider,
            style: config.style.clone().unwrap_or_else(|| "auto".into()),
            language: config.language.clone().unwrap_or_else(|| "auto".into()),
            history_window: config.history_window.unwrap_or(DEFAULT_HISTORY_WINDOW),
            exemplars: config.exemplars.unwrap_or(DEFAULT_EXEMPLARS),
            max_subject_len: config
                .max_subject_len
                .unwrap_or(crate::msg::DEFAULT_MAX_SUBJECT),
            include_body: config.include_body.unwrap_or(false),
            max_diff_tokens: config.max_diff_tokens.unwrap_or(DEFAULT_MAX_DIFF_TOKENS),
            context_max_chars: config.context_max_chars.unwrap_or(6000),
            repair: config.repair.unwrap_or(true),
            diff_context_lines: config.diff_context_lines.unwrap_or(3),
            cache: config
                .cache
                .unwrap_or_else(|| std::env::var("WYCKOFF_NO_CACHE").is_err()),
            prompt_extra: config.prompt_extra.clone(),
            extra_commit_args: config.extra_commit_args.clone(),
            skip: config.skip.clone(),
        }
    }
}
