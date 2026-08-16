// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Product-neutral TLS configuration model used by the DSL and adapters.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::testssl::TestSslImport;

/// A TLS wire protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProtocolVersion {
    /// SSL 2.0.
    #[serde(rename = "ssl2")]
    Ssl2,
    /// SSL 3.0.
    #[serde(rename = "ssl3")]
    Ssl3,
    /// TLS 1.0.
    #[serde(rename = "tls1.0")]
    Tls10,
    /// TLS 1.1.
    #[serde(rename = "tls1.1")]
    Tls11,
    /// TLS 1.2.
    #[serde(rename = "tls1.2")]
    Tls12,
    /// TLS 1.3.
    #[serde(rename = "tls1.3")]
    Tls13,
}

impl ProtocolVersion {
    /// Stable `SslicTcl` spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ssl2 => "ssl2",
            Self::Ssl3 => "ssl3",
            Self::Tls10 => "tls1.0",
            Self::Tls11 => "tls1.1",
            Self::Tls12 => "tls1.2",
            Self::Tls13 => "tls1.3",
        }
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ProtocolVersion {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().replace(['_', '-'], "").as_str() {
            "ssl2" | "sslv2" | "ssl2.0" => Ok(Self::Ssl2),
            "ssl3" | "sslv3" | "ssl3.0" => Ok(Self::Ssl3),
            "tls1" | "tls10" | "tls1.0" | "tlsv1" | "tlsv1.0" => Ok(Self::Tls10),
            "tls11" | "tls1.1" | "tlsv1.1" => Ok(Self::Tls11),
            "tls12" | "tls1.2" | "tlsv1.2" => Ok(Self::Tls12),
            "tls13" | "tls1.3" | "tlsv1.3" => Ok(Self::Tls13),
            _ => Err(format!("unknown TLS protocol `{value}`")),
        }
    }
}

/// A lossless scalar/list/block value retained from a declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TlsValue {
    /// One literal Tcl word.
    Scalar(String),
    /// An ordered Tcl list.
    List(Vec<String>),
    /// Repeated or nested named values.
    Object(BTreeMap<String, Vec<TlsValue>>),
}

/// A certificate declared inline in a `SslicTcl` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateDeclaration {
    /// Stable document-local identifier.
    pub name: String,
    /// PEM or DER-as-hex material. PEM is the normal interchange form.
    pub material: String,
    /// Optional key identifier used to correlate a separately managed key.
    pub key: Option<String>,
    /// Unrecognised declaration fields, preserved for forwards compatibility.
    #[serde(default)]
    pub extensions: BTreeMap<String, Vec<TlsValue>>,
}

/// HTTP Strict Transport Security policy observed or configured on an endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HstsPolicy {
    /// Whether an HSTS header is emitted.
    pub enabled: bool,
    /// `max-age` seconds, when known.
    pub max_age: Option<u64>,
    /// Whether the policy covers subdomains.
    pub include_subdomains: bool,
    /// Whether the policy requests browser preload treatment.
    pub preload: bool,
}

/// Effective TLS-facing configuration of one listener/server/virtual server.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Endpoint {
    /// Stable name supplied by the source adapter.
    pub name: String,
    /// DNS name clients use, if known.
    pub hostname: Option<String>,
    /// Enabled protocol versions.
    #[serde(default)]
    pub protocols: Vec<ProtocolVersion>,
    /// Enabled cipher names in configured preference order.
    #[serde(default)]
    pub ciphers: Vec<String>,
    /// Enabled finite-field or elliptic-curve groups.
    #[serde(default)]
    pub groups: Vec<String>,
    /// Enabled signature schemes.
    #[serde(default)]
    pub signature_schemes: Vec<String>,
    /// Certificate declaration identifiers, leaf first.
    #[serde(default)]
    pub certificate_chain: Vec<String>,
    /// Configured or observed HSTS policy.
    pub hsts: Option<HstsPolicy>,
    /// Source-specific fields retained without teaching this crate the source.
    #[serde(default)]
    pub extensions: BTreeMap<String, Vec<TlsValue>>,
}

/// One fully declarative `SslicTcl` document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SslicModel {
    /// DSL vocabulary major version.
    pub vocabulary: u32,
    /// Named certificates.
    #[serde(default)]
    pub certificates: BTreeMap<String, CertificateDeclaration>,
    /// Named endpoints.
    #[serde(default)]
    pub endpoints: BTreeMap<String, Endpoint>,
    /// Imported testssl.sh documents, including normalized findings and the
    /// complete source JSON for forward-compatible reprocessing.
    #[serde(default)]
    pub testssl_imports: BTreeMap<String, TestSslImport>,
    /// Unknown top-level declarations retained losslessly.
    #[serde(default)]
    pub extensions: BTreeMap<String, Vec<TlsValue>>,
}

impl Default for SslicModel {
    fn default() -> Self {
        Self {
            vocabulary: 1,
            certificates: BTreeMap::new(),
            endpoints: BTreeMap::new(),
            testssl_imports: BTreeMap::new(),
            extensions: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_spellings_are_normalised() {
        assert_eq!("TLSv1.2".parse(), Ok(ProtocolVersion::Tls12));
        assert_eq!("tls1_3".parse(), Ok(ProtocolVersion::Tls13));
        assert!(ProtocolVersion::from_str("quic").is_err());
    }
}
