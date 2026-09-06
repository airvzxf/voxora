//! Fuzz target: `voxora-registry::id::ModelId::parse`.
//!
//! ## Property under test (closes #54, EPIC #133)
//!
//! ```text
//! for all s: &str:
//!     if let Ok(id) = ModelId::parse(s):
//!         let canonical = id.canonical();
//!         assert_eq!(ModelId::parse(&canonical)?, id)
//! ```
//!
//! This is the **canonical round-trip property**: every string
//! the parser accepts must round-trip through `canonical()`
//! back to an equal `ModelId`. If the parser ever accepts a
//! string that does not round-trip, future code that joins
//! `ModelId::path` onto a base or compares two `ModelId`s by
//! their canonical form will silently disagree.
//!
//! The `debug_assert!` at `voxora-registry/src/id.rs:146`
//! (`ModelId::path` must not be empty) is intentionally NOT
//! exercised by the round-trip — the parser never constructs
//! an empty `path`. The corpus seed under
//! `fuzz/corpus/registry_id/` covers the unit-test vectors
//! from `voxora-registry/src/id.rs::tests` so regression
//! crashes start from a known-good baseline.
//!
//! Run for 60 s with:
//!
//! ```text
//! cargo +nightly fuzz run registry_id -- -max_total_time=60
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use voxora_registry::ModelId;

fuzz_target!(|data: &[u8]| {
    // The parser takes `&str`. Lossy UTF-8 conversion is
    // acceptable here: any byte sequence is a valid input,
    // and we want the fuzzer to explore what the parser
    // does with non-UTF-8 bytes too.
    let s = std::str::from_utf8(data).unwrap_or("");
    if let Ok(id) = ModelId::parse(s) {
        let canonical = id.canonical();
        match ModelId::parse(&canonical) {
            Ok(parsed) => {
                assert_eq!(
                    parsed, id,
                    "round-trip mismatch: {s:?} -> {id:?} -> {parsed:?} \
                     (canonical = {canonical:?})"
                );
            }
            Err(e) => panic!(
                "round-trip failed: {s:?} -> {id:?} parses again as {e:?} \
                 (canonical = {canonical:?})"
            ),
        }
    }
});
