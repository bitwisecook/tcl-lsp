// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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
//! The network-probe builtins are compiled out (`bigip-report-gen-rust` →
//! `tcl-bigip-query` with `default-features = false`), so this binary has no
//! socket / TLS / x509 dependency.

use wasm_bindgen::prelude::*;

use bigip_report_gen_rust::{
    RenderOptions, Source, build_report, count_encrypted_secrets,
    decrypt_secrets as report_decrypt_secrets,
};
use tcl_bigip_io::{
    PassphraseOptions, is_pgp_bytes, is_ucs_bytes, resolve_passphrase, ucs_archive_to_scf,
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

/// Recover certificate and private-key PEMs from a UCS filestore, keyed by
/// `cache-path`.
///
/// A modern `sys file ssl-cert` stanza is metadata-free — subject / issuer /
/// validity / SANs live only in the PEM in `/config/filestore/...`, not the
/// config — so the certs tab needs the actual bytes. Given the same archive
/// `bytes` that [`extract_source`] flattened and its resulting `scf`, this parses
/// every `sys file ssl-cert` and `sys file ssl-key` cache-path and reads those
/// members out of the archive, returning a JSON object `{cache_path: pem}`.
/// Private key bytes are used only for an in-memory SPKI match and are never
/// embedded in the report model or HTML. A non-UCS input (a bare
/// `bigip.conf`) has no filestore and returns `{}`. Feed the merged map of all
/// files into [`generate_report`].
#[wasm_bindgen]
pub fn extract_cert_files(
    name: &str,
    bytes: &[u8],
    passphrase: &str,
    scf: &str,
) -> Result<String, JsError> {
    use std::collections::BTreeMap;

    use tcl_bigip::model::ModelObject;
    use tcl_bigip::parser::driver::parse_bigip_conf;
    use tcl_bigip_io::read_ucs_member;

    let is_ucs_ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ucs"));
    if !(is_pgp_bytes(bytes) || (is_ucs_bytes(bytes) && is_ucs_ext)) {
        return Ok("{}".to_string());
    }

    let opts = PassphraseOptions {
        explicit: (!passphrase.is_empty()).then(|| passphrase.to_owned()),
        allow_prompt: false,
        ..PassphraseOptions::default()
    };
    let provider = move || resolve_passphrase(&opts);

    let config = parse_bigip_conf(scf, "Common");
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    for placed in &config.objects {
        let (cache_path, source_path) = match &placed.object {
            ModelObject::SysFileSslCert(file) => (&file.cache_path, &file.source_path),
            ModelObject::SysFileSslKey(file) => (&file.cache_path, &file.source_path),
            _ => continue,
        };
        let member = if cache_path.is_empty() {
            source_path
                .strip_prefix("file://")
                .unwrap_or(source_path)
                .to_owned()
        } else {
            cache_path.clone()
        };
        if member.is_empty() || map.contains_key(&member) {
            continue;
        }
        if let Ok(pem) = read_ucs_member(bytes, &member, &provider, name) {
            map.insert(member, String::from_utf8_lossy(&pem).into_owned());
        }
    }
    serde_json::to_string(&map).map_err(|e| JsError::new(&e.to_string()))
}

/// Recover the forensic file inventory from a UCS archive.
///
/// Enumerates the members an attacker tampers with on a compromised BIG-IP
/// (login-persistence dotfiles + SSH keys, the local account databases,
/// external-auth / PAM config, syslog-ng, cron — see
/// [`tcl_bigip_io::MemberScope::Forensic`]) with per-file metadata (size,
/// SHA-256, text/binary), and reads back the content of the small text files so
/// the Forensics tab can run its heuristics and show them. Content over
/// `MAX_FORENSIC_CONTENT` bytes, or binary, is metadata-only. A non-UCS input
/// (a bare `bigip.conf`) returns `[]`. Returns a JSON array of
/// `{path, size, sha256, isText, content?}`; feed the map `{name: <that array>}`
/// into [`generate_report`]. Nothing leaves the page.
#[wasm_bindgen]
pub fn extract_files(name: &str, bytes: &[u8], passphrase: &str) -> Result<String, JsError> {
    use tcl_bigip_io::{MemberScope, list_ucs_members, read_ucs_member};

    /// Cap on embedded content per file (256 KiB): forensic config/auth files
    /// are small, but a runaway log-config include shouldn't bloat the page.
    const MAX_FORENSIC_CONTENT: u64 = 256 * 1024;

    let is_ucs_ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ucs"));
    if !(is_pgp_bytes(bytes) || (is_ucs_bytes(bytes) && is_ucs_ext)) {
        return Ok("[]".to_string());
    }

    let opts = PassphraseOptions {
        explicit: (!passphrase.is_empty()).then(|| passphrase.to_owned()),
        allow_prompt: false,
        ..PassphraseOptions::default()
    };
    let provider = move || resolve_passphrase(&opts);

    let entries = list_ucs_members(bytes, &provider, name, MemberScope::Forensic)
        .map_err(|e| JsError::new(&e.to_string()))?;

    let mut out: Vec<serde_json::Value> = Vec::with_capacity(entries.len());
    for e in entries {
        // Read content for small text files only; the Forensics shaper drops
        // sensitive bodies (shadow / private keys) before they reach the DOM.
        let content = if e.is_text && e.size <= MAX_FORENSIC_CONTENT {
            read_ucs_member(bytes, &e.path, &provider, name)
                .ok()
                .map(|b| String::from_utf8_lossy(&b).into_owned())
        } else {
            None
        };
        out.push(serde_json::json!({
            "path": e.path,
            "size": e.size,
            "sha256": e.sha256,
            "isText": e.is_text,
            "content": content,
        }));
    }
    serde_json::to_string(&out).map_err(|e| JsError::new(&e.to_string()))
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
/// * `cert_files_json` — a JSON object `{cache_path: pem}` containing
///   certificate and private-key material from every file's
///   [`extract_cert_files`], nested **by source URI** —
///   `{uri: {cache_path: pem}}` — so the certs tab can parse the real
///   certificates (issue date, SANs, trust chain) and a filestore `cache-path`
///   shared across two devices doesn't collide. `"{}"` for none.
/// * `files_json` — a JSON object `{uri: [{path, size, sha256, isText,
///   content?}]}` from each file's [`extract_files`], powering the Forensics
///   tab. `"{}"` for none.
/// * `title` — the report title.
/// * `generated_at` — a generation timestamp string (the caller stamps it with
///   the browser's local clock; the engine itself is time-free).
/// * `embed_console` — embed the in-browser `f5-query` WASM console.
/// * `architecture_manifest` — an optional architecture manifest declaring each
///   device's role/tier and explicit inter-device links. It is a small **Tcl
///   script** (`device <match> -role R -tier N -label L`, `link <from> <to>`),
///   parsed with the project's Tcl tokeniser. Empty means pure auto-detection
///   (an upstream device's pool member / GTM server address that a downstream
///   device serves is a tier hop). A malformed manifest is surfaced in the
///   report's Architecture section, not fatal.
///
/// Returns the full HTML document, or a `JsError` carrying the engine's error.
#[wasm_bindgen]
pub fn generate_report(
    sources_json: &str,
    cert_files_json: &str,
    files_json: &str,
    title: &str,
    generated_at: &str,
    embed_console: bool,
    architecture_manifest: &str,
    report_id: &str,
    settings_json: &str,
) -> Result<String, JsError> {
    let sources: Vec<Source> = serde_json::from_str(sources_json)
        .map_err(|e| JsError::new(&format!("invalid sources JSON: {e}")))?;
    if sources.is_empty() {
        return Err(JsError::new("no config sources provided"));
    }
    let cert_pems: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
        if cert_files_json.trim().is_empty() {
            std::collections::HashMap::new()
        } else {
            serde_json::from_str(cert_files_json)
                .map_err(|e| JsError::new(&format!("invalid cert-files JSON: {e}")))?
        };
    let files: std::collections::HashMap<String, Vec<serde_json::Value>> =
        if files_json.trim().is_empty() {
            std::collections::HashMap::new()
        } else {
            serde_json::from_str(files_json)
                .map_err(|e| JsError::new(&format!("invalid files JSON: {e}")))?
        };
    let architecture = if architecture_manifest.trim().is_empty() {
        None
    } else {
        Some(architecture_manifest.to_owned())
    };
    // Optional report settings the builder page persists / exports as JSON: the
    // report *content* copyright notice (distinct from the fixed tooling
    // copyright in the footer) and, in later phases, the front-matter and logo.
    // Parsed leniently via `Value` (no serde-derive dep) so an empty string or
    // an older builder page still works; only `copyright` is consumed today.
    let settings: serde_json::Value = if settings_json.trim().is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_str(settings_json)
            .map_err(|e| JsError::new(&format!("invalid settings JSON: {e}")))?
    };
    let setting = |key: &str| {
        settings
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned()
    };
    let opts = RenderOptions {
        title: title.to_owned(),
        generated_at: generated_at.to_owned(),
        embed_console,
        cert_pems,
        files,
        architecture,
        report_id: report_id.to_owned(),
        copyright: setting("copyright"),
        front_matter: setting("frontMatter"),
        logo: setting("logo"),
    };
    build_report(&sources, &opts).map_err(|e| JsError::new(&e.to_string()))
}

/// Re-run cross-device architecture / topology detection for the builder's GUI
/// editor. `devices_json` is the report model's `devices` array; `manifest` is
/// the topology DSL (device/link/zone/interface/dns-zone/…). Returns the
/// `architecture` object as JSON. Reuses the same detection the report embeds.
#[wasm_bindgen]
pub fn build_architecture(devices_json: &str, manifest: &str) -> Result<String, JsError> {
    let devices: Vec<serde_json::Value> = serde_json::from_str(devices_json)
        .map_err(|e| JsError::new(&format!("invalid devices JSON: {e}")))?;
    let m = if manifest.trim().is_empty() {
        None
    } else {
        Some(manifest)
    };
    let arch = bigip_report_gen_rust::build_architecture(&devices, m);
    serde_json::to_string(&arch).map_err(|e| JsError::new(&e.to_string()))
}

/// The full f5-query manual (grammar + builtins + cookbook) for the builder's
/// reference panel. `topic` is currently ignored (the whole manual is returned).
#[wasm_bindgen]
pub fn manual(_topic: &str) -> String {
    bigip_report_gen_rust::format_manual()
}

/// The report engine version string (for the page's status line).
#[wasm_bindgen]
pub fn engine_version() -> String {
    bigip_report_gen_rust::ENGINE_VERSION.to_string()
}

#[cfg(test)]
mod tests {
    use super::generate_report;

    #[test]
    fn wasm_report_keeps_iapp_diagnostics_in_the_embedded_apps_view() {
        let sources = r##"[
          ["memory://iapp/presentation.apl", "#include \"missing.inc\"\nsection basic {\n  string addr\n  string port\n}\n"],
          ["memory://iapp/implementation.impl", "set ok $::basic__addr\nset missing $::basic__missing\n"]
        ]"##;
        let html = generate_report(sources, "", "", "iApp", "", false, "", "", "")
            .expect("render iApp report");
        for code in ["IAPP7001", "IAPP7002", "IAPP7003"] {
            assert!(html.contains(code), "{code} is retained by the wasm report");
        }
        assert!(html.contains("iApp diagnostic evidence"));
    }
}
