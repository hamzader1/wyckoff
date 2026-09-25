//! The project's own words: read by the developer, written once, edited rarely.
//!
//! `wyckoff init` writes a first draft, but the file is meant to be owned by the
//! humans on the project — it is the only place the *why* of a codebase lives
//! that a diff cannot show.

use crate::context::manifest;
use crate::error::{Error, Result};
use crate::git::Git;

pub fn draft(git: &Git) -> Result<String> {
    let root = git.root();
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "this project".to_string());

    let manifests = manifest::collect(root);
    let description = manifests
        .iter()
        .flat_map(|(_, facts)| facts.split(". "))
        .find(|part| {
            !part.starts_with("crate")
                && !part.starts_with("package")
                && !part.starts_with("module")
                && !part.starts_with("workspace")
                && !part.contains(':')
                && part.len() > 15
        })
        .map(|s| s.to_string());

    let mut out = String::new();
    out.push_str(&format!(
        "# {name}\n\n\
         One or two sentences on what this project is, and who uses it.\n\
         wyckoff reads this file to understand your changes beyond the diff.\n"
    ));

    if let Some(description) = &description {
        out.push_str(&format!(
            "\nAuto-detected from the manifest: {description}\n"
        ));
    }

    out.push_str("\n## Architecture\n\n");
    out.push_str(
        "How the pieces fit: the main modules, the data flow, where the tricky\n\
         parts live. Two short paragraphs beat an exhaustive map.\n",
    );

    let top: Vec<String> = git
        .file_count_by_top_dir()?
        .into_iter()
        .take(8)
        .map(|(dir, count)| format!("- {dir} ({count} files)"))
        .collect();
    if !top.is_empty() {
        out.push_str("\nLayout as git sees it:\n");
        out.push_str(&top.join("\n"));
        out.push('\n');
    }

    for (file, facts) in &manifests {
        out.push_str(&format!("\nFrom {file}: {facts}\n"));
    }

    out.push_str(
        "\n## Conventions\n\n\
         Rules a newcomer would get wrong: error handling, naming, where tests\n\
         go, what must never be touched without care.\n\n\
         ## Domain vocabulary\n\n\
         Names that mean something specific here (\"page\", \"cell\", \"wallet\")\n\
         and the word you would use in a commit message for each.\n\n\
         ## Current work\n\n\
         What is being built or refactored right now. This is the part that makes\n\
         the 'why' of a commit visible, so keep it roughly up to date.\n",
    );

    Ok(out)
}

/// Write the draft, refusing to clobber by default.
pub fn write(path: &std::path::Path, text: &str, force: bool) -> Result<()> {
    if path.exists() && !force {
        return Err(Error::msg(format!(
            "{} already exists (pass --force to overwrite)",
            crate::paths::display(path)
        )));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::write(path, text).map_err(|e| Error::io(path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_mentions_real_facts_and_the_hard_questions() {
        // `cargo test` runs inside this crate, which is a git repo, so the
        // layout section has something real to describe.
        let Ok(git) = Git::discover() else {
            return;
        };
        let text = draft(&git).unwrap();
        assert!(text.contains("## Architecture"), "{text}");
        assert!(text.contains("## Conventions"), "{text}");
        assert!(text.contains("## Domain vocabulary"), "{text}");
        assert!(text.contains("## Current work"), "{text}");
        assert!(text.contains("From Cargo.toml"), "{text}");
        assert!(text.contains("wyckoff"), "{text}");
    }

    #[test]
    fn write_refuses_to_clobber() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".context");
        write(&path, "one", false).unwrap();
        assert!(write(&path, "two", false).is_err());
        write(&path, "two", true).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
    }
}
