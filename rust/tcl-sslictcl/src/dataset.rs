// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Normalized input schema and deterministic compiler for trust-store updates.
//!
//! Fetchers are intentionally outside this crate: release tooling may obtain
//! Trust Stores Observatory or vendor data, but ordinary builds and reports
//! never use the network. Each fetcher emits this small normalized snapshot,
//! then this module performs identity validation, DER projection, merge, sort,
//! and canonical serialization.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::certificate::parse_certificates;
use crate::trust::{
    Anchor, ClientFamily, EmbeddedDataset, Membership, Provenance, SourceProvenance, TrustPurpose,
    TrustStore,
};

/// One root-program snapshot produced by a source-specific updater.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustProgramSnapshot {
    /// Normalized input schema.
    pub schema: u32,
    /// Root program represented by the snapshot.
    pub client: ClientFamily,
    /// Product/root-store version or upstream revision.
    pub version: String,
    /// ISO-8601 retrieval/generation timestamp.
    pub generated_at: String,
    /// Exact upstream receipt.
    pub source: SourceProvenance,
    /// Roots present or explicitly distrusted in the snapshot.
    pub anchors: Vec<ProgramAnchor>,
}

/// One normalized anchor record from a root-program snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramAnchor {
    /// SHA-256 of certificate DER. May be empty when `der_base64` is present;
    /// the compiler then derives it.
    #[serde(default)]
    pub fingerprint_sha256: String,
    /// Display subject. When DER exists the compiler uses its parsed subject.
    #[serde(default)]
    pub subject: String,
    /// Complete certificate DER as base64, strongly recommended.
    pub der_base64: Option<String>,
    /// Trust purposes in this root program.
    #[serde(default)]
    pub purposes: Vec<TrustPurpose>,
    /// Included (`true`) or explicitly distrusted (`false`).
    pub trusted: bool,
    /// Optional policy distrust time as Unix seconds.
    pub distrust_after: Option<i64>,
}

/// Invalid normalized input or conflicting source records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetError {
    /// Human-readable explanation.
    pub message: String,
}

impl fmt::Display for DatasetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for DatasetError {}

#[derive(Debug)]
struct MergedAnchor {
    anchor: Anchor,
}

/// Merge normalized snapshots into the exact embedded-data schema.
pub fn compile_trust_snapshots(
    snapshots: &[TrustProgramSnapshot],
) -> Result<EmbeddedDataset, DatasetError> {
    if snapshots.is_empty() {
        return Err(error("at least one trust-program snapshot is required"));
    }
    let mut ordered = snapshots.to_vec();
    ordered.sort_by(|left, right| {
        (left.client, &left.version, &left.source.name).cmp(&(
            right.client,
            &right.version,
            &right.source.name,
        ))
    });
    let mut sources = Vec::new();
    let mut merged: BTreeMap<String, MergedAnchor> = BTreeMap::new();
    let mut generated_at = String::new();

    for snapshot in &ordered {
        if snapshot.schema != 1 {
            return Err(error(format!(
                "unsupported normalized snapshot schema {} from {}",
                snapshot.schema, snapshot.source.name
            )));
        }
        if snapshot.version.is_empty() {
            return Err(error(format!(
                "snapshot from {} has no version",
                snapshot.source.name
            )));
        }
        generated_at = generated_at.max(snapshot.generated_at.clone());
        if !sources.contains(&snapshot.source) {
            sources.push(snapshot.source.clone());
        }
        for source_anchor in &snapshot.anchors {
            let projected = project_source_anchor(source_anchor)?;
            let fingerprint = projected.fingerprint_sha256.clone();
            let entry = merged
                .entry(fingerprint.clone())
                .or_insert_with(|| MergedAnchor {
                    anchor: Anchor {
                        fingerprint_sha256: fingerprint,
                        subject: projected.subject.clone(),
                        der_base64: projected.der_base64.clone(),
                        spki_sha256: projected.spki_sha256.clone(),
                        subject_key_id: projected.subject_key_id.clone(),
                        not_before: projected.not_before,
                        not_after: projected.not_after,
                        purposes: Vec::new(),
                        memberships: Vec::new(),
                    },
                });
            merge_identity(&mut entry.anchor, &projected)?;
            entry
                .anchor
                .purposes
                .extend(source_anchor.purposes.iter().copied());
            entry
                .anchor
                .memberships
                .push(snapshot_membership(snapshot, source_anchor));
        }
    }

    sources
        .sort_by(|left, right| (&left.name, &left.revision).cmp(&(&right.name, &right.revision)));
    let mut anchors: Vec<Anchor> = merged
        .into_values()
        .map(|mut merged| {
            merged.anchor.purposes.sort_unstable();
            merged.anchor.purposes.dedup();
            merged.anchor.memberships.sort_by(compare_memberships);
            merged.anchor.memberships.dedup();
            merged.anchor
        })
        .collect();
    anchors.sort_by(|left, right| left.fingerprint_sha256.cmp(&right.fingerprint_sha256));
    let provenance = Provenance {
        schema: 1,
        generated_at,
        bootstrap_seed: false,
        notice: "Generated deterministically from normalized, pinned root-program snapshots. Version ranges describe only imported snapshots; absence remains unknown rather than distrust.".to_owned(),
        sources,
    };
    let trust = TrustStore::new(provenance, anchors).map_err(error)?;
    // Decode/parse every available DER now so broken updater output can never
    // be committed and deferred to report generation.
    trust.anchor_certificates().map_err(error)?;
    Ok(EmbeddedDataset { trust })
}

fn snapshot_membership(snapshot: &TrustProgramSnapshot, anchor: &ProgramAnchor) -> Membership {
    Membership {
        client: snapshot.client,
        first_version: None,
        last_version: None,
        snapshot_version: Some(snapshot.version.clone()),
        snapshot_date: snapshot
            .generated_at
            .split('T')
            .next()
            .filter(|date| !date.is_empty())
            .map(str::to_owned),
        purposes: anchor.purposes.clone(),
        policy_complete: true,
        trusted: anchor.trusted,
        distrust_after: anchor.distrust_after,
    }
}

fn compare_memberships(left: &Membership, right: &Membership) -> std::cmp::Ordering {
    (
        left.client,
        left.first_version.as_deref(),
        left.last_version.as_deref(),
        left.snapshot_version.as_deref(),
        left.snapshot_date.as_deref(),
    )
        .cmp(&(
            right.client,
            right.first_version.as_deref(),
            right.last_version.as_deref(),
            right.snapshot_version.as_deref(),
            right.snapshot_date.as_deref(),
        ))
}

/// Canonical pretty JSON used by generators and drift gates.
pub fn canonical_dataset_json(dataset: &EmbeddedDataset) -> Result<String, DatasetError> {
    let mut output = serde_json::to_string_pretty(dataset)
        .map_err(|error| self::error(format!("cannot serialize trust dataset: {error}")))?;
    output.push('\n');
    Ok(output)
}

fn project_source_anchor(source: &ProgramAnchor) -> Result<Anchor, DatasetError> {
    let mut projected = Anchor {
        fingerprint_sha256: normalise_fingerprint(&source.fingerprint_sha256),
        subject: source.subject.clone(),
        der_base64: source.der_base64.clone(),
        spki_sha256: None,
        subject_key_id: None,
        not_before: None,
        not_after: None,
        purposes: source.purposes.clone(),
        memberships: Vec::new(),
    };
    if let Some(encoded) = &source.der_base64 {
        let der = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|decode_error| error(format!("invalid anchor DER base64: {decode_error}")))?;
        let certificates = parse_certificates(&der).map_err(|certificate_error| {
            error(format!("invalid anchor DER: {certificate_error}"))
        })?;
        if certificates.len() != 1 {
            return Err(error(
                "one anchor record must contain exactly one certificate",
            ));
        }
        let certificate = &certificates[0];
        if !projected.fingerprint_sha256.is_empty()
            && projected.fingerprint_sha256 != certificate.fingerprint_sha256
        {
            return Err(error(format!(
                "declared fingerprint {} does not match DER {}",
                projected.fingerprint_sha256, certificate.fingerprint_sha256
            )));
        }
        projected
            .fingerprint_sha256
            .clone_from(&certificate.fingerprint_sha256);
        projected.subject.clone_from(&certificate.subject);
        projected.spki_sha256 = Some(certificate.spki_sha256.clone());
        projected
            .subject_key_id
            .clone_from(&certificate.subject_key_id);
        projected.not_before = Some(certificate.not_before);
        projected.not_after = Some(certificate.not_after);
        projected.der_base64 = Some(base64::engine::general_purpose::STANDARD.encode(&der));
    }
    if projected.fingerprint_sha256.len() != 64
        || !projected
            .fingerprint_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(error(
            "anchor requires a SHA-256 fingerprint or complete DER",
        ));
    }
    Ok(projected)
}

fn merge_identity(target: &mut Anchor, source: &Anchor) -> Result<(), DatasetError> {
    if let (Some(left), Some(right)) = (&target.der_base64, &source.der_base64)
        && left != right
    {
        return Err(error(format!(
            "conflicting DER for anchor {}",
            target.fingerprint_sha256
        )));
    }
    if target.der_base64.is_none() {
        target.der_base64.clone_from(&source.der_base64);
        target.spki_sha256.clone_from(&source.spki_sha256);
        target.subject_key_id.clone_from(&source.subject_key_id);
        target.not_before = source.not_before;
        target.not_after = source.not_after;
    }
    if target.subject.is_empty() || (!source.subject.is_empty() && source.subject < target.subject)
    {
        target.subject.clone_from(&source.subject);
    }
    Ok(())
}

fn normalise_fingerprint(fingerprint: &str) -> String {
    fingerprint
        .chars()
        .filter(char::is_ascii_hexdigit)
        .flat_map(char::to_lowercase)
        .collect()
}

fn error(message: impl Into<String>) -> DatasetError {
    DatasetError {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(name: &str) -> SourceProvenance {
        SourceProvenance {
            name: name.to_owned(),
            url: format!("https://example.test/{name}"),
            revision: "abc123".to_owned(),
            license: "CC0-1.0".to_owned(),
        }
    }

    #[test]
    fn compilation_is_deterministic_across_input_order() {
        let anchor = ProgramAnchor {
            fingerprint_sha256: "ab".repeat(32),
            subject: "CN=Root".to_owned(),
            der_base64: None,
            purposes: vec![TrustPurpose::ServerAuth],
            trusted: true,
            distrust_after: None,
        };
        let mozilla = TrustProgramSnapshot {
            schema: 1,
            client: ClientFamily::Mozilla,
            version: "128".to_owned(),
            generated_at: "2026-01-01T00:00:00Z".to_owned(),
            source: source("mozilla"),
            anchors: vec![anchor.clone()],
        };
        let chrome = TrustProgramSnapshot {
            client: ClientFamily::Chrome,
            version: "130".to_owned(),
            source: source("chrome"),
            ..mozilla.clone()
        };
        let first = compile_trust_snapshots(&[mozilla.clone(), chrome.clone()]).unwrap();
        let second = compile_trust_snapshots(&[chrome, mozilla]).unwrap();
        assert_eq!(
            canonical_dataset_json(&first),
            canonical_dataset_json(&second)
        );
        assert_eq!(first.trust.anchors[0].memberships.len(), 2);
    }

    #[test]
    fn bad_fingerprint_is_rejected() {
        let snapshot = TrustProgramSnapshot {
            schema: 1,
            client: ClientFamily::Mozilla,
            version: "1".to_owned(),
            generated_at: "2026-01-01T00:00:00Z".to_owned(),
            source: source("bad"),
            anchors: vec![ProgramAnchor {
                fingerprint_sha256: "nope".to_owned(),
                subject: String::new(),
                der_base64: None,
                purposes: Vec::new(),
                trusted: true,
                distrust_after: None,
            }],
        };
        assert!(compile_trust_snapshots(&[snapshot]).is_err());
    }
}
