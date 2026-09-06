//! Word error rate (WER) calculation for the cross-engine parity test.
//!
//! Inlined copy of `voxora_testkit::wer` and
//! `voxora_testkit::edit_distance` (testkit is `publish = false` so
//! `voxora-bridge` cannot declare it as a `[dev-dependencies]`; see
//! issue #140). Mirrors `voxora-testkit/src/wer.rs:1-35` byte-for-byte
//! so the test's scoring math stays in lock-step with the testkit
//! behaviour for every other voxora-* parity test.
//!
//! Each integration test file is its own crate root, so this file is
//! pulled into `tests/cross_engine_parity.rs` via `mod wer;` rather
//! than a sibling `pub mod`.

/// Compute the Levenshtein distance between two slices of tokens.
pub fn edit_distance(reference: &[&str], hypothesis: &[&str]) -> usize {
    if reference.is_empty() {
        return hypothesis.len();
    }
    if hypothesis.is_empty() {
        return reference.len();
    }
    let mut prev: Vec<usize> = (0..=hypothesis.len()).collect();
    let mut curr = vec![0; hypothesis.len() + 1];
    for (i, r) in reference.iter().enumerate() {
        curr[0] = i + 1;
        for (j, h) in hypothesis.iter().enumerate() {
            let cost = if r == h { 0 } else { 1 };
            curr[j + 1] = (curr[j] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[hypothesis.len()]
}

/// Compute word error rate. Returns 0.0 if both inputs are empty.
pub fn wer(reference: &str, hypothesis: &str) -> f64 {
    let r: Vec<&str> = reference.split_whitespace().collect();
    let h: Vec<&str> = hypothesis.split_whitespace().collect();
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    edit_distance(&r, &h) as f64 / r.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_on_identical() {
        assert_eq!(wer("the quick brown fox", "the quick brown fox"), 0.0);
    }

    #[test]
    fn counts_one_substitution_per_word() {
        let w = wer("a b c", "a x c");
        assert!((w - 1.0 / 3.0).abs() < 1e-9, "wer = {w}");
    }

    #[test]
    fn empty_reference_with_hypothesis_is_one() {
        assert_eq!(wer("", "hello world"), 1.0);
    }

    #[test]
    fn empty_both_is_zero() {
        assert_eq!(wer("", ""), 0.0);
    }

    #[test]
    fn edit_distance_matches_known_values() {
        assert_eq!(edit_distance(&["a", "b", "c"], &["a", "x", "c"]), 1);
        assert_eq!(edit_distance(&["a", "b"], &["a", "b", "c"]), 1);
    }
}
