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

//! The walk over source edges from each root: the child an edge names, the frontier steps of a view and the scope the work settles.

use super::super::{Arc, Backend, DocumentSourceResult};
use super::{
    OriginalSourceRegionWork, OriginalSourceRootScope, OriginalSourceViewAssociation,
    OriginalSourceViewAttempt, OriginalSourceViewDebt, OriginalSourceViewDraft,
    OriginalSourceViewEdges, OriginalSourceViewFrontierStep, OriginalSourceViewId,
    OriginalSourceViewWorkState, PartialDefinitionReason, PublishedOriginalSourceView,
    SourceViewWork,
};
use std::collections::{HashMap, HashSet};
use tcl_compiler::registry_invocation::source_structure::{
    OriginalProjectSourceCurrentCut, OriginalProjectSourceFrontier as SourceFrontier,
};
use tower_lsp_server::ls_types::Uri;

impl OriginalSourceViewDebt {
    pub(super) fn same_request(&self, other: &Self) -> bool {
        self.owner.id == other.owner.id
            && self.owner.draft.same_live_producer(&other.owner.draft)
            && self.edge == other.edge
    }
}

impl OriginalSourceViewWorkState {
    fn finish_scope(
        self,
        root: OriginalSourceViewId,
        root_uri: Uri,
        complete: bool,
    ) -> OriginalSourceRootScope {
        let completion = self
            .finished
            .get(&root)
            .cloned()
            .flatten()
            .filter(|_| complete);
        let required = self
            .visited
            .into_iter()
            .chain(
                self.association_owners
                    .iter()
                    .map(|association| association.target.clone()),
            )
            .collect();
        OriginalSourceRootScope {
            recapture: None,
            root: Some(root),
            root_uri,
            required,
            association_owners: self.association_owners,
            ids: self.membership.into_iter().collect(),
            complete,
            partial_definition: None,
            completion,
            eligibility: HashSet::new(),
            conditional_obligations: self.conditional_obligations,
            root_world_uncertain: false,
        }
    }
}

impl Backend {
    pub(super) async fn cached_original_source_view(
        &self,
        uri: &Uri,
        inherited: Option<(
            &PublishedOriginalSourceView,
            &OriginalProjectSourceCurrentCut,
        )>,
    ) -> DocumentSourceResult<Option<Arc<PublishedOriginalSourceView>>> {
        let candidates = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            table
                .by_uri
                .get(uri)
                .into_iter()
                .flatten()
                .filter_map(|id| {
                    let view = table.by_id.get(id)?;
                    if view.draft.region.is_some() {
                        return None;
                    }
                    let matches = match (&view.draft.inherited, inherited) {
                        (None, None) => true,
                        (Some(actual), Some((parent, cut))) => {
                            actual.parent.id == parent.id
                                && actual.point.as_ref() == cut.point()
                                && view.draft.incoming_paths.as_ref().is_some_and(|advice| {
                                    advice.cut.as_ref() == cut && advice.matches_inherited(actual)
                                })
                        }
                        _ => false,
                    };
                    matches.then(|| Arc::clone(view))
                })
                .collect::<Vec<_>>()
        };
        if candidates.len() != 1 {
            return Ok(None);
        }
        let candidate = &candidates[0];
        // This returned Arc retains ownership only. Publication/render rejoins
        // its exact ID and complete ancestry again after every async phase.
        self.with_current_original_source_view(candidate.id, |view| Some(Arc::clone(view)))
            .await
    }

    fn source_view_target(
        parent: &PublishedOriginalSourceView,
        edge: &tcl_lsp_core::workspace_index::WorkspaceSource,
    ) -> Option<Uri> {
        Self::source_draft_target(&parent.draft, edge)
    }

    pub(super) fn source_draft_target(
        draft: &OriginalSourceViewDraft,
        edge: &tcl_lsp_core::workspace_index::WorkspaceSource,
    ) -> Option<Uri> {
        let expression = edge.original_path_expression.as_ref()?;
        let analysis = &draft.producer.seed.analysis;
        if !draft.matches_producer()
            || !expression.matches_analysis(analysis)
            || expression.source_span().start() > edge.range.start()
            || edge.range.start() >= expression.source_span().end()
        {
            return None;
        }
        let path = draft.physical.admission.path()?;
        let info_script = draft.physical.admission.analyser_path();
        let constants = match edge.original_source_frontier.as_deref() {
            Some(SourceFrontier::CurrentCut(cut))
                if edge.original_project_source_load.as_deref() == Some(cut.source_load()) =>
            {
                draft.path_constants_before_cut(cut)
            }
            _ => None,
        };
        let folded = expression.evaluate(info_script, &|name| {
            let constants = constants.as_ref()?;
            if draft.inherited.is_some() || draft.region.is_some() {
                let policy = constants.naming_policy()?;
                if policy
                    .recipe()
                    .variable_root_input(name.as_bytes())
                    .qualification()
                    != tcl_syntax::naming::NativeNameQualification::Absolute
                {
                    return None;
                }
            }
            tcl_compiler::auto_path_eval::PathConstantLookup::path_constant(
                &constants.at(edge.range.start()),
                name,
            )
        })?;
        let target = tcl_lsp_core::source_graph::resolve_source_target(path, &folded);
        super::super::super::canonical_file_uri(&target)
    }

    /// Heavy index construction runs on the actual retained owner outside
    /// publication leases; the exact ID/producer is rejoined after the worker.
    async fn original_source_view_edges(
        &self,
        view: Arc<PublishedOriginalSourceView>,
        unknown: &[OriginalSourceViewDebt],
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<OriginalSourceViewEdges>> {
        let saved = Arc::clone(&view);
        let unknown = unknown.to_vec();
        let edges = super::super::super::rt::spawn_blocking(move || {
            use tcl_lsp_core::original_declaration::OriginalSourceViewPublication;
            use tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView;
            let publication = OriginalSourceViewPublication::retain(saved.id);
            let source = Arc::clone(&saved.draft.physical.source);
            let analysis = Arc::clone(&saved.draft.producer.seed.analysis);
            let row = match &saved.draft.region {
                None => WorkspaceOriginalSourceView::capture(publication, source, analysis),
                Some(owned) => WorkspaceOriginalSourceView::capture_region(
                    publication,
                    source,
                    analysis,
                    owned.region.clone(),
                ),
            }?;
            let edges = row.source_edges(saved.draft.physical.uri.as_str())?;
            let replayed = unknown
                .into_iter()
                .filter(|debt| {
                    let Some(owned) = saved.draft.region.as_ref() else {
                        return false;
                    };
                    if debt.owner.id != owned.parent.id
                        || !debt.owner.draft.same_live_producer(&owned.parent.draft)
                    {
                        return false;
                    }
                    let Some(load) = debt.edge.original_project_source_load.as_deref() else {
                        return false;
                    };
                    let Some((load, frontier)) = owned.region.reissued_source_frontier(
                        &debt.owner.draft.producer.seed.analysis,
                        load,
                        &saved.draft.producer.seed.analysis,
                    ) else {
                        return false;
                    };
                    // Even Unknown transfers only to this exact reissued card.
                    // The successor is classified again below; it grants no
                    // negative lookup or completion merely by matching bytes.
                    edges.iter().any(|edge| {
                        edge.original_project_source_load.as_deref() == Some(&load)
                            && edge.original_source_frontier.as_deref() == Some(&frontier)
                    })
                })
                .collect();
            Some(OriginalSourceViewEdges { edges, replayed })
        })
        .await
        .map_err(|_| super::super::super::DocumentSourceUnavailable::Worker {
            uri: view.draft.physical.uri.clone(),
        })?;
        let Some(edges) = edges else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let current = self
            .with_current_original_source_view(view.id, |current| {
                current
                    .draft
                    .same_live_producer(&view.draft)
                    .then(|| edges.clone())
            })
            .await?;
        Ok(current.map_or(
            OriginalSourceViewAttempt::RetryCapture,
            OriginalSourceViewAttempt::Ready,
        ))
    }

    /// A genuine current association removes competing independent authoring
    /// roots, never sibling inherited interpretations. Retirement is atomic
    /// with compatibility-row withdrawal under the publication transaction.
    async fn retire_associated_original_source_roots(
        &self,
        parent: &PublishedOriginalSourceView,
        target: &Uri,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<()>> {
        let draft = &parent.draft;
        let epoch = draft.producer.seed.analyser_inputs_epoch;
        let factory = draft.producer.seed.class_factory_generation;
        let Some(saved) = self
            .readonly_saved_sources(&[&draft.physical.publication])
            .await?
        else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        let result = self
            .readonly_source_transaction(
                epoch,
                factory,
                &[draft.physical.admission.as_ref()],
                Some(&saved),
                |state| {
                    draft
                        .matches_physical_state(state, epoch, factory)
                        .then_some(())
                },
                |_, ()| {
                    let _publication = self.rehoming_gate.try_lock().ok()?;
                    let mut table = self.original_source_views.try_lock().ok()?;
                    let mut index = self.workspace_index.try_write().ok()?;
                    (table.contains_actual(parent)
                        && table.parent_chain_is_live(draft)
                        && parent.is_indexed(&index))
                    .then_some(())?;
                    let retired = table
                        .by_uri
                        .get(target)
                        .into_iter()
                        .flatten()
                        .filter_map(|id| {
                            table
                                .by_id
                                .get(id)
                                .filter(|view| !view.draft.is_interpreted())
                                .map(|_| *id)
                        })
                        .collect::<Vec<_>>();
                    if retired.is_empty() {
                        return Some(());
                    }
                    // Recursive same-root associations cannot retire their own
                    // active prefix and then claim successful publication.
                    (!retired.contains(&parent.id)).then_some(())?;
                    table.retire_ids(retired);
                    let live = table.by_id.keys().copied().map(
                    tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain,
                ).collect::<Vec<_>>();
                    index.retain_original_source_view_publications(&live)?;
                    Some(())
                },
            )
            .await?;
        Ok(match result {
            super::super::ReadonlyTransaction::Published(()) => {
                OriginalSourceViewAttempt::Ready(())
            }
            super::super::ReadonlyTransaction::CurrencyRefused
            | super::super::ReadonlyTransaction::PublishRefused => {
                OriginalSourceViewAttempt::RetryCapture
            }
        })
    }

    /// A first source edge admits its closed child through the normal disk
    /// publisher before capturing an inherited interpretation at the real cut.
    pub(super) async fn source_child_document_snapshot(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<super::super::super::RehomingDocumentSnapshot> {
        let snapshot = self.rehoming_document_snapshot(uri).await?;
        if !matches!(
            snapshot,
            super::super::super::RehomingDocumentSnapshot::Missing
        ) {
            return Ok(snapshot);
        }
        let admission = self.document_path_admission(uri).await?;
        self.reindex_index_from_disk_using(uri, Some(admission))
            .await?;
        self.rehoming_document_snapshot(uri).await
    }

    async fn source_view_child(
        &self,
        parent: &Arc<PublishedOriginalSourceView>,
        edge: &tcl_lsp_core::workspace_index::WorkspaceSource,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<Arc<PublishedOriginalSourceView>>> {
        let Some(uri) = Self::source_view_target(parent, edge) else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let Some(
            tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceFrontier::CurrentCut(cut),
        ) = edge.original_source_frontier.as_deref() else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        if !cut.matches_parent_analysis(&parent.draft.producer.seed.analysis)
            || edge.original_project_source_load.as_deref() != Some(cut.source_load())
        {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        }
        if let Some(cached) = self
            .cached_original_source_view(&uri, Some((parent, cut)))
            .await?
        {
            return Ok(OriginalSourceViewAttempt::Ready(cached));
        }
        let snapshot = self.source_child_document_snapshot(&uri).await?;
        let document = match snapshot {
            super::super::super::RehomingDocumentSnapshot::Ready(document) => document,
            super::super::super::RehomingDocumentSnapshot::Pending => {
                return Ok(OriginalSourceViewAttempt::RetryCapture);
            }
            super::super::super::RehomingDocumentSnapshot::Missing => {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
        };
        let draft = match self
            .capture_inherited_original_source_view(
                &uri,
                document.text,
                Arc::clone(parent),
                Arc::new(cut.clone()),
            )
            .await?
        {
            OriginalSourceViewAttempt::Ready(draft) => draft,
            OriginalSourceViewAttempt::RetryCapture => {
                return Ok(OriginalSourceViewAttempt::RetryCapture);
            }
            OriginalSourceViewAttempt::Unavailable => {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
        };
        self.append_inherited_original_source_view(&uri, draft)
            .await
    }

    async fn original_source_view_frontier_step(
        &self,
        parent: &Arc<PublishedOriginalSourceView>,
        state: &mut OriginalSourceViewWorkState,
    ) -> DocumentSourceResult<OriginalSourceViewFrontierStep> {
        let batch = match self
            .original_source_view_edges(Arc::clone(parent), &state.unknown)
            .await?
        {
            OriginalSourceViewAttempt::Ready(batch) => batch,
            OriginalSourceViewAttempt::RetryCapture => {
                return Ok(OriginalSourceViewFrontierStep::RetryCapture);
            }
            OriginalSourceViewAttempt::Unavailable => {
                return Ok(OriginalSourceViewFrontierStep::Pending);
            }
        };
        state.unknown.retain(|debt| {
            !batch
                .replayed
                .iter()
                .any(|mapped| debt.same_request(mapped))
        });
        state.association_owners.retain(|association| {
            !batch.replayed.iter().any(|mapped| {
                association.owner.id == mapped.owner.id
                    && association
                        .owner
                        .draft
                        .same_live_producer(&mapped.owner.draft)
                    && association.edge == mapped.edge
            })
        });
        let edges = batch.edges;
        if edges.is_empty() {
            return Ok(OriginalSourceViewFrontierStep::NoCut);
        }
        // Source order, reachability and missing points cannot choose a
        // frontier. Only the actual finite kernel cut may suspend this view.
        let analysis = &parent.draft.producer.seed.analysis;
        let mut selected = None;
        let mut unknown = false;
        for edge in &edges {
            if let Some(target) = Self::source_view_target(parent, edge) {
                if !state.association_owners.iter().any(|old| {
                    old.target == target && old.owner.id == parent.id && old.edge == *edge
                }) {
                    state
                        .association_owners
                        .push(OriginalSourceViewAssociation {
                            target: target.clone(),
                            owner: Arc::clone(parent),
                            edge: edge.clone(),
                        });
                }
                match self
                    .retire_associated_original_source_roots(parent, &target)
                    .await?
                {
                    OriginalSourceViewAttempt::Ready(()) => {}
                    OriginalSourceViewAttempt::RetryCapture => {
                        return Ok(OriginalSourceViewFrontierStep::RetryCapture);
                    }
                    OriginalSourceViewAttempt::Unavailable => {
                        return Ok(OriginalSourceViewFrontierStep::Pending);
                    }
                }
            }
            match edge.original_source_frontier.as_deref() {
                Some(SourceFrontier::CurrentCut(cut))
                    if cut.matches_parent_analysis(analysis)
                        && edge.original_project_source_load.as_deref()
                            == Some(cut.source_load()) =>
                {
                    if selected.replace(edge).is_some() {
                        return Ok(OriginalSourceViewFrontierStep::Pending);
                    }
                }
                Some(SourceFrontier::ConditionalAssociation(association))
                    if association.matches_parent_analysis(analysis)
                        && edge.original_project_source_load.as_deref()
                            == Some(association.source_load()) =>
                {
                    // Prefix scheduling is available, but aggregate/negative
                    // coverage keeps the actual conditional body's obligations.
                    state.conditional_obligations = true;
                }
                _ => {
                    unknown = true;
                    let debt = OriginalSourceViewDebt {
                        owner: Arc::clone(parent),
                        edge: edge.clone(),
                    };
                    if !state.unknown.iter().any(|old| old.same_request(&debt)) {
                        state.unknown.push(debt);
                    }
                }
            }
        }
        Ok(match selected {
            Some(edge) => OriginalSourceViewFrontierStep::Cut(Box::new(edge.clone())),
            None if unknown => OriginalSourceViewFrontierStep::Pending,
            None => OriginalSourceViewFrontierStep::NoCut,
        })
    }

    async fn visit_original_source_view(
        &self,
        parent: Arc<PublishedOriginalSourceView>,
        state: &mut OriginalSourceViewWorkState,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<()>> {
        if state.finished.contains_key(&parent.id) {
            return Ok(OriginalSourceViewAttempt::Ready(()));
        }
        if !state.active.insert(parent.id) {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        }
        state.membership.insert(parent.id);
        state.visited.insert(parent.draft.physical.uri.clone());
        let edge = match self
            .original_source_view_frontier_step(&parent, state)
            .await?
        {
            OriginalSourceViewFrontierStep::NoCut => {
                state.active.remove(&parent.id);
                state.finished.insert(parent.id, None);
                return Ok(OriginalSourceViewAttempt::Ready(()));
            }
            OriginalSourceViewFrontierStep::Pending => {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
            OriginalSourceViewFrontierStep::RetryCapture => {
                return Ok(OriginalSourceViewAttempt::RetryCapture);
            }
            OriginalSourceViewFrontierStep::Cut(edge) => edge,
        };
        let child = match self.source_view_child(&parent, &edge).await? {
            OriginalSourceViewAttempt::Ready(child) => child,
            OriginalSourceViewAttempt::RetryCapture => {
                return Ok(OriginalSourceViewAttempt::RetryCapture);
            }
            OriginalSourceViewAttempt::Unavailable => {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
        };
        let Some(SourceFrontier::CurrentCut(cut)) = edge.original_source_frontier.as_deref() else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let load = Arc::new(cut.source_load().clone());
        state.tasks.push(SourceViewWork::AfterChild {
            parent,
            child: Arc::clone(&child),
            load,
        });
        state.tasks.push(SourceViewWork::Enter(child));
        Ok(OriginalSourceViewAttempt::Ready(()))
    }

    /// Every issued region must keep its source-frontier debts even when the
    /// kernel already owns a conditional normal route. Nonfinal frame regions
    /// cannot hide a cut or be reordered behind a later outer suffix.
    async fn reconcile_issued_region_frontiers(
        &self,
        work: &OriginalSourceRegionWork,
        state: &mut OriginalSourceViewWorkState,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<()>> {
        for (index, region) in work.views.iter().enumerate() {
            let is_unfinished_tail = !work.chain.complete() && index + 1 == work.views.len();
            if is_unfinished_tail {
                continue;
            }
            match self
                .original_source_view_frontier_step(region, state)
                .await?
            {
                OriginalSourceViewFrontierStep::NoCut => {}
                OriginalSourceViewFrontierStep::RetryCapture => {
                    return Ok(OriginalSourceViewAttempt::RetryCapture);
                }
                _ => return Ok(OriginalSourceViewAttempt::Unavailable),
            }
        }
        Ok(OriginalSourceViewAttempt::Ready(()))
    }

    /// Continue the same graph work item; no lease or retry budget leaves it.
    async fn continue_original_source_child(
        &self,
        parent: Arc<PublishedOriginalSourceView>,
        child: Arc<PublishedOriginalSourceView>,
        load: Arc<tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad>,
        state: &mut OriginalSourceViewWorkState,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<()>> {
        let Some(completed_child) = state.finished.get(&child.id) else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let work = match self
            .materialize_conditional_source_regions(
                Arc::clone(&parent),
                child,
                load,
                completed_child.as_ref(),
            )
            .await?
        {
            OriginalSourceViewAttempt::Ready(work) => work,
            OriginalSourceViewAttempt::RetryCapture => {
                return Ok(OriginalSourceViewAttempt::RetryCapture);
            }
            OriginalSourceViewAttempt::Unavailable => {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
        };
        state
            .membership
            .extend(work.views.iter().map(|view| view.id));
        match self.reconcile_issued_region_frontiers(&work, state).await? {
            OriginalSourceViewAttempt::Ready(()) => {}
            OriginalSourceViewAttempt::RetryCapture => {
                return Ok(OriginalSourceViewAttempt::RetryCapture);
            }
            OriginalSourceViewAttempt::Unavailable => {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
        }
        if let Some(completion) = work.completion() {
            state.active.remove(&parent.id);
            state.finished.insert(parent.id, Some(completion));
        } else {
            // An incomplete chain still stops at its exact final issued region.
            let Some(last) = work.views.last().cloned() else {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            };
            state.tasks.push(SourceViewWork::AfterRegion {
                work,
                region: Arc::clone(&last),
            });
            state.tasks.push(SourceViewWork::Enter(last));
        }
        Ok(OriginalSourceViewAttempt::Ready(()))
    }

    pub(super) async fn run_original_source_view_work(
        &self,
        root: Arc<PublishedOriginalSourceView>,
    ) -> DocumentSourceResult<OriginalSourceRootScope> {
        let root_id = root.id;
        let root_uri = root.draft.physical.uri.clone();
        let root_analysis = Arc::clone(&root.draft.producer.seed.analysis);
        let mut state = OriginalSourceViewWorkState {
            visited: HashSet::from([root.draft.physical.uri.clone()]),
            association_owners: Vec::new(),
            conditional_obligations: false,
            tasks: vec![SourceViewWork::Enter(root)],
            active: HashSet::new(),
            finished: HashMap::new(),
            membership: HashSet::new(),
            unknown: Vec::new(),
        };
        while let Some(task) = state.tasks.pop() {
            if state.membership.len() > 4096 {
                return Ok(state.finish_scope(root_id, root_uri, false));
            }
            match task {
                SourceViewWork::Enter(parent) => {
                    match self.visit_original_source_view(parent, &mut state).await? {
                        OriginalSourceViewAttempt::Ready(()) => {}
                        OriginalSourceViewAttempt::RetryCapture => {
                            return Ok(state.finish_scope(root_id, root_uri, false).retrying());
                        }
                        OriginalSourceViewAttempt::Unavailable => {
                            return Ok(state.finish_scope(root_id, root_uri, false));
                        }
                    }
                }
                SourceViewWork::AfterChild {
                    parent,
                    child,
                    load,
                } => {
                    match self
                        .continue_original_source_child(parent, child, load, &mut state)
                        .await?
                    {
                        OriginalSourceViewAttempt::Ready(()) => {}
                        OriginalSourceViewAttempt::RetryCapture => {
                            return Ok(state.finish_scope(root_id, root_uri, false).retrying());
                        }
                        OriginalSourceViewAttempt::Unavailable => {
                            return Ok(state.finish_scope(root_id, root_uri, false));
                        }
                    }
                }
                SourceViewWork::AfterRegion { work, region } => {
                    let Some(Some(completed)) = state.finished.get(&region.id) else {
                        let eligible = matches!(state.finished.get(&region.id), Some(None))
                            && state.tasks.is_empty()
                            && state.unknown.is_empty()
                            && !state.conditional_obligations
                            && work.parent.id == root_id;
                        let mut scope = state.finish_scope(root_id, root_uri, false);
                        scope.partial_definition = eligible
                            .then_some(PartialDefinitionReason::UnfinishedSuffixWithoutFurtherCut);
                        return Ok(scope);
                    };
                    let Some(completion) = work.completion_through_region(completed) else {
                        return Ok(state.finish_scope(root_id, root_uri, false));
                    };
                    state.active.remove(&work.parent.id);
                    state.finished.insert(work.parent.id, Some(completion));
                }
            }
        }
        // Finishing a real first cut cannot silently discharge unrelated or
        // held Unknown cards outside the actual replayed region inventory.
        if !state.unknown.is_empty() {
            return Ok(state.finish_scope(root_id, root_uri, false));
        }
        let no_cut = matches!(state.finished.get(&root_id), Some(None));
        let mut scope = state.finish_scope(root_id, root_uri, true);
        // `eval $unknown` can rebind any name the root reaches. Without a
        // retained realm the answer is unknown unless the root's world completed.
        scope.root_world_uncertain = no_cut
            && root_analysis
                .root_unknown_command_mutation()
                .unwrap_or_else(|| root_analysis.original_completed_command_world().is_none());
        Ok(scope)
    }
}
