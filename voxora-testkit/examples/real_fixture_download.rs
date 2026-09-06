//! Library-API smoke for `voxora-testkit`: invoke
//! [`voxora_testkit::resolve_real_fixture`] for a named fixture and
//! print the resulting path.
//!
//! This is the canonical entry point every engine parity test in
//! the workspace should reach for when it needs a real audio or
//! model file. Pass a fixture name from [`KNOWN_FIXTURES`]:
//!
//! ```text
//! cargo run --example real_fixture_download -p voxora-testkit -- jfk.wav
//! cargo run --example real_fixture_download -p voxora-testkit -- ggml-tiny.bin
//! ```
//!
//! If the fixture is already on disk in the cache
//! (`$XDG_CACHE_HOME/voxora/fixtures/` or
//! `$VOXORA_FIXTURE_CACHE_DIR`) the path is printed and the program
//! exits 0.
//!
//! If the fixture is not yet cached, the example prints the
//! [`FixtureError::Network`] message that the canonical download
//! surface returns today and exits non-zero. Wiring up the actual
//! network fetch is tracked separately — this example is the
//! publicly-visible API surface parity tests should target once it
//! lands.

use std::process::ExitCode;

use voxora_testkit::{FixtureError, KNOWN_FIXTURES, resolve_real_fixture};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let name = match std::env::args().nth(1) {
        Some(n) => n,
        None => {
            eprintln!("usage: real_fixture_download <fixture-name>");
            eprintln!();
            eprintln!("known fixtures:");
            for f in KNOWN_FIXTURES {
                eprintln!("  - {f}");
            }
            return ExitCode::from(2);
        }
    };

    match resolve_real_fixture(&name).await {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(FixtureError::UnknownFixture { name, known }) => {
            eprintln!("unknown fixture {name:?}; known: {known}");
            ExitCode::from(2)
        }
        Err(FixtureError::CacheNotConfigured) => {
            eprintln!("fixture cache directory not configured");
            ExitCode::from(1)
        }
        Err(FixtureError::Network {
            fixture,
            message,
            source,
        }) => {
            eprintln!("download for {fixture:?} failed: {message}");
            if let Some(src) = source {
                eprintln!("caused by: {src}");
            }
            ExitCode::from(1)
        }
        Err(other) => {
            eprintln!("unexpected fixture error: {other}");
            ExitCode::from(1)
        }
    }
}
