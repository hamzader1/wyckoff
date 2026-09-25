//! Turning `git diff` text into [`FileDiff`]s.

use super::headers::{clean, hunk_starts, split_paths};
use super::parse::{FileDiff, Hunk, Status};

pub fn parse(text: &str) -> Vec<FileDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut current: Option<FileDiff> = None;
    let mut hunk: Option<Hunk> = None;
    let mut raw = String::new();

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            finish(&mut files, &mut current, &mut hunk, &mut raw);
            let (old, new) = split_paths(rest);
            raw.push_str(line);
            raw.push('\n');
            current = Some(FileDiff {
                path: new.unwrap_or_default(),
                old_path: old,
                status: Status::Modified,
                ..Default::default()
            });
            continue;
        }

        let Some(file) = current.as_mut() else {
            continue; // preamble before the first file
        };
        raw.push_str(line);
        raw.push('\n');

        if line.starts_with("@@") {
            if let Some(prev) = hunk.take() {
                file.hunks.push(prev);
            }
            hunk = Some(new_hunk(line));
            continue;
        }

        if let Some(open) = hunk.as_mut() {
            match line.chars().next() {
                Some('+') => {
                    file.insertions += 1;
                    open.lines.push(line.to_string());
                }
                Some('-') => {
                    file.deletions += 1;
                    open.lines.push(line.to_string());
                }
                Some(' ') | Some('\\') => open.lines.push(line.to_string()),
                _ => {
                    // Not hunk content: the hunk is over, treat it as a header.
                    file.hunks.push(hunk.take().unwrap());
                    apply_header(file, line);
                }
            }
            continue;
        }

        apply_header(file, line);
    }

    finish(&mut files, &mut current, &mut hunk, &mut raw);
    files
}

fn new_hunk(header: &str) -> Hunk {
    let (old_start, new_start) = hunk_starts(header);
    Hunk {
        header: header.to_string(),
        old_start,
        new_start,
        lines: Vec::new(),
    }
}

fn apply_header(file: &mut FileDiff, line: &str) {
    if line.starts_with("new file mode") {
        file.status = Status::Added;
    } else if line.starts_with("deleted file mode") {
        file.status = Status::Deleted;
    } else if line.starts_with("old mode") || line.starts_with("new mode") {
        file.mode_changed = true;
    } else if let Some(rest) = line.strip_prefix("rename from ") {
        file.old_path = Some(clean(rest));
        file.status = Status::Renamed;
    } else if let Some(rest) = line.strip_prefix("rename to ") {
        file.path = clean(rest);
        file.status = Status::Renamed;
    } else if let Some(rest) = line.strip_prefix("copy from ") {
        file.old_path = Some(clean(rest));
        file.status = Status::Copied;
    } else if let Some(rest) = line.strip_prefix("copy to ") {
        file.path = clean(rest);
        file.status = Status::Copied;
    } else if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
        file.binary = true;
    } else if line.starts_with("Submodule ") {
        file.submodule = true;
        file.status = Status::TypeChanged;
    }
}

fn finish(
    files: &mut Vec<FileDiff>,
    current: &mut Option<FileDiff>,
    hunk: &mut Option<Hunk>,
    raw: &mut String,
) {
    if let Some(mut file) = current.take() {
        if let Some(last) = hunk.take() {
            file.hunks.push(last);
        }
        file.raw = std::mem::take(raw);
        if !file.path.is_empty() {
            files.push(file);
        }
    }
    raw.clear();
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod scan_tests;
