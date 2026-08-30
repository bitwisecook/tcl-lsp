// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The authored half of the callback-surface gate (issue #1706).
//!
//! [`crate::callback_inventory`] projects what the registry *says*. On its own
//! that is a mirror: downgrade `fcopy -command` from a command prefix to a
//! plain value and the row simply vanishes, `cargo xtask callback-inventory`
//! writes the smaller file back out, and the check passes again. Nothing in
//! the generated pair can tell a deliberate retirement from a lost callback,
//! and nothing in it notices a documented callback the registry never
//! classified at all.
//!
//! This module is the other half: a curated, sourced list of callback surfaces
//! that Tcl, Tk, Expect, Tcllib and the supported dialects *document*, each
//! pinned to the classification the registry has to keep — kind, timing,
//! appended-argument contract, and the dialects the surface must reach. The
//! manifest is authored, never generated, and it is enforced in **both**
//! `--check` and write mode, so regenerating the inventory cannot paper over a
//! downgrade.
//!
//! Surfaces the audit found documented but *unclassified* are not dropped on
//! the floor either: they go in `known_gaps` with their evidence and tracking
//! issue, following the waiver shape `audit-option-dialects` established
//! (`KNOWN_UNSPECIFIED`). A waiver whose gap has closed fails the gate, so the
//! surface has to be promoted to a requirement rather than left half-known.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::callback_inventory::{SurfaceKind, Timing};

const MANIFEST_PATH: &str = "docs/references/command-spec/callback-surface-requirements.json";

/// The registry families the issue's coverage question spans. A row has to
/// name one of them, so a new surface cannot arrive under a private grouping
/// nobody reviews.
const FAMILIES: &[&str] = &["tcl", "tk", "expect", "tcllib", "dialect"];

/// The projection of one generated inventory row this gate reads.
///
/// Deliberately a narrow view rather than the row itself: the manifest pins
/// the *classification* of a surface, and giving it the whole row would let a
/// requirement start depending on presentation fields (forms, notes) that the
/// generator is free to reword.
pub struct SurfaceRow<'a> {
    pub owner: &'a str,
    pub location: &'a str,
    pub kind: SurfaceKind,
    pub timing: Timing,
    pub appended_arity: Option<&'a str>,
    pub dialects: &'a [String],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u8,
    requirements: Vec<Requirement>,
    known_gaps: Vec<KnownGap>,
}

/// One documented callback surface the registry must keep classified.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Requirement {
    /// Stable id for the failure message; not a registry key.
    id: String,
    /// `tcl`, `tk`, `expect`, `tcllib`, or `dialect` — the report grouping.
    family: String,
    /// The inventory row's owner (`fcopy`, `chan copy`, `ttk::treeview sort`).
    owner: String,
    /// The inventory row's location (`option -command value`, `arg[1]`, a
    /// resolver suffix such as `dynamic-command-prefix`).
    location: String,
    kind: SurfaceKind,
    timing: Timing,
    /// The appended-argument contract, spelled as the report spells it
    /// (`exactly 2`, `at least 1`, `one of [2, 3]`, `unknown`). `null` where
    /// the surface has no appended-argument axis (a body/script or dynamic
    /// row), which the gate then requires to stay absent.
    appended_arity: Option<String>,
    /// Dialects the surface must reach, as a lower bound: the merged row may
    /// list more, never fewer. This is where a version/dialect floor is
    /// pinned (`chan push` is 8.6+, `trace vdelete` is gone by 9.0).
    dialects: Vec<String>,
    /// Where the contract is documented, precisely enough to re-check.
    source: String,
    /// Interpreter or library-source evidence, where it was measured.
    oracle: Option<String>,
    /// A classification the registry cannot yet state precisely, with the
    /// issue tracking the axis. The pinned value is still enforced — this
    /// records *why* it is the conservative answer rather than the true one.
    imprecision: Option<Imprecision>,
    notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Imprecision {
    issue: String,
    reason: String,
}

/// A documented callback surface the registry does not classify yet.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownGap {
    id: String,
    owner: String,
    location: String,
    source: String,
    issue: String,
    notes: String,
}

/// Check every authored requirement and waiver against the projected rows.
///
/// Collects every disagreement before failing: a metadata change that moves
/// several rows at once should report all of them, not the first.
pub fn enforce(root: &Path, rows: &[SurfaceRow<'_>]) -> Result<()> {
    let path = root.join(MANIFEST_PATH);
    let manifest: Manifest = serde_json::from_str(
        &fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))?;
    if manifest.schema_version != 1 {
        bail!(
            "unsupported callback coverage manifest schema {}",
            manifest.schema_version
        );
    }
    validate_manifest(&manifest)?;

    let mut failures: Vec<String> = Vec::new();
    for requirement in &manifest.requirements {
        check_requirement(requirement, rows, &mut failures);
    }
    for gap in &manifest.known_gaps {
        check_gap(gap, rows, &mut failures);
    }
    if failures.is_empty() {
        return Ok(());
    }
    bail!(
        "{} authored callback-coverage failure(s) against {MANIFEST_PATH}:\n{}\n\nSee \
         docs/design/contracts/callback-surface-inventory.md — a surface that genuinely \
         changed is re-pinned in the manifest in the same commit as the registry edit; a \
         surface that lost its declaration is a regression, not a manifest update.",
        failures.len(),
        failures.join("\n")
    );
}

/// Reject a manifest that cannot mean what it says before comparing anything:
/// a duplicate id, an unsourced row, or a dialect name no environment answers
/// to would otherwise weaken the gate silently.
fn validate_manifest(manifest: &Manifest) -> Result<()> {
    let mut ids = BTreeSet::new();
    for id in manifest
        .requirements
        .iter()
        .map(|row| &row.id)
        .chain(manifest.known_gaps.iter().map(|row| &row.id))
    {
        if !ids.insert(id) {
            bail!("duplicate callback coverage id {id}");
        }
    }
    for requirement in &manifest.requirements {
        if requirement.source.trim().is_empty() {
            bail!("requirement {} has no source", requirement.id);
        }
        if !FAMILIES.contains(&requirement.family.as_str()) {
            bail!(
                "requirement {} names family {}, which is not one of {FAMILIES:?}",
                requirement.id,
                requirement.family
            );
        }
        if requirement.dialects.is_empty() {
            bail!(
                "requirement {} names no dialect floor; a requirement that pins no \
                 dialect cannot catch a surface that lost one",
                requirement.id
            );
        }
        for dialect in &requirement.dialects {
            if crate::environment::known_profile_for_dialect(dialect).is_none() {
                bail!(
                    "requirement {} names unknown dialect {dialect}",
                    requirement.id
                );
            }
        }
    }
    for gap in &manifest.known_gaps {
        if gap.issue.trim().is_empty() || gap.source.trim().is_empty() {
            bail!(
                "known gap {} needs both a source and a tracking issue",
                gap.id
            );
        }
    }
    Ok(())
}

impl Requirement {
    /// The surface as the failure message names it.
    fn surface(&self) -> String {
        format!(
            "{} ({} `{} {}`)",
            self.id, self.family, self.owner, self.location
        )
    }

    /// Everything the reader needs to decide whether the registry or the
    /// manifest is wrong: what documents the contract, what measured it, why
    /// the pinned answer is the conservative one, and the authoring note.
    fn context(&self) -> String {
        let mut parts = vec![format!("documented by {}", self.source)];
        if let Some(oracle) = &self.oracle {
            parts.push(format!("measured as {oracle}"));
        }
        if let Some(imprecision) = &self.imprecision {
            parts.push(format!(
                "pinned conservatively pending {} ({})",
                imprecision.issue, imprecision.reason
            ));
        }
        if !self.notes.trim().is_empty() {
            parts.push(self.notes.clone());
        }
        parts.join("; ")
    }
}

fn check_requirement(
    requirement: &Requirement,
    rows: &[SurfaceRow<'_>],
    failures: &mut Vec<String>,
) {
    // One owner/location can carry more than one row: a name with both a Tcl
    // and a dialect spec (`proc`, `after`) is projected once per spec, and the
    // rows differ in provenance and reach. Judge the requirement against the
    // candidate that covers most of the dialect floor it names, so a Tcl row
    // never answers for an iRules one.
    let Some(row) = rows
        .iter()
        .filter(|row| row.owner == requirement.owner && row.location == requirement.location)
        .max_by_key(|row| covered_dialects(row, requirement))
    else {
        failures.push(format!(
            "- {} has no classification. The registry declares no executable role there — \
             either it was downgraded to a plain value/flag, or the surface was never \
             classified. {}",
            requirement.surface(),
            requirement.context()
        ));
        return;
    };
    if row.kind != requirement.kind {
        failures.push(format!(
            "- {} is classified {} but must be {}. {}",
            requirement.surface(),
            kind_spelling(row.kind),
            kind_spelling(requirement.kind),
            requirement.context()
        ));
    }
    if row.timing != requirement.timing {
        failures.push(format!(
            "- {} runs {} but must be {}. {}",
            requirement.surface(),
            timing_spelling(row.timing),
            timing_spelling(requirement.timing),
            requirement.context()
        ));
    }
    if row.appended_arity != requirement.appended_arity.as_deref() {
        failures.push(format!(
            "- {} appends {} but must append {}. {}",
            requirement.surface(),
            row.appended_arity.unwrap_or("nothing declared"),
            requirement.appended_arity.as_deref().unwrap_or("nothing"),
            requirement.context()
        ));
    }
    let missing: Vec<&str> = requirement
        .dialects
        .iter()
        .filter(|dialect| !row.dialects.iter().any(|have| have == *dialect))
        .map(String::as_str)
        .collect();
    if !missing.is_empty() {
        failures.push(format!(
            "- {} no longer reaches {}. {}",
            requirement.surface(),
            missing.join(", "),
            requirement.context()
        ));
    }
}

/// How much of a requirement's dialect floor one candidate row reaches.
fn covered_dialects(row: &SurfaceRow<'_>, requirement: &Requirement) -> usize {
    requirement
        .dialects
        .iter()
        .filter(|dialect| row.dialects.iter().any(|have| have == *dialect))
        .count()
}

fn check_gap(gap: &KnownGap, rows: &[SurfaceRow<'_>], failures: &mut Vec<String>) {
    if rows
        .iter()
        .any(|row| row.owner == gap.owner && row.location == gap.location)
    {
        failures.push(format!(
            "- {}: the waived gap `{} {}` is closed — the inventory now classifies it. \
             Promote it to a requirement (pinning the classification and its source) and \
             drop the waiver; {} tracks the gap, documented by {} ({})",
            gap.id, gap.owner, gap.location, gap.issue, gap.source, gap.notes
        ));
    }
}

fn kind_spelling(kind: SurfaceKind) -> &'static str {
    match kind {
        SurfaceKind::CommandPrefix => "a command prefix",
        SurfaceKind::BodyScript => "a script body",
        SurfaceKind::ReferenceOnly => "reference-only",
        SurfaceKind::Dynamic => "dynamic (resolver-decided)",
        SurfaceKind::ExternalDispatch => "external dispatch",
    }
}

fn timing_spelling(timing: Timing) -> &'static str {
    match timing {
        Timing::SameInvocation => "in the same invocation",
        Timing::Deferred => "deferred",
        Timing::ReferenceOnly => "never (reference-only)",
        Timing::Dynamic => "at a resolver-decided time",
        Timing::BlockingExternalRpc => "as a blocking external RPC",
        Timing::FireAndForgetExternal => "as fire-and-forget external dispatch",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row<'a>(owner: &'a str, location: &'a str, dialects: &'a [String]) -> SurfaceRow<'a> {
        SurfaceRow {
            owner,
            location,
            kind: SurfaceKind::CommandPrefix,
            timing: Timing::Deferred,
            appended_arity: Some("at least 1"),
            dialects,
        }
    }

    fn requirement() -> Requirement {
        Requirement {
            id: "tcl/fcopy/-command".to_owned(),
            family: "tcl".to_owned(),
            owner: "fcopy".to_owned(),
            location: "option -command value".to_owned(),
            kind: SurfaceKind::CommandPrefix,
            timing: Timing::Deferred,
            appended_arity: Some("at least 1".to_owned()),
            dialects: vec!["tcl9.0".to_owned()],
            source: "Tcl fcopy(n)".to_owned(),
            oracle: None,
            imprecision: None,
            notes: String::new(),
        }
    }

    /// The regression this gate exists for: a callback option downgraded to a
    /// plain value loses its row, and a lost row is a failure — not a smaller
    /// file to regenerate.
    #[test]
    fn a_downgraded_callback_option_fails_its_requirement() {
        let dialects = vec!["tcl9.0".to_owned()];
        let mut failures = Vec::new();
        check_requirement(&requirement(), &[], &mut failures);
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(
            failures[0].contains("has no classification"),
            "{failures:?}"
        );

        let mut kept = Vec::new();
        check_requirement(
            &requirement(),
            &[row("fcopy", "option -command value", &dialects)],
            &mut kept,
        );
        assert!(kept.is_empty(), "{kept:?}");
    }

    /// A kept row whose contract slipped — the callback still exists but its
    /// arity, timing, kind, or dialect reach changed — fails just as loudly.
    #[test]
    fn a_weakened_contract_fails_its_requirement() {
        let dialects = vec!["tcl9.0".to_owned()];
        let mut failures = Vec::new();
        let weakened = SurfaceRow {
            kind: SurfaceKind::BodyScript,
            timing: Timing::SameInvocation,
            appended_arity: None,
            ..row("fcopy", "option -command value", &dialects)
        };
        check_requirement(&requirement(), &[weakened], &mut failures);
        assert_eq!(failures.len(), 3, "{failures:?}");

        let mut narrowed = Vec::new();
        let elsewhere = vec!["tcl8.6".to_owned()];
        check_requirement(
            &requirement(),
            &[row("fcopy", "option -command value", &elsewhere)],
            &mut narrowed,
        );
        assert_eq!(narrowed.len(), 1, "{narrowed:?}");
        assert!(
            narrowed[0].contains("no longer reaches tcl9.0"),
            "{narrowed:?}"
        );
    }

    /// A waiver is a temporary record, not a parking space: once the surface
    /// is classified the waiver must go, or the gate would keep excusing a
    /// row it is now able to pin.
    #[test]
    fn a_closed_gap_fails_its_waiver() {
        let gap = KnownGap {
            id: "tcl/http::config/-proxyfilter".to_owned(),
            owner: "http::config".to_owned(),
            location: "option -proxyfilter value".to_owned(),
            source: "Tcl http(n)".to_owned(),
            issue: "#1706".to_owned(),
            notes: String::new(),
        };
        let dialects = vec!["tcl9.0".to_owned()];
        let mut open = Vec::new();
        check_gap(&gap, &[], &mut open);
        assert!(open.is_empty(), "{open:?}");

        let mut closed = Vec::new();
        check_gap(
            &gap,
            &[row("http::config", "option -proxyfilter value", &dialects)],
            &mut closed,
        );
        assert_eq!(closed.len(), 1, "{closed:?}");
        assert!(closed[0].contains("is closed"), "{closed:?}");
    }
}
