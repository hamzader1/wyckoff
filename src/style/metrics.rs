//! Per-subject measurements used to build a [`super::StyleProfile`].

use serde::{Deserialize, Serialize};

use crate::msg::clean::has_type_prefix;
use crate::util;

#[cfg(test)]
#[path = "metrics_tests.rs"]
mod metrics_tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    Arabic,
    French,
    Other,
}

impl Language {
    pub fn label(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Arabic => "Arabic",
            Language::French => "French",
            Language::Other => "non-English",
        }
    }
}

/// Opening words that mean the author did *not* write in the imperative.
const NON_IMPERATIVE: &[&str] = &[
    "this",
    "these",
    "minor",
    "various",
    "some",
    "more",
    "misc",
    "general",
    "cleanup",
    "cleanups",
    "wip",
    "temp",
    "temporary",
    "improved",
    "working",
    "started",
    "stuff",
    "things",
    "trying",
];

pub fn is_conventional(subject: &str) -> bool {
    has_type_prefix(subject)
}

pub fn starts_sentence_case(subject: &str) -> bool {
    subject
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
}

pub fn has_trailing_period(subject: &str) -> bool {
    subject.trim_end().ends_with('.')
}

/// Heuristic: past tense (`Added`), gerunds (`Fixing`), or a vague opener.
pub fn looks_non_imperative(subject: &str) -> bool {
    let Some(first) = subject.split_whitespace().next() else {
        return true;
    };
    let first = first
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    if first.is_empty() {
        return true;
    }
    if NON_IMPERATIVE.contains(&first.as_str()) {
        return true;
    }
    // "Fixes"/"Adds" are imperative-ish enough; "-ed"/"-ing" are not.
    (first.ends_with("ed") || first.ends_with("ing"))
        && !["need", "red", "feed", "speed", "embed"].contains(&first.as_str())
}

pub fn has_ticket(subject: &str) -> bool {
    if let Some(index) = subject.find('#') {
        let digits: String = subject[index + 1..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.len() >= 2 {
            return true;
        }
    }
    let chars: Vec<char> = subject.chars().collect();
    for (i, ch) in chars.iter().enumerate() {
        if *ch != '-' {
            continue;
        }
        let left: String = chars[..i]
            .iter()
            .rev()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let right: String = chars[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if left.len() >= 2 && (1..=6).contains(&right.len()) {
            return true;
        }
    }
    false
}

pub fn has_emoji(subject: &str) -> bool {
    subject.chars().any(|c| {
        let code = c as u32;
        (0x1F300..=0x1FAFF).contains(&code)
            || (0x2600..=0x27BF).contains(&code)
            || (0x2190..=0x21FF).contains(&code)
    })
}

pub fn opening_word(subject: &str) -> Option<String> {
    let first = subject.split_whitespace().next()?;
    let cleaned = first
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Script counting with a stopword backstop. Committing in Arabic or French is
/// common enough that guessing "English" and writing English would be an
/// obvious failure.
pub fn detect_language(subjects: &[String]) -> Language {
    const FRENCH: &[&str] = &[
        "le", "la", "les", "des", "une", "pour", "avec", "dans", "sur", "est", "etre", "ajout",
        "ajoute", "corrige", "ameliore", "supprime", "renomme", "remplace", "gestion", "lors",
        "quand", "sans", "plus", "nouveau", "nouvelle",
    ];
    const ENGLISH: &[&str] = &[
        "the",
        "add",
        "adds",
        "fix",
        "fixes",
        "use",
        "using",
        "and",
        "for",
        "with",
        "remove",
        "refactor",
        "update",
        "handle",
        "make",
        "move",
        "rename",
        "split",
        "clean",
        "allow",
        "support",
        "avoid",
        "improve",
        "implement",
    ];

    let mut arabic = 0usize;
    let mut latin = 0usize;
    let mut other_script = 0usize;
    let mut french_hits = 0usize;
    let mut english_hits = 0usize;

    for subject in subjects {
        for word in util::words(subject) {
            if word.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)) {
                arabic += 1;
            } else if word.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c))
                || word.chars().any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
            {
                other_script += 1;
            } else {
                latin += 1;
                if FRENCH.contains(&word.as_str()) {
                    french_hits += 1;
                }
                if ENGLISH.contains(&word.as_str()) {
                    english_hits += 1;
                }
            }
        }
    }

    let total = arabic + latin + other_script;
    if total == 0 {
        return Language::English;
    }
    if arabic * 100 / total >= 20 {
        return Language::Arabic;
    }
    if other_script * 100 / total >= 20 {
        return Language::Other;
    }
    if french_hits > english_hits && french_hits >= 3 {
        return Language::French;
    }
    Language::English
}
