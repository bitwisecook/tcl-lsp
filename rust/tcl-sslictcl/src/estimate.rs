// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Transparent, offline SSL Labs-style estimate and testssl evidence merge.
//!
//! This is deliberately called an estimate: a static configuration cannot
//! prove what a live endpoint negotiates, whether an upstream terminates TLS,
//! or how a particular scanner version scores a new primitive.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::certificate::Certificate;
use crate::chain::{ChainEvaluation, ChainFindingKind, ChainStatus, evaluate_chain};
use crate::model::{Endpoint, ProtocolVersion};
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
    let protocol_score = protocol_score(&input.endpoint.protocols);
    let key_exchange_score = key_exchange_score(input.certificates.first(), input.endpoint);
    let cipher_score = cipher_score(&input.endpoint.ciphers);
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

    apply_protocol_caps(input.endpoint, &mut grade, &mut caps, &mut findings);
    apply_cipher_caps(input.endpoint, &mut grade, &mut caps, &mut findings);
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

fn protocol_score(protocols: &[ProtocolVersion]) -> u8 {
    protocols
        .iter()
        .map(|protocol| match protocol {
            ProtocolVersion::Ssl2 => 0,
            ProtocolVersion::Ssl3 => 20,
            ProtocolVersion::Tls10 => 65,
            ProtocolVersion::Tls11 => 70,
            ProtocolVersion::Tls12 => 95,
            ProtocolVersion::Tls13 => 100,
        })
        .min()
        .unwrap_or(0)
}

fn key_exchange_score(certificate: Option<&Certificate>, endpoint: &Endpoint) -> u8 {
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
            && endpoint.ciphers.iter().all(|cipher| {
                let upper = cipher.to_ascii_uppercase();
                upper.contains("ECDHE") || upper.contains("DHE") || upper.starts_with("TLS_")
            }));
    if has_forward_secrecy {
        base
    } else {
        base.min(80)
    }
}

fn cipher_score(ciphers: &[String]) -> u8 {
    ciphers
        .iter()
        .map(|cipher| match cipher_strength(cipher) {
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
    match grade {
        Grade::Unknown | Grade::T | Grade::M => 0,
        Grade::F => 1,
        Grade::E => 2,
        Grade::D => 3,
        Grade::C => 4,
        Grade::B => 5,
        Grade::A => 6,
        Grade::APlus => 7,
    }
}

fn apply_protocol_caps(
    endpoint: &Endpoint,
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
    grade: &mut Grade,
    caps: &mut Vec<String>,
    findings: &mut Vec<EstimateFinding>,
) {
    for cipher in &endpoint.ciphers {
        let upper = cipher.to_ascii_uppercase();
        if upper.contains("NULL") || upper.contains("EXPORT") || upper.contains("ANON") {
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
            protocol_score(&[ProtocolVersion::Tls12, ProtocolVersion::Tls13]),
            95
        );
        assert_eq!(
            cipher_score(&[
                "TLS_AES_128_GCM_SHA256".to_owned(),
                "TLS_AES_256_GCM_SHA384".to_owned()
            ]),
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
        apply_protocol_caps(&endpoint, &mut grade, &mut caps, &mut findings);
        apply_cipher_caps(&endpoint, &mut grade, &mut caps, &mut findings);
        assert_eq!(grade, Grade::C);
        assert!(findings.len() >= 2);
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
