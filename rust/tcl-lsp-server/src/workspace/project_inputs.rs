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

//! Workspace-derived inputs of every project file: call-site evidence, declared commands, source modules and subclass methods.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp_server::ls_types::Uri;

use super::class_factories::{reindex_unopened_factory_consumers, sync_workspace_class_factories};
use super::{ClassFactorySync, EvidenceHandles, RoundReschedule};
use crate::diagnostics::{
    DiagInputs, DiagSlot, refresh_can_change_result, reschedule_peers, sync_evaluator_epoch,
};
use crate::tracked_locks::DbSnapshot;
use crate::uri_norm::DocumentPath as _;

/// Refresh the project's cross-file call-site evidence after a worker pass
/// published, then reschedule whatever the refresh invalidated.
pub(in super::super) async fn refresh_cross_file_evidence(
    handles: &EvidenceHandles,
    slots: &Arc<Mutex<HashMap<Uri, DiagSlot>>>,
    uri: &Uri,
) {
    // Call-site evidence first, class factories second — deliberately, and
    // *not* in dependency order.  `file_call_site_evidence` does read the
    // factory oracle transitively (`project_proc_names` / `file_decls` come off
    // `item_tree`, which reads `SourceFile::workspace_class_factories`), so
    // running the factory sync first looks like the tidier order.  Measured, it
    // is much worse: the factory publish invalidates every file's `item_tree`,
    // so the call-site pass that follows it is a *cold* one — the seconds-long
    // whole-project lower-and-CFG path this function is explicitly kept off the
    // edit handler for — and that lands in front of `reschedule_peers`, which
    // is what actually republishes the files the factory publish invalidated.
    // On a large workspace that delays convergence enough to lose a two-level
    // cross-file chain that would otherwise resolve inside the client's window.
    // Run in this order, the call-site pass reads memoised state and the
    // reschedule follows the factory publish immediately; the call-site tables
    // computed under the pre-sync oracle are corrected by the peers' own next
    // refresh, which the reschedule has already scheduled.
    //
    // First, the evaluator epoch: a hook quarantined during the pass
    // moved it on the worker that ran the pass, and this is where the server
    // first sees that. Taken before the evidence snapshot, whose revision the
    // write below must still match, so the next analysis keys its lattices by
    // the evaluators as they now stand. Nothing is rescheduled for it: a
    // quarantine is no verdict on the answers already published, and a
    // pass on another worker would only run the crashing body again.
    sync_evaluator_epoch(&handles.db).await;
    let evidence_changes = sync_cross_file_evidence(handles).await;
    let evidence_requires_self_refresh = evidence_changes.requires_self_refresh.contains(uri);
    let mut changed = evidence_changes.changed;
    let wake = RoundReschedule {
        slots,
        self_uri: uri,
    };
    let ClassFactorySync {
        moved: factory_moved,
        affected_names,
    } = sync_workspace_class_factories(handles, Some(wake)).await;
    let oracle_moved = !factory_moved.is_empty();
    let factory_requires_self_refresh = factory_moved.contains(uri);
    for uri in factory_moved {
        if !changed.contains(&uri) {
            changed.push(uri);
        }
    }
    // An edit that adds or removes a metaclass changes what *unopened*
    // documents manufacture too, and `reschedule_peers` below can only reach
    // documents that have a diagnostics slot — i.e. open ones.
    // Only when the oracle actually moved: an unchanged oracle re-indexes
    // nothing, so the ordinary editing loop pays nothing for this.
    if oracle_moved {
        reindex_unopened_factory_consumers(handles, &affected_names).await;
    }
    // Re-analyse *this* document only when the refresh can change its result:
    // a non-empty table retracts a fold, while losing a previously-covered
    // `source` boundary must withdraw a fold the just-finished run could have
    // made from stale evidence. Going from no view to an empty view can only
    // add a hint, so leaving that until the next edit avoids a byte-identical
    // duplicate publish in the common case. Any class-factory move can alter
    // the document that drove the round, including removal of the last one.
    if changed.contains(uri)
        && (evidence_requires_self_refresh
            || factory_requires_self_refresh
            || refresh_can_change_result(&handles.db, &handles.db_files, uri).await)
        && let Some(slot) = slots.lock().await.get_mut(uri)
    {
        slot.mark_dirty();
    }
    // The subclass-provided-method view rides the same
    // refresh: computed off the workspace index the publish just updated,
    // single-round (nothing it writes feeds its own computation).  A moved
    // view marks this document's own slot dirty directly — the
    // `refresh_can_change_result` gate above is a call-site-evidence
    // criterion and says nothing about W308 abstentions.
    let mut subclass_moved = sync_workspace_subclass_methods(handles).await;
    for moved in sync_workspace_declared_commands(handles).await {
        if !subclass_moved.contains(&moved) {
            subclass_moved.push(moved);
        }
    }
    for moved in sync_workspace_source_modules(handles).await {
        if !subclass_moved.contains(&moved) {
            subclass_moved.push(moved);
        }
    }
    if subclass_moved.iter().any(|u| u == uri)
        && let Some(slot) = slots.lock().await.get_mut(uri)
    {
        slot.mark_dirty();
    }
    for moved in subclass_moved {
        if !changed.contains(&moved) {
            changed.push(moved);
        }
    }
    // Republish every peer the refresh invalidated. Only documents
    // that already have resolved inputs are eligible: a file nobody
    // has analysed has no published diagnostics to go stale, and its
    // inputs cannot be resolved from here anyway. This converges — the
    // peer's own run re-syncs, finds nothing changed
    // (compare-then-set), and reschedules nobody.
    reschedule_peers(slots, uri, changed).await;
}

pub(in super::super) struct CrossFileEvidenceSnapshot {
    pub(in super::super) snapshot: DbSnapshot<tcl_lsp_db::TclDatabase>,
    pub(in super::super) files: Vec<(Uri, tcl_lsp_db::SourceFile)>,
    pub(in super::super) members: HashSet<Uri>,
    pub(in super::super) project: tcl_lsp_db::Project,
    pub(in super::super) revision: salsa::Revision,
}

pub(in super::super) type CrossFileEvidenceUpdate = (
    Uri,
    tcl_lsp_db::SourceFile,
    Option<Arc<tcl_compiler::unit_scope::CallSiteEvidence>>,
    bool,
);

#[derive(Default)]
pub(in super::super) struct CrossFileEvidenceChanges {
    pub(in super::super) changed: Vec<Uri>,
    requires_self_refresh: HashSet<Uri>,
}

impl EvidenceHandles {
    pub(in super::super) fn from_inputs(inputs: &DiagInputs) -> Self {
        Self {
            db_config: Arc::clone(&inputs.db_config),
            folder_db_configs: Arc::clone(&inputs.folder_db_configs),
            db: Arc::clone(&inputs.db),
            db_files: Arc::clone(&inputs.db_files),
            db_project_members: Arc::clone(&inputs.db_project_members),
            db_project: Arc::clone(&inputs.db_project),
            workspace_index: Arc::clone(&inputs.workspace_index),
            documents: Arc::clone(&inputs.documents),
            rehoming_gate: Arc::clone(&inputs.rehoming_gate),
            live_publication_gate: Arc::clone(&inputs.live_publication_gate),
            class_factory_generation: Arc::clone(&inputs.class_factory_generation),
        }
    }
}

pub(in super::super) async fn capture_cross_file_evidence_snapshot(
    handles: &EvidenceHandles,
) -> Option<CrossFileEvidenceSnapshot> {
    // Capture membership, project, and every input handle behind `db` in
    // publication order. Retaining non-member handles lets the write phase
    // clear an edited orphan's old project evidence. Coverage is derived from
    // the admitted Salsa inputs later; the workspace index deliberately lags
    // an ordinary edit until its diagnostics pass, so it cannot prove coverage
    // for this snapshot.
    loop {
        let db = handles.db.lock().await;
        let Ok(db_files) = handles.db_files.try_lock() else {
            drop(db);
            drop(handles.db_files.lock().await);
            continue;
        };
        let Ok(members) = handles.db_project_members.try_lock() else {
            drop(db_files);
            drop(db);
            drop(handles.db_project_members.lock().await);
            continue;
        };
        let Ok(db_project) = handles.db_project.try_lock() else {
            drop(members);
            drop(db_files);
            drop(db);
            drop(handles.db_project.lock().await);
            continue;
        };
        let &project = db_project.as_ref()?;
        let files = db_files
            .iter()
            .map(|(uri, &file)| (uri.clone(), file))
            .collect();
        let members = members.clone();
        let revision = tcl_lsp_db::database_revision(&db);
        let snapshot = handles.db.snapshot_from(&db, "sync_cross_file_evidence")?;
        return Some(CrossFileEvidenceSnapshot {
            snapshot,
            files,
            members,
            project,
            revision,
        });
    }
}

pub(in super::super) async fn apply_cross_file_evidence_if_current(
    handles: &EvidenceHandles,
    revision: salsa::Revision,
    pending: Vec<CrossFileEvidenceUpdate>,
) -> CrossFileEvidenceChanges {
    use salsa::Setter as _;

    let mut db = handles.db.lock().await;
    if tcl_lsp_db::database_revision(&db) != revision {
        return CrossFileEvidenceChanges::default();
    }
    let mut changes = CrossFileEvidenceChanges {
        changed: Vec::with_capacity(pending.len()),
        requires_self_refresh: HashSet::new(),
    };
    for (uri, file, want, requires_self_refresh) in pending {
        if call_site_evidence_matches(file.external_call_sites(&*db).as_ref(), want.as_ref()) {
            continue;
        }
        file.set_external_call_sites(&mut *db).to(want);
        if requires_self_refresh {
            changes.requires_self_refresh.insert(uri.clone());
        }
        changes.changed.push(uri);
    }
    changes
}

/// Refresh every project file's [`tcl_lsp_db::SourceFile::external_call_sites`]
/// — the call sites in *other* files that reach the procedures it declares —
/// and report the URIs whose evidence actually moved.
///
/// **Runs on the debounced diagnostics worker, never on the edit handler.**
/// The cold path lowers and builds a CFG for every project file
/// (`file_call_site_evidence`), seconds of work for a large workspace;
/// doing that inside `did_change` while the `documents` and `db` locks are
/// held would block the LSP message loop on every keystroke. On the worker it
/// is already off the message loop, already debounced, and already the place
/// that pays for analysis.
///
/// **Compare-then-set**, so an edit that moves no call-site literal writes no
/// input and invalidates nothing. The returned URIs are the files whose
/// published diagnostics are now stale — editing `main.tcl` can change what
/// `lib.tcl` folds, and `did_change` only ever schedules the edited document,
/// so the caller must reschedule them or `lib.tcl` keeps showing a diagnostic
/// the project no longer supports.
///
/// Salsa cancellation is caught and reported as "nothing changed": the edit
/// that cancelled this pass drives its own refresh.
///
/// **The project-wide read runs on a cloned salsa snapshot inside
/// `spawn_blocking`**, so the `db` mutex — which every feature read path and
/// every `db_set_source` contends on — is held only long enough to clone the
/// handle, and again for the short write section at the end. Held across the
/// read, one worker pass over a large workspace would stall hover, completion
/// and the next keystroke's `set_text` for the whole scan. The snapshot shares
/// salsa's memo storage with the live database, so
/// the writes below still see everything the read computed.
///
/// **Not gated on `crossFileResolution`**, despite feeding a cross-file fact.
/// This work remains on regardless of the toggle. The diagnostics paths use
/// that toggle for their broad W123 tier and callback-arity validation; exact
/// direct-call resolution is always workspace-index based. What this writes is
/// consumed by
/// `document_compilation_unit`'s interprocedural seed, which is *soundness*,
/// not an opt-in refinement: without the project's call sites a library file
/// folds on its own callers' literals and reports an I230 the workspace
/// contradicts. Skipping the pass when the toggle is off
/// reinstates exactly that unsound fold —
/// `caller_in_a_sourcing_file_with_a_differing_literal_clears_i230` in the e2e
/// suite runs with `crossFileResolution` at its default (off) and fails
/// immediately if this is gated on it.
pub(in super::super) async fn sync_cross_file_evidence(
    handles: &EvidenceHandles,
) -> CrossFileEvidenceChanges {
    // A removal changes membership and the Salsa project under the rehoming
    // transaction; a live orphan edit does so under live publication. Exclude
    // both until one matching Salsa snapshot has captured them. Coverage then
    // comes from that snapshot's own source sites, not the independently-
    // published workspace index. Both gates leave before the project-wide read.
    let CrossFileEvidenceSnapshot {
        snapshot,
        files,
        members,
        project,
        revision,
    } = {
        let _rehoming = handles.rehoming_gate.lock().await;
        let _live_publication = handles.live_publication_gate.lock().await;
        let Some(captured) = capture_cross_file_evidence_snapshot(handles).await else {
            return CrossFileEvidenceChanges::default();
        };
        captured
    };
    // `AssertUnwindSafe`: the closure only *reads* the database, and a
    // cancellation unwind discards the whole `pending` list without applying
    // anything, so no torn state can be observed afterwards.
    let pending = crate::rt::spawn_blocking(move || {
        salsa::Cancelled::catch(std::panic::AssertUnwindSafe(|| {
            let covered = files_with_covered_load_targets(&snapshot, &files, &members, project);
            files
                .into_iter()
                .filter_map(|(uri, file)| {
                    let want = if members.contains(&uri) {
                        wanted_call_site_evidence(&snapshot, &uri, file, project, &covered)
                    } else {
                        None
                    };
                    let have = file.external_call_sites(&*snapshot).as_ref();
                    let requires_self_refresh =
                        members.contains(&uri) && !covered.contains(&uri) && have.is_some();
                    (!call_site_evidence_matches(have, want.as_ref())).then_some((
                        uri,
                        file,
                        want,
                        requires_self_refresh,
                    ))
                })
                .collect::<Vec<_>>()
        }))
        .ok()
    })
    .await;
    let Ok(Some(pending)) = pending else {
        return CrossFileEvidenceChanges::default();
    };
    // Short write section. Any intervening Salsa write invalidates the whole
    // result: applying one file at a time after a source retirement or newer
    // sync could otherwise restore stale evidence. With the captured revision
    // still current, re-run each comparison so a byte-identical value writes
    // no input and invalidates nothing.
    apply_cross_file_evidence_if_current(handles, revision, pending).await
}

/// Refresh every file's
/// [`tcl_lsp_db::SourceFile::workspace_subclass_methods`] — the
/// subclass-provided-method view behind the cross-file half of the
/// template-method W308 abstention — and report the URIs whose
/// stored view actually moved.
///
/// One round always suffices, unlike the factory fixpoint: the view is
/// computed from the **workspace index**, which is maintained from published
/// analyses independently of this input, so publishing it cannot change what
/// the next computation would say.  (It deliberately does *not* read the
/// salsa analyses of other files — a per-file analysis reading a
/// project-wide fold of per-file analyses is the cycle the input channel
/// exists to break.)
///
/// **Each file stores only its own slice** — the entries for classes *it*
/// defines (creation site or `oo::define` stub), which are the only
/// receivers its `my`-dispatch W308 sites can name
/// (`enclosing_class_at_offset` resolves within the document).  Storing the
/// whole project map on every file would turn every index movement into a
/// whole-project invalidation: during a scan or heavy document churn the map
/// moves with nearly every publish, so N documents cost ~N² re-analyses.
/// Sliced, an edit invalidates exactly the files whose own classes gained or
/// lost subclass-provided methods.
///
/// The project fold itself is one bounded pass
/// ([`core_workspace_index::WorkspaceIndex::subclass_provided_methods`] —
/// edge maps built once, not per class), so running it per publish costs the
/// same order as the sibling evidence syncs.  An empty slice is stored as
/// `None`, so a workspace with no class hierarchies never leaves the
/// input's default.
///
/// Lock discipline: the index read completes and drops before the `db` lock
/// is taken (the global order puts `db` before `workspace_index`; holding
/// neither across the other sidesteps the ordering entirely).
/// Give every document the procedures the *other* documents define that a
/// call may trust ([`tcl_lsp_db::world_preserving_procedure_declarations`])
/// as declarations, so a cross-file call keeps the rest of the document proved.
/// A document's own procedures are excluded: its definitions resolve them.
///
/// Compare-then-set, like the evidence syncs: an unmoved slice invalidates
/// nothing. Trusting a declaration only removes widening, so verdicts grow
/// monotonically and repeated publishes converge.
pub(super) async fn sync_workspace_declared_commands(handles: &EvidenceHandles) -> Vec<Uri> {
    use salsa::Setter as _;
    let mut db = handles.db.lock().await;
    let db_files = handles.db_files.lock().await;
    // A name two documents define resolves the same way on every sync.
    let mut providers = db_files
        .iter()
        .map(|(uri, &file)| (uri, file))
        .collect::<Vec<_>>();
    providers.sort_by(|(left, _), (right, _)| left.as_str().cmp(right.as_str()));
    let mut changed = Vec::new();
    for (uri, &file) in db_files.iter() {
        let want = tcl_lsp_db::merged_workspace_procedure_declarations(
            &*db,
            providers
                .iter()
                .filter(|(owner, _)| *owner != uri)
                .map(|&(_, provider)| provider),
        );
        let have = file.workspace_declared_commands(&*db);
        let matches = match (have.as_ref(), want.as_ref()) {
            (Some(have), Some(want)) => Arc::ptr_eq(have, want) || **have == **want,
            (None, None) => true,
            _ => false,
        };
        if matches {
            continue;
        }
        file.set_workspace_declared_commands(&mut *db).to(want);
        changed.push(uri.clone());
    }
    changed
}

/// Offer every document the sibling files that define classes, as an implicit
/// auto-load index: a call to one of those classes while it is absent sources
/// the whole sibling file, as `auto_load` does from a `tclIndex` entry. A
/// document never offers itself.
///
/// Compare-then-set, like the other evidence syncs.
pub(super) async fn sync_workspace_source_modules(handles: &EvidenceHandles) -> Vec<Uri> {
    use salsa::Setter as _;
    let classes_by_uri = {
        let index = handles.workspace_index.read().await;
        let mut classes_by_uri: HashMap<String, Vec<String>> = HashMap::new();
        for class in index.live_classes() {
            classes_by_uri.entry(class.uri.clone()).or_default().push(
                tcl_registry::rooted_command_descriptor_label(&class.qualified_name),
            );
        }
        classes_by_uri
    };
    let mut db = handles.db.lock().await;
    let db_files = handles.db_files.lock().await;
    let providers: Vec<(
        Uri,
        tcl_compiler::command_binding::TrustedSourceModuleLoader,
    )> = db_files
        .iter()
        .filter_map(|(uri, &file)| {
            let mut names = classes_by_uri.get(uri.as_str())?.clone();
            names.sort_unstable();
            names.dedup();
            let path = file
                .path(&*db)
                .clone()
                .unwrap_or_else(|| uri.as_str().to_owned());
            Some((
                uri.clone(),
                tcl_compiler::command_binding::TrustedSourceModuleLoader::workspace_document(
                    path,
                    file.text(&*db),
                    names,
                ),
            ))
        })
        .collect();
    let mut changed = Vec::new();
    for (uri, &file) in db_files.iter() {
        let modules = providers
            .iter()
            .filter(|(owner, _)| owner != uri)
            .map(|(_, loader)| loader.clone())
            .collect::<Vec<_>>();
        let want = (!modules.is_empty()).then(|| Arc::new(modules));
        if file.workspace_source_modules(&*db) == &want {
            continue;
        }
        file.set_workspace_source_modules(&mut *db).to(want);
        changed.push(uri.clone());
    }
    changed
}

pub(super) async fn sync_workspace_subclass_methods(handles: &EvidenceHandles) -> Vec<Uri> {
    use salsa::Setter as _;
    // The project fold and each document's class list, under one index read.
    let (full, classes_by_uri) = {
        let index = handles.workspace_index.read().await;
        let full = index.subclass_provided_methods();
        let mut classes_by_uri: HashMap<String, Vec<String>> = HashMap::new();
        for c in index.live_classes() {
            classes_by_uri
                .entry(c.uri.clone())
                .or_default()
                .push(c.qualified_name.clone());
        }
        (full, classes_by_uri)
    };
    let mut db = handles.db.lock().await;
    let db_files = handles.db_files.lock().await;
    let mut changed = Vec::new();
    for (uri, &file) in db_files.iter() {
        let slice: tcl_compiler::analyser::SubclassProvidedMethods = classes_by_uri
            .get(uri.as_str())
            .into_iter()
            .flatten()
            .filter_map(|qname| {
                full.get(qname)
                    .map(|methods| (qname.clone(), methods.clone()))
            })
            .collect();
        let want = (!slice.is_empty()).then(|| Arc::new(slice));
        let have = file.workspace_subclass_methods(&*db);
        let matches = match (have.as_ref(), want.as_ref()) {
            (Some(have), Some(want)) => Arc::ptr_eq(have, want) || **have == **want,
            (None, None) => true,
            _ => false,
        };
        if matches {
            continue;
        }
        file.set_workspace_subclass_methods(&mut *db).to(want);
        changed.push(uri.clone());
    }
    changed
}

/// The evidence `uri` should be carrying, or `None` when the host cannot claim
/// a closed world over its callers.
///
/// A file whose `source` target the host never indexed has a caller the project
/// cannot enumerate, so it gets **no** evidence: `None` keeps its
/// `LOADS_EXTERNAL_UNIT` boundary closed rather than letting `Some(empty)`
/// assert a closed world the scan cannot back.
fn wanted_call_site_evidence(
    db: &tcl_lsp_db::TclDatabase,
    uri: &Uri,
    file: tcl_lsp_db::SourceFile,
    project: tcl_lsp_db::Project,
    covered: &HashSet<Uri>,
) -> Option<Arc<tcl_compiler::unit_scope::CallSiteEvidence>> {
    covered
        .contains(uri)
        .then(|| tcl_lsp_db::file_external_call_sites_for_inputs(db, file, project))
        .flatten()
}

/// Whether a file's stored cross-file evidence already equals what the project
/// says it should be — the compare half of the compare-then-set.
///
/// The pointer check is the steady-state fast path: `file_external_call_sites`
/// hands back the *same* memoised `Arc` the previous sync stored whenever the
/// query did not re-execute, so an unchanged project settles in one comparison
/// per file instead of a deep set comparison per file.
fn call_site_evidence_matches(
    have: Option<&Arc<tcl_compiler::unit_scope::CallSiteEvidence>>,
    want: Option<&Arc<tcl_compiler::unit_scope::CallSiteEvidence>>,
) -> bool {
    match (have, want) {
        (Some(have), Some(want)) => Arc::ptr_eq(have, want) || **have == **want,
        (None, None) => true,
        _ => false,
    }
}

/// The project files whose unit-loading boundaries the host can actually
/// account for — the precondition for claiming a closed world over their
/// callers.
///
/// Enumerating the configured project proves only that the server knows its own
/// files; it does not prove what a `source` boundary implies, namely that the
/// loaded unit is among them. A `source ../shared.tcl` pointing outside the
/// workspace runs a script that can call this file's procedures with anything,
/// so handing that file `Some(empty)` would re-open exactly the unsound fold
/// this guards against.
///
/// A file is covered when **every** `source` site it contains is a literal
/// path resolving to a document the project holds. A non-literal path
/// (`source [file join $dir x.tcl]`) is never covered — the target is a
/// runtime value.
///
/// `package require` / `load` are deliberately not consulted: the required
/// unit is third-party code written without knowledge of this file, so it
/// cannot name its procedures. The one way it *can* re-enter — a callback
/// this file registers — is already recorded as an opaque caller by
/// `tcl_compiler::unit_scope::record_indirect_callers`.
pub(in super::super) fn files_with_covered_load_targets(
    db: &tcl_lsp_db::TclDatabase,
    files: &[(Uri, tcl_lsp_db::SourceFile)],
    members: &HashSet<Uri>,
    project: tcl_lsp_db::Project,
) -> HashSet<Uri> {
    let known_paths: HashSet<std::path::PathBuf> = files
        .iter()
        .filter(|(uri, _)| members.contains(uri))
        .filter_map(|(uri, _)| uri.document_path().map(|p| p.to_path_buf()))
        .collect();
    files
        .iter()
        .filter(|(uri, _)| members.contains(uri))
        .filter(|(uri, file)| {
            let Some(targets) = tcl_lsp_db::file_source_targets_for_inputs(db, *file, project)
            else {
                return false;
            };
            let Some(parent) = uri.document_path().map(|p| p.to_path_buf()) else {
                return targets.is_empty();
            };
            targets.iter().all(|site| {
                site.is_literal
                    && known_paths.contains(&tcl_lsp_core::source_graph::resolve_source_target(
                        &parent,
                        &site.raw_path,
                    ))
            })
        })
        .map(|(uri, _)| uri.clone())
        .collect()
}
