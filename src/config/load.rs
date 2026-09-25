//! Loading config from disk.

use std::path::{Path, PathBuf};

use super::{Config, presets};
use crate::error::{Error, Result};

impl Config {
    /// Load from `--config`, `$WYCKOFF_CONFIG`, or the default location.
    /// A missing file is not an error: presets and flags are enough to run.
    pub fn load(explicit: Option<&Path>) -> Result<(Self, PathBuf)> {
        let path = explicit
            .map(|p| p.to_path_buf())
            .unwrap_or_else(crate::paths::config_file);

        if !path.exists() {
            return Ok((Self::default(), path));
        }
        let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
        let config: Config = toml::from_str(&text).map_err(|e| Error::Config {
            path: path.clone(),
            msg: e.to_string(),
        })?;
        Ok((config, path))
    }

    /// Every provider name the user could ask for: built-ins plus their own.
    pub fn known_providers(&self) -> Vec<String> {
        let mut names: Vec<String> = presets::names().iter().map(|s| s.to_string()).collect();
        for name in self.providers.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        names.sort();
        names
    }
}
