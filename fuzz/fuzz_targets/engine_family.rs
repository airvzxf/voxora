//! Fuzz target: `voxora-engine::family::EngineFamily::from_config`.
//!
//! ## Property under test (closes #54, EPIC #133)
//!
//! ```text
//! for all s: &str:
//!     let lowered = s.to_ascii_lowercase();
//!     matches!(lowered.as_str(), "whisper" | "qwen3-asr" | "qwen3asr" | "qwen3_asr")
//!         == EngineFamily::from_config(s).is_some()
//! ```
//!
//! The four canonical literals are the only inputs that
//! must return `Some(_)`. Everything else returns `None`.
//! Today `from_config` is a single `match` with an exhaustive
//! list of accepted aliases; the fuzzer's job is to keep it
//! that way — if a future refactor accidentally drops a
//! case or accepts an unrelated string, this target catches
//! the regression in a 60 s nightly run.
//!
//! The corpus seed under `fuzz/corpus/engine_family/` covers
//! the unit-test vectors from
//! `voxora-engine/src/family.rs::tests`:
//!
//! - `whisper`, `qwen3-asr`, `qwen3asr`, `qwen3_asr`
//! - case variants (`WHISPER`, `Qwen3-ASR`)
//! - reject set (`parakeet`, empty string)
//!
//! Run for 60 s with:
//!
//! ```text
//! cargo +nightly fuzz run engine_family -- -max_total_time=60
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use voxora_engine::EngineFamily;

fuzz_target!(|data: &[u8]| {
    let s = std::str::from_utf8(data).unwrap_or("");
    let parsed = EngineFamily::from_config(s);
    let expected_some = matches!(
        s.to_ascii_lowercase().as_str(),
        "whisper" | "qwen3-asr" | "qwen3asr" | "qwen3_asr"
    );
    match (parsed, expected_some) {
        (Some(_), true) => {}
        (None, false) => {}
        (Some(found), false) => panic!(
            "from_config({s:?}) = Some({found:?}) but the lowered input \
             does not match any canonical literal"
        ),
        (None, true) => panic!(
            "from_config({s:?}) = None but the lowered input matches \
             one of the canonical literals"
        ),
    }
});
