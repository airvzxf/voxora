//! Compile-check that the `registry` feature re-exports the
//! expected symbols (closes #145).
//!
//! The body is intentionally a pair of trait-bound shims: the
//! assertion is "the symbols resolve at the bridge level", not
//! "the registry resolves a real model". The full HF resolve
//! path is exercised by `examples/registry_resolve_local.rs`,
//! which builds a `Registry` with the built-in descriptors and
//! prints the count (no model weights touched).

#![cfg(feature = "registry")]

use voxora_bridge::{Registry, RegistryHfExt, builtin_local_descriptor};

#[test]
fn registry_and_trait_resolve_at_bridge_level() {
    // Compile-only assertion: the symbols resolve and the trait
    // method is callable. The two helper functions are never
    // called — they exist purely to anchor the trait bounds so
    // any future drift that removes `RegistryHfExt` or renames
    // `builtin_local_descriptor` surfaces as a compile error
    // here, not as a silent breakage downstream.
    fn _accepts<R: RegistryHfExt>(_: &R) {}
    fn _is_a_registry(_: &Registry) {}
    let _ = builtin_local_descriptor;
}
