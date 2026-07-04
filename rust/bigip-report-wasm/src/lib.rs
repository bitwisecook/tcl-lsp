//! WebAssembly binding for the F5 BIG-IP report generator.
//!
//! The whole pipeline runs in the browser: [`extract_source`] turns an uploaded
//! `.ucs` (plain or OpenPGP-encrypted, with an optional passphrase) or a plain
//! `bigip.conf`/SCF into SCF text, and [`generate_report`] runs the `f5-query`
//! engine over one or more such sources and returns a single, self-contained
//! HTML report. Nothing is uploaded and no network is used once the wasm module
//! has loaded — the decrypt (RustCrypto), parse, query and render steps are all
//! compiled in.
//!
//! The network-probe builtins are compiled out (`tcl-bigip-report` →
//! `tcl-bigip-query` with `default-features = false`), so this binary has no
//! socket / TLS / x509 dependency.

use wasm_bindgen::prelude::*;

use tcl_bigip_io::{
    is_pgp_bytes, is_ucs_bytes, resolve_passphrase, ucs_archive_to_scf, PassphraseOptions,
};
use tcl_bigip_report::{
    build_report, count_encrypted_secrets, decrypt_secrets as report_decrypt_secrets, RenderOptions,
    Source,
};

/// Turn one uploaded file into SCF text.
///
/// * `name` — the file name (its `.ucs` extension disambiguates a plain-gzip
///   UCS from an arbitrary gzip payload).
/// * `bytes` — the raw file bytes.
/// * `passphrase` — the UCS passphrase; empty means "none" (an OpenPGP-encrypted
///   UCS then errors with a clear "passphrase required" message).
///
/// An OpenPGP-encrypted archive, or a gzip `.ucs`, is decrypted/extracted to
/// SCF; anything else is decoded as UTF-8 config text. Master-key secret
/// decryption is a separate step ([`decrypt_secrets`]) so the UI can first
/// detect whether a master key is even needed ([`secret_count`]). Returns a
/// `JsError` carrying the extraction failure (bad passphrase, corrupt archive).
#[wasm_bindgen]
pub fn extract_source(name: &str, bytes: &[u8], passphrase: &str) -> Result<String, JsError> {
    let is_ucs_ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ucs"));

    if is_pgp_bytes(bytes) || (is_ucs_bytes(bytes) && is_ucs_ext) {
        let opts = PassphraseOptions {
            explicit: (!passphrase.is_empty()).then(|| passphrase.to_owned()),
            allow_prompt: false,
            ..PassphraseOptions::default()
        };
        let provider = move || resolve_passphrase(&opts);
        ucs_archive_to_scf(bytes, &provider, false, name).map_err(|e| JsError::new(&e.to_string()))
    } else {
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// Count the `f5mku` `$M$…` encrypted secrets in an SCF source.
///
/// The page uses this to decide whether to ask for a master key at all — the
/// input only appears when at least one source carries encrypted secrets.
#[wasm_bindgen]
pub fn secret_count(scf: &str) -> usize {
    count_encrypted_secrets(scf)
}

/// Decrypt the `f5mku` `$M$…` secrets in one SCF source with the base64
/// `master_key` (`f5mku -K`). Returns the SCF with secrets in clear, or a
/// `JsError` if the key is wrong / malformed.
#[wasm_bindgen]
pub fn decrypt_secrets(scf: &str, master_key: &str) -> Result<String, JsError> {
    report_decrypt_secrets(scf, master_key)
        .map(|(text, _n)| text)
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Render a standalone HTML report from one or more SCF sources.
///
/// * `sources_json` — an ordered JSON array of `[uri, scf_text]` pairs (each
///   `uri` a display name, each `scf_text` the output of [`extract_source`]).
/// * `title` — the report title.
/// * `generated_at` — a generation timestamp string (the caller stamps it with
///   the browser's local clock; the engine itself is time-free).
/// * `embed_console` — embed the in-browser `f5-query` WASM console.
///
/// Returns the full HTML document, or a `JsError` carrying the engine's error.
#[wasm_bindgen]
pub fn generate_report(
    sources_json: &str,
    title: &str,
    generated_at: &str,
    embed_console: bool,
) -> Result<String, JsError> {
    let sources: Vec<Source> = serde_json::from_str(sources_json)
        .map_err(|e| JsError::new(&format!("invalid sources JSON: {e}")))?;
    if sources.is_empty() {
        return Err(JsError::new("no config sources provided"));
    }
    let opts = RenderOptions {
        title: title.to_owned(),
        generated_at: generated_at.to_owned(),
        embed_console,
    };
    build_report(&sources, &opts).map_err(|e| JsError::new(&e.to_string()))
}

/// The report engine version string (for the page's status line).
#[wasm_bindgen]
pub fn engine_version() -> String {
    tcl_bigip_report::ENGINE_VERSION.to_string()
}
