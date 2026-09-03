//! Best-effort device selection.
//!
//! ASR-specific: returns one of CPU / CUDA / Metal / Vulkan depending
//! on what is compiled in. This is intentionally a thin re-export of
//! candle's `best_device` once a downstream engine adds candle as a
//! dependency; for now we expose the compile-time defaults so callers
//! can write `best_device()` against a stable contract.

use voxora_engine::BackendKind;

use crate::capabilities::Capabilities;
use crate::error::BackendError;

/// Returned by [`best_device`] when no compiled backend can be used.
///
/// In Phase 0 we hard-code `Cpu` as the always-available fallback so
/// the function never errors on a normal Linux build.
pub fn best_device() -> BackendKind {
    // TODO(0.2.x): replace with candle-core's best_device once
    // voxora-backend gains a `candle` feature. For now we pick
    // CPU because it is the only backend that is always compiled in.
    BackendKind::Cpu
}

/// Detect the runtime capabilities of the current binary. Always
/// returns `vec![Cpu]` in Phase 0; will grow to query CUDA / Metal /
/// Vulkan at runtime once the `candle` feature lands.
pub fn detect() -> Capabilities {
    Capabilities::new(vec![BackendKind::Cpu])
}

/// Like [`best_device`] but errors if no usable backend exists.
///
/// In Phase 0 this is infallible because Cpu is always compiled in;
/// the `Result` wrapper is kept so callers can future-proof their
/// error handling.
pub fn best_device_or_error() -> Result<BackendKind, BackendError> {
    let caps = detect();
    if caps.is_available(BackendKind::Cpu) {
        Ok(BackendKind::Cpu)
    } else {
        let compiled: Vec<String> = caps
            .compiled
            .iter()
            .map(|k| k.as_config().to_string())
            .collect();
        let available: Vec<String> = caps
            .available
            .iter()
            .map(|k| k.as_config().to_string())
            .collect();
        Err(BackendError::NoUsableBackend {
            compiled,
            available,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_device_returns_cpu_in_phase_0() {
        assert_eq!(best_device(), BackendKind::Cpu);
    }

    #[test]
    fn detect_reports_cpu_as_compiled_and_available() {
        let caps = detect();
        assert!(caps.is_compiled(BackendKind::Cpu));
        assert!(caps.is_available(BackendKind::Cpu));
    }

    #[test]
    fn best_device_or_error_succeeds_in_phase_0() {
        let kind = best_device_or_error().expect("cpu is always available");
        assert_eq!(kind, BackendKind::Cpu);
    }
}
