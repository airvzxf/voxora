//! Heuristic mapping from `config.json` to [`ModelCapabilities`].
//!
//! We deliberately do **not** ship a 500-line table of every HF
//! architecture; instead we pattern-match on `architectures[0]` and
//! `model_type`, which are the only fields that are reasonably
//! consistent across repos. Anything we don't recognise returns
//! [`ModelCapabilities::UNKNOWN`] and a future phase can extend the
//! table without breaking the public API.

use std::collections::HashMap;

use serde::Deserialize;
use voxora_core::ModelCapabilities;

/// Minimal subset of `config.json` we care about. Anything else is
/// ignored.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct RawConfig {
    #[serde(default)]
    pub architectures: Vec<String>,
    #[serde(default)]
    pub model_type: Option<String>,
    /// Whisper-specific: number of languages the model was trained on.
    #[serde(default)]
    pub num_languages: Option<u32>,
    /// Whisper-specific: legacy field.
    #[serde(default)]
    pub n_languages: Option<u32>,
    /// Qwen3-ASR and similar: human-readable language names.
    #[serde(default)]
    pub support_languages: Vec<String>,
    /// Qwen3-ASR and similar: nested thinker/audio config.
    #[serde(default)]
    pub thinker_config: Option<Box<RawConfig>>,
}

impl RawConfig {
    /// Try to extract the most informative `architectures[0]` value.
    pub(crate) fn primary_arch(&self) -> Option<String> {
        self.architectures
            .first()
            .cloned()
            .or_else(|| self.thinker_config.as_ref().and_then(|c| c.primary_arch()))
    }

    /// Walk to the deepest `model_type` we can find.
    pub(crate) fn primary_model_type(&self) -> Option<String> {
        self.model_type.clone().or_else(|| {
            self.thinker_config
                .as_ref()
                .and_then(|c| c.primary_model_type())
        })
    }

    /// First non-empty `support_languages` list (might live in a nested
    /// config).
    pub(crate) fn primary_languages(&self) -> Vec<String> {
        if !self.support_languages.is_empty() {
            return self.support_languages.clone();
        }
        self.thinker_config
            .as_ref()
            .map(|c| c.primary_languages())
            .unwrap_or_default()
    }
}

/// Build a [`ModelCapabilities`] from a `config.json` payload.
pub(crate) fn from_config(raw: &RawConfig) -> ModelCapabilities {
    let arch = raw.primary_arch().unwrap_or_default();
    let model_type = raw.primary_model_type().unwrap_or_default();
    let mut caps = ModelCapabilities::UNKNOWN;

    let key = arch_key(&arch, &model_type);

    match key {
        ArchKey::Whisper => {
            caps.multilingual = true;
            caps.word_timestamps = true;
            // Whisper does not have a first-class streaming mode in
            // whisper.cpp; leave false.
            caps.languages = whisper_languages(raw);
        }
        ArchKey::Qwen3Asr => {
            caps.multilingual = true;
            caps.word_timestamps = false;
            caps.streaming = false;
            caps.languages = normalise_languages(&raw.primary_languages());
        }
        ArchKey::Unknown => {
            // Conservative: every flag off, no language list.
        }
    }

    caps
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchKey {
    Whisper,
    Qwen3Asr,
    Unknown,
}

fn arch_key(arch: &str, model_type: &str) -> ArchKey {
    let lower_arch = arch.to_ascii_lowercase();
    let lower_type = model_type.to_ascii_lowercase();
    if lower_arch.contains("whisper") || lower_type == "whisper" {
        ArchKey::Whisper
    } else if lower_arch.contains("qwen3asr") || lower_type == "qwen3_asr" {
        ArchKey::Qwen3Asr
    } else {
        ArchKey::Unknown
    }
}

/// Whisper has historically exposed its language count in one of two
/// fields; newer configs use `num_languages`. Fall back to 99 if
/// neither is set (Whisper's published training set).
fn whisper_languages(raw: &RawConfig) -> Vec<String> {
    // Whisper's language list is the standard ISO 639-1 set used by
    // every HF Whisper config. We only return the count, not the
    // names, because the canonical mapping lives in the engine
    // adapter. We use English 2-letter codes as a placeholder so the
    // vector is non-empty when multilingual is true.
    let _n = raw.num_languages.or(raw.n_languages).unwrap_or(99);
    // Common ISO 639-1 codes that Whisper was trained on.
    let common: &[&str] = &[
        "en", "zh", "de", "es", "ru", "ko", "fr", "ja", "pt", "tr", "pl", "ca", "nl", "ar", "sv",
        "it", "id", "hi", "fi", "vi", "he", "uk", "el", "ms", "cs", "ro", "da", "hu", "ta", "no",
        "th", "ur", "hr", "bg", "lt", "la", "mi", "ml", "cy", "sk", "te", "fa", "lv", "bn", "sr",
        "az", "sl", "kn", "et", "mk", "br", "eu", "is", "hy", "ne", "mn", "bs", "kk", "sq", "sw",
        "gl", "mr", "pa", "si", "km", "sn", "yo", "so", "af", "oc", "ka", "be", "tg", "sd", "gu",
        "am", "yi", "lo", "uz", "fo", "ht", "ps", "tk", "nn", "mt", "sa", "lb", "my", "bo", "tl",
        "mg", "as", "tt", "haw", "ln", "ha", "ba", "jw", "su",
    ];
    common.iter().map(|s| s.to_string()).collect()
}

/// Map a list of human-readable language names (as published in
/// `support_languages`) to the best two-letter codes we can. For now
/// we just lowercase and return them — engines are responsible for
/// any further normalisation. This keeps the test simple and
/// predictable.
fn normalise_languages(names: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(names.len());
    for n in names {
        let lc = n.to_ascii_lowercase();
        if !lc.is_empty() && !out.contains(&lc) {
            out.push(lc);
        }
    }
    out
}

/// Read a `RawConfig` from a JSON byte slice. Convenience wrapper that
/// keeps the `serde_json::Error` typed at the call site.
pub(crate) fn parse_config(bytes: &[u8]) -> Result<RawConfig, serde_json::Error> {
    serde_json::from_slice::<RawConfig>(bytes)
}

/// Read a cached `ModelCapabilities` from disk (best-effort). Returns
/// `None` if the file is missing or malformed. Wired into the
/// `capabilities_for` flow when the caller already has the cache
/// populated.
#[allow(dead_code)]
pub(crate) fn read_cached(dir: &std::path::Path) -> Option<ModelCapabilities> {
    let path = crate::cache::capabilities_cache_path(dir);
    let bytes = std::fs::read(&path).ok()?;
    let raw: HashMap<String, serde_json::Value> = serde_json::from_slice(&bytes).ok()?;
    let languages = raw
        .get("languages")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Some(ModelCapabilities::new(
        raw.get("multilingual")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        raw.get("word_timestamps")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        raw.get("streaming")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        languages,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qwen_config() -> RawConfig {
        serde_json::from_value(serde_json::json!({
            "architectures": ["Qwen3ASRForConditionalGeneration"],
            "model_type": "qwen3_asr",
            "support_languages": ["English", "Chinese", "Spanish"],
        }))
        .unwrap()
    }

    fn whisper_config() -> RawConfig {
        serde_json::from_value(serde_json::json!({
            "architectures": ["WhisperForConditionalGeneration"],
            "model_type": "whisper",
            "num_languages": 99,
        }))
        .unwrap()
    }

    #[test]
    fn qwen3_asr_is_multilingual_with_languages() {
        let caps = from_config(&qwen_config());
        assert!(caps.multilingual);
        assert_eq!(
            caps.languages,
            vec![
                "english".to_string(),
                "chinese".to_string(),
                "spanish".to_string()
            ]
        );
        assert!(!caps.word_timestamps);
    }

    #[test]
    fn whisper_supports_word_timestamps() {
        let caps = from_config(&whisper_config());
        assert!(caps.multilingual);
        assert!(caps.word_timestamps);
        assert!(!caps.languages.is_empty());
    }

    #[test]
    fn unknown_arch_returns_unknown() {
        let raw: RawConfig = serde_json::from_value(serde_json::json!({
            "architectures": ["GladosForConditionalGeneration"],
            "model_type": "audio",
        }))
        .unwrap();
        let caps = from_config(&raw);
        assert_eq!(caps, ModelCapabilities::UNKNOWN);
    }

    #[test]
    fn nested_thinker_config_is_used() {
        let raw: RawConfig = serde_json::from_value(serde_json::json!({
            "thinker_config": {
                "architectures": ["Qwen3ASRForConditionalGeneration"],
                "model_type": "qwen3_asr",
                "support_languages": ["English", "Japanese"]
            }
        }))
        .unwrap();
        let caps = from_config(&raw);
        assert!(caps.multilingual);
        assert_eq!(
            caps.languages,
            vec!["english".to_string(), "japanese".to_string()]
        );
    }

    #[test]
    fn parse_config_handles_minimal_payload() {
        let bytes = br#"{"architectures":["X"]}"#;
        let raw = parse_config(bytes).unwrap();
        assert_eq!(raw.architectures, vec!["X"]);
    }

    #[test]
    fn parse_config_rejects_non_object() {
        let bytes = br#"[1,2,3]"#;
        assert!(parse_config(bytes).is_err());
    }

    #[test]
    fn empty_config_yields_unknown() {
        let raw = RawConfig::default();
        let caps = from_config(&raw);
        assert_eq!(caps, ModelCapabilities::UNKNOWN);
    }
}
