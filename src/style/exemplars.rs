//! Choosing which past commit subjects to show the model.
//!
//! Relevance beats recency here. A commit that touched `src/btree/page.rs` is a
//! far better model for the voice to use on a staged change to that file than
//! whatever happened to land yesterday in a totally different subsystem.

use super::Exemplar;
use crate::git::CommitRecord;

pub fn pick(
    records: &[CommitRecord],
    staged_paths: &[String],
    exemplar_count: usize,
) -> Vec<Exemplar> {
    if exemplar_count == 0 {
        return Vec::new();
    }

    let mut scored: Vec<(usize, &CommitRecord)> = records
        .iter()
        .filter(|r| !r.subject.trim().is_empty())
        .filter(|r| r.subject.chars().count() <= 140)
        .map(|record| (overlap_score(&record.paths, staged_paths), record))
        .collect();

    // Sorting only by score keeps the newest-first order git gave us for ties.
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score));

    let mut out: Vec<Exemplar> = Vec::new();
    for (score, record) in scored.iter() {
        if out.len() >= exemplar_count {
            break;
        }
        if out.iter().any(|e| e.subject == record.subject.trim()) {
            continue;
        }
        out.push(Exemplar {
            subject: record.subject.trim().to_string(),
            overlap: *score,
        });
    }

    // Nothing overlapped (or a brand new repo): fall back to the newest
    // messages so the model still has a voice to copy.
    if out.is_empty() {
        for record in records.iter().take(exemplar_count) {
            if record.subject.trim().is_empty() {
                continue;
            }
            out.push(Exemplar {
                subject: record.subject.trim().to_string(),
                overlap: 0,
            });
        }
    }

    out
}

/// Exact path beats same directory or same filename, which beat same extension.
fn overlap_score(commit_paths: &[String], staged_paths: &[String]) -> usize {
    let mut score = 0;
    for path in commit_paths {
        let dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        let base = path.rsplit('/').next().unwrap_or(path);
        let ext = path.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
        for staged in staged_paths {
            if staged == path {
                score += 4;
            } else if (!dir.is_empty() && staged.starts_with(dir))
                || staged.rsplit('/').next().unwrap_or("") == base
            {
                // Same directory, or the same file name elsewhere in the tree.
                score += 2;
            } else if !ext.is_empty() && staged.ends_with(ext) {
                score += 1;
            }
        }
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(subject: &str, paths: &[&str]) -> CommitRecord {
        CommitRecord {
            sha: "x".into(),
            subject: subject.into(),
            paths: paths.iter().map(|p| p.to_string()).collect(),
        }
    }

    #[test]
    fn scores_exact_paths_highest() {
        let staged = vec!["src/btree/page.rs".to_string()];
        assert!(overlap_score(&["src/btree/page.rs".into()], &staged) > 0);
        assert_eq!(overlap_score(&["src/other/page.rs".into()], &staged), 2);
        assert_eq!(overlap_score(&["totally/unrelated.rs".into()], &staged), 1);
        assert_eq!(overlap_score(&["docs/readme.md".into()], &staged), 0);
    }

    #[test]
    fn picks_relevant_then_dedupes() {
        let records = vec![
            record("Add cache warming", &["src/cache.rs"]),
            record("Add cache warming", &["src/cache.rs"]),
            record("Fix parser", &["src/diff/parse.rs"]),
        ];
        let picked = pick(&records, &["src/cache.rs".into()], 3);
        assert_eq!(picked.len(), 2);
        assert_eq!(picked[0].subject, "Add cache warming");
        assert!(picked[0].overlap > 0);
    }

    #[test]
    fn falls_back_to_recent_when_nothing_overlaps() {
        let records = vec![
            record("Add cache warming", &["src/cache.rs"]),
            record("Fix parser", &["src/diff/parse.rs"]),
        ];
        let picked = pick(&records, &["docs/notes.md".into()], 2);
        assert_eq!(picked.len(), 2);
        assert!(picked.iter().all(|e| e.overlap == 0), "{picked:?}");
    }

    #[test]
    fn nothing_to_pick_from() {
        assert!(pick(&[], &[], 3).is_empty());
    }
}
