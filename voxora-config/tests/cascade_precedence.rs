//! Integration test: explicit override beats file beats env beats defaults.

use std::env;
use std::io::Write;

use voxora_config::{CacheConfig, HfConfig, VoxoraConfig};

#[test]
fn explicit_beats_file_beats_default() {
    // Write a file with token "from-file".
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("voxora.toml");
    let mut f = std::fs::File::create(&path).expect("create");
    writeln!(f, "[hf]\ntoken = \"from-file\"").expect("write");

    // Set an env var with token "from-env".
    // SAFETY: this test binary holds a single test, so no other thread
    // reads the environment while it is mutated here.
    unsafe { env::set_var("VOXORA_HF_TOKEN", "from-env") };

    // Env beats the built-in default (anonymous).
    assert_eq!(
        VoxoraConfig::default().hf_token().as_deref(),
        Some("from-env")
    );

    // File wins over env.
    let from_file = VoxoraConfig::from_file(&path).expect("from_file");
    assert_eq!(from_file.hf_token().as_deref(), Some("from-file"));

    // Explicit override wins over file.
    let explicit = VoxoraConfig::new(
        CacheConfig::default(),
        HfConfig::new(Some("explicit".into()), None, None),
    );
    assert_eq!(explicit.hf_token().as_deref(), Some("explicit"));

    // SAFETY: see above.
    unsafe { env::remove_var("VOXORA_HF_TOKEN") };
}
