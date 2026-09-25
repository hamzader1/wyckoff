//! Python and Go manifests.
//!
//! Split out of `manifest.rs` to keep both files small enough to read in one
//! sitting; the shape mirrors the Rust/Node extraction.

use super::manifest::read;
use std::path::Path;

const MAX_NAMES: usize = 40;

pub(super) fn collect(root: &Path, name: &str) -> Option<(String, String)> {
    match name {
        "pyproject.toml" => python(root),
        "go.mod" => go(root),
        _ => None,
    }
}

fn python(root: &Path) -> Option<(String, String)> {
    let text = read(root, "pyproject.toml")?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    let mut parts = Vec::new();

    if let Some(project) = value.get("project") {
        if let Some(name) = project.get("name").and_then(|v| v.as_str()) {
            parts.push(format!("package `{name}`"));
        }
        if let Some(description) = project.get("description").and_then(|v| v.as_str()) {
            parts.push(description.to_string());
        }
        if let Some(deps) = project.get("dependencies").and_then(|v| v.as_array()) {
            let names = dependency_names(deps.iter().filter_map(|d| d.as_str()));
            if !names.is_empty() {
                parts.push(format!("dependencies: {}", names.join(", ")));
            }
        }
    }
    // Poetry-style layouts keep dependencies in a nested table.
    if parts.is_empty()
        && let Some(deps) = nested_table_keys(&value, "tool.poetry.dependencies")
    {
        parts.push(format!("dependencies: {}", deps.join(", ")));
    }

    if parts.is_empty() {
        None
    } else {
        Some(("pyproject.toml".to_string(), parts.join(". ")))
    }
}

fn go(root: &Path) -> Option<(String, String)> {
    let text = read(root, "go.mod")?;
    let mut parts = Vec::new();
    let mut requires = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if let Some(module) = line.strip_prefix("module ") {
            parts.push(format!("module `{}`", module.trim()));
        } else if let Some(version) = line.strip_prefix("go ") {
            parts.push(format!("go {}", version.trim()));
        } else if !line.starts_with("//")
            && line.contains('.')
            && line.contains('/')
            && let Some(name) = line.split_whitespace().next()
            && requires.len() < MAX_NAMES
        {
            requires.push(name.to_string());
        }
    }
    if !requires.is_empty() {
        parts.push(format!("dependencies: {}", requires.join(", ")));
    }

    if parts.is_empty() {
        None
    } else {
        Some(("go.mod".to_string(), parts.join(". ")))
    }
}

/// `requests>=2` -> `requests`, keeping only real package names.
fn dependency_names<'a>(items: impl Iterator<Item = &'a str>) -> Vec<String> {
    items
        .take(MAX_NAMES)
        .map(|dep| {
            dep.split(['>', '<', '=', '[', ';', ' '])
                .next()
                .unwrap_or(dep)
                .trim()
                .to_string()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

/// Nested tables use dotted lookup, so `tool.poetry` needs walking rather than
/// a flat key.
pub(super) fn nested_table_keys(value: &toml::Value, path: &str) -> Option<Vec<String>> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    let table = current.as_table()?;
    let names: Vec<String> = table.keys().take(MAX_NAMES).cloned().collect();
    if names.is_empty() { None } else { Some(names) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_version_constraints_from_python_deps() {
        let names = dependency_names(["requests>=2", "click", "foo[bar]"].into_iter());
        assert_eq!(names, vec!["requests", "click", "foo"]);
    }

    #[test]
    fn walks_nested_toml_tables() {
        let value: toml::Value =
            toml::from_str("[tool.poetry.dependencies]\nserde = \"1\"\n").unwrap();
        let keys = nested_table_keys(&value, "tool.poetry.dependencies").unwrap();
        assert_eq!(keys, vec!["serde"]);
        assert!(nested_table_keys(&value, "tool.missing").is_none());
    }
}
