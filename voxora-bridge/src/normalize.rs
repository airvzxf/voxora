//! Canonical transcript normalisation for cross-engine WER.
//!
//! `voxora_testkit::wer` splits on whitespace and compares
//! tokens verbatim. Without normalisation, semantically
//! identical transcripts ("Hello, world!" vs "hello world"
//! vs "Hello World") produce a non-zero WER. The [`normalize`]
//! helper applies the four canonical transforms so the
//! comparison reflects semantic agreement.
//!
//! ## Transforms (in order)
//!
//! 1. Lower-case (ASCII).
//! 2. Strip ASCII punctuation; collapse curly / smart quotes
//!    to ASCII apostrophes so `it's` matches `it's`.
//! 3. Collapse runs of whitespace to a single space and trim
//!    leading / trailing whitespace.
//!
//! Returns the empty string for inputs that reduce to nothing
//! after stripping (e.g. `"…!"`).
//!
//! ## Where this lives
//!
//! Extracted out of the `cross_engine_parity` integration test
//! (PR #137 follow-up to #135) so the unit tests run in the
//! default offline test lane. The pure-string logic does not
//! need real model weights and was previously hidden behind
//! `--features parity`, which CI's T2 lane (`cargo test
//! --workspace --all-targets`) does not enable.

/// Canonical transcript normalisation for cross-engine WER.
///
/// See the [module docs](self) for the full transform list.
pub fn normalize(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    // First pass: map smart-quote variants to ASCII apostrophes
    // so "don't" and "don't" tokenise identically. We do this
    // before stripping punctuation because some punctuation
    // chars carry semantic information in the apostrophe
    // position (e.g. "can't" vs "cant").
    let smart_quoted = lower
        .replace(['\u{2018}', '\u{2019}'], "'")
        .replace(['\u{201C}', '\u{201D}'], "\"");
    // Second pass: keep ASCII apostrophes; drop everything else
    // that is not an ASCII alphanumeric or whitespace.
    let mut out = String::with_capacity(smart_quoted.len());
    for ch in smart_quoted.chars() {
        let keep = ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || ch == '\'';
        if keep {
            out.push(ch);
        } else {
            out.push(' ');
        }
    }
    // Third pass: collapse runs of whitespace.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn normalize_lowercases_ascii() {
        assert_eq!(normalize("HELLO World"), "hello world");
    }

    #[test]
    fn normalize_strips_punctuation() {
        assert_eq!(normalize("Hello, world!"), "hello world");
    }

    #[test]
    fn normalize_keeps_apostrophes() {
        assert_eq!(normalize("don't stop"), "don't stop");
    }

    #[test]
    fn normalize_smart_quotes_to_ascii() {
        // Smart single quotes (U+2018, U+2019) carry semantic
        // information (apostrophe / opening quote) so they are
        // kept as ASCII apostrophes. Smart double quotes (U+201C,
        // U+201D) are stripped along with the rest of the ASCII
        // punctuation so `Hello "world"` matches `hello world`
        // token-for-token after normalisation.
        assert_eq!(
            normalize("\u{2018}hello\u{2019} world\u{201D}"),
            "'hello' world"
        );
    }

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize("  hello\t\n world  "), "hello world");
    }

    #[test]
    fn normalize_pure_punctuation_yields_empty() {
        assert_eq!(normalize("…!?"), "");
    }
}
