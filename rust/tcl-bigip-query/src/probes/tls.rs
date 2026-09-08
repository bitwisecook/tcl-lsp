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

//! TLS-handshake probe backend for `tls_handshake` (faithful-but-not-golden).
//!
//! Opens a verifying TLS connection via `rustls` and reports the negotiated
//! `{protocol, cipher, peer_cert, alpn_selected, verify_status, error}` plus a
//! `reason` sub-object.
//!
//! A verification failure is reported (`verify_status` / `reason.kind`) rather
//! than retried with verification disabled; the peer cert is captured when the
//! handshake reached the certificate message, else `null`.

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Once};
use std::time::Duration;

use indexmap::IndexMap;
use rustls::pki_types::ServerName;

use crate::value::Value;

/// Open a TLS connection to `(host, port)` with the given SNI and report the
/// negotiated peer metadata. Returns `Err(message)` only when the TCP / TLS
/// connection couldn't complete (DNS / refused / timeout / fatal protocol
/// error) — the caller turns that into a fatal `connection_error` result.
pub(super) fn handshake(
    host: &str,
    port: i64,
    sni: &str,
    ca_bundle: Option<&str>,
) -> Result<Value, String> {
    let config = client_config(ca_bundle).map_err(|e| e.to_string())?;
    let server_name =
        ServerName::try_from(sni.to_owned()).map_err(|_| format!("invalid SNI host: {sni}"))?;
    let target = format!("{host}:{port}");
    let sa = target
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| format!("cannot resolve {target}"))?;
    let mut sock =
        TcpStream::connect_timeout(&sa, Duration::from_secs(5)).map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    sock.set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let mut conn =
        rustls::ClientConnection::new(Arc::new(config), server_name).map_err(|e| e.to_string())?;

    // Drive the handshake to completion.
    let verify_error = match conn.complete_io(&mut sock) {
        Ok(_) => None,
        Err(e) => {
            // A verification failure still leaves the peer cert captured in the
            // connection state; report it rather than aborting. Anything else
            // (refused / reset / timeout) is a fatal connection error.
            let msg = e.to_string();
            if msg.to_lowercase().contains("certificate")
                || msg.to_lowercase().contains("invalid peer")
                || msg.to_lowercase().contains("unknownissuer")
                || msg.to_lowercase().contains("expired")
            {
                Some(msg)
            } else {
                return Err(msg);
            }
        }
    };

    Ok(build_result(&conn, verify_error))
}

fn build_result(conn: &rustls::ClientConnection, verify_error: Option<String>) -> Value {
    let protocol = conn
        .protocol_version()
        .map(|v| format!("{v:?}"))
        .map_or(Value::Null, Value::Str);
    let cipher = conn
        .negotiated_cipher_suite()
        .map(|cs| format!("{:?}", cs.suite()))
        .map_or(Value::Null, Value::Str);
    let alpn = conn
        .alpn_protocol()
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .map_or(Value::Null, Value::Str);
    let peer_cert = conn
        .peer_certificates()
        .and_then(<[_]>::first)
        .map_or(Value::Null, |der| {
            super::x509_parse_der(der.as_ref()).unwrap_or(Value::Null)
        });

    let mut m: IndexMap<String, Value> = IndexMap::new();
    m.insert("protocol".to_owned(), protocol);
    m.insert("cipher".to_owned(), cipher);
    m.insert("peer_cert".to_owned(), peer_cert);
    m.insert("alpn_selected".to_owned(), alpn);

    let mut reason: IndexMap<String, Value> = IndexMap::new();
    match verify_error {
        None => {
            m.insert("verify_status".to_owned(), Value::Str("ok".to_owned()));
            m.insert("error".to_owned(), Value::Null);
            reason.insert("kind".to_owned(), Value::Str("ok".to_owned()));
            reason.insert("message".to_owned(), Value::Str(String::new()));
            reason.insert("fatal".to_owned(), Value::Bool(false));
        }
        Some(msg) => {
            let kind = classify_verify_kind(&msg);
            m.insert("verify_status".to_owned(), Value::Str(msg.clone()));
            m.insert("error".to_owned(), Value::Null);
            reason.insert("kind".to_owned(), Value::Str(kind));
            reason.insert("message".to_owned(), Value::Str(msg));
            reason.insert("fatal".to_owned(), Value::Bool(false));
        }
    }
    m.insert("reason".to_owned(), Value::Object(reason));
    Value::Object(m)
}

/// Map a rustls verification error message to a `reason.kind` tag, sorting it
/// into the verification-failure buckets (best-effort string match).
fn classify_verify_kind(msg: &str) -> String {
    let lower = msg.to_lowercase();
    if lower.contains("expired") {
        "expired"
    } else if lower.contains("not yet valid") || lower.contains("notvalidyet") {
        "not_yet_valid"
    } else if lower.contains("self") {
        "self_signed"
    } else if lower.contains("unknownissuer") || lower.contains("unknown issuer") {
        "untrusted_ca"
    } else if lower.contains("not valid for name") || lower.contains("hostname") {
        "hostname_mismatch"
    } else {
        "other_verification"
    }
    .to_owned()
}

/// Install the process-level `rustls` crypto provider, once.
///
/// Both providers are linked in — `rustls`'s own default features select
/// `aws-lc-rs` and `ureq` pulls `ring` — so rustls cannot infer one from the
/// feature set, and [`rustls::ClientConfig::builder`] panics with "Could not
/// automatically determine the process-level `CryptoProvider`" until a default
/// is installed. `aws-lc-rs` is the provider this crate's own `rustls`
/// dependency asks for, so that is the one installed.
///
/// The default is process-wide state rustls owns, not configuration of ours:
/// [`Once`] keeps two probes from racing the install, and an already-installed
/// provider — another consumer of the same process got there first — is the
/// wanted outcome, not a failure.
fn install_crypto_provider() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

fn client_config(ca_bundle: Option<&str>) -> Result<rustls::ClientConfig, rustls::Error> {
    install_crypto_provider();
    let mut roots = rustls::RootCertStore::empty();
    if let Some(path) = ca_bundle {
        if let Ok(pem) = std::fs::read(path) {
            for block in x509_parser::pem::Pem::iter_from_buffer(&pem).flatten() {
                if block.label == "CERTIFICATE" {
                    let _ = roots.add(rustls::pki_types::CertificateDer::from(block.contents));
                }
            }
        }
    } else {
        for cert in rustls_native_certs::load_native_certs().certs {
            let _ = roots.add(cert);
        }
    }
    Ok(rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth())
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::thread;

    use super::{classify_verify_kind, client_config, handshake, install_crypto_provider};

    /// Without an installed default provider `ClientConfig::builder()` panics
    /// ("Could not automatically determine the process-level
    /// `CryptoProvider`"), which is what every `tls_handshake` consumer hit in
    /// a debug build. Installing twice must be a no-op, not a second install.
    #[test]
    fn the_crypto_provider_installs_once_and_a_client_config_builds() {
        install_crypto_provider();
        install_crypto_provider();
        assert!(
            client_config(None).is_ok(),
            "a ClientConfig must build once the provider is installed"
        );
    }

    /// The shared entry point, over a listener that accepts and hangs up: the
    /// handshake cannot succeed, but it must *fail*, not panic — which is what
    /// `tls_handshake` did before the provider was installed here. Local-only
    /// and deterministic; no certificate or network is involved.
    #[test]
    fn a_handshake_against_a_closing_listener_reports_an_error_rather_than_panicking() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback listener");
        let port = i64::from(listener.local_addr().expect("local addr").port());
        let accepted = thread::spawn(move || drop(listener.accept()));

        let result = handshake("127.0.0.1", port, "localhost", None);
        accepted.join().expect("listener thread");

        assert!(
            result.is_err(),
            "a peer that hangs up mid-handshake is a connection error, got {result:?}"
        );
    }

    #[test]
    fn classify_verify_kind_buckets_rustls_messages() {
        // Each rustls verification message maps to `reason.kind` tag.
        assert_eq!(
            classify_verify_kind("the certificate has expired"),
            "expired"
        );
        assert_eq!(classify_verify_kind("CertNotValidYet"), "not_yet_valid");
        assert_eq!(
            classify_verify_kind("the certificate is not yet valid"),
            "not_yet_valid"
        );
        assert_eq!(
            classify_verify_kind("self-signed certificate in chain"),
            "self_signed"
        );
        assert_eq!(classify_verify_kind("UnknownIssuer"), "untrusted_ca");
        assert_eq!(
            classify_verify_kind("presented certificate is not valid for name foo.example.com"),
            "hostname_mismatch"
        );
        assert_eq!(
            classify_verify_kind("hostname check failed"),
            "hostname_mismatch"
        );
        assert_eq!(
            classify_verify_kind("decryption failed or bad record mac"),
            "other_verification"
        );
        // Classification is case-insensitive (precedence: expired wins first).
        assert_eq!(classify_verify_kind("EXPIRED"), "expired");
    }
}
