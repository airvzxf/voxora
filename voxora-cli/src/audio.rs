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
///
/// PCM amplitude scaling honours the WAV's declared bit depth so
/// 16-bit / 24-bit / 32-bit integer WAVs all normalise into the
/// expected `[-1.0, 1.0]` f32 range. A 16-bit WAV with full-scale
/// 0 dBFS samples hits ±1.0; a 24-bit WAV hits ±1.0; a 32-bit WAV
/// hits ±1.0. (The previous implementation divided every PCM value
/// by `i32::MAX`, which made 16-bit audio 65536× too quiet — engines
/// then saw what looked like silence.)
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

    // Read each frame as i32 with the bit depth honoured, then
    // downmix to mono. Float WAVs (32-bit IEEE float) are converted
    // to the equivalent integer range before the same downmix path.
    let ch = spec.channels as usize;
    let mono: Vec<i32> = match spec.sample_format {
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => {
                let mut acc: Vec<i32> = Vec::with_capacity(/* will refine */ 0);
                // Iterate per frame (ch samples) so downmix averages over
                // the interleaved channels, not a sliding window.
                read_frames_int::<i16, _>(&mut reader, ch, |v| v as i32, &mut acc)
                    .map_err(|e| CliError::Asr(AsrError::audio_io(path.to_path_buf(), e)))?;
                acc
            }
            24 => {
                let mut acc: Vec<i32> = Vec::with_capacity(0);
                read_frames_int::<i32, _>(&mut reader, ch, |v| v, &mut acc)
                    .map_err(|e| CliError::Asr(AsrError::audio_io(path.to_path_buf(), e)))?;
                acc
            }
            32 => {
                let mut acc: Vec<i32> = Vec::with_capacity(0);
                read_frames_int::<i32, _>(&mut reader, ch, |v| v, &mut acc)
                    .map_err(|e| CliError::Asr(AsrError::audio_io(path.to_path_buf(), e)))?;
                acc
            }
            other => {
                return Err(CliError::Asr(AsrError::audio_io(
                    path.to_path_buf(),
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unsupported bit depth: {other} (expected 16/24/32)"),
                    ),
                )));
            }
        },
        hound::SampleFormat::Float => {
            // hound exposes f32 WAVs as `samples::<f32>()`. Convert to
            // the equivalent i32 range so the downmix / scaling logic
            // is identical to the integer path.
            let mut acc: Vec<i32> = Vec::with_capacity(0);
            read_frames_float::<f32, _>(
                &mut reader,
                ch,
                |f| (f.clamp(-1.0, 1.0) * i32::MAX as f32) as i32,
                &mut acc,
            )
            .map_err(|e| CliError::Asr(AsrError::audio_io(path.to_path_buf(), e)))?;
            acc
        }
    };

    let samples: Vec<f32> = mono
        .into_iter()
        .map(|v| pcm_to_f32(v, spec.bits_per_sample, spec.sample_format))
        .collect();

    Ok(DecodedAudio {
        samples,
        sample_rate: spec.sample_rate,
        channels: spec.channels,
    })
}

/// Iterate the WAV as `T` (the hound sample type), average `ch` samples
/// per frame (downmix), cast each averaged sample via `cast`, and push
/// the result into `acc`.
fn read_frames_int<T, F>(
    reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>,
    ch: usize,
    cast: F,
    acc: &mut Vec<i32>,
) -> Result<(), std::io::Error>
where
    T: hound::Sample + Copy + Into<i32>,
    F: Fn(T) -> i32,
{
    let mut iter = reader.samples::<T>();
    loop {
        let mut sum: i64 = 0;
        let mut got = 0;
        for _ in 0..ch {
            match iter.next() {
                Some(Ok(v)) => {
                    sum += cast(v) as i64;
                    got += 1;
                }
                Some(Err(e)) => return Err(std_to_io(e)),
                None => break,
            }
        }
        if got == 0 {
            break;
        }
        acc.push((sum / got as i64) as i32);
    }
    Ok(())
}

/// Same as [`read_frames_int`] but for float WAVs.
fn read_frames_float<T, F>(
    reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>,
    ch: usize,
    cast: F,
    acc: &mut Vec<i32>,
) -> Result<(), std::io::Error>
where
    T: hound::Sample + Copy,
    F: Fn(T) -> i32,
{
    let mut iter = reader.samples::<T>();
    loop {
        let mut sum: i64 = 0;
        let mut got = 0;
        for _ in 0..ch {
            match iter.next() {
                Some(Ok(v)) => {
                    sum += cast(v) as i64;
                    got += 1;
                }
                Some(Err(e)) => return Err(std_to_io(e)),
                None => break,
            }
        }
        if got == 0 {
            break;
        }
        acc.push((sum / got as i64) as i32);
    }
    Ok(())
}

/// Normalise a single PCM sample (already cast to i32) into
/// `[-1.0, 1.0]` `f32`, honouring the WAV's declared bit depth and
/// sample format. Float WAVs are pre-multiplied into the i32 range
/// by the caller and go through the same `i32::MAX` divisor here.
fn pcm_to_f32(v: i32, bits_per_sample: u16, sample_format: hound::SampleFormat) -> f32 {
    match sample_format {
        hound::SampleFormat::Float => v as f32 / i32::MAX as f32,
        hound::SampleFormat::Int => {
            // Use 2^(bits-1) as the divisor (full symmetric range).
            // For 16-bit that is 32768, not i16::MAX (32767), so a
            // full-scale negative sample maps to exactly -1.0.
            let denom = 1i64 << (bits_per_sample.saturating_sub(1) as i64);
            (v as f64 / denom as f64) as f32
        }
    }
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
