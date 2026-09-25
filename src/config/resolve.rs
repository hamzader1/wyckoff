//! Turning a provider name plus config into something we can call.

use super::{Config, ProviderConfig, Shape, presets};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct ResolvedProvider {
    pub name: String,
    pub label: String,
    pub shape: Shape,
    pub base: String,
    pub model: String,
    pub api_key: Option<String>,
    pub key_env: String,
    pub api_version: String,
    pub json_mode: bool,
    pub max_output_tokens: Option<u32>,
    pub timeout_secs: u64,
    pub headers: std::collections::BTreeMap<String, String>,
    pub params: serde_json::Map<String, serde_json::Value>,
}

impl Config {
    /// Merge presets, user config and an optional `--model` override.
    pub fn resolved(&self, name: &str, model_override: Option<&str>) -> Result<ResolvedProvider> {
        let preset = presets::find(name);
        let user = self.providers.get(name);

        if preset.is_none() && user.is_none() {
            return Err(Error::UnknownProvider {
                name: name.to_string(),
                known: self.known_providers().join(", "),
            });
        }

        let shape = user
            .and_then(|c| c.shape.as_deref())
            .and_then(Shape::parse)
            .or_else(|| preset.map(|p| p.shape))
            .unwrap_or(Shape::ChatCompletions);

        let base = user
            .and_then(|c| c.base.clone())
            .or_else(|| preset.map(|p| p.base.to_string()))
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_string();

        let model = model_override
            .map(|m| m.to_string())
            .or_else(|| user.and_then(|c| c.model.clone()))
            .or_else(|| preset.map(|p| p.model.to_string()))
            .unwrap_or_default();

        let key_env = user
            .and_then(|c| c.key_env.clone())
            .or_else(|| preset.map(|p| p.key_env.to_string()))
            .unwrap_or_else(|| format!("{}_API_KEY", name.to_uppercase().replace('-', "_")));

        let resolved = ResolvedProvider {
            name: name.to_string(),
            label: user
                .and_then(|c| c.label.clone())
                .or_else(|| preset.map(|p| p.label.to_string()))
                .unwrap_or_else(|| name.to_string()),
            shape,
            base,
            model,
            api_key: resolve_key(user, &key_env),
            key_env,
            api_version: user
                .and_then(|c| c.api_version.clone())
                .unwrap_or_else(|| "2023-06-01".to_string()),
            json_mode: user
                .and_then(|c| c.json_mode)
                .or_else(|| preset.map(|p| p.json_mode))
                .unwrap_or(false),
            max_output_tokens: user.and_then(|c| c.max_output_tokens),
            timeout_secs: user.and_then(|c| c.timeout_secs).unwrap_or(120),
            headers: user.map(|c| c.headers.clone()).unwrap_or_default(),
            params: user.map(|c| c.params.clone()).unwrap_or_default(),
        };

        validate(&resolved, preset.map(|p| p.needs_key).unwrap_or(true))?;
        Ok(resolved)
    }
}

/// `api_key` in config, then `$KEY_ENV` (or `$WYCKOFF_API_KEY`), then `key_cmd`.
fn resolve_key(user: Option<&ProviderConfig>, key_env: &str) -> Option<String> {
    if let Some(key) = user.and_then(|c| c.api_key.clone())
        && !key.trim().is_empty()
    {
        return Some(key.trim().to_string());
    }

    for var in [key_env, "WYCKOFF_API_KEY"] {
        if let Ok(value) = std::env::var(var)
            && !value.trim().is_empty()
        {
            return Some(value.trim().to_string());
        }
    }

    let command = user.and_then(|c| c.key_cmd.clone())?;
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    let output = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", command])
            .output()
    } else {
        std::process::Command::new("sh")
            .args(["-c", command])
            .output()
    }
    .ok()?;

    if !output.status.success() {
        return None;
    }
    let key = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if key.is_empty() { None } else { Some(key) }
}

fn validate(provider: &ResolvedProvider, needs_key: bool) -> Result<()> {
    if provider.shape == Shape::Mock {
        return Ok(());
    }
    if provider.base.is_empty() {
        return Err(Error::ProviderField {
            name: provider.name.clone(),
            field: "base",
        });
    }
    if provider.model.trim().is_empty() {
        return Err(Error::msg(format!(
            "provider `{}` needs a model.\n  \
             pass --model <id>, or set `model` under [providers.{}] in {}\n  \
             local servers: `wyckoff models --provider {}` lists what is loaded",
            provider.name,
            provider.name,
            crate::paths::display(&crate::paths::config_file()),
            provider.name
        )));
    }
    if needs_key && provider.api_key.is_none() {
        return Err(Error::NoKey {
            name: provider.name.clone(),
            env: provider.key_env.clone(),
            config: crate::paths::display(&crate::paths::config_file()),
        });
    }
    Ok(())
}
