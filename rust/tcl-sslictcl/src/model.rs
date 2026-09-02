// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Product-neutral TLS configuration model used by the DSL and adapters.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::dataset::{ProgramAnchor, TrustProgramSnapshot};
use crate::estimate::{EstimateSeverity, Grade};
use crate::testssl::TestSslImport;
use crate::trust::{ClientFamily, SourceProvenance, TrustPurpose};

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
    /// Certificate declaration identifiers, leaf first. Filled from
    /// [`Self::chain`] by the loader's resolution pass when a chain is named.
    #[serde(default)]
    pub certificate_chain: Vec<String>,
    /// Name of the `chain` declaration this endpoint uses, when it names one.
    /// Mutually exclusive with a literal `certificate-chain` list.
    #[serde(default)]
    pub chain: Option<String>,
    /// Name of the `policy` declaration evaluated against this endpoint.
    #[serde(default)]
    pub policy: Option<String>,
    /// Configured or observed HSTS policy.
    pub hsts: Option<HstsPolicy>,
    /// Source-specific fields retained without teaching this crate the source.
    #[serde(default)]
    pub extensions: BTreeMap<String, Vec<TlsValue>>,
}

/// Catalogue judgement carried by a `protocol` or `cipher` fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TlsStatus {
    /// Preferred for new deployments.
    Recommended,
    /// Permitted, but not preferred.
    Acceptable,
    /// Retained only for compatibility.
    Deprecated,
    /// Must not be offered.
    Prohibited,
}

impl TlsStatus {
    /// The stable `SslicTcl` spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recommended => "recommended",
            Self::Acceptable => "acceptable",
            Self::Deprecated => "deprecated",
            Self::Prohibited => "prohibited",
        }
    }
}

impl fmt::Display for TlsStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TlsStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "recommended" => Ok(Self::Recommended),
            "acceptable" => Ok(Self::Acceptable),
            "deprecated" => Ok(Self::Deprecated),
            "prohibited" => Ok(Self::Prohibited),
            _ => Err(format!("unknown status `{value}`")),
        }
    }
}

/// A declared catalogue fact about one protocol version.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProtocolFact {
    /// Catalogue judgement.
    #[serde(default)]
    pub status: Option<TlsStatus>,
    /// Protocol component score, 0-100, overriding the built-in heuristic.
    #[serde(default)]
    pub score: Option<u8>,
    /// Free-form citation for the judgement.
    #[serde(default)]
    pub reference: Option<String>,
}

/// A declared catalogue fact about one cipher suite.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CipherFact {
    /// IANA registry name.
    #[serde(default)]
    pub iana_name: Option<String>,
    /// `OpenSSL` name.
    #[serde(default)]
    pub openssl_name: Option<String>,
    /// Key-exchange primitive.
    #[serde(default)]
    pub key_exchange: Option<String>,
    /// Authentication primitive.
    #[serde(default)]
    pub authentication: Option<String>,
    /// Bulk-encryption primitive.
    #[serde(default)]
    pub encryption: Option<String>,
    /// Effective symmetric strength, overriding the built-in heuristic.
    #[serde(default)]
    pub bits: Option<u16>,
    /// Whether the suite provides forward secrecy, overriding the name
    /// heuristic.
    #[serde(default)]
    pub forward_secrecy: Option<bool>,
    /// Whether the suite is an AEAD construction.
    #[serde(default)]
    pub aead: Option<bool>,
    /// Catalogue judgement.
    #[serde(default)]
    pub status: Option<TlsStatus>,
    /// Protocol versions the suite may be negotiated on.
    #[serde(default)]
    pub protocols: Vec<ProtocolVersion>,
}

/// The declared protocol and cipher catalogue an estimate may consult before
/// falling back to its built-in heuristics.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TlsFacts {
    /// Facts keyed by protocol version.
    #[serde(default)]
    pub protocols: BTreeMap<ProtocolVersion, ProtocolFact>,
    /// Facts keyed by cipher name, exactly as declared.
    #[serde(default)]
    pub ciphers: BTreeMap<String, CipherFact>,
}

impl TlsFacts {
    /// Whether the catalogue declares nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.protocols.is_empty() && self.ciphers.is_empty()
    }
}

/// A named, ordered certificate chain, leaf first.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChainDeclaration {
    /// Stable document-local identifier.
    pub name: String,
    /// Certificate declaration names, leaf first.
    #[serde(default)]
    pub certificates: Vec<String>,
}

/// One root-program snapshot declared inline in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustProgramDeclaration {
    /// Stable document-local identifier.
    pub name: String,
    /// Root program represented by the declaration.
    pub client: ClientFamily,
    /// Product/root-store version or upstream revision.
    #[serde(default)]
    pub version: String,
    /// ISO-8601 retrieval/generation timestamp.
    #[serde(default)]
    pub generated_at: String,
    /// Stable source label.
    #[serde(default)]
    pub source_name: String,
    /// Source URL; informational only, never fetched while analysing.
    #[serde(default)]
    pub source_url: String,
    /// Upstream version, commit, or snapshot identifier.
    #[serde(default)]
    pub source_revision: String,
    /// SPDX licence expression or a precise data-use label.
    #[serde(default)]
    pub source_license: String,
    /// Anchors keyed by lowercase SHA-256 DER fingerprint.
    #[serde(default)]
    pub anchors: BTreeMap<String, TrustAnchorDeclaration>,
    /// Unrecognised declaration members, preserved for forwards compatibility.
    #[serde(default)]
    pub extensions: BTreeMap<String, Vec<TlsValue>>,
}

impl TrustProgramDeclaration {
    /// Project the declaration onto the normalised snapshot schema the
    /// deterministic trust-store compiler consumes.
    #[must_use]
    pub fn to_snapshot(&self) -> TrustProgramSnapshot {
        TrustProgramSnapshot {
            schema: 1,
            client: self.client,
            version: self.version.clone(),
            generated_at: self.generated_at.clone(),
            source: SourceProvenance {
                name: self.source_name.clone(),
                url: self.source_url.clone(),
                revision: self.source_revision.clone(),
                license: self.source_license.clone(),
            },
            anchors: self
                .anchors
                .values()
                .map(TrustAnchorDeclaration::to_program_anchor)
                .collect(),
        }
    }
}

/// One declared root anchor inside a `trust-program`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TrustAnchorDeclaration {
    /// SHA-256 of the certificate DER, lowercase hexadecimal.
    pub fingerprint_sha256: String,
    /// Display subject.
    #[serde(default)]
    pub subject: String,
    /// Complete certificate DER as base64, when the source publishes it.
    #[serde(default)]
    pub der_base64: Option<String>,
    /// Trust purposes asserted by this root program.
    #[serde(default)]
    pub purposes: Vec<TrustPurpose>,
    /// Included (`true`) or explicitly distrusted (`false`).
    #[serde(default)]
    pub trusted: bool,
    /// Optional policy distrust time as Unix seconds.
    #[serde(default)]
    pub distrust_after: Option<i64>,
}

impl TrustAnchorDeclaration {
    /// Project onto the normalised anchor record.
    #[must_use]
    pub fn to_program_anchor(&self) -> ProgramAnchor {
        ProgramAnchor {
            fingerprint_sha256: self.fingerprint_sha256.clone(),
            subject: self.subject.clone(),
            der_base64: self.der_base64.clone(),
            purposes: self.purposes.clone(),
            trusted: self.trusted,
            distrust_after: self.distrust_after,
        }
    }
}

/// The minimum acceptable estimate grade for a policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GradeRule {
    /// Lowest grade that satisfies the policy.
    pub minimum: Grade,
}

/// One declarative policy check. Every populated member is a conjunct: the
/// check fails when any of them is unsatisfied.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PolicyCheck {
    /// Document-local check identifier.
    pub id: String,
    /// Severity of a finding this check produces.
    #[serde(default)]
    pub severity: Option<EstimateSeverity>,
    /// Message of a finding this check produces.
    #[serde(default)]
    pub message: Option<String>,
    /// Protocol versions that must all be enabled.
    #[serde(default)]
    pub require_protocols: Vec<ProtocolVersion>,
    /// Protocol versions that must not be enabled.
    #[serde(default)]
    pub forbid_protocols: Vec<ProtocolVersion>,
    /// Tcl-style glob patterns no enabled cipher may match.
    #[serde(default)]
    pub forbid_ciphers: Vec<String>,
    /// Whether every enabled cipher must provide forward secrecy.
    #[serde(default)]
    pub require_forward_secrecy: Option<bool>,
    /// Minimum leaf public-key size in bits.
    #[serde(default)]
    pub min_key_bits: Option<u32>,
    /// Whether HSTS must be enabled.
    #[serde(default)]
    pub require_hsts: Option<bool>,
    /// Minimum HSTS `max-age` in seconds.
    #[serde(default)]
    pub min_hsts_max_age: Option<u64>,
    /// Retained verbatim and never evaluated in vocabulary 1.
    #[serde(default)]
    pub predicate: Option<String>,
}

/// A named set of declarative checks plus an optional grade floor.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Policy {
    /// Stable document-local identifier.
    pub name: String,
    /// Checks keyed by check identifier.
    #[serde(default)]
    pub checks: BTreeMap<String, PolicyCheck>,
    /// Optional minimum estimate grade.
    #[serde(default)]
    pub grade: Option<GradeRule>,
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
    /// Named certificate chains, leaf first.
    #[serde(default)]
    pub chains: BTreeMap<String, ChainDeclaration>,
    /// Imported testssl.sh documents, including normalized findings and the
    /// complete source JSON for forward-compatible reprocessing.
    #[serde(default)]
    pub testssl_imports: BTreeMap<String, TestSslImport>,
    /// Declared root-program snapshots.
    #[serde(default)]
    pub trust_programs: BTreeMap<String, TrustProgramDeclaration>,
    /// Declared protocol and cipher catalogue facts.
    #[serde(default)]
    pub facts: TlsFacts,
    /// Named declarative policies.
    #[serde(default)]
    pub policies: BTreeMap<String, Policy>,
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
            chains: BTreeMap::new(),
            testssl_imports: BTreeMap::new(),
            trust_programs: BTreeMap::new(),
            facts: TlsFacts::default(),
            policies: BTreeMap::new(),
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
