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

//! Which open documents a publish must wake: the workspace facts another document's diagnostics depend on.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::workspace_index as core_workspace_index;
use tokio::sync::Mutex;
use tower_lsp_server::ls_types::Uri;

use crate::workspace::{entry_point_uri, resolve_source_edge};
use crate::{Backend, DocumentState};

use super::package_refinement::workspace_source_edges;
use super::{DiagSlot, IndexedDiagnosticChange};

/// The workspace facts from one document that can change another open
/// document's diagnostics.
#[derive(Debug, Default, PartialEq, Eq)]
pub(in super::super) struct IndexedDiagnosticFacts {
    original_source: Option<core_workspace_index::WorkspaceDiagnosticSourceContext>,
    original_procs: Vec<
        tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ProcDef,
        >,
    >,
    original_classes: Vec<
        tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
            tcl_compiler::analyser::ClassDef,
        >,
    >,
    original_publications: Vec<tcl_compiler::command_binding::OriginalCommandPublication>,
    pub(in super::super) procs: HashSet<(String, tcl_registry::Arity)>,
    pub(in super::super) classes: HashSet<String>,
    links: Vec<core_workspace_index::WorkspaceCommandLink>,
    pub(in super::super) sources: Vec<core_workspace_index::WorkspaceSource>,
    /// Raw path-constant write facts: a constants-only edit (`set dir /a` →
    /// `/b`) changes which child a computed `source` resolves to — and which
    /// values sourced children import — while the source rows themselves
    /// compare equal, so the constants are resolution-relevant exactly as
    /// the rows are.
    pub(in super::super) path_constants: tcl_compiler::auto_path_eval::PathConstantAssignments,
    pub(in super::super) package_requires: Vec<core_workspace_index::WorkspacePackageRequire>,
    pub(in super::super) package_provides: Vec<core_workspace_index::WorkspacePackageProvide>,
    package_ifneededs: Vec<core_workspace_index::WorkspacePackageIfneeded>,
    package_prefers: Vec<core_workspace_index::WorkspacePackagePrefer>,
}

impl IndexedDiagnosticFacts {
    pub(in super::super) fn capture(
        index: &core_workspace_index::WorkspaceIndex,
        uri: &str,
    ) -> Self {
        Self {
            original_source: index
                .diagnostic_source_context(uri)
                .filter(|context| context.uses_original_names())
                .cloned(),
            original_procs: index
                .original_procedure_declarations()
                .filter(|(owner, _)| *owner == uri)
                .map(|(_, row)| row.clone())
                .collect(),
            original_classes: index
                .original_class_declarations()
                .filter(|(owner, _)| *owner == uri)
                .map(|(_, row)| row.clone())
                .collect(),
            original_publications: index
                .original_command_publications()
                .filter(|(owner, _)| *owner == uri)
                .map(|(_, row)| row.clone())
                .collect(),
            procs: index
                .live_procs_in(uri)
                .into_iter()
                .map(|proc_def| (proc_def.qualified_name.clone(), proc_def.arity))
                .collect(),
            classes: index
                .live_classes_in(uri)
                .into_iter()
                .map(|class_def| class_def.qualified_name.clone())
                .collect(),
            links: index
                .command_links()
                .filter(|link| link.uri == uri)
                .cloned()
                .collect(),
            sources: index
                .sources()
                .filter(|source| source.uri == uri)
                .cloned()
                .collect(),
            path_constants: index.path_constant_assignments(uri).clone(),
            package_requires: index
                .package_requires()
                .filter(|require| require.uri == uri)
                .cloned()
                .collect(),
            package_provides: index
                .package_provides()
                .filter(|provide| provide.uri == uri)
                .cloned()
                .collect(),
            package_ifneededs: index
                .package_ifneededs()
                .filter(|ifneeded| ifneeded.uri == uri)
                .cloned()
                .collect(),
            package_prefers: index
                .package_prefers()
                .filter(|prefer| prefer.uri == uri)
                .cloned()
                .collect(),
        }
    }

    /// The documents this one sourced **before** the edit — resolved from
    /// the captured rows *and the captured constants*, so a computed path
    /// reconstructs the edge it actually had.  Call on
    /// the pre-edit capture; the post-edit consumers come from
    /// [`source_diagnostic_consumers`] against the live index.
    pub(super) fn stale_source_consumers(
        &self,
        index: &core_workspace_index::WorkspaceIndex,
        uri: &str,
    ) -> HashSet<String> {
        source_diagnostic_consumers_with_sources(
            index,
            uri,
            Some(ReplacedSources {
                rows: &self.sources,
                constants: &self.path_constants,
            }),
        )
    }

    pub(in super::super) fn change_to(&self, newer: &Self) -> IndexedDiagnosticChange {
        let mut command_names: HashSet<String> = self
            .procs
            .symmetric_difference(&newer.procs)
            .map(|(name, _)| name.clone())
            .collect();
        command_names.extend(self.classes.symmetric_difference(&newer.classes).cloned());
        if self.links != newer.links {
            command_names.extend(
                self.links
                    .iter()
                    .chain(&newer.links)
                    .map(|link| link.linked_qname.clone()),
            );
        }
        let original_changed = self.original_source != newer.original_source
            || self.original_procs != newer.original_procs
            || self.original_classes != newer.original_classes
            || self.original_publications != newer.original_publications;
        let mut original_commands = Vec::new();
        let mut original_sites = Vec::new();
        if original_changed {
            for facts in [self, newer] {
                for row in &facts.original_procs {
                    let key = (row.name().slot().clone(), row.name().policy());
                    if !original_commands.contains(&key) {
                        original_commands.push(key);
                    }
                    if !original_sites.contains(row.declaration_site()) {
                        original_sites.push(row.declaration_site().clone());
                    }
                }
                for row in &facts.original_classes {
                    let key = (row.name().slot().clone(), row.name().policy());
                    if !original_commands.contains(&key) {
                        original_commands.push(key);
                    }
                    if !original_sites.contains(row.declaration_site()) {
                        original_sites.push(row.declaration_site().clone());
                    }
                }
                for row in &facts.original_publications {
                    let key = (row.slot().clone(), row.policy());
                    if !original_commands.contains(&key) {
                        original_commands.push(key);
                    }
                    if !original_sites.contains(row.declaration_site()) {
                        original_sites.push(row.declaration_site().clone());
                    }
                    if let Some(definition) = row.definition()
                        && !original_sites.contains(&definition.allocation().site)
                    {
                        original_sites.push(definition.allocation().site.clone());
                    }
                }
            }
        }
        IndexedDiagnosticChange {
            original_changed,
            original_commands,
            original_sites,
            command_names,
            source_or_package_changed: self.sources != newer.sources
                || self.path_constants != newer.path_constants
                || self.package_requires != newer.package_requires
                || self.package_provides != newer.package_provides
                || self.package_ifneededs != newer.package_ifneededs
                || self.package_prefers != newer.package_prefers,
        }
    }
}

impl IndexedDiagnosticChange {
    /// Whether this publish moved any workspace fact another document's
    /// diagnostics can be a function of.
    pub(in super::super) fn moved_workspace_facts(&self) -> bool {
        self.original_changed || !self.command_names.is_empty() || self.source_or_package_changed
    }
}

/// The **open** documents the index does not currently hold — the ones a
/// workspace-fact write has to wake by hand.
///
/// Every other consumer set is read *from* the index, so a document that is
/// open but has no entry cannot be found in one — and that is exactly the
/// document whose analysis ran against a snapshot older than the write that
/// just happened: `did_open` drops the entry and the debounced publish is what
/// puts it back, so its own call-site records do not exist yet. Without this
/// set, such a document keeps whatever verdict it reached against the older
/// index, because re-opening an unedited buffer starts no new analysis and
/// nothing else ever names it a consumer.
///
/// Call only when the publish moved something
/// ([`IndexedDiagnosticChange::moved_workspace_facts`]): with no change there
/// is nothing a peer could conclude differently, and waking one anyway would
/// republish a byte-identical set for an unchanged version, which the
/// one-publish-per-version contract (`diagnostics_delivery_smoke`) forbids.
///
/// Terminating: each woken peer's own publish puts it into the index, so the
/// set this can draw from strictly shrinks, and a peer that moves no workspace
/// fact of its own — the common case, a caller that defines nothing — wakes
/// nobody.
///
/// Orphaned buffers are excluded — this is the **transient**
/// set, and only transience makes it self-limiting: a document is in it just
/// between its `didOpen` and its first debounced publish, after which
/// `replace_document` puts it in the index and it leaves for good. A buffer
/// whose backing file was deleted out of band is the one way to be in it
/// permanently, because [`publish_diagnostics_result`] deliberately calls
/// `remove_document` for those rather than re-adding them (re-adding would
/// resurrect the ghost `did_change_watched_files` has just retired), so it is
/// open and never indexed for as long as it stays open. Blanket-waking it on
/// every fact-moving publish anywhere in the workspace is pure waste.
///
/// That exclusion is **not** a ruling that an orphan needs no diagnostics
/// refresh — it still consumes workspace facts.
/// [`orphaned_fact_consumers`] carries the consumption
/// half, targeted by what actually moved; the two sets are disjoint by
/// construction and are extended together at the one call site.
pub(in super::super) fn unindexed_open_documents(
    index: &core_workspace_index::WorkspaceIndex,
    docs: &HashMap<Uri, DocumentState>,
) -> Vec<String> {
    docs.iter()
        .filter(|(_, doc)| !doc.backing_file_deleted)
        .map(|(uri, _)| uri.as_str())
        .filter(|uri| !index.contains_document(uri))
        .map(ToOwned::to_owned)
        .collect()
}

/// Workspace command-header read horizon returned by the actual diagnostic
/// producers. `NoRead` says only that those producers made no command-header
/// query; it says nothing about executable bodies or runtime command absence.
#[derive(Clone, Copy, Debug, Default)]
pub(in super::super) enum WorkspaceCommandQueries {
    #[default]
    NoRead,
    /// The producer consulted a roster or has no retained finite query receipt.
    Broad,
}

impl WorkspaceCommandQueries {
    pub(in super::super) fn join(self, other: Self) -> Self {
        match (self, other) {
            (Self::NoRead, Self::NoRead) => Self::NoRead,
            _ => Self::Broad,
        }
    }
}

/// Read-only dependency currency of one complete configured diagnostic pass.
/// Original analysis input/image/Registry remain retained, not reconstructed
/// from a report name. There is deliberately no runtime-body census here.
pub(in super::super) struct OrphanCommandDependencies {
    pub(in super::super) uri: Uri,
    pub(in super::super) source: Arc<str>,
    pub(in super::super) dialect: String,
    pub(in super::super) revision: u64,
    pub(in super::super) version: Option<i32>,
    pub(in super::super) analysis: Arc<AnalysisResult>,
    pub(in super::super) expected_epoch: u64,
    pub(in super::super) current_epoch: Arc<std::sync::atomic::AtomicU64>,
    pub(in super::super) inputs_gate: Arc<tokio::sync::RwLock<()>>,
    pub(in super::super) slots: std::sync::Weak<Mutex<HashMap<Uri, DiagSlot>>>,
    pub(in super::super) slot_generation: Option<u64>,
    pub(in super::super) factory_generation: Arc<tokio::sync::RwLock<u64>>,
    pub(in super::super) expected_factory_generation: u64,
    pub(in super::super) queries: WorkspaceCommandQueries,
}

impl std::fmt::Debug for OrphanCommandDependencies {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OrphanCommandDependencies")
            .field("uri", &self.uri)
            .field("revision", &self.revision)
            .field("queries", &self.queries)
            .finish_non_exhaustive()
    }
}

impl OrphanCommandDependencies {
    pub(in super::super) fn may_read_commands(&self, uri: &Uri, document: &DocumentState) -> bool {
        if matches!(self.queries, WorkspaceCommandQueries::Broad) {
            return true;
        }
        // A busy or changing input owner is unknown, never NoRead. This guard
        // covers the final no-await source/configuration/slot join and decision.
        let Ok(_inputs) = self.inputs_gate.try_read() else {
            return true;
        };
        let Some(slot_owner) = self.slots.upgrade() else {
            return true;
        };
        let Ok(slots) = slot_owner.try_lock() else {
            return true;
        };
        let Ok(factory) = self.factory_generation.try_read() else {
            return true;
        };
        let slot = slots.get(uri);
        if &self.uri != uri
            || self.source != document.text
            || self.dialect != document.dialect
            || self.revision != document.revision
            || self.version != document.version
            || document.dialect_resolution_generation != document.resolved_dialect_generation
            || self
                .current_epoch
                .load(std::sync::atomic::Ordering::Acquire)
                != self.expected_epoch
            || *factory != self.expected_factory_generation
            || self.slot_generation != slot.map(|slot| slot.generation)
            || slot.is_some_and(|slot| slot.dirty)
        {
            return true;
        }
        // Keep the entire actual source input live; missing source ownership
        // cannot become a complete dependency result merely because it has rows.
        tcl_compiler::source_graph::current_analysis(
            &tcl_lexer::normalise_lone_cr(&document.text),
            &self.analysis,
        )
        .is_none()
    }
}

/// The **orphaned** open buffers a fact-moving publish must still wake.
///
/// Absence from the index is two separate facts about such a buffer, and
/// [`unindexed_open_documents`] alone conflates them:
///
/// * It must not **contribute** facts. Its path is dead, so re-indexing it
///   duplicates every proc of the file it was renamed to and sends
///   go-to-definition somewhere the editor cannot open — the ghost
///   `did_change_watched_files` has just retired. `publish_diagnostics_result`
///   keeps calling `remove_document` for it, and that stays true.
/// * It does still **consume** them. The buffer is on screen and its squiggles
///   are read like any other's, so when a peer changes the arity of a proc it
///   calls, its `E002`/`W123` are wrong until something recomputes them. No
///   other set can name it: [`command_diagnostic_consumers`] and the source
///   walkers all read the consumer's own records *from* the index, and it has
///   none there for as long as it stays open.
///
/// So the contribution filter lives on the index write, and this is the
/// consumption half — deliberately a *separate* set rather than a relaxation
/// of [`unindexed_open_documents`], which must stay orphan-free to keep its own
/// property: that set is the transient-race fallback, and it is self-limiting
/// only because every document in it leaves at its first publish.
///
/// Source/package moves wake every orphan. Command-signature moves may omit
/// an orphan only with a current complete diagnostic-query receipt. Unsupported
/// callback/global/settlement query horizons are Broad, and missing or stale
/// receipts wake all. Text mentions remain a harmless positive wake heuristic;
/// they never certify an absent source or runtime dependency.
///
/// Terminating for the same reason the transient set is: an orphan's own
/// publish has `before = absent, after = absent`, so it moves no fact and wakes
/// nobody.
pub(in super::super) fn orphaned_fact_consumers(
    docs: &HashMap<Uri, DocumentState>,
    change: &IndexedDiagnosticChange,
) -> Vec<String> {
    // Match the index rule's key: it compares invocation names against the
    // changed names' unqualified tails, and a call site spells the tail.
    let tails: Vec<&str> = change
        .command_names
        .iter()
        .map(|name| tcl_compiler::naming::key_tail(name))
        .filter(|tail| !tail.is_empty())
        .collect();
    docs.iter()
        .filter(|(_, doc)| doc.backing_file_deleted)
        .filter(|(uri, doc)| {
            change.source_or_package_changed
                || tails.iter().any(|tail| doc.text.contains(tail))
                || doc
                    .orphan_command_dependencies
                    .as_ref()
                    .is_none_or(|dependencies| dependencies.may_read_commands(uri, doc))
        })
        .map(|(uri, _)| uri.as_str().to_owned())
        .collect()
}

/// Original provider changes are invalidation dependencies, not bindings.
/// Every relevant open Tcl source is retained. An unknown relevant caller
/// inventory conservatively wakes that whole set; foreign surfaces do not.
pub(in super::super) fn original_diagnostic_consumers(
    index: &core_workspace_index::WorkspaceIndex,
    docs: &HashMap<Uri, DocumentState>,
    change: &IndexedDiagnosticChange,
) -> HashSet<String> {
    if !change.original_changed {
        return HashSet::new();
    }
    let relevant: Vec<_> = docs
        .iter()
        .filter(|(uri, document)| {
            Backend::original_declaration_document_participates(uri, document) != Some(false)
        })
        .collect();
    let all = || {
        relevant
            .iter()
            .map(|(uri, _)| uri.as_str().to_owned())
            .collect()
    };
    if change.original_commands.is_empty() {
        return all();
    }
    let mut consumers = HashSet::new();
    for (uri, document) in &relevant {
        let Some(context) = index.diagnostic_source_context(uri.as_str()) else {
            return all();
        };
        if !context.uses_original_names()
            || context.image() != &tcl_lexer::SourceImage::document(&document.text)
        {
            return all();
        }
        for invocation in index
            .invocations()
            .filter(|invocation| invocation.uri == uri.as_str())
        {
            if matches!(
                invocation.lookup,
                tcl_compiler::signature_scan::types::SignatureCommandLookup::DeferredReference
                    | tcl_compiler::signature_scan::types::SignatureCommandLookup::EnsembleTarget { .. }
            ) {
                continue;
            }
            let (Some(input), Some(lookup)) =
                (&invocation.original_name_input, &invocation.original_lookup)
            else {
                return all();
            };
            if input != lookup.name_input()
                || lookup.site().source.source_image() != context.image()
                || input
                    .original_word_key()
                    .is_some_and(|key| key.lexer_config() != context.config())
            {
                return all();
            }
            let selected = lookup.matching_slot_publications(
                change
                    .original_commands
                    .iter()
                    .map(|(slot, policy)| (slot, *policy, ())),
            );
            let Some(selected) = selected else {
                return all();
            };
            let actual = invocation
                .resolved_command_reference
                .as_ref()
                .and_then(|reference| {
                    reference
                        .linked_definition()
                        .or_else(|| reference.definition())
                })
                .or(invocation.resolved_definition.as_ref());
            if !selected.is_empty()
                || actual.is_some_and(|definition| {
                    change
                        .original_sites
                        .contains(&definition.allocation().site)
                })
            {
                consumers.insert(uri.as_str().to_owned());
            }
        }
    }
    consumers
}

/// Documents whose own call-site records consume one of the changed command
/// signatures. Both exact qualified candidates and the opt-in W123 tail tier
/// are dependencies; unrelated open documents are deliberately absent.
pub(in super::super) fn command_diagnostic_consumers(
    index: &core_workspace_index::WorkspaceIndex,
    changed_names: &HashSet<String>,
) -> HashSet<String> {
    if changed_names.is_empty() {
        return HashSet::new();
    }
    let qualified: HashSet<&str> = changed_names
        .iter()
        .map(|name| tcl_compiler::naming::unroot_rooted_key(name).unwrap_or(name))
        .collect();
    // Constructed declaration keys and their bare tails are invalidation
    // dependencies. They supply no command binding or editable occurrence.
    let tails: HashSet<&str> = changed_names
        .iter()
        .map(|name| tcl_compiler::naming::key_tail(name))
        .collect();
    index
        .invocations()
        .filter(|invocation| {
            tails.contains(invocation.name.as_str())
                || invocation.resolution_candidates.iter().any(|candidate| {
                    qualified.contains(
                        tcl_compiler::naming::unroot_rooted_key(candidate).unwrap_or(candidate),
                    )
                })
        })
        .map(|invocation| invocation.uri.clone())
        .collect()
}

/// Source ancestors consume a child's package facts; source descendants
/// consume an ancestor's package facts. Walk each direction separately so a
/// sibling under the same parent is not spuriously refreshed.
pub(in super::super) fn source_diagnostic_consumers(
    index: &core_workspace_index::WorkspaceIndex,
    changed_uri: &str,
) -> HashSet<String> {
    source_diagnostic_consumers_with_sources(index, changed_uri, None)
}

/// The replacement rows plus the constants they resolved under **when they
/// were live** — the pre-replacement capture.  Folding an old row with the
/// post-edit constants would reconstruct an edge the old document never had
/// so the old facts travel together.
pub(in super::super) struct ReplacedSources<'a> {
    pub(in super::super) rows: &'a [core_workspace_index::WorkspaceSource],
    pub(in super::super) constants: &'a tcl_compiler::auto_path_eval::PathConstantAssignments,
}

/// [`source_diagnostic_consumers`] with one document's source rows replaced.
/// The publish path uses this after the atomic index replacement to recover
/// the pre-change graph without scanning the workspace before every ordinary
/// body-only publish.
pub(in super::super) fn source_diagnostic_consumers_with_sources(
    index: &core_workspace_index::WorkspaceIndex,
    changed_uri: &str,
    replacement: Option<ReplacedSources<'_>>,
) -> HashSet<String> {
    fn closure(
        initial: &str,
        edges: &[tcl_lsp_core::source_graph::RunEdge],
        forward: bool,
    ) -> HashSet<String> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut pending = vec![initial.to_owned()];
        while let Some(current) = pending.pop() {
            for edge in edges {
                let next = if forward && edge.parent == current {
                    Some(edge.child.as_str())
                } else if !forward && edge.child == current {
                    Some(edge.parent.as_str())
                } else {
                    None
                };
                if let Some(next) = next
                    && next != initial
                    && seen.insert(next.to_owned())
                {
                    pending.push(next.to_owned());
                }
            }
        }
        seen
    }

    let mut edges = workspace_source_edges(index);
    if let Some(replaced) = replacement {
        edges.retain(|edge| edge.parent != changed_uri);
        edges.extend(replaced.rows.iter().filter_map(|source| {
            // Old rows fold with the old constants.  Imports are the one
            // best-effort input: the pre-edit import view is not
            // reconstructible once the index has moved, and using the
            // current one can only vary which *stale* child gets a refresh.
            resolve_source_edge(
                &source.uri,
                &source.raw_path,
                source.original_path_expression.as_ref(),
                source.range.start(),
                replaced.constants,
                &index.imported_path_constants_for(&source.uri),
            )
            .map(|child| tcl_lsp_core::source_graph::RunEdge {
                parent: source.uri.clone(),
                child,
                at: source.range.start(),
                enclosing_body: source.enclosing_body,
                kind: tcl_lsp_core::source_graph::RunEdgeKind::Source,
            })
        }));
    }
    let mut consumers = closure(changed_uri, &edges, true);
    consumers.extend(closure(changed_uri, &edges, false));
    consumers
}

/// Add open documents whose configured entry-point inheritance reads the
/// changed document's package facts.
pub(super) async fn add_entry_point_diagnostic_consumers(
    slots: &Arc<Mutex<HashMap<Uri, DiagSlot>>>,
    changed_uri: &str,
    consumers: &mut HashSet<String>,
) {
    let slots = slots.lock().await;
    for (uri, slot) in slots.iter() {
        let Some(inputs) = &slot.latest_inputs else {
            continue;
        };
        if inputs.entry_points.iter().any(|entry| {
            entry_point_uri(entry, inputs.folder_root.as_deref()).as_deref() == Some(changed_uri)
        }) {
            consumers.insert(uri.as_str().to_owned());
        }
    }
}
