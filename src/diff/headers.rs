//! Parsing the small tokens git puts in diff headers.
//!
//! Isolated from the main scan loop because it is the fiddliest part of the
//! format (quoted paths, rename pairs, hunk range syntax) and is worth its own
//! unit tests.

/// `a/foo.rs b/foo.rs` -> (`foo.rs`, `foo.rs`), honouring quoted paths.
pub(super) fn split_paths(rest: &str) -> (Option<String>, Option<String>) {
    let mut parts: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for ch in rest.chars() {
        if escaped {
            buf.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_quotes => {
                buf.push(ch);
                escaped = true;
            }
            '"' => {
                in_quotes = !in_quotes;
                buf.push(ch);
            }
            ' ' | '\t' if !in_quotes => {
                if !buf.is_empty() {
                    parts.push(std::mem::take(&mut buf));
                }
            }
            _ => buf.push(ch),
        }
    }
    if !buf.is_empty() {
        parts.push(buf);
    }

    let norm = |token: &str| -> Option<String> {
        let value = clean(token);
        if value == "/dev/null" || value.is_empty() {
            return None;
        }
        Some(strip_ab_prefix(&value))
    };

    match parts.len() {
        0 => (None, None),
        1 => (None, norm(&parts[0])),
        _ => (norm(&parts[0]), norm(&parts[1])),
    }
}

/// Drop git's `a/` or `b/` path prefix.
fn strip_ab_prefix(value: &str) -> String {
    for prefix in ["a/", "b/"] {
        if let Some(rest) = value.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    value.to_string()
}

/// Unquote a git path (`"src/a b.rs"` -> `src/a b.rs`).
pub(super) fn clean(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() < 2 || !trimmed.starts_with('"') || !trimmed.ends_with('"') {
        return trimmed.to_string();
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => {}
        }
    }
    out
}

/// `@@ -12,7 +14,9 @@ fn thing()` -> `(12, 14)`
pub(super) fn hunk_starts(header: &str) -> (usize, usize) {
    let mut nums: Vec<usize> = Vec::with_capacity(2);
    let mut current: Option<usize> = None;

    for ch in header.chars() {
        match ch {
            '-' | '+' if current.is_none() => current = Some(0),
            c if c.is_ascii_digit() && current.is_some() => {
                current = Some(current.unwrap() * 10 + c.to_digit(10).unwrap_or(0) as usize);
            }
            ',' | ' ' if current.is_some() => nums.push(current.take().unwrap()),
            '@' if nums.len() >= 2 => break,
            _ => {
                if let Some(n) = current.take() {
                    nums.push(n);
                }
            }
        }
    }
    if let Some(n) = current {
        nums.push(n);
    }
    (
        nums.first().copied().unwrap_or(0),
        nums.get(1).copied().unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_plain_and_quoted_pairs() {
        assert_eq!(
            split_paths("a/src/foo.rs b/src/foo.rs"),
            (Some("src/foo.rs".into()), Some("src/foo.rs".into()))
        );
        assert_eq!(
            split_paths("\"a/my file.rs\" \"b/my file.rs\""),
            (Some("my file.rs".into()), Some("my file.rs".into()))
        );
        assert_eq!(
            split_paths("a/gone.rs /dev/null"),
            (Some("gone.rs".into()), None)
        );
    }

    #[test]
    fn unescapes_quotes_and_tabs() {
        assert_eq!(clean("\"a\\tb.rs\""), "a\tb.rs");
        assert_eq!(clean("\"say \\\"hi\\\"\""), "say \"hi\"");
        assert_eq!(clean("plain.rs"), "plain.rs");
    }

    #[test]
    fn reads_hunk_ranges() {
        assert_eq!(hunk_starts("@@ -12,7 +14,9 @@ fn thing()"), (12, 14));
        assert_eq!(hunk_starts("@@ -1 +1 @@"), (1, 1));
        assert_eq!(hunk_starts("@@ -0,0 +1,25 @@"), (0, 1));
        assert_eq!(hunk_starts("@@ -100,3 +100,4 @@"), (100, 100));
    }
}
