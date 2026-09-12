//! In-memory WAV header writer for `f32` LE PCM payloads.
//!
//! MiniMax accepts `wav` / `aiff` / `flac` / `alac(m4a)` / `mp3` /
//! `aac` / `opus` / `ogg`. We pick WAV as the simplest container
//! the crate can produce without pulling in an encoder dependency:
//! the format is a 44-byte header followed by interleaved samples,
//! and the only thing voxora needs to encode is `f32` little-endian
//! PCM.
//!
//! The format spec is RIFF/WAVE. We always emit the canonical
//! 16-byte `fmt ` chunk with `audio_format = 3` (IEEE float) and a
//! 16-byte `data` chunk — the body length is computed from the
//! sample count, not passed in.
//!
//! Reference:
//! <http://soundfile.sapp.org/doc/WaveFormat/>

/// Encode `samples` as a WAV byte buffer at `sample_rate` Hz and
/// `channels` (1 or 2).
///
/// The caller is responsible for ensuring the channel layout of
/// `samples` matches `channels` (mono = `len` samples; stereo =
/// `2 * len` interleaved). The encoder does not validate this
/// contract — passing the wrong length silently produces a
/// malformed file (a half-empty trailing channel for stereo with a
/// mono-sized buffer). The MiniMax endpoint tolerates the wrong
/// shape up to its size cap, but the transcription quality is
/// undefined. Downmix yourself.
pub fn write_wav_pcm_f32(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<u8> {
    // RIFF chunk: 12 bytes total. `ChunkSize` is the file size minus
    // 8 (the RIFF header itself), per the spec.
    //
    // `data_size` is the byte count of the data chunk payload —
    // `samples.len() * 4 * channels`. The caller is responsible
    // for ensuring the `samples` length matches the channel
    // layout (mono = `len` samples; stereo = `2 * len` interleaved
    // samples). Passing the wrong shape silently produces a
    // malformed file; the MiniMax endpoint tolerates the wrong
    // shape up to its size cap but transcription quality is
    // undefined.
    let byte_rate = sample_rate * u32::from(channels) * 4;
    let block_align = channels * 4;
    let data_size = (samples.len() * 4) as u32;
    let chunk_size = 36 + data_size;

    let mut buf = Vec::with_capacity(44 + samples.len() * 4);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&chunk_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // `fmt ` sub-chunk (16 bytes payload).
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16_u32.to_le_bytes()); // PCM/float fmt size
    buf.extend_from_slice(&3_u16.to_le_bytes()); // audio_format = 3 (IEEE float)
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&block_align.to_le_bytes());
    buf.extend_from_slice(&32_u16.to_le_bytes()); // bits per sample

    // `data` sub-chunk.
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_size.to_le_bytes());
    for s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_layout_matches_spec() {
        // 4 samples at 16 kHz mono: 44-byte header + 16-byte body.
        let buf = write_wav_pcm_f32(&[0.0; 4], 16_000, 1);
        assert_eq!(buf.len(), 44 + 16);
        assert_eq!(&buf[0..4], b"RIFF");
        assert_eq!(&buf[8..12], b"WAVE");
        assert_eq!(&buf[12..16], b"fmt ");
        // audio_format = 3 (IEEE float)
        assert_eq!(u16::from_le_bytes([buf[20], buf[21]]), 3);
        // channels = 1
        assert_eq!(u16::from_le_bytes([buf[22], buf[23]]), 1);
        // sample_rate = 16000
        assert_eq!(
            u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]),
            16_000
        );
        // bits per sample = 32
        assert_eq!(u16::from_le_bytes([buf[34], buf[35]]), 32);
        assert_eq!(&buf[36..40], b"data");
    }

    #[test]
    fn chunk_size_accounts_for_data_bytes() {
        let buf = write_wav_pcm_f32(&[0.0; 4], 16_000, 1);
        // RIFF chunk size = file_size - 8 = (44 + 16) - 8 = 52.
        assert_eq!(u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]), 52);
    }

    #[test]
    fn stereo_doubles_data_bytes() {
        // 8 stereo samples (interleaved L/R for 4 frames).
        let stereo = write_wav_pcm_f32(&[0.0; 8], 16_000, 2);
        // 8 mono samples.
        let mono = write_wav_pcm_f32(&[0.0; 8], 16_000, 1);
        // Same frame count → same data byte count.
        assert_eq!(mono.len(), stereo.len());
        // And the byte_rate in the header is 2x for stereo.
        assert_eq!(
            u32::from_le_bytes([mono[28], mono[29], mono[30], mono[31]]),
            16_000 * 4
        );
        assert_eq!(
            u32::from_le_bytes([stereo[28], stereo[29], stereo[30], stereo[31]]),
            16_000 * 4 * 2
        );
        // block_align is 4 for mono, 8 for stereo.
        assert_eq!(u16::from_le_bytes([mono[32], mono[33]]), 4);
        assert_eq!(u16::from_le_bytes([stereo[32], stereo[33]]), 8);
    }

    #[test]
    fn samples_round_trip_through_hound() {
        // Write some samples, decode with hound, confirm we get
        // back roughly the same f32 values (PCM f32 is a lossless
        // container).
        let samples: Vec<f32> = (0..32).map(|i| (i as f32 / 32.0) * 2.0 - 1.0).collect();
        let buf = write_wav_pcm_f32(&samples, 16_000, 1);
        let cursor = std::io::Cursor::new(buf);
        let mut reader = hound::WavReader::new(cursor).expect("hound reader");
        assert_eq!(reader.spec().sample_rate, 16_000);
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().bits_per_sample, 32);
        let decoded: Vec<f32> = reader
            .samples::<f32>()
            .map(|s| s.expect("decode sample"))
            .collect();
        assert_eq!(decoded, samples);
    }
}
