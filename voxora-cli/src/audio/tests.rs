//! Unit tests for the WAV decoder.

use super::*;

fn write_i16_wav(path: &Path, samples: &[i16], channels: u16, sample_rate: u32) {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .expect("create wav");
    for s in samples {
        writer.write_sample(*s).expect("write sample");
    }
    writer.finalize().expect("finalize");
}

fn write_f32_wav(path: &Path, samples: &[f32], channels: u16, sample_rate: u32) {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .expect("create wav");
    for s in samples {
        writer.write_sample(*s).expect("write sample");
    }
    writer.finalize().expect("finalize");
}

#[test]
fn decode_wav_handles_mono_i16() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("mono.wav");
    write_i16_wav(&path, &[100, 200, -300, 400], 1, 16_000);

    let audio = decode_wav(&path).expect("decode");
    assert_eq!(audio.sample_rate, 16_000);
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.samples.len(), 4);
    // Samples must be in [-1, 1] (i.e. divided by i16::MAX max).
    for s in &audio.samples {
        assert!((-1.0..=1.0).contains(s), "out of range: {s}");
    }
    // Sign preserved.
    assert!(audio.samples[2] < 0.0, "negative sample lost sign");
}

#[test]
fn decode_wav_handles_stereo_and_downmixes_to_mono() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("stereo.wav");
    // 2 channels × 4 frames.
    write_i16_wav(
        &path,
        &[1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000],
        2,
        32_000,
    );

    let audio = decode_wav(&path).expect("decode");
    assert_eq!(audio.channels, 2, "original channel count preserved");
    assert_eq!(audio.sample_rate, 32_000);
    assert_eq!(audio.samples.len(), 4, "downmixed to one sample per frame");
}

#[test]
fn decode_wav_handles_f32_samples() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("float.wav");
    write_f32_wav(&path, &[0.0, 0.25, -0.5, 0.75, 1.0], 1, 22_050);

    let audio = decode_wav(&path).expect("decode");
    assert_eq!(audio.channels, 1);
    assert_eq!(audio.sample_rate, 22_050);
    assert_eq!(audio.samples.len(), 5);
}

#[test]
fn decode_wav_errors_on_missing_file() {
    let err = decode_wav(Path::new("/nonexistent/file.wav")).unwrap_err();
    assert!(matches!(err, CliError::Asr(AsrError::AudioIo { .. })));
    assert_eq!(err.exit_code(), 1, "I/O is a runtime failure (exit 1)");
}

#[test]
fn decode_wav_errors_on_non_wav_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("not-a-wav.txt");
    std::fs::write(&path, b"definitely not a wav").expect("write");
    let err = decode_wav(&path).unwrap_err();
    assert!(matches!(err, CliError::Asr(_)));
}
