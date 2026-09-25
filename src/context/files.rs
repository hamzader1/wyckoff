//! Finding and reading `.context` files.
//!
//! A hand-written description of the project is the single highest-value input
//! this tool has (the diff says *what*, only the project says *why*), but a
//! single root file does not scale past a toy repo. So we look in three places:
//!
//! 1. the repo root (`.context`, `.context.md`, `.context/`)
//! 2. the nearest ancestors of each staged file (`.context` next to the code)
//! 3. `AGENTS.md` / `CLAUDE.md`, which many repos already have and which mean
//!    roughly the same thing

use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ContextFile {
    pub path: String,
    pub body: String,
}

const ROOT_CANDIDATES: &[&str] = &[".context", ".context.md", ".context.txt"];
/// Fallbacks only consulted when no `.context` exists anywhere.
const AGENT_CANDIDATES: &[&str] = &["AGENTS.md", "CLAUDE.md"];
const NESTED_MAX_DEPTH: usize = 4;

pub fn discover(root: &Path, staged_paths: &[String], max_chars: usize) -> Vec<ContextFile> {
    let mut found: Vec<ContextFile> = Vec::new();

    for name in ROOT_CANDIDATES {
        push_file(&mut found, root, name, max_chars);
        let dir = root.join(name);
        if dir.is_dir() {
            push_dir(&mut found, root, &dir, max_chars);
        }
    }
    if found.is_empty() {
        for name in AGENT_CANDIDATES {
            push_file(&mut found, root, name, max_chars);
        }
    }

    // Nearest-ancestor files, deepest first, so the most specific context wins
    // the limited prompt budget.
    let mut nested: Vec<(usize, ContextFile)> = Vec::new();
    for staged in staged_paths {
        let mut dir = match staged.rsplit_once('/') {
            Some((dir, _)) => dir.to_string(),
            None => continue,
        };
        for depth in 0..NESTED_MAX_DEPTH {
            let candidate_dir = root.join(&dir);
            for name in [".context", ".context.md"] {
                let path = candidate_dir.join(name);
                if path.is_file()
                    && let Ok(body) = std::fs::read_to_string(&path)
                {
                    let relative = format!("{dir}/{name}");
                    if !nested.iter().any(|(_, f)| f.path == relative)
                        && !found.iter().any(|f| f.path == relative)
                    {
                        nested.push((
                            depth,
                            ContextFile {
                                path: relative,
                                body: trim_body(&body, max_chars),
                            },
                        ));
                    }
                }
            }
            match dir.rsplit_once('/') {
                Some((parent, _)) => dir = parent.to_string(),
                None => break,
            }
        }
    }
    nested.sort_by_key(|a| a.0);
    for (_, file) in nested {
        found.push(file);
    }

    found
}

fn push_file(found: &mut Vec<ContextFile>, root: &Path, name: &str, max_chars: usize) {
    let path = root.join(name);
    if !path.is_file() {
        return;
    }
    let Ok(body) = std::fs::read_to_string(&path) else {
        return;
    };
    if body.trim().is_empty() {
        return;
    }
    if found.iter().any(|f| f.path == name) {
        return;
    }
    found.push(ContextFile {
        path: name.to_string(),
        body: trim_body(&body, max_chars),
    });
}

fn push_dir(found: &mut Vec<ContextFile>, root: &Path, dir: &Path, max_chars: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && matches!(
                    p.extension().and_then(|e| e.to_str()),
                    Some("md") | Some("txt")
                )
        })
        .collect();
    paths.sort();
    for path in paths {
        let name = path
            .strip_prefix(root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());
        push_file(found, root, &name, max_chars);
    }
}

fn trim_body(body: &str, max_chars: usize) -> String {
    let trimmed = body.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(max_chars).collect();
    out.push_str("\n… (truncated)");
    out
}
