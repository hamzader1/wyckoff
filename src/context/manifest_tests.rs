//! Tests for manifest extraction.

use super::*;

#[test]
fn reads_cargo_facts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"wyckoff\"\ndescription = \"commit messages\"\n\n[dependencies]\nserde = \"1\"\nclap = \"4\"\n",
    )
    .unwrap();
    let facts = collect(dir.path());
    assert_eq!(facts.len(), 1);
    assert!(facts[0].1.contains("crate `wyckoff`"), "{:?}", facts);
    assert!(facts[0].1.contains("serde"), "{:?}", facts);
    assert!(facts[0].1.contains("commit messages"), "{:?}", facts);
}

#[test]
fn reads_workspace_members() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"core\", \"cli\"]\n",
    )
    .unwrap();
    let facts = collect(dir.path());
    assert!(
        facts[0].1.contains("workspace members: core, cli"),
        "{:?}",
        facts
    );
}

#[test]
fn reads_npm_facts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"name":"web","description":"the ui","scripts":{"dev":"vite"},"dependencies":{"react":"19"}}"#,
    )
    .unwrap();
    let facts = collect(dir.path());
    assert_eq!(facts[0].0, "package.json");
    assert!(facts[0].1.contains("react"), "{:?}", facts);
    assert!(facts[0].1.contains("dev"), "{:?}", facts);
}

#[test]
fn reads_python_and_go_facts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\nname = \"tool\"\ndependencies = [\"requests>=2\", \"click\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("go.mod"),
        "module github.com/me/tool\n\ngo 1.24\n\nrequire github.com/foo/bar v1.2.3\n",
    )
    .unwrap();
    let facts = collect(dir.path());
    let py = facts.iter().find(|(n, _)| n == "pyproject.toml").unwrap();
    assert!(py.1.contains("requests"), "{:?}", py);
    let go = facts.iter().find(|(n, _)| n == "go.mod").unwrap();
    assert!(go.1.contains("github.com/me/tool"), "{:?}", go);
}

#[test]
fn missing_manifests_are_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    assert!(collect(dir.path()).is_empty());
}
