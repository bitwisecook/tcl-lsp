//! iControl REST transport for the `f5` remote verbs.
//!
//! Built on `ureq` 3.x + `rustls`. The BIG-IP management interface self-signs
//! by default, so `insecure` (the verb default) disables certificate
//! verification via [`ureq::tls::TlsConfig::disable_verification`].
//!
//! This transport requires a live device, so it is **not** exercised by the
//! offline tests; the golden tests cover the `--dry-run`, credential, and
//! error surfaces only.

use std::time::Duration;

use base64::Engine;
use ureq::config::Config;
use ureq::http::Request;
use ureq::tls::TlsConfig;
use ureq::{Agent, Body};

use super::auth::Credentials;

/// A minimal HTTP response: status code plus the raw body bytes.
pub struct RestResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// A REST transport error. Split so callers can react to a failed *connection*
/// (the `auto` fetch transport falls back to SSH on one) without matching on
/// human-facing error text.
#[derive(Debug)]
pub enum RestError {
    /// The device could not be reached — a connect / TLS / transport failure.
    Connection(String),
    /// Any other REST failure: a non-2xx HTTP status, a body read failure, or
    /// request construction.
    Other(String),
}

impl std::fmt::Display for RestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(m) | Self::Other(m) => f.write_str(m),
        }
    }
}

impl From<RestError> for String {
    fn from(e: RestError) -> Self {
        e.to_string()
    }
}

/// Build the `Authorization: Basic …` header value for *credentials*.
fn auth_header(credentials: &Credentials) -> String {
    let raw = format!("{}:{}", credentials.user, credentials.password);
    let encoded = base64::engine::general_purpose::STANDARD.encode(raw.as_bytes());
    format!("Basic {encoded}")
}

/// Construct a `ureq::Agent` honouring `insecure` (TLS verification) and the
/// per-request timeout.
fn build_agent(insecure: bool, timeout: f64) -> Agent {
    let tls = TlsConfig::builder().disable_verification(insecure).build();
    let mut builder = Config::builder()
        .tls_config(tls)
        // Inspect non-2xx status manually, so do not let ureq turn 4xx/5xx
        // into transport errors.
        .http_status_as_error(false);
    if timeout > 0.0 {
        builder = builder.timeout_global(Some(Duration::from_secs_f64(timeout)));
    }
    Agent::new_with_config(builder.build())
}

/// Maximum body we will buffer from a single response (256 MiB) — large enough
/// for a UCS archive download.
const MAX_BODY: u64 = 256 * 1024 * 1024;

/// Issue one request against `https://<host>:<port><path>`.
///
/// `accept` defaults to `application/json`; pass [`Accept::Octet`] for binary
/// downloads. Returns the status + buffered body.
///
/// # Errors
/// [`RestError::Connection`] on a connect / TLS failure, [`RestError::Other`]
/// on a request-build / read failure.
pub fn request(
    credentials: &Credentials,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    insecure: bool,
    timeout: f64,
) -> Result<RestResponse, RestError> {
    request_with_accept(
        credentials,
        method,
        path,
        body,
        Accept::Json,
        insecure,
        timeout,
    )
}

/// The `Accept` header to request.
#[derive(Clone, Copy)]
pub enum Accept {
    Json,
    Octet,
}

impl Accept {
    fn header(self) -> &'static str {
        match self {
            Accept::Json => "application/json",
            Accept::Octet => "application/octet-stream",
        }
    }
}

/// Issue one request with an explicit `Accept` header.
///
/// # Errors
/// [`RestError::Connection`] on a connect / TLS failure, [`RestError::Other`]
/// on a request-build / read failure.
pub fn request_with_accept(
    credentials: &Credentials,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    accept: Accept,
    insecure: bool,
    timeout: f64,
) -> Result<RestResponse, RestError> {
    let agent = build_agent(insecure, timeout);
    let url = format!("https://{}:{}{}", credentials.host, credentials.port, path);

    let mut req = Request::builder()
        .method(method)
        .uri(&url)
        .header("Authorization", auth_header(credentials))
        .header("Accept", accept.header());
    if body.is_some() {
        req = req.header("Content-Type", "application/json");
    }
    let owned = body.map(<[u8]>::to_vec).unwrap_or_default();
    let request = req
        .body(owned)
        .map_err(|e| RestError::Other(format!("REST request to {}: {e}", credentials.host)))?;

    let mut resp = agent
        .run(request)
        .map_err(|e| RestError::Connection(format!("REST connection to {}: {e}", credentials.host)))?;
    let status = resp.status().as_u16();
    let bytes = read_body(resp.body_mut())
        .map_err(|e| RestError::Other(format!("REST read from {}: {e}", credentials.host)))?;
    Ok(RestResponse {
        status,
        body: bytes,
    })
}

fn read_body(body: &mut Body) -> Result<Vec<u8>, ureq::Error> {
    body.with_config().limit(MAX_BODY).read_to_vec()
}

/// Render the `data[:n]` bytes-repr prefix used in error messages
/// (`b'...'`). Best-effort: only the common printable / escape cases are
/// reproduced, which is all the error paths surface.
#[must_use]
pub fn repr_bytes(data: &[u8], limit: usize) -> String {
    use std::fmt::Write as _;
    let slice = &data[..data.len().min(limit)];
    let mut out = String::from("b'");
    for &b in slice {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'\'' => out.push_str("\\'"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\x{b:02x}");
            }
        }
    }
    out.push('\'');
    out
}

/// POST `/mgmt/tm/sys/ucs` with `{"command":"save","name":<name>}` to trigger
/// UCS-archive creation on the device.
///
/// # Errors
/// [`RestError::Connection`] if the device is unreachable, else
/// [`RestError::Other`] (serialisation / non-2xx HTTP status).
pub fn save_ucs(
    credentials: &Credentials,
    name: &str,
    insecure: bool,
    timeout: f64,
) -> Result<(), RestError> {
    let payload = serde_json::json!({"command": "save", "name": name});
    let body = serde_json::to_vec(&payload).map_err(|e| RestError::Other(e.to_string()))?;
    let resp = request(
        credentials,
        "POST",
        "/mgmt/tm/sys/ucs",
        Some(&body),
        insecure,
        timeout,
    )?;
    if resp.status >= 400 {
        return Err(RestError::Other(format!(
            "POST /mgmt/tm/sys/ucs -> HTTP {}: {}",
            resp.status,
            repr_bytes(&resp.body, 400)
        )));
    }
    Ok(())
}

/// GET `path`, retrying briefly (404 / 503) while the device flushes the file.
///
/// # Errors
/// [`RestError::Connection`] if the device is unreachable, else
/// [`RestError::Other`] with the last HTTP error once the poll window elapses.
pub fn download(
    credentials: &Credentials,
    path: &str,
    insecure: bool,
    timeout: f64,
) -> Result<Vec<u8>, RestError> {
    let poll_interval = Duration::from_millis(500);
    let deadline = std::time::Instant::now() + Duration::from_mins(1);
    let mut last_status = 0u16;
    let mut last_body: Vec<u8> = Vec::new();
    while std::time::Instant::now() < deadline {
        let resp = request_with_accept(
            credentials,
            "GET",
            path,
            None,
            Accept::Octet,
            insecure,
            timeout,
        )?;
        if resp.status == 200 {
            return Ok(resp.body);
        }
        last_status = resp.status;
        last_body = resp.body;
        if matches!(last_status, 404 | 503) {
            std::thread::sleep(poll_interval);
            continue;
        }
        break;
    }
    Err(RestError::Other(format!(
        "GET {path} -> HTTP {last_status}: {}",
        repr_bytes(&last_body, 200)
    )))
}
