//! Library-API smoke for `voxora-config`: print the resolved cache
//! root, HF token, HF base URL, and default HF revision with all
//! overrides applied. Useful for debugging "why is my config not
//! picked up?".
//!
//! By default the example builds a [`VoxoraConfig`] via
//! `VoxoraConfig::default()` so every layer of the cascade
//! (built-in defaults + environment) participates. Pass a single
//! positional argument to load a TOML file via
//! [`VoxoraConfig::from_file`] instead — useful for verifying that
//! a `voxora.toml` is read at the path you expect.
//!
//! The example never *writes* to the environment, so it is safe to
//! run inside CI or alongside other voxora binaries.
//!
//! Run with:
//!
//! ```text
//! cargo run --example env_cascade -p voxora-config
//! cargo run --example env_cascade -p voxora-config -- voxora.toml
//! ```

use voxora_config::VoxoraConfig;

fn print_resolved(label: &str, cfg: &VoxoraConfig) {
    println!("[{label}]");
    println!("  cache_root            : {}", cfg.cache_root().display());
    println!(
        "  hf_token              : {}",
        cfg.hf_token().as_deref().unwrap_or("<none>")
    );
    println!("  hf_base_url           : {}", cfg.hf_base_url());
    println!("  hf_default_revision   : {}", cfg.hf_default_revision());
    println!();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = std::env::args().nth(1);

    match arg {
        Some(path) => {
            let cfg = VoxoraConfig::from_file(std::path::Path::new(&path))?;
            print_resolved(&format!("from_file({path})"), &cfg);
        }
        None => {
            let cfg = VoxoraConfig::default();
            print_resolved("default()", &cfg);
        }
    }

    Ok(())
}
