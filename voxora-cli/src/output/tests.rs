//! Unit tests for the output formatting helpers.

use super::*;

#[test]
fn format_bytes_handles_byte_boundaries() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1024), "1.00 KiB");
    assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GiB");
}

#[test]
fn format_bytes_handles_fractional_units() {
    assert_eq!(format_bytes(1536), "1.50 KiB");
    assert_eq!(format_bytes(1024 * 1024 * 3 + 1024 * 512), "3.50 MiB");
}

#[test]
fn render_cached_table_emits_header_and_one_row_per_model() {
    let m = CachedModel::new(
        std::path::PathBuf::from("/cache/Qwen/Qwen3-ASR-0.6B/main"),
        1024,
        3,
        true,
    );
    let s = render_cached_table(&[m]);
    assert!(s.contains("PATH"), "{s}");
    assert!(s.contains("BYTES"), "{s}");
    assert!(s.contains("FILES"), "{s}");
    assert!(s.contains("DONE"), "{s}");
    assert!(s.contains("/cache/Qwen/Qwen3-ASR-0.6B/main"), "{s}");
    assert!(s.contains("1.00 KiB"), "{s}");
    assert!(s.contains("true"), "{s}");
}

#[test]
fn render_info_includes_every_capability_field() {
    let caps = ModelCapabilities::new(true, false, false, vec!["english".into()]);
    let s = render_info("Qwen/Qwen3-ASR-0.6B", "huggingface", &caps);
    assert!(s.contains("model_id      : Qwen/Qwen3-ASR-0.6B"), "{s}");
    assert!(s.contains("source        : huggingface"), "{s}");
    assert!(s.contains("multilingual  : true"), "{s}");
    assert!(s.contains("word_timestamps: false"), "{s}");
    assert!(s.contains("streaming     : false"), "{s}");
    assert!(s.contains("- english"), "{s}");
}

#[test]
fn dir_size_bytes_sums_top_level_files_only() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::write(tmp.path().join("a"), vec![0u8; 10]).unwrap();
    std::fs::write(tmp.path().join("b"), vec![0u8; 20]).unwrap();
    let nested = tmp.path().join("sub");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("deep"), vec![0u8; 9999]).unwrap();
    let total = dir_size_bytes(tmp.path());
    assert_eq!(total, 30, "sub-directory bytes must not count");
}

#[test]
fn dir_size_bytes_returns_zero_for_missing_path() {
    let total = dir_size_bytes(Path::new("/nonexistent/voxis/dir"));
    assert_eq!(total, 0);
}
