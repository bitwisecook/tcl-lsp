//! Shared file-input helpers for f5 verbs.
//!
//! `.ucs` archives — plain *or* encrypted (`OpenPGP`), and gzipped / `OpenPGP`
//! streams from stdin — are transparently extracted to SCF, so every verb that
//! reads via [`read_path`] accepts a UCS the same way it accepts `.scf` /
//! `.conf`. This is the I/O shell over the pure [`crate::ucs`] core: the only
//! place in the crate that touches the filesystem / stdin.

use std::io::Read;
use std::path::Path;

use tcl_bigip::parser::{BigipConfig, parse_bigip_conf};

use crate::ucs::{
    DEFAULT_PASSPHRASE_ENV, UcsError, is_pgp_bytes, is_ucs_bytes, ucs_archive_to_scf,
};

/// An error resolving a path input. Rendered as `error: {msg}` (exit 2) by the
/// binary, mirroring `(OSError, ValueError, FileNotFoundError)`
/// handling.
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    /// Missing file: matches `FileNotFoundError(f"not a file: {path}")`.
    #[error("not a file: {0}")]
    NotAFile(String),
    /// An I/O failure reading the file or stdin.
    #[error("{0}")]
    Io(String),
    /// A UCS / decode failure (invalid archive, wrong passphrase, …).
    #[error("{0}")]
    Ucs(String),
}

impl From<UcsError> for PathError {
    fn from(e: UcsError) -> Self {
        PathError::Ucs(e.0)
    }
}

/// How to resolve a UCS decryption passphrase (the `add_passphrase_args` /
/// `provider_from_args` shape
#[derive(Debug, Clone)]
pub struct PassphraseOptions {
    /// An explicit passphrase (highest priority).
    pub explicit: Option<String>,
    /// Environment variable consulted when no explicit value is given.
    pub env_var: String,
    /// Whether interactive prompting is permitted.
    pub allow_prompt: bool,
    /// Interactive-prompt callback, supplied by a binary. The pure library
    /// performs no I/O itself; when `allow_prompt` is set and neither
    /// `explicit` nor the env var resolved a value, this is invoked as the last
    /// resort (a binary wires a secure TTY prompt here). `Err`/empty falls
    /// through to the standard "set the env var" error — e.g. when there is no
    /// controlling terminal — matching the Python provider's TTY fallback.
    pub prompt: Option<fn() -> Result<String, String>>,
}

impl Default for PassphraseOptions {
    fn default() -> Self {
        PassphraseOptions {
            explicit: None,
            env_var: DEFAULT_PASSPHRASE_ENV.to_owned(),
            allow_prompt: true,
            prompt: None,
        }
    }
}

/// Resolve a passphrase: explicit → `$env_var` → interactive prompt → error.
///
/// Mirrors `resolve_passphrase`'s resolution order and its
/// `UcsPassphraseError` message. The pure library never performs I/O itself; the
/// interactive step calls [`PassphraseOptions::prompt`] (a binary-supplied
/// secure TTY prompt) only when [`PassphraseOptions::allow_prompt`] is set and
/// nothing earlier resolved a value.
pub fn resolve_passphrase(opts: &PassphraseOptions) -> Result<String, UcsError> {
    if let Some(p) = &opts.explicit
        && !p.is_empty()
    {
        return Ok(p.clone());
    }
    if !opts.env_var.is_empty()
        && let Ok(v) = std::env::var(&opts.env_var)
        && !v.is_empty()
    {
        return Ok(v);
    }
    if opts.allow_prompt
        && let Some(prompt) = opts.prompt
        && let Ok(p) = prompt()
        && !p.is_empty()
    {
        return Ok(p);
    }
    Err(UcsError(format!(
        "this UCS archive is encrypted and requires a passphrase; set the {} \
         environment variable or pass it explicitly",
        opts.env_var
    )))
}

/// `_UCS_SUFFIXES` — extensions that force UCS handling.
fn is_ucs_suffix(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ucs"))
}

/// Whether `raw` should be extracted/decrypted as a UCS archive (mirrors
/// `_looks_like_ucs`): `OpenPGP` magic is unambiguous, so an encrypted UCS is
/// recognised regardless of extension; plain gzip magic is only treated as a
/// UCS for `.ucs` paths or stdin.
fn looks_like_ucs(raw: &[u8], is_ucs_ext: bool, is_stdin: bool) -> bool {
    if is_pgp_bytes(raw) {
        return true;
    }
    if is_ucs_bytes(raw) {
        return is_stdin || is_ucs_ext;
    }
    false
}

/// Return `(uri, source)` for `path_str`. `-` reads stdin.
///
/// `.ucs` archives (plain or OpenPGP-encrypted) — and gzip / `OpenPGP` streams
/// from stdin — are transparently extracted to SCF via [`ucs_archive_to_scf`].
/// Other inputs are decoded UTF-8 with lossy replacement (Python `errors=
/// "replace"`); `strict` makes an undecodable byte an error instead.
pub fn read_path(
    path_str: &str,
    strict: bool,
    opts: &PassphraseOptions,
) -> Result<(String, String), PathError> {
    let provider = move || resolve_passphrase(opts);

    if path_str == "-" {
        let mut raw = Vec::new();
        std::io::stdin()
            .read_to_end(&mut raw)
            .map_err(|e| PathError::Io(format!("failed to read stdin: {e}")))?;
        if looks_like_ucs(&raw, false, true) {
            let scf = ucs_archive_to_scf(&raw, &provider, false, "stdin")?;
            return Ok(("stdin://input".to_owned(), scf));
        }
        return Ok(("stdin://input".to_owned(), decode(&raw, strict)?));
    }

    let path = Path::new(path_str);
    let abs = std::fs::canonicalize(path).map_err(|_| PathError::NotAFile(path_str.to_owned()))?;
    if !abs.is_file() {
        return Err(PathError::NotAFile(path_str.to_owned()));
    }
    let raw = std::fs::read(&abs)
        .map_err(|e| PathError::Io(format!("failed to read {path_str}: {e}")))?;
    if looks_like_ucs(&raw, is_ucs_suffix(path), false) {
        let scf = ucs_archive_to_scf(&raw, &provider, false, path_str)?;
        return Ok((file_uri(&abs), scf));
    }
    if is_ucs_suffix(path) {
        return Err(PathError::Ucs(format!(
            "{path_str}: not a valid UCS archive"
        )));
    }
    Ok((file_uri(&abs), decode(&raw, strict)?))
}

/// Decode bytes to text: lossy replacement, or strict (error on invalid UTF-8).
fn decode(raw: &[u8], strict: bool) -> Result<String, PathError> {
    if strict {
        String::from_utf8(raw.to_vec())
            .map_err(|e| PathError::Io(format!("invalid UTF-8 in input: {e}")))
    } else {
        Ok(String::from_utf8_lossy(raw).into_owned())
    }
}

/// A resolved config input: its URI key, the post-decode source text (the
/// *extracted* SCF for a UCS), and the parsed model.
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    /// `file://` URI (or `stdin://input`) — the canonical key.
    pub uri: String,
    /// The source text fed to the parser.
    pub source: String,
    /// The parsed BIG-IP config.
    pub config: BigipConfig,
}

/// Resolve a list of path inputs into ordered [`LoadedConfig`]s. Each path is
/// read via [`read_path`] and parsed with the
/// `Common` default partition.
pub fn load_paths(
    paths: &[String],
    opts: &PassphraseOptions,
) -> Result<Vec<LoadedConfig>, PathError> {
    let mut out = Vec::with_capacity(paths.len());
    for p in paths {
        let (uri, source) = read_path(p, false, opts)?;
        let config = parse_bigip_conf(&source, "Common");
        out.push(LoadedConfig {
            uri,
            source,
            config,
        });
    }
    Ok(out)
}

/// Build a `file://` URI from an absolute path, percent-encoding like Python's
/// `Path.as_uri()` (`urllib.quote` with `safe="/"`).
fn file_uri(abs: &Path) -> String {
    use std::fmt::Write as _;
    let s = abs.to_string_lossy();
    let mut uri = String::from("file://");
    for &b in s.as_bytes() {
        let safe = b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b'~' | b'/');
        if safe {
            uri.push(b as char);
        } else {
            let _ = write!(uri, "%{b:02X}");
        }
    }
    uri
}
