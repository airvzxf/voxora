//! `voxora list` subcommand: enumerate the models cached on disk
//! under the cache root.

use crate::args::Cli;

/// Pretty-print the locally cached models. Exits 0 even when no models
/// are found, so callers can pipe the output through `grep` etc.
pub fn run(cli: &Cli) -> Result<(), crate::error::CliError> {
    let cache_root = cli.hf_cache_dir().ok_or_else(|| {
        crate::error::CliError::InvalidInput(
            "could not determine a cache root (set --cache or VOXORA_CACHE_DIR)".into(),
        )
    })?;
    let entries = voxora_hf::cache::list_cached(&cache_root)?;

    if entries.is_empty() {
        if !cli.quiet {
            eprintln!("voxora list: no models under {}", cache_root.display());
        }
        return Ok(());
    }

    crate::output::print_cached_models(&entries);
    Ok(())
}
