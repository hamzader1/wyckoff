//! Recognising definitions, in any of the languages this tool is likely to see.
//!
//! Deliberately a keyword scan rather than a parser: we only need the *names* of
//! things that were introduced, and a false negative costs far less than a false
//! positive (which would put an invented identifier in the message).

use super::Symbol;

const KEYWORDS: &[(&str, &str)] = &[
    ("macro_rules!", "macro"),
    ("pub fn", "function"),
    ("fn", "function"),
    ("pub struct", "struct"),
    ("struct", "struct"),
    ("pub enum", "enum"),
    ("enum", "enum"),
    ("pub trait", "trait"),
    ("trait", "trait"),
    ("pub const", "constant"),
    ("pub static", "constant"),
    ("const", "constant"),
    ("static", "constant"),
    ("pub type", "type"),
    ("type", "type"),
    ("def", "function"),
    ("class", "class"),
    ("function", "function"),
    ("interface", "interface"),
    ("func", "function"),
];

/// Words that may precede a definition keyword. Anything else before the keyword
/// means we are looking at a statement, not a signature.
const MODIFIERS: &[&str] = &[
    "pub",
    "pub(crate)",
    "pub(super)",
    "pub(self)",
    "export",
    "default",
    "async",
    "unsafe",
    "extern",
    "const",
    "static",
    "declare",
    "abstract",
    "final",
    "open",
    "override",
    "partial",
    "private",
    "protected",
    "internal",
    "public",
    "readonly",
    "sealed",
];

pub fn symbols_in_line(line: &str) -> Vec<Symbol> {
    let text = line.trim();

    // Comments and attributes are not definitions. This is what stops
    // "// a struct is not defined here" from registering as `struct is`.
    if text.starts_with("//")
        || text.starts_with('#')
        || text.starts_with('*')
        || text.starts_with("/*")
        || text.starts_with("--")
    {
        return Vec::new();
    }

    // One definition per line is the norm, so stopping at the first match avoids
    // counting `pub fn` as both `pub fn` and `fn`.
    for (keyword, kind) in KEYWORDS {
        if let Some(name) = name_after(text, keyword) {
            return vec![Symbol { name, kind }];
        }
    }
    Vec::new()
}

fn name_after(text: &str, keyword: &str) -> Option<String> {
    let mut from = 0;
    while from < text.len() {
        let index = text[from..].find(keyword)? + from;
        let before = &text[..index];
        if is_modifier_prefix(before)
            && let Some(name) = take_ident(&text[index + keyword.len()..])
        {
            return Some(name);
        }
        from = index + keyword.len();
    }
    None
}

fn is_modifier_prefix(before: &str) -> bool {
    before
        .split_whitespace()
        .all(|word| MODIFIERS.contains(&word.trim().to_lowercase().as_str()))
}

fn take_ident(rest: &str) -> Option<String> {
    let rest = rest.trim_start();
    // Go methods: `func (r *Reader) Read(p []byte)`.
    let rest = if rest.starts_with('(') {
        let close = rest.find(')')?;
        rest[close + 1..].trim_start()
    } else {
        rest
    };

    let ident: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    let starts_ok = ident
        .chars()
        .next()
        .map(|c| c.is_alphabetic() || c == '_')
        .unwrap_or(false);
    if ident.is_empty() || !starts_ok {
        return None;
    }

    // A definition is followed by something structural, not by punctuation that
    // would make it a call or a list element.
    let after = rest[ident.len()..].trim_start();
    if after.starts_with(',') || after.starts_with(')') || after.starts_with('[') {
        return None;
    }
    Some(ident)
}
