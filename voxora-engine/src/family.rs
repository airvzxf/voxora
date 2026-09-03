//! Engine family — the canonical spelling used in config files and
//! CLI flags.
//!
//! Coexists with `voxora-bridge::ModelKind` and
//! `voxora-cli::BackendKind` during the 0.1.x → 0.2.0 migration;
//! both duplicates are slated for removal in 0.2.0 once the engine
//! crates adopt the new [`crate::adapter::EngineAdapter`] trait.
//! Adding a new variant here is a SemVer-minor bump because the enum
//! is `#[non_exhaustive]` for downstream pattern matches, but the
//! `from_config` parser MUST be extended in lockstep.

use std::fmt;
use std::str::FromStr;

/// Which engine family an [`crate::adapter::EngineAdapter`] represents.
///
/// Variants are matched by [`EngineFamily::from_config`] for the
/// canonical config spelling. The enum is `#[non_exhaustive]` so
/// adding a new family (parakeet, voxtral, …) does not break
/// downstream pattern matches — but consumers that exhaustively
/// match today must add a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EngineFamily {
    /// whisper.cpp family (`voxora-whisper`).
    Whisper,
    /// Qwen3-ASR family (`voxora-qwen3asr`).
    Qwen3Asr,
}

impl EngineFamily {
    /// Parse the canonical config spelling. Accepts both
    /// `"qwen3-asr"` and the legacy `"qwen3asr"` /
    /// `"qwen3_asr"` aliases for ergonomics.
    pub fn from_config(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "whisper" => Some(Self::Whisper),
            "qwen3-asr" | "qwen3asr" | "qwen3_asr" => Some(Self::Qwen3Asr),
            _ => None,
        }
    }

    /// Canonical config spelling (matches `voxora-cli --engine`).
    pub fn as_config(self) -> &'static str {
        match self {
            Self::Whisper => "whisper",
            Self::Qwen3Asr => "qwen3-asr",
        }
    }

    /// Short crate-label that identifies the voxora-* crate.
    pub fn crate_label(self) -> &'static str {
        match self {
            Self::Whisper => "voxora-whisper",
            Self::Qwen3Asr => "voxora-qwen3asr",
        }
    }
}

impl fmt::Display for EngineFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_config())
    }
}

impl FromStr for EngineFamily {
    type Err = InvalidEngineFamily;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_config(s).ok_or_else(|| InvalidEngineFamily(s.to_string()))
    }
}

/// Error returned by [`EngineFamily`]'s [`FromStr`] impl when the
/// input does not match a known engine family. The original input is
/// preserved so callers can render it in their own error messages.
#[derive(Debug, thiserror::Error)]
#[error("unknown engine_family {0:?}; expected one of `whisper` or `qwen3-asr`")]
pub struct InvalidEngineFamily(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_config_accepts_canonical_and_aliases() {
        assert_eq!(
            EngineFamily::from_config("whisper"),
            Some(EngineFamily::Whisper)
        );
        assert_eq!(
            EngineFamily::from_config("qwen3-asr"),
            Some(EngineFamily::Qwen3Asr)
        );
        assert_eq!(
            EngineFamily::from_config("qwen3asr"),
            Some(EngineFamily::Qwen3Asr)
        );
        assert_eq!(
            EngineFamily::from_config("qwen3_asr"),
            Some(EngineFamily::Qwen3Asr)
        );
    }

    #[test]
    fn from_config_is_case_insensitive() {
        assert_eq!(
            EngineFamily::from_config("WHISPER"),
            Some(EngineFamily::Whisper)
        );
        assert_eq!(
            EngineFamily::from_config("Qwen3-ASR"),
            Some(EngineFamily::Qwen3Asr)
        );
    }

    #[test]
    fn from_config_rejects_unknown() {
        assert_eq!(EngineFamily::from_config("parakeet"), None);
        assert_eq!(EngineFamily::from_config(""), None);
    }

    #[test]
    fn as_config_round_trips() {
        for f in [EngineFamily::Whisper, EngineFamily::Qwen3Asr] {
            assert_eq!(EngineFamily::from_config(f.as_config()), Some(f));
        }
    }

    #[test]
    fn from_str_error_carries_input() {
        let err: InvalidEngineFamily = "bogus".parse::<EngineFamily>().unwrap_err();
        assert_eq!(err.0, "bogus");
    }

    #[test]
    fn crate_labels_match_workspace_member_names() {
        assert_eq!(EngineFamily::Whisper.crate_label(), "voxora-whisper");
        assert_eq!(EngineFamily::Qwen3Asr.crate_label(), "voxora-qwen3asr");
    }
}
