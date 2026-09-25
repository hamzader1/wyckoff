//! Facts pulled out of the repo's manifest files (Rust, Node).
//!
//! These are written by tooling rather than by the developer, so unlike
//! `.context` they can never go stale. Cheap to extract, and they ground the
//! message in the real vocabulary of the project: crate names, scripts, deps.

use std::path::Path;

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;

const MAX_NAMES: usize = 40;

pub fn collect(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(facts) = cargo(root) {
        out.push(facts);
    }
    if let Some(facts) = npm(root) {
        out.push(facts);
    }
    for name in ["pyproject.toml", "go.mod"] {
        if let Some(facts) = super::pyproject::collect(root, name) {
            out.push(facts);
        }
    }
    out
}

pub(super) fn read(root: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(root.join(name)).ok()
}

pub(super) fn table_keys(value: &toml::Value, key: &str) -> Option<Vec<String>> {
    let table = value.get(key)?.as_table()?;
    let names: Vec<String> = table.keys().take(MAX_NAMES).cloned().collect();
    if names.is_empty() { None } else { Some(names) }
}

fn cargo(root: &Path) -> Option<(String, String)> {
    let text = read(root, "Cargo.toml")?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    let mut parts = Vec::new();

    if let Some(package) = value.get("package") {
        if let Some(name) = package.get("name").and_then(|v| v.as_str()) {
            parts.push(format!("crate `{name}`"));
        }
        if let Some(description) = package.get("description").and_then(|v| v.as_str()) {
            parts.push(description.to_string());
        }
    }
    if let Some(members) = value
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
    {
        let names: Vec<String> = members
            .iter()
            .filter_map(|m| m.as_str())
            .take(MAX_NAMES)
            .map(|m| m.to_string())
            .collect();
        if !names.is_empty() {
            parts.push(format!("workspace members: {}", names.join(", ")));
        }
    }
    if let Some(deps) = table_keys(&value, "dependencies") {
        parts.push(format!("main dependencies: {}", deps.join(", ")));
    }
    if parts.is_empty() {
        None
    } else {
        Some(("Cargo.toml".to_string(), parts.join(". ")))
    }
}

fn npm(root: &Path) -> Option<(String, String)> {
    let text = read(root, "package.json")?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let mut parts = Vec::new();

    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
        parts.push(format!("package `{name}`"));
    }
    if let Some(description) = value.get("description").and_then(|v| v.as_str()) {
        parts.push(description.to_string());
    }
    for key in ["scripts", "dependencies", "devDependencies"] {
        if let Some(object) = value.get(key).and_then(|v| v.as_object()) {
            let names: Vec<String> = object.keys().take(MAX_NAMES).cloned().collect();
            if !names.is_empty() {
                parts.push(format!("{key}: {}", names.join(", ")));
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(("package.json".to_string(), parts.join(". ")))
    }
}
