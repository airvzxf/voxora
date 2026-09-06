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
//! If the fixture is not yet cached, EPIC #133 (PR #59) makes the
//! underlying `ureq`-based download run synchronously: the first
//! run pulls the file into the cache and prints the path; later
//! runs hit the cache and exit immediately.

use std::process::ExitCode;

use voxora_testkit::{FixtureError, KNOWN_FIXTURES, resolve_real_fixture};

fn main() -> ExitCode {
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

    match resolve_real_fixture(&name) {
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
