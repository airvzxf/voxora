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

#[test]
fn decode_wav_normalises_16bit_full_scale_to_unit_range() {
    // Regression test for the bug where every PCM value was divided by
    // `i32::MAX`, making 16-bit audio 65536× too quiet (engines then
    // saw what looked like silence). A full-scale 16-bit sample must
    // land near ±1.0, not near 0.
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("fullscale.wav");
    write_i16_wav(&path, &[i16::MAX, i16::MIN, 0, i16::MAX / 2], 1, 16_000);

    let audio = decode_wav(&path).expect("decode");
    assert_eq!(audio.samples.len(), 4);
    assert!(
        audio.samples[0] > 0.99 && audio.samples[0] <= 1.0,
        "i16::MAX must map to ~1.0, got {}",
        audio.samples[0]
    );
    assert!(
        audio.samples[1] >= -1.0 && audio.samples[1] < -0.99,
        "i16::MIN must map to ~-1.0, got {}",
        audio.samples[1]
    );
    assert_eq!(audio.samples[2], 0.0, "silence stays silence");
    assert!(
        audio.samples[3] > 0.49 && audio.samples[3] < 0.51,
        "i16::MAX/2 must map to ~0.5, got {}",
        audio.samples[3]
    );
}

#[test]
fn decode_wav_normalises_stereo_i16_full_scale() {
    // Both channels at full scale → mono average should also be at
    // full scale. Confirms the downmix happens BEFORE normalisation,
    // not after.
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("stereo-fullscale.wav");
    write_i16_wav(&path, &[i16::MAX, i16::MAX, i16::MIN, i16::MIN], 2, 16_000);

    let audio = decode_wav(&path).expect("decode");
    assert_eq!(audio.samples.len(), 2);
    assert!(audio.samples[0] > 0.99, "max+max average must be ~1.0");
    assert!(audio.samples[1] < -0.99, "min+min average must be ~-1.0");
}

fn write_i24_wav(path: &Path, samples: &[i32], channels: u16, sample_rate: u32) {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .expect("create wav");
    for s in samples {
        writer.write_sample(*s).expect("write sample");
    }
    writer.finalize().expect("finalize");
}

#[test]
fn decode_wav_normalises_24bit_full_scale() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("i24.wav");
    let max = (1 << 23) - 1;
    let min = -(1 << 23);
    write_i24_wav(&path, &[max, min, 0], 1, 48_000);

    let audio = decode_wav(&path).expect("decode");
    assert_eq!(audio.samples.len(), 3);
    assert!(audio.samples[0] > 0.99, "24-bit max must map to ~1.0");
    assert!(audio.samples[1] < -0.99, "24-bit min must map to ~-1.0");
    assert_eq!(audio.samples[2], 0.0);
}

fn write_i32_wav(path: &Path, samples: &[i32], channels: u16, sample_rate: u32) {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .expect("create wav");
    for s in samples {
        writer.write_sample(*s).expect("write sample");
    }
    writer.finalize().expect("finalize");
}

#[test]
fn decode_wav_normalises_32bit_full_scale() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("i32.wav");
    write_i32_wav(&path, &[i32::MAX, i32::MIN, 0], 1, 48_000);

    let audio = decode_wav(&path).expect("decode");
    assert_eq!(audio.samples.len(), 3);
    assert!(audio.samples[0] > 0.99, "32-bit max must map to ~1.0");
    assert!(audio.samples[1] < -0.99, "32-bit min must map to ~-1.0");
    assert_eq!(audio.samples[2], 0.0);
}

#[test]
fn decode_wav_rejects_unsupported_bit_depths() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("8bit.wav");
    let mut writer = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 8,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .expect("create wav");
    writer.write_sample(0i16).expect("write");
    writer.finalize().expect("finalize");

    let err = decode_wav(&path).expect_err("8-bit should be rejected");
    match err {
        CliError::Asr(AsrError::AudioIo { .. }) => {}
        other => panic!("expected AudioIo, got: {other:?}"),
    }
}
