//! Token estimation without a tokenizer.
//!
//! Budgeting needs to be *roughly* right, not exact, and pulling in a real BPE
//! tokenizer would add a multi-megabyte table to a tool that runs on every
//! commit. Source code sits around 3-3.5 characters per token, English prose is
//! closer to 4. We use 3.5, which errs slightly on the safe side for code.

/// Estimated tokens for a chunk of text (chars / 3.5).
pub fn estimate(text: &str) -> usize {
    // chars * 2 / 7 == chars / 3.5, integer-only.
    (text.chars().count() * 2).div_ceil(7)
}

/// Estimated tokens for a contiguous block of text (used for raw file diffs).
pub fn estimate_block(block: &str) -> usize {
    estimate(block) + block.lines().count().div_ceil(8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_zero() {
        assert_eq!(estimate(""), 0);
    }

    #[test]
    fn roughly_a_quarter_of_chars() {
        let text = "a".repeat(350);
        let got = estimate(&text);
        assert!((90..=110).contains(&got), "expected ~100, got {got}");
    }

    #[test]
    fn blocks_cost_more_than_the_sum_of_their_text() {
        let text = "line one\nline two\nline three\n";
        assert!(estimate_block(text) > estimate(text));
    }
}
