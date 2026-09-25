//! Talking to git.
//!
//! We shell out to the real `git` binary rather than linking libgit2/gix:
//! no C build dependency, and the diff we analyse is byte-for-byte the diff the
//! developer just looked at in their own tooling (their algorithm, their rename
//! detection). Every call pins the settings that would otherwise make output
//! interactive or non-deterministic.

mod log;
mod ops;

pub use log::CommitRecord;

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};

/// Which diff the user wants a message for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSource {
    /// `git diff --cached` (default): what a plain `git commit` would record.
    Staged,
    /// `git diff`: working tree versus index.
    Unstaged,
    /// `git diff HEAD`: everything since the last commit.
    Head,
}

impl DiffSource {
    pub fn flag(self) -> &'static [&'static str] {
        match self {
            DiffSource::Staged => &["--cached"],
            DiffSource::Unstaged => &[],
            DiffSource::Head => &["HEAD"],
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            DiffSource::Staged => "staged",
            DiffSource::Unstaged => "unstaged",
            DiffSource::Head => "everything since HEAD",
        }
    }
}

pub struct Git {
    root: PathBuf,
    git_dir: PathBuf,
}

impl Git {
    pub fn discover() -> Result<Self> {
        let root = run_git(None, &["rev-parse", "--show-toplevel"]).map_err(|_| Error::NotARepo)?;
        let root = root.trim().to_string();
        if root.is_empty() {
            return Err(Error::NotARepo);
        }
        let root = PathBuf::from(root);
        let git_dir = run_git(Some(&root), &["rev-parse", "--absolute-git-dir"])?;
        Ok(Self {
            git_dir: PathBuf::from(git_dir.trim()),
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// Run a git command in the repo root and return stdout.
    pub fn run(&self, args: &[&str]) -> Result<String> {
        run_git(Some(&self.root), args)
    }

    pub fn run_paths(&self, args: &[String]) -> Result<String> {
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.run(&refs)
    }

    pub fn branch(&self) -> Result<String> {
        // `--abbrev-ref` prints "HEAD" when detached and fails outright in a
        // repository with no commits yet; the symbolic ref covers both.
        match self.run(&["rev-parse", "--abbrev-ref", "HEAD"]) {
            Ok(name) => Ok(name.trim().to_string()),
            Err(_) => Ok(self
                .run(&["symbolic-ref", "--short", "-q", "HEAD"])
                .map(|name| name.trim().to_string())
                .unwrap_or_default()),
        }
    }

    pub fn head_sha(&self) -> Result<String> {
        match self.run(&["rev-parse", "HEAD"]) {
            Ok(sha) => Ok(sha.trim().to_string()),
            Err(_) => Ok(String::new()), // repo with no commits yet
        }
    }

    /// Cheap, side-effect-free staleness key for the index.
    pub fn index_fingerprint(&self) -> Result<String> {
        let raw = self.run(&["diff", "--cached", "--raw"])?;
        Ok(crate::paths::short_hash(&raw))
    }

    pub fn has_staged(&self) -> Result<bool> {
        Ok(!self.staged_paths()?.is_empty())
    }

    pub fn staged_paths(&self) -> Result<Vec<String>> {
        let raw = self.run(&["diff", "--cached", "--name-only"])?;
        Ok(raw
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }
}

fn base_command(dir: Option<&Path>) -> Command {
    let mut command = Command::new("git");
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    command
        .env("GIT_PAGER", "cat")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "0");
    command
}

pub(crate) fn run_git(dir: Option<&Path>, args: &[&str]) -> Result<String> {
    let output = base_command(dir)
        .arg("--no-pager")
        .args(args)
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Error::NotARepo,
            _ => Error::msg(e.to_string()),
        })?;

    if !output.status.success() {
        return Err(Error::Git {
            cmd: args.join(" "),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
