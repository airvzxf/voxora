//! Best-effort device selection.
//!
//! ASR-specific: returns one of CPU / CUDA / Metal / Vulkan depending
//! on what is compiled in. With the `candle` Cargo feature enabled,
//! uses `candle-core`'s runtime probe (CUDA → Metal → CPU). Without
//! it, returns Cpu as the safe fallback.

use voxora_engine::BackendKind;

use crate::capabilities::Capabilities;
use crate::error::BackendError;

/// Best-effort device selection.
///
/// Default behaviour (without the `candle` feature): returns Cpu.
/// With the `candle` feature: tries CUDA → Metal → CPU based on
/// runtime probe.
pub fn best_device() -> BackendKind {
    #[cfg(feature = "candle")]
    {
        best_device_candle().unwrap_or(BackendKind::Cpu)
    }
    #[cfg(not(feature = "candle"))]
    {
        BackendKind::Cpu
    }
}

#[cfg(feature = "candle")]
fn best_device_candle() -> Option<BackendKind> {
    use candle_core::Device;

    // `Device::cuda_if_available` / `metal_if_available` succeed with
    // `Device::Cpu` when the accelerator is not present, so they
    // cannot be used for detection on their own. Probe
    // `Device::new_cuda` / `Device::new_metal` instead, which error
    // when the backend is not usable, and only accept the result when
    // the returned variant is the matching accelerator.
    if let Ok(Device::Cuda(_)) = Device::new_cuda(0) {
        return Some(BackendKind::Cuda);
    }
    if let Ok(Device::Metal(_)) = Device::new_metal(0) {
        return Some(BackendKind::Metal);
    }
    Some(BackendKind::Cpu)
}

/// Detect the runtime capabilities of the current binary.
///
/// Without `candle` feature: only Cpu is reported as both compiled
/// and available. With `candle` feature: compiled list is widened to
/// the backends candle was built with, and `available` is reordered
/// so the best detected backend is first.
pub fn detect() -> Capabilities {
    #[cfg(feature = "candle")]
    {
        let compiled = candle_compiled_backends();
        let mut caps = Capabilities::new(compiled);
        caps.update_available_with(best_device_candle().unwrap_or(BackendKind::Cpu));
        caps
    }
    #[cfg(not(feature = "candle"))]
    {
        Capabilities::new(vec![BackendKind::Cpu])
    }
}

#[cfg(feature = "candle")]
fn candle_compiled_backends() -> Vec<BackendKind> {
    let mut compiled = vec![BackendKind::Cpu];
    // The user opts into CUDA reporting by setting the env var
    // (the candle-core `cuda` feature is not visible at this
    // layer). The runtime probe remains the source of truth for
    // availability.
    if std::env::var("VOXORA_ENABLE_CUDA").is_ok() {
        compiled.push(BackendKind::Cuda);
    }
    if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
        compiled.push(BackendKind::Metal);
    }
    compiled
}

#[cfg(feature = "candle")]
impl Capabilities {
    /// Move `best` to the front of the `available` list (deduping
    /// any earlier occurrence). `best_device`'s result is by
    /// construction always available.
    pub(crate) fn update_available_with(&mut self, best: BackendKind) {
        self.available.retain(|b| *b != best);
        self.available.insert(0, best);
    }
}

/// Like [`best_device`] but errors if no usable backend exists.
pub fn best_device_or_error() -> Result<BackendKind, BackendError> {
    let caps = detect();
    if caps.is_available(BackendKind::Cpu) {
        Ok(BackendKind::Cpu)
    } else {
        let compiled: Vec<String> = caps
            .iter_compiled()
            .map(|k| k.as_config().to_string())
            .collect();
        let available: Vec<String> = caps
            .iter_available()
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
    fn best_device_returns_cpu_when_candle_feature_off() {
        // Without the candle feature, `best_device` always returns
        // Cpu. With the candle feature on a normal Linux box it
        // still returns Cpu (no CUDA / Metal available), so this
        // assertion holds in both modes.
        assert_eq!(best_device(), BackendKind::Cpu);
    }

    #[test]
    fn detect_reports_cpu_as_compiled_and_available() {
        let caps = detect();
        assert!(caps.is_compiled(BackendKind::Cpu));
        assert!(caps.is_available(BackendKind::Cpu));
    }

    #[test]
    fn best_device_or_error_is_infallible_when_cpu_available() {
        let kind = best_device_or_error().expect("cpu is always available");
        assert_eq!(kind, BackendKind::Cpu);
    }
}
