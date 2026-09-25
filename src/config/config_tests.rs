//! Tests for settings defaults and config loading.

use super::*;
use std::io::Write;

fn write_config(text: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
    (dir, path)
}

#[test]
fn defaults_are_sane() {
    let settings = Settings::from_config(&Config::default());
    assert_eq!(settings.history_window, DEFAULT_HISTORY_WINDOW);
    assert_eq!(settings.exemplars, DEFAULT_EXEMPLARS);
    assert_eq!(settings.max_subject_len, 120);
    assert!(!settings.include_body, "single-line by default");
    assert_eq!(settings.style, "auto");
    assert_eq!(settings.max_diff_tokens, DEFAULT_MAX_DIFF_TOKENS);
    assert!(settings.cache);
    assert!(settings.repair);
}

#[test]
fn missing_config_file_is_not_an_error() {
    let path = std::path::Path::new("/tmp/wyckoff-does-not-exist-hopefully.toml");
    let (config, resolved_path) = Config::load(Some(path)).unwrap();
    assert!(config.providers.is_empty());
    assert!(resolved_path.ends_with("wyckoff-does-not-exist-hopefully.toml"));
}

#[test]
fn reads_values_from_a_file() {
    let (_dir, path) = write_config(
        "default_provider = \"nvidia\"\nmax_subject_len = 100\ninclude_body = true\nskip = [\"*.lock\"]\n\n[providers.gemini]\nmodel = \"gemini-2.5-flash-lite\"\napi_key = \"k\"\n",
    );
    let (config, _) = Config::load(Some(&path)).unwrap();
    let settings = Settings::from_config(&config);
    assert_eq!(settings.default_provider, "nvidia");
    assert_eq!(settings.max_subject_len, 100);
    assert!(settings.include_body);
    assert_eq!(settings.skip, vec!["*.lock"]);

    let provider = config.resolved("gemini", None).unwrap();
    assert_eq!(provider.model, "gemini-2.5-flash-lite");
}

#[test]
fn broken_config_reports_the_file() {
    let (_dir, path) = write_config("this is not = = toml\n");
    let error = Config::load(Some(&path)).unwrap_err();
    assert!(error.to_string().contains("config"), "{error}");
}

#[test]
fn known_providers_include_presets_and_user_entries() {
    let (_dir, path) = write_config(
        "[providers.mine]\nshape = \"chat_completions\"\nbase = \"https://x/v1\"\nmodel = \"m\"\napi_key = \"k\"\n",
    );
    let (config, _) = Config::load(Some(&path)).unwrap();
    let names = config.known_providers();
    assert!(names.contains(&"mine".to_string()));
    assert!(names.contains(&"gemini".to_string()));
    assert!(names.windows(2).all(|w| w[0] <= w[1]), "sorted for display");
}
