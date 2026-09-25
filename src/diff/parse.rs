//! The shape of a parsed diff.
//!
//! Downstream code (budgeting, classification, symbol extraction) works off
//! these structures instead of raw text, which keeps the rest of the pipeline
//! testable without a repo or a network.

use std::fmt::Write as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    Added,
    Deleted,
    #[default]
    Modified,
    Renamed,
    Copied,
    TypeChanged,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Added => "added",
            Status::Deleted => "deleted",
            Status::Modified => "modified",
            Status::Renamed => "renamed",
            Status::Copied => "copied",
            Status::TypeChanged => "type-changed",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Hunk {
    pub header: String,
    // Parsed for completeness: asserted on in tests, and the obvious hook for
    // line-numbered output later.
    #[allow(dead_code)]
    pub old_start: usize,
    #[allow(dead_code)]
    pub new_start: usize,
    /// Raw lines including their leading `+`, `-` or space.
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: Status,
    pub binary: bool,
    pub mode_changed: bool,
    pub submodule: bool,
    pub insertions: usize,
    pub deletions: usize,
    pub hunks: Vec<Hunk>,
    /// The exact text git produced for this file.
    pub raw: String,
}

impl FileDiff {
    pub fn display_name(&self) -> String {
        match (&self.old_path, self.status) {
            (Some(old), Status::Renamed | Status::Copied) if old != &self.path => {
                format!("{old} -> {}", self.path)
            }
            _ => self.path.clone(),
        }
    }

    pub fn changed_lines(&self) -> usize {
        self.insertions + self.deletions
    }

    /// Added content, without the leading `+`.
    pub fn added_lines(&self) -> Vec<&str> {
        self.lines_with('+')
    }

    /// Removed content, without the leading `-`.
    pub fn removed_lines(&self) -> Vec<&str> {
        self.lines_with('-')
    }

    fn lines_with(&self, marker: char) -> Vec<&str> {
        let mut out = Vec::new();
        for hunk in &self.hunks {
            for line in &hunk.lines {
                let mut chars = line.chars();
                if chars.next() == Some(marker) {
                    out.push(chars.as_str());
                }
            }
        }
        out
    }

    /// One-line summary used when a file's body is noise (lockfiles, bundles).
    pub fn summary_line(&self) -> String {
        let mut out = String::new();
        let _ = write!(out, "{} {}", self.status.as_str(), self.display_name());
        if self.binary {
            out.push_str(" (binary)");
        }
        if self.mode_changed {
            out.push_str(" (mode changed)");
        }
        if !self.binary && self.changed_lines() > 0 {
            let _ = write!(out, " (+{} -{})", self.insertions, self.deletions);
        }
        out
    }
}
