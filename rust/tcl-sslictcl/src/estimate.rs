// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Transparent, offline SSL Labs-style estimate and testssl evidence merge.
//!
//! This is deliberately called an estimate: a static configuration cannot
//! prove what a live endpoint negotiates, whether an upstream terminates TLS,
//! or how a particular scanner version scores a new primitive.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::certificate::Certificate;
use crate::chain::{ChainEvaluation, ChainFindingKind, ChainStatus, evaluate_chain};
use crate::model::{Endpoint, ProtocolVersion, TlsFacts, TlsStatus};
use crate::testssl::TestSslImport;
use crate::trust::TrustStore;

/// Coarse SSL Labs-compatible presentation grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Grade {
    /// A with qualifying HSTS.
    #[serde(rename = "A+")]
    APlus,
    /// A.
    A,
    /// B.
    B,
    /// C.
    C,
    /// D.
    D,
    /// E.
    E,
    /// F.
    F,
    /// Trust failure.
    T,
    /// Certificate name mismatch.
    M,
    /// Insufficient static evidence.
    Unknown,
}

impl Grade {
    /// Ordinal rank used by cap logic and by declarative grade floors:
    /// `Unknown`/`T`/`M` are 0, `F` is 1, and `A+` is 7.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Unknown | Self::T | Self::M => 0,
            Self::F => 1,
            Self::E => 2,
            Self::D => 3,
            Self::C => 4,
            Self::B => 5,
            Self::A => 6,
            Self::APlus => 7,
        }
    }
}

impl FromStr for Grade {
    type Err = String;

    /// Parses the seven declarable grades. `T`, `M`, and `?` are estimator
    /// outcomes, not policy inputs, so they are not accepted.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_uppercase().as_str() {
            "A+" => Ok(Self::APlus),
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            "E" => Ok(Self::E),
            "F" => Ok(Self::F),
            _ => Err(format!("unknown grade `{value}`")),
        }
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::APlus => "A+",
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::T => "T",
            Self::M => "M",
            Self::Unknown => "?",
        })
    }
}

/// Finding severity independent of editor/report presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstimateSeverity {
    /// Context only.
    Info,
    /// Improvement or uncertainty.
    Warning,
    /// Security-relevant problem.
    Error,
    /// Immediately exploitable/invalid condition.
    Critical,
}

impl EstimateSeverity {
    /// The stable `SslicTcl` spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }
}

impl fmt::Display for EstimateSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EstimateSeverity {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("unknown severity `{value}`")),
        }
    }
}

/// One stable finding from static or imported evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EstimateFinding {
    /// Stable `SslicTcl` identifier.
    pub code: String,
    /// Severity.
    pub severity: EstimateSeverity,
    /// Explanation.
    pub message: String,
    /// Evidence origin (`configuration`, `certificate`, `chain`, `testssl`).
    pub source: String,
}

/// Inputs to one offline estimate.
#[derive(Debug, Clone, Copy)]
pub struct EstimateInput<'a> {
    /// Effective endpoint configuration.
    pub endpoint: &'a Endpoint,
    /// Leaf-first certificate material.
    pub certificates: &'a [Certificate],
    /// Embedded or caller-supplied client trust data.
    pub trust_store: &'a TrustStore,
    /// Optional live testssl evidence to merge.
    pub testssl: Option<&'a TestSslImport>,
    /// Optional declared protocol/cipher catalogue. Declared facts are
    /// consulted before the built-in heuristics.
    pub facts: Option<&'a TlsFacts>,
    /// Evaluation time as Unix seconds.
    pub unix_time: i64,
}

/// Reproducible offline estimate with its component scores and evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Estimate {
    /// Presentation grade after caps.
    pub grade: Grade,
    /// Weighted score before hard caps.
    pub numeric_score: u8,
    /// Protocol component, 0–100.
    pub protocol_score: u8,
    /// Certificate/key-exchange component, 0–100.
    pub key_exchange_score: u8,
    /// Cipher component, 0–100.
    pub cipher_score: u8,
    /// Confidence in the static estimate.
    pub confidence: u8,
    /// Method identifier for report reproducibility.
    pub methodology: String,
    /// Reasons a raw score was capped or replaced.
    pub caps: Vec<String>,
    /// Certificate path result.
    pub chain: ChainEvaluation,
    /// Static and imported findings.
    pub findings: Vec<EstimateFinding>,
}

/// Estimate endpoint posture without network access.
#[must_use]
pub fn estimate(input: EstimateInput<'_>) -> Estimate {
    let chain = evaluate_chain(
        input.certificates,
        input.trust_store,
        input.endpoint.hostname.as_deref(),
        input.unix_time,
    );
    let facts = input.facts;
    let protocol_score = protocol_score(&input.endpoint.protocols, facts);
    let key_exchange_score = key_exchange_score(input.certificates.first(), input.endpoint, facts);
    let cipher_score = cipher_score(&input.endpoint.ciphers, facts);
    let numeric_score = weighted(protocol_score, key_exchange_score, cipher_score);
    let mut grade = grade_for_score(numeric_score);
    let mut caps = Vec::new();
    let mut findings = Vec::new();

    if input.endpoint.protocols.is_empty() {
        grade = Grade::Unknown;
        findings.push(finding(
            "SSLICTL1001",
            EstimateSeverity::Warning,
            "no effective TLS protocol set is known",
            "configuration",
        ));
    }
    if input.endpoint.ciphers.is_empty() {
        grade = Grade::Unknown;
        findings.push(finding(
            "SSLICTL1002",
            EstimateSeverity::Warning,
            "no effective cipher set is known",
            "configuration",
        ));
    }

    apply_protocol_caps(input.endpoint, facts, &mut grade, &mut caps, &mut findings);
    apply_cipher_caps(input.endpoint, facts, &mut grade, &mut caps, &mut findings);
    apply_certificate_caps(
        input.certificates.first(),
        &mut grade,
        &mut caps,
        &mut findings,
    );
    apply_chain_result(&chain, &mut grade, &mut caps, &mut findings);
    if let Some(testssl) = input.testssl {
        apply_testssl(testssl, &mut grade, &mut caps, &mut findings);
    }

    if grade == Grade::A
        && input.endpoint.hsts.as_ref().is_some_and(|hsts| {
            hsts.enabled && hsts.max_age.is_some_and(|seconds| seconds >= 15_552_000)
        })
    {
        grade = Grade::APlus;
    } else if input
        .endpoint
        .hsts
        .as_ref()
        .is_none_or(|hsts| !hsts.enabled)
    {
        findings.push(finding(
            "SSLICTL1010",
            EstimateSeverity::Info,
            "HSTS is absent; an A+ estimate requires at least 180 days",
            "configuration",
        ));
    }

    let known_surfaces = u8::from(!input.endpoint.protocols.is_empty())
        + u8::from(!input.endpoint.ciphers.is_empty())
        + u8::from(!input.certificates.is_empty())
        + u8::from(input.testssl.is_some());
    let confidence = [20, 45, 70, 85, 100][usize::from(known_surfaces)];

    Estimate {
        grade,
        numeric_score,
        protocol_score,
        key_exchange_score,
        cipher_score,
        confidence,
        methodology: "sslictcl-offline-estimate-v1".to_owned(),
        caps,
        chain,
        findings,
    }
}

fn protocol_score(protocols: &[ProtocolVersion], facts: Option<&TlsFacts>) -> u8 {
    protocols
        .iter()
        .map(|protocol| declared_protocol_score(*protocol, facts))
        .min()
        .unwrap_or(0)
}

/// A declared `protocol … score N` overrides the built-in judgement for that
/// version; everything else keeps the heuristic.
fn declared_protocol_score(protocol: ProtocolVersion, facts: Option<&TlsFacts>) -> u8 {
    if let Some(score) = facts
        .and_then(|facts| facts.protocols.get(&protocol))
        .and_then(|fact| fact.score)
    {
        return score.min(100);
    }
    match protocol {
        ProtocolVersion::Ssl2 => 0,
        ProtocolVersion::Ssl3 => 20,
        ProtocolVersion::Tls10 => 65,
        ProtocolVersion::Tls11 => 70,
        ProtocolVersion::Tls12 => 95,
        ProtocolVersion::Tls13 => 100,
    }
}

/// Whether the declared catalogue prohibits `protocol`.
fn protocol_is_prohibited(protocol: ProtocolVersion, facts: Option<&TlsFacts>) -> bool {
    facts
        .and_then(|facts| facts.protocols.get(&protocol))
        .and_then(|fact| fact.status)
        == Some(TlsStatus::Prohibited)
}

/// Whether the declared catalogue prohibits `cipher`.
fn cipher_is_prohibited(cipher: &str, facts: Option<&TlsFacts>) -> bool {
    facts
        .and_then(|facts| facts.ciphers.get(cipher))
        .and_then(|fact| fact.status)
        == Some(TlsStatus::Prohibited)
}

/// Whether `cipher` provides forward secrecy: a declared `forward-secrecy`
/// fact wins, otherwise the suite-name heuristic decides.
#[must_use]
pub fn cipher_has_forward_secrecy(cipher: &str, facts: Option<&TlsFacts>) -> bool {
    if let Some(declared) = facts
        .and_then(|facts| facts.ciphers.get(cipher))
        .and_then(|fact| fact.forward_secrecy)
    {
        return declared;
    }
    let upper = cipher.to_ascii_uppercase();
    upper.contains("ECDHE") || upper.contains("DHE") || upper.starts_with("TLS_")
}

fn key_exchange_score(
    certificate: Option<&Certificate>,
    endpoint: &Endpoint,
    facts: Option<&TlsFacts>,
) -> u8 {
    let Some(certificate) = certificate else {
        return 0;
    };
    let base = match (
        certificate.public_key_algorithm.as_str(),
        certificate.public_key_bits,
    ) {
        ("1.2.840.113549.1.1.1", Some(bits)) if bits >= 4096 => 100,
        ("1.2.840.113549.1.1.1", Some(bits)) if bits >= 2048 => 90,
        ("1.2.840.113549.1.1.1", Some(bits)) if bits >= 1024 => 60,
        ("1.2.840.10045.2.1", Some(bits)) if bits >= 384 => 100,
        ("1.2.840.10045.2.1", Some(bits)) if bits >= 256 => 95,
        ("1.3.101.112" | "1.3.101.113", _) => 100,
        (_, Some(bits)) if bits < 1024 => 20,
        _ => 70,
    };
    let has_forward_secrecy = endpoint.protocols.contains(&ProtocolVersion::Tls13)
        || (!endpoint.ciphers.is_empty()
            && endpoint
                .ciphers
                .iter()
                .all(|cipher| cipher_has_forward_secrecy(cipher, facts)));
    if has_forward_secrecy {
        base
    } else {
        base.min(80)
    }
}

fn cipher_score(ciphers: &[String], facts: Option<&TlsFacts>) -> u8 {
    ciphers
        .iter()
        .map(|cipher| match declared_cipher_strength(cipher, facts) {
            256.. => 100,
            128..=255 => 90,
            112..=127 => 70,
            80..=111 => 60,
            56..=79 => 40,
            1..=55 => 20,
            0 => 0,
        })
        .min()
        .unwrap_or(0)
}

/// A declared `cipher … bits N` overrides [`cipher_strength`] for that suite.
fn declared_cipher_strength(cipher: &str, facts: Option<&TlsFacts>) -> u16 {
    facts
        .and_then(|facts| facts.ciphers.get(cipher))
        .and_then(|fact| fact.bits)
        .unwrap_or_else(|| cipher_strength(cipher))
}

fn cipher_strength(cipher: &str) -> u16 {
    let upper = cipher.to_ascii_uppercase();
    if upper.contains("NULL") || upper.contains("EXPORT") || upper.contains("ANON") {
        0
    } else if upper.contains("CHACHA20") || upper.contains("AES256") || upper.contains("AES_256") {
        256
    } else if upper.contains("3DES") || upper.contains("DES-CBC3") {
        112
    } else if upper.contains("DES") {
        56
    } else if upper.contains("RC4") || upper.contains("AES128") || upper.contains("AES_128") {
        128
    } else {
        // Unknown does not mean weak, but it lowers confidence through the
        // conservative component rather than being silently rated at 256.
        128
    }
}

fn weighted(protocol: u8, key_exchange: u8, cipher: u8) -> u8 {
    let score =
        (u16::from(protocol) * 30 + u16::from(key_exchange) * 30 + u16::from(cipher) * 40) / 100;
    u8::try_from(score).unwrap_or(100)
}

const fn grade_for_score(score: u8) -> Grade {
    match score {
        80.. => Grade::A,
        65..=79 => Grade::B,
        50..=64 => Grade::C,
        35..=49 => Grade::D,
        20..=34 => Grade::E,
        _ => Grade::F,
    }
}

fn cap(grade: &mut Grade, maximum: Grade, reason: &str, caps: &mut Vec<String>) {
    if grade_rank(*grade) > grade_rank(maximum) {
        *grade = maximum;
    }
    caps.push(reason.to_owned());
}

const fn grade_rank(grade: Grade) -> u8 {
    grade.rank()
}

fn apply_protocol_caps(
    endpoint: &Endpoint,
    facts: Option<&TlsFacts>,
    grade: &mut Grade,
    caps: &mut Vec<String>,
    findings: &mut Vec<EstimateFinding>,
) {
    if endpoint.protocols.contains(&ProtocolVersion::Ssl2) {
        cap(grade, Grade::F, "SSL 2.0 is enabled", caps);
        findings.push(finding(
            "SSLICTL1101",
            EstimateSeverity::Critical,
            "SSL 2.0 is enabled",
            "configuration",
        ));
    } else if endpoint.protocols.contains(&ProtocolVersion::Ssl3) {
        cap(grade, Grade::C, "SSL 3.0 is enabled", caps);
        findings.push(finding(
            "SSLICTL1102",
            EstimateSeverity::Error,
            "SSL 3.0 is enabled",
            "configuration",
        ));
    }
    for protocol in endpoint
        .protocols
        .iter()
        .filter(|protocol| protocol_is_prohibited(**protocol, facts))
    {
        cap(
            grade,
            Grade::F,
            "a declared-prohibited protocol is enabled",
            caps,
        );
        findings.push(finding(
            "SSLICTL1104",
            EstimateSeverity::Critical,
            &format!("prohibited protocol `{protocol}` is enabled"),
            "configuration",
        ));
    }
    if endpoint
        .protocols
        .iter()
        .any(|protocol| matches!(protocol, ProtocolVersion::Tls10 | ProtocolVersion::Tls11))
    {
        cap(grade, Grade::B, "TLS 1.0 or TLS 1.1 is enabled", caps);
        findings.push(finding(
            "SSLICTL1103",
            EstimateSeverity::Warning,
            "obsolete TLS 1.0/1.1 remains enabled",
            "configuration",
        ));
    }
}

fn apply_cipher_caps(
    endpoint: &Endpoint,
    facts: Option<&TlsFacts>,
    grade: &mut Grade,
    caps: &mut Vec<String>,
    findings: &mut Vec<EstimateFinding>,
) {
    for cipher in &endpoint.ciphers {
        let upper = cipher.to_ascii_uppercase();
        if cipher_is_prohibited(cipher, facts) {
            cap(
                grade,
                Grade::F,
                "a declared-prohibited cipher is enabled",
                caps,
            );
            findings.push(finding(
                "SSLICTL1204",
                EstimateSeverity::Critical,
                &format!("prohibited cipher `{cipher}` is enabled"),
                "configuration",
            ));
        } else if upper.contains("NULL") || upper.contains("EXPORT") || upper.contains("ANON") {
            cap(
                grade,
                Grade::F,
                "null, export, or anonymous cipher is enabled",
                caps,
            );
            findings.push(finding(
                "SSLICTL1201",
                EstimateSeverity::Critical,
                &format!("prohibited cipher `{cipher}` is enabled"),
                "configuration",
            ));
        } else if upper.contains("RC4") {
            cap(grade, Grade::B, "RC4 is enabled", caps);
            findings.push(finding(
                "SSLICTL1202",
                EstimateSeverity::Error,
                &format!("RC4 cipher `{cipher}` is enabled"),
                "configuration",
            ));
        } else if upper.contains("3DES") || upper.contains("DES-CBC3") {
            cap(grade, Grade::B, "3DES is enabled", caps);
            findings.push(finding(
                "SSLICTL1203",
                EstimateSeverity::Warning,
                &format!("3DES cipher `{cipher}` is enabled"),
                "configuration",
            ));
        }
    }
}

fn apply_certificate_caps(
    certificate: Option<&Certificate>,
    grade: &mut Grade,
    caps: &mut Vec<String>,
    findings: &mut Vec<EstimateFinding>,
) {
    let Some(certificate) = certificate else {
        *grade = Grade::T;
        caps.push("no leaf certificate is available".to_owned());
        return;
    };
    if certificate.signature_algorithm == "1.2.840.113549.1.1.4" {
        cap(grade, Grade::F, "certificate uses an MD5 signature", caps);
        findings.push(finding(
            "SSLICTL1301",
            EstimateSeverity::Critical,
            "certificate uses an MD5 signature",
            "certificate",
        ));
    } else if matches!(
        certificate.signature_algorithm.as_str(),
        "1.2.840.113549.1.1.5" | "1.2.840.10045.4.1" | "1.2.840.10040.4.3"
    ) {
        cap(grade, Grade::B, "certificate uses a SHA-1 signature", caps);
        findings.push(finding(
            "SSLICTL1302",
            EstimateSeverity::Error,
            "certificate uses a SHA-1 signature",
            "certificate",
        ));
    }
    if certificate.public_key_bits.is_some_and(|bits| {
        certificate.public_key_algorithm == "1.2.840.113549.1.1.1" && bits < 2048
    }) {
        cap(
            grade,
            Grade::B,
            "RSA certificate key is smaller than 2048 bits",
            caps,
        );
        findings.push(finding(
            "SSLICTL1303",
            EstimateSeverity::Error,
            "RSA certificate key is smaller than 2048 bits",
            "certificate",
        ));
    }
}

fn apply_chain_result(
    chain: &ChainEvaluation,
    grade: &mut Grade,
    caps: &mut Vec<String>,
    findings: &mut Vec<EstimateFinding>,
) {
    if chain
        .findings
        .iter()
        .any(|finding| finding.kind == ChainFindingKind::HostnameMismatch)
    {
        *grade = Grade::M;
        caps.push("certificate name mismatch".to_owned());
    } else if chain.status != ChainStatus::Valid {
        *grade = Grade::T;
        caps.push(format!("certificate path status is {:?}", chain.status));
    }
    findings.extend(chain.findings.iter().map(|chain_finding| EstimateFinding {
        code: format!("SSLICTL-CHAIN-{:?}", chain_finding.kind).to_ascii_uppercase(),
        severity: if matches!(
            chain_finding.kind,
            ChainFindingKind::IssuerNotFound
                | ChainFindingKind::AmbiguousIssuer
                | ChainFindingKind::TrustUnknown
        ) {
            EstimateSeverity::Warning
        } else {
            EstimateSeverity::Error
        },
        message: chain_finding.message.clone(),
        source: "chain".to_owned(),
    }));
}

fn apply_testssl(
    imported: &TestSslImport,
    _grade: &mut Grade,
    _caps: &mut Vec<String>,
    findings: &mut Vec<EstimateFinding>,
) {
    for result in &imported.findings {
        let severity = result
            .severity
            .as_deref()
            .unwrap_or("")
            .to_ascii_uppercase();
        let estimate_severity = match severity.as_str() {
            "CRITICAL" => EstimateSeverity::Critical,
            "HIGH" => EstimateSeverity::Error,
            "MEDIUM" | "WARN" | "WARNING" => EstimateSeverity::Warning,
            _ => continue,
        };
        // Scanner severity is imported evidence, not a versioned SSL Labs
        // scoring rule. Only explicit check-ID mappings may alter a grade.
        findings.push(EstimateFinding {
            code: format!("TESTSSL-{}", result.id),
            severity: estimate_severity,
            message: result
                .finding
                .clone()
                .unwrap_or_else(|| format!("testssl severity {severity}")),
            source: "testssl".to_owned(),
        });
    }
}

fn finding(code: &str, severity: EstimateSeverity, message: &str, source: &str) -> EstimateFinding {
    EstimateFinding {
        code: code.to_owned(),
        severity,
        message: message.to_owned(),
        source: source.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_cipher_and_protocol_components_are_high() {
        assert_eq!(
            protocol_score(&[ProtocolVersion::Tls12, ProtocolVersion::Tls13], None),
            95
        );
        assert_eq!(
            cipher_score(
                &[
                    "TLS_AES_128_GCM_SHA256".to_owned(),
                    "TLS_AES_256_GCM_SHA384".to_owned()
                ],
                None
            ),
            90
        );
    }

    #[test]
    fn legacy_protocol_and_ciphers_cap_grade() {
        let endpoint = Endpoint {
            protocols: vec![ProtocolVersion::Ssl3, ProtocolVersion::Tls12],
            ciphers: vec!["RC4-SHA".to_owned()],
            ..Endpoint::default()
        };
        let mut grade = Grade::A;
        let mut caps = Vec::new();
        let mut findings = Vec::new();
        apply_protocol_caps(&endpoint, None, &mut grade, &mut caps, &mut findings);
        apply_cipher_caps(&endpoint, None, &mut grade, &mut caps, &mut findings);
        assert_eq!(grade, Grade::C);
        assert!(findings.len() >= 2);
    }

    fn facts_with_protocol(version: ProtocolVersion, fact: crate::model::ProtocolFact) -> TlsFacts {
        let mut facts = TlsFacts::default();
        facts.protocols.insert(version, fact);
        facts
    }

    fn facts_with_cipher(name: &str, fact: crate::model::CipherFact) -> TlsFacts {
        let mut facts = TlsFacts::default();
        facts.ciphers.insert(name.to_owned(), fact);
        facts
    }

    #[test]
    fn declared_protocol_score_overrides_the_built_in_judgement() {
        assert_eq!(protocol_score(&[ProtocolVersion::Tls10], None), 65);
        let facts = facts_with_protocol(
            ProtocolVersion::Tls10,
            crate::model::ProtocolFact {
                score: Some(20),
                ..crate::model::ProtocolFact::default()
            },
        );
        assert_eq!(protocol_score(&[ProtocolVersion::Tls10], Some(&facts)), 20);
    }

    #[test]
    fn declared_cipher_bits_override_the_strength_heuristic() {
        let ciphers = ["MYSTERY-SUITE".to_owned()];
        assert_eq!(cipher_score(&ciphers, None), 90);
        let facts = facts_with_cipher(
            "MYSTERY-SUITE",
            crate::model::CipherFact {
                bits: Some(40),
                ..crate::model::CipherFact::default()
            },
        );
        assert_eq!(cipher_score(&ciphers, Some(&facts)), 20);
    }

    #[test]
    fn declared_prohibited_protocol_caps_the_grade() {
        let endpoint = Endpoint {
            protocols: vec![ProtocolVersion::Tls12],
            ..Endpoint::default()
        };
        let facts = facts_with_protocol(
            ProtocolVersion::Tls12,
            crate::model::ProtocolFact {
                status: Some(TlsStatus::Prohibited),
                ..crate::model::ProtocolFact::default()
            },
        );
        let mut grade = Grade::A;
        let mut caps = Vec::new();
        let mut findings = Vec::new();
        apply_protocol_caps(
            &endpoint,
            Some(&facts),
            &mut grade,
            &mut caps,
            &mut findings,
        );
        assert_eq!(grade, Grade::F);
        assert_eq!(findings[0].code, "SSLICTL1104");
    }

    #[test]
    fn declared_prohibited_cipher_caps_the_grade() {
        let endpoint = Endpoint {
            ciphers: vec!["ECDHE-RSA-AES128-GCM-SHA256".to_owned()],
            ..Endpoint::default()
        };
        let facts = facts_with_cipher(
            "ECDHE-RSA-AES128-GCM-SHA256",
            crate::model::CipherFact {
                status: Some(TlsStatus::Prohibited),
                ..crate::model::CipherFact::default()
            },
        );
        let mut grade = Grade::A;
        let mut caps = Vec::new();
        let mut findings = Vec::new();
        apply_cipher_caps(
            &endpoint,
            Some(&facts),
            &mut grade,
            &mut caps,
            &mut findings,
        );
        assert_eq!(grade, Grade::F);
        assert_eq!(findings[0].code, "SSLICTL1204");
    }

    #[test]
    fn declared_forward_secrecy_replaces_the_name_heuristic() {
        assert!(!cipher_has_forward_secrecy("AES128-SHA", None));
        let facts = facts_with_cipher(
            "AES128-SHA",
            crate::model::CipherFact {
                forward_secrecy: Some(true),
                ..crate::model::CipherFact::default()
            },
        );
        assert!(cipher_has_forward_secrecy("AES128-SHA", Some(&facts)));
        let denied = facts_with_cipher(
            "ECDHE-RSA-AES128-SHA",
            crate::model::CipherFact {
                forward_secrecy: Some(false),
                ..crate::model::CipherFact::default()
            },
        );
        assert!(!cipher_has_forward_secrecy(
            "ECDHE-RSA-AES128-SHA",
            Some(&denied)
        ));
    }

    #[test]
    fn declarable_grades_round_trip_and_estimator_only_grades_do_not() {
        for grade in [
            Grade::APlus,
            Grade::A,
            Grade::B,
            Grade::C,
            Grade::D,
            Grade::E,
            Grade::F,
        ] {
            assert_eq!(grade.to_string().parse(), Ok(grade));
        }
        assert!(Grade::from_str("T").is_err());
        assert!(Grade::from_str("?").is_err());
        assert!(Grade::APlus.rank() > Grade::A.rank());
        assert_eq!("critical".parse(), Ok(EstimateSeverity::Critical));
        assert!(EstimateSeverity::from_str("nope").is_err());
    }

    #[test]
    fn scanner_severity_is_evidence_not_a_grade_cap() {
        let imported = crate::testssl::import_testssl_json(
            r#"[{"id":"HEARTBLEED","severity":"CRITICAL","finding":"vulnerable"}]"#,
        )
        .unwrap();
        let mut grade = Grade::A;
        let mut caps = Vec::new();
        let mut findings = Vec::new();
        apply_testssl(&imported, &mut grade, &mut caps, &mut findings);
        assert_eq!(grade, Grade::A);
        assert!(caps.is_empty());
        assert_eq!(findings[0].code, "TESTSSL-HEARTBLEED");
    }
}
