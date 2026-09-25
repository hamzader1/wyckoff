//! Where wyckoff keeps its config, cache and repo state.
//!
//! Deliberately env-var based instead of pulling in a platform crate: it keeps
//! the dependency tree tiny and lets people move things around with
//! `XDG_CONFIG_HOME` / `XDG_CACHE_HOME` / `WYCKOFF_*` without surprises.

use std::path::PathBuf;

pub const APP: &str = "wyckoff";

pub fn env_path(var: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn home() -> Option<PathBuf> {
    if cfg!(windows) {
        env_path("USERPROFILE").or_else(|| env_path("HOME"))
    } else {
        env_path("HOME")
    }
}

/// `$WYCKOFF_CONFIG` > `$XDG_CONFIG_HOME/wyckoff/config.toml` >
/// `~/.config/wyckoff/config.toml` (Windows: `%APPDATA%\wyckoff\config.toml`).
pub fn config_file() -> PathBuf {
    if let Some(p) = env_path("WYCKOFF_CONFIG") {
        return p;
    }
    if let Some(dir) = env_path("XDG_CONFIG_HOME") {
        return dir.join(APP).join("config.toml");
    }
    if cfg!(windows)
        && let Some(appdata) = env_path("APPDATA")
    {
        return appdata.join(APP).join("config.toml");
    }
    home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join(APP)
        .join("config.toml")
}

/// `$WYCKOFF_CACHE` > `$XDG_CACHE_HOME/wyckoff` > `~/.cache/wyckoff`.
pub fn cache_dir() -> PathBuf {
    if let Some(p) = env_path("WYCKOFF_CACHE") {
        return p;
    }
    if let Some(dir) = env_path("XDG_CACHE_HOME") {
        return dir.join(APP);
    }
    if cfg!(windows)
        && let Some(local) = env_path("LOCALAPPDATA")
    {
        return local.join(APP).join("cache");
    }
    home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache")
        .join(APP)
}

/// Stable short name for a path, used to namespace cache entries per repo.
pub fn short_hash(text: &str) -> String {
    let digest = blake3::hash(text.as_bytes());
    digest.to_hex()[..16].to_string()
}

/// `~`-shortened display form.
pub fn display(path: &std::path::Path) -> String {
    if let Some(home) = home()
        && let Ok(rest) = path.strip_prefix(&home)
    {
        return format!("~/{}", rest.display());
    }
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_hash_is_stable_and_short() {
        let a = short_hash("/tmp/some/repo");
        let b = short_hash("/tmp/some/repo");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
        assert_ne!(a, short_hash("/tmp/other/repo"));
    }
}
