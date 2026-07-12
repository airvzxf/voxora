//! Pretty-printers for every subcommand's output. Centralised here so
//! the formatting is testable (unit tests assert on the rendered
//! substrings) and the `voxora-cli` crate never writes output from
//! more than one place.

use std::path::Path;

use voxora_core::{ModelCapabilities, TranscriptionResult};

use voxora_hf::cache::CachedModel;

/// Number of bytes in `path` summed across regular files. Non-recursive
/// (does not enter sub-directories) because the cache layout puts
/// every file at the top level of the model directory.
pub fn dir_size_bytes(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    let mut total = 0u64;
    for entry in entries.flatten() {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

/// Print the result of `voxora list`.
pub fn print_cached_models(entries: &[CachedModel]) {
    println!("{}", render_cached_table(entries));
}

/// Render the cached-models table as a String. Split out from
/// `print_cached_models` so unit tests can assert against it.
pub fn render_cached_table(entries: &[CachedModel]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "{:<60} {:>10}  {:>6}  DONE", "PATH", "BYTES", "FILES");
    for entry in entries {
        let _ = writeln!(
            out,
            "{:<60} {:>10}  {:>6}  {}",
            entry.path.display().to_string(),
            format_bytes(entry.bytes_total),
            entry.file_count,
            entry.complete_marker_present,
        );
    }
    out
}

/// Print the result of `voxora info <model_id>`.
pub fn print_info(model_id: &str, source_name: &str, caps: &ModelCapabilities) {
    println!("{}", render_info(model_id, source_name, caps));
}

/// Render the info block as a String.
pub fn render_info(model_id: &str, source_name: &str, caps: &ModelCapabilities) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "model_id      : {model_id}");
    let _ = writeln!(out, "source        : {source_name}");
    let _ = writeln!(out, "multilingual  : {}", caps.multilingual);
    let _ = writeln!(out, "word_timestamps: {}", caps.word_timestamps);
    let _ = writeln!(out, "streaming     : {}", caps.streaming);
    let _ = writeln!(out, "languages ({}) :", caps.languages.len());
    for lang in &caps.languages {
        let _ = writeln!(out, "    - {lang}");
    }
    out
}

/// Print a transcription result to stdout, with per-segment
/// timestamps on stderr (matching the convention the engine crate
/// examples already use).
pub fn print_transcription(result: &TranscriptionResult) {
    println!("{}", result.text);
    if !result.segments.is_empty() {
        eprintln!("---");
        for seg in &result.segments {
            let start = seg.start_sample as f64 / 16_000.0;
            let end = seg.end_sample as f64 / 16_000.0;
            eprintln!("[{start:7.2}s - {end:7.2}s] {}", seg.text);
        }
    }
    if let Some(lang) = &result.language {
        eprintln!("language: {lang}");
    }
}

/// Human-readable byte formatter. Kept identical to
/// `voxora-hf/examples/inspect.rs::human_bytes` for visual consistency
/// across voxora's CLI surface.
pub fn format_bytes(n: u64) -> String {
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

#[cfg(test)]
mod tests;
