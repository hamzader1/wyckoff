//! Turning CLI flags plus config into the options the pipeline runs on.

use std::path::PathBuf;

use crate::config::Settings;
use crate::git::DiffSource;

use super::Mode;

#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    pub provider: String,
    pub model: Option<String>,
    pub style: Option<String>,
    pub language: Option<String>,
    pub intent: Option<String>,
    pub include_body: bool,
    pub max_subject_len: usize,
    pub diff_source: DiffSource,
    pub paths: Vec<String>,
    pub context_file: Option<PathBuf>,
    pub cache: bool,
    pub dry_run: bool,
    pub allow_secrets: bool,
    pub exclude: Vec<String>,
    pub max_diff_tokens: usize,
    pub context_max_chars: usize,
    pub history_window: usize,
    pub exemplars: usize,
    pub repair: bool,
    pub diff_context_lines: u32,
    pub prompt_extra: Option<String>,
}

impl Options {
    /// Flags win over config, which wins over defaults.
    pub fn from_args(args: &crate::cli::GenerateArgs, settings: &Settings) -> Self {
        let diff_source = if args.everything {
            DiffSource::Head
        } else if args.unstaged {
            DiffSource::Unstaged
        } else {
            DiffSource::Staged
        };

        Self {
            mode: Mode::Message,
            provider: args
                .provider
                .clone()
                .unwrap_or_else(|| settings.default_provider.clone()),
            model: args.model.clone(),
            style: Some(
                args.style
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_else(|| settings.style.clone()),
            ),
            language: Some(
                args.language
                    .clone()
                    .unwrap_or_else(|| settings.language.clone()),
            ),
            intent: args.intent.clone(),
            include_body: args.body || settings.include_body,
            max_subject_len: args.max_len.unwrap_or(settings.max_subject_len),
            diff_source,
            paths: args.paths.clone(),
            context_file: args.context_file.clone(),
            cache: settings.cache && !args.no_cache && !args.refresh,
            dry_run: args.dry_run,
            allow_secrets: args.allow_secrets,
            exclude: args.exclude.clone(),
            max_diff_tokens: settings.max_diff_tokens,
            context_max_chars: settings.context_max_chars,
            history_window: settings.history_window,
            exemplars: settings.exemplars,
            repair: settings.repair,
            diff_context_lines: settings.diff_context_lines,
            prompt_extra: settings.prompt_extra.clone(),
        }
    }

    /// The style decision, honouring an explicit `--style`.
    pub fn forced_style(&self) -> Option<crate::style::Style> {
        match self.style.as_deref() {
            Some("plain") => Some(crate::style::Style::Plain),
            Some("conventional") => Some(crate::style::Style::Conventional),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{GenerateArgs, StyleChoice};
    use crate::config::Config;

    fn settings() -> Settings {
        Settings::from_config(&Config::default())
    }

    #[test]
    fn flags_beat_config() {
        let args = GenerateArgs {
            provider: Some("nvidia".into()),
            model: Some("openai/gpt-oss-120b".into()),
            max_len: Some(80),
            body: true,
            intent: Some("fix the race".into()),
            no_cache: true,
            ..Default::default()
        };
        let options = Options::from_args(&args, &settings());
        assert_eq!(options.provider, "nvidia");
        assert_eq!(options.model.as_deref(), Some("openai/gpt-oss-120b"));
        assert_eq!(options.max_subject_len, 80);
        assert!(options.include_body);
        assert!(!options.cache, "--no-cache wins");
        assert_eq!(options.intent.as_deref(), Some("fix the race"));
    }

    #[test]
    fn provider_falls_back_to_config_default() {
        let options = Options::from_args(&GenerateArgs::default(), &settings());
        assert_eq!(options.provider, "gemini");
        assert_eq!(options.max_subject_len, 120);
        assert!(!options.include_body);
    }

    #[test]
    fn diff_source_flags() {
        let options = Options::from_args(
            &GenerateArgs {
                everything: true,
                ..Default::default()
            },
            &settings(),
        );
        assert_eq!(options.diff_source, DiffSource::Head);

        let options = Options::from_args(
            &GenerateArgs {
                unstaged: true,
                ..Default::default()
            },
            &settings(),
        );
        assert_eq!(options.diff_source, DiffSource::Unstaged);

        let options = Options::from_args(&GenerateArgs::default(), &settings());
        assert_eq!(options.diff_source, DiffSource::Staged);
    }

    #[test]
    fn style_override_is_interpreted() {
        let options = Options::from_args(
            &GenerateArgs {
                style: Some(StyleChoice::Plain),
                ..Default::default()
            },
            &settings(),
        );
        assert_eq!(options.forced_style(), Some(crate::style::Style::Plain));

        let options = Options::from_args(
            &GenerateArgs {
                style: Some(StyleChoice::Conventional),
                ..Default::default()
            },
            &settings(),
        );
        assert_eq!(
            options.forced_style(),
            Some(crate::style::Style::Conventional)
        );

        let options = Options::from_args(&GenerateArgs::default(), &settings());
        assert_eq!(options.forced_style(), None, "auto lets the history decide");
    }
}
