//! `voxora run <hf-model-id> <audio.wav>` subcommand.

use std::path::PathBuf;

use clap::Args;
use clap::ValueHint;
use voxora_engine::BackendKind;

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

    /// Hardware backend the binary was compiled for.
    ///
    /// Validates against the Cargo features enabled at build time
    /// (`cpu` is always on; `cuda` / `metal` / `vulkan` require the
    /// matching `--features` flag at `cargo build`). Does NOT
    /// influence engine dispatch — whisper-rs picks its own runtime
    /// backend from the compiled Cargo features — so the flag is
    /// build-time validation + observability, not a runtime
    /// selector. `--hardware vulkan` against `--engine qwen3-asr`
    /// fails fast because qwen3-asr upstream has no Vulkan
    /// backend.
    #[arg(long, value_name = "HARDWARE", ignore_case = true)]
    pub hardware: Option<BackendKindArg>,

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

/// `clap`-parseable wrapper around [`BackendKind`].
///
/// We can't derive `ValueEnum` directly on `BackendKind` because it
/// lives in `voxora-engine` and adding a `clap` dependency there
/// would leak the CLI surface into the library. This thin shim
/// round-trips through [`BackendKind::from_config`] so the
/// vocabulary stays in lock-step with the engine crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendKindArg(pub BackendKind);

impl clap::ValueEnum for BackendKindArg {
    fn value_variants<'a>() -> &'a [Self] {
        // Static slice of every variant known to clap's parser.
        // Order matches the user-facing help output (cpu, cuda,
        // metal, vulkan).
        const VARIANTS: &[BackendKindArg] = &[
            BackendKindArg(BackendKind::Cpu),
            BackendKindArg(BackendKind::Cuda),
            BackendKindArg(BackendKind::Metal),
            BackendKindArg(BackendKind::Vulkan),
        ];
        VARIANTS
    }

    fn from_str(input: &str, ignore_case: bool) -> Result<Self, String> {
        // Honour the clap `ignore_case` knob explicitly. The default
        // is `false`, but `BackendKind::from_config` is always
        // case-insensitive under the hood — so when the caller
        // opts in to case-insensitive matching we lowercase and
        // forward, and when they opt out we still pass through
        // (and rely on `from_config`'s lowercasing to do the same
        // job). The downstream matcher is the source of truth.
        let normalized = if ignore_case {
            input.to_ascii_lowercase()
        } else {
            input.to_owned()
        };
        BackendKind::from_config(&normalized)
            .map(BackendKindArg)
            .ok_or_else(|| {
                format!(
                    "unknown hardware backend {input:?}; \
                     expected one of `cpu`, `cuda`, `metal`, `vulkan`"
                )
            })
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.0.as_config()))
    }
}

impl std::fmt::Display for BackendKindArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_config())
    }
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
        let kind = crate::engine::from_cli_label(label)?;
        crate::engine::ensure_available(kind, label)?;
    }

    // Validate `--hardware` against the build-time Cargo features
    // before doing anything else. This is build-time validation +
    // observability; whisper-rs picks its own runtime backend from
    // the compiled Cargo features and there is no upstream
    // per-call GPU switch today (closes #121).
    //
    // We keep the user-original input (preserved by clap after the
    // case-insensitive lookup) to surface in error messages — `arg`
    // derives `Display` from `BackendKind::as_config` which is
    // always lowercase, so we would otherwise lose the casing the
    // user typed if it differed from the canonical spelling.
    let hardware_kind: Option<(BackendKind, String)> = if let Some(arg) = opts.hardware {
        let kind = arg.0;
        let label = arg.to_string();
        crate::engine::ensure_hardware_available(kind, &label)?;
        Some((kind, label))
    } else {
        None
    };

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

    // Cross-validate `--hardware` against the resolved engine. Vulkan
    // is whisper-only; qwen3-asr upstream has no Vulkan backend.
    if let Some((kind, label)) = &hardware_kind {
        crate::engine::ensure_hardware_compatible_with_engine(*kind, engine_kind, label)?;
    }

    if !cli.quiet {
        eprintln!(
            "voxora run: loaded {} ({} Hz, {} ch), {} mono samples ({:.2} s)",
            opts.model_id,
            audio.sample_rate,
            audio.channels,
            audio.samples.len(),
            audio.samples.len() as f64 / audio.sample_rate as f64,
        );
        eprintln!(
            "voxora run: backend = {}, hardware = {}",
            engine_kind.crate_label(),
            hardware_kind
                .as_ref()
                .map(|(_, label)| label.as_str())
                .unwrap_or("(compiled-in default)")
        );
    }

    let transcribe_opts = voxora_traits::TranscribeOptions::new(
        opts.language.clone(),
        opts.translate,
        opts.timestamps,
    );

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

    if engine_kind == voxora_engine::EngineFamily::Qwen3Asr && opts.timestamps && !cli.quiet {
        eprintln!(
            "voxora run: note — qwen3-asr does not emit per-segment boundaries; \
             --timestamps produced an empty segment list."
        );
    }

    crate::output::print_transcription(&result);
    Ok(())
}

#[cfg(test)]
mod tests;
