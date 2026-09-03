//! Integration test: env-var cascade overrides defaults.

use std::env;

use voxora_config::VoxoraConfig;

#[test]
fn voxora_hf_token_overrides_default() {
    // SAFETY: each test in this file touches a distinct variable, so no
    // other thread reads the one being set here.
    unsafe { env::set_var("VOXORA_HF_TOKEN", "from-env-token") };
    let resolved = VoxoraConfig::default().hf_token();
    unsafe { env::remove_var("VOXORA_HF_TOKEN") };
    assert_eq!(resolved.as_deref(), Some("from-env-token"));
}

#[test]
fn voxora_hf_base_url_overrides_default() {
    // SAFETY: see `voxora_hf_token_overrides_default`.
    unsafe { env::set_var("VOXORA_HF_BASE_URL", "http://localhost:9999") };
    let resolved = VoxoraConfig::default().hf_base_url();
    unsafe { env::remove_var("VOXORA_HF_BASE_URL") };
    assert_eq!(resolved, "http://localhost:9999");
}

#[test]
fn voxora_cache_dir_overrides_xdg() {
    // SAFETY: see `voxora_hf_token_overrides_default`.
    unsafe { env::set_var("VOXORA_CACHE_DIR", "/tmp/voxora-test-cache") };
    let resolved = VoxoraConfig::default().cache_root();
    unsafe { env::remove_var("VOXORA_CACHE_DIR") };
    assert_eq!(resolved, std::path::PathBuf::from("/tmp/voxora-test-cache"));
}

#[test]
fn voxora_hf_revision_overrides_default() {
    // SAFETY: see `voxora_hf_token_overrides_default`.
    unsafe { env::set_var("VOXORA_HF_REVISION", "from-env-revision") };
    let resolved = VoxoraConfig::default().hf_default_revision();
    unsafe { env::remove_var("VOXORA_HF_REVISION") };
    assert_eq!(resolved, "from-env-revision");
}
