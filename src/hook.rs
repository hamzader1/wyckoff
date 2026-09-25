//! Installing wyckoff as a git hook.
//!
//! `prepare-commit-msg` is the right hook: git calls it after the staging area
//! is final but before the editor opens, and it hands us a file to write into.
//! A plain `git commit` then opens the editor already filled in — the message
//! is generated only when the human did not supply one.

use std::path::Path;

use crate::error::{Error, Result};
use crate::git::Git;

pub const HOOK_NAME: &str = "prepare-commit-msg";
pub const MARKER: &str = "wyckoff";

/// Skips: merges, squashes, `-m`, `-c/-C/--amend`, templates, rebases.
pub const SCRIPT: &str = r#"#!/bin/sh
# wyckoff — fills in a commit message when you did not write one.
# Delete this file, or run `wyckoff hook uninstall`, to stop it.
case "$2" in
  message|merge|squash|commit|template) exit 0 ;;
esac
command -v wyckoff >/dev/null 2>&1 || exit 0
wyckoff hook-fill "$1" || true
exit 0
"#;

pub fn install(git: &Git, force: bool) -> Result<std::path::PathBuf> {
    let path = git.hook_path(HOOK_NAME);
    if path.exists() && !force {
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if !existing.contains(MARKER) {
            return Err(Error::msg(format!(
                "{} already exists and is not ours.\n\
                 Merge it by hand, or re-run with --force to overwrite it.",
                crate::paths::display(&path)
            )));
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::write(&path, SCRIPT).map_err(|e| Error::io(&path, e))?;
    make_executable(&path)?;
    Ok(path)
}

pub fn uninstall(git: &Git) -> Result<bool> {
    let path = git.hook_path(HOOK_NAME);
    if !path.exists() {
        return Ok(false);
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    if !existing.contains(MARKER) {
        return Err(Error::msg(format!(
            "{} is not a wyckoff hook; leaving it alone",
            crate::paths::display(&path)
        )));
    }
    std::fs::remove_file(&path).map_err(|e| Error::io(&path, e))?;
    Ok(true)
}

pub fn status(git: &Git) -> (std::path::PathBuf, bool) {
    let path = git.hook_path(HOOK_NAME);
    let installed = path
        .exists()
        .then(|| std::fs::read_to_string(&path).unwrap_or_default())
        .map(|text| text.contains(MARKER))
        .unwrap_or(false);
    (path, installed)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)
        .map_err(|e| Error::io(path, e))?
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).map_err(|e| Error::io(path, e))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// Decide whether the hook should act, given git's message file.
pub fn should_fill(message_file: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(message_file) else {
        return true;
    };
    // Anything the human actually typed wins — including a filled-in template.
    !text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .any(|line| !line.starts_with('#'))
}

/// Write the generated message into git's message file.
pub fn write_message(message_file: &Path, message: &str) -> Result<()> {
    let mut body = message.trim().to_string();
    body.push('\n');
    std::fs::write(message_file, body).map_err(|e| Error::io(message_file, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_skips_the_cases_that_must_not_be_touched() {
        for source in ["message", "merge", "squash", "commit", "template"] {
            assert!(
                SCRIPT.contains(&format!("{source}|"))
                    || SCRIPT.contains(&format!("|{source}"))
                    || SCRIPT.contains(source),
                "hook should short-circuit for {source}"
            );
        }
        assert!(SCRIPT.starts_with("#!/bin/sh"));
        assert!(SCRIPT.contains(MARKER));
    }

    #[test]
    fn should_fill_only_when_the_human_said_nothing() {
        let dir = tempfile::tempdir().unwrap();

        let empty = dir.path().join("empty");
        std::fs::write(&empty, "\n").unwrap();
        assert!(should_fill(&empty));

        let comments = dir.path().join("comments");
        std::fs::write(&comments, "# Please enter the commit message\n#\n").unwrap();
        assert!(should_fill(&comments));

        let typed = dir.path().join("typed");
        std::fs::write(&typed, "Fix the thing\n").unwrap();
        assert!(!should_fill(&typed));

        // A missing file counts as "nothing typed yet".
        assert!(should_fill(&dir.path().join("nope")));
    }

    #[test]
    fn writing_the_message_trims_and_newlines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("msg");
        write_message(&path, "  Add cache warming  ").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "Add cache warming\n"
        );
    }
}
