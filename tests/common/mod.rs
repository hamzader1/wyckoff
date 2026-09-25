//! Shared helpers for the end-to-end tests.
//!
//! Every test gets a real git repository, the real binary and an isolated
//! config/cache directory, so nothing leaks in from the developer's machine and
//! nothing needs the network.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wyckoff"))
}

pub struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    pub fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo.git(&["config", "user.name", "Test"]);
        repo
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn write(&self, relative: &str, contents: &str) {
        let path = self.path().join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    pub fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.path().join(relative)).unwrap_or_default()
    }

    pub fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(self.path())
            .args(args)
            .output()
            .expect("git must be installed for these tests");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    /// Run wyckoff with an isolated config, cache and provider.
    pub fn wyckoff(&self, args: &[&str]) -> Output {
        Command::new(binary())
            .current_dir(self.path())
            .env("WYCKOFF_CONFIG", self.path().join("wyckoff.toml"))
            .env("WYCKOFF_CACHE", self.path().join("wyckoff-cache"))
            .env("WYCKOFF_PROVIDER", "mock")
            .env_remove("WYCKOFF_API_KEY")
            .env_remove("GEMINI_API_KEY")
            .env_remove("ANTHROPIC_API_KEY")
            .args(args)
            .output()
            .expect("the binary should be built by cargo test")
    }
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// A repo with one staged source file, a README and a `.context` file.
pub fn seeded_repo() -> Repo {
    let repo = Repo::new();
    repo.write(
        ".context",
        "# demo\n\nA tiny btree demo used by the tests.\n",
    );
    repo.write(
        "src/btree/page.rs",
        "pub fn cell_count(&self) -> usize {\n    0\n}\n",
    );
    repo.write("README.md", "# demo\n\nBtree experiment.\n");
    repo.git(&["add", "-A"]);
    repo
}
