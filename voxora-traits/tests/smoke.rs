//! Compile-check that the public surface is intact.
//!
//! Every public type in [`voxora_traits`] is imported here. The
//! `#[allow(unused_imports)]` is deliberate: the goal of this test is
//! that the imports compile, not that they are exercised. Touching the
//! public surface in one place keeps this file the single source of
//! truth for "the type is reachable from the crate root".

#![allow(unused_imports)]

use voxora_traits::{
    AsrEngine, AsrError, ModelCapabilities, ModelDescriptor, ModelDir, ModelSource,
    ModelSourceKind, Quantization, QuantizationPreference, ResolveOptions, TranscribeOptions,
    TranscriptionResult, TranscriptionSegment,
};

#[test]
fn smoke_compile_check() {
    // Just need the import to be valid; we don't exercise async here.
    let _ = ModelCapabilities::default();
    let _ = ModelDir::new(
        std::path::PathBuf::from("/tmp/x"),
        ModelSourceKind::Local,
        Quantization::F16,
    );
    let _ = ResolveOptions::default();
    let _ = TranscribeOptions::default();
}
