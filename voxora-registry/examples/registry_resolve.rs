//! Library-API smoke for `voxora-registry`: build a [`Registry`]
//! with both built-in descriptors, resolve a model id, and print the
//! descriptor + on-disk [`voxora_traits::ModelDir`] the resolver
//! chose.
//!
//! Run with:
//!
//! ```text
//! cargo run --example registry_resolve -p voxora-registry -- \
//!     ggerganov/whisper.cpp/ggml-tiny.bin
//! ```
//!
//! Like every example in this workspace, the first run downloads
//! ~75 MB of weights into the cache; subsequent runs resolve from
//! disk in milliseconds. The `hf` feature is on by default but is
//! pinned by `required-features = ["hf"]` in `Cargo.toml` so the
//! example still builds under `--no-default-features`.

#[cfg(feature = "hf")]
use std::sync::Arc;

#[cfg(feature = "hf")]
use voxora_hf::HuggingFaceSource;
#[cfg(feature = "hf")]
use voxora_registry::{ModelId, Registry, RegistryHfExt};
#[cfg(feature = "hf")]
use voxora_traits::{ModelSource, ResolveOptions};

#[cfg(feature = "hf")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_id = std::env::args()
        .nth(1)
        .ok_or("usage: registry_resolve <hf-model-id>")?;

    let source: Arc<dyn ModelSource> = Arc::new(HuggingFaceSource::new()?);
    let registry = Registry::new(source).with_builtin_descriptors();

    let id = ModelId::parse(&model_id)?;
    let resolved = registry.resolve(&id, &ResolveOptions::default()).await?;

    println!("input          : {model_id}");
    println!(
        "descriptor     : {} ({})",
        resolved.descriptor.label, resolved.descriptor.family
    );
    println!("model_dir.path : {}", resolved.model_dir.path.display());
    match resolved.model_dir.entry.as_ref() {
        Some(entry) => println!("model_dir.entry: {}", entry.display()),
        None => println!("model_dir.entry: <none>"),
    }
    println!("source_kind    : {}", resolved.model_dir.kind.tag());
    println!("quantization   : {:?}", resolved.model_dir.quantization);

    Ok(())
}

#[cfg(not(feature = "hf"))]
fn main() {
    eprintln!(
        "this example requires the `hf` feature: cargo run --example registry_resolve -p voxora-registry --features voxora-registry/hf -- <hf-model-id>"
    );
    std::process::exit(2);
}
