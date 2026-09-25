//! Pulling real definition names out of added and removed lines.
//!
//! Used to hand the model the exact vocabulary of a change (`+ \`replace_cell\`
//! (function) in src/btree/page.rs`) instead of hoping it invents the right
//! words. Every name comes from the diff, so nothing here can hallucinate.

mod defs;

#[cfg(test)]
#[path = "symbols_tests.rs"]
mod symbols_tests;

use crate::diff::FileDiff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: &'static str,
}

pub const MAX_SYMBOLS: usize = 12;

pub fn added_symbols(file: &FileDiff) -> Vec<Symbol> {
    collect(file.added_lines().into_iter())
}

pub fn removed_symbols(file: &FileDiff) -> Vec<Symbol> {
    collect(file.removed_lines().into_iter())
}

fn collect<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<Symbol> {
    let mut out: Vec<Symbol> = Vec::new();
    for line in lines {
        for symbol in defs::symbols_in_line(line) {
            if out.iter().any(|s| s.name == symbol.name) {
                continue;
            }
            if out.len() >= MAX_SYMBOLS {
                return out;
            }
            out.push(symbol);
        }
    }
    out
}
