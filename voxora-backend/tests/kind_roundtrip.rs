//! Integration test: BackendKind round-trips and Capabilities
//! expose the expected surface.

use voxora_backend::{BackendKind, Capabilities, best_device, best_device_or_error, detect};

#[test]
fn best_device_is_cpu_in_phase_0() {
    assert_eq!(best_device(), BackendKind::Cpu);
}

#[test]
fn best_device_or_error_is_infallible_in_phase_0() {
    let k = best_device_or_error().expect("cpu is always available");
    assert_eq!(k, BackendKind::Cpu);
}

#[test]
fn detect_capabilities_contains_cpu() {
    let caps = detect();
    assert!(caps.is_compiled(BackendKind::Cpu));
}

#[test]
fn capabilities_new_is_empty_for_empty_input() {
    let caps = Capabilities::new(Vec::new());
    assert!(!caps.is_compiled(BackendKind::Cpu));
    assert!(caps.compiled.is_empty());
    assert!(caps.available.is_empty());
}
