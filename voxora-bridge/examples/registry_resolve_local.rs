//! End-to-end smoke for the new `registry` Cargo feature
//! (closes #145). Demonstrates that `voxora_bridge::Registry`
//! and `RegistryHfExt` reach the chained-source helper without
//! the consumer adding `voxora-registry` as a direct dependency.
//!
//! Run with:
//!
//! ```text
//! cargo run --example registry_resolve_local -p voxora-bridge \
//!     --features registry --features voxora-bridge/whisper
//! ```
//!
//! Like every example in this workspace, the first run downloads
//! ~75 MB of weights into the cache; subsequent runs resolve
//! from disk in milliseconds. The `registry` feature is opt-in;
//! the `whisper` feature flag is required because the example is
//! wired via `required-features = ["registry", "whisper"]` in
//! `voxora-bridge/Cargo.toml` (it shares the engine-feature
//! family with the other registry consumers). No model
//! inference is performed — the example just builds a registry
//! with the HF built-in descriptors and prints the count, so no
//! real audio is required.

#![cfg(all(feature = "registry", feature = "whisper"))]

use std::sync::Arc;

use voxora_bridge::{HuggingFaceSource, ModelSource, Registry, RegistryHfExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source: Arc<dyn ModelSource> = Arc::new(HuggingFaceSource::new()?);
    let registry = Registry::new(source).with_builtin_descriptors();
    println!(
        "Registry built with {} descriptors",
        registry.descriptors().len()
    );
    Ok(())
}
