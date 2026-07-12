//! Small helpers for building [`voxora_core::ResolveOptions`] from
//! CLI flags.
//!
//! `ResolveOptions` is `#[non_exhaustive]`, so downstream crates
//! cannot use struct expressions. We carry the construction logic
//! here in one place and expose a single
//! [`build_resolve_opts`] helper that downstream subcommand modules
//! call.

use voxora_core::ResolveOptions;

use crate::error::CliError;

/// Build a [`ResolveOptions`] from CLI inputs. `populate` is called
/// with a `&mut ResolveOptions` so callers can set fields (`quantization`
/// in particular) that need custom validation; the resolver
/// construction (token, revision) is set unconditionally here, the
/// caller gets to fill in the rest.
///
/// Return an error from `populate` to reject bad user input (e.g. an
/// unknown `--quantization` value).
pub fn build_resolve_opts(
    token: Option<&str>,
    revision: Option<&str>,
    populate: impl FnOnce(&mut ResolveOptions) -> Result<(), CliError>,
) -> Result<ResolveOptions, CliError> {
    let mut opts = ResolveOptions::default();
    opts.token = token.map(str::to_string);
    opts.revision = revision.map(str::to_string);
    populate(&mut opts)?;
    Ok(opts)
}

#[cfg(test)]
mod tests;
