//! Writing a starter config.

use std::path::Path;

use crate::error::{Error, Result};

pub fn starter_config() -> String {
    String::from(
        r#"# wyckoff — commit messages written from your staged diff, in your own voice.
# Everything here is optional. Command-line flags override this file.

# gemini | gemini-lite | claude | openai | zen | nvidia | ollama | lmstudio
default_provider = "gemini"

# "auto" follows your repo's own history. "plain" always writes a bare
# sentence. "conventional" always uses type(scope): prefixes.
style = "auto"

# Message language: "auto" detects it from your commit history.
language = "auto"

# How much history to read for the style profile, and how many of those
# messages to show the model as examples.
history_window = 40
exemplars = 3

# Commit subjects are capped at this length.
max_subject_len = 120

# Single line by default. true allows a short bullet body.
include_body = false

# Diff budget in estimated tokens. Bigger diffs get condensed, never dumped.
max_diff_tokens = 24000

# Let the model fix its own output once if it breaks the rules.
repair = true

# Paths whose diff is never sent. "*" is a wildcard.
skip = ["*.lock", "*.min.js", "dist/*", "*.snap"]

# Extra arguments for `git commit`, e.g. ["--signoff"].
extra_commit_args = []

# --- Providers -------------------------------------------------------------
# Any provider can be overridden. Only the fields you set are changed.

# [providers.gemini]
# model  = "gemini-2.5-flash-lite"    # the cheapest option
# key_env = "GEMINI_API_KEY"          # default; export it in your shell

# [providers.claude]
# key_cmd = "security find-generic-password -w -s wyckoff-claude"

# [providers.nvidia]
# model = "openai/gpt-oss-120b"

# [providers.ollama]
# model = "qwen2.5-coder:7b"          # run `ollama list` to see what you have

# A provider that is not built in needs only these lines:
# [providers.my-gateway]
# shape   = "chat_completions"        # chat_completions | responses | anthropic_messages
# base    = "https://gateway.internal/v1"
# model   = "some-model"
# key_env = "MY_GATEWAY_KEY"
# [providers.my-gateway.params]       # merged into the request body as-is
# temperature = 0.2
"#,
    )
}

pub fn write_starter(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Err(Error::msg(format!(
            "{} already exists (pass --force to overwrite)",
            crate::paths::display(path)
        )));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::write(path, starter_config()).map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Settings};

    #[test]
    fn starter_config_is_valid_toml_and_parses() {
        let text = starter_config();
        let config: Config = toml::from_str(&text).expect("starter config must parse");
        let settings = Settings::from_config(&config);
        assert_eq!(settings.default_provider, "gemini");
        assert!(text.contains("key_cmd"));
        assert!(settings.skip.contains(&"*.lock".to_string()));
    }

    #[test]
    fn write_starter_refuses_to_clobber() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        write_starter(&path, false).unwrap();
        assert!(write_starter(&path, false).is_err());
        write_starter(&path, true).unwrap();
    }
}
