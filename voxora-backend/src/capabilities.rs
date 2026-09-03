//! Snapshot of what backends the current binary supports.

use voxora_engine::BackendKind;

/// Compile-time + runtime snapshot of hardware backend capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Capabilities {
    /// Backends that were compiled in (selected via Cargo features).
    pub compiled: Vec<BackendKind>,
    /// Backends that compiled in AND are usable at runtime (e.g. a
    /// `cuda` feature compiled but no CUDA driver → not in this list).
    /// Same as `compiled` for now; populated by
    /// [`crate::device::detect`] in a later phase.
    pub available: Vec<BackendKind>,
}

impl Capabilities {
    /// Build a snapshot from the list of compiled-in backends.
    ///
    /// `available` is initialised as a clone of `compiled`; real
    /// runtime filtering lands once `voxora-backend` gains a
    /// `candle` feature in 0.2.x.
    pub fn new(compiled: Vec<BackendKind>) -> Self {
        // Until runtime detection exists, assume every compiled
        // backend is available. Real detection lands in 0.2.x.
        let available = compiled.clone();
        Self {
            compiled,
            available,
        }
    }

    /// True iff `kind` is compiled in.
    pub fn is_compiled(&self, kind: BackendKind) -> bool {
        self.compiled.contains(&kind)
    }

    /// True iff `kind` is compiled in AND usable at runtime.
    pub fn is_available(&self, kind: BackendKind) -> bool {
        self.available.contains(&kind)
    }

    /// Iterate over backends that were compiled in.
    pub fn iter_compiled(&self) -> std::slice::Iter<'_, BackendKind> {
        self.compiled.iter()
    }

    /// Iterate over backends that compiled in AND are usable at runtime.
    pub fn iter_available(&self) -> std::slice::Iter<'_, BackendKind> {
        self.available.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_capabilities_reports_nothing_compiled() {
        let caps = Capabilities::new(Vec::new());
        assert!(!caps.is_compiled(BackendKind::Cpu));
        assert!(!caps.is_available(BackendKind::Cpu));
    }

    #[test]
    fn compiled_and_available_match_in_phase_1() {
        let caps = Capabilities::new(vec![BackendKind::Cpu, BackendKind::Cuda]);
        assert!(caps.is_compiled(BackendKind::Cpu));
        assert!(caps.is_available(BackendKind::Cpu));
        assert!(caps.is_compiled(BackendKind::Cuda));
        assert!(caps.is_available(BackendKind::Cuda));
        assert!(!caps.is_compiled(BackendKind::Metal));
    }
}
