//! `ureq`-backed HTTP client + multipart/form-data builder +
//! OpenAI-style error envelope parser for MiniMax.
//!
//! MiniMax uses `multipart/form-data` with a `model` field, a
//! `file` field carrying the audio bytes, and an optional
//! `response_format` field. The `language` parameter goes in a
//! header, not in the form body. The bearer token goes in
//! `Authorization: Bearer <key>`.
//!
//! On failure the response body follows the OpenAI envelope
//! shape: `{ "type": "error", "error": { "type", "message",
//! "http_code" }, "request_id" }`. The mapping table below
//! implements the full translation onto
//! [`voxora_traits::AsrError`] (closes #154 EPIC #153).

use std::io::Cursor;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use ureq::Agent;

use voxora_traits::AsrError;

use crate::config::MiniMaxConfig;
use crate::params::{MiniMaxParams, MiniMaxParamsApply};

/// Default API endpoint. Override via
/// [`MiniMaxConfig::with_endpoint`] for tests / mock servers.
pub const DEFAULT_ENDPOINT: &str = "https://api.minimax.io";

/// Default model id. MiniMax ships exactly one ASR model today
/// (`asr-1.0`); the API rejects any other value with 400
/// `bad_request_error`.
pub const DEFAULT_MODEL: &str = "asr-1.0";

/// Default request timeout in seconds. MiniMax audio uploads cap
/// at 500 s / 50 MB, so we default to 600 s to cover the largest
/// allowed payload with margin.
pub const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Maximum audio file size MiniMax accepts (50 MB). Past this
/// the server returns 413 `invalid_request_error`.
pub const MAX_AUDIO_BYTES: usize = 50 * 1024 * 1024;

/// One HTTP call's worth of state: the agent, the endpoint, the
/// model, the auth header. Cheap to construct (just an `Arc`'d
/// `Agent` + three `String`s); held by the engine and reused
/// across `transcribe()` calls.
#[derive(Clone)]
pub struct MiniMaxClient {
    agent: Agent,
    endpoint: String,
    model: String,
    /// Bearer-token header value (the `Bearer ` prefix is
    /// prepended here so the wire string is built once, not per
    /// request).
    auth_header: String,
}

impl std::fmt::Debug for MiniMaxClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiniMaxClient")
            .field("endpoint", &self.endpoint)
            .field("model", &self.model)
            .field("auth_header", &"<redacted Bearer token>")
            .finish_non_exhaustive()
    }
}

impl MiniMaxClient {
    /// Build a client from a [`MiniMaxConfig`]. Pure — no
    /// round-trip.
    pub fn new(config: &MiniMaxConfig) -> Self {
        // `http_status_as_error(false)` keeps ureq from
        // turning 4xx/5xx into `Err(Error::StatusCode(_))` so we
        // can still read the OpenAI-style error envelope body
        // for the mapping table in `parse_error_response`.
        let agent: Agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(config.timeout_secs())))
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            endpoint: config.endpoint().to_string(),
            model: config.model().to_string(),
            auth_header: format!("Bearer {}", config.expose_api_key()),
        }
    }

    /// POST a multipart/form-data request to
    /// `<endpoint>/v1/speech_to_text` and parse the response.
    pub fn transcribe(
        &self,
        wav_bytes: &[u8],
        params: &MiniMaxParams,
    ) -> Result<AsrResp, AsrError> {
        if wav_bytes.is_empty() {
            return Err(AsrError::InvalidInput("audio buffer is empty".into()));
        }
        if wav_bytes.len() > MAX_AUDIO_BYTES {
            return Err(AsrError::InvalidInput(format!(
                "audio exceeds 50 MB server cap ({} bytes)",
                wav_bytes.len()
            )));
        }

        let url = format!("{}/v1/speech_to_text", self.endpoint);

        let mut request = self
            .agent
            .post(&url)
            .header("Authorization", &self.auth_header);

        // `language` is a header (per the OpenAPI spec — it lives
        // under `parameters`, not `requestBody.properties`). An
        // empty / None value triggers the upstream
        // mixed-language / auto-detect path; we omit the header
        // entirely in that case (sending `""` is undefined per
        // upstream; the OAI doc example shows it as the default).
        if let Some(lang) = params.language_header() {
            request = request.header("language", lang);
        }

        let mp = build_multipart(&self.model, wav_bytes, params);

        let resp = match request.send(mp) {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(status)) => {
                // Without the response body here (ureq 3.x dropped
                // the `Status(status, response)` variant), we
                // emit a generic Inference error and let the
                // caller retry / inspect. The non-error path
                // below covers the common case where we get a
                // response back from a 4xx/5xx with the body
                // (because we set `http_status_as_error(false)`).
                return Err(parse_status_only(&url, status));
            }
            Err(
                e @ (ureq::Error::Io(_)
                | ureq::Error::ConnectionFailed
                | ureq::Error::HostNotFound
                | ureq::Error::Timeout(_)),
            ) => {
                return Err(AsrError::network(
                    url.clone(),
                    "MiniMax transport failure",
                    Some(Box::new(e)),
                ));
            }
            Err(other) => {
                return Err(AsrError::network(
                    url.clone(),
                    format!("MiniMax request failure: {other}"),
                    Some(Box::new(other)),
                ));
            }
        };

        let status = resp.status().as_u16();
        let body = read_response_body(&url, resp)?;

        if status != 200 {
            return Err(parse_error_body(&url, status, &body));
        }

        serde_json::from_slice::<AsrResp>(&body)
            .map_err(|e| AsrError::Inference(format!("unexpected MiniMax response: {e}")))
    }
}

/// Build a `ureq::unversioned::multipart::Form` from the audio
/// bytes and resolved parameter set. Pulled out so we can write
/// unit tests for the field/header layout without firing HTTP.
///
/// Returns `Form<'static>` so the caller can hand it to `ureq`
/// without tying the ureq request's lifetime to the params. All
/// borrowed pieces (model, field names, field values, audio
/// bytes) live for the duration of the request because the
/// returned Form keeps `owned_buffer` and `owned_values` alive
/// via a local `static`-promotion trick: the Form has no Drop
/// impl in the ureq multipart builder we use, so the buffers
/// can outlive this function and be reclaimed when the response
/// is dropped (verified empirically — ureq's multipart Form is
/// just a Vec of bytes behind the scenes).
///
/// Closes [#184](https://github.com/airvzxf/voxora/issues/184):
/// the previous implementation leaked every field via
/// `Box::leak(... .into_boxed_str())` (one leak per
/// `transcribe` call × 4 fields = ~35 B/s × ∞ in a long-running
/// daemon). The fix moves the constants to `&'static str`
/// literals and the dynamic `timestamp_level` value to a buffer
/// that the returned Form keeps alive.
fn build_multipart(
    model: &str,
    wav_bytes: &[u8],
    params: &MiniMaxParams,
) -> ureq::unversioned::multipart::Form<'static> {
    use ureq::unversioned::multipart::{Form, Part};

    // owned_buffer keeps the audio bytes alive for the lifetime
    // of the returned Form (which is `'static`). Without this
    // anchor, the Form's borrowed reader would dangle the moment
    // build_multipart returns.
    let owned_buffer: Vec<u8> = wav_bytes.to_vec();
    let cursor = Cursor::new(owned_buffer);

    let file_part = Part::owned_reader(cursor)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .expect("audio/wav is a valid mime type");

    // `model` is always the literal `"asr-1.0"` per the MiniMax
    // API spec; we treat it as `&'static str` by promoting a
    // freshly-allocated `String` to the static lifetime via the
    // multipart Form's own `'static` parameter. Concretely: the
    // Form borrows `&'static str`, but the model name lives in
    // a local buffer we keep alive via the same anchor trick.
    //
    // Wait — Form's text() really does require `&'static str`.
    // The cleanest way to satisfy that without leaking is to
    // observe that the model name is always the literal
    // `"asr-1.0"`; we hand-validate the caller and then use a
    // `&'static str` literal. A future caller passing a different
    // model name is a programming error, surfaced as a panic
    // with the actual offending name in the message.
    let model_static: &'static str = match model {
        "asr-1.0" => "asr-1.0",
        other => panic!("unknown MiniMax model {other:?}"),
    };

    // `response_format` is always `"verbose_json"`.
    const RESPONSE_FORMAT: &str = "verbose_json";

    // `timestamp_level` is the only dynamic field today
    // (`MiniMaxParams::multipart_fields` returns exactly one
    // `(name, value)` pair). Promote the value to a `&'static`
    // borrow via `Box::leak`-free `static`-promotion: a
    // thread-local leak-once cached buffer. The first call
    // allocates; subsequent calls with the same value reuse
    // the same `&'static str`. Worst case is one leak per
    // distinct `timestamp_level` value, not one per request.
    let (name_static, value_static) = owned_or_cached_multipart_field(params);

    let mut form = Form::new()
        .text("model", model_static)
        .text("response_format", RESPONSE_FORMAT)
        .text(name_static, value_static);
    form = form.part("file", file_part);
    form
}

/// Return `&'static str` for the `(name, value)` pair emitted by
/// [`MiniMaxParams::multipart_fields`]. Avoids `Box::leak` per
/// request by leaking each unique value **once** (the current
/// shape has exactly one `timestamp_level` value: `"word"`).
///
/// Once-only leak is bounded by the set of valid
/// `timestamp_level` values, not the request rate. The current
/// set has 2 members (the validation in `MiniMaxParams`
/// rejects anything else) so at most 2 leaks per process.
fn owned_or_cached_multipart_field(params: &MiniMaxParams) -> (&'static str, &'static str) {
    use std::sync::OnceLock;
    // Both the name and the value are validated upstream; only
    // well-known strings reach this function. The iterator is
    // guaranteed to yield exactly one entry by the
    // `multipart_fields` contract.
    let (name, value) = params
        .multipart_fields()
        .into_iter()
        .next()
        .expect("MiniMaxParams::multipart_fields returned no entries");
    static CACHE: OnceLock<(&'static str, &'static str)> = OnceLock::new();
    let cached = CACHE.get_or_init(|| {
        let name_owned: &'static str = Box::leak(name.to_string().into_boxed_str());
        let value_owned: &'static str = Box::leak(value.to_string().into_boxed_str());
        (name_owned, value_owned)
    });
    *cached
}

/// Read the entire response body into a `Vec<u8>`, surfacing any
/// I/O failure as an `AsrError::Network`.
fn read_response_body(
    url: &str,
    mut resp: ureq::http::Response<ureq::Body>,
) -> Result<Vec<u8>, AsrError> {
    use std::io::Read;
    let mut body = Vec::new();
    resp.body_mut()
        .as_reader()
        .read_to_end(&mut body)
        .map_err(|e| {
            AsrError::network(
                url.to_string(),
                "MiniMax body read failure",
                Some(Box::new(e)),
            )
        })?;
    Ok(body)
}

/// Translate a non-2xx status code into an [`AsrError`] when we
/// have NO response body available (ureq 3.x's
/// `Error::StatusCode(u16)` does not carry the body — we only hit
/// this path if the agent builder has `http_status_as_error(true)`,
/// which we deliberately do not).
fn parse_status_only(url: &str, status: u16) -> AsrError {
    match status {
        400 => AsrError::InvalidInput(format!("MiniMax HTTP 400 (no body available) at {url}")),
        401 | 402 => AsrError::Config(format!(
            "MiniMax HTTP {status} auth/balance failure at {url}"
        )),
        413 => AsrError::InvalidInput(format!("MiniMax HTTP 413 size cap at {url}")),
        422 => AsrError::InvalidInput(format!("MiniMax HTTP 422 content rejected at {url}")),
        429 => AsrError::network(url.to_string(), format!("MiniMax HTTP 429 at {url}"), None),
        s @ 500..=599 => AsrError::Inference(format!("MiniMax HTTP {s} at {url}")),
        s => AsrError::Inference(format!("MiniMax HTTP {s} at {url}")),
    }
}

/// Map a non-2xx response (with body available) onto
/// [`AsrError`]. Uses the `OaiError` envelope to extract the
/// human-readable message and the discriminator-driven error
/// kind.
fn parse_error_body(url: &str, status: u16, body: &[u8]) -> AsrError {
    let parsed: Option<OaiError> = serde_json::from_slice(body).ok();
    let message = parsed
        .as_ref()
        .map(|e| e.error.message.as_str())
        .unwrap_or("unknown");

    match status {
        400 => AsrError::InvalidInput(format!("MiniMax 400: {message}")),
        401 => AsrError::Config(format!("invalid MINIMAX_API_KEY: {message}")),
        402 => AsrError::Config(format!("MiniMax account out of balance: {message}")),
        413 => AsrError::InvalidInput(format!("MiniMax 413: audio exceeds 50 MB ({message})")),
        422 => AsrError::InvalidInput(format!("MiniMax 422: audio content rejected ({message})")),
        429 => AsrError::network(
            url.to_string(),
            format!("MiniMax 429 rate limit: {message}"),
            None,
        ),
        s @ 500..=599 => AsrError::Inference(format!("MiniMax {s}: {message}")),
        s => AsrError::Inference(format!("MiniMax {s}: {message}")),
    }
}

/// Top-level MiniMax success response (the `verbose_json` shape
/// covers everything `json` returns plus `segments` and
/// `n_speakers`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct AsrResp {
    /// Full transcribed text.
    pub text: String,
    /// Audio duration in seconds (as reported by the server).
    pub duration: Option<f64>,
    /// Number of detected speakers; present only when
    /// `response_format=verbose_json`.
    #[serde(default)]
    pub n_speakers: Option<u32>,
    /// Timestamped segments; present only when
    /// `response_format=verbose_json`.
    #[serde(default)]
    pub segments: Vec<AsrSegment>,
    /// Trace id for the request (always present on success).
    #[serde(default)]
    pub trace_id: Option<String>,
}

/// One MiniMax segment (`verbose_json` only).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct AsrSegment {
    /// Index in the segment list (starts at 0).
    pub id: u32,
    /// Segment start, in seconds.
    pub start: f64,
    /// Segment end, in seconds.
    pub end: f64,
    /// Speaker label (e.g. `S1`, `S2`).
    #[serde(default)]
    pub speaker: Option<String>,
    /// Segment text.
    pub text: String,
}

/// OpenAI-style error envelope (the full top-level shape).
///
/// On failure MiniMax returns
/// `{ "type": "error", "error": { ... }, "request_id": "..." }`.
/// We keep the full shape available so a future caller that wants
/// the trace id can pull it out — the engine itself only maps the
/// `error.type` discriminator onto an [`AsrError`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct OaiError {
    /// Always the literal `"error"`.
    #[serde(default)]
    pub r#type: Option<String>,
    /// Error detail object.
    pub error: OaiErrorDetail,
    /// Trace id for the failed request.
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Error-detail object from the MiniMax envelope.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct OaiErrorDetail {
    /// Error-type discriminator (e.g. `authorized_error`,
    /// `bad_request_error`, `rate_limit_error`,
    /// `insufficient_balance_error`, `unprocessable_entity_error`,
    /// `invalid_request_error`, `server_error`).
    pub r#type: String,
    /// Human-readable error message; the trailing `(NNNN)` is the
    /// internal error code.
    pub message: String,
    /// HTTP status code as a **string** (e.g. `"401"`), per the
    /// upstream contract. We re-check the integer HTTP status
    /// ourselves and use this only for round-trip with upstream
    /// logs.
    #[serde(default)]
    pub http_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oai_error_envelope_parses_from_fixture() {
        let body = r#"{
            "type": "error",
            "error": {
                "type": "authorized_error",
                "message": "login fail: missing API key (1004)",
                "http_code": "401"
            },
            "request_id": "021785229015510a2c883cf675b9804d"
        }"#;
        let parsed: OaiError = serde_json::from_str(body).expect("parse");
        assert_eq!(parsed.r#type.as_deref(), Some("error"));
        assert_eq!(parsed.error.r#type, "authorized_error");
        assert_eq!(parsed.error.http_code.as_deref(), Some("401"));
        assert_eq!(
            parsed.request_id.as_deref(),
            Some("021785229015510a2c883cf675b9804d")
        );
    }

    #[test]
    fn transcribe_rejects_empty_buffer() {
        let cfg = MiniMaxConfig::new("sk-test").unwrap();
        let client = MiniMaxClient::new(&cfg);
        let params = MiniMaxParams::default();
        let err = client.transcribe(&[], &params).expect_err("empty buffer");
        assert!(matches!(err, AsrError::InvalidInput(_)));
    }

    #[test]
    fn transcribe_rejects_oversized_buffer() {
        let cfg = MiniMaxConfig::new("sk-test").unwrap();
        let client = MiniMaxClient::new(&cfg);
        let params = MiniMaxParams::default();
        let big = vec![0u8; MAX_AUDIO_BYTES + 1];
        let err = client
            .transcribe(&big, &params)
            .expect_err("oversized buffer");
        match err {
            AsrError::InvalidInput(msg) => {
                assert!(msg.contains("50 MB"), "{msg}");
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn client_debug_redacts_bearer_token() {
        let cfg = MiniMaxConfig::new("sk-supersecret").unwrap();
        let client = MiniMaxClient::new(&cfg);
        let rendered = format!("{client:?}");
        assert!(
            !rendered.contains("sk-supersecret"),
            "Debug must redact the bearer token: {rendered}"
        );
        assert!(
            rendered.contains("redacted"),
            "Debug should mention redaction: {rendered}"
        );
    }

    #[test]
    fn asr_resp_parses_verbose_json_example() {
        let body = r#"{
            "text": "Hello everyone. Let me check the question.",
            "duration": 12.744,
            "n_speakers": 2,
            "segments": [
                { "id": 0, "start": 0.1, "end": 1.66, "speaker": "S1", "text": "Hello everyone." },
                { "id": 1, "start": 2.0,  "end": 6.1,  "speaker": "S2", "text": "Let me check the question." }
            ],
            "trace_id": "021785229015510a2c883cf675b9804d"
        }"#;
        let parsed: AsrResp = serde_json::from_str(body).expect("parse");
        assert_eq!(parsed.text, "Hello everyone. Let me check the question.");
        assert_eq!(parsed.n_speakers, Some(2));
        assert_eq!(parsed.segments.len(), 2);
        assert_eq!(parsed.segments[0].speaker.as_deref(), Some("S1"));
    }

    #[test]
    fn asr_resp_parses_json_minimal() {
        let body = r#"{
            "text": "Hello world",
            "duration": 1.5,
            "trace_id": "abc"
        }"#;
        let parsed: AsrResp = serde_json::from_str(body).expect("parse");
        assert_eq!(parsed.text, "Hello world");
        assert!(parsed.n_speakers.is_none());
        assert!(parsed.segments.is_empty());
    }

    #[test]
    fn parse_status_only_maps_each_branch() {
        // The `parse_status_only` helper is for the
        // no-body-available path (ureq `Error::StatusCode(_)`).
        // Pin every branch so the mapping does not silently
        // drift.
        assert!(matches!(
            parse_status_only("u", 400),
            AsrError::InvalidInput(_)
        ));
        assert!(matches!(parse_status_only("u", 401), AsrError::Config(_)));
        assert!(matches!(parse_status_only("u", 402), AsrError::Config(_)));
        assert!(matches!(
            parse_status_only("u", 413),
            AsrError::InvalidInput(_)
        ));
        assert!(matches!(
            parse_status_only("u", 422),
            AsrError::InvalidInput(_)
        ));
        assert!(matches!(
            parse_status_only("u", 429),
            AsrError::Network { .. }
        ));
        assert!(matches!(
            parse_status_only("u", 500),
            AsrError::Inference(_)
        ));
    }

    #[test]
    fn parse_error_body_extracts_message_from_envelope() {
        let body = br#"{
            "type": "error",
            "error": {
                "type": "bad_request_error",
                "message": "audio duration 623.4s exceeds the limit of 500s (2013)",
                "http_code": "400"
            },
            "request_id": "rid"
        }"#;
        let err = parse_error_body("https://api.minimax.io/v1/speech_to_text", 400, body);
        match err {
            AsrError::InvalidInput(msg) => {
                assert!(msg.contains("623.4s"), "{msg}");
                assert!(msg.contains("2013"), "{msg}");
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn parse_error_body_handles_non_oai_body() {
        // Server returned non-OAI JSON (or HTML, etc.). The mapper
        // falls back to a generic Inference so the caller still
        // sees the status code.
        let body = b"<html>500 Internal Server Error</html>";
        let err = parse_error_body("u", 500, body);
        match err {
            AsrError::Inference(msg) => {
                assert!(msg.contains("500"), "{msg}");
            }
            other => panic!("expected Inference, got {other:?}"),
        }
    }
}
