// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compact `SslicTcl` analysis embedded in the single-file report model.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{Map, Value as J};
use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip::tls::import_bigip_tls;
use tcl_bigip::validator::{ProfileDefaultState, collect_profile_default_evidence};
use tcl_registry::profile_defaults::BigipVersion;
use tcl_sslictcl::{
    ClientFamily, EstimateInput, KeyMatchStatus, TrustPurpose, embedded_dataset, estimate,
};

/// Resolve BIG-IP TLS configuration and produce deterministic offline grades.
#[must_use]
pub(crate) fn collect_tls(
    source: &str,
    version: Option<BigipVersion>,
    certificate_pems: &HashMap<String, String>,
    analysis_time: i64,
) -> J {
    let config = parse_bigip_conf(source, "Common");
    let imported = import_bigip_tls(&config, version, certificate_pems);
    let profile_default_evidence: Vec<J> = collect_profile_default_evidence(&config, version)
        .into_iter()
        .map(|evidence| {
            serde_json::json!({
                "owner": evidence.owner,
                "profileType": evidence.profile_type,
                "field": evidence.field,
                "configured": evidence.configured,
                "defaultValue": evidence.default_value,
                "state": match evidence.state {
                    ProfileDefaultState::Explicit => "explicit",
                    ProfileDefaultState::Inherited => "inherited",
                    ProfileDefaultState::Factory => "factory",
                    ProfileDefaultState::UnknownVersion => "unknown-version",
                },
                "securityRelevant": evidence.security_relevant,
            })
        })
        .collect();
    let dataset = embedded_dataset();
    let endpoints: Vec<J> = imported
        .endpoints
        .iter()
        .map(|endpoint| {
            let result = estimate(EstimateInput {
                endpoint: &endpoint.endpoint,
                certificates: &endpoint.certificates,
                trust_store: &dataset.trust,
                testssl: None,
                unix_time: analysis_time,
            });
            let mut estimate_value = serde_json::to_value(result).unwrap_or(J::Null);
            let key_finding = match endpoint.key_match.status {
                KeyMatchStatus::Match => None,
                KeyMatchStatus::Mismatch => {
                    estimate_value["grade"] = J::String("F".to_owned());
                    if let Some(caps) = estimate_value.get_mut("caps").and_then(J::as_array_mut) {
                        caps.push(J::String(
                            "certificate/private-key SPKI mismatch prevents a valid TLS identity"
                                .to_owned(),
                        ));
                    }
                    Some(serde_json::json!({
                        "code": "SSLICTL1020",
                        "severity": "critical",
                        "message": endpoint.key_match.message,
                        "source": "private-key",
                    }))
                }
                KeyMatchStatus::Unknown => {
                    if !estimate_value["grade"].as_str().is_some_and(is_fatal_grade) {
                        estimate_value["grade"] = J::String("Unknown".to_owned());
                    }
                    let confidence = estimate_value["confidence"].as_u64().unwrap_or(0).min(45);
                    estimate_value["confidence"] = J::from(confidence);
                    Some(serde_json::json!({
                        "code": "SSLICTL1021",
                        "severity": "warning",
                        "message": endpoint.key_match.message,
                        "source": "private-key",
                    }))
                }
            };
            if let Some(finding) = key_finding
                && let Some(findings) = estimate_value.get_mut("findings").and_then(J::as_array_mut)
            {
                findings.push(finding);
            }
            let mut value = serde_json::to_value(endpoint).unwrap_or(J::Null);
            if let Some(object) = value.as_object_mut() {
                object.insert("estimate".to_owned(), estimate_value);
            }
            value
        })
        .collect();
    let findings: Vec<J> = endpoints
        .iter()
        .flat_map(|endpoint| {
            let identity = endpoint
                .get("virtual_server")
                .and_then(J::as_str)
                .unwrap_or_default()
                .to_owned();
            endpoint
                .get("estimate")
                .and_then(|estimate| estimate.get("findings"))
                .and_then(J::as_array)
                .into_iter()
                .flatten()
                .map(move |finding| {
                    let mut finding = finding.clone();
                    if let Some(object) = finding.as_object_mut() {
                        object.insert("virtualServer".to_owned(), J::String(identity.clone()));
                    }
                    finding
                })
        })
        .collect();
    let (der_complete, der_total) = dataset.trust.der_coverage();
    let (server_auth_policy_complete, policy_memberships) =
        dataset.trust.purpose_coverage(TrustPurpose::ServerAuth);
    let mut purpose_coverage: BTreeMap<&str, usize> = BTreeMap::new();
    let mut client_anchors: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut product_ranges: BTreeMap<&str, usize> = BTreeMap::new();
    for anchor in &dataset.trust.anchors {
        for purpose in &anchor.purposes {
            *purpose_coverage.entry(purpose_name(*purpose)).or_default() += 1;
        }
        for membership in &anchor.memberships {
            let client = client_name(membership.client);
            client_anchors
                .entry(client)
                .or_default()
                .insert(&anchor.fingerprint_sha256);
            if membership.first_version.is_some() || membership.last_version.is_some() {
                *product_ranges.entry(client).or_default() += 1;
            }
        }
    }
    let client_coverage: BTreeMap<&str, J> = client_anchors
        .into_iter()
        .map(|(client, anchors)| {
            (
                client,
                serde_json::json!({
                    "anchors": anchors.len(),
                    "productRanges": product_ranges.get(client).copied().unwrap_or(0),
                }),
            )
        })
        .collect();
    let mut result = Map::new();
    result.insert(
        "profiles".to_owned(),
        serde_json::to_value(imported.profiles).unwrap_or(J::Array(Vec::new())),
    );
    result.insert("endpoints".to_owned(), J::Array(endpoints));
    result.insert("findings".to_owned(), J::Array(findings));
    result.insert(
        "notices".to_owned(),
        serde_json::to_value(imported.notices).unwrap_or(J::Array(Vec::new())),
    );
    result.insert("analysisTime".to_owned(), J::from(analysis_time));
    result.insert(
        "profileDefaultEvidence".to_owned(),
        J::Array(profile_default_evidence),
    );
    result.insert(
        "provenance".to_owned(),
        serde_json::to_value(&dataset.trust.provenance).unwrap_or(J::Null),
    );
    result.insert(
        "trustDerCoverage".to_owned(),
        serde_json::json!({ "complete": der_complete, "total": der_total }),
    );
    result.insert(
        "trustCoverage".to_owned(),
        serde_json::json!({
            "anchors": dataset.trust.anchors.len(),
            "der": { "complete": der_complete, "total": der_total },
            "serverAuthPolicy": {
                "complete": server_auth_policy_complete,
                "total": policy_memberships,
            },
            "purposes": purpose_coverage,
            "clients": client_coverage,
        }),
    );
    J::Object(result)
}

fn is_fatal_grade(grade: &str) -> bool {
    matches!(grade, "F" | "T" | "M")
}

const fn purpose_name(purpose: TrustPurpose) -> &'static str {
    match purpose {
        TrustPurpose::ServerAuth => "server-auth",
        TrustPurpose::ClientAuth => "client-auth",
        TrustPurpose::EmailProtection => "email-protection",
        TrustPurpose::CodeSigning => "code-signing",
        TrustPurpose::Any => "any",
    }
}

const fn client_name(client: ClientFamily) -> &'static str {
    match client {
        ClientFamily::Mozilla => "mozilla",
        ClientFamily::Chrome => "chrome",
        ClientFamily::Apple => "apple",
        ClientFamily::Microsoft => "microsoft",
        ClientFamily::Android => "android",
        ClientFamily::OpenJdk => "open-jdk",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_evidence_does_not_hide_a_fatal_grade() {
        assert!(is_fatal_grade("F"));
        assert!(is_fatal_grade("T"));
        assert!(is_fatal_grade("M"));
        assert!(!is_fatal_grade("A"));
        assert!(!is_fatal_grade("Unknown"));
    }

    #[test]
    fn multi_sni_profiles_become_separate_report_endpoints() {
        let source = r"
ltm profile client-ssl sni {
    defaults-from /Common/clientssl
    cert-key-chain {
        rsa { cert /Common/rsa.crt key /Common/rsa.key }
        ecdsa { cert /Common/ecdsa.crt key /Common/ecdsa.key }
    }
}
ltm virtual https {
    destination /Common/192.0.2.1:443
    profiles { /Common/sni { context clientside } }
}
";
        let tls = collect_tls(
            source,
            BigipVersion::parse("17.1"),
            &HashMap::new(),
            1_786_838_400,
        );
        assert_eq!(tls["endpoints"].as_array().map(Vec::len), Some(2));
        assert!(
            tls["provenance"]["sources"]
                .as_array()
                .is_some_and(|sources| !sources.is_empty())
        );
        assert!(
            tls["trustDerCoverage"]["total"]
                .as_u64()
                .is_some_and(|total| total > 100)
        );
    }
}
