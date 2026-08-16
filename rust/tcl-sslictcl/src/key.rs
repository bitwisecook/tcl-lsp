// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline certificate/private-key public-key correlation.

use std::fmt::Write as _;

use rsa::pkcs1::DecodeRsaPrivateKey as _;
use rsa::pkcs8::{DecodePrivateKey as _, EncodePublicKey as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use x509_parser::pem::Pem;

use crate::Certificate;

/// Result of comparing a certificate SPKI with supplied private-key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyMatchStatus {
    /// The private key derives the certificate's exact SPKI.
    Match,
    /// Key material parsed, but derives a different SPKI.
    Mismatch,
    /// Key material was absent, encrypted, or unsupported.
    Unknown,
}

/// Evidence for one certificate/private-key comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyMatch {
    /// Match verdict.
    pub status: KeyMatchStatus,
    /// Derived private-key SPKI SHA-256, never private material.
    pub key_spki_sha256: Option<String>,
    /// Human-readable evidence or limitation.
    pub message: String,
}

impl KeyMatch {
    /// Construct an explicit unknown result.
    #[must_use]
    pub fn unknown(message: impl Into<String>) -> Self {
        Self {
            status: KeyMatchStatus::Unknown,
            key_spki_sha256: None,
            message: message.into(),
        }
    }
}

/// Derive SHA-256 of `SubjectPublicKeyInfo` from an RSA, P-256, or P-384
/// private key in PKCS#8, PKCS#1, or SEC1 DER/PEM form.
pub fn private_key_spki_sha256(input: &[u8]) -> Result<String, String> {
    let marker = b"-----BEGIN";
    let der = if input.windows(marker.len()).any(|window| window == marker) {
        let mut private_key = None;
        for block in Pem::iter_from_buffer(input) {
            let pem = block.map_err(|error| format!("invalid private-key PEM: {error}"))?;
            if pem.label.contains("ENCRYPTED PRIVATE KEY") {
                return Err(
                    "encrypted private key cannot be inspected without its passphrase".to_owned(),
                );
            }
            if matches!(
                pem.label.as_str(),
                "PRIVATE KEY" | "RSA PRIVATE KEY" | "EC PRIVATE KEY"
            ) {
                private_key.get_or_insert(pem.contents);
            }
        }
        private_key.ok_or_else(|| "private-key PEM contains no private-key block".to_owned())?
    } else {
        input.to_vec()
    };
    let spki = rsa_spki(&der)
        .or_else(|| p256_spki(&der))
        .or_else(|| p384_spki(&der))
        .ok_or_else(|| "unsupported or invalid RSA/P-256/P-384 private key".to_owned())?;
    Ok(hex_digest(&spki))
}

/// Compare supplied private-key material with a certificate SPKI.
#[must_use]
pub fn evaluate_private_key_match(certificate: &Certificate, key: &[u8]) -> KeyMatch {
    match private_key_spki_sha256(key) {
        Ok(fingerprint) if fingerprint == certificate.spki_sha256 => KeyMatch {
            status: KeyMatchStatus::Match,
            key_spki_sha256: Some(fingerprint),
            message: "private key derives the certificate's SPKI".to_owned(),
        },
        Ok(fingerprint) => KeyMatch {
            status: KeyMatchStatus::Mismatch,
            key_spki_sha256: Some(fingerprint),
            message: "private key does not match the certificate SPKI".to_owned(),
        },
        Err(message) => KeyMatch::unknown(message),
    }
}

fn rsa_spki(der: &[u8]) -> Option<Vec<u8>> {
    let key = rsa::RsaPrivateKey::from_pkcs8_der(der)
        .or_else(|_| rsa::RsaPrivateKey::from_pkcs1_der(der))
        .ok()?;
    rsa::RsaPublicKey::from(&key)
        .to_public_key_der()
        .ok()
        .map(|document| document.as_bytes().to_vec())
}

fn p256_spki(der: &[u8]) -> Option<Vec<u8>> {
    let key = p256::SecretKey::from_pkcs8_der(der)
        .or_else(|_| p256::SecretKey::from_sec1_der(der))
        .ok()?;
    key.public_key()
        .to_public_key_der()
        .ok()
        .map(|document| document.as_bytes().to_vec())
}

fn p384_spki(der: &[u8]) -> Option<Vec<u8>> {
    let key = p384::SecretKey::from_pkcs8_der(der)
        .or_else(|_| p384::SecretKey::from_sec1_der(der))
        .ok()?;
    key.public_key()
        .to_public_key_der()
        .ok()
        .map(|document| document.as_bytes().to_vec())
}

fn hex_digest(input: &[u8]) -> String {
    let digest = Sha256::digest(input);
    let mut value = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs8::EncodePrivateKey as _;

    #[test]
    fn missing_and_encrypted_keys_are_explicitly_unknown() {
        assert!(private_key_spki_sha256(b"not a key").is_err());
        assert!(
            private_key_spki_sha256(
                b"-----BEGIN ENCRYPTED PRIVATE KEY-----\nAA==\n-----END ENCRYPTED PRIVATE KEY-----\n"
            )
            .is_err()
        );
    }

    #[test]
    fn derives_spki_from_pkcs8_p256_key() {
        let key = p256::SecretKey::from_slice(&[1; 32]).expect("valid fixed scalar");
        let der = key.to_pkcs8_der().expect("PKCS#8 encoding");
        let fingerprint = private_key_spki_sha256(der.as_bytes()).expect("supported key");
        assert_eq!(fingerprint.len(), 64);
        assert!(fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }

    #[test]
    fn finds_key_after_unrelated_pem_block() {
        use base64::Engine as _;

        let key = p256::SecretKey::from_slice(&[1; 32]).expect("valid fixed scalar");
        let der = key.to_pkcs8_der().expect("PKCS#8 encoding");
        let encoded = base64::engine::general_purpose::STANDARD.encode(der.as_bytes());
        let pem = format!(
            "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n\
             -----BEGIN PRIVATE KEY-----\n{encoded}\n-----END PRIVATE KEY-----\n"
        );
        assert!(private_key_spki_sha256(pem.as_bytes()).is_ok());
    }
}
