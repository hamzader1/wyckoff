//! Everything we know about the project that is *not* the diff.
//!
//! Layered by how fast it changes:
//!
//!   - `.context` files and manifests: effectively static, cached forever
//!   - repo layout: changes when directories come and go
//!
//! All of it is derived from the working tree, so it can be recomputed safely.

pub mod files;
pub mod manifest;
pub mod pyproject;
pub mod signature;

pub use files::ContextFile;

use crate::error::Result;
use crate::git::Git;
use crate::util;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectContext {
    pub context_files: Vec<ContextFile>,
    pub manifests: Vec<(String, String)>,
    pub layout: String,
    pub file_count: usize,
    /// Extra `.context` path from `--context-file`.
    #[serde(default)]
    pub extra: Option<ContextFile>,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub max_chars_per_file: usize,
    pub extra_context_file: Option<std::path::PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_chars_per_file: 6000,
            extra_context_file: None,
        }
    }
}

impl ProjectContext {
    pub fn collect(git: &Git, staged_paths: &[String], options: &Options) -> Result<Self> {
        let root = git.root();
        let context_files = files::discover(root, staged_paths, options.max_chars_per_file);
        let manifests = manifest::collect(root);
        let tree = git.file_count_by_top_dir()?;
        let all_files = git.ls_files()?;

        let layout = describe_layout(&tree, &all_files);
        let extra = options
            .extra_context_file
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .map(|body| ContextFile {
                path: options
                    .extra_context_file
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                body: body.trim().to_string(),
            });

        Ok(Self {
            context_files,
            manifests,
            layout,
            file_count: all_files.len(),
            extra,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.context_files.is_empty()
            && self.manifests.is_empty()
            && self.layout.is_empty()
            && self.extra.is_none()
    }

    pub fn render(&self) -> String {
        let mut out = String::new();

        if let Some(extra) = &self.extra {
            out.push_str(&format!("### Project notes (from {})\n", extra.path));
            out.push_str(&extra.body);
            out.push_str("\n\n");
        }

        for file in &self.context_files {
            out.push_str(&format!("### Project notes (from {})\n", file.path));
            out.push_str(&file.body);
            out.push_str("\n\n");
        }

        if !self.manifests.is_empty() {
            out.push_str("### Manifests\n");
            for (name, facts) in &self.manifests {
                out.push_str(&format!("- {name}: {facts}\n"));
            }
            out.push('\n');
        }

        if !self.layout.is_empty() {
            out.push_str("### Repo layout\n");
            out.push_str(&self.layout);
            out.push('\n');
        }

        out.trim_end().to_string()
    }
}

fn describe_layout(tree: &[(String, usize)], all_files: &[String]) -> String {
    if tree.is_empty() {
        return String::new();
    }
    let dirs: Vec<String> = tree
        .iter()
        .take(12)
        .map(|(dir, count)| format!("{dir} ({count})"))
        .collect();

    let mut extensions: std::collections::BTreeMap<&str, usize> = Default::default();
    for path in all_files {
        let base = path.rsplit('/').next().unwrap_or(path);
        if let Some((_, ext)) = base.rsplit_once('.') {
            *extensions.entry(ext).or_insert(0) += 1;
        }
    }
    let mut ext_list: Vec<(&str, usize)> = extensions.into_iter().collect();
    ext_list.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let exts: Vec<String> = ext_list
        .iter()
        .take(10)
        .map(|(ext, count)| format!("{ext} ({count})"))
        .collect();

    let mut out = format!(
        "{} tracked files. Top-level: {}. File types: {}",
        all_files.len(),
        dirs.join(", "),
        exts.join(", ")
    );
    out = util::truncate(&out, 900);
    out
}
