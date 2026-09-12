//! [`MiniMaxConfig`] — the per-engine settings this crate needs.
//!
//! This is a **per-crate** config struct (mirrors the pattern from
//! `voxora-whisper` / `voxora-qwen3asr`). The cross-crate
//! env-cascade lives in [`voxora_config::VoxoraConfig`]; the CLI
//! resolves the token + endpoint there and passes the result down
//! to this struct via [`MiniMaxConfig::new`].

use secrecy::{ExposeSecret, SecretString};

/// Settings that build a [`MiniMaxEngine`](crate::MiniMaxEngine).
///
/// The `api_key` is wrapped in a [`SecretString`] so it does not
/// leak through the Debug / Display impls and is zero-on-drop.
#[derive(Clone)]
#[non_exhaustive]
pub struct MiniMaxConfig {
    /// Bearer token presented in the `Authorization` header. Wrapped
    /// in `SecretString` so it never appears in the `Debug` /
    /// `Display` impls / panic messages and is zero-on-drop. Use
    /// [`MiniMaxConfig::expose_api_key`] to read it back.
    pub(crate) api_key: SecretString,
    /// API endpoint (`https://api.minimax.io` by default).
    pub(crate) endpoint: String,
    /// Model id (`asr-1.0` is the only model MiniMax ships today).
    pub(crate) model: String,
    /// Request timeout in seconds. MiniMax audio uploads cap at
    /// 500 s / 50 MB; we default the timeout to 600 s so the first
    /// ~500 s request never times out on its own upload budget.
    pub(crate) timeout_secs: u64,
}

impl std::fmt::Debug for MiniMaxConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiniMaxConfig")
            .field("api_key", &"<redacted SecretString>")
            .field("endpoint", &self.endpoint)
            .field("model", &self.model)
            .field("timeout_secs", &self.timeout_secs)
            .finish_non_exhaustive()
    }
}

impl MiniMaxConfig {
    /// Build a [`MiniMaxConfig`] with the bare minimum: a bearer
    /// token. The endpoint and model default to MiniMax's public
    /// endpoint and the only model it ships (`asr-1.0`).
    ///
    /// # Errors
    ///
    /// Returns `MiniMaxConfigError::EmptyApiKey` if `api_key` is
    /// empty or whitespace-only.
    pub fn new<S: Into<String>>(api_key: S) -> Result<Self, MiniMaxConfigError> {
        let key = api_key.into();
        if key.trim().is_empty() {
            return Err(MiniMaxConfigError::EmptyApiKey);
        }
        Ok(Self {
            api_key: SecretString::new(key.into_boxed_str()),
            endpoint: crate::client::DEFAULT_ENDPOINT.to_string(),
            model: crate::client::DEFAULT_MODEL.to_string(),
            timeout_secs: crate::client::DEFAULT_TIMEOUT_SECS,
        })
    }

    /// Override the endpoint (defaults to
    /// `https://api.minimax.io`). Mostly useful for tests pointing
    /// at a mock server.
    #[must_use]
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    /// Override the model id (defaults to `asr-1.0`).
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Override the request timeout in seconds (defaults to 600).
    #[must_use]
    pub fn with_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Borrow the bearer token (leaks it through the closure).
    /// Use sparingly; this is the one place the secret escapes the
    /// `SecretString` wrapper and reaches the wire.
    pub fn expose_api_key(&self) -> &str {
        self.api_key.expose_secret()
    }

    /// Borrow the API endpoint.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Borrow the model id.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Borrow the request timeout in seconds.
    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }
}

/// Errors returned by [`MiniMaxConfig::new`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MiniMaxConfigError {
    /// The supplied API key was empty or whitespace-only.
    #[error("MINIMAX_API_KEY must be non-empty")]
    EmptyApiKey,
}

impl From<MiniMaxConfigError> for voxora_traits::AsrError {
    fn from(err: MiniMaxConfigError) -> Self {
        match err {
            MiniMaxConfigError::EmptyApiKey => {
                voxora_traits::AsrError::Config("MINIMAX_API_KEY must be non-empty".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_non_empty_token() {
        let cfg = MiniMaxConfig::new("sk-test").expect("non-empty token");
        assert_eq!(cfg.expose_api_key(), "sk-test");
        assert_eq!(cfg.endpoint(), crate::client::DEFAULT_ENDPOINT);
        assert_eq!(cfg.model(), crate::client::DEFAULT_MODEL);
        assert_eq!(cfg.timeout_secs(), crate::client::DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn new_rejects_empty_token() {
        let err = MiniMaxConfig::new("").expect_err("empty token rejected");
        assert!(matches!(err, MiniMaxConfigError::EmptyApiKey));
    }

    #[test]
    fn new_rejects_whitespace_token() {
        let err = MiniMaxConfig::new("   ").expect_err("whitespace token rejected");
        assert!(matches!(err, MiniMaxConfigError::EmptyApiKey));
    }

    #[test]
    fn debug_redacts_api_key() {
        let cfg = MiniMaxConfig::new("sk-supersecret").unwrap();
        let rendered = format!("{cfg:?}");
        assert!(
            !rendered.contains("sk-supersecret"),
            "Debug must redact the API key: {rendered}"
        );
        assert!(
            rendered.contains("redacted"),
            "Debug should mention redaction: {rendered}"
        );
    }

    #[test]
    fn builders_override_defaults() {
        let cfg = MiniMaxConfig::new("sk-test")
            .unwrap()
            .with_endpoint("http://localhost:9999")
            .with_model("asr-2.0")
            .with_timeout_secs(120);
        assert_eq!(cfg.endpoint(), "http://localhost:9999");
        assert_eq!(cfg.model(), "asr-2.0");
        assert_eq!(cfg.timeout_secs(), 120);
        assert_eq!(cfg.expose_api_key(), "sk-test");
    }

    #[test]
    fn config_is_clone() {
        let cfg = MiniMaxConfig::new("sk-test").unwrap();
        let cfg2 = cfg.clone();
        assert_eq!(cfg.expose_api_key(), cfg2.expose_api_key());
        assert_eq!(cfg.endpoint(), cfg2.endpoint());
    }
}
