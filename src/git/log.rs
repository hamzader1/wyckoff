//! Reading the recent commit log.
//!
//! Format we ask git for: `%x01<sha>%x02<subject>` followed by `--name-only`
//! output. Both subjects and paths land in one stream, so one process gives us
//! everything the style engine and the exemplar picker need.

#[derive(Debug, Clone)]
pub struct CommitRecord {
    pub sha: String,
    pub subject: String,
    pub paths: Vec<String>,
}

pub fn parse(raw: &str) -> Vec<CommitRecord> {
    let mut records: Vec<CommitRecord> = Vec::new();
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix('\u{1}') {
            let (sha, subject) = match rest.split_once('\u{2}') {
                Some((sha, subject)) => (sha.to_string(), subject.to_string()),
                None => (rest.to_string(), String::new()),
            };
            records.push(CommitRecord {
                sha,
                subject,
                paths: Vec::new(),
            });
        } else if !line.trim().is_empty()
            && let Some(last) = records.last_mut()
        {
            last.paths.push(line.trim().to_string());
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_records_with_paths() {
        let raw = "\u{1}abc123\u{2}Add cache\nsrc/cache.rs\nsrc/lib.rs\n\n\u{1}def456\u{2}Fix parser\nsrc/diff/parse.rs\n";
        let records = parse(raw);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].sha, "abc123");
        assert_eq!(records[0].subject, "Add cache");
        assert_eq!(records[0].paths, vec!["src/cache.rs", "src/lib.rs"]);
        assert_eq!(records[1].paths, vec!["src/diff/parse.rs"]);
    }

    #[test]
    fn subjects_containing_separators_survive() {
        let records = parse("\u{1}abc\u{2}Handle a:b and c\n");
        assert_eq!(records[0].subject, "Handle a:b and c");
    }

    #[test]
    fn commits_without_paths_are_fine() {
        let records = parse("\u{1}abc\u{2}Empty\n\u{1}def\u{2}Other\n");
        assert_eq!(records.len(), 2);
        assert!(records[0].paths.is_empty());
    }
}
