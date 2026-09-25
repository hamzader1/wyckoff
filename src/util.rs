//! Small string helpers shared across the pipeline.

/// `src/diff/classify.rs` -> `classify`, `src/diff/mod.rs` -> `diff`,
/// `my_module/thing.rs` -> `thing`. Falls back to the stem of the path.
pub fn short_name(path: &str) -> String {
    let cleaned = path.trim_end_matches('/');
    let base = cleaned.rsplit('/').next().unwrap_or(cleaned);
    let stem = match base.rsplit_once('.') {
        Some((head, ext)) if !head.is_empty() && ext.len() <= 5 => head,
        _ => base,
    };
    let stem = match stem {
        // `mod.rs`, `index.ts`, `__init__.py` are named after their directory.
        // `lib.rs` and `main.rs` keep their own names: `src` says nothing.
        "mod" | "index" | "__init__" => cleaned.rsplit('/').nth(1).unwrap_or(stem),
        other => other,
    };
    stem.to_string()
}

/// Uppercase the first alphabetic character, leave the rest alone.
pub fn sentence_case(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_lowercase() => {
            let mut out = first.to_uppercase().collect::<String>();
            out.push_str(chars.as_str());
            out
        }
        _ => text.to_string(),
    }
}

/// Cut to at most `max` characters, preferring a word boundary and never
/// leaving a dangling conjunction.
pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out = String::new();
    for word in text.split_whitespace() {
        let candidate = if out.is_empty() {
            word.to_string()
        } else {
            format!("{out} {word}")
        };
        if candidate.chars().count() > max {
            break;
        }
        out = candidate;
    }
    if out.is_empty() {
        return text.chars().take(max).collect();
    }
    let trimmed = out.trim_end_matches([',', ';', ':', ' ', '&', '-']);
    for tail in [" and", " to", " for", " the", " with", " in", " of"] {
        if let Some(stripped) = trimmed.strip_suffix(tail) {
            return stripped.to_string();
        }
    }
    trimmed.to_string()
}

pub fn plural(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// Split a sentence into lowercase word tokens (letters, digits, `_`).
pub fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_name_handles_rust_conventions() {
        assert_eq!(short_name("src/diff/classify.rs"), "classify");
        assert_eq!(short_name("src/diff/mod.rs"), "diff");
        assert_eq!(short_name("src/main.rs"), "main");
        assert_eq!(short_name("src/lib.rs"), "lib");
        assert_eq!(short_name("tests/parser_test.rs"), "parser_test");
        assert_eq!(short_name("BTree/page.rs"), "page");
        assert_eq!(short_name("web/index.ts"), "web");
    }

    #[test]
    fn truncate_prefers_word_boundaries() {
        assert_eq!(truncate("hello world and more", 11), "hello world");
        assert_eq!(truncate("short", 20), "short");
    }

    #[test]
    fn sentence_case_only_touches_lowercase_start() {
        assert_eq!(sentence_case("add a thing"), "Add a thing");
        assert_eq!(sentence_case("BTree fix"), "BTree fix");
    }
}
