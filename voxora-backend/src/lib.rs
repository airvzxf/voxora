//! Cross-cutting hardware backend selection for voxora ASR engines.
//!
//! Centralises `Cpu / Cuda / Metal / Vulkan` selection so engines no
//! longer each declare their own `cpu` / `cuda` / `metal` / `vulkan`
//! feature wiring.
//!
//! ASR-specific: this crate only models hardware backends that are
//! relevant to the voxora speech-recognition stack. It does not
//! abstract LLama.cpp or any other non-ASR backend.
//!
//! ## Phase 0 surface
//!
//! Today the crate only exposes:
//!
//! - [`BackendKind`] (re-exported from `voxora-engine`).
//! - [`Capabilities`] — compile-time + runtime snapshot.
//! - [`best_device`] / [`best_device_or_error`] — current "always
//!   returns CPU" implementation.
//!
//! The real candle-backed detection lands in 0.2.x once voxora-backend
//! gains a `candle` feature flag.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod capabilities;
pub mod device;
pub mod error;
pub mod kind;

pub use capabilities::Capabilities;
pub use device::{best_device, best_device_or_error, detect};
pub use error::BackendError;
pub use kind::BackendKind;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_surface_round_trips() {
        let kind = BackendKind::Cpu;
        assert_eq!(kind.as_config(), "cpu");
    }

    #[test]
    fn detect_compiles_in_cpu_always() {
        let caps = detect();
        assert!(caps.is_compiled(BackendKind::Cpu));
    }
}
