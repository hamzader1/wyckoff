//! A cheap fingerprint of everything static we read.
//!
//! Reading a few small files is not slow, but the *result* is what gets cached,
//! and we want editing `.context` (or `Cargo.toml`) to invalidate that cache
//! immediately. Stat-only: size plus mtime, no file contents, no git calls.

use std::path::{Path, PathBuf};

use crate::paths;

const ROOT_CANDIDATES: &[&str] = &[
    ".context",
    ".context.md",
    ".context.txt",
    "AGENTS.md",
    "CLAUDE.md",
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "go.mod",
];

pub fn of(
    root: &Path,
    staged_paths: &[String],
    extra: Option<&Path>,
    index_fingerprint: &str,
) -> String {
    let mut entries: Vec<String> = Vec::new();

    for name in ROOT_CANDIDATES {
        push_stat(&mut entries, root, name);
    }

    // `.context/` directory: every file in it counts.
    let context_dir = root.join(".context");
    if context_dir.is_dir()
        && let Ok(files) = std::fs::read_dir(&context_dir)
    {
        let mut names: Vec<PathBuf> = files.flatten().map(|entry| entry.path()).collect();
        names.sort();
        for path in names {
            if path.is_file()
                && let Some(name) = path
                    .strip_prefix(root)
                    .ok()
                    .map(|p| p.display().to_string())
            {
                push_stat(&mut entries, root, &name);
            }
        }
    }

    // Nearest `.context` next to the code being changed.
    for staged in staged_paths {
        let mut dir = match staged.rsplit_once('/') {
            Some((dir, _)) => dir.to_string(),
            None => continue,
        };
        for _ in 0..4 {
            for name in [".context", ".context.md"] {
                push_stat(&mut entries, root, &format!("{dir}/{name}"));
            }
            match dir.rsplit_once('/') {
                Some((parent, _)) => dir = parent.to_string(),
                None => break,
            }
        }
    }

    if let Some(path) = extra {
        match std::fs::metadata(path) {
            Ok(meta) => entries.push(format!(
                "{}:{}:{}",
                path.display(),
                meta.len(),
                modified(meta)
            )),
            Err(_) => entries.push(format!("{}:missing", path.display())),
        }
    }

    entries.push(format!("index:{index_fingerprint}"));
    paths::short_hash(&entries.join("|"))
}

fn push_stat(entries: &mut Vec<String>, root: &Path, name: &str) {
    match std::fs::metadata(root.join(name)) {
        Ok(meta) if meta.is_file() => {
            entries.push(format!("{name}:{}:{}", meta.len(), modified(meta)))
        }
        _ => entries.push(format!("{name}:absent")),
    }
}

fn modified(meta: std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_inputs_give_the_same_signature() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".context"), "project notes").unwrap();
        let staged = vec!["src/a.rs".to_string()];

        let first = of(dir.path(), &staged, None, "idx");
        let second = of(dir.path(), &staged, None, "idx");
        assert_eq!(first, second);
        assert_eq!(first.len(), 16);
    }

    #[test]
    fn editing_context_changes_the_signature() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".context"), "one").unwrap();
        let before = of(dir.path(), &[], None, "idx");

        // Different length guarantees a different stat, even within the same
        // filesystem timestamp granularity.
        std::fs::write(dir.path().join(".context"), "one and a lot more text").unwrap();
        let after = of(dir.path(), &[], None, "idx");
        assert_ne!(before, after);
    }

    #[test]
    fn index_changes_are_visible() {
        let dir = tempfile::tempdir().unwrap();
        assert_ne!(
            of(dir.path(), &[], None, "idx-a"),
            of(dir.path(), &[], None, "idx-b")
        );
    }

    #[test]
    fn nested_context_files_count() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/diff")).unwrap();
        let staged = vec!["src/diff/parse.rs".to_string()];
        let without = of(dir.path(), &staged, None, "idx");

        std::fs::write(dir.path().join("src/diff/.context"), "diff notes").unwrap();
        let with = of(dir.path(), &staged, None, "idx");
        assert_ne!(without, with);
    }
}
