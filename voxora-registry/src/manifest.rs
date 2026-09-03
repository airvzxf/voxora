//! `.voxora-manifest.json` — on-disk metadata for a cached model.
//!
//! Written next to the cached weights so a registry can answer
//! "which engine does this directory belong to?" without re-parsing
//! every file in the directory.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use voxora_engine::EngineFamily;

use crate::error::RegistryError;

/// Filename of the on-disk manifest written next to cached weights.
pub const MANIFEST_FILENAME: &str = ".voxora-manifest.json";
/// Bump this whenever the manifest schema changes in a
/// non-backwards-compatible way.
pub const MANIFEST_VERSION: u32 = 1;

/// On-disk sidecar that records which engine a cached model directory
/// belongs to. Written by [`CacheManifest::write`], read by
/// [`CacheManifest::read`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct CacheManifest {
    /// Schema version this manifest was written with.
    pub manifest_version: u32,
    /// Engine the cached weights belong to. Serialised via
    /// `family_serde` so we don't depend on `voxora-engine` having a
    /// `serde` feature.
    #[serde(with = "family_serde")]
    pub family: EngineFamily,
    /// Stable identifier (HF `org/repo[/file]` or local absolute path).
    pub model_id: String,
    /// When the manifest was last written (seconds since UNIX epoch).
    pub written_at_unix: u64,
}

impl CacheManifest {
    /// Build a [`CacheManifest`] with the current schema version.
    pub fn new(family: EngineFamily, model_id: impl Into<String>, written_at_unix: u64) -> Self {
        Self {
            manifest_version: MANIFEST_VERSION,
            family,
            model_id: model_id.into(),
            written_at_unix,
        }
    }

    /// Read a manifest at `dir/.voxora-manifest.json`.
    pub fn read(dir: &Path) -> Result<Self, RegistryError> {
        let p = dir.join(MANIFEST_FILENAME);
        let text = std::fs::read_to_string(&p)
            .map_err(|e| RegistryError::Parse(format!("manifest read {p:?}: {e}")))?;
        serde_json::from_str(&text)
            .map_err(|e| RegistryError::Parse(format!("manifest parse {p:?}: {e}")))
    }

    /// Write this manifest to `dir/.voxora-manifest.json`.
    pub fn write(&self, dir: &Path) -> Result<PathBuf, RegistryError> {
        let p = dir.join(MANIFEST_FILENAME);
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| RegistryError::Parse(format!("manifest serialize: {e}")))?;
        std::fs::write(&p, text)
            .map_err(|e| RegistryError::Parse(format!("manifest write {p:?}: {e}")))?;
        Ok(p)
    }
}

mod family_serde {
    //! `EngineFamily` doesn't derive `Serialize`/`Deserialize`, so we
    //! round-trip it through its canonical config spelling.
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use voxora_engine::EngineFamily;

    pub fn serialize<S: Serializer>(family: &EngineFamily, s: S) -> Result<S::Ok, S::Error> {
        family.as_config().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<EngineFamily, D::Error> {
        let raw = String::deserialize(d)?;
        EngineFamily::from_config(&raw)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown engine family {raw:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_via_disk() {
        let dir = tempfile::tempdir().unwrap();
        let m = CacheManifest::new(
            voxora_engine::EngineFamily::Whisper,
            "ggerganov/whisper.cpp",
            1_700_000_000,
        );
        m.write(dir.path()).expect("write");
        let back = CacheManifest::read(dir.path()).expect("read");
        assert_eq!(m, back);
    }

    #[test]
    fn read_missing_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = CacheManifest::read(dir.path()).expect_err("missing manifest errors");
        match err {
            RegistryError::Parse(msg) => assert!(msg.contains("manifest read")),
            _ => panic!("expected Parse, got {err:?}"),
        }
    }
}
