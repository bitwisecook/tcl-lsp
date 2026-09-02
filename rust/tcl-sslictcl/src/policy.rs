// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Declarative policy evaluation — a phase of its own, never part of loading.
//!
//! Loading a document produces [`Policy`] values; nothing is evaluated until a
//! caller hands one to [`evaluate_policy`] along with the endpoint, its
//! certificates, and an estimate. A check is the conjunction of its populated
//! members, and a failing check yields exactly one [`PolicyFinding`]: the
//! identity of a finding is the pair `(check_id, endpoint)`.
//!
//! A check's `predicate` script is retained on the check and ignored here. It
//! is never parsed as statements and never evaluated.

use serde::{Deserialize, Serialize};
use tcl_syntax::glob::string_match;

use crate::certificate::Certificate;
use crate::estimate::{Estimate, EstimateSeverity, cipher_has_forward_secrecy};
use crate::model::{Endpoint, Policy, PolicyCheck};

/// One policy check that an endpoint did not satisfy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyFinding {
    /// The failing check's identifier, or `grade` for the grade rule.
    pub check_id: String,
    /// The endpoint the check was evaluated against.
    pub endpoint: String,
    /// Declared severity, defaulting to `warning`.
    pub severity: EstimateSeverity,
    /// Declared message, or one derived from the check identity.
    pub message: String,
    /// Stable finding code, `SSLICTL-POLICY-<check_id>`.
    pub code: String,
    /// Why the check failed, one line per unsatisfied conjunct.
    pub evidence: Vec<String>,
}

/// Evaluate every check in `policy` against one endpoint.
///
/// Findings are returned in check order, with the grade rule last.
#[must_use]
pub fn evaluate_policy(
    policy: &Policy,
    endpoint: &Endpoint,
    certificates: &[Certificate],
    estimate: &Estimate,
) -> Vec<PolicyFinding> {
    let mut findings: Vec<PolicyFinding> = policy
        .checks
        .values()
        .filter_map(|check| {
            let evidence = check_evidence(check, endpoint, certificates);
            (!evidence.is_empty()).then(|| finding(&check.id, endpoint, check, evidence))
        })
        .collect();

    if let Some(rule) = policy.grade
        && estimate.grade.rank() < rule.minimum.rank()
    {
        findings.push(PolicyFinding {
            check_id: "grade".to_owned(),
            endpoint: endpoint.name.clone(),
            severity: EstimateSeverity::Warning,
            message: format!(
                "estimated grade {} is below the policy minimum {}",
                estimate.grade, rule.minimum
            ),
            code: "SSLICTL-POLICY-grade".to_owned(),
            evidence: vec![format!(
                "estimate grade {} ranks below required {}",
                estimate.grade, rule.minimum
            )],
        });
    }
    findings
}

fn finding(
    check_id: &str,
    endpoint: &Endpoint,
    check: &PolicyCheck,
    evidence: Vec<String>,
) -> PolicyFinding {
    PolicyFinding {
        check_id: check_id.to_owned(),
        endpoint: endpoint.name.clone(),
        severity: check.severity.unwrap_or(EstimateSeverity::Warning),
        message: check.message.clone().unwrap_or_else(|| {
            format!(
                "policy check `{check_id}` failed for endpoint `{}`",
                endpoint.name
            )
        }),
        code: format!("SSLICTL-POLICY-{check_id}"),
        evidence,
    }
}

/// The unsatisfied conjuncts of one check. Empty means the check passed.
fn check_evidence(
    check: &PolicyCheck,
    endpoint: &Endpoint,
    certificates: &[Certificate],
) -> Vec<String> {
    let mut evidence = Vec::new();
    for protocol in &check.require_protocols {
        if !endpoint.protocols.contains(protocol) {
            evidence.push(format!("required protocol `{protocol}` is not enabled"));
        }
    }
    for protocol in &check.forbid_protocols {
        if endpoint.protocols.contains(protocol) {
            evidence.push(format!("forbidden protocol `{protocol}` is enabled"));
        }
    }
    for pattern in &check.forbid_ciphers {
        for cipher in &endpoint.ciphers {
            if string_match(pattern, cipher) {
                evidence.push(format!(
                    "cipher `{cipher}` matches forbidden pattern `{pattern}`"
                ));
            }
        }
    }
    if check.require_forward_secrecy == Some(true) {
        for cipher in &endpoint.ciphers {
            if !cipher_has_forward_secrecy(cipher, None) {
                evidence.push(format!("cipher `{cipher}` has no forward secrecy"));
            }
        }
    }
    if let Some(minimum) = check.min_key_bits {
        evidence.extend(key_bits_evidence(minimum, certificates));
    }
    if check.require_hsts == Some(true) && !endpoint.hsts.as_ref().is_some_and(|hsts| hsts.enabled)
    {
        evidence.push("HSTS is not enabled".to_owned());
    }
    if let Some(minimum) = check.min_hsts_max_age {
        let actual = endpoint.hsts.as_ref().and_then(|hsts| hsts.max_age);
        match actual {
            Some(seconds) if seconds >= minimum => {}
            Some(seconds) => evidence.push(format!(
                "HSTS max-age {seconds} is below the required {minimum}"
            )),
            None => evidence.push(format!(
                "HSTS max-age is not declared; the policy requires at least {minimum}"
            )),
        }
    }
    evidence
}

/// An unknown key size cannot demonstrate compliance, so it fails the check
/// rather than silently passing.
fn key_bits_evidence(minimum: u32, certificates: &[Certificate]) -> Option<String> {
    let Some(leaf) = certificates.first() else {
        return Some(format!(
            "no leaf certificate is available to satisfy the {minimum}-bit minimum"
        ));
    };
    match leaf.public_key_bits {
        Some(bits) if bits >= minimum => None,
        Some(bits) => Some(format!(
            "leaf public key is {bits} bits, below the required {minimum}"
        )),
        None => Some(format!(
            "leaf public-key size is unknown; the policy requires at least {minimum} bits"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{ChainEvaluation, ChainStatus};
    use crate::estimate::Grade;
    use crate::model::{GradeRule, HstsPolicy, ProtocolVersion};

    fn estimate_with(grade: Grade) -> Estimate {
        Estimate {
            grade,
            numeric_score: 0,
            protocol_score: 0,
            key_exchange_score: 0,
            cipher_score: 0,
            confidence: 0,
            methodology: "test".to_owned(),
            caps: Vec::new(),
            chain: ChainEvaluation {
                status: ChainStatus::Incomplete,
                path: Vec::new(),
                alternate_paths: Vec::new(),
                trust: std::collections::BTreeMap::new(),
                findings: Vec::new(),
            },
            findings: Vec::new(),
        }
    }

    fn endpoint() -> Endpoint {
        Endpoint {
            name: "vs".to_owned(),
            protocols: vec![ProtocolVersion::Tls12],
            ciphers: vec![
                "AES128-SHA".to_owned(),
                "ECDHE-RSA-AES128-GCM-SHA256".to_owned(),
            ],
            hsts: Some(HstsPolicy {
                enabled: true,
                max_age: Some(600),
                include_subdomains: false,
                preload: false,
            }),
            ..Endpoint::default()
        }
    }

    fn policy_with(check: PolicyCheck) -> Policy {
        let mut policy = Policy {
            name: "p".to_owned(),
            ..Policy::default()
        };
        policy.checks.insert(check.id.clone(), check);
        policy
    }

    fn check(id: &str) -> PolicyCheck {
        PolicyCheck {
            id: id.to_owned(),
            ..PolicyCheck::default()
        }
    }

    fn evaluate(check: PolicyCheck) -> Vec<PolicyFinding> {
        evaluate_policy(
            &policy_with(check),
            &endpoint(),
            &[],
            &estimate_with(Grade::B),
        )
    }

    #[test]
    fn require_protocols_fails_when_absent() {
        let mut rule = check("modern");
        rule.require_protocols = vec![ProtocolVersion::Tls13];
        let findings = evaluate(rule);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "SSLICTL-POLICY-modern");
        assert_eq!(findings[0].severity, EstimateSeverity::Warning);
        assert!(findings[0].evidence[0].contains("tls1.3"));
    }

    #[test]
    fn require_protocols_passes_when_present() {
        let mut rule = check("modern");
        rule.require_protocols = vec![ProtocolVersion::Tls12];
        assert!(evaluate(rule).is_empty());
    }

    #[test]
    fn forbid_protocols_fails_when_enabled() {
        let mut rule = check("legacy");
        rule.forbid_protocols = vec![ProtocolVersion::Tls12];
        rule.severity = Some(EstimateSeverity::Critical);
        rule.message = Some("no TLS 1.2".to_owned());
        let findings = evaluate(rule);
        assert_eq!(findings[0].severity, EstimateSeverity::Critical);
        assert_eq!(findings[0].message, "no TLS 1.2");
    }

    #[test]
    fn forbid_ciphers_uses_glob_semantics() {
        let mut rule = check("weak");
        rule.forbid_ciphers = vec!["AES128-*".to_owned()];
        let findings = evaluate(rule);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].evidence.len(), 1);
        assert!(findings[0].evidence[0].contains("AES128-SHA"));

        let mut miss = check("weak");
        miss.forbid_ciphers = vec!["RC4*".to_owned()];
        assert!(evaluate(miss).is_empty());
    }

    #[test]
    fn require_forward_secrecy_flags_static_rsa_suites() {
        let mut rule = check("fs");
        rule.require_forward_secrecy = Some(true);
        let findings = evaluate(rule);
        assert_eq!(
            findings[0].evidence,
            ["cipher `AES128-SHA` has no forward secrecy"]
        );

        let mut off = check("fs");
        off.require_forward_secrecy = Some(false);
        assert!(evaluate(off).is_empty());
    }

    #[test]
    fn min_key_bits_fails_without_a_leaf() {
        let mut rule = check("keysize");
        rule.min_key_bits = Some(2048);
        let findings = evaluate(rule);
        assert!(findings[0].evidence[0].contains("no leaf certificate"));
    }

    #[test]
    fn hsts_rules_read_the_declared_policy() {
        let mut required = check("hsts");
        required.require_hsts = Some(true);
        assert!(evaluate(required).is_empty());

        let mut age = check("hsts-age");
        age.min_hsts_max_age = Some(15_552_000);
        let findings = evaluate(age);
        assert!(findings[0].evidence[0].contains("below the required"));

        let mut disabled = check("hsts");
        disabled.require_hsts = Some(true);
        let mut endpoint = endpoint();
        endpoint.hsts = None;
        let findings = evaluate_policy(
            &policy_with(disabled),
            &endpoint,
            &[],
            &estimate_with(Grade::B),
        );
        assert_eq!(findings[0].evidence, ["HSTS is not enabled"]);
    }

    #[test]
    fn predicate_is_retained_and_ignored() {
        let mut rule = check("scripted");
        rule.predicate = Some("expr {1 == 2}".to_owned());
        assert!(evaluate(rule).is_empty());
    }

    #[test]
    fn grade_rule_fires_below_the_minimum_only() {
        let mut policy = Policy {
            name: "p".to_owned(),
            ..Policy::default()
        };
        policy.grade = Some(GradeRule { minimum: Grade::A });
        let findings = evaluate_policy(&policy, &endpoint(), &[], &estimate_with(Grade::B));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check_id, "grade");
        assert_eq!(findings[0].code, "SSLICTL-POLICY-grade");
        assert!(
            evaluate_policy(&policy, &endpoint(), &[], &estimate_with(Grade::APlus)).is_empty()
        );
    }

    #[test]
    fn finding_identity_is_check_and_endpoint() {
        let mut rule = check("weak");
        rule.forbid_ciphers = vec!["*".to_owned()];
        let findings = evaluate(rule);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].check_id, "weak");
        assert_eq!(findings[0].endpoint, "vs");
        assert_eq!(findings[0].evidence.len(), 2);
    }
}
