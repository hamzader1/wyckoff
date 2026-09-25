//! Path classification.
//!
//! The single biggest quality win in a commit-message tool is knowing what
//! *not* to look at. A lockfile bump, a rebuilt `dist/` bundle or a vendored
//! dependency can be most of the diff and none of the meaning, and stuffing it
//! into the prompt both costs money and degrades the output.

use super::tables::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Source,
    Test,
    Docs,
    Config,
    Ci,
    Lockfile,
    Generated,
    Asset,
    Vendored,
    Other,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Source => "source",
            Kind::Test => "test",
            Kind::Docs => "docs",
            Kind::Config => "config",
            Kind::Ci => "ci",
            Kind::Lockfile => "lockfile",
            Kind::Generated => "generated",
            Kind::Asset => "asset",
            Kind::Vendored => "vendored",
            Kind::Other => "other",
        }
    }

    /// Kinds whose *content* is never worth sending to a model.
    pub fn body_is_noise(self) -> bool {
        matches!(
            self,
            Kind::Lockfile | Kind::Generated | Kind::Vendored | Kind::Asset
        )
    }
}

pub fn classify(path: &str) -> Kind {
    let lower = path.to_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    let ext = lower.rsplit_once('.').map(|(_, e)| e).unwrap_or("");

    if in_any_dir(&lower, GENERATED_DIRS) {
        return Kind::Generated;
    }
    if in_any_dir(&lower, VENDOR_DIRS) {
        return Kind::Vendored;
    }
    if LOCKFILES.contains(&file) {
        return Kind::Lockfile;
    }
    if in_any_dir(&lower, CI_DIRS) || CI_FILES.contains(&file) {
        return Kind::Ci;
    }
    // Docs before config: `docs/config.txt` is docs, not a text config.
    if in_any_dir(&lower, DOC_DIRS)
        || DOC_EXTS.contains(&ext)
        || DOC_FILES.contains(&stem(file).as_str())
    {
        return Kind::Docs;
    }
    if is_test(&lower, file) {
        return Kind::Test;
    }
    if ASSET_EXTS.contains(&ext) {
        return Kind::Asset;
    }
    if CONFIG_FILES.contains(&file)
        || file.starts_with(".env")
        || CONFIG_EXTS.contains(&ext)
        || ext.starts_with("config.")
    {
        return Kind::Config;
    }
    if SOURCE_EXTS.contains(&ext) {
        return Kind::Source;
    }
    Kind::Other
}

fn stem(file: &str) -> String {
    match file.rsplit_once('.') {
        Some((head, _)) => head.to_string(),
        None => file.to_string(),
    }
}

fn in_any_dir(path: &str, dirs: &[&str]) -> bool {
    dirs.iter()
        .any(|d| path.starts_with(d) || (path.len() > d.len() && path.contains(&format!("/{d}"))))
}

fn is_test(path: &str, file: &str) -> bool {
    if in_any_dir(path, TEST_DIRS) {
        return true;
    }
    file.ends_with("_test.rs")
        || file.ends_with("_tests.rs")
        || file.ends_with("_test.go")
        || file.ends_with("_test.py")
        || file.ends_with("_spec.rb")
        || file.ends_with(".test.ts")
        || file.ends_with(".test.tsx")
        || file.ends_with(".test.js")
        || file.ends_with(".spec.ts")
        || file.ends_with(".spec.js")
        || file.contains(".test.")
        || file.starts_with("test_")
        || file == "conftest.py"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_the_obvious_cases() {
        assert_eq!(classify("src/diff/classify.rs"), Kind::Source);
        assert_eq!(classify("tests/parser.rs"), Kind::Test);
        assert_eq!(classify("src/diff/parser_test.rs"), Kind::Test);
        assert_eq!(classify("Cargo.lock"), Kind::Lockfile);
        assert_eq!(classify("docs/design.md"), Kind::Docs);
        assert_eq!(classify("README.md"), Kind::Docs);
        assert_eq!(classify(".github/workflows/ci.yml"), Kind::Ci);
        assert_eq!(classify("target/debug/wyckoff"), Kind::Generated);
        assert_eq!(classify("node_modules/left-pad/index.js"), Kind::Vendored);
        assert_eq!(classify("assets/logo.svg"), Kind::Asset);
        assert_eq!(classify("Cargo.toml"), Kind::Config);
        assert_eq!(classify("Dockerfile"), Kind::Config);
        assert_eq!(classify("build.rs"), Kind::Source);
    }

    #[test]
    fn docs_beat_config_but_config_dir_wins_over_extension() {
        assert_eq!(classify("docs/config.txt"), Kind::Docs);
        assert_eq!(classify("config/app.yaml"), Kind::Config);
    }

    #[test]
    fn nested_noise_dirs_are_still_noise() {
        assert_eq!(classify("web/app/dist/bundle.js"), Kind::Generated);
        assert_eq!(classify("crates/foo/generated/schema.rs"), Kind::Generated);
    }

    #[test]
    fn noise_kinds_are_not_sent_to_models() {
        assert!(Kind::Lockfile.body_is_noise());
        assert!(Kind::Generated.body_is_noise());
        assert!(Kind::Vendored.body_is_noise());
        assert!(Kind::Asset.body_is_noise());
        assert!(!Kind::Source.body_is_noise());
        assert!(!Kind::Test.body_is_noise());
        assert!(!Kind::Config.body_is_noise());
    }
}
