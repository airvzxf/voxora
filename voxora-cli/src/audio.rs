//! WAV decoding + downmix-to-mono for the `voxora run` path.

use std::path::Path;

use voxora_core::AsrError;

use crate::error::CliError;

/// Decoded audio ready to be fed into a [`voxora_core::AsrEngine`].
#[derive(Debug)]
pub struct DecodedAudio {
    /// Mono `f32` samples in `[-1.0, 1.0]` at the WAV's native rate.
    pub samples: Vec<f32>,
    /// Original sample rate in Hz (typically 16 000).
    pub sample_rate: u32,
    /// Original channel count before downmix (1 or 2 typically).
    pub channels: u16,
}

/// Open and decode the WAV at `path` to mono `f32` samples at the
/// file's native sample rate. We do **not** resample — every engine
/// in the workspace documents 16 kHz as the assumed rate and handles
/// other rates internally.
pub fn decode_wav(path: &Path) -> Result<DecodedAudio, CliError> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|e| CliError::Asr(AsrError::audio_io(path.to_path_buf(), std_to_io(e))))?;
    let spec = reader.spec();

    // Reject obviously-broken inputs (zero-channel files, etc.) before
    // we descend into the downmix loop.
    if spec.channels == 0 {
        return Err(CliError::Asr(AsrError::audio_io(
            path.to_path_buf(),
            std::io::Error::new(std::io::ErrorKind::InvalidData, "WAV has zero channels"),
        )));
    }

    let bits = spec.bits_per_sample;
    let raw: Vec<i32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .collect::<Result<_, _>>()
            .map_err(|e| CliError::Asr(AsrError::audio_io(path.to_path_buf(), std_to_io(e))))?,
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.map(|f| (f.clamp(-1.0, 1.0) * i32::MAX as f32) as i32))
            .collect::<Result<_, _>>()
            .map_err(|e| CliError::Asr(AsrError::audio_io(path.to_path_buf(), std_to_io(e))))?,
    };

    let ch = spec.channels as usize;
    let samples = if ch == 1 {
        raw.into_iter().map(i32_to_f32).collect()
    } else {
        raw.chunks(ch)
            .map(|frame| {
                let sum: i64 = frame.iter().map(|&v| v as i64).sum();
                i32_to_f32((sum / ch as i64) as i32)
            })
            .collect()
    };

    let _ = bits; // currently ignored; amplitude scaling below handles the rest.

    Ok(DecodedAudio {
        samples,
        sample_rate: spec.sample_rate,
        channels: spec.channels,
    })
}

/// Map an `i32` PCM sample (any width, 16/24/32-bit) into `[-1.0, 1.0]`
/// `f32`, using the full i32 range as the denominator so 24-bit files
/// don't clip.
fn i32_to_f32(v: i32) -> f32 {
    v as f32 / i32::MAX as f32
}

/// `hound` exposes its own `hound::Error`. We only use its `Display`
/// form, then synthesise a matching `std::io::Error` so we can reuse
/// `AsrError::audio_io` for the I/O path.
fn std_to_io(e: hound::Error) -> std::io::Error {
    match e {
        hound::Error::IoError(io) => io,
        other => std::io::Error::other(other.to_string()),
    }
}

#[cfg(test)]
mod tests;
