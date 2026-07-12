//! `voxora run <hf-model-id> <audio.wav>` subcommand.

use std::path::PathBuf;

use clap::Args;
use clap::ValueHint;

use crate::args::Cli;
use crate::error::CliError;
use crate::resolve_opts::build_resolve_opts;

#[derive(Debug, Args)]
pub struct RunOpts {
    /// Hugging Face model id (e.g. `Qwen/Qwen3-ASR-0.6B`).
    pub model_id: String,

    /// Path to a mono (or stereo) PCM WAV file.
    #[arg(value_hint = ValueHint::FilePath)]
    pub audio: PathBuf,

    /// Pin a specific git revision (branch, tag, or SHA).
    #[arg(long, value_name = "REV")]
    pub revision: Option<String>,

    /// Force a specific engine.
    ///
    /// When omitted, voxora-cli inspects `config.json` to pick the
    /// right engine (Whisper vs Qwen3-ASR).
    #[arg(long, value_name = "ENGINE")]
    pub engine: Option<String>,

    /// ISO 639-1 language code (whisper) or full English name
    /// (qwen3-asr, e.g. `english`). When omitted the engine
    /// auto-detects.
    #[arg(long, value_name = "LANG")]
    pub language: Option<String>,

    /// Ask the engine to translate the output to English (multilingual
    /// models only).
    #[arg(long)]
    pub translate: bool,

    /// Emit per-segment timestamps to stderr (whisper only; qwen3-asr
    /// always returns empty segments and a warning is printed).
    #[arg(long)]
    pub timestamps: bool,

    /// Reserved for `--force-redownload` semantics in a future release.
    /// Accepts the flag silently so old invocations don't break.
    #[arg(long, hide = true)]
    pub force_redownload: bool,
}

pub async fn run(cli: &Cli, opts: &RunOpts) -> Result<(), CliError> {
    if !opts.model_id.contains('/') {
        return Err(CliError::InvalidInput(format!(
            "model id {:?} must be in 'org/name' form",
            opts.model_id
        )));
    }

    // Validate `--engine` (and the build-time feature) up front so
    // bad values are rejected before any network call or audio I/O.
    if let Some(label) = opts.engine.as_deref() {
        let kind = crate::engine::BackendKind::from_cli_label(label)?;
        crate::engine::ensure_available(kind, label)?;
    }

    let source = cli.build_source()?;
    let resolve_opts =
        build_resolve_opts(cli.token.as_deref(), opts.revision.as_deref(), |_| Ok(()))?;

    // Audio after a valid `--engine` value so users get the engine
    // error first if both are wrong (faster, clearer message).
    let audio = crate::audio::decode_wav(&opts.audio)?;

    // Auto-detect or honour the user's `--engine` flag.
    let engine_kind = crate::engine::select(
        cli,
        opts.engine.as_deref(),
        &source,
        &opts.model_id,
        opts.force_redownload,
    )
    .await?;

    if !cli.quiet {
        eprintln!(
            "voxora run: loaded {} ({} Hz, {} ch), {} mono samples ({:.2} s)",
            opts.model_id,
            audio.sample_rate,
            audio.channels,
            audio.samples.len(),
            audio.samples.len() as f64 / audio.sample_rate as f64,
        );
        eprintln!("voxora run: backend = {}", engine_kind.label());
    }

    let transcribe_opts =
        voxora_core::TranscribeOptions::new(opts.language.clone(), opts.translate, opts.timestamps);

    let result = crate::engine::run(
        engine_kind,
        &source,
        &opts.model_id,
        &resolve_opts,
        &audio.samples,
        &transcribe_opts,
    )
    .await
    .map_err(CliError::Asr)?;

    if engine_kind == crate::engine::BackendKind::Qwen3Asr && opts.timestamps && !cli.quiet {
        eprintln!(
            "voxora run: note — qwen3-asr does not emit per-segment boundaries; \
             --timestamps produced an empty segment list."
        );
    }

    crate::output::print_transcription(&result);
    Ok(())
}
