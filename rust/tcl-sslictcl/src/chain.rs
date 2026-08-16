// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Certificate path construction and offline RFC 5280-oriented checks.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::certificate::{Certificate, normalise_name, signed_by};
use crate::trust::{ClientFamily, TrustDecision, TrustPurpose, TrustStore};

const MAX_PATH_CERTIFICATES: usize = 16;
const CLIENTS: [ClientFamily; 6] = [
    ClientFamily::Mozilla,
    ClientFamily::Chrome,
    ClientFamily::Apple,
    ClientFamily::Microsoft,
    ClientFamily::Android,
    ClientFamily::OpenJdk,
];

type CandidateEvaluation = (
    Vec<usize>,
    Vec<ChainFinding>,
    BTreeMap<ClientFamily, TrustDecision>,
);

/// Overall state of the selected certificate path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChainStatus {
    /// Path is structurally and cryptographically valid and trusted by at
    /// least one represented client program.
    Valid,
    /// A required issuer could not be found.
    Incomplete,
    /// A path was built but no represented client trusts its terminus.
    Untrusted,
    /// A certificate, signature, usage, constraint, time, or name check failed.
    Invalid,
}

/// Stable machine-readable certificate-path finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChainFindingKind {
    /// Certificate is not yet valid.
    NotYetValid,
    /// Certificate has expired.
    Expired,
    /// Leaf SAN does not cover the requested hostname.
    HostnameMismatch,
    /// Leaf EKU excludes TLS server authentication.
    LeafNotServerAuth,
    /// A critical extension could not be interpreted.
    UnknownCriticalExtension,
    /// No issuer certificate was supplied.
    IssuerNotFound,
    /// Multiple issuer paths were possible.
    AmbiguousIssuer,
    /// Issuer lacks Basic Constraints CA=true.
    IssuerNotCa,
    /// Issuer Key Usage excludes certificate signing.
    IssuerCannotSign,
    /// Certificate signature does not verify under the issuer key.
    InvalidSignature,
    /// Basic Constraints path length was exceeded.
    PathLengthExceeded,
    /// A DNS name violates an issuer name constraint.
    NameConstraintViolation,
    /// A critical name-constraint form is not implemented by this validator.
    UnsupportedNameConstraint,
    /// Issuer graph loops or exceeds the defensive depth bound.
    PathLoop,
    /// A represented program explicitly distrusts the terminus.
    ExplicitlyDistrusted,
    /// No represented program has data for the terminus.
    TrustUnknown,
}

/// One chain finding, tied to a certificate fingerprint where applicable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainFinding {
    /// Stable kind.
    pub kind: ChainFindingKind,
    /// Affected certificate SHA-256 fingerprint.
    pub certificate: Option<String>,
    /// Human-readable detail.
    pub message: String,
}

/// Full deterministic result. The selected path and alternate candidate paths
/// contain SHA-256 fingerprints, leaf first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainEvaluation {
    /// Overall state.
    pub status: ChainStatus,
    /// Best path selected by validity and trust, leaf first.
    pub path: Vec<String>,
    /// Other structurally possible paths, leaf first.
    pub alternate_paths: Vec<Vec<String>>,
    /// Per-client trust result for the selected terminus.
    pub trust: BTreeMap<ClientFamily, TrustDecision>,
    /// Findings for the selected path.
    pub findings: Vec<ChainFinding>,
}

/// Construct and evaluate certificate paths from a leaf and unordered pool.
///
/// `certificates[0]` is the leaf. Remaining entries may contain intermediates,
/// roots, duplicates, and unrelated certificates.
#[must_use]
pub fn evaluate_chain(
    certificates: &[Certificate],
    trust_store: &TrustStore,
    hostname: Option<&str>,
    unix_time: i64,
) -> ChainEvaluation {
    if certificates.is_empty() {
        return ChainEvaluation {
            status: ChainStatus::Incomplete,
            path: Vec::new(),
            alternate_paths: Vec::new(),
            trust: unknown_trust(),
            findings: vec![ChainFinding {
                kind: ChainFindingKind::IssuerNotFound,
                certificate: None,
                message: "no leaf certificate was supplied".to_owned(),
            }],
        };
    }

    let mut certificate_pool = certificates.to_vec();
    if let Ok(anchors) = trust_store.anchor_certificates() {
        for anchor in anchors {
            if !certificate_pool
                .iter()
                .any(|certificate| certificate.fingerprint_sha256 == anchor.fingerprint_sha256)
            {
                certificate_pool.push(anchor);
            }
        }
    }
    evaluate_chain_pool(&certificate_pool, trust_store, hostname, unix_time)
}

// Path selection, trust projection, and status derivation are intentionally
// kept together so the ordering policy cannot drift between helpers.
#[allow(clippy::too_many_lines)]
fn evaluate_chain_pool(
    certificates: &[Certificate],
    trust_store: &TrustStore,
    hostname: Option<&str>,
    unix_time: i64,
) -> ChainEvaluation {
    let mut paths = Vec::new();
    let mut incomplete = Vec::new();
    walk_paths(
        certificates,
        trust_store,
        vec![0],
        &mut paths,
        &mut incomplete,
    );
    if paths.is_empty() {
        paths.push(incomplete.first().cloned().unwrap_or_else(|| vec![0]));
    }

    let mut evaluated: Vec<CandidateEvaluation> = paths
        .iter()
        .map(|path| {
            let findings = evaluate_path(certificates, path, trust_store, hostname, unix_time);
            let trust = trust_for_terminal(certificates, path, trust_store, unix_time);
            (path.clone(), findings, trust)
        })
        .collect();
    evaluated.sort_by_key(|(_, findings, trust)| {
        let invalid = findings
            .iter()
            .filter(|finding| is_invalid(finding.kind))
            .count();
        let trusted = trust
            .values()
            .filter(|decision| **decision == TrustDecision::Trusted)
            .count();
        (invalid, usize::MAX - trusted)
    });

    let (selected, mut findings, trust) = evaluated.remove(0);
    if paths.len() > 1 {
        findings.push(ChainFinding {
            kind: ChainFindingKind::AmbiguousIssuer,
            certificate: certificates
                .get(1)
                .map(|cert| cert.fingerprint_sha256.clone()),
            message: format!(
                "{} structurally possible certificate paths found",
                paths.len()
            ),
        });
    }
    if incomplete.iter().any(|path| path == &selected) {
        let terminal = selected.last().and_then(|index| certificates.get(*index));
        findings.push(ChainFinding {
            kind: ChainFindingKind::IssuerNotFound,
            certificate: terminal.map(|cert| cert.fingerprint_sha256.clone()),
            message: terminal.map_or_else(
                || "issuer certificate is missing".to_owned(),
                |cert| {
                    format!(
                        "issuer `{}` for `{}` was not supplied",
                        cert.issuer, cert.subject
                    )
                },
            ),
        });
    }
    let trusted = trust
        .values()
        .any(|decision| *decision == TrustDecision::Trusted);
    let distrusted = trust
        .values()
        .any(|decision| *decision == TrustDecision::Distrusted);
    if !trusted {
        findings.push(ChainFinding {
            kind: if distrusted {
                ChainFindingKind::ExplicitlyDistrusted
            } else {
                ChainFindingKind::TrustUnknown
            },
            certificate: selected
                .last()
                .and_then(|index| certificates.get(*index))
                .map(|cert| cert.fingerprint_sha256.clone()),
            message: if distrusted {
                "certificate path terminus is explicitly distrusted by represented clients"
                    .to_owned()
            } else {
                "certificate path terminus is absent from the embedded trust snapshot".to_owned()
            },
        });
    }
    findings.sort_by_key(|finding| finding.kind);
    findings
        .dedup_by(|left, right| left.kind == right.kind && left.certificate == right.certificate);

    let status = if findings.iter().any(|finding| is_invalid(finding.kind)) {
        ChainStatus::Invalid
    } else if findings
        .iter()
        .any(|finding| finding.kind == ChainFindingKind::IssuerNotFound)
    {
        ChainStatus::Incomplete
    } else if trusted {
        ChainStatus::Valid
    } else {
        ChainStatus::Untrusted
    };

    ChainEvaluation {
        status,
        path: fingerprint_path(certificates, &selected),
        alternate_paths: evaluated
            .iter()
            .map(|(path, _, _)| fingerprint_path(certificates, path))
            .collect(),
        trust,
        findings,
    }
}

fn walk_paths(
    certificates: &[Certificate],
    trust_store: &TrustStore,
    path: Vec<usize>,
    complete: &mut Vec<Vec<usize>>,
    incomplete: &mut Vec<Vec<usize>>,
) {
    let Some(&current_index) = path.last() else {
        return;
    };
    let current = &certificates[current_index];
    if trust_store.anchor(&current.fingerprint_sha256).is_some() || current.is_self_issued() {
        complete.push(path);
        return;
    }
    if path.len() >= MAX_PATH_CERTIFICATES {
        incomplete.push(path);
        return;
    }

    let used: BTreeSet<usize> = path.iter().copied().collect();
    let mut candidates: Vec<usize> = certificates
        .iter()
        .enumerate()
        .filter(|(index, candidate)| {
            !used.contains(index)
                && normalise_name(&candidate.subject) == normalise_name(&current.issuer)
        })
        .map(|(index, _)| index)
        .collect();
    if let Some(authority_key_id) = &current.authority_key_id {
        let key_matches: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|index| certificates[*index].subject_key_id.as_ref() == Some(authority_key_id))
            .collect();
        if !key_matches.is_empty() {
            candidates = key_matches;
        }
    }
    if candidates.is_empty() {
        incomplete.push(path);
        return;
    }
    for candidate in candidates {
        let mut next = path.clone();
        next.push(candidate);
        walk_paths(certificates, trust_store, next, complete, incomplete);
    }
}

#[allow(clippy::too_many_lines)]
fn evaluate_path(
    certificates: &[Certificate],
    path: &[usize],
    trust_store: &TrustStore,
    hostname: Option<&str>,
    unix_time: i64,
) -> Vec<ChainFinding> {
    let mut findings = Vec::new();
    let leaf = &certificates[path[0]];
    if let Some(hostname) = hostname
        && !leaf.covers_hostname(hostname)
    {
        finding(
            &mut findings,
            ChainFindingKind::HostnameMismatch,
            leaf,
            format!("leaf certificate SAN does not cover `{hostname}`"),
        );
    }
    if !leaf.permits_server_auth() {
        finding(
            &mut findings,
            ChainFindingKind::LeafNotServerAuth,
            leaf,
            "leaf certificate EKU excludes server authentication".to_owned(),
        );
    }

    for (position, &index) in path.iter().enumerate() {
        let cert = &certificates[index];
        let is_trust_anchor =
            position + 1 == path.len() && trust_store.anchor(&cert.fingerprint_sha256).is_some();
        if !is_trust_anchor && unix_time < cert.not_before {
            finding(
                &mut findings,
                ChainFindingKind::NotYetValid,
                cert,
                "certificate is not yet valid".to_owned(),
            );
        } else if !is_trust_anchor && unix_time > cert.not_after {
            finding(
                &mut findings,
                ChainFindingKind::Expired,
                cert,
                "certificate has expired".to_owned(),
            );
        }
        if !cert.unknown_critical_extensions.is_empty() {
            finding(
                &mut findings,
                ChainFindingKind::UnknownCriticalExtension,
                cert,
                format!(
                    "unrecognised critical extension(s): {}",
                    cert.unknown_critical_extensions.join(", ")
                ),
            );
        }
    }

    for pair in path.windows(2) {
        let child = &certificates[pair[0]];
        let issuer = &certificates[pair[1]];
        if !issuer.is_ca {
            finding(
                &mut findings,
                ChainFindingKind::IssuerNotCa,
                issuer,
                "issuer certificate lacks Basic Constraints CA=true".to_owned(),
            );
        }
        if !issuer.permits_cert_signing() {
            finding(
                &mut findings,
                ChainFindingKind::IssuerCannotSign,
                issuer,
                "issuer Key Usage excludes certificate signing".to_owned(),
            );
        }
        if !signed_by(child, issuer).unwrap_or(false) {
            finding(
                &mut findings,
                ChainFindingKind::InvalidSignature,
                child,
                format!(
                    "certificate signature does not verify under `{}`",
                    issuer.subject
                ),
            );
        }
    }

    for (position, &index) in path.iter().enumerate().skip(1) {
        let issuer = &certificates[index];
        let subordinate_cas = path[..position]
            .iter()
            .filter(|candidate| certificates[**candidate].is_ca)
            .count();
        if issuer
            .path_len_constraint
            .is_some_and(|limit| subordinate_cas > limit as usize)
        {
            finding(
                &mut findings,
                ChainFindingKind::PathLengthExceeded,
                issuer,
                "Basic Constraints path length exceeded".to_owned(),
            );
        }
        check_name_constraints(issuer, leaf, hostname, &mut findings);
    }

    findings
}

fn check_name_constraints(
    issuer: &Certificate,
    leaf: &Certificate,
    hostname: Option<&str>,
    findings: &mut Vec<ChainFinding>,
) {
    let unsupported: Vec<&str> = issuer
        .permitted_name_subtrees
        .iter()
        .chain(&issuer.excluded_name_subtrees)
        .map(String::as_str)
        .filter(|constraint| !constraint.starts_with("dns:"))
        .collect();
    if !unsupported.is_empty() {
        finding(
            findings,
            ChainFindingKind::UnsupportedNameConstraint,
            issuer,
            format!(
                "unsupported critical name-constraint form(s): {}",
                unsupported.join(", ")
            ),
        );
    }
    let names: Vec<&str> = hostname
        .into_iter()
        .chain(leaf.dns_names.iter().map(String::as_str))
        .collect();
    for name in names {
        if issuer.excluded_name_subtrees.iter().any(|constraint| {
            constraint
                .strip_prefix("dns:")
                .is_some_and(|suffix| within_dns_subtree(name, suffix))
        }) {
            finding(
                findings,
                ChainFindingKind::NameConstraintViolation,
                leaf,
                format!("DNS name `{name}` is in an excluded name subtree"),
            );
        }
        let permitted_dns: Vec<&str> = issuer
            .permitted_name_subtrees
            .iter()
            .filter_map(|constraint| constraint.strip_prefix("dns:"))
            .collect();
        if !permitted_dns.is_empty()
            && !permitted_dns
                .iter()
                .any(|suffix| within_dns_subtree(name, suffix))
        {
            finding(
                findings,
                ChainFindingKind::NameConstraintViolation,
                leaf,
                format!("DNS name `{name}` is outside every permitted name subtree"),
            );
        }
    }
}

fn within_dns_subtree(name: &str, subtree: &str) -> bool {
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    let subtree = subtree
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    name == subtree || name.ends_with(&format!(".{subtree}"))
}

fn trust_for_terminal(
    certificates: &[Certificate],
    path: &[usize],
    trust_store: &TrustStore,
    unix_time: i64,
) -> BTreeMap<ClientFamily, TrustDecision> {
    let Some(terminal) = path.last().and_then(|index| certificates.get(*index)) else {
        return unknown_trust();
    };
    CLIENTS
        .into_iter()
        .map(|client| {
            (
                client,
                trust_store.decision(
                    &terminal.fingerprint_sha256,
                    client,
                    TrustPurpose::ServerAuth,
                    unix_time,
                ),
            )
        })
        .collect()
}

fn unknown_trust() -> BTreeMap<ClientFamily, TrustDecision> {
    CLIENTS
        .into_iter()
        .map(|client| (client, TrustDecision::Unknown))
        .collect()
}

fn fingerprint_path(certificates: &[Certificate], path: &[usize]) -> Vec<String> {
    path.iter()
        .filter_map(|index| certificates.get(*index))
        .map(|certificate| certificate.fingerprint_sha256.clone())
        .collect()
}

fn finding(
    findings: &mut Vec<ChainFinding>,
    kind: ChainFindingKind,
    certificate: &Certificate,
    message: String,
) {
    findings.push(ChainFinding {
        kind,
        certificate: Some(certificate.fingerprint_sha256.clone()),
        message,
    });
}

const fn is_invalid(kind: ChainFindingKind) -> bool {
    matches!(
        kind,
        ChainFindingKind::NotYetValid
            | ChainFindingKind::Expired
            | ChainFindingKind::HostnameMismatch
            | ChainFindingKind::LeafNotServerAuth
            | ChainFindingKind::UnknownCriticalExtension
            | ChainFindingKind::IssuerNotCa
            | ChainFindingKind::IssuerCannotSign
            | ChainFindingKind::InvalidSignature
            | ChainFindingKind::PathLengthExceeded
            | ChainFindingKind::NameConstraintViolation
            | ChainFindingKind::UnsupportedNameConstraint
            | ChainFindingKind::PathLoop
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic(subject: &str, issuer: &str, fingerprint: &str) -> Certificate {
        Certificate {
            fingerprint_sha256: fingerprint.repeat(64),
            spki_sha256: "1".repeat(64),
            subject: subject.to_owned(),
            issuer: issuer.to_owned(),
            common_names: Vec::new(),
            serial: "01".to_owned(),
            not_before: 0,
            not_after: i64::MAX,
            public_key_algorithm: "1.2.840.113549.1.1.1".to_owned(),
            public_key_bits: Some(2048),
            signature_algorithm: "1.2.840.113549.1.1.11".to_owned(),
            subject_key_id: None,
            authority_key_id: None,
            dns_names: vec!["example.test".to_owned()],
            ip_addresses: Vec::new(),
            key_usage: Vec::new(),
            extended_key_usage: Vec::new(),
            is_ca: subject != "leaf",
            path_len_constraint: None,
            policy_oids: Vec::new(),
            permitted_name_subtrees: Vec::new(),
            excluded_name_subtrees: Vec::new(),
            ocsp_uris: Vec::new(),
            ca_issuer_uris: Vec::new(),
            crl_uris: Vec::new(),
            unknown_critical_extensions: Vec::new(),
            raw_der: Vec::new(),
        }
    }

    #[test]
    fn empty_chain_is_incomplete() {
        let evaluation = evaluate_chain(&[], &crate::trust::embedded_dataset().trust, None, 0);
        assert_eq!(evaluation.status, ChainStatus::Incomplete);
    }

    #[test]
    fn missing_issuer_is_reported_without_panicking() {
        let leaf = synthetic("leaf", "missing", "a");
        let evaluation = evaluate_chain(
            &[leaf],
            &crate::trust::embedded_dataset().trust,
            Some("example.test"),
            1,
        );
        assert!(
            evaluation
                .findings
                .iter()
                .any(|finding| finding.kind == ChainFindingKind::IssuerNotFound)
        );
    }

    #[test]
    fn dns_subtree_matching_is_label_bounded() {
        assert!(within_dns_subtree("www.example.test", ".example.test"));
        assert!(!within_dns_subtree("badexample.test", ".example.test"));
    }

    #[test]
    fn unsupported_name_constraint_forms_fail_closed() {
        let leaf = synthetic("leaf", "issuer", "a");
        let mut issuer = synthetic("issuer", "issuer", "b");
        issuer
            .permitted_name_subtrees
            .push("ip:192.0.2.0/24".to_owned());
        let evaluation = evaluate_chain(
            &[leaf, issuer],
            &crate::trust::embedded_dataset().trust,
            Some("example.test"),
            1,
        );
        assert!(
            evaluation
                .findings
                .iter()
                .any(|finding| { finding.kind == ChainFindingKind::UnsupportedNameConstraint })
        );
        assert_eq!(evaluation.status, ChainStatus::Invalid);
    }
}
