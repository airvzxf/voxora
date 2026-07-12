//! Resolve a Hugging Face model and print what we got.
//!
//! This is a smoke program for `voxora-hf`: it downloads a model
//! (or hits the cache if already there), prints the resolved
//! [`voxora_core::ModelDir`] and the capabilities discovered from
//! `config.json`, and lists every file that landed on disk.
//!
//! Use it to verify a Phase 2 install end-to-end:
//!
//! ```text
//! cargo run --example inspect -- openai/whisper-tiny
//! cargo run --example inspect -- Qwen/Qwen3-ASR-0.6B
//! cargo run --example inspect -- Qwen/Qwen3-ASR-1.7B --revision main
//! cargo run --example inspect -- ggerganov/whisper.cpp --cache /tmp/whisper
//! ```
//!
//! Flags:
//!
//! - `--revision REV` — pin a specific git revision (default `main`).
//! - `--cache DIR` — override the cache root. Defaults to
//!   `$XDG_CACHE_HOME/voxora/models/huggingface`.
//! - `--token TOK` — override the bearer token (default: read
//!   `HF_TOKEN` / `HUGGING_FACE_HUB_TOKEN` from env).
//! - `--recompute-caps` — re-fetch `config.json` even if the cache
//!   marker is already in place.
//!
//! Files are streamed; safetensors shards of multi-GB models are
//! supported. Pass `--quiet` to suppress per-file progress.

use std::path::PathBuf;
use std::process::ExitCode;

use voxora_core::{ModelSource, ResolveOptions};
use voxora_hf::HuggingFaceSource;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse_args(&args) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("voxora-hf inspect: {msg}");
            eprintln!();
            eprintln!("usage: inspect <hf-model-id> [--revision REV] [--cache DIR] \\");
            eprintln!("                          [--token TOK] [--recompute-caps] [--quiet]");
            return ExitCode::from(2);
        }
    };

    let mut builder = HuggingFaceSource::builder();
    if let Some(ref rev) = parsed.revision {
        builder = builder.default_revision(rev.clone());
    }
    if let Some(dir) = parsed.cache {
        builder = builder.cache_dir(dir);
    }
    if let Some(token) = parsed.token {
        builder = builder.token(Some(token));
    }
    let source = match builder.build() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("voxora-hf inspect: failed to build source: {e}");
            return ExitCode::from(1);
        }
    };

    let opts = match &parsed.revision {
        Some(r) => ResolveOptions::with_revision(r.clone()),
        None => ResolveOptions::default(),
    };

    let dir = match source.resolve(&parsed.model_id, &opts).await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("voxora-hf inspect: resolve failed: {e}");
            return ExitCode::from(1);
        }
    };

    println!("source      : {}", source.name());
    println!("model_id    : {}", parsed.model_id);
    println!("path        : {}", dir.path.display());
    println!("kind        : {}", dir.kind.tag());
    println!("quantization: {:?}", dir.quantization);

    let caps = match source.capabilities_for(&parsed.model_id).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("voxora-hf inspect: capabilities_for failed: {e}");
            return ExitCode::from(1);
        }
    };
    println!("capabilities:");
    println!("  multilingual   : {}", caps.multilingual);
    println!("  word_timestamps: {}", caps.word_timestamps);
    println!("  streaming      : {}", caps.streaming);
    println!("  languages ({}) :", caps.languages.len());
    if !parsed.quiet {
        for lang in &caps.languages {
            println!("    - {lang}");
        }
    }

    if !parsed.quiet {
        println!("files:");
        let mut entries = match tokio::fs::read_dir(&dir.path).await {
            Ok(e) => e,
            Err(e) => {
                eprintln!(
                    "voxora-hf inspect: read_dir({}) failed: {e}",
                    dir.path.display()
                );
                return ExitCode::from(1);
            }
        };
        let mut rows: Vec<(String, String)> = Vec::new();
        while let Some(entry) = match entries.next_entry().await {
            Ok(e) => e,
            Err(e) => {
                eprintln!("voxora-hf inspect: read_dir entry failed: {e}");
                return ExitCode::from(1);
            }
        } {
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };
            let size = if meta.is_file() {
                human_bytes(meta.len())
            } else {
                "<dir>".into()
            };
            rows.push((name, size));
        }
        rows.sort();
        for (name, size) in rows {
            println!("  {name:<46} {size}");
        }
    }
    ExitCode::SUCCESS
}

/// CLI args after positional `model_id`.
struct Cli {
    model_id: String,
    revision: Option<String>,
    cache: Option<PathBuf>,
    token: Option<String>,
    quiet: bool,
}

fn parse_args(args: &[String]) -> Result<Cli, String> {
    let mut iter = args.iter();
    let model_id = iter
        .next()
        .ok_or_else(|| "missing <hf-model-id>".to_string())?
        .clone();
    if model_id.is_empty() {
        return Err("model id is empty".into());
    }
    if !model_id.contains('/') {
        return Err(format!("model id {model_id:?} must be in 'org/name' form"));
    }

    let mut revision = None;
    let mut cache = None;
    let mut token = None;
    let mut quiet = false;

    while let Some(a) = iter.next() {
        match a.as_str() {
            "--revision" => {
                revision = Some(iter.next().ok_or("--revision needs a value")?.clone());
            }
            "--cache" => {
                cache = Some(PathBuf::from(
                    iter.next().ok_or("--cache needs a value")?.clone(),
                ));
            }
            "--token" => {
                token = Some(iter.next().ok_or("--token needs a value")?.clone());
            }
            "--quiet" => quiet = true,
            "--recompute-caps" => {
                // Reserved for a future phase that caches the
                // capabilities JSON. Today `capabilities_for` always
                // re-fetches `config.json`, so this is a no-op.
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(Cli {
        model_id,
        revision,
        cache,
        token,
        quiet,
    })
}

fn human_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n} {}", UNITS[0])
    } else {
        format!("{v:.2} {}", UNITS[u])
    }
}
