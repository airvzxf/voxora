//! HTTP client used by every `voxora-hf` request.
//!
//! Holds a configured [`reqwest::Client`] plus the base URL of the HF
//! Hub being targeted (default `https://huggingface.co`). The client
//! is cheap to clone — `reqwest::Client` is internally `Arc`-shared —
//! so [`HfClient`] is itself `Clone` and lives happily behind an
//! `Arc<HuggingFaceSource>`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::error::HfError;

/// Per-process counter that guarantees unique tmp-file suffixes for
/// `HfClient::get_to_file` even when two concurrent calls sample
/// `SystemTime::now()` in the same nanosecond. Order is
/// `Relaxed` because we only need combinatorial uniqueness, not
/// happens-before with any other memory.
static DOWNLOAD_COUNTER: AtomicU64 = AtomicU64::new(0);

const DEFAULT_BASE_URL: &str = "https://huggingface.co";
const DEFAULT_USER_AGENT: &str = concat!(
    "voxora-hf/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/airvzxf/voxora)",
);

/// Closes [#113](https://github.com/airvzxf/voxora/issues/113):
/// retry budget for transient HTTP failures. 1 first try + 2 retries
/// = 3 attempts total.
const MAX_RETRY_ATTEMPTS: u32 = 3;
/// Backoff schedule (ms): start at 250 ms, double each attempt,
/// apply ±25 % jitter. Caps at 4 s.
const BASE_BACKOFF_MS: u64 = 250;
const MAX_BACKOFF_MS: u64 = 4_000;

/// Built HTTP client plus its endpoint configuration.
#[derive(Debug, Clone)]
pub(crate) struct HfClient {
    http: reqwest::Client,
    base_url: String,
    token: Option<String>,
}

impl HfClient {
    /// Return a fresh [`HfClientBuilder`].
    ///
    /// Reserved for tests and integration scripts — production code
    /// goes through [`crate::source::HuggingFaceSource::builder`].
    #[cfg(test)]
    pub(crate) fn builder() -> HfClientBuilder {
        HfClientBuilder::default()
    }

    /// `GET <base>/<path>` and return the typed response.
    pub(crate) async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, HfError> {
        let url = self.absolute(path);
        let resp = self.execute_with_retry(&url, self.http.get(&url)).await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            if status.as_u16() == 404 {
                return Err(HfError::HttpStatus {
                    url,
                    status: status.as_u16(),
                    body,
                });
            }
            return Err(HfError::HttpStatus {
                url,
                status: status.as_u16(),
                body,
            });
        }
        resp.json::<T>().await.map_err(|e| HfError::Transport {
            url,
            message: format!("failed to decode JSON: {e}"),
            source: Box::new(e),
        })
    }

    /// `GET <base>/<path>` and return the raw text body.
    pub(crate) async fn get_text(&self, path: &str) -> Result<String, HfError> {
        let url = self.absolute(path);
        let resp = self.execute_with_retry(&url, self.http.get(&url)).await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(HfError::HttpStatus {
                url,
                status: status.as_u16(),
                body,
            });
        }
        resp.text().await.map_err(|e| HfError::Transport {
            url,
            message: format!("failed to read body: {e}"),
            source: Box::new(e),
        })
    }

    /// `GET <base>/<path>` and stream the body into the supplied
    /// [`tokio::fs::File`].
    ///
    /// Uses [`reqwest::Response::bytes_stream`] to avoid loading large
    /// model weights into memory.
    pub(crate) async fn get_to_file(
        &self,
        path: &str,
        dest: &std::path::Path,
    ) -> Result<u64, HfError> {
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        let url = self.absolute(path);
        let resp = self.execute_with_retry(&url, self.http.get(&url)).await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(HfError::HttpStatus {
                url,
                status: status.as_u16(),
                body,
            });
        }

        // Write to a sibling temp file, atomically renamed by the
        // caller. The tmp path includes a per-call unique suffix so
        // two concurrent downloads of the same file do not trample
        // each other's partial bytes, even if both sample the
        // monotonic clock in the same nanosecond (the previous
        // implementation used only `SystemTime::now().as_nanos()`,
        // which is a real race under `try_join_all` of sharded model
        // downloads — see #103).
        //
        // Suffix shape: `<nanos_hex>-<counter>`. The nanos stay for
        // log correlation; uniqueness comes from the counter.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let counter = DOWNLOAD_COUNTER.fetch_add(1, Ordering::Relaxed);
        let unique = format!("{nanos:x}-{counter}");
        let tmp = dest.with_extension(format!(
            "{}.partial.{unique}",
            dest.extension().and_then(|e| e.to_str()).unwrap_or("bin")
        ));
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| HfError::Io {
                path: tmp.clone(),
                message: format!("create tmp: {e}"),
                source: e,
            })?;
        let mut stream = resp.bytes_stream();
        let mut written: u64 = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| HfError::Transport {
                url: url.clone(),
                message: format!("chunk read: {e}"),
                source: Box::new(e),
            })?;
            file.write_all(&chunk).await.map_err(|e| HfError::Io {
                path: tmp.clone(),
                message: format!("write: {e}"),
                source: e,
            })?;
            written += chunk.len() as u64;
        }
        file.flush().await.map_err(|e| HfError::Io {
            path: tmp.clone(),
            message: format!("flush: {e}"),
            source: e,
        })?;
        // fsync so the rename after this cannot expose half-written data.
        file.sync_all().await.map_err(|e| HfError::Io {
            path: tmp.clone(),
            message: format!("sync_all: {e}"),
            source: e,
        })?;
        drop(file);

        // If the destination already exists (a concurrent resolve
        // won the race), drop our tmp and accept the existing file.
        if dest.exists() {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Ok(written);
        }
        tokio::fs::rename(&tmp, dest)
            .await
            .map_err(|e| HfError::Io {
                path: dest.to_path_buf(),
                message: format!("rename tmp→dest: {e}"),
                source: e,
            })?;
        Ok(written)
    }

    /// Prepend the configured base URL to a path.
    fn absolute(&self, path: &str) -> String {
        if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        }
    }

    /// Apply auth header (if any) and dispatch the request.
    ///
    /// The `url` argument is for diagnostic purposes only — it is
    /// included in the [`HfError::Transport`] payload when the
    /// request fails before the response status is observable.
    /// The bearer token is set via the `Authorization` header
    /// (reqwest, not the URL), so the URL itself carries no
    /// credentials and is safe to surface in error messages.
    ///
    /// Note: this is the low-level transport call without retries;
    /// all in-flight callers go through [`Self::execute_with_retry`].
    /// Kept available so unit tests can assert transport-only
    /// behaviour without the retry wrapper interfering.
    #[allow(dead_code)]
    async fn execute(
        &self,
        url: &str,
        builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, HfError> {
        let builder = if let Some(token) = &self.token {
            builder.bearer_auth(token)
        } else {
            builder
        };
        builder.send().await.map_err(|e| HfError::Transport {
            url: url.to_string(),
            message: format!("request failed: {e}"),
            source: Box::new(e),
        })
    }

    /// Closes [#113](https://github.com/airvzxf/voxora/issues/113):
    /// wrap the request with a bounded retry policy.
    ///
    /// Retries on:
    /// - [`reqwest::Error`] with `is_timeout() / is_connect() /
    ///   is_request()` true (transient network blips).
    /// - HTTP `5xx` and `429` responses, respecting `Retry-After`
    ///   when present (capped at 30 s).
    ///
    /// Backoff: 250 ms × 2^(attempt-1), capped at 4 s, with ±25 %
    /// jitter. Budget: [`MAX_RETRY_ATTEMPTS`] (1 first try + 2 retries).
    ///
    /// Returns [`HfError::RetriesExhausted`] when the budget is
    /// exhausted so callers can distinguish a transient outage
    /// from a deterministic 4xx.
    async fn execute_with_retry(
        &self,
        url: &str,
        _builder: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, HfError> {
        let mut last_err: Option<String> = None;
        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            // Re-build the request each attempt; reqwest's
            // `RequestBuilder` is single-use, so the caller's
            // `_builder` is consumed on the first iteration
            // regardless. We use `self.http.get(url)` here because
            // we own the URL and the auth is applied per-attempt.
            let mut b = self.http.get(url);
            if let Some(token) = &self.token {
                b = b.bearer_auth(token);
            }
            match b.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_server_error() || status.as_u16() == 429 {
                        let retry_after = retry_after_secs(&resp);
                        let msg = format!("HTTP {status}");
                        last_err = Some(msg.clone());
                        if attempt < MAX_RETRY_ATTEMPTS {
                            backoff_sleep(attempt, retry_after).await;
                            continue;
                        }
                        return Err(HfError::RetriesExhausted {
                            url: url.to_string(),
                            attempts: attempt,
                            last_error: msg,
                        });
                    }
                    return Ok(resp);
                }
                Err(reqwest_err) => {
                    let transient = reqwest_err.is_timeout()
                        || reqwest_err.is_connect()
                        || reqwest_err.is_request();
                    let msg = format!("request failed: {reqwest_err}");
                    last_err = Some(msg.clone());
                    if !transient {
                        // Deterministic failure (TLS handshake,
                        // redirect loop, etc.) — surface immediately,
                        // do not retry.
                        return Err(HfError::Transport {
                            url: url.to_string(),
                            message: msg,
                            source: Box::new(reqwest_err),
                        });
                    }
                    if attempt >= MAX_RETRY_ATTEMPTS {
                        return Err(HfError::RetriesExhausted {
                            url: url.to_string(),
                            attempts: attempt,
                            last_error: msg,
                        });
                    }
                    backoff_sleep(attempt, None).await;
                }
            }
        }
        Err(HfError::RetriesExhausted {
            url: url.to_string(),
            attempts: MAX_RETRY_ATTEMPTS,
            last_error: last_err.unwrap_or_default(),
        })
    }
}

/// Closes [#113](https://github.com/airvzxf/voxora/issues/113):
/// parse the `Retry-After` header. Per RFC 9110 §10.2.3 the
/// header is either a delta-seconds integer (`Retry-After: 30`)
/// or an HTTP-date. We support the integer form (the one HF /
/// GitHub / Cloudflare actually emit) and return `None` for
/// everything else so the caller falls back to the exponential
/// path. Capped at 30 s — anything longer is treated as a hint to
/// fail-fast rather than sleep for minutes.
fn retry_after_secs(resp: &reqwest::Response) -> Option<u64> {
    let raw = resp
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?;
    let secs = raw.trim().parse::<u64>().ok()?;
    if secs > 0 && secs <= 30 {
        Some(secs)
    } else {
        None
    }
}

/// Closes [#113](https://github.com/airvzxf/voxora/issues/113):
/// exponential backoff with ±25 % jitter. If `retry_after_secs` is
/// provided (e.g. from a `Retry-After` header), uses the larger of
/// the two so the operator's hint always wins.
async fn backoff_sleep(attempt: u32, retry_after_secs: Option<u64>) {
    let exp = BASE_BACKOFF_MS.saturating_mul(1u64 << (attempt - 1).min(4));
    let capped = exp.min(MAX_BACKOFF_MS);
    // Cheap deterministic jitter keyed off the attempt counter and
    // the process monotonic clock; we don't need cryptographic
    // randomness for a backoff.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let jitter_seed = now ^ (attempt as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let jitter_delta = (jitter_seed % (2 * capped / 4).max(1)) as i64 - (capped / 4) as i64;
    let mut sleep_ms = (capped as i64 + jitter_delta).max(0) as u64;
    if let Some(hint) = retry_after_secs {
        let hint_ms = hint.saturating_mul(1000);
        sleep_ms = sleep_ms.max(hint_ms);
    }
    tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
}

/// Fluent builder for [`HfClient`].
#[derive(Debug, Clone)]
pub(crate) struct HfClientBuilder {
    base_url: String,
    token: Option<String>,
    timeout: Duration,
    user_agent: String,
}

impl Default for HfClientBuilder {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            token: None,
            timeout: Duration::from_secs(600),
            user_agent: DEFAULT_USER_AGENT.to_string(),
        }
    }
}

impl HfClientBuilder {
    /// Override the base URL (mostly useful for tests pointing at a
    /// local mock server).
    pub(crate) fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Override the bearer token.
    pub(crate) fn token(mut self, token: Option<String>) -> Self {
        self.token = token;
        self
    }

    /// Per-request timeout.
    pub(crate) fn timeout(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }

    /// Override the User-Agent header.
    pub(crate) fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    /// Build the client.
    pub(crate) fn build(self) -> Result<HfClient, HfError> {
        let http = reqwest::Client::builder()
            .timeout(self.timeout)
            .user_agent(self.user_agent)
            .build()
            .map_err(|e| HfError::Transport {
                url: String::new(),
                message: format!("client build: {e}"),
                source: Box::new(e),
            })?;
        Ok(HfClient {
            http,
            base_url: self.base_url.trim_end_matches('/').to_string(),
            token: self.token,
        })
    }
}
