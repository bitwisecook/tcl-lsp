// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Version *ranges*, from several released snapshots of one package.
//!
//! [`crate::infer::import_package`] reads one snapshot and can therefore say
//! only what the package looks like *now*. That is not a range claim: stamping
//! `introduced_version` with the snapshot's own `package provide` version says
//! every command arrived in the newest release, which is almost never true.
//!
//! Give this module the sources of *N* releases and it derives the ranges the
//! releases actually witness. Nothing is guessed — a lifecycle field is only
//! written when two snapshots disagree about whether something exists, which
//! is exactly the evidence [`tcl_registry::lifecycle`] is defined against:
//! `introduced` is the first release where an entity exists, `retired` is the
//! first release where it no longer does (an **exclusive** bound), and an
//! absent field means unbounded.
//!
//! ## What is derived, and from what
//!
//! 1. **Snapshot order** — labels are sorted with
//!    [`tcl_registry::version::compare`], not trusted in caller order; a
//!    disagreement is a warning, and a duplicate label is an error entry in
//!    the warnings. A snapshot whose own `package provide` version contradicts
//!    its label is a warning too: the label wins, because it is the release the
//!    caller says these sources are.
//! 2. **Base shape** — each snapshot is drafted independently by the ordinary
//!    importer, and a command's merged draft is the draft from the **newest**
//!    snapshot defining it. Arity, roles, hover, options and forms therefore
//!    describe the latest reality.
//! 3. **`introduced_version`** — the first snapshot defining the command, but
//!    only when that is a *definitive* appearance: either an earlier snapshot
//!    lacks the command, or the caller declared the history complete
//!    ([`VersionedImportOptions::complete_history`]) so the earliest snapshot
//!    really is the first release. Otherwise the field is left unset and a note
//!    says the introduction is unknown.
//! 4. **`retired_version`** — the first snapshot where a previously-present
//!    command is gone. Exclusive-bound semantics line up exactly: the entity
//!    does not exist in that release.
//! 5. **`deprecated_version`** — never derived. Deprecation is not a structural
//!    fact, so the importer only reports the obvious textual evidence: the
//!    first snapshot whose doc comment says "deprecated" becomes a *suggested*
//!    version recorded in the notes and nowhere else.
//! 6. **Option rows** — diffed by option name across the snapshots in which the
//!    command exists, and gated the same way. An option that later disappears
//!    is kept in the merged row list carrying its `retired_version`, because
//!    dropping the row would drop the fact.
//! 7. **Closed value sets** — diffed by membership. On a subcommand-shaped
//!    draft the result is written to `versioned_arg_values`, the draft
//!    vocabulary's existing per-value gate. Command-level values have no field
//!    to hold a gate yet, so those become structured notes instead (below).
//! 8. **Arity and role changes** — reported, never invented: the registry
//!    cannot express a versioned arity, so a change is a note naming both
//!    releases and both shapes.
//!
//! Any pattern of present → absent → present leaves the lifecycle unbounded
//! and raises a warning naming the gap: a range cannot describe a hole, and
//! guessing which end to believe would be a fabrication.
//!
//! ## `version-gate:` notes
//!
//! Facts the draft model cannot yet carry as a field are emitted as notes with
//! the stable [`VERSION_GATE_NOTE`] prefix, so a later pass can mechanically
//! upgrade them into fields once the registry extension lands:
//!
//! ```text
//! version-gate: command=encode arg=0 value=utf-8 introduced=1.2
//! version-gate: command=encode arg=0 value=latin1 retired=1.2
//! ```
//!
//! The fields are space-separated `key=value` pairs in that order;
//! `introduced` and `retired` appear only when derived.
//!
//! Every derived fact carries its evidence in [`crate::infer::Inferred::notes`],
//! the same discipline the single-snapshot importer keeps — the studio shows
//! the reasoning next to the guess so an author can overrule it.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use tcl_registry::version::compare;

use crate::draft::Draft;
use crate::infer::{Import, Inferred, SourceFile, import_package};
use crate::render_rs::rust_string;

/// The prefix every structured note carries when it records a version fact the
/// draft model has no field for yet.
pub const VERSION_GATE_NOTE: &str = "version-gate:";

/// One released version of a package, as its own set of sources.
#[derive(Debug, Clone)]
pub struct VersionedSnapshot {
    /// The release label, as `package require` would spell it (`1.2`).
    pub version: String,
    /// The package's files at that release.
    pub files: Vec<SourceFile>,
}

/// What the caller knows about the snapshots beyond the sources themselves.
#[derive(Debug, Clone, Copy, Default)]
pub struct VersionedImportOptions {
    /// Whether the snapshots cover **every** release of the package.
    ///
    /// With a complete history the earliest snapshot is the package's first
    /// release, so a command present there really was introduced there. With a
    /// partial history the same observation only means "present as far back as
    /// we looked", which is not an introduction.
    pub complete_history: bool,
}

/// The result of importing several releases of one package.
#[derive(Debug, Clone, Default)]
pub struct VersionedImport {
    /// Package name from `package provide`, when any snapshot declares one.
    pub package: Option<String>,
    /// The release labels analysed, in ascending version order.
    pub versions: Vec<String>,
    /// One merged draft per command seen in any release, in name order.
    pub commands: Vec<Inferred>,
    /// Ordering problems, snapshots that produced nothing, and every fact the
    /// releases contradict each other about.
    pub warnings: Vec<String>,
}

impl VersionedImport {
    /// The JSON a wasm or CLI layer marshals, mirroring
    /// [`Import::to_json`](crate::infer::Import::to_json).
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "package": self.package,
            "versions": self.versions.clone(),
            "commands": self.commands.iter().map(Inferred::to_json).collect::<Vec<_>>(),
            "warnings": self.warnings.clone(),
        })
    }
}

/// Analyse several releases of one package and derive each command's version
/// range from what the releases witness.
///
/// `dialect` is a registry dialect name, used for every snapshot — a package's
/// releases are analysed as the same language.
#[must_use]
pub fn import_package_versions(
    snapshots: &[VersionedSnapshot],
    dialect: &str,
    options: &VersionedImportOptions,
) -> VersionedImport {
    let mut warnings: Vec<String> = Vec::new();
    let order = order_snapshots(snapshots, &mut warnings);
    let releases = analyse(snapshots, &order, dialect, &mut warnings);
    let versions: Vec<String> = releases.iter().map(|r| r.version.clone()).collect();
    let package = releases.iter().find_map(|r| r.import.package.clone());

    let mut index: BTreeMap<&str, BTreeMap<usize, &Inferred>> = BTreeMap::new();
    for (at, release) in releases.iter().enumerate() {
        for command in &release.import.commands {
            index
                .entry(command.name.as_str())
                .or_default()
                .insert(at, command);
        }
    }

    let history = History {
        versions: &versions,
        complete: options.complete_history,
    };
    let mut commands: Vec<Inferred> = Vec::with_capacity(index.len());
    for (name, by_release) in index {
        let (merged, raised) = history.merge(name, &by_release);
        commands.push(merged);
        warnings.extend(raised);
    }

    VersionedImport {
        package,
        versions,
        commands,
        warnings,
    }
}

/// One analysed snapshot: the label we trust, and what the ordinary importer
/// made of that release's sources.
struct Release {
    version: String,
    import: Import,
}

/// The snapshot indices to analyse, in ascending version order, with the
/// duplicates dropped.
fn order_snapshots(snapshots: &[VersionedSnapshot], warnings: &mut Vec<String>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..snapshots.len()).collect();
    order.sort_by(|a, b| compare(&snapshots[*a].version, &snapshots[*b].version));
    if order.iter().enumerate().any(|(at, index)| at != *index) {
        warnings.push(format!(
            "snapshots were not given in ascending version order; analysed as {}",
            labels(snapshots, &order)
        ));
    }

    let mut kept: Vec<usize> = Vec::with_capacity(order.len());
    for index in order {
        let duplicate = kept.last().is_some_and(|last| {
            compare(&snapshots[*last].version, &snapshots[index].version) == Ordering::Equal
        });
        if duplicate {
            warnings.push(format!(
                "error: duplicate snapshot version `{}`; only the first is analysed",
                snapshots[index].version
            ));
            continue;
        }
        kept.push(index);
    }
    kept
}

fn labels(snapshots: &[VersionedSnapshot], order: &[usize]) -> String {
    order
        .iter()
        .map(|index| snapshots[*index].version.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Draft every snapshot independently, checking each one's own
/// `package provide` version against the label the caller gave it.
fn analyse(
    snapshots: &[VersionedSnapshot],
    order: &[usize],
    dialect: &str,
    warnings: &mut Vec<String>,
) -> Vec<Release> {
    let mut releases = Vec::with_capacity(order.len());
    for index in order {
        let snapshot = &snapshots[*index];
        let import = import_package(&snapshot.files, dialect);
        if let Some(declared) = &import.version
            && compare(declared, &snapshot.version) != Ordering::Equal
        {
            warnings.push(format!(
                "snapshot labelled {} declares `package provide … {declared}`; \
                 trusting the label",
                snapshot.version
            ));
        }
        for warning in &import.warnings {
            warnings.push(format!("{}: {warning}", snapshot.version));
        }
        releases.push(Release {
            version: snapshot.version.clone(),
            import,
        });
    }
    releases
}

/// The releases, and what the caller claims about their completeness.
struct History<'a> {
    versions: &'a [String],
    complete: bool,
}

/// Notes and warnings gathered while merging one command.
#[derive(Debug, Default)]
struct Evidence {
    notes: Vec<String>,
    warnings: Vec<String>,
}

/// The bounds a presence pattern witnesses; both absent means unbounded.
#[derive(Debug, Clone, Default)]
struct Lifespan {
    introduced: Option<String>,
    retired: Option<String>,
}

/// One entity's presence, and the container whose releases it lives inside.
struct RangeInput<'a> {
    /// How the entity is named in a note.
    subject: String,
    /// Release indices where the entity exists, ascending.
    present: &'a [usize],
    /// Release indices where the *container* exists, ascending. A command's
    /// container is the whole analysed history; an option's is the releases
    /// defining its command.
    universe: &'a [usize],
    /// The file each release was drafted from, for the evidence line.
    files: &'a BTreeMap<usize, String>,
    /// Whether presence in the first release of `universe` is itself an
    /// introduction.
    first_definitive: bool,
    /// Whether an unknowable introduction is worth a note of its own.
    report_unknown: bool,
}

/// One per-value version gate.
struct Gate {
    index: u64,
    value: String,
    span: Lifespan,
}

/// One merged row of a named table, with the releases it appeared in.
struct MergedRow {
    name: String,
    row: Value,
    present: Vec<usize>,
}

/// Which named table is being merged, and how to talk about it.
struct RowMerge<'a> {
    /// Draft key holding the rows (`options`, `subcommands`).
    key: &'a str,
    /// What one row is called in a note.
    noun: &'a str,
    /// The command the rows belong to.
    command: &'a str,
}

impl History<'_> {
    /// Merge one command's per-release drafts into a single draft whose
    /// lifecycle fields the releases actually witness.
    fn merge(
        &self,
        name: &str,
        by_release: &BTreeMap<usize, &Inferred>,
    ) -> (Inferred, Vec<String>) {
        let present: Vec<usize> = by_release.keys().copied().collect();
        let files: BTreeMap<usize, String> = by_release
            .iter()
            .map(|(at, inferred)| (*at, file_of(inferred)))
            .collect();
        let newest_at = *present
            .last()
            .expect("an indexed command appears in at least one release");
        let newest = by_release[&newest_at];

        let mut draft = newest.draft.clone();
        let mut evidence = Evidence::default();
        evidence.notes.push(format!(
            "seen in {} of {} analysed release(s): {}",
            present.len(),
            self.versions.len(),
            self.span_list(&present)
        ));
        evidence.notes.push(format!(
            "shape taken from {}, the newest analysed release defining `{name}`",
            self.versions[newest_at]
        ));

        let whole: Vec<usize> = (0..self.versions.len()).collect();
        let span = self.derive(
            &RangeInput {
                subject: format!("`{name}`"),
                present: &present,
                universe: &whole,
                files: &files,
                first_definitive: self.complete,
                report_unknown: true,
            },
            &mut evidence,
        );
        draft.insert(
            "introduced_version".into(),
            version_value(span.introduced.as_deref()),
        );
        draft.insert(
            "retired_version".into(),
            version_value(span.retired.as_deref()),
        );

        if let Some(note) = self.deprecation_note(name, by_release) {
            evidence.notes.push(note);
        }
        self.shape_notes(&present, by_release, arity_label, "arity", &mut evidence);
        self.shape_notes(
            &present,
            by_release,
            role_label,
            "argument roles",
            &mut evidence,
        );
        self.merge_tables(
            name,
            by_release,
            &present,
            &files,
            &mut draft,
            &mut evidence,
        );

        let mut notes = newest.notes.clone();
        notes.append(&mut evidence.notes);
        notes.extend(evidence.warnings.iter().map(|w| format!("warning: {w}")));
        // Every contradiction already names its own subject, so the import-level
        // copy needs no prefix to be traceable back to this command.
        let warnings = evidence.warnings;
        (
            Inferred {
                name: name.to_owned(),
                draft,
                notes,
            },
            warnings,
        )
    }

    /// Options, subcommands, and the closed value sets hanging off both.
    fn merge_tables(
        &self,
        name: &str,
        by_release: &BTreeMap<usize, &Inferred>,
        present: &[usize],
        files: &BTreeMap<usize, String>,
        draft: &mut Draft,
        out: &mut Evidence,
    ) {
        let options = self.merge_named_rows(
            &RowMerge {
                key: "options",
                noun: "option",
                command: name,
            },
            by_release,
            present,
            files,
            out,
        );
        draft.insert(
            "options".into(),
            Value::Array(options.into_iter().map(|merged| merged.row).collect()),
        );

        let subcommands = self.merge_named_rows(
            &RowMerge {
                key: "subcommands",
                noun: "subcommand",
                command: name,
            },
            by_release,
            present,
            files,
            out,
        );
        let gated = self.gate_subcommands(name, subcommands, by_release, files, out);
        draft.insert("subcommands".into(), Value::Array(gated));

        self.command_value_gates(name, by_release, present, files, out);
    }

    /// The bounds `input`'s presence pattern witnesses, with the evidence for
    /// each one — and a warning instead of a bound when presence has a hole.
    fn derive(&self, input: &RangeInput<'_>, out: &mut Evidence) -> Lifespan {
        let positions: Vec<usize> = input
            .present
            .iter()
            .filter_map(|at| input.universe.iter().position(|other| other == at))
            .collect();
        let (Some(&first), Some(&last)) = (positions.first(), positions.last()) else {
            return Lifespan::default();
        };

        let gaps: Vec<usize> = (first..=last)
            .filter(|position| !positions.contains(position))
            .map(|position| input.universe[position])
            .collect();
        if !gaps.is_empty() {
            out.warnings.push(format!(
                "{} is present in {}, absent in {}, then present again in {} — \
                 lifecycle left unbounded",
                input.subject,
                self.versions[input.universe[first]],
                self.span_list(&gaps),
                self.versions[input.universe[last]],
            ));
            return Lifespan::default();
        }

        let mut span = Lifespan::default();
        let introduced = &self.versions[input.universe[first]];
        if first > 0 {
            out.notes.push(format!(
                "{} introduced {introduced} — absent in {}, present in {introduced} ({})",
                input.subject,
                self.versions[input.universe[first - 1]],
                file_at(input.files, input.universe[first]),
            ));
            span.introduced = Some(introduced.clone());
        } else if input.first_definitive {
            out.notes.push(format!(
                "{} introduced {introduced} — present in {introduced}, the package's \
                 first release (the caller declared a complete history)",
                input.subject
            ));
            span.introduced = Some(introduced.clone());
        } else if input.report_unknown {
            out.notes.push(format!(
                "{} is present in earliest analysed version {introduced}; \
                 introduction unknown",
                input.subject
            ));
        }

        if let Some(after) = input.universe.get(last + 1) {
            let retired = &self.versions[*after];
            out.notes.push(format!(
                "{} retired {retired} — present in {}, absent in {retired} ({})",
                input.subject,
                self.versions[input.universe[last]],
                file_at(input.files, input.universe[last]),
            ));
            span.retired = Some(retired.clone());
        }
        span
    }

    /// Diff one named table across the releases, gating each row and keeping
    /// rows that later releases dropped so the retirement survives.
    fn merge_named_rows(
        &self,
        merge: &RowMerge<'_>,
        by_release: &BTreeMap<usize, &Inferred>,
        present: &[usize],
        files: &BTreeMap<usize, String>,
        out: &mut Evidence,
    ) -> Vec<MergedRow> {
        let per_release: BTreeMap<usize, Vec<(String, Value)>> = present
            .iter()
            .map(|at| (*at, named_rows(by_release[at], merge.key)))
            .collect();
        let newest_at = *present
            .last()
            .expect("an indexed command appears in at least one release");

        let mut order: Vec<String> = per_release[&newest_at]
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        let dropped: BTreeSet<String> = per_release
            .values()
            .flatten()
            .map(|(name, _)| name.clone())
            .filter(|name| !order.contains(name))
            .collect();
        order.extend(dropped);

        let mut merged = Vec::with_capacity(order.len());
        for name in order {
            let row_present: Vec<usize> = present
                .iter()
                .copied()
                .filter(|at| per_release[at].iter().any(|(other, _)| *other == name))
                .collect();
            let Some(latest) = row_present.last() else {
                continue;
            };
            let Some((_, template)) = per_release[latest].iter().find(|(other, _)| *other == name)
            else {
                continue;
            };
            let mut row = template.clone();
            let span = self.derive(
                &RangeInput {
                    subject: format!("{} `{name}` of `{}`", merge.noun, merge.command),
                    present: &row_present,
                    universe: present,
                    files,
                    first_definitive: false,
                    report_unknown: false,
                },
                out,
            );
            if let Some(object) = row.as_object_mut() {
                object.insert(
                    "introduced_version".into(),
                    version_value(span.introduced.as_deref()),
                );
                object.insert(
                    "retired_version".into(),
                    version_value(span.retired.as_deref()),
                );
            }
            merged.push(MergedRow {
                name,
                row,
                present: row_present,
            });
        }
        merged
    }

    /// Write each subcommand's per-value gates into `versioned_arg_values`,
    /// the draft vocabulary's existing spelling for them.
    fn gate_subcommands(
        &self,
        command: &str,
        rows: Vec<MergedRow>,
        by_release: &BTreeMap<usize, &Inferred>,
        files: &BTreeMap<usize, String>,
        out: &mut Evidence,
    ) -> Vec<Value> {
        rows.into_iter()
            .map(|mut merged| {
                let per_release: BTreeMap<usize, BTreeMap<u64, BTreeSet<String>>> = merged
                    .present
                    .iter()
                    .filter_map(|at| {
                        let row = row_named(by_release[at], "subcommands", &merged.name)?;
                        Some((*at, value_sets(row.get("arg_values"))))
                    })
                    .collect();
                let gates = self.diff_values(
                    &format!("`{command} {}`", merged.name),
                    &per_release,
                    &merged.present,
                    files,
                    out,
                );
                if !gates.is_empty()
                    && let Some(object) = merged.row.as_object_mut()
                {
                    object.insert(
                        "versioned_arg_values".into(),
                        json!(gate_expression(&gates)),
                    );
                }
                merged.row
            })
            .collect()
    }

    /// Command-level closed value sets, which no draft field can gate yet, so
    /// they leave a [`VERSION_GATE_NOTE`] note a later pass can lift.
    fn command_value_gates(
        &self,
        command: &str,
        by_release: &BTreeMap<usize, &Inferred>,
        present: &[usize],
        files: &BTreeMap<usize, String>,
        out: &mut Evidence,
    ) {
        let per_release: BTreeMap<usize, BTreeMap<u64, BTreeSet<String>>> = present
            .iter()
            .map(|at| (*at, value_sets(by_release[at].draft.get("arg_values"))))
            .collect();
        let gates = self.diff_values(&format!("`{command}`"), &per_release, present, files, out);
        for gate in &gates {
            out.notes.push(gate_note(command, gate));
        }
    }

    /// Diff closed value-set membership per argument index, one gate per value
    /// whose availability the releases actually bound.
    fn diff_values(
        &self,
        subject: &str,
        per_release: &BTreeMap<usize, BTreeMap<u64, BTreeSet<String>>>,
        universe: &[usize],
        files: &BTreeMap<usize, String>,
        out: &mut Evidence,
    ) -> Vec<Gate> {
        let mut indices: BTreeSet<u64> = BTreeSet::new();
        for sets in per_release.values() {
            indices.extend(sets.keys().copied());
        }

        let mut gates = Vec::new();
        for index in indices {
            let known: Vec<usize> = universe
                .iter()
                .copied()
                .filter(|at| {
                    per_release
                        .get(at)
                        .is_some_and(|sets| sets.contains_key(&index))
                })
                .collect();
            if known.len() < 2 {
                continue;
            }
            let mut values: BTreeSet<String> = BTreeSet::new();
            for at in &known {
                values.extend(per_release[at][&index].iter().cloned());
            }
            for value in values {
                let present: Vec<usize> = known
                    .iter()
                    .copied()
                    .filter(|at| per_release[at][&index].contains(&value))
                    .collect();
                let span = self.derive(
                    &RangeInput {
                        subject: format!("value `{value}` of {subject} argument {index}"),
                        present: &present,
                        universe: &known,
                        files,
                        first_definitive: false,
                        report_unknown: false,
                    },
                    out,
                );
                if span.introduced.is_some() || span.retired.is_some() {
                    gates.push(Gate { index, value, span });
                }
            }
        }
        gates
    }

    /// The first release whose doc comment calls the command deprecated, as a
    /// suggestion only — the draft keeps `deprecated_version` unset.
    fn deprecation_note(
        &self,
        name: &str,
        by_release: &BTreeMap<usize, &Inferred>,
    ) -> Option<String> {
        let (at, _) = by_release
            .iter()
            .find(|(_, inferred)| doc_says_deprecated(inferred))?;
        Some(format!(
            "suggested deprecated_version {} — the doc comment on `{name}` says \
             \"deprecated\" from that release; left out of the draft, deprecation is \
             not structurally derivable",
            self.versions[*at]
        ))
    }

    /// Report a shape that changed between releases. The merged draft keeps the
    /// newest shape; the registry has no versioned arity, so the older ones are
    /// evidence and nothing more.
    fn shape_notes(
        &self,
        present: &[usize],
        by_release: &BTreeMap<usize, &Inferred>,
        label: fn(&Draft) -> String,
        what: &str,
        out: &mut Evidence,
    ) {
        let mut runs: Vec<(String, usize, usize)> = Vec::new();
        for at in present {
            let shape = label(&by_release[at].draft);
            match runs.last_mut() {
                Some(run) if run.0 == shape => run.2 = *at,
                _ => runs.push((shape, *at, *at)),
            }
        }
        for pair in runs.windows(2) {
            let [previous, current] = pair else { continue };
            out.notes.push(format!(
                "{what} {} since {}; was {} in {}",
                current.0,
                self.versions[current.1],
                previous.0,
                self.span_text(previous.1, previous.2),
            ));
        }
    }

    /// `1.0` for one release, `1.0–1.3` for a run of them.
    fn span_text(&self, first: usize, last: usize) -> String {
        if first == last {
            self.versions[first].clone()
        } else {
            format!("{}–{}", self.versions[first], self.versions[last])
        }
    }

    fn span_list(&self, indices: &[usize]) -> String {
        indices
            .iter()
            .map(|at| self.versions[*at].as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The source file a draft was inferred from, recovered from its hover source.
fn file_of(inferred: &Inferred) -> String {
    inferred
        .draft
        .get("hover")
        .and_then(|hover| hover.get("source"))
        .and_then(Value::as_str)
        .and_then(|source| source.strip_prefix("inferred from "))
        .unwrap_or("the package sources")
        .to_owned()
}

fn file_at(files: &BTreeMap<usize, String>, at: usize) -> &str {
    files.get(&at).map_or("the package sources", String::as_str)
}

fn doc_says_deprecated(inferred: &Inferred) -> bool {
    inferred
        .draft
        .get("hover")
        .and_then(|hover| hover.get("summary"))
        .and_then(Value::as_str)
        .is_some_and(|doc| doc.to_lowercase().contains("deprecated"))
}

/// The named rows under `key`, in the order the draft holds them.
fn named_rows(inferred: &Inferred, key: &str) -> Vec<(String, Value)> {
    inferred
        .draft
        .get(key)
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |rows| {
            rows.iter()
                .filter_map(|row| Some((row.get("name")?.as_str()?.to_owned(), row.clone())))
                .collect()
        })
}

fn row_named<'a>(inferred: &'a Inferred, key: &str, name: &str) -> Option<&'a Value> {
    inferred
        .draft
        .get(key)?
        .as_array()?
        .iter()
        .find(|row| row.get("name").and_then(Value::as_str) == Some(name))
}

/// The closed value sets an `arg_values` table declares, by argument index.
/// An empty set is dropped: it says the heuristic found nothing, not that the
/// argument accepts nothing.
fn value_sets(arg_values: Option<&Value>) -> BTreeMap<u64, BTreeSet<String>> {
    let mut sets = BTreeMap::new();
    let Some(rows) = arg_values.and_then(Value::as_array) else {
        return sets;
    };
    for row in rows {
        let Some(index) = row.get("index").and_then(Value::as_u64) else {
            continue;
        };
        let values: BTreeSet<String> =
            row.get("values")
                .and_then(Value::as_array)
                .map_or_else(BTreeSet::new, |values| {
                    values
                        .iter()
                        .filter_map(|value| Some(value.get("value")?.as_str()?.to_owned()))
                        .collect()
                });
        if !values.is_empty() {
            sets.insert(index, values);
        }
    }
    sets
}

fn version_value(version: Option<&str>) -> Value {
    version.map_or(Value::Null, |version| json!(version))
}

fn gate_note(command: &str, gate: &Gate) -> String {
    let introduced = gate
        .span
        .introduced
        .as_ref()
        .map_or_else(String::new, |version| format!(" introduced={version}"));
    let retired = gate
        .span
        .retired
        .as_ref()
        .map_or_else(String::new, |version| format!(" retired={version}"));
    format!(
        "{VERSION_GATE_NOTE} command={command} arg={} value={}{introduced}{retired}",
        gate.index, gate.value
    )
}

/// The `versioned_arg_values` Rust expression, spelled exactly as
/// [`crate::draft`] renders one so the existing renderers can read it back.
fn gate_expression(gates: &[Gate]) -> String {
    let items: Vec<String> = gates
        .iter()
        .map(|gate| {
            format!(
                "VersionedArgValue {{ index: {}, value: {}, lifecycle: Lifecycle {{ \
                 introduced: {}, deprecated: None, retired: {}, deprecation_fix: None }} }}",
                gate.index,
                rust_string(&gate.value),
                option_expression(gate.span.introduced.as_deref()),
                option_expression(gate.span.retired.as_deref()),
            )
        })
        .collect();
    format!("&[{}]", items.join(", "))
}

fn option_expression(version: Option<&str>) -> String {
    version.map_or_else(
        || "None".to_owned(),
        |version| format!("Some({})", rust_string(version)),
    )
}

/// A draft's arity as a note reads it: `2`, `2..3`, or `2..` when variadic.
fn arity_label(draft: &Draft) -> String {
    let Some(arity) = draft.get("arity") else {
        return "unknown".to_owned();
    };
    let min = arity.get("min").and_then(Value::as_u64).unwrap_or(0);
    match arity.get("max").and_then(Value::as_u64) {
        None => format!("{min}.."),
        Some(max) if max == min => min.to_string(),
        Some(max) => format!("{min}..{max}"),
    }
}

/// A draft's inferred argument roles as a note reads them.
fn role_label(draft: &Draft) -> String {
    let roles: Vec<String> = draft
        .get("arg_roles")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |roles| {
            roles
                .iter()
                .filter_map(|role| {
                    Some(format!(
                        "{}:{}",
                        role.get("index")?.as_u64()?,
                        role.get("role")?.as_str()?
                    ))
                })
                .collect()
        });
    if roles.is_empty() {
        "none".to_owned()
    } else {
        roles.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gate_expression_matches_the_drafts_own_spelling() {
        let gates = vec![Gate {
            index: 1,
            value: "utf-8".to_owned(),
            span: Lifespan {
                introduced: Some("1.3".to_owned()),
                retired: None,
            },
        }];
        assert_eq!(
            gate_expression(&gates),
            "&[VersionedArgValue { index: 1, value: \"utf-8\", lifecycle: Lifecycle \
             { introduced: Some(\"1.3\"), deprecated: None, retired: None, \
             deprecation_fix: None } }]"
        );
    }

    #[test]
    fn an_arity_label_reads_the_draft_shape() {
        let mut draft = Draft::new();
        draft.insert("arity".into(), json!({ "min": 2, "max": 3 }));
        assert_eq!(arity_label(&draft), "2..3");
        draft.insert("arity".into(), json!({ "min": 2, "max": 2 }));
        assert_eq!(arity_label(&draft), "2");
        draft.insert("arity".into(), json!({ "min": 1, "max": Value::Null }));
        assert_eq!(arity_label(&draft), "1..");
    }

    #[test]
    fn an_empty_value_set_is_not_a_declared_set() {
        let sets = value_sets(Some(&json!([{ "index": 0, "values": [] }])));
        assert!(sets.is_empty(), "{sets:?}");
    }
}
