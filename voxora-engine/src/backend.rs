//! Hardware backend descriptor shared across engine adapters.

use std::fmt;

/// Compile-time hardware backend the engine was built against.
///
/// Engines can ship multiple backends as separate Cargo features
/// (`cpu` / `cuda` / `metal` / `vulkan`); this enum captures which
/// one was selected at runtime via `BackendDescriptor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BackendKind {
    /// Pure CPU execution.
    Cpu,
    /// NVIDIA CUDA (requires `cuda` feature on the engine).
    Cuda,
    /// Apple Metal (requires `metal` feature on the engine).
    Metal,
    /// Vulkan compute (requires `vulkan` feature on the engine).
    Vulkan,
}

impl BackendKind {
    /// Canonical config spelling.
    pub fn as_config(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
            Self::Metal => "metal",
            Self::Vulkan => "vulkan",
        }
    }

    /// Parse the canonical config spelling. Case-insensitive;
    /// returns `None` for unknown values so callers can render their
    /// own diagnostic message.
    pub fn from_config(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "cpu" => Some(Self::Cpu),
            "cuda" => Some(Self::Cuda),
            "metal" => Some(Self::Metal),
            "vulkan" => Some(Self::Vulkan),
            _ => None,
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_config())
    }
}

/// Concrete backend a particular engine instance was loaded with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BackendDescriptor {
    /// Backend kind the engine was loaded against.
    pub kind: BackendKind,
}

impl BackendDescriptor {
    /// The CPU descriptor — most common, useful as a default.
    pub const CPU: Self = Self {
        kind: BackendKind::Cpu,
    };

    /// Build a descriptor for an arbitrary backend kind.
    pub fn new(kind: BackendKind) -> Self {
        Self { kind }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for k in [
            BackendKind::Cpu,
            BackendKind::Cuda,
            BackendKind::Metal,
            BackendKind::Vulkan,
        ] {
            assert_eq!(BackendKind::from_config(k.as_config()), Some(k));
        }
    }

    #[test]
    fn rejects_unknown() {
        assert_eq!(BackendKind::from_config("webgpu"), None);
        assert_eq!(BackendKind::from_config(""), None);
    }

    #[test]
    fn is_case_insensitive() {
        assert_eq!(BackendKind::from_config("CUDA"), Some(BackendKind::Cuda));
    }

    #[test]
    fn cpu_descriptor_is_const() {
        let d = BackendDescriptor::CPU;
        assert_eq!(d.kind, BackendKind::Cpu);
    }
}
