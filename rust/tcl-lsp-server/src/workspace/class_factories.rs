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

//! The workspace class-factory oracle and its fixpoint publication.

use super::{
    AnalysisResult, Arc, Backend, ClassFactorySync, EvidenceHandles, FromStr, HashSet,
    RoundReschedule, Uri, longest_folder_match, reschedule_peers,
};

/// Re-index closed factory consumers with their checked document input. The
/// oracle supplies reporting hints; it does not replace source metadata.
pub(in super::super) async fn reindex_unopened_factory_consumers(
    handles: &EvidenceHandles,
    affected_factory_names: &HashSet<String>,
) {
    if affected_factory_names.is_empty() {
        return;
    }
    let keys: HashSet<&str> = affected_factory_names
        .iter()
        .map(|k| tcl_compiler::naming::unroot_rooted_key(k).unwrap_or(k))
        .collect();
    let candidates = handles
        .workspace_index
        .read()
        .await
        .documents_invoking_classes(&keys);
    if candidates.is_empty() {
        return;
    }
    let _rehoming_guard = handles.rehoming_gate.lock().await;
    let generation = *handles.class_factory_generation.read().await;
    let open: HashSet<String> = handles
        .documents
        .lock("warm_open_documents")
        .await
        .keys()
        .map(|u| u.as_str().to_owned())
        .collect();
    let global = *handles.db_config.lock().await;
    let folders = handles.folder_db_configs.lock().await.clone();
    let (oracle, work) = {
        let db = handles.db.lock().await;
        let files = handles.db_files.lock().await;
        let oracle = Backend::published_class_factories(&db, &files);
        let work = candidates
            .into_iter()
            .filter_map(|uri| {
                if open.contains(&uri) {
                    return None;
                }
                let uri = Uri::from_str(&uri).ok()?;
                let file = *files.get(&uri)?;
                let config = longest_folder_match(&folders, &uri)
                    .copied()
                    .unwrap_or(global);
                Some((
                    uri,
                    file,
                    config,
                    Backend::configured_analyser_from_db_config(config, &*db),
                ))
            })
            .collect::<Vec<_>>();
        (oracle, work)
    };
    if work.is_empty() {
        return;
    }
    let snapshot = handles
        .db
        .snapshot("reindex_unopened_factory_consumers")
        .await;
    let rows = crate::rt::spawn_blocking(move || {
        // Each analyser and partial row vector belongs to this worker and is
        // discarded on cancellation; no index publication occurs in the catch.
        salsa::Cancelled::catch(std::panic::AssertUnwindSafe(|| {
            work.into_iter()
                .map(|(uri, file, config, analyser)| {
                    let text = file.text(&*snapshot).clone();
                    let dialect = file.dialect(&*snapshot).clone();
                    let input = tcl_lsp_db::document_analysis_input(&*snapshot, file, config);
                    let analysis = match &input {
                        Ok(input) => analyser
                            .with_resolved_input(input.as_ref().clone())
                            .with_workspace_class_factories(oracle.clone())
                            .analyse(&text, &dialect),
                        Err(miss) => {
                            let mut result = AnalysisResult::default();
                            result.dialect.clone_from(&dialect);
                            result.analysis_context_unavailable = Some(miss.clone());
                            result
                        }
                    };
                    FactoryIndexSeed {
                        uri,
                        file,
                        text,
                        dialect,
                        input,
                        analysis,
                    }
                })
                .collect::<Vec<_>>()
        }))
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();
    publish_factory_index_seeds(handles, generation, &rows).await;
}

struct FactoryIndexSeed {
    pub(in super::super) uri: Uri,
    pub(in super::super) file: tcl_lsp_db::SourceFile,
    pub(in super::super) text: String,
    pub(in super::super) dialect: String,
    pub(in super::super) input: Result<
        Arc<tcl_compiler::analyser::ResolvedAnalysisInput>,
        tcl_registry::model::OverlayMiss,
    >,
    pub(in super::super) analysis: AnalysisResult,
}

async fn publish_factory_index_seeds(
    handles: &EvidenceHandles,
    generation: u64,
    rows: &[FactoryIndexSeed],
) {
    loop {
        let factory_generation = handles.class_factory_generation.read().await;
        if *factory_generation != generation {
            return;
        }
        let global = *handles.db_config.lock().await;
        let folders = handles.folder_db_configs.lock().await.clone();
        let docs = handles.documents.lock("factory_index_publish").await;
        let db = handles.db.lock().await;
        let files = handles.db_files.lock().await;
        let Ok(mut index) = handles.workspace_index.try_write() else {
            drop(files);
            drop(db);
            drop(docs);
            drop(factory_generation);
            drop(handles.workspace_index.write().await);
            continue;
        };
        for row in rows {
            let config = longest_folder_match(&folders, &row.uri)
                .copied()
                .unwrap_or(global);
            let source_current = match &row.input {
                Ok(input) => {
                    row.analysis.resolved_input.as_ref() == Some(input.as_ref())
                        && tcl_compiler::source_graph::current_analysis(&row.text, &row.analysis)
                            .is_some()
                }
                Err(miss) => {
                    row.analysis.analysis_context_unavailable.as_ref() == Some(miss)
                        && row.analysis.resolved_input.is_none()
                }
            };
            if !source_current
                || docs.contains_key(&row.uri)
                || files.get(&row.uri) != Some(&row.file)
                || row.file.text(&*db) != &row.text
                || row.file.dialect(&*db) != &row.dialect
                || tcl_lsp_db::document_analysis_input(&*db, row.file, config) != row.input
            {
                continue;
            }
            index.remove_document(row.uri.as_str());
            index.add_document(row.uri.as_str(), &row.analysis);
        }
        return;
    }
}

/// How many publish rounds [`sync_workspace_class_factories`] will run before
/// it gives up and logs.
///
/// The round function only ever *adds* an entry some file proved, so it
/// converges after one round per link of the longest metaclass chain plus one
/// to observe the fixpoint.  Real chains are two or three links
/// (`oo::class` → `Megawidget` → a widget's own metaclass); this leaves an
/// order of magnitude of headroom while still making the loop provably
/// bounded, so a pathological or half-edited workspace — one where an added
/// entry re-shapes a body such that a previously-proved entry disappears, and
/// the index oscillates instead of settling — costs a bounded amount of work
/// rather than spinning the diagnostics worker forever.
pub(in super::super) const CLASS_FACTORY_SYNC_ROUNDS: usize = 16;

/// Refresh every project file's [`tcl_lsp_db::SourceFile::workspace_class_factories`]
/// — the workspace's user-defined `TclOO` metaclasses — to a
/// **fixpoint**, and report the union of the URIs whose oracle moved across
/// all rounds.
///
/// One index serves every file: a metaclass is a project-wide declaration,
/// not a per-file slice, and there is no per-file narrowing to make.  It is
/// **empty for the overwhelming majority of workspaces**, and an empty index
/// is stored as `None`, so the input never leaves its default and nothing is
/// ever invalidated by it — a workspace with no metaclass pays nothing for the
/// index and settles in exactly one round that writes nothing.
///
/// **Why a fixpoint and not a single round.**  The index is
/// computed from [`tcl_lsp_db::project_class_factories_for_inputs`], which reads
/// each file's checked structure-only item tree, which reads the very input this function writes.  A
/// metaclass that is *itself* manufactured by another file's metaclass is
/// therefore provable only once the manufacturing metaclass is already
/// published: round 1 proves `MetaA` (written out as `oo::class create`, so
/// provable with no index at all), and only in round 2 can the file holding
/// `MetaA create MetaB` prove `MetaB`.  A single round leaves `MetaB` — and so
/// every class `MetaB` makes — unknown, so a cross-file three-level chain
/// resolves to nothing from a call site.  Each round is run
/// against a snapshot taken *after* the previous round's writes, so the next
/// link of the chain is visible to it.
///
/// **Termination.**  A round writes only what some file proved — never a
/// guess — so the normal case is monotone growth over a set bounded by the
/// project's creation calls, and the loop ends as soon as a round moves
/// nothing.  A cyclic declaration (`A` made by `B` made by `A`) proves
/// neither, so it settles empty on the first round.  Convergence is not
/// *assumed*, though: [`CLASS_FACTORY_SYNC_ROUNDS`] caps the loop, and hitting
/// the cap logs to stderr and returns what has been published so far, which is
/// a valid — if possibly incomplete — index, because every entry in it was
/// proved by some file.
///
/// Each round has the same shape as [`sync_cross_file_evidence`] and for the
/// same reasons: the project-wide read runs on a cloned salsa snapshot inside
/// `spawn_blocking` so the `db` mutex is held only for the short write
/// section, and the write is compare-then-set against the *live* database
/// rather than the snapshot.  A round the concurrent write cancelled publishes
/// nothing and is **not** the fixpoint — it is simply a round whose answer we
/// never learned, so the loop takes another one against a fresh snapshot
/// rather than mistaking "unknown" for "settled".  That retry is what the cap
/// bounds in the pathological case; without it a `did_open` arriving mid-pass
/// would truncate the chain at whatever link had been published so far, leaving
/// the deeper links to whatever unrelated edit happened to run the sync next.
///
/// `wake`, when supplied, republishes each round's invalidated peers **as that
/// round lands** rather than after the whole fixpoint.  A round only reads, but
/// the round *after* a publish has to recompute every file's `item_tree`
/// through the input the publish just moved, so on a large workspace the
/// confirming round — the one that merely agrees the index has stopped moving —
/// is the slowest part of the pass.  Left in front of the reschedule it delays
/// the republish that the *previous* round already earned, so a two-level chain
/// that is already provable stops resolving until the whole fixpoint lands.
/// Waking per round
/// keeps the fixpoint's extra rounds strictly additive: nothing that was
/// already provable waits on them.
///
/// Called from two places, because the answer must be in place before either
/// consumer needs it: at the end of the workspace scan (so a document opened
/// afterwards is analysed with the oracle on its very first pass; no peers to
/// wake there, since nothing has published yet) and after each diagnostics
/// publish (so a metaclass added by an *edit* propagates).
pub(in super::super) async fn sync_workspace_class_factories(
    handles: &EvidenceHandles,
    wake: Option<RoundReschedule<'_>>,
) -> ClassFactorySync {
    let before = published_factory_names(handles).await;
    let mut changed: Vec<Uri> = Vec::new();
    let mut seen: HashSet<Uri> = HashSet::new();
    let mut settled = false;
    for round in 0..CLASS_FACTORY_SYNC_ROUNDS {
        workspace_scan_trace!(
            "factory.round.begin sync={handles:p} round={round} wake={}",
            wake.is_some()
        );
        match sync_workspace_class_factories_round(handles).await {
            // The fixpoint: every file already carries the index the project
            // proves, so another round would recompute a byte-identical one
            // off unchanged inputs.
            ClassFactoryRound::Settled => {
                workspace_scan_trace!("factory.round.settled sync={handles:p} round={round}");
                settled = true;
                break;
            }
            ClassFactoryRound::Cancelled => {
                workspace_scan_trace!("factory.round.cancelled sync={handles:p} round={round}");
            }
            ClassFactoryRound::Moved(moved) => {
                workspace_scan_trace!(
                    "factory.round.moved sync={handles:p} round={round} uris={moved:?}"
                );
                if let Some(wake) = wake.as_ref() {
                    reschedule_peers(wake.slots, wake.self_uri, moved.clone()).await;
                }
                for uri in moved {
                    if seen.insert(uri.clone()) {
                        changed.push(uri);
                    }
                }
            }
        }
    }
    if !settled {
        eprintln!(
            "tcl-lsp: the workspace class-factory index did not settle in \
             {CLASS_FACTORY_SYNC_ROUNDS} rounds; publishing what has been proved so far. \
             Classes made by a metaclass deeper than that in the chain may not resolve \
             until the next sync."
        );
    }
    let mut affected_names = before;
    affected_names.extend(published_factory_names(handles).await);
    ClassFactorySync {
        moved: changed,
        affected_names,
    }
}

async fn published_factory_names(handles: &EvidenceHandles) -> HashSet<String> {
    let db = handles.db.lock().await;
    let files = handles.db_files.lock().await;
    Backend::published_class_factories(&db, &files)
        .as_deref()
        .into_iter()
        .flat_map(|index| index.keys().cloned())
        .collect()
}

/// What one [`sync_workspace_class_factories`] round learned.
///
/// `Settled` and `Cancelled` are deliberately distinct: both publish nothing,
/// but only the first is evidence that the index has stopped moving.
pub(in super::super) enum ClassFactoryRound {
    /// The published index already equals the one the project proves.
    Settled,
    /// These files' published index moved.
    Moved(Vec<Uri>),
    /// A concurrent write cancelled the read before it finished; nothing was
    /// published and nothing was learned.
    Cancelled,
}

/// One round of [`sync_workspace_class_factories`]: recompute the project's
/// class-factory index off a fresh snapshot and compare-then-set it on every
/// file.
pub(in super::super) async fn sync_workspace_class_factories_round(
    handles: &EvidenceHandles,
) -> ClassFactoryRound {
    let (snapshot, files, project, revision) = loop {
        // Register before the admission check so a concurrent publisher cannot
        // retire between refusal and waiting. No snapshot or publisher lock
        // survives the failed capture below.
        let publication_advanced = handles.db.snapshots.publication_advanced.notified();
        tokio::pin!(publication_advanced);
        publication_advanced.as_mut().enable();
        let captured = {
            let db = handles.db.lock().await;
            let db_files = handles.db_files.lock().await;
            let db_project = handles.db_project.lock().await;
            let Some(&project) = db_project.as_ref() else {
                // No project input yet — nothing to merge, and nothing a further
                // round could learn.
                return ClassFactoryRound::Settled;
            };
            let files: Vec<(Uri, tcl_lsp_db::SourceFile)> =
                db_files.iter().map(|(u, &f)| (u.clone(), f)).collect();
            let snapshot = handles
                .db
                .snapshot_from(&db, "sync_workspace_class_factories_round");
            if snapshot.is_none() {
                workspace_scan_trace!(
                    "factory.round.snapshot_wait sync={handles:p} reason=snapshot_unavailable"
                );
            }
            snapshot.map(|snapshot| (snapshot, files, project, tcl_lsp_db::database_revision(&db)))
        };
        if let Some(captured) = captured {
            break captured;
        }
        publication_advanced.await;
    };
    // `AssertUnwindSafe`: the closure only *reads* the database, and a
    // cancellation unwind discards the whole `pending` list without applying
    // anything, so no torn state can be observed afterwards.
    let computed = crate::rt::spawn_blocking(move || {
        salsa::Cancelled::catch(std::panic::AssertUnwindSafe(|| {
            let index = tcl_lsp_db::project_class_factories_for_inputs(&*snapshot, project);
            let want = (!index.is_empty()).then_some(index);
            let pending: Vec<(Uri, tcl_lsp_db::SourceFile)> = files
                .into_iter()
                .filter(|(_, file)| {
                    file.workspace_class_factories(&*snapshot).as_deref() != want.as_deref()
                })
                .collect();
            (want, pending)
        }))
        .ok()
    })
    .await;
    let Ok(Some((want, pending))) = computed else {
        workspace_scan_trace!(
            "factory.round.cancelled_reason sync={handles:p} reason=join_or_salsa_unavailable"
        );
        return ClassFactoryRound::Cancelled;
    };
    apply_workspace_class_factories_if_current(handles, revision, want, pending).await
}

pub(in super::super) async fn apply_workspace_class_factories_if_current(
    handles: &EvidenceHandles,
    revision: salsa::Revision,
    want: Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>>,
    pending: Vec<(Uri, tcl_lsp_db::SourceFile)>,
) -> ClassFactoryRound {
    use salsa::Setter as _;
    // Keep the existing generation-writer/database ordering. A source,
    // configuration, project mapping or oracle edit invalidates the captured
    // revision before any snapshot-derived hint can be published.
    let mut class_factory_generation = handles.class_factory_generation.write().await;
    let mut db = handles.db.lock().await;
    if tcl_lsp_db::database_revision(&db) != revision {
        workspace_scan_trace!(
            "factory.round.cancelled_reason sync={handles:p} reason=revision_changed captured={revision:?}"
        );
        return ClassFactoryRound::Cancelled;
    }
    let mut changed = Vec::with_capacity(pending.len());
    for (uri, file) in pending {
        if file.workspace_class_factories(&*db).as_deref() == want.as_deref() {
            continue;
        }
        file.set_workspace_class_factories(&mut *db)
            .to(want.clone());
        changed.push(uri);
    }
    if changed.is_empty() {
        ClassFactoryRound::Settled
    } else {
        *class_factory_generation = class_factory_generation.saturating_add(1);
        ClassFactoryRound::Moved(changed)
    }
}
