// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Embedded trust-anchor membership data and deterministic trust decisions.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::certificate::{Certificate, parse_certificates};

/// Browser/operating-system/runtime root program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClientFamily {
    /// Mozilla NSS (and Firefox).
    Mozilla,
    /// Chrome Root Store.
    Chrome,
    /// Apple trust stores.
    Apple,
    /// Microsoft trusted roots.
    Microsoft,
    /// Android system roots.
    Android,
    /// `OpenJDK` `cacerts`.
    OpenJdk,
}

/// Intended trust purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustPurpose {
    /// TLS server authentication.
    ServerAuth,
    /// TLS client authentication.
    ClientAuth,
    /// S/MIME email protection.
    EmailProtection,
    /// Code signing.
    CodeSigning,
    /// Any purpose allowed by the source root program.
    Any,
}

/// Membership of one anchor in one client program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Membership {
    /// Client root program.
    pub client: ClientFamily,
    /// First known product/version snapshot, inclusive.
    pub first_version: Option<String>,
    /// Last known product/version snapshot, inclusive.
    pub last_version: Option<String>,
    /// Root-program snapshot identifier (not a client product range).
    #[serde(default)]
    pub snapshot_version: Option<String>,
    /// ISO-8601 date on which the represented root-program snapshot was fetched.
    #[serde(default)]
    pub snapshot_date: Option<String>,
    /// Trust purposes asserted by this specific root program. Empty means the
    /// source did not publish purpose semantics; it must not inherit purposes
    /// asserted by another client program for the same certificate.
    #[serde(default)]
    pub purposes: Vec<TrustPurpose>,
    /// Whether all root-program constraints needed for this decision are
    /// represented. False makes the result Unknown rather than overclaiming.
    #[serde(default)]
    pub policy_complete: bool,
    /// Whether the anchor is trusted in the represented snapshot.
    pub trusted: bool,
    /// Optional Unix time after which the root program distrusts the anchor.
    pub distrust_after: Option<i64>,
}

/// One root certificate identity and its program memberships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    /// SHA-256 DER fingerprint, lowercase hexadecimal.
    pub fingerprint_sha256: String,
    /// Human-readable distinguished name.
    pub subject: String,
    /// Complete root certificate DER as base64, when the source publishes it.
    /// This lets the path builder complete and cryptographically verify a
    /// chain without external files.
    pub der_base64: Option<String>,
    /// SHA-256 of `SubjectPublicKeyInfo`, when available from source data.
    pub spki_sha256: Option<String>,
    /// Subject Key Identifier, lowercase hexadecimal, when present.
    pub subject_key_id: Option<String>,
    /// Root validity start as Unix seconds.
    pub not_before: Option<i64>,
    /// Root validity end as Unix seconds.
    pub not_after: Option<i64>,
    /// Trust purposes represented by this data snapshot.
    pub purposes: Vec<TrustPurpose>,
    /// Per-client root-program membership.
    pub memberships: Vec<Membership>,
}

/// One upstream input used to produce the embedded data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceProvenance {
    /// Stable source label.
    pub name: String,
    /// Source URL; informational only, never fetched while analysing.
    pub url: String,
    /// Upstream version, commit, or snapshot identifier.
    pub revision: String,
    /// SPDX license expression or a precise data-use label.
    pub license: String,
}

/// Reproducibility metadata carried with the embedded dataset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Data schema version.
    pub schema: u32,
    /// ISO-8601 generation timestamp.
    pub generated_at: String,
    /// Whether this is a deliberately small bootstrap dataset.
    pub bootstrap_seed: bool,
    /// Important accuracy/coverage qualification.
    pub notice: String,
    /// Source receipts.
    pub sources: Vec<SourceProvenance>,
}

/// Compact, deterministic trust store indexed by certificate fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustStore {
    /// Data provenance.
    pub provenance: Provenance,
    /// Anchors in stable fingerprint order.
    pub anchors: Vec<Anchor>,
    #[serde(skip)]
    index: BTreeMap<String, usize>,
}

impl TrustStore {
    /// Build a validated trust store from generated or caller-supplied data.
    pub fn new(provenance: Provenance, anchors: Vec<Anchor>) -> Result<Self, String> {
        let mut store = Self {
            provenance,
            anchors,
            index: BTreeMap::new(),
        };
        store.validate()?;
        store.build_index();
        Ok(store)
    }

    fn build_index(&mut self) {
        self.index = self
            .anchors
            .iter()
            .enumerate()
            .map(|(index, anchor)| (normalise_fingerprint(&anchor.fingerprint_sha256), index))
            .collect();
    }

    /// Validate schema, fingerprint uniqueness, and source receipts.
    pub fn validate(&self) -> Result<(), String> {
        if self.provenance.schema != 1 {
            return Err(format!(
                "unsupported embedded trust schema {}",
                self.provenance.schema
            ));
        }
        if self.provenance.sources.is_empty() {
            return Err("embedded trust data has no provenance sources".to_owned());
        }
        let mut seen = BTreeSet::new();
        for anchor in &self.anchors {
            let fingerprint = normalise_fingerprint(&anchor.fingerprint_sha256);
            if fingerprint.len() != 64 || !fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(format!(
                    "anchor `{}` has invalid SHA-256 fingerprint",
                    anchor.subject
                ));
            }
            if !seen.insert(fingerprint) {
                return Err(format!("duplicate trust anchor `{}`", anchor.subject));
            }
        }
        Ok(())
    }

    /// Find an anchor by punctuation-insensitive SHA-256 fingerprint.
    #[must_use]
    pub fn anchor(&self, fingerprint: &str) -> Option<&Anchor> {
        let normalised = normalise_fingerprint(fingerprint);
        self.index
            .get(&normalised)
            .and_then(|index| self.anchors.get(*index))
    }

    /// Parse all anchors whose source snapshot includes complete DER.
    pub fn anchor_certificates(&self) -> Result<Vec<Certificate>, String> {
        self.anchors
            .iter()
            .filter_map(|anchor| {
                anchor
                    .der_base64
                    .as_ref()
                    .map(|der| (anchor.fingerprint_sha256.as_str(), der))
            })
            .map(|(fingerprint, encoded)| {
                let der = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|error| {
                        format!("invalid base64 DER for anchor {fingerprint}: {error}")
                    })?;
                let mut certificates = parse_certificates(&der)
                    .map_err(|error| format!("invalid DER for anchor {fingerprint}: {error}"))?;
                let certificate = certificates
                    .pop()
                    .ok_or_else(|| format!("anchor {fingerprint} DER contained no certificate"))?;
                if normalise_fingerprint(&certificate.fingerprint_sha256)
                    != normalise_fingerprint(fingerprint)
                {
                    return Err(format!("anchor {fingerprint} DER fingerprint mismatch"));
                }
                Ok(certificate)
            })
            .collect()
    }

    /// `(anchors with complete DER, total anchors)` for provenance/reporting.
    #[must_use]
    pub fn der_coverage(&self) -> (usize, usize) {
        (
            self.anchors
                .iter()
                .filter(|anchor| anchor.der_base64.is_some())
                .count(),
            self.anchors.len(),
        )
    }

    /// `(memberships with complete policy and an answer for purpose, total
    /// memberships)`. Reports should show this alongside DER coverage so a
    /// large anchor count cannot be mistaken for comprehensive trust answers.
    #[must_use]
    pub fn purpose_coverage(&self, purpose: TrustPurpose) -> (usize, usize) {
        let total = self
            .anchors
            .iter()
            .map(|anchor| anchor.memberships.len())
            .sum();
        let complete = self
            .anchors
            .iter()
            .flat_map(|anchor| &anchor.memberships)
            .filter(|membership| {
                membership.policy_complete
                    && (membership.purposes.contains(&TrustPurpose::Any)
                        || membership.purposes.contains(&purpose))
            })
            .count();
        (complete, total)
    }

    /// Decide whether one fingerprint is trusted for a client and purpose at
    /// a particular Unix time. Unknown is distinct from explicit distrust.
    #[must_use]
    pub fn decision(
        &self,
        fingerprint: &str,
        client: ClientFamily,
        purpose: TrustPurpose,
        unix_time: i64,
    ) -> TrustDecision {
        let Some(anchor) = self.anchor(fingerprint) else {
            return TrustDecision::Unknown;
        };
        let Some(membership) = anchor
            .memberships
            .iter()
            .filter(|membership| membership.client == client)
            .max_by(|left, right| membership_version(left).cmp(membership_version(right)))
        else {
            return TrustDecision::Unknown;
        };
        if !membership.trusted
            || membership
                .distrust_after
                .is_some_and(|deadline| unix_time >= deadline)
        {
            return TrustDecision::Distrusted;
        }
        if !membership.policy_complete
            || membership.purposes.is_empty()
            || (!membership.purposes.contains(&TrustPurpose::Any)
                && !membership.purposes.contains(&purpose))
        {
            return TrustDecision::Unknown;
        }
        TrustDecision::Trusted
    }

    /// Decide trust for a particular product version. Version comparison is
    /// numeric-component aware (`120.1` sorts after `99`) with lexical
    /// fallback for vendor labels.
    #[must_use]
    pub fn decision_for_version(
        &self,
        fingerprint: &str,
        client: ClientFamily,
        client_version: &str,
        purpose: TrustPurpose,
        unix_time: i64,
    ) -> TrustDecision {
        let Some(anchor) = self.anchor(fingerprint) else {
            return TrustDecision::Unknown;
        };
        let Some(membership) = anchor.memberships.iter().find(|membership| {
            membership.client == client
                && (membership.first_version.is_some() || membership.last_version.is_some())
                && membership
                    .first_version
                    .as_deref()
                    .is_none_or(|first| compare_versions(client_version, first).is_ge())
                && membership
                    .last_version
                    .as_deref()
                    .is_none_or(|last| compare_versions(client_version, last).is_le())
        }) else {
            return TrustDecision::Unknown;
        };
        if !membership.trusted
            || membership
                .distrust_after
                .is_some_and(|deadline| unix_time >= deadline)
        {
            return TrustDecision::Distrusted;
        }
        if !membership.policy_complete
            || membership.purposes.is_empty()
            || (!membership.purposes.contains(&TrustPurpose::Any)
                && !membership.purposes.contains(&purpose))
        {
            return TrustDecision::Unknown;
        }
        TrustDecision::Trusted
    }
}

/// Result of consulting a root-program snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustDecision {
    /// Anchor is included and usable for the purpose.
    Trusted,
    /// Anchor is explicitly untrusted, expired by policy, or purpose-excluded.
    Distrusted,
    /// This data snapshot cannot answer.
    Unknown,
}

/// Top-level embedded-data envelope. Further registries can be added without
/// changing the trust store's wire schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddedDataset {
    /// Root program data.
    pub trust: TrustStore,
}

static EMBEDDED: LazyLock<EmbeddedDataset> = LazyLock::new(|| {
    let mut dataset: EmbeddedDataset =
        serde_json::from_str(include_str!("../data/generated/trust-seed.json"))
            .expect("committed SslicTcl trust data must be valid JSON");
    dataset
        .trust
        .validate()
        .expect("committed SslicTcl trust data must satisfy schema invariants");
    hydrate_anchor_der(&mut dataset.trust)
        .expect("committed SslicTcl PEM bundles must match trust-anchor fingerprints");
    dataset.trust.build_index();
    dataset
});

// Full root material stays in auditable, updater-owned PEM bundles rather than
// duplicating base64 into the compact membership JSON. `include_bytes!` keeps
// it available to native, WASM, and single-file report builds with no runtime
// filesystem or network dependency.
const EMBEDDED_PEM_BUNDLES: &[&[u8]] = &[
    include_bytes!("../data/raw/pem/apple.pem"),
    include_bytes!("../data/raw/pem/google_aosp.pem"),
    include_bytes!("../data/raw/pem/microsoft_windows.pem"),
    include_bytes!("../data/raw/pem/mozilla_nss.pem"),
    include_bytes!("../data/raw/pem/openjdk.pem"),
    include_bytes!("../data/raw/pem/oracle_java.pem"),
];

fn hydrate_anchor_der(store: &mut TrustStore) -> Result<(), String> {
    let indices: BTreeMap<String, usize> = store
        .anchors
        .iter()
        .enumerate()
        .map(|(index, anchor)| (normalise_fingerprint(&anchor.fingerprint_sha256), index))
        .collect();
    for bundle in EMBEDDED_PEM_BUNDLES {
        let certificates = parse_certificates(bundle)
            .map_err(|error| format!("cannot parse embedded root bundle: {error}"))?;
        for certificate in certificates {
            let Some(index) = indices.get(&certificate.fingerprint_sha256) else {
                continue;
            };
            let anchor = &mut store.anchors[*index];
            let encoded = base64::engine::general_purpose::STANDARD.encode(&certificate.raw_der);
            if anchor
                .der_base64
                .as_ref()
                .is_some_and(|existing| existing != &encoded)
            {
                return Err(format!(
                    "conflicting DER for anchor {}",
                    anchor.fingerprint_sha256
                ));
            }
            anchor.der_base64 = Some(encoded);
            anchor.spki_sha256 = Some(certificate.spki_sha256);
            anchor.subject_key_id = certificate.subject_key_id;
            anchor.not_before = Some(certificate.not_before);
            anchor.not_after = Some(certificate.not_after);
        }
    }
    Ok(())
}

/// Access the compiled-in, network-free dataset.
#[must_use]
pub fn embedded_dataset() -> &'static EmbeddedDataset {
    &EMBEDDED
}

fn normalise_fingerprint(fingerprint: &str) -> String {
    fingerprint
        .chars()
        .filter(char::is_ascii_hexdigit)
        .flat_map(char::to_lowercase)
        .collect()
}

fn membership_version(membership: &Membership) -> &str {
    membership
        .snapshot_date
        .as_deref()
        .or(membership.snapshot_version.as_deref())
        .or(membership
            .last_version
            .as_deref()
            .or(membership.first_version.as_deref()))
        .unwrap_or("")
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let numeric = |value: &str| {
        value
            .split(|character: char| !character.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .map(|part| part.parse::<u64>().unwrap_or(u64::MAX))
            .collect::<Vec<_>>()
    };
    let left_numeric = numeric(left);
    let right_numeric = numeric(right);
    if !left_numeric.is_empty() && !right_numeric.is_empty() {
        left_numeric.cmp(&right_numeric)
    } else {
        left.cmp(right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_is_valid_and_indexed() {
        let dataset = embedded_dataset();
        assert!(dataset.trust.validate().is_ok());
        assert!(dataset.trust.anchors.len() >= 5);
        let (complete, total) = dataset.trust.der_coverage();
        assert!(complete > total / 2, "DER coverage is {complete}/{total}");
        assert!(dataset
            .trust
            .anchor("96:BC:EC:06:26:49:76:F3:74:60:77:9A:CF:28:C5:A7:CF:E8:A3:C0:AA:E1:1A:8F:FC:EE:05:C0:BD:DF:08:C6")
            .is_some());
    }

    #[test]
    fn unknown_and_explicit_distrust_stay_distinct() {
        let fingerprint = "ab".repeat(32);
        let store = TrustStore::new(
            Provenance {
                schema: 1,
                generated_at: "2026-01-01T00:00:00Z".to_owned(),
                bootstrap_seed: false,
                notice: "test".to_owned(),
                sources: vec![SourceProvenance {
                    name: "test".to_owned(),
                    url: "https://example.test".to_owned(),
                    revision: "1".to_owned(),
                    license: "CC0-1.0".to_owned(),
                }],
            },
            vec![Anchor {
                fingerprint_sha256: fingerprint.clone(),
                subject: "CN=Test".to_owned(),
                der_base64: None,
                spki_sha256: None,
                subject_key_id: None,
                not_before: None,
                not_after: None,
                purposes: vec![TrustPurpose::ServerAuth],
                memberships: vec![Membership {
                    client: ClientFamily::Mozilla,
                    first_version: None,
                    last_version: None,
                    snapshot_version: Some("test-snapshot".to_owned()),
                    snapshot_date: Some("2026-01-01".to_owned()),
                    purposes: vec![TrustPurpose::ServerAuth],
                    policy_complete: true,
                    trusted: false,
                    distrust_after: None,
                }],
            }],
        )
        .expect("test store is valid");
        assert_eq!(
            store.decision("00", ClientFamily::Mozilla, TrustPurpose::ServerAuth, 0),
            TrustDecision::Unknown
        );
        assert_eq!(
            store.decision(
                &fingerprint,
                ClientFamily::Mozilla,
                TrustPurpose::ServerAuth,
                0
            ),
            TrustDecision::Distrusted
        );
    }

    #[test]
    fn purposes_never_leak_between_client_memberships() {
        let fingerprint = "cd".repeat(32);
        let store = TrustStore::new(
            Provenance {
                schema: 1,
                generated_at: "2026-01-01T00:00:00Z".to_owned(),
                bootstrap_seed: false,
                notice: "test".to_owned(),
                sources: vec![SourceProvenance {
                    name: "test".to_owned(),
                    url: "https://example.test".to_owned(),
                    revision: "1".to_owned(),
                    license: "CC0-1.0".to_owned(),
                }],
            },
            vec![Anchor {
                fingerprint_sha256: fingerprint.clone(),
                subject: "CN=Shared".to_owned(),
                der_base64: None,
                spki_sha256: None,
                subject_key_id: None,
                not_before: None,
                not_after: None,
                purposes: vec![TrustPurpose::ServerAuth],
                memberships: vec![
                    Membership {
                        client: ClientFamily::Mozilla,
                        first_version: None,
                        last_version: None,
                        snapshot_version: None,
                        snapshot_date: None,
                        purposes: vec![TrustPurpose::ServerAuth],
                        policy_complete: true,
                        trusted: true,
                        distrust_after: None,
                    },
                    Membership {
                        client: ClientFamily::Apple,
                        first_version: None,
                        last_version: None,
                        snapshot_version: None,
                        snapshot_date: None,
                        purposes: Vec::new(),
                        policy_complete: true,
                        trusted: true,
                        distrust_after: None,
                    },
                ],
            }],
        )
        .unwrap();
        assert_eq!(
            store.decision(
                &fingerprint,
                ClientFamily::Mozilla,
                TrustPurpose::ServerAuth,
                0
            ),
            TrustDecision::Trusted
        );
        assert_eq!(
            store.decision(
                &fingerprint,
                ClientFamily::Apple,
                TrustPurpose::ServerAuth,
                0
            ),
            TrustDecision::Unknown
        );
    }
}
