//! `voxora download <hf-model-id>` subcommand.

use clap::Args;
use voxora_core::ModelSource;

use crate::args::Cli;
use crate::error::CliError;
use crate::resolve_opts::build_resolve_opts;

#[derive(Debug, Args)]
pub struct DownloadOpts {
    /// The Hugging Face model id (e.g. `Qwen/Qwen3-ASR-0.6B`).
    pub model_id: String,

    /// Pin a specific git revision (branch, tag, or SHA).
    #[arg(long, value_name = "REV")]
    pub revision: Option<String>,

    /// Preferred quantization (`auto` lets voxora-hf pick).
    #[arg(long, value_name = "Q", default_value = "auto")]
    pub quantization: String,
}

pub async fn run(cli: &Cli, opts: &DownloadOpts) -> Result<(), CliError> {
    if !opts.model_id.contains('/') {
        return Err(CliError::InvalidInput(format!(
            "model id {:?} must be in 'org/name' form",
            opts.model_id
        )));
    }

    let source = cli.build_source()?;
    let resolve_opts = build_resolve_opts(cli.token.as_deref(), opts.revision.as_deref(), |ro| {
        ro.quantization = parse_quantization(&opts.quantization)?;
        Ok(())
    })?;

    if !cli.quiet {
        eprintln!(
            "voxora download: resolving {} (revision {})",
            opts.model_id,
            opts.revision.as_deref().unwrap_or("main")
        );
    }

    let dir = source.resolve(&opts.model_id, &resolve_opts).await?;

    let bytes = crate::output::dir_size_bytes(&dir.path);
    if !cli.quiet {
        eprintln!(
            "voxora download: cached at {} ({}, {:.2} MiB)",
            dir.path.display(),
            dir.kind.tag(),
            bytes as f64 / (1024.0 * 1024.0)
        );
    }
    println!("{}", dir.path.display());
    Ok(())
}

pub(crate) fn parse_quantization(
    raw: &str,
) -> Result<voxora_core::QuantizationPreference, CliError> {
    use voxora_core::QuantizationPreference as Qp;
    let lc = raw.to_ascii_lowercase();
    Ok(match lc.as_str() {
        "auto" => Qp::Auto,
        "f32" => Qp::F32,
        "bf16" | "bfloat16" => Qp::Bf16,
        "f16" | "float16" => Qp::F16,
        "q4_k" | "q4k" | "q4_k_m" | "q4_k_s" => Qp::Q4K,
        "q8_0" | "q8" => Qp::Q8_0,
        other => {
            return Err(CliError::InvalidInput(format!(
                "unknown --quantization value {other:?}; expected one of \
                 auto|f32|bf16|f16|q4_k|q8_0"
            )));
        }
    })
}

#[cfg(test)]
mod tests;
