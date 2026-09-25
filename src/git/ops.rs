//! The git operations wyckoff actually needs, beyond the read-only basics.

use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

use super::{CommitRecord, DiffSource, Git, base_command, log};
use crate::error::{Error, Result};

/// Diff flags we always pass: no colour, no external diff driver, no textconv
/// filters, and rename detection on (so a moved file reads as a move, not as a
/// delete plus an add).
const DIFF_BASE: &[&str] = &[
    "-c",
    "core.pager=cat",
    "-c",
    "diff.external=",
    "-c",
    "core.quotepath=false",
    "diff",
    "--no-color",
    "--no-ext-diff",
    "--no-textconv",
    "--find-renames",
];

impl Git {
    pub fn diff(&self, source: DiffSource, context_lines: u32) -> Result<String> {
        let context = format!("-U{context_lines}");
        let mut args: Vec<&str> = DIFF_BASE.to_vec();
        args.extend_from_slice(source.flag());
        args.push(&context);
        self.run(&args)
    }

    /// Diff limited to `pathspecs` when non-empty (used by `-- <paths>`).
    pub fn diff_for(
        &self,
        source: DiffSource,
        context_lines: u32,
        pathspecs: &[String],
    ) -> Result<String> {
        if pathspecs.is_empty() {
            return self.diff(source, context_lines);
        }
        let mut args: Vec<String> = DIFF_BASE.iter().map(|s| s.to_string()).collect();
        args.extend(source.flag().iter().map(|s| s.to_string()));
        args.push(format!("-U{context_lines}"));
        args.push("--".to_string());
        args.extend(pathspecs.iter().cloned());
        self.run_paths(&args)
    }

    /// Recent non-merge commits with the paths they touched. A single process,
    /// so this stays cheap enough to run on every invocation.
    pub fn recent_commits(&self, count: usize) -> Result<Vec<CommitRecord>> {
        // A brand new repository has no HEAD, and `git log` fails on it. That is
        // not an error: it just means there is no house style to learn yet.
        if self.head_sha()?.is_empty() {
            return Ok(Vec::new());
        }
        let n = format!("-n{count}");
        let raw = self.run(&[
            "log",
            &n,
            "--no-merges",
            "--name-only",
            "--format=%x01%H%x02%s",
        ])?;
        Ok(log::parse(&raw))
    }

    pub fn ls_files(&self) -> Result<Vec<String>> {
        let raw = self.run(&["ls-files"])?;
        Ok(raw.lines().map(|l| l.to_string()).collect())
    }

    pub fn file_count_by_top_dir(&self) -> Result<Vec<(String, usize)>> {
        let files = self.ls_files()?;
        let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
        for path in files {
            let top = path.split('/').next().unwrap_or(&path).to_string();
            let key = if path.contains('/') {
                format!("{top}/")
            } else {
                "(root files)".to_string()
            };
            *counts.entry(key).or_insert(0) += 1;
        }
        let mut out: Vec<(String, usize)> = counts.into_iter().collect();
        out.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        Ok(out)
    }

    /// Merges, rebases, cherry-picks and reverts: cases where a generated
    /// message would be wrong or actively unhelpful.
    pub fn operation_in_progress(&self) -> Option<&'static str> {
        for (marker, label) in [
            ("MERGE_HEAD", "merge"),
            ("rebase-merge", "rebase"),
            ("rebase-apply", "rebase"),
            ("CHERRY_PICK_HEAD", "cherry-pick"),
            ("REVERT_HEAD", "revert"),
            ("BISECT_LOG", "bisect"),
        ] {
            if self.git_dir().join(marker).exists() {
                return Some(label);
            }
        }
        None
    }

    pub fn hook_path(&self, name: &str) -> PathBuf {
        // Respect core.hooksPath when it is set.
        if let Ok(custom) = self.run(&["config", "core.hooksPath"]) {
            let custom = custom.trim();
            if !custom.is_empty() {
                let path = PathBuf::from(custom);
                return if path.is_absolute() {
                    path.join(name)
                } else {
                    self.root().join(path).join(name)
                };
            }
        }
        self.git_dir().join("hooks").join(name)
    }

    pub fn commit(&self, message: &str, extra_args: &[String]) -> Result<()> {
        let mut command = base_command(Some(self.root()));
        command.args(["commit", "-F", "-"]);
        for arg in extra_args {
            command.arg(arg);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let mut child = command.spawn().map_err(|e| Error::msg(e.to_string()))?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(message.as_bytes())
                .map_err(|e| Error::msg(e.to_string()))?;
        }
        drop(child.stdin.take());
        let status = child.wait().map_err(|e| Error::msg(e.to_string()))?;
        if !status.success() {
            return Err(Error::msg("git commit failed"));
        }
        Ok(())
    }
}
