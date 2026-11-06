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

//! Retained source interpretations and their independent physical publications.
//!
//! Source-only interpretations retain physical leases independently of loading,
//! whole-program completeness, Native execution and edit authority.
//! A view key is a locator; only a live table join under the source lease answers.

mod symbol_sources;
pub(super) use symbol_sources::WorkspaceSymbolViewGuards;
pub(crate) use symbol_sources::{WorkspaceSymbolSourceView, WorkspaceSymbolSourceViewCache};

use super::{
    Arc, Backend, DocumentSourceResult, ReadonlySourceDocument, ReadonlySourceState,
    WorkspaceSymbolFreshInput, WorkspaceSymbolSourceAnalysis,
    original_readonly_analysis_matches_input,
};
use std::collections::{HashMap, HashSet};
use tcl_compiler::auto_path_eval::{
    FoldedPathConstants, PathConstantAssignments, PathConstantValue,
};
use tcl_compiler::command_binding::OriginalSourcePointEntry;
use tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceCurrentCut;
use tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceFrontier as SourceFrontier;
use tower_lsp_server::ls_types::Uri;

/// Conditional globals issued from this exact live parent's pre-handler cut.
/// The values supply neither runtime stores nor absence of namespace shadows.
#[derive(Clone)]
struct IncomingSourcePathAdvice {
    parent: Arc<PublishedOriginalSourceView>,
    cut: Arc<OriginalProjectSourceCurrentCut>,
    constants: Option<FoldedPathConstants>,
}

impl IncomingSourcePathAdvice {
    fn matches_inherited(&self, inherited: &InheritedSourceView) -> bool {
        Arc::ptr_eq(&self.parent, &inherited.parent)
            && self.cut.point() == inherited.point.as_ref()
            && self
                .cut
                .matches_parent_analysis(&self.parent.draft.producer.seed.analysis)
    }

    fn same_request(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.parent, &other.parent)
            && self.cut == other.cut
            && self.constants == other.constants
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct OriginalSourceViewId(u64);

#[derive(Clone)]
struct InheritedSourceView {
    parent: Arc<PublishedOriginalSourceView>,
    point: Arc<OriginalSourcePointEntry>,
}

#[derive(Clone)]
struct ConditionalRegionSourceView {
    parent: Arc<PublishedOriginalSourceView>,
    child_prefix: Arc<PublishedOriginalSourceView>,
    completion_dependencies: Vec<Arc<PublishedOriginalSourceView>>,
    chain: Arc<tcl_compiler::command_binding::OriginalSourceConditionalRegionSet>,
    region: tcl_compiler::command_binding::OriginalSourceConditionalRegion,
}

impl ConditionalRegionSourceView {
    fn same_request(&self, other: &Self) -> bool {
        self.parent.id == other.parent.id
            && self.child_prefix.id == other.child_prefix.id
            && self.chain == other.chain
            && self.region == other.region
            && self.completion_dependencies.len() == other.completion_dependencies.len()
            && self
                .completion_dependencies
                .iter()
                .zip(&other.completion_dependencies)
                .all(|(left, right)| left.id == right.id)
    }
}

#[derive(Clone)]
pub(crate) struct OriginalSourceViewDraft {
    physical: Arc<ReadonlySourceDocument>,
    producer: Arc<WorkspaceSymbolFreshInput>,
    inherited: Option<InheritedSourceView>,
    region: Option<ConditionalRegionSourceView>,
    incoming_paths: Option<Arc<IncomingSourcePathAdvice>>,
}

pub(crate) struct PublishedOriginalSourceView {
    id: OriginalSourceViewId,
    draft: OriginalSourceViewDraft,
}

/// Tentative ordinary authoring input, held privately until actual source
/// associations select roots. It supplies no locator or child interpretation.
#[derive(Clone)]
struct OriginalSourceRootCandidate {
    draft: OriginalSourceViewDraft,
    published: Option<Arc<PublishedOriginalSourceView>>,
}

struct OriginalSourceRootWork {
    candidates: Vec<OriginalSourceRootCandidate>,
    pending: HashSet<Uri>,
    associated: HashSet<Uri>,
    incoming: HashMap<Uri, HashSet<Uri>>,
}

struct OriginalSourceRootReconciliation {
    bootstrap: Vec<OriginalSourceViewDraft>,
    scopes: Vec<OriginalSourceRootScope>,
    excluded: Vec<OriginalSourceRootCandidate>,
    incoming: HashMap<Uri, HashSet<Uri>>,
}

/// Session-owned sequence is never reset when the workspace index is replaced.
#[derive(Default)]
pub(crate) struct OriginalSourceViewTable {
    next_id: u64,
    revision: u64,
    revision_exhausted: bool,
    by_id: HashMap<OriginalSourceViewId, Arc<PublishedOriginalSourceView>>,
    by_uri: HashMap<Uri, Vec<OriginalSourceViewId>>,
}

impl OriginalSourceViewDraft {
    fn matches_producer(&self) -> bool {
        let physical = &self.physical;
        let producer = &self.producer;
        if self.incoming_paths.as_ref().is_some_and(|advice| {
            self.inherited
                .as_ref()
                .is_none_or(|inherited| !advice.matches_inherited(inherited))
        }) {
            return false;
        }
        if physical.uri != producer.uri
            || physical.source != producer.snapshot.text
            || physical.publication.uri_generation != producer.uri_generation
            || !producer.open_snapshot_matches(&physical.publication.document)
            || physical.publication.open_revision.is_some() != producer.was_open
            || physical.admission != producer.seed.path_admission
            || physical.config != producer.config
            || physical.configuration != producer.configuration
            || !original_readonly_analysis_matches_input(
                &physical.source,
                &producer.seed.analysis,
                &producer.input,
            )
        {
            return false;
        }
        if let Some(region) = &self.region {
            return self.inherited.is_none()
                && region.chain.regions().contains(&region.region)
                && region.region.matches_dependencies(
                    &region.parent.draft.producer.seed.analysis,
                    &region.child_prefix.draft.producer.seed.analysis,
                )
                && region
                    .region
                    .matches_region_analysis(&producer.seed.analysis);
        }
        match &self.inherited {
            Some(inherited) => {
                inherited
                    .point
                    .matches_parent_analysis(&inherited.parent.draft.producer.seed.analysis)
                    && inherited
                        .point
                        .matches_child_analysis(&producer.seed.analysis)
            }
            // Only the private initial-root producer constructor can mint
            // this variant; inherited/missing-point work cannot call it.
            None => physical.input == producer.input,
        }
    }

    fn path_constants_before_cut(
        &self,
        cut: &OriginalProjectSourceCurrentCut,
    ) -> Option<FoldedPathConstants> {
        if !self.matches_producer() {
            return None;
        }
        let raw = cut.path_advice_before_handler(&self.producer.seed.analysis)?;
        self.fold_source_path_prefix(&raw)
    }

    fn fold_source_path_prefix(
        &self,
        raw: &PathConstantAssignments,
    ) -> Option<FoldedPathConstants> {
        let empty = FoldedPathConstants::default();
        let mut imported = &empty;
        if self.inherited.is_some() || self.region.is_some() {
            if raw.is_empty() {
                if let Some(advice) = &self.incoming_paths {
                    imported = advice.constants.as_ref()?;
                }
            } else {
                // Value-only exports cannot prove unknown incoming shadows absent.
                // Only variable-free own RHSs can supply new inherited advice.
                let filename = self.physical.admission.analyser_path();
                if !raw.iter().all(|write| match &write.value {
                    PathConstantValue::Original(expression) => {
                        expression.matches_assignments(raw)
                            && expression.evaluate(filename, &|_| None).is_some()
                    }
                    _ => false,
                }) {
                    return None;
                }
            }
        }
        Some(
            tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(
                raw,
                self.physical.admission.analyser_path(),
                imported,
            ),
        )
    }

    fn matches_physical_state(
        &self,
        state: &ReadonlySourceState<'_>,
        epoch: u64,
        factory: u64,
    ) -> bool {
        let physical = &self.physical;
        let producer = &self.producer;
        // Source-view currency checks the actual source/config/provider inputs.
        // The existing report's global database-revision guard stays unchanged:
        // unrelated child materialisation cannot renew or retire this producer.
        physical.publication.matches_open_documents(state.open)
            && state
                .uri_generations
                .get(&physical.uri)
                .copied()
                .unwrap_or_default()
                == physical.publication.uri_generation
            && state.files.get(&physical.uri) == Some(&physical.file)
            && state.published.get(&physical.uri).is_some_and(|published| {
                published.file == physical.file
                    && published.admission.as_ref() == physical.admission.as_ref()
            })
            && physical.file.path(state.db).as_deref() == physical.admission.analyser_path()
            && physical.file.text(state.db).as_str() == physical.source.as_ref()
            && physical.file.dialect(state.db) == &physical.publication.document.dialect
            && physical.config.diagnostic_report_configuration(state.db) == physical.configuration
            && { tcl_lsp_db::document_analysis_input(state.db, physical.file, physical.config) }
                .is_ok_and(|input| input.as_ref() == physical.input.as_ref())
            && producer.uri_generation_matches(state)
            && producer.matches_input_state(state, epoch, factory)
            && self.matches_producer()
    }

    fn same_live_producer(&self, other: &Self) -> bool {
        // This is reuse of an already current producer, never standalone
        // reanalysis or reconstruction from equal bytes/configuration.
        Arc::ptr_eq(&self.producer, &other.producer)
            && Arc::ptr_eq(&self.physical, &other.physical)
            && match (&self.incoming_paths, &other.incoming_paths) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_request(right),
                _ => false,
            }
            && match (&self.region, &other.region) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_request(right),
                _ => false,
            }
            && match (&self.inherited, &other.inherited) {
                (None, None) => true,
                (Some(a), Some(b)) => a.parent.id == b.parent.id && a.point == b.point,
                _ => false,
            }
    }

    fn same_view_request(&self, other: &Self) -> bool {
        // Subtractive duplicate detection only; equality cannot admit a view
        // or replace the same-live-producer reuse/currentness checks.
        let a = &self.physical;
        let b = &other.physical;
        a.uri == b.uri
            && a.file == b.file
            && a.admission == b.admission
            && a.source == b.source
            && a.input == b.input
            && a.config == b.config
            && a.configuration == b.configuration
            && a.publication.uri_generation == b.publication.uri_generation
            && a.publication.open_revision == b.publication.open_revision
            && self
                .producer
                .open_snapshot_matches(&other.producer.snapshot)
            && match (&self.incoming_paths, &other.incoming_paths) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_request(right),
                _ => false,
            }
            && match (&self.region, &other.region) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_request(right),
                _ => false,
            }
            && match (&self.inherited, &other.inherited) {
                (None, None) => true,
                (Some(a), Some(b)) => a.parent.id == b.parent.id && a.point == b.point,
                _ => false,
            }
    }

    fn direct_dependencies(&self) -> Vec<Arc<PublishedOriginalSourceView>> {
        if let Some(region) = &self.region {
            let mut owners = vec![Arc::clone(&region.parent), Arc::clone(&region.child_prefix)];
            owners.extend(region.completion_dependencies.iter().cloned());
            owners
        } else {
            self.inherited
                .as_ref()
                .map(|inherited| vec![Arc::clone(&inherited.parent)])
                .unwrap_or_default()
        }
    }

    fn is_interpreted(&self) -> bool {
        self.inherited.is_some() || self.region.is_some()
    }

    /// A region continues its own physical owner; only an inherited view makes
    /// the URI another root's interpreted member.
    fn is_interpreted_member(&self) -> bool {
        self.inherited.is_some()
    }

    fn ancestors(&self) -> Option<Vec<Arc<PublishedOriginalSourceView>>> {
        let mut ancestors = Vec::new();
        let mut active = HashSet::new();
        let mut complete = HashSet::new();
        let mut pending = self
            .direct_dependencies()
            .into_iter()
            .map(|view| (view, false))
            .collect::<Vec<_>>();
        while let Some((view, exiting)) = pending.pop() {
            if exiting {
                active.remove(&view.id);
                complete.insert(view.id);
                continue;
            }
            if complete.contains(&view.id) {
                continue;
            }
            if ancestors.len() >= 4096 || !active.insert(view.id) {
                return None;
            }
            pending.push((Arc::clone(&view), true));
            pending.extend(
                view.draft
                    .direct_dependencies()
                    .into_iter()
                    .map(|view| (view, false)),
            );
            ancestors.push(view);
        }
        Some(ancestors)
    }
}

impl OriginalSourceViewTable {
    fn current_revision(&self) -> Option<u64> {
        (!self.revision_exhausted).then_some(self.revision)
    }

    fn advance_revision(&mut self) {
        if let Some(next) = self.revision.checked_add(1) {
            self.revision = next;
        } else {
            self.revision_exhausted = true;
        }
    }

    fn contains_actual(&self, view: &PublishedOriginalSourceView) -> bool {
        self.by_uri
            .get(&view.draft.physical.uri)
            .is_some_and(|ids| ids.contains(&view.id))
            && self
                .by_id
                .get(&view.id)
                .is_some_and(|current| current.draft.same_live_producer(&view.draft))
    }

    fn parent_chain_is_live(&self, draft: &OriginalSourceViewDraft) -> bool {
        draft.ancestors().is_some_and(|ancestors| {
            ancestors
                .iter()
                .all(|ancestor| self.contains_actual(ancestor))
        })
    }

    fn reuse_current_batch(
        &self,
        uri: &Uri,
        drafts: &[OriginalSourceViewDraft],
        index: &tcl_lsp_core::workspace_index::WorkspaceIndex,
    ) -> Option<Vec<Arc<PublishedOriginalSourceView>>> {
        let all_ids = self.by_uri.get(uri)?;
        let existing = all_ids
            .iter()
            .filter(|id| {
                self.by_id
                    .get(id)
                    .is_some_and(|view| view.draft.region.is_none())
            })
            .collect::<Vec<_>>();
        if existing.len() != drafts.len()
            || index.original_source_views(uri.as_str()).len() != all_ids.len()
        {
            return None;
        }

        existing
            .iter()
            .zip(drafts)
            .map(|(id, draft)| {
                let view = self.by_id.get(*id)?;
                (view.is_indexed(index) && view.draft.same_live_producer(draft))
                    .then(|| Arc::clone(view))
            })
            .collect()
    }

    fn stage(
        &self,
        drafts: Vec<OriginalSourceViewDraft>,
    ) -> Option<(u64, Vec<(OriginalSourceViewId, OriginalSourceViewDraft)>)> {
        let mut next_id = self.next_id;
        let mut staged = Vec::with_capacity(drafts.len());
        for draft in drafts {
            if !draft.matches_producer() || !self.parent_chain_is_live(&draft) {
                return None;
            }
            if staged
                .iter()
                .any(|(_, previous): &(_, OriginalSourceViewDraft)| {
                    previous.same_view_request(&draft)
                })
            {
                return None;
            }
            let existing = self
                .by_id
                .values()
                .find(|view| view.draft.same_live_producer(&draft));
            let id = if let Some(view) = existing {
                view.id
            } else {
                next_id = next_id.checked_add(1)?;
                OriginalSourceViewId(next_id)
            };
            staged.push((id, draft));
        }
        Some((next_id, staged))
    }

    /// Retire both the changed physical owner and every retained descendant.
    /// URI generations and this table use the same URI-generation -> table order.
    pub(crate) fn retire_uri(&mut self, uri: &Uri) {
        // Private bootstrap participants may have no issued view yet. Their
        // physical invalidation must still invalidate an in-flight projection.
        self.advance_revision();
        let retired = self.by_uri.get(uri).cloned().unwrap_or_default();
        self.retire_ids(retired);
    }

    fn retire_ids(&mut self, mut retired: Vec<OriginalSourceViewId>) {
        if retired.is_empty() {
            return;
        }
        self.advance_revision();
        loop {
            let more = self
                .by_id
                .iter()
                .filter_map(|(id, view)| {
                    (!retired.contains(id)
                        && view.draft.ancestors().is_none_or(|ancestors| {
                            ancestors.iter().any(|parent| retired.contains(&parent.id))
                        }))
                    .then_some(*id)
                })
                .collect::<Vec<_>>();
            if more.is_empty() {
                break;
            }
            retired.extend(more);
        }
        for id in &retired {
            self.by_id.remove(id);
        }
        self.by_uri.retain(|_, ids| {
            ids.retain(|id| !retired.contains(id));
            !ids.is_empty()
        });
    }

    fn append_regions(
        &mut self,
        uri: &Uri,
        next_id: u64,
        views: &[Arc<PublishedOriginalSourceView>],
    ) {
        // All validation precedes the index commit under this same table lock.
        self.advance_revision();
        self.next_id = next_id;
        self.by_uri
            .entry(uri.clone())
            .or_default()
            .extend(views.iter().map(|view| view.id));
        for view in views {
            self.by_id.insert(view.id, Arc::clone(view));
        }
    }

    fn replace(&mut self, uri: &Uri, next_id: u64, views: &[Arc<PublishedOriginalSourceView>]) {
        let kept = views.iter().map(|view| view.id).collect::<HashSet<_>>();
        let removed = self
            .by_uri
            .get(uri)
            .into_iter()
            .flatten()
            .copied()
            .filter(|id| !kept.contains(id))
            .collect();
        self.retire_ids(removed);
        self.advance_revision();
        self.next_id = next_id;
        self.by_uri
            .insert(uri.clone(), views.iter().map(|view| view.id).collect());
        for view in views {
            self.by_id.insert(view.id, Arc::clone(view));
        }
        // Unchanged actual producers keep their issued IDs when a sibling
        // interpretation is removed. Only removed IDs and their descendants
        // retire; full physical/config invalidation still calls retire_uri.
    }
}

/// Publication currency can be recaptured; absent semantic producers cannot.
/// This carries no authority until the existing final source/index lease joins.
pub(crate) enum OriginalSourceViewAttempt<T> {
    Ready(T),
    RetryCapture,
    Unavailable,
}

impl<T> OriginalSourceViewAttempt<T> {
    fn from_semantic(value: Option<T>) -> Self {
        value.map_or(Self::Unavailable, Self::Ready)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceViewPublicationMode {
    ReplaceFirstEdge,
    AppendInherited,
    AppendRegions,
}

impl Backend {
    /// Commit all interpretations of one physical source in the same index
    /// transaction. Pending/unavailable required drafts never call this method.
    pub(crate) async fn publish_original_source_views(
        &self,
        uri: &Uri,
        drafts: Vec<OriginalSourceViewDraft>,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<Vec<Arc<PublishedOriginalSourceView>>>>
    {
        self.publish_original_source_view_batch(
            uri,
            drafts,
            SourceViewPublicationMode::ReplaceFirstEdge,
        )
        .await
    }

    async fn append_inherited_original_source_view(
        &self,
        uri: &Uri,
        draft: OriginalSourceViewDraft,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<Arc<PublishedOriginalSourceView>>> {
        Ok(
            match self
                .publish_original_source_view_batch(
                    uri,
                    vec![draft],
                    SourceViewPublicationMode::AppendInherited,
                )
                .await?
            {
                OriginalSourceViewAttempt::Ready(mut views) => {
                    OriginalSourceViewAttempt::from_semantic(
                        (views.len() == 1).then(|| views.pop()).flatten(),
                    )
                }
                OriginalSourceViewAttempt::RetryCapture => OriginalSourceViewAttempt::RetryCapture,
                OriginalSourceViewAttempt::Unavailable => OriginalSourceViewAttempt::Unavailable,
            },
        )
    }

    async fn append_original_source_regions(
        &self,
        uri: &Uri,
        drafts: Vec<OriginalSourceViewDraft>,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<Vec<Arc<PublishedOriginalSourceView>>>>
    {
        self.publish_original_source_view_batch(
            uri,
            drafts,
            SourceViewPublicationMode::AppendRegions,
        )
        .await
    }

    fn original_source_view_publications(
        sources: &[Arc<ReadonlySourceDocument>],
    ) -> Vec<&super::super::LibraryPublicationSource> {
        sources
            .iter()
            .map(|source| &source.publication)
            .collect::<Vec<_>>()
    }

    fn original_source_view_admissions(
        sources: &[Arc<ReadonlySourceDocument>],
    ) -> Vec<&super::super::DocumentPathAdmission> {
        sources
            .iter()
            .map(|source| source.admission.as_ref())
            .collect::<Vec<_>>()
    }

    fn original_source_view_id_for_handle(
        &self,
        handle: tcl_lsp_core::original_declaration::OriginalDeclarationHandle,
    ) -> Option<OriginalSourceViewId> {
        let Ok(registry) = self.original_declaration_registry.try_lock() else {
            return None;
        };
        registry
            .source_view_publication::<OriginalSourceViewId>(handle)
            .copied()
    }

    fn source_view_call_groups(
        views: &[Arc<PublishedOriginalSourceView>],
        incoming: bool,
        publication: &tcl_lsp_core::original_declaration::OriginalSourceViewPublication,
        worker_identity: &tcl_lsp_core::original_declaration::OriginalDeclarationIdentity,
    ) -> Option<
        Vec<(
            tcl_lsp_core::original_source_views::OriginalSourceViewDeclaration,
            Vec<tower_lsp_server::ls_types::Range>,
        )>,
    > {
        let (publications, ancestors) = Self::original_source_view_labels(views)?;
        let inventory = Self::original_source_view_inventory(views, &publications, &ancestors);
        let mut groups: Vec<(
            tcl_lsp_core::original_source_views::OriginalSourceViewDeclaration,
            Vec<tower_lsp_server::ls_types::Range>,
        )> = Vec::new();
        for edge in tcl_lsp_core::original_source_views::procedure_edges(&inventory) {
            let (selected, other) = if incoming {
                (&edge.target, &edge.caller)
            } else {
                (&edge.caller, &edge.target)
            };
            if selected.publication != *publication || selected.identity != *worker_identity {
                continue;
            }
            let caller = inventory
                .iter()
                .find(|row| row.publication == &edge.caller.publication)?;
            let range = tcl_lsp_core::definition::span_to_range(
                caller.document.source,
                &tcl_lexer::LineIndex::new_lsp(caller.document.source),
                edge.span,
            );
            let range = super::super::lift_lsp_range(range);
            if let Some((_, ranges)) = groups.iter_mut().find(|(key, _)| {
                key.publication == other.publication && key.identity == other.identity
            }) {
                if !ranges.contains(&range) {
                    ranges.push(range);
                }
            } else {
                groups.push((other.clone(), vec![range]));
            }
        }
        Some(groups)
    }

    fn original_source_view_batch_ancestors(
        uri: &Uri,
        drafts: &[OriginalSourceViewDraft],
        mode: SourceViewPublicationMode,
        epoch: u64,
        factory: u64,
    ) -> Option<Vec<Vec<Arc<PublishedOriginalSourceView>>>> {
        if drafts.iter().any(|draft| {
            &draft.physical.uri != uri
                || draft.producer.seed.analyser_inputs_epoch != epoch
                || draft.producer.seed.class_factory_generation != factory
                // Recursive physical imports remain unavailable. Only a sealed
                // region may append to its own prefix's physical URI.
                || (mode != SourceViewPublicationMode::AppendRegions
                    && draft.ancestors().is_none_or(|ancestors| {
                        ancestors.iter().any(|parent| &parent.draft.physical.uri == uri)
                    }))
                || (mode == SourceViewPublicationMode::AppendRegions && draft.region.is_none())
                || (mode == SourceViewPublicationMode::AppendInherited
                    && (draft.inherited.is_none() || draft.region.is_some()))
        }) {
            return None;
        }
        drafts
            .iter()
            .map(OriginalSourceViewDraft::ancestors)
            .collect::<Option<Vec<_>>>()
    }

    fn original_source_view_batch_sources(
        drafts: &[OriginalSourceViewDraft],
        ancestors: &[Vec<Arc<PublishedOriginalSourceView>>],
    ) -> Vec<Arc<ReadonlySourceDocument>> {
        drafts
            .iter()
            .map(|draft| Arc::clone(&draft.physical))
            .chain(
                ancestors
                    .iter()
                    .flatten()
                    .map(|view| Arc::clone(&view.draft.physical)),
            )
            .collect::<Vec<_>>()
    }

    /// Build ownership carriers; the caller retains every host publication lock.
    fn original_source_view_rows(
        staged: &[(OriginalSourceViewId, OriginalSourceViewDraft)],
    ) -> Option<Vec<tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView>> {
        staged
            .iter()
            .map(|(id, draft)| {
                let publication =
                    tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(*id);
                let source = Arc::clone(&draft.physical.source);
                let analysis = Arc::clone(&draft.producer.seed.analysis);
                match &draft.region {
                    None => tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView::capture(
                        publication,
                        source,
                        analysis,
                    ),
                    Some(owned) => {
                        tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView::capture_region(
                            publication,
                            source,
                            analysis,
                            owned.region.clone(),
                        )
                    }
                }
            })
            .collect()
    }

    /// Issue only inside the current source transaction while the caller holds
    /// the rehoming, table and index guards; this helper cannot renew a lease.
    fn commit_original_source_view_batch(
        uri: &Uri,
        drafts: Vec<OriginalSourceViewDraft>,
        mode: SourceViewPublicationMode,
        ancestors: &[Vec<Arc<PublishedOriginalSourceView>>],
        validation_drafts: &[OriginalSourceViewDraft],
        table: &mut OriginalSourceViewTable,
        index: &mut tcl_lsp_core::workspace_index::WorkspaceIndex,
    ) -> Option<OriginalSourceViewAttempt<Vec<Arc<PublishedOriginalSourceView>>>> {
        if ancestors
            .iter()
            .flatten()
            .any(|view| !table.contains_actual(view) || !view.is_indexed(index))
        {
            return None;
        }
        if mode == SourceViewPublicationMode::ReplaceFirstEdge
            && validation_drafts
                .iter()
                .all(|draft| !draft.is_interpreted())
            && table.by_uri.get(uri).into_iter().flatten().any(|id| {
                table
                    .by_id
                    .get(id)
                    .is_some_and(|view| view.draft.is_interpreted_member())
            })
        {
            return Some(OriginalSourceViewAttempt::Unavailable);
        }
        if mode == SourceViewPublicationMode::ReplaceFirstEdge
            && let Some(existing) = table.reuse_current_batch(uri, &drafts, index)
        {
            return Some(OriginalSourceViewAttempt::Ready(existing));
        }
        let Some((next_id, staged)) = table.stage(drafts) else {
            return Some(OriginalSourceViewAttempt::Unavailable);
        };
        if mode != SourceViewPublicationMode::ReplaceFirstEdge
            && (staged.iter().any(|(id, _)| table.by_id.contains_key(id))
                || (mode == SourceViewPublicationMode::AppendRegions
                    && table.by_uri.get(uri).is_none_or(Vec::is_empty)))
        {
            return None;
        }
        let Some(indexed) = Self::original_source_view_rows(&staged) else {
            return Some(OriginalSourceViewAttempt::Unavailable);
        };
        let published = match mode {
            SourceViewPublicationMode::ReplaceFirstEdge => {
                index.replace_original_source_views(uri.as_str(), indexed)
            }
            SourceViewPublicationMode::AppendInherited => {
                index.append_original_source_interpretations(uri.as_str(), indexed)
            }
            SourceViewPublicationMode::AppendRegions => {
                index.append_original_source_regions(uri.as_str(), indexed)
            }
        };
        published?;
        let views = staged
            .into_iter()
            .map(|(id, draft)| Arc::new(PublishedOriginalSourceView { id, draft }))
            .collect::<Vec<_>>();
        match mode {
            SourceViewPublicationMode::ReplaceFirstEdge => table.replace(uri, next_id, &views),
            SourceViewPublicationMode::AppendInherited
            | SourceViewPublicationMode::AppendRegions => {
                table.append_regions(uri, next_id, &views);
            }
        }
        Some(OriginalSourceViewAttempt::Ready(views))
    }

    async fn publish_original_source_view_batch(
        &self,
        uri: &Uri,
        drafts: Vec<OriginalSourceViewDraft>,
        mode: SourceViewPublicationMode,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<Vec<Arc<PublishedOriginalSourceView>>>>
    {
        let Some(first) = drafts.first() else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let epoch = first.producer.seed.analyser_inputs_epoch;
        let factory = first.producer.seed.class_factory_generation;
        let Some(ancestors) =
            Self::original_source_view_batch_ancestors(uri, &drafts, mode, epoch, factory)
        else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let sources = Self::original_source_view_batch_sources(&drafts, &ancestors);
        let publications = Self::original_source_view_publications(&sources);
        let Some(saved) = self.readonly_saved_sources(&publications).await? else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        let admissions = Self::original_source_view_admissions(&sources);
        let validation_drafts = drafts.clone();
        let mut drafts = Some(drafts);
        let transaction = self
            .readonly_source_transaction(
                epoch,
                factory,
                &admissions,
                Some(&saved),
                |state| {
                    (validation_drafts
                        .iter()
                        .all(|draft| draft.matches_physical_state(state, epoch, factory))
                        && ancestors
                            .iter()
                            .flatten()
                            .all(|view| view.draft.matches_physical_state(state, epoch, factory)))
                    .then_some(())
                },
                |_state, ()| {
                    let _rehoming = self.rehoming_gate.try_lock().ok()?;
                    let mut table = self.original_source_views.try_lock().ok()?;
                    let mut index = self.workspace_index.try_write().ok()?;
                    let drafts = drafts.take()?;
                    Self::commit_original_source_view_batch(
                        uri,
                        drafts,
                        mode,
                        &ancestors,
                        &validation_drafts,
                        &mut table,
                        &mut index,
                    )
                },
            )
            .await?;
        Ok(match transaction {
            super::ReadonlyTransaction::Published(outcome) => outcome,
            super::ReadonlyTransaction::CurrencyRefused
            | super::ReadonlyTransaction::PublishRefused => OriginalSourceViewAttempt::RetryCapture,
        })
    }
}

impl Backend {
    /// A saved locator rejoins its own still-live table row before any metadata
    /// lookup. It never analyses a URI/text pair to reconstruct a missing view.
    async fn with_current_original_source_view<T>(
        &self,
        id: OriginalSourceViewId,
        mut render: impl FnMut(&Arc<PublishedOriginalSourceView>) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        let view = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            table.by_id.get(&id).cloned()
        };
        let Some(view) = view else {
            return Ok(None);
        };
        let Some(ancestors) = view.draft.ancestors() else {
            return Ok(None);
        };
        let sources = std::iter::once(&view)
            .chain(ancestors.iter())
            .collect::<Vec<_>>();
        let publications = sources
            .iter()
            .map(|source| &source.draft.physical.publication)
            .collect::<Vec<_>>();
        let Some(saved) = self.readonly_saved_sources(&publications).await? else {
            return Ok(None);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        let admissions = sources
            .iter()
            .map(|source| source.draft.physical.admission.as_ref())
            .collect::<Vec<_>>();
        let producer = &view.draft.producer.seed;
        let epoch = producer.analyser_inputs_epoch;
        let factory = producer.class_factory_generation;
        self.with_current_readonly_source_state(
            producer.analyser_inputs_epoch,
            producer.class_factory_generation,
            &admissions,
            Some(&saved),
            |state| {
                sources
                    .iter()
                    .all(|source| source.draft.matches_physical_state(state, epoch, factory))
                    .then_some(())
            },
            |_state, ()| {
                let _rehoming = self.rehoming_gate.try_lock().ok()?;
                let table = self.original_source_views.try_lock().ok()?;
                let index = self.workspace_index.try_read().ok()?;
                if sources
                    .iter()
                    .any(|source| !source.is_indexed(&index) || !table.contains_actual(source))
                {
                    return None;
                }
                render(&view)
            },
        )
        .await
    }
}

impl super::super::FreshAnalysisDriver {
    fn analyse_conditional_region(
        mut self: Box<Self>,
        region: &tcl_compiler::command_binding::OriginalSourceConditionalRegion,
        parent: &tcl_compiler::analyser::AnalysisResult,
        child_prefix: &tcl_compiler::analyser::AnalysisResult,
    ) -> DocumentSourceResult<Option<super::super::FreshAnalysisSeed>> {
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let analysis = self
            .analyser
            .analyse_source_region(region, parent, child_prefix);
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let path_admission = self.path_admission;
        let analyser_inputs_epoch = &self.analyser_inputs_epoch;
        let class_factory_generation = &self.class_factory_generation;
        Ok(analysis.map(|analysis| super::super::FreshAnalysisSeed {
            path_admission,
            analysis: Arc::new(analysis),
            analyser_inputs_epoch: *analyser_inputs_epoch,
            class_factory_generation: *class_factory_generation,
        }))
    }

    fn analyse_source_view(
        mut self: Box<Self>,
        source: &str,
        dialect: &str,
        point: &OriginalSourcePointEntry,
    ) -> DocumentSourceResult<Option<super::super::FreshAnalysisSeed>> {
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let source = tcl_lexer::normalise_lone_cr(source);
        let analysis = self
            .analyser
            .analyse_with_source_point_entry(&source, dialect, point);
        self.path_admission
            .check_current(self.source_store.as_ref())?;
        let path_admission = self.path_admission;
        let analyser_inputs_epoch = &self.analyser_inputs_epoch;
        let class_factory_generation = &self.class_factory_generation;
        Ok(analysis.map(|analysis| super::super::FreshAnalysisSeed {
            path_admission,
            analysis: Arc::new(analysis),
            analyser_inputs_epoch: *analyser_inputs_epoch,
            class_factory_generation: *class_factory_generation,
        }))
    }
}

impl Backend {
    /// Initial root production only. The reconciler must select this route
    /// from an actual root work item; a missing inherited entry never calls it.
    async fn capture_initial_original_source_root(
        &self,
        uri: &Uri,
        document: super::super::DocumentState,
    ) -> DocumentSourceResult<Option<OriginalSourceViewDraft>> {
        let Some(physical) = self
            .capture_readonly_source_document(uri.as_str(), Arc::clone(&document.text))
            .await?
        else {
            return Ok(None);
        };
        let Some(analysis) = self
            .capture_fresh_source_analysis(
                uri,
                document,
                super::FreshSourceAnalysisPurpose::ReadonlyComplete,
            )
            .await?
        else {
            return Ok(None);
        };
        let super::WorkspaceSymbolAnalysisKind::Fresh(producer) = analysis.kind else {
            return Ok(None);
        };
        let draft = OriginalSourceViewDraft {
            physical: Arc::new(physical),
            producer: Arc::from(producer),
            inherited: None,
            region: None,
            incoming_paths: None,
        };
        Ok(draft.matches_producer().then_some(draft))
    }

    /// Child production uses the exact still-live parent point once. A child
    /// refusal remains unavailable rather than entering a root/namespace route.
    async fn capture_inherited_original_source_view(
        &self,
        uri: &Uri,
        source: Arc<str>,
        parent: Arc<PublishedOriginalSourceView>,
        cut: Arc<OriginalProjectSourceCurrentCut>,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<OriginalSourceViewDraft>> {
        let point = Arc::new(cut.point().clone());
        if self
            .with_current_original_source_view(parent.id, |current| {
                (current.draft.same_live_producer(&parent.draft)
                    && point.matches_parent_analysis(&current.draft.producer.seed.analysis))
                .then_some(())
            })
            .await?
            .is_none()
        {
            return Ok(OriginalSourceViewAttempt::RetryCapture);
        }
        let Some(physical) = self
            .capture_readonly_source_document(uri.as_str(), source)
            .await?
        else {
            return Ok(OriginalSourceViewAttempt::RetryCapture);
        };
        let mut driver = self
            .fresh_analysis_driver(Arc::clone(&physical.admission))
            .await;
        // Physical child configuration is retained independently; the
        // inherited interpreter input belongs to the exact actual point.
        driver.analyser = driver
            .analyser
            .with_resolved_input(parent.draft.producer.input.as_ref().clone());
        if !point.configure_analyser(&parent.draft.producer.seed.analysis, &mut driver.analyser) {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        }
        let text = Arc::clone(&physical.source);
        let dialect = parent.draft.producer.seed.analysis.dialect.clone();
        let point_for_worker = Arc::clone(&point);
        let seed = super::super::rt::spawn_blocking(move || {
            driver.analyse_source_view(&text, &dialect, &point_for_worker)
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker { uri: uri.clone() })??;
        let Some(seed) = seed else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let Some(interpretation_input) = seed.analysis.resolved_input.clone().map(Arc::new) else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let producer = Arc::new(WorkspaceSymbolFreshInput {
            purpose: super::FreshSourceAnalysisPurpose::ReadonlyComplete,
            uri: uri.clone(),
            snapshot: physical.publication.document.clone(),
            was_open: physical.publication.open_revision.is_some(),
            seed,
            input: interpretation_input,
            config: physical.config,
            configuration: physical.configuration.clone(),
            database_revision: physical.database_revision,
            uri_generation: physical.publication.uri_generation,
        });
        let incoming_paths = Some(Arc::new(IncomingSourcePathAdvice {
            constants: parent
                .draft
                .path_constants_before_cut(&cut)
                .map(|values| values.exported()),
            parent: Arc::clone(&parent),
            cut,
        }));
        let draft = OriginalSourceViewDraft {
            physical: Arc::new(physical),
            producer,
            inherited: Some(InheritedSourceView { parent, point }),
            region: None,
            incoming_paths,
        };
        Ok(OriginalSourceViewAttempt::from_semantic(
            draft.matches_producer().then_some(draft),
        ))
    }
}

/// A compiler-issued completion retains every physical view used to derive it.
/// A saved outcome alone supplies no host publication currency.
#[derive(Clone)]
struct OriginalSourceViewCompletion {
    prefix: Arc<PublishedOriginalSourceView>,
    outcome: tcl_compiler::command_binding::OriginalSourceConditionalOutcome,
    dependencies: Vec<Arc<PublishedOriginalSourceView>>,
}

struct OriginalSourceRegionWork {
    parent: Arc<PublishedOriginalSourceView>,
    child_prefix: Arc<PublishedOriginalSourceView>,
    chain: Arc<tcl_compiler::command_binding::OriginalSourceConditionalRegionSet>,
    views: Vec<Arc<PublishedOriginalSourceView>>,
    completion_dependencies: Vec<Arc<PublishedOriginalSourceView>>,
}

impl OriginalSourceRegionWork {
    /// A kernel-issued outcome remains ownership only until the host checks
    /// this exact prefix and all retained physical dependencies again.
    fn completion(&self) -> Option<OriginalSourceViewCompletion> {
        if !self.chain.complete() {
            return None;
        }
        let outcome = self.chain.normal_outcome()?;
        outcome
            .matches_prefix_analysis(&self.parent.draft.producer.seed.analysis)
            .then_some(())?;
        Some(self.retain_completion(outcome, &[]))
    }

    fn completion_through_region(
        &self,
        completed: &OriginalSourceViewCompletion,
    ) -> Option<OriginalSourceViewCompletion> {
        let mut matches = self.views.iter().filter(|view| {
            view.id == completed.prefix.id && view.draft.same_live_producer(&completed.prefix.draft)
        });
        let view = matches.next()?;
        matches.next().is_none().then_some(())?;
        let owned = view.draft.region.as_ref()?;
        (owned.chain == self.chain).then_some(())?;
        let outcome = owned.region.return_outcome(
            &self.parent.draft.producer.seed.analysis,
            &self.child_prefix.draft.producer.seed.analysis,
            &view.draft.producer.seed.analysis,
            &completed.outcome,
        )?;
        Some(self.retain_completion(outcome, &completed.dependencies))
    }

    fn retain_completion(
        &self,
        outcome: tcl_compiler::command_binding::OriginalSourceConditionalOutcome,
        additional: &[Arc<PublishedOriginalSourceView>],
    ) -> OriginalSourceViewCompletion {
        let mut dependencies = self.completion_dependencies.clone();
        for view in std::iter::once(&self.child_prefix)
            .chain(&self.views)
            .chain(additional)
        {
            if !dependencies.iter().any(|old| old.id == view.id) {
                dependencies.push(Arc::clone(view));
            }
        }
        OriginalSourceViewCompletion {
            prefix: Arc::clone(&self.parent),
            outcome,
            dependencies,
        }
    }
}

impl Backend {
    async fn capture_conditional_region_draft(
        &self,
        request: ConditionalRegionSourceView,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<OriginalSourceViewDraft>> {
        let ids = std::iter::once(request.parent.id)
            .chain(std::iter::once(request.child_prefix.id))
            .chain(request.completion_dependencies.iter().map(|view| view.id))
            .collect::<Vec<_>>();
        if self
            .with_original_source_scope(&ids, |views| {
                let dependencies = [request.parent.as_ref(), request.child_prefix.as_ref()];
                dependencies
                    .into_iter()
                    .chain(request.completion_dependencies.iter().map(AsRef::as_ref))
                    .all(|saved| {
                        views.iter().any(|current| {
                            current.id == saved.id && current.draft.same_live_producer(&saved.draft)
                        })
                    })
                    .then_some(())
            })
            .await?
            .is_none()
        {
            return Ok(OriginalSourceViewAttempt::RetryCapture);
        }
        let physical = Arc::clone(&request.parent.draft.physical);
        let mut driver = self
            .fresh_analysis_driver(Arc::clone(&physical.admission))
            .await;
        driver.analyser = driver
            .analyser
            .with_resolved_input(request.parent.draft.producer.input.as_ref().clone());
        let region = request.region.clone();
        let parent = Arc::clone(&request.parent.draft.producer.seed.analysis);
        let child = Arc::clone(&request.child_prefix.draft.producer.seed.analysis);
        let seed = super::super::rt::spawn_blocking(move || {
            driver.analyse_conditional_region(&region, &parent, &child)
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
            uri: physical.uri.clone(),
        })??;
        let Some(seed) = seed else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let Some(input) = seed.analysis.resolved_input.clone().map(Arc::new) else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let producer = Arc::new(WorkspaceSymbolFreshInput {
            purpose: super::FreshSourceAnalysisPurpose::ConditionalSourceRegion,
            uri: physical.uri.clone(),
            snapshot: physical.publication.document.clone(),
            was_open: physical.publication.open_revision.is_some(),
            seed,
            input,
            config: physical.config,
            configuration: physical.configuration.clone(),
            database_revision: physical.database_revision,
            uri_generation: physical.publication.uri_generation,
        });
        let draft = OriginalSourceViewDraft {
            physical,
            producer,
            inherited: None,
            region: Some(request),
            incoming_paths: None,
        };
        Ok(OriginalSourceViewAttempt::from_semantic(
            draft.matches_producer().then_some(draft),
        ))
    }

    async fn cached_conditional_source_region(
        &self,
        request: &ConditionalRegionSourceView,
    ) -> DocumentSourceResult<Option<Arc<PublishedOriginalSourceView>>> {
        let candidates = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            table
                .by_id
                .values()
                .filter(|view| {
                    view.draft
                        .region
                        .as_ref()
                        .is_some_and(|owned| owned.same_request(request))
                })
                .map(|view| view.id)
                .collect::<Vec<_>>()
        };
        if candidates.len() != 1 {
            return Ok(None);
        }
        self.with_current_original_source_view(candidates[0], |view| Some(Arc::clone(view)))
            .await
    }

    fn completed_source_child_matches_view(
        completed: &OriginalSourceViewCompletion,
        child_prefix: &PublishedOriginalSourceView,
    ) -> bool {
        completed.prefix.id == child_prefix.id
            && completed
                .prefix
                .draft
                .same_live_producer(&child_prefix.draft)
            && completed
                .outcome
                .matches_prefix_analysis(&child_prefix.draft.producer.seed.analysis)
    }

    /// Ask the existing kernel for the exact ordered suffix chain. There is no
    /// fresh whole-parent analysis or URI/text reconstruction on this route.
    async fn materialize_conditional_source_regions(
        &self,
        parent: Arc<PublishedOriginalSourceView>,
        child_prefix: Arc<PublishedOriginalSourceView>,
        load: Arc<tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad>,
        completed_child: Option<&OriginalSourceViewCompletion>,
    ) -> DocumentSourceResult<OriginalSourceViewAttempt<OriginalSourceRegionWork>> {
        let outcome = if let Some(completed) = completed_child {
            if !Self::completed_source_child_matches_view(completed, &child_prefix) {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
            Some(completed.outcome.clone())
        } else {
            None
        };
        let completion_dependencies = completed_child
            .map(|completed| completed.dependencies.clone())
            .unwrap_or_default();
        let parent_analysis = Arc::clone(&parent.draft.producer.seed.analysis);
        let child_analysis = Arc::clone(&child_prefix.draft.producer.seed.analysis);
        let chain = super::super::rt::spawn_blocking(move || {
            let continuation = if let Some(outcome) = outcome.as_ref() {
                load.conditional_continuation_with_outcome(
                    &parent_analysis,
                    &child_analysis,
                    outcome,
                )
            } else {
                load.conditional_continuation(&parent_analysis, &child_analysis)
            }?;
            continuation
                .source_regions(&parent_analysis, &child_analysis)
                .map(Arc::new)
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
            uri: parent.draft.physical.uri.clone(),
        })?;
        let Some(chain) = chain else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        let mut views = Vec::new();
        let mut pending = Vec::new();
        for region in chain.regions() {
            let request = ConditionalRegionSourceView {
                parent: Arc::clone(&parent),
                child_prefix: Arc::clone(&child_prefix),
                completion_dependencies: completion_dependencies.clone(),
                chain: Arc::clone(&chain),
                region: region.clone(),
            };
            if let Some(cached) = self.cached_conditional_source_region(&request).await? {
                views.push(Some(cached));
            } else {
                let draft = match self.capture_conditional_region_draft(request).await? {
                    OriginalSourceViewAttempt::Ready(draft) => draft,
                    OriginalSourceViewAttempt::RetryCapture => {
                        return Ok(OriginalSourceViewAttempt::RetryCapture);
                    }
                    OriginalSourceViewAttempt::Unavailable => {
                        return Ok(OriginalSourceViewAttempt::Unavailable);
                    }
                };
                pending.push((views.len(), draft));
                views.push(None);
            }
        }
        if !pending.is_empty() {
            let (positions, drafts): (Vec<_>, Vec<_>) = pending.into_iter().unzip();
            let published = match self
                .append_original_source_regions(&parent.draft.physical.uri, drafts)
                .await?
            {
                OriginalSourceViewAttempt::Ready(published) => published,
                OriginalSourceViewAttempt::RetryCapture => {
                    return Ok(OriginalSourceViewAttempt::RetryCapture);
                }
                OriginalSourceViewAttempt::Unavailable => {
                    return Ok(OriginalSourceViewAttempt::Unavailable);
                }
            };
            if published.len() != positions.len() {
                return Ok(OriginalSourceViewAttempt::Unavailable);
            }
            for (position, view) in positions.into_iter().zip(published) {
                views[position] = Some(view);
            }
        }
        let Some(views) = views.into_iter().collect::<Option<Vec<_>>>() else {
            return Ok(OriginalSourceViewAttempt::Unavailable);
        };
        Ok(OriginalSourceViewAttempt::Ready(OriginalSourceRegionWork {
            parent,
            child_prefix,
            chain,
            views,
            completion_dependencies,
        }))
    }
}

/// One unresolved source association belongs to the exact retained view.
/// It can move to a successor only through the compiler's direct replay join.
#[derive(Clone)]
struct OriginalSourceViewDebt {
    owner: Arc<PublishedOriginalSourceView>,
    edge: tcl_lsp_core::workspace_index::WorkspaceSource,
}

impl OriginalSourceViewDebt {
    fn same_request(&self, other: &Self) -> bool {
        self.owner.id == other.owner.id
            && self.owner.draft.same_live_producer(&other.owner.draft)
            && self.edge == other.edge
    }
}

#[derive(Clone)]
struct OriginalSourceViewEdges {
    edges: Vec<tcl_lsp_core::workspace_index::WorkspaceSource>,
    replayed: Vec<OriginalSourceViewDebt>,
}

struct OriginalSourceViewAssociation {
    target: Uri,
    owner: Arc<PublishedOriginalSourceView>,
    edge: tcl_lsp_core::workspace_index::WorkspaceSource,
}

struct OriginalSourceViewWorkState {
    tasks: Vec<SourceViewWork>,
    active: HashSet<OriginalSourceViewId>,
    finished: HashMap<OriginalSourceViewId, Option<OriginalSourceViewCompletion>>,
    membership: HashSet<OriginalSourceViewId>,
    unknown: Vec<OriginalSourceViewDebt>,
    visited: HashSet<Uri>,
    association_owners: Vec<OriginalSourceViewAssociation>,
    conditional_obligations: bool,
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

enum OriginalSourceViewFrontierStep {
    NoCut,
    Pending,
    RetryCapture,
    Cut(Box<tcl_lsp_core::workspace_index::WorkspaceSource>),
}

enum SourceViewWork {
    Enter(Arc<PublishedOriginalSourceView>),
    AfterChild {
        parent: Arc<PublishedOriginalSourceView>,
        child: Arc<PublishedOriginalSourceView>,
        load: Arc<tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad>,
    },
    AfterRegion {
        work: OriginalSourceRegionWork,
        region: Arc<PublishedOriginalSourceView>,
    },
}

/// Host membership of one bounded exact source-view reconciliation. Kernel
/// conditional outcomes remain separate from physical/Native completion.
#[derive(Clone, Copy)]
enum PartialDefinitionReason {
    UnfinishedSuffixWithoutFurtherCut,
}

/// Recapture eligibility only; supplies no source, index or completion proof.
struct OriginalSourceViewRecapture;

struct OriginalSourceRootScope {
    recapture: Option<OriginalSourceViewRecapture>,
    root: Option<OriginalSourceViewId>,
    root_uri: Uri,
    required: HashSet<Uri>,
    association_owners: Vec<OriginalSourceViewAssociation>,
    ids: Vec<OriginalSourceViewId>,
    complete: bool,
    partial_definition: Option<PartialDefinitionReason>,
    completion: Option<OriginalSourceViewCompletion>,
    eligibility: HashSet<Uri>,
    conditional_obligations: bool,
    /// A root without a source cut whose own command world may hold an
    /// unknown mutation. Only positive references consume it.
    root_world_uncertain: bool,
}

impl OriginalSourceRootScope {
    fn retrying(mut self) -> Self {
        self.recapture = Some(OriginalSourceViewRecapture);
        self.complete = false;
        self.partial_definition = None;
        self.completion = None;
        // No row rejected by publication is surveyed as a current producer.
        self.ids.clear();
        self
    }

    fn require_known_targets(&mut self, incoming: &HashMap<Uri, HashSet<Uri>>) {
        loop {
            let targets = incoming
                .iter()
                .filter(|(_, parents)| !parents.is_disjoint(&self.required))
                .map(|(target, _)| target.clone())
                .collect::<Vec<_>>();
            let before = self.required.len();
            self.required.extend(targets);
            if self.required.len() == before {
                break;
            }
        }
    }

    fn pending(uri: Uri) -> Self {
        Self {
            recapture: None,
            root: None,
            root_uri: uri.clone(),
            required: HashSet::from([uri]),
            association_owners: Vec::new(),
            ids: Vec::new(),
            complete: false,
            partial_definition: None,
            completion: None,
            eligibility: HashSet::new(),
            conditional_obligations: false,
            root_world_uncertain: false,
        }
    }
}

pub(crate) struct OriginalSourceViewSettlement {
    scopes: Vec<OriginalSourceRootScope>,
    revision: u64,
}

pub(crate) struct OriginalCommandSourceWorldCapture {
    settlement: OriginalSourceViewSettlement,
}

impl OriginalCommandSourceWorldCapture {
    /// Select the actual still-leased prefix producer for independent collision
    /// participants. The selected declaration retains its original owner.
    fn document_owners(
        &self,
        documents: &[super::super::OriginalDocumentSnapshot],
        selected_uri: &str,
    ) -> Option<Vec<super::super::OriginalDocumentSnapshot>> {
        documents.iter().map(|document| {
            if document.0 == selected_uri || document.2.original_completed_command_world().is_some() {
                return Some(document.clone());
            }
            let mut scopes = self.settlement.scopes.iter().filter(|scope|
                scope.root_uri.as_str() == document.0.as_str() && scope.completion.is_some());
            let Some(scope) = scopes.next() else { return Some(document.clone()) };
            if scopes.next().is_some() || !scope.complete || scope.conditional_obligations {
                return None;
            }
            let completion = scope.completion.as_ref()?;
            let prefix = &completion.prefix.draft;
            let analysis = &prefix.producer.seed.analysis;
            let context = tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(&document.2)?;
            if scope.root != Some(completion.prefix.id)
                || prefix.inherited.is_some() || prefix.region.is_some()
                || prefix.physical.uri.as_str() != document.0.as_str()
                || prefix.physical.source.as_ref() != document.1.as_ref()
                || prefix.producer.seed.path_admission.as_ref() != prefix.physical.admission.as_ref()
                || !prefix.matches_producer()
                || !original_readonly_analysis_matches_input(&document.1, &document.2, &prefix.producer.input)
                || tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(analysis) != Some(context)
                || !completion.outcome.matches_prefix_analysis(analysis)
            {
                return None;
            }
            Some((document.0.clone(), Arc::clone(&document.1), Arc::clone(analysis)))
        }).collect()
    }

    pub(crate) fn worlds(
        &self,
    ) -> Vec<tcl_lsp_core::original_command_rename::OriginalConditionalCommandWorld<'_>> {
        self.settlement
            .scopes
            .iter()
            .filter_map(|scope| {
                let completion = scope.completion.as_ref()?;
                Some(
                    tcl_lsp_core::original_command_rename::OriginalConditionalCommandWorld {
                        uri: scope.root_uri.as_str(),
                        outcome: &completion.outcome,
                    },
                )
            })
            .collect()
    }
}

struct OriginalSourceScopeProjection {
    ids: Vec<OriginalSourceViewId>,
    revision: u64,
    partial_definition_groups: Vec<Vec<OriginalSourceViewId>>,
}

impl OriginalSourceViewSettlement {
    fn requires_recapture(&self, uri: &Uri) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.recapture.is_some() && scope.required.contains(uri))
    }

    fn aggregate_complete(&self) -> bool {
        self.scopes
            .iter()
            .all(|scope| scope.complete && !scope.conditional_obligations)
    }

    fn ordinary_definition_scope_is_complete(
        &self,
        uri: &Uri,
        projection: &OriginalSourceScopeProjection,
    ) -> bool {
        if projection.revision != self.revision || !projection.partial_definition_groups.is_empty()
        {
            return false;
        }
        let mut required = self
            .scopes
            .iter()
            .filter(|scope| scope.required.contains(uri));
        let Some(first) = required.next() else {
            return false;
        };
        first.complete
            && !first.conditional_obligations
            && required.all(|scope| scope.complete && !scope.conditional_obligations)
    }

    fn navigation_projection(
        &self,
        uri: &Uri,
        references: Option<bool>,
    ) -> Option<OriginalSourceScopeProjection> {
        if references.is_some() {
            self.positive_reference_projection(uri)
        } else {
            self.definition_projection(uri)
        }
    }

    fn definition_projection(&self, uri: &Uri) -> Option<OriginalSourceScopeProjection> {
        let required = self
            .scopes
            .iter()
            .filter(|scope| scope.required.contains(uri))
            .collect::<Vec<_>>();
        (!required.is_empty()
            && required
                .iter()
                .all(|scope| scope.complete || scope.partial_definition.is_some()))
        .then_some(())?;
        let mut ids = Vec::new();
        let mut partial_definition_groups = Vec::new();
        for scope in required {
            if !scope.complete {
                partial_definition_groups.push(scope.ids.clone());
            }
            for id in &scope.ids {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        (!ids.is_empty()).then_some(OriginalSourceScopeProjection {
            ids,
            revision: self.revision,
            partial_definition_groups,
        })
    }

    /// Positive references require the selected dependency scopes to close,
    /// while every currently retained producer remains leased and surveyed.
    /// Incomplete unrelated scopes supply neither absence nor edit authority.
    fn positive_reference_projection(&self, uri: &Uri) -> Option<OriginalSourceScopeProjection> {
        // Positive reference answers survey every retained eligible producer.
        // A recapturing root has no leased inventory to contribute, even when
        // it is outside the cursor's selected source interpretation.
        if self.scopes.iter().any(|scope| scope.recapture.is_some()) {
            return None;
        }
        let required = self
            .scopes
            .iter()
            .filter(|scope| scope.required.contains(uri))
            .collect::<Vec<_>>();
        (!required.is_empty()
            && required.iter().all(|scope| {
                scope.complete && !scope.conditional_obligations && !scope.root_world_uncertain
            }))
        .then_some(())?;
        let mut ids = Vec::new();
        for scope in &self.scopes {
            for id in &scope.ids {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        (!ids.is_empty()).then_some(OriginalSourceScopeProjection {
            ids,
            revision: self.revision,
            partial_definition_groups: Vec::new(),
        })
    }

    fn projection(&self, uri: &Uri, aggregate: bool) -> Option<OriginalSourceScopeProjection> {
        let required = self
            .scopes
            .iter()
            .filter(|scope| scope.required.contains(uri))
            .collect::<Vec<_>>();
        (!required.is_empty() && required.iter().all(|scope| scope.complete)).then_some(())?;
        if aggregate && !self.aggregate_complete() {
            return None;
        }
        let mut ids = Vec::new();
        for scope in &self.scopes {
            if aggregate || scope.required.contains(uri) {
                for id in &scope.ids {
                    if !ids.contains(id) {
                        ids.push(*id);
                    }
                }
            }
        }
        (!ids.is_empty()).then_some(OriginalSourceScopeProjection {
            ids,
            revision: self.revision,
            partial_definition_groups: Vec::new(),
        })
    }
}

impl Backend {
    async fn cached_original_source_view(
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

    fn source_draft_target(
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
        super::super::canonical_file_uri(&target)
    }

    /// Build serialization is separate from the final rehoming publication
    /// gate. No source read/analysis awaits while holding that publication gate.
    pub(crate) async fn refresh_original_source_view_graph(
        &self,
    ) -> DocumentSourceResult<Option<OriginalSourceViewSettlement>> {
        // The view table is read with `try_lock`, so a publisher holding it
        // (the one that follows `didOpen`) refuses a settlement that a moment
        // later succeeds. Only a refusal that persists is reported.
        for _ in 1..SOURCE_VIEW_CONTENTION_ATTEMPTS {
            if let Some(settlement) = self.refresh_original_source_view_graph_once().await? {
                return Ok(Some(settlement));
            }
            crate::rt::sleep(SOURCE_VIEW_CONTENTION_BACKOFF).await;
        }
        self.refresh_original_source_view_graph_once().await
    }

    async fn refresh_original_source_view_graph_for_uri(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<Option<OriginalSourceViewSettlement>> {
        for _ in 1..SOURCE_VIEW_CONTENTION_ATTEMPTS {
            if let Some(settlement) = self.refresh_original_source_view_graph_once().await?
                && !settlement.requires_recapture(uri)
            {
                return Ok(Some(settlement));
            }
            crate::rt::sleep(SOURCE_VIEW_CONTENTION_BACKOFF).await;
        }
        self.refresh_original_source_view_graph_once().await
    }

    async fn refresh_original_source_view_graph_once(
        &self,
    ) -> DocumentSourceResult<Option<OriginalSourceViewSettlement>> {
        let _build = self.original_source_view_build_gate.lock().await;
        if !self
            .prune_retired_original_source_view_presentations()
            .await
        {
            return Ok(None);
        }
        let Some(work) = self.original_source_root_work().await? else {
            return Ok(None);
        };
        let mut reconciliation = OriginalSourceRootReconciliation {
            bootstrap: work
                .candidates
                .iter()
                .map(|candidate| candidate.draft.clone())
                .collect(),
            scopes: work
                .pending
                .into_iter()
                .map(OriginalSourceRootScope::pending)
                .collect(),
            excluded: Vec::new(),
            incoming: work.incoming,
        };
        let roots = self
            .publish_initial_original_roots(work.candidates, &work.associated, &mut reconciliation)
            .await?;
        // Each actual root retains its own pending state; failure cannot erase
        // debt or globally suppress a different completed producer.
        for root in roots {
            let mut scope = self.run_original_source_view_work(root).await?;
            if scope.recapture.is_some() {
                scope.require_known_targets(&reconciliation.incoming);
            }
            reconciliation.scopes.push(scope);
        }
        if !self
            .reconcile_excluded_original_roots(&mut reconciliation)
            .await?
        {
            return Ok(None);
        }
        let result = self
            .finish_original_root_reconciliation(reconciliation)
            .await?;
        Ok(result)
    }

    async fn publish_initial_original_roots(
        &self,
        candidates: Vec<OriginalSourceRootCandidate>,
        associated: &HashSet<Uri>,
        reconciliation: &mut OriginalSourceRootReconciliation,
    ) -> DocumentSourceResult<Vec<Arc<PublishedOriginalSourceView>>> {
        let mut roots = Vec::new();
        for candidate in candidates {
            let uri = candidate.draft.physical.uri.clone();
            if associated.contains(&uri) {
                reconciliation.excluded.push(candidate);
                continue;
            }
            if let Some(cached) = candidate.published {
                roots.push(cached);
                continue;
            }
            let views = match self
                .publish_original_source_views(&uri, vec![candidate.draft])
                .await?
            {
                OriginalSourceViewAttempt::Ready(views) => views,
                OriginalSourceViewAttempt::RetryCapture => {
                    let mut scope = OriginalSourceRootScope::pending(uri).retrying();
                    scope.require_known_targets(&reconciliation.incoming);
                    reconciliation.scopes.push(scope);
                    continue;
                }
                OriginalSourceViewAttempt::Unavailable => {
                    reconciliation
                        .scopes
                        .push(OriginalSourceRootScope::pending(uri));
                    continue;
                }
            };
            roots.extend(views);
        }
        Ok(roots)
    }

    async fn reconcile_excluded_original_roots(
        &self,
        reconciliation: &mut OriginalSourceRootReconciliation,
    ) -> DocumentSourceResult<bool> {
        // Reconsider only candidates whose tentative incoming parents were
        // replaced by genuine completed source associations. An unrelated
        // completed root cannot break a rootless cycle or choose its entry.
        loop {
            let Some((actual, completed)) = self
                .current_original_scope_associations(&reconciliation.scopes)
                .await?
            else {
                return Ok(false);
            };
            let mut progress = false;
            let mut remaining = Vec::new();
            for candidate in std::mem::take(&mut reconciliation.excluded) {
                let uri = candidate.draft.physical.uri.clone();
                if actual.contains(&uri) {
                    continue;
                }
                let eligible = reconciliation.incoming.get(&uri).is_some_and(|parents| {
                    !parents.is_empty() && parents.iter().all(|parent| completed.contains(parent))
                });
                if !eligible {
                    remaining.push(candidate);
                    continue;
                }
                let view = if let Some(view) = candidate.published {
                    let Some(current) = self
                        .with_current_original_source_view(view.id, |current| {
                            current
                                .draft
                                .same_live_producer(&view.draft)
                                .then(|| Arc::clone(current))
                        })
                        .await?
                    else {
                        reconciliation
                            .scopes
                            .push(OriginalSourceRootScope::pending(uri));
                        continue;
                    };
                    current
                } else {
                    let mut views = match self
                        .publish_original_source_views(&uri, vec![candidate.draft])
                        .await?
                    {
                        OriginalSourceViewAttempt::Ready(views) => views,
                        OriginalSourceViewAttempt::RetryCapture => {
                            let mut scope = OriginalSourceRootScope::pending(uri).retrying();
                            scope.require_known_targets(&reconciliation.incoming);
                            reconciliation.scopes.push(scope);
                            continue;
                        }
                        OriginalSourceViewAttempt::Unavailable => {
                            reconciliation
                                .scopes
                                .push(OriginalSourceRootScope::pending(uri));
                            continue;
                        }
                    };
                    if views.len() != 1 {
                        return Ok(false);
                    }
                    views.remove(0)
                };
                let mut scope = self.run_original_source_view_work(view).await?;
                if scope.recapture.is_some() {
                    scope.require_known_targets(&reconciliation.incoming);
                }
                scope.eligibility = reconciliation
                    .incoming
                    .get(&uri)
                    .cloned()
                    .unwrap_or_default();
                reconciliation.scopes.push(scope);
                progress = true;
            }
            reconciliation.excluded = remaining;
            if !progress {
                break;
            }
        }
        Ok(true)
    }

    async fn finish_original_root_reconciliation(
        &self,
        mut reconciliation: OriginalSourceRootReconciliation,
    ) -> DocumentSourceResult<Option<OriginalSourceViewSettlement>> {
        let final_revision = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            let Some(revision) = table.current_revision() else {
                return Ok(None);
            };
            revision
        };
        // Completion dependencies may have retired during another root's
        // reconciliation. Propagate those losses through root eligibility.
        loop {
            let Some((_, completed)) = self
                .current_original_scope_associations(&reconciliation.scopes)
                .await?
            else {
                return Ok(None);
            };
            let mut changed = false;
            for scope in &mut reconciliation.scopes {
                if !scope.eligibility.is_subset(&completed) {
                    if scope.complete {
                        scope.complete = false;
                        changed = true;
                    }
                    scope.partial_definition = None;
                }
            }
            if !changed {
                break;
            }
        }
        let Some((actual_associations, _)) = self
            .current_original_scope_associations(&reconciliation.scopes)
            .await?
        else {
            return Ok(None);
        };
        for candidate in reconciliation.excluded {
            let uri = candidate.draft.physical.uri.clone();
            if !actual_associations.contains(&uri) {
                reconciliation
                    .scopes
                    .push(OriginalSourceRootScope::pending(uri.clone()));
            }
        }
        let superseded = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            reconciliation
                .scopes
                .iter()
                .filter_map(|scope| {
                    let id = scope.root?;
                    (!table.by_id.contains_key(&id)
                        && actual_associations.contains(&scope.root_uri))
                    .then_some(id)
                })
                .collect::<HashSet<_>>()
        };
        // A root disappears only after an actual association has retired its
        // exact publication. No failed or unknown replay can discharge it.
        reconciliation.scopes.retain(|scope| match scope.root {
            Some(root) => !superseded.contains(&root),
            None => !actual_associations.contains(&scope.root_uri),
        });
        // Capturing a final revision after a concurrent physical/config
        // mutation cannot bless an older bootstrap classification.
        for draft in &reconciliation.bootstrap {
            if !self.original_source_bootstrap_is_current(draft).await? {
                return Ok(None);
            }
        }
        let revision = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            if table.current_revision() != Some(final_revision) {
                return Ok(None);
            }
            final_revision
        };
        Ok(Some(OriginalSourceViewSettlement {
            scopes: reconciliation.scopes,
            revision,
        }))
    }

    async fn original_source_bootstrap_is_current(
        &self,
        draft: &OriginalSourceViewDraft,
    ) -> DocumentSourceResult<bool> {
        let epoch = draft.producer.seed.analyser_inputs_epoch;
        let factory = draft.producer.seed.class_factory_generation;
        let Some(saved) = self
            .readonly_saved_sources(&[&draft.physical.publication])
            .await?
        else {
            return Ok(false);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        let current = self
            .with_current_readonly_source_state(
                epoch,
                factory,
                &[draft.physical.admission.as_ref()],
                Some(&saved),
                |state| {
                    draft
                        .matches_physical_state(state, epoch, factory)
                        .then_some(())
                },
                |_, ()| Some(()),
            )
            .await?;
        Ok(current.is_some())
    }

    /// Coverage here means every current association to a candidate parent
    /// owns an actual materialized producer and completed source-view scope.
    /// A target URI or potential-body `NoCut` cannot manufacture that producer.
    async fn current_original_scope_associations(
        &self,
        scopes: &[OriginalSourceRootScope],
    ) -> DocumentSourceResult<Option<(HashSet<Uri>, HashSet<Uri>)>> {
        let mut actual = HashSet::new();
        let mut coverage = HashMap::<Uri, bool>::new();
        for scope in scopes {
            if scope.root.is_some_and(|root| {
                self.original_source_views
                    .try_lock()
                    .is_ok_and(|table| !table.by_id.contains_key(&root))
            }) {
                continue;
            }
            let owners = self
                .with_original_source_scope(&scope.ids, |views| Some(views.to_vec()))
                .await?;
            let complete = scope.complete && owners.is_some();
            for association in &scope.association_owners {
                let owner = &association.owner;
                if self
                    .with_current_original_source_view(owner.id, |current| {
                        current.draft.same_live_producer(&owner.draft).then_some(())
                    })
                    .await?
                    .is_none()
                {
                    coverage.insert(association.target.clone(), false);
                    continue;
                }
                actual.insert(association.target.clone());
                let materialized = complete
                    && owners.as_ref().is_some_and(|views| {
                        let Some(SourceFrontier::CurrentCut(cut)) =
                            association.edge.original_source_frontier.as_deref()
                        else {
                            return false;
                        };
                        cut.matches_parent_analysis(&owner.draft.producer.seed.analysis)
                            && association.edge.original_project_source_load.as_deref()
                                == Some(cut.source_load())
                            && views.iter().any(|child| {
                                child.draft.physical.uri == association.target
                                    && child.draft.region.is_none()
                                    && child.draft.inherited.as_ref().is_some_and(|inherited| {
                                        inherited.parent.id == owner.id
                                            && inherited
                                                .parent
                                                .draft
                                                .same_live_producer(&owner.draft)
                                            && inherited.point.as_ref() == cut.point()
                                            && cut.point().matches_child_analysis(
                                                &child.draft.producer.seed.analysis,
                                            )
                                    })
                            })
                    });
                coverage
                    .entry(association.target.clone())
                    .and_modify(|old| *old &= materialized)
                    .or_insert(materialized);
            }
        }
        let completed = coverage
            .into_iter()
            .filter_map(|(uri, complete)| complete.then_some(uri))
            .collect();
        Ok(Some((actual, completed)))
    }

    /// Synchronous retirement already made saved handles terminal. Pruning
    /// retracts their compatibility rows under the publication transaction; it
    /// never reacquires a retired producer from URI, text or equal metadata.
    async fn prune_retired_original_source_view_presentations(&self) -> bool {
        let _publication = self.rehoming_gate.lock().await;
        let Ok(mut table) = self.original_source_views.try_lock() else {
            return false;
        };
        let Ok(mut index) = self.workspace_index.try_write() else {
            return false;
        };
        // A legacy index replacement has already revoked membership: saved
        // handles fail their exact indexed-owner check. Retire those locators
        // and descendants before building any replacement source views.
        let lost = table
            .by_id
            .values()
            .filter(|view| !view.is_indexed(&index))
            .map(|view| view.id)
            .collect();
        table.retire_ids(lost);
        let live = table
            .by_id
            .values()
            .map(|view| {
                tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(view.id)
            })
            .collect::<Vec<_>>();
        index
            .retain_original_source_view_publications(&live)
            .is_some()
    }

    /// Root discovery never treats flattened edge rows as live compiler
    /// authority. Existing interpreted membership has no fresh-root fallback.
    async fn original_source_root_work(
        &self,
    ) -> DocumentSourceResult<Option<OriginalSourceRootWork>> {
        let uris = self.workspace_index.read().await.document_uris();
        let mut work = OriginalSourceRootWork {
            candidates: Vec::new(),
            pending: HashSet::new(),
            associated: HashSet::new(),
            incoming: HashMap::new(),
        };
        for uri in uris {
            let Ok(uri) = uri.parse::<Uri>() else {
                return Ok(None);
            };
            if let Some(view) = self.cached_original_source_view(&uri, None).await? {
                work.candidates.push(OriginalSourceRootCandidate {
                    draft: view.draft.clone(),
                    published: Some(view),
                });
                continue;
            }
            let interpreted = {
                let Ok(table) = self.original_source_views.try_lock() else {
                    return Ok(None);
                };
                table.by_uri.get(&uri).into_iter().flatten().any(|id| {
                    table
                        .by_id
                        .get(id)
                        .is_some_and(|view| view.draft.is_interpreted_member())
                })
            };
            if interpreted {
                continue;
            }
            let snapshot = self.rehoming_document_snapshot(&uri).await?;
            let super::super::RehomingDocumentSnapshot::Ready(document) = snapshot else {
                work.pending.insert(uri);
                continue;
            };
            match Self::original_declaration_document_participates(&uri, &document) {
                Some(false) => continue,
                Some(true) => {}
                None => {
                    work.pending.insert(uri);
                    continue;
                }
            }
            let Some(draft) = self
                .capture_initial_original_source_root(&uri, *document)
                .await?
            else {
                work.pending.insert(uri);
                continue;
            };
            work.candidates.push(OriginalSourceRootCandidate {
                draft,
                published: None,
            });
        }
        // Classification is tentative until every candidate's exact physical,
        // config, input and producer lease has rejoined after its worker.
        for candidate in &work.candidates {
            let Some(edges) = self.original_source_candidate_edges(candidate).await? else {
                work.pending.insert(candidate.draft.physical.uri.clone());
                continue;
            };
            for edge in edges {
                if let Some(target) = Self::source_draft_target(&candidate.draft, &edge) {
                    work.incoming
                        .entry(target.clone())
                        .or_default()
                        .insert(candidate.draft.physical.uri.clone());
                    work.associated.insert(target);
                }
            }
        }
        Ok(Some(work))
    }

    async fn original_source_candidate_edges(
        &self,
        candidate: &OriginalSourceRootCandidate,
    ) -> DocumentSourceResult<Option<Vec<tcl_lsp_core::workspace_index::WorkspaceSource>>> {
        let draft = candidate.draft.clone();
        let edges = super::super::rt::spawn_blocking(move || {
            tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView::source_edges_for_analysis(
                draft.physical.uri.as_str(),
                &draft.physical.source,
                &draft.producer.seed.analysis,
                None,
            )
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
            uri: candidate.draft.physical.uri.clone(),
        })?;
        let Some(edges) = edges else {
            return Ok(None);
        };
        self.with_current_original_root_candidate(candidate, || Some(edges.clone()))
            .await
    }

    /// Private bootstrap lease: no publication key is minted to validate a
    /// tentative authoring candidate. Equal text cannot renew a published view.
    async fn with_current_original_root_candidate<T>(
        &self,
        candidate: &OriginalSourceRootCandidate,
        mut render: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        let draft = &candidate.draft;
        let epoch = draft.producer.seed.analyser_inputs_epoch;
        let factory = draft.producer.seed.class_factory_generation;
        let Some(saved) = self
            .readonly_saved_sources(&[&draft.physical.publication])
            .await?
        else {
            return Ok(None);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        self.with_current_readonly_source_state(
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
                let table = self.original_source_views.try_lock().ok()?;
                let index = self.workspace_index.try_read().ok()?;
                if let Some(view) = &candidate.published {
                    (table.contains_actual(view)
                        && view.is_indexed(&index)
                        && view.draft.same_live_producer(draft))
                    .then_some(())?;
                } else {
                    (!table
                        .by_uri
                        .get(&draft.physical.uri)
                        .into_iter()
                        .flatten()
                        .any(|id| {
                            table
                                .by_id
                                .get(id)
                                .is_some_and(|view| view.draft.is_interpreted_member())
                        }))
                    .then_some(())?;
                }
                render()
            },
        )
        .await
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
        let edges = super::super::rt::spawn_blocking(move || {
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
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
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
            super::ReadonlyTransaction::Published(()) => OriginalSourceViewAttempt::Ready(()),
            super::ReadonlyTransaction::CurrencyRefused
            | super::ReadonlyTransaction::PublishRefused => OriginalSourceViewAttempt::RetryCapture,
        })
    }

    /// A first source edge admits its closed child through the normal disk
    /// publisher before capturing an inherited interpretation at the real cut.
    async fn source_child_document_snapshot(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<super::super::RehomingDocumentSnapshot> {
        let snapshot = self.rehoming_document_snapshot(uri).await?;
        if !matches!(snapshot, super::super::RehomingDocumentSnapshot::Missing) {
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
            super::super::RehomingDocumentSnapshot::Ready(document) => document,
            super::super::RehomingDocumentSnapshot::Pending => {
                return Ok(OriginalSourceViewAttempt::RetryCapture);
            }
            super::super::RehomingDocumentSnapshot::Missing => {
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

    async fn run_original_source_view_work(
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

    pub(crate) async fn command_source_documents(
        &self,
        member: bool,
        documents: Vec<super::super::OriginalDocumentSnapshot>,
        selected_uri: &str,
    ) -> DocumentSourceResult<(
        Vec<super::super::OriginalDocumentSnapshot>,
        Option<OriginalCommandSourceWorldCapture>,
    )> {
        let capture = {
            let owners = super::super::implementation_inventory(&documents);
            self.capture_original_command_source_worlds_if_needed(member, &owners)
                .await?
        };
        let Some(capture) = capture else {
            return Ok((documents, None));
        };
        let Some(selected_documents) = capture.document_owners(&documents, selected_uri) else {
            // Keep the original unknown-world refusal; no alternate owner is
            // installed when a current full-context/physical join is missing.
            return Ok((documents, None));
        };
        Ok((selected_documents, Some(capture)))
    }

    /// Enclose the complete source survey in the existing physical and
    /// interpretation publication leases. No borrowed view escapes rendering.
    pub(crate) async fn original_command_plan_with_source_views(
        &self,
        documents: &[tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>],
        identity: &tcl_lsp_core::original_declaration::OriginalDeclarationIdentity,
        new_tail: &str,
        cursor: (&str, u32),
        capture: Option<&OriginalCommandSourceWorldCapture>,
    ) -> DocumentSourceResult<
        Option<
            Result<
                Vec<tcl_lsp_core::rename::WorkspaceTextEdit>,
                tcl_lsp_core::original_command_rename::OriginalCommandRenameRefusal,
            >,
        >,
    > {
        let Some(capture) = capture else {
            return Ok(Some(tcl_lsp_core::original_command_rename::original_command_rename_edits_with_conditional_worlds(
                documents, identity, new_tail, &[],
            )));
        };
        if !self
            .original_command_source_worlds_are_current(capture)
            .await?
        {
            return Ok(None);
        }
        let mut ids = Vec::new();
        for scope in &capture.settlement.scopes {
            for id in &scope.ids {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        let worlds = capture.worlds();
        self.with_selected_original_source_views(&ids, Some(capture.settlement.revision), |views| {
            let (publications, ancestors) = Self::original_source_view_labels(views)?;
            let inventory = Self::original_source_view_inventory(views, &publications, &ancestors);
            Some(tcl_lsp_core::original_command_rename::original_command_rename_edits_with_selected_source_view(
                documents, identity, new_tail, cursor.0, cursor.1, &worlds, &inventory,
            ))
        }).await
    }

    /// Select and consume the inherited declaration only inside the same
    /// indexed source-view revision used by the subsequent rename planner.
    pub(crate) async fn original_command_prepare_with_source_views(
        &self,
        documents: &[tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>],
        identity: &tcl_lsp_core::original_declaration::OriginalDeclarationIdentity,
        cursor_uri: &str,
        cursor: u32,
        capture: Option<&OriginalCommandSourceWorldCapture>,
    ) -> DocumentSourceResult<Option<tcl_lexer::Span>> {
        let Some(capture) = capture else {
            return Ok(
                tcl_lsp_core::original_command_rename::original_command_prepare_span_in(
                    documents, identity, cursor_uri, cursor,
                )
                .ok(),
            );
        };
        if !self
            .original_command_source_worlds_are_current(capture)
            .await?
        {
            return Ok(None);
        }
        let mut ids = Vec::new();
        for scope in &capture.settlement.scopes {
            for id in &scope.ids {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        let worlds = capture.worlds();
        self.with_selected_original_source_views(&ids, Some(capture.settlement.revision), |views| {
            let (publications, ancestors) = Self::original_source_view_labels(views)?;
            let inventory = Self::original_source_view_inventory(views, &publications, &ancestors);
            tcl_lsp_core::original_command_rename::original_command_prepare_span_in_with_selected_source_view(
                documents, identity, cursor_uri, cursor, &worlds, &inventory,
            ).ok()
        }).await
    }

    pub(crate) async fn capture_original_command_source_worlds_if_needed(
        &self,
        member: bool,
        owners: &[tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>],
    ) -> DocumentSourceResult<Option<OriginalCommandSourceWorldCapture>> {
        if !member
            && owners
                .iter()
                .any(|owner| owner.analysis.original_completed_command_world().is_none())
        {
            self.capture_original_command_source_worlds().await
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn capture_original_command_source_worlds(
        &self,
    ) -> DocumentSourceResult<Option<OriginalCommandSourceWorldCapture>> {
        let Some(settlement) = self.refresh_original_source_view_graph().await? else {
            return Ok(None);
        };
        let capture = OriginalCommandSourceWorldCapture { settlement };
        Ok(self
            .original_command_source_worlds_are_current(&capture)
            .await?
            .then_some(capture))
    }

    pub(crate) async fn original_command_source_worlds_are_current(
        &self,
        capture: &OriginalCommandSourceWorldCapture,
    ) -> DocumentSourceResult<bool> {
        if !self.original_source_view_settlement_is_indexed(&capture.settlement) {
            return Ok(false);
        }
        for scope in &capture.settlement.scopes {
            let Some(completion) = &scope.completion else {
                continue;
            };
            let current = self
                .with_selected_original_source_views(
                    &scope.ids,
                    Some(capture.settlement.revision),
                    |views| {
                        (views.iter().any(|view| {
                            view.id == completion.prefix.id
                                && view.draft.same_live_producer(&completion.prefix.draft)
                        }) && completion.dependencies.iter().all(|dependency| {
                            views.iter().any(|view| {
                                view.id == dependency.id
                                    && view.draft.same_live_producer(&dependency.draft)
                            })
                        }) && completion.outcome.matches_prefix_analysis(
                            &completion.prefix.draft.producer.seed.analysis,
                        ))
                        .then_some(())
                    },
                )
                .await?;
            if current.is_none() {
                return Ok(false);
            }
        }
        Ok(self.original_source_view_settlement_is_indexed(&capture.settlement))
    }

    pub(crate) fn original_source_view_settlement_is_indexed(
        &self,
        settlement: &OriginalSourceViewSettlement,
    ) -> bool {
        settlement.aggregate_complete()
            && self.original_source_view_settlement_views_are_indexed(settlement)
    }

    /// Every view of the current settlement is still published to the index.
    /// Completeness stays with the projections and the world capture.
    pub(crate) fn original_source_view_settlement_views_are_indexed(
        &self,
        settlement: &OriginalSourceViewSettlement,
    ) -> bool {
        if settlement
            .scopes
            .iter()
            .any(|scope| scope.recapture.is_some())
        {
            return false;
        }
        let Ok(table) = self.original_source_views.try_lock() else {
            return false;
        };
        let Ok(index) = self.workspace_index.try_read() else {
            return false;
        };
        table.current_revision() == Some(settlement.revision)
            && settlement.scopes.iter().all(|scope| {
            scope.ids.iter().all(|id| table.by_id.get(id).is_some_and(|view| {
            table.contains_actual(view) && view.is_indexed(&index)
                && index.original_source_views(view.draft.physical.uri.as_str()).iter().any(|row| {
                    Arc::ptr_eq(row.analysis(), &view.draft.producer.seed.analysis)
                        && row.publication() == &tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(*id)
                })
            }))
        })
    }
}

impl Backend {
    /// Retain an inventory only for the duration of the current physical and
    /// interpretation leases. The callback cannot acquire async dependencies.
    async fn with_original_source_scope<T>(
        &self,
        ids: &[OriginalSourceViewId],
        render: impl FnMut(&[Arc<PublishedOriginalSourceView>]) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.with_selected_original_source_views(ids, None, render)
            .await
    }

    async fn with_settled_original_source_scope<T>(
        &self,
        projection: &OriginalSourceScopeProjection,
        render: impl FnMut(&[Arc<PublishedOriginalSourceView>]) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.with_selected_original_source_views(&projection.ids, Some(projection.revision), render)
            .await
    }

    async fn with_selected_original_source_views<T>(
        &self,
        ids: &[OriginalSourceViewId],
        revision: Option<u64>,
        render: impl FnMut(&[Arc<PublishedOriginalSourceView>]) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.with_selected_original_source_views_and_provider_headers(ids, revision, &[], render)
            .await
    }

    async fn with_selected_original_source_views_and_provider_headers<T>(
        &self,
        ids: &[OriginalSourceViewId],
        revision: Option<u64>,
        providers: &[ConditionalAutoloadProvider],
        mut render: impl FnMut(&[Arc<PublishedOriginalSourceView>]) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        let views = {
            let Ok(table) = self.original_source_views.try_lock() else {
                return Ok(None);
            };
            if revision.is_some_and(|revision| table.current_revision() != Some(revision)) {
                return Ok(None);
            }
            let Some(views) = ids
                .iter()
                .map(|id| table.by_id.get(id).cloned())
                .collect::<Option<Vec<_>>>()
            else {
                return Ok(None);
            };
            views
        };
        let Some(first) = views.first() else {
            return Ok(None);
        };
        let epoch = first.draft.producer.seed.analyser_inputs_epoch;
        let factory = first.draft.producer.seed.class_factory_generation;
        let mut publications = views
            .iter()
            .map(|view| &view.draft.physical.publication)
            .collect::<Vec<_>>();
        publications.extend(
            providers
                .iter()
                .map(|provider| &provider.source.publication),
        );
        let mut admissions = views
            .iter()
            .map(|view| view.draft.physical.admission.as_ref())
            .collect::<Vec<_>>();
        admissions.extend(
            providers
                .iter()
                .map(|provider| provider.source.admission.as_ref()),
        );
        let saved = if providers.is_empty() {
            // Preserve the established ordinary navigation/reference/edit lease.
            // Its separate currency audit is outside this conditional purpose.
            let Some(saved) = self.readonly_saved_sources(&publications).await? else {
                return Ok(None);
            };
            saved
                .into_iter()
                .map(|source| (source.admission, Arc::<[u8]>::from(source.bytes)))
                .collect()
        } else {
            let Some(saved) = Self::conditional_autoload_saved_sources(&publications, &admissions)
            else {
                return Ok(None);
            };
            saved
        };
        let saved = saved
            .iter()
            .map(|(admission, bytes)| (admission.as_ref(), bytes.as_ref()))
            .collect::<Vec<_>>();
        self.with_current_readonly_source_state(epoch, factory, &admissions, Some(&saved),
            |state| (views.iter().all(|view| view.draft.matches_physical_state(state, epoch, factory))
                && providers.iter().all(|provider| provider.owner.matches_current_state(state, epoch, factory)
                    && provider.source.publication.matches_open_documents(state.open)
                    && state.uri_generations.get(&provider.source.uri).copied().unwrap_or_default()
                        == provider.source.publication.uri_generation)).then_some(()),
            |_, ()| {
                let _publication = self.rehoming_gate.try_lock().ok()?;
                let table = self.original_source_views.try_lock().ok()?;
                let index = self.workspace_index.try_read().ok()?;
                (revision.is_none_or(|revision| table.current_revision() == Some(revision)) && views.iter().all(|view| {
                    table.contains_actual(view) && table.parent_chain_is_live(&view.draft)
                        && view.is_indexed(&index)
                        && index.original_source_views(view.draft.physical.uri.as_str()).iter().any(|row| {
                            Arc::ptr_eq(row.analysis(), &view.draft.producer.seed.analysis)
                                && row.publication() == &tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(view.id)
                        })
                })).then_some(())?;
                render(&views)
            },
        ).await
    }

    fn original_source_view_inventory<'a>(
        views: &'a [Arc<PublishedOriginalSourceView>],
        publications: &'a [tcl_lsp_core::original_declaration::OriginalSourceViewPublication],
        ancestors: &'a [Vec<tcl_lsp_core::original_declaration::OriginalSourceViewPublication>],
    ) -> Vec<tcl_lsp_core::original_source_views::OriginalSourceViewDocument<'a>> {
        views
            .iter()
            .zip(publications)
            .zip(ancestors)
            .map(|((view, publication), ancestors)| {
                tcl_lsp_core::original_source_views::OriginalSourceViewDocument {
                    publication,
                    document: tcl_lsp_core::original_declaration::OriginalDeclarationDocument {
                        uri: view.draft.physical.uri.as_str(),
                        source: &view.draft.physical.source,
                        analysis: &view.draft.producer.seed.analysis,
                    },
                    region: view.draft.region.as_ref().map(|owned| &owned.region),
                    ancestors,
                }
            })
            .collect()
    }

    fn original_source_view_labels(
        views: &[Arc<PublishedOriginalSourceView>],
    ) -> Option<(
        Vec<tcl_lsp_core::original_declaration::OriginalSourceViewPublication>,
        Vec<Vec<tcl_lsp_core::original_declaration::OriginalSourceViewPublication>>,
    )> {
        use tcl_lsp_core::original_declaration::OriginalSourceViewPublication as Stamp;
        let publications = views.iter().map(|view| Stamp::retain(view.id)).collect();
        let ancestors = views
            .iter()
            .map(|view| {
                Some(
                    view.draft
                        .ancestors()?
                        .iter()
                        .map(|parent| Stamp::retain(parent.id))
                        .collect(),
                )
            })
            .collect::<Option<Vec<_>>>()?;
        Some((publications, ancestors))
    }

    /// Lift metadata and issue the wire locator from the same exact live view.
    /// Call only inside a selected source scope's checked render.
    fn lift_current_source_view_call_item(
        &self,
        selected: &tcl_lsp_core::original_source_views::OriginalSourceViewDeclaration,
        views: &[Arc<PublishedOriginalSourceView>],
    ) -> Option<tower_lsp_server::ls_types::CallHierarchyItem> {
        use tcl_lsp_core::original_declaration::OriginalDeclarationRole as Role;

        let mut matching = views.iter().filter(|view| {
            selected.publication
                == tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(
                    view.id,
                )
        });
        let view = matching.next()?;
        matching.next().is_none().then_some(())?;
        let physical = &view.draft.physical;
        let core = tcl_lsp_core::original_call_hierarchy::item(
            &selected.identity,
            &tcl_lsp_core::original_declaration::OriginalDeclarationDocument {
                uri: physical.uri.as_str(),
                source: &physical.source,
                analysis: &view.draft.producer.seed.analysis,
            },
        )?;
        let handle = self
            .original_declaration_registry
            .try_lock()
            .ok()?
            .issue_in_source_view(selected.identity.clone(), selected.publication.clone())?;
        let kind = match selected.identity.role() {
            Role::Class => tower_lsp_server::ls_types::SymbolKind::CLASS,
            Role::Method(_) => tower_lsp_server::ls_types::SymbolKind::METHOD,
            _ => tower_lsp_server::ls_types::SymbolKind::FUNCTION,
        };
        Some(tower_lsp_server::ls_types::CallHierarchyItem {
            name: core.name,
            kind,
            tags: None,
            detail: core.detail,
            uri: physical.uri.clone(),
            range: super::super::lift_lsp_range(core.range),
            selection_range: super::super::lift_lsp_range(core.selection_range),
            data: Some(serde_json::json!({"originalDeclaration": handle})),
        })
    }

    async fn prepare_original_source_view_calls(
        &self,
        projection: &OriginalSourceScopeProjection,
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::CallHierarchyItem>>> {
        let uri = uri.clone();
        let Some((owners, selected)) = self
            .compute_original_source_views(projection, move |views| {
                let (publications, ancestors) = Self::original_source_view_labels(views)?;
                let inventory =
                    Self::original_source_view_inventory(views, &publications, &ancestors);
                let source = views.iter().find(|view| view.draft.physical.uri == uri)?;
                let source = &source.draft.physical.source;
                let offset = tcl_lexer::LineIndex::new(source).offset_at_utf16(
                    position.line,
                    tcl_lexer::Utf16Col::new(position.character),
                    source,
                );
                tcl_lsp_core::original_source_views::cursor_publications(
                    uri.as_str(),
                    offset,
                    &inventory,
                )?
                .iter()
                .map(|publication| {
                    tcl_lsp_core::original_source_views::prepare(publication, offset, &inventory)
                })
                .collect::<Option<Vec<_>>>()
            })
            .await?
        else {
            return Ok(None);
        };
        self.with_settled_original_source_scope(projection, |views| {
            Self::same_original_source_view_inventory(&owners, views).then_some(())?;
            selected
                .iter()
                .map(|selected| self.lift_current_source_view_call_item(selected, views))
                .collect::<Option<Vec<_>>>()
        })
        .await
    }

    /// Saved handles are looked up before any reconciliation or analysis. A
    /// retired A cannot be renewed by opening or materialising byte-equal B.
    pub(crate) async fn walk_original_source_view_calls(
        &self,
        item: &tower_lsp_server::ls_types::CallHierarchyItem,
        incoming: bool,
    ) -> DocumentSourceResult<
        Option<
            Vec<(
                tower_lsp_server::ls_types::CallHierarchyItem,
                Vec<tower_lsp_server::ls_types::Range>,
            )>,
        >,
    > {
        let Some(handle) = item
            .data
            .as_ref()
            .and_then(|data| data.get("originalDeclaration"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
        else {
            return Ok(None);
        };
        let saved = self.original_source_view_id_for_handle(handle);
        let Some(id) = saved else {
            return Ok(None);
        };
        // Ownership may leave this lease; currency must be rejoined below.
        let Some(identity) = self
            .with_current_original_source_view(id, |view| {
                (view.draft.physical.uri == item.uri).then_some(())?;
                let publication =
                    tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(id);
                let identity = self
                    .original_declaration_registry
                    .try_lock()
                    .ok()?
                    .resolve_in_source_view(
                        handle,
                        item.uri.as_str(),
                        &view.draft.physical.source,
                        &view.draft.producer.seed.analysis,
                        &publication,
                    )?;
                Some(identity)
            })
            .await?
        else {
            return Ok(None);
        };
        let Some(settlement) = self.refresh_original_source_view_graph().await? else {
            return Ok(None);
        };
        let Some(ids) = settlement.projection(&item.uri, incoming) else {
            return Ok(None);
        };
        if !ids.ids.contains(&id) {
            return Ok(None);
        }
        let expected_identity = identity;
        let worker_identity = expected_identity.clone();
        let publication =
            tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(id);
        let Some((owners, groups)) = self
            .compute_original_source_views(&ids, move |views| {
                Self::source_view_call_groups(views, incoming, &publication, &worker_identity)
            })
            .await?
        else {
            return Ok(None);
        };
        // Refusal is terminal: no legacy, URI/text resolver or fresh graph.
        let mut groups = Some(groups);
        self.with_settled_original_source_scope(&ids, |views| {
            Self::same_original_source_view_inventory(&owners, views).then_some(())?;
            let view = views.iter().find(|view| view.id == id)?;
            (view.draft.physical.uri == item.uri).then_some(())?;
            let publication =
                tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(id);
            let identity = self
                .original_declaration_registry
                .try_lock()
                .ok()?
                .resolve_in_source_view(
                    handle,
                    item.uri.as_str(),
                    &view.draft.physical.source,
                    &view.draft.producer.seed.analysis,
                    &publication,
                )?;
            (identity == expected_identity).then_some(())?;
            groups
                .take()?
                .into_iter()
                .map(|(selected, mut ranges)| {
                    ranges.sort_by_key(|range| {
                        (
                            range.start.line,
                            range.start.character,
                            range.end.line,
                            range.end.character,
                        )
                    });
                    Some((
                        self.lift_current_source_view_call_item(&selected, views)?,
                        ranges,
                    ))
                })
                .collect::<Option<Vec<_>>>()
        })
        .await
    }
}

impl Backend {
    /// Routing asks the actual retained producer, never a new standalone child.
    pub(crate) async fn original_source_view_call_prepare_route(
        &self,
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
    ) -> DocumentSourceResult<
        std::ops::ControlFlow<Option<Vec<tower_lsp_server::ls_types::CallHierarchyItem>>>,
    > {
        let Some(settlement) = self.refresh_original_source_view_graph().await? else {
            return Ok(std::ops::ControlFlow::Break(None));
        };
        let Some(ids) = settlement.projection(uri, false) else {
            return Ok(std::ops::ControlFlow::Break(None));
        };
        let logical = self
            .with_settled_original_source_scope(&ids, |views| {
                let selected = views
                    .iter()
                    .filter(|view| &view.draft.physical.uri == uri)
                    .collect::<Vec<_>>();
                (!selected.is_empty()).then(|| {
                    !selected.iter().any(|view| view.draft.is_interpreted())
                        || selected.iter().all(|view| {
                            view.draft
                                .producer
                                .seed
                                .analysis
                                .allows_lexical_declaration_advice()
                        })
                })
            })
            .await?;
        if logical == Some(true) && settlement.aggregate_complete() {
            return Ok(std::ops::ControlFlow::Continue(()));
        }
        Ok(std::ops::ControlFlow::Break(
            self.prepare_original_source_view_calls(&ids, uri, position)
                .await?,
        ))
    }
}

impl Backend {
    async fn await_original_source_navigation_readiness(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<()> {
        self.edits_settled().await;
        let is_open = {
            let documents = self
                .documents
                .lock("source_view_navigation_readiness")
                .await;
            documents.contains_key(uri)
        };
        if is_open {
            // Join live index publication before acquiring either view gate.
            // Closed and virtual source-view roots keep their existing route.
            let _ = self.read_document(uri).await?;
        }
        Ok(())
    }

    /// View-aware navigation for inherited interpretations. Ordinary roots keep
    /// their existing providers; a missing required inherited view is terminal.
    pub(crate) async fn original_source_view_navigation_route(
        &self,
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
        references: Option<bool>,
    ) -> DocumentSourceResult<
        std::ops::ControlFlow<Option<Vec<tower_lsp_server::ls_types::Location>>>,
    > {
        self.await_original_source_navigation_readiness(uri).await?;
        // A settled scope refused while another task holds the view table is
        // contention, not an answer; only a refusal that persists is terminal.
        for _ in 1..SOURCE_VIEW_CONTENTION_ATTEMPTS {
            if let Some(route) = self
                .original_source_view_navigation_attempt(uri, position, references)
                .await?
            {
                return Ok(route);
            }
            crate::rt::sleep(SOURCE_VIEW_CONTENTION_BACKOFF).await;
        }
        Ok(self
            .original_source_view_navigation_attempt(uri, position, references)
            .await?
            .unwrap_or(std::ops::ControlFlow::Break(None)))
    }

    /// One navigation attempt; `None` when the settled scope could not be read
    /// consistently and the attempt may be repeated.
    async fn original_source_view_navigation_attempt(
        &self,
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
        references: Option<bool>,
    ) -> DocumentSourceResult<Option<HostNavigationRoute>> {
        // The caller owns the bounded retry budget; do not nest a second one.
        let Some(settlement) = self.refresh_original_source_view_graph_once().await? else {
            return Ok(None);
        };
        if settlement.requires_recapture(uri) {
            return Ok(None);
        }
        let Some(ids) = settlement.navigation_projection(uri, references) else {
            return Ok(Some(std::ops::ControlFlow::Break(None)));
        };
        let aggregate_complete = settlement.aggregate_complete();
        let ordinary_scope_complete = if references.is_none() {
            settlement.ordinary_definition_scope_is_complete(uri, &ids)
        } else {
            aggregate_complete
        };
        let partial_definition_groups = ids.partial_definition_groups.clone();
        let uri = uri.clone();
        let computed = self
            .compute_original_source_views(&ids, move |views| {
                let ordinary = Self::calculate_original_source_navigation(
                    views,
                    &uri,
                    position,
                    references,
                    ordinary_scope_complete,
                    &partial_definition_groups,
                );
                let conditional = if ordinary.is_none()
                    && references.is_none()
                    && aggregate_complete
                {
                    let (publications, ancestors) = Self::original_source_view_labels(views)?;
                    let inventory =
                        Self::original_source_view_inventory(views, &publications, &ancestors);
                    let source = views.iter().find(|view| view.draft.physical.uri == uri)?;
                    let source = &source.draft.physical.source;
                    let lines = tcl_lexer::LineIndex::new_lsp(source);
                    let offset = lines.offset_at_utf16(
                        position.line,
                        tcl_lexer::Utf16Col::new(position.character),
                        source,
                    );
                    let exact = lines.position_at_utf16(offset, source);
                    if exact.line != position.line || exact.character.get() != position.character {
                        return Some((ordinary, None));
                    }
                    tcl_lsp_core::original_source_views::conditional_external_autoload_requests(
                        uri.as_str(),
                        offset,
                        &inventory,
                    )
                } else {
                    None
                };
                Some((ordinary, conditional))
            })
            .await?;
        let Some((owners, (ordinary, conditional))) = computed else {
            // Retain the existing bounded retry behavior for an initial lease.
            return Ok(None);
        };
        let output = if let Some(output) = ordinary {
            output
        } else if let Some(requests) = conditional {
            let Some(locations) = self
                .conditional_source_view_autoload_locations(&ids, &owners, &requests)
                .await?
            else {
                return Ok(Some(std::ops::ControlFlow::Break(None)));
            };
            // The provider and view leases have already been joined together.
            return Ok(Some(std::ops::ControlFlow::Break(Some(locations))));
        } else {
            return Ok(Some(std::ops::ControlFlow::Break(None)));
        };
        let mut output = Some(output);
        let result = self
            .with_settled_original_source_scope(&ids, |views| {
                Self::same_original_source_view_inventory(&owners, views).then_some(())?;
                output.take()
            })
            .await?;
        Ok(result)
    }
}

impl Backend {
    #[cfg(test)]
    pub(crate) async fn original_source_view_hover_route(
        &self,
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Option<tcl_lsp_core::hover::Hover>>> {
        self.await_original_source_navigation_readiness(uri).await?;
        let Some(document) = self.read_document(uri).await? else {
            return Ok(std::ops::ControlFlow::Break(None));
        };
        let analysis = self
            .analysis_for(uri, Arc::clone(&document.text), document.dialect)
            .await?;
        self.original_source_view_hover_route_for_snapshot(uri, &document.text, &analysis, position)
            .await
    }

    /// Completed source interpretations keep their original declaration owners
    /// through rendering. Ordinary source keeps its existing provider route.
    pub(crate) async fn original_source_view_hover_route_for_snapshot(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
        position: tower_lsp_server::ls_types::Position,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Option<tcl_lsp_core::hover::Hover>>> {
        if !self
            .original_source_hover_is_required(uri, source, analysis)
            .await?
        {
            return Ok(std::ops::ControlFlow::Continue(()));
        }
        let Some(settlement) = self.refresh_original_source_view_graph_for_uri(uri).await? else {
            return Ok(std::ops::ControlFlow::Break(
                self.original_source_authoring_hover(uri, position).await?,
            ));
        };
        let (projection, partial) = if let Some(projection) = settlement.projection(uri, false) {
            (projection, false)
        } else if let Some(projection) = settlement.definition_projection(uri)
            && !projection.partial_definition_groups.is_empty()
        {
            (projection, true)
        } else {
            return Ok(std::ops::ControlFlow::Break(
                self.original_source_authoring_hover(uri, position).await?,
            ));
        };
        let worker_uri = uri.clone();
        let partial_groups = projection.partial_definition_groups.clone();
        let computed = self
            .compute_original_source_views(&projection, move |views| {
                Some(if partial {
                    Self::calculate_partial_original_procedure_hover(
                        views,
                        &worker_uri,
                        position,
                        &partial_groups,
                    )
                } else {
                    Self::calculate_original_source_hover(views, &worker_uri, position)
                })
            })
            .await?;
        let Some((owners, Some(output))) = computed else {
            return if partial {
                Ok(std::ops::ControlFlow::Break(
                    self.original_source_authoring_hover(uri, position).await?,
                ))
            } else {
                Ok(std::ops::ControlFlow::Break(None))
            };
        };
        let mut output = Some(output);
        let result = self
            .with_settled_original_source_scope(&projection, |views| {
                Self::same_original_source_view_inventory(&owners, views).then_some(())?;
                output.take()
            })
            .await?;
        Ok(result.unwrap_or(std::ops::ControlFlow::Break(None)))
    }

    /// Positive source cards select this route. A declined route supplies no
    /// absence, completion or permission to any existing provider.
    async fn original_source_hover_is_required(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
    ) -> DocumentSourceResult<bool> {
        let local =
            tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView::source_edges_for_analysis(
                uri.as_str(),
                source,
                analysis,
                analysis.original_source_region(),
            );
        if local.is_some_and(|edges| !edges.is_empty()) {
            return Ok(true);
        }
        let ids = {
            let table = self
                .original_source_views
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            table.by_uri.get(uri).cloned().unwrap_or_default()
        };
        for id in ids {
            if self
                .with_current_original_source_view(id, |view| Some(view.draft.is_interpreted()))
                .await?
                == Some(true)
            {
                return Ok(true);
            }
        }
        let Some(work) = self.original_source_root_work().await? else {
            return Ok(false);
        };
        Ok(work.associated.contains(uri))
    }

    /// An unavailable graph can still document its current original source
    /// vocabulary; this path grants no declaration, variable value or load.
    async fn original_source_authoring_hover(
        &self,
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
    ) -> DocumentSourceResult<Option<tcl_lsp_core::hover::Hover>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(snapshot) = self.read_document(uri).await? else {
            return Ok(None);
        };
        let Some(owner) = self
            .capture_workspace_symbol_analysis(uri, &snapshot)
            .await?
        else {
            return Ok(None);
        };
        let Some(publication) = self.library_publication_source(uri, snapshot.clone()).await else {
            return Ok(None);
        };
        let analysis = match &owner.kind {
            super::WorkspaceSymbolAnalysisKind::Database(document) => {
                Arc::clone(&document.analysis)
            }
            super::WorkspaceSymbolAnalysisKind::Fresh(document) => {
                Arc::clone(&document.seed.analysis)
            }
        };
        let text = Arc::clone(&snapshot.text);
        let request_uri = uri.clone();
        let mut hover = super::super::rt::spawn_blocking(move || {
            tcl_lsp_core::hover::source_documentation_hover(
                &text,
                position.line,
                position.character,
                &analysis,
            )
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker { uri: request_uri })?;
        let Some(saved) = self.readonly_saved_sources(&[&publication]).await? else {
            return Ok(None);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        self.with_current_readonly_source_state(
            epoch,
            generation,
            &[owner.admission()],
            Some(&saved),
            |state| {
                (owner.matches_snapshot(uri, &snapshot)
                    && owner.matches_current_state(state, epoch, generation))
                .then_some(())
            },
            |_, ()| hover.take(),
        )
        .await
    }

    fn calculate_partial_original_procedure_hover(
        views: &[Arc<PublishedOriginalSourceView>],
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
        groups: &[Vec<OriginalSourceViewId>],
    ) -> Option<std::ops::ControlFlow<Option<tcl_lsp_core::hover::Hover>>> {
        let source = &views
            .iter()
            .find(|view| &view.draft.physical.uri == uri)?
            .draft
            .physical
            .source;
        let index = tcl_lexer::LineIndex::new_lsp(source);
        let offset = index.offset_at_utf16(
            position.line,
            tcl_lexer::Utf16Col::new(position.character),
            source,
        );
        let actual = index.position_at_utf16(offset, source);
        if actual.line != position.line || actual.character.get() != position.character {
            return None;
        }
        let (publications, ancestors) = Self::original_source_view_labels(views)?;
        let inventory = Self::original_source_view_inventory(views, &publications, &ancestors);
        let selected = tcl_lsp_core::original_source_views::cursor_publications(
            uri.as_str(),
            offset,
            &inventory,
        )?;
        for group in groups {
            let selected_in_group = views
                .iter()
                .zip(&inventory)
                .filter(|(view, row)| {
                    group.contains(&view.id) && selected.contains(row.publication)
                })
                .collect::<Vec<_>>();
            if selected_in_group.is_empty()
                || selected_in_group.iter().any(|(_, row)| {
                    row.region.is_none()
                        || tcl_lsp_core::original_source_views::region_owns_cursor(row, offset)
                            != Some(true)
                })
            {
                return None;
            }
        }
        let mut rendered = Vec::new();
        for publication in selected {
            let declaration =
                tcl_lsp_core::original_source_views::prepare(&publication, offset, &inventory)?;
            let hover =
                tcl_lsp_core::original_source_views::procedure_hover(&declaration, &inventory)?;
            if !rendered.contains(&hover) {
                rendered.push(hover);
            }
        }
        let first = rendered.first()?.clone();
        Some(std::ops::ControlFlow::Break(Some(
            tcl_lsp_core::hover::Hover {
                value: rendered
                    .into_iter()
                    .map(|hover| hover.value)
                    .collect::<Vec<_>>()
                    .join("\n\n---\n\n"),
                kind: first.kind,
            },
        )))
    }

    fn calculate_original_source_hover(
        views: &[Arc<PublishedOriginalSourceView>],
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
    ) -> Option<std::ops::ControlFlow<Option<tcl_lsp_core::hover::Hover>>> {
        let selected = views
            .iter()
            .filter(|view| &view.draft.physical.uri == uri)
            .collect::<Vec<_>>();
        let first = selected.first()?;
        let source = &first.draft.physical.source;
        let index = tcl_lexer::LineIndex::new_lsp(source);
        let offset = index.offset_at_utf16(
            position.line,
            tcl_lexer::Utf16Col::new(position.character),
            source,
        );
        let actual = index.position_at_utf16(offset, source);
        if actual.line != position.line || actual.character.get() != position.character {
            return None;
        }
        let (publications, ancestors) = Self::original_source_view_labels(views)?;
        let inventory = Self::original_source_view_inventory(views, &publications, &ancestors);
        let mut rendered = Vec::new();
        for publication in tcl_lsp_core::original_source_views::hover_publications(
            uri.as_str(),
            offset,
            &inventory,
        )? {
            let hover = match tcl_lsp_core::original_source_views::prepare(
                &publication,
                offset,
                &inventory,
            ) {
                Some(selected) => {
                    tcl_lsp_core::original_source_views::procedure_hover(&selected, &inventory)
                }
                None => tcl_lsp_core::original_source_views::documentation_hover(
                    &publication,
                    offset,
                    &inventory,
                ),
            }?;
            if !rendered.contains(&hover) {
                rendered.push(hover);
            }
        }
        let first = rendered.first()?.clone();
        let hover = tcl_lsp_core::hover::Hover {
            value: rendered
                .into_iter()
                .map(|hover| hover.value)
                .collect::<Vec<_>>()
                .join("\n\n---\n\n"),
            kind: first.kind,
        };
        Some(std::ops::ControlFlow::Break(Some(hover)))
    }
}

type HostNavigationRoute = std::ops::ControlFlow<Option<Vec<tower_lsp_server::ls_types::Location>>>;

/// Bound on source-view navigation attempts refused by table contention.
pub(crate) const SOURCE_VIEW_CONTENTION_ATTEMPTS: usize = 40;
/// Pause between those attempts.
pub(crate) const SOURCE_VIEW_CONTENTION_BACKOFF: std::time::Duration =
    std::time::Duration::from_millis(25);
impl Backend {
    fn calculate_original_source_navigation(
        views: &[Arc<PublishedOriginalSourceView>],
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
        references: Option<bool>,
        ordinary_scope_complete: bool,
        partial_definition_groups: &[Vec<OriginalSourceViewId>],
    ) -> Option<HostNavigationRoute> {
        let selected_views = views
            .iter()
            .filter(|view| &view.draft.physical.uri == uri)
            .collect::<Vec<_>>();
        if ordinary_scope_complete
            && !selected_views
                .iter()
                .any(|view| view.draft.is_interpreted())
        {
            return Some(std::ops::ControlFlow::Continue(()));
        }
        let (publications, ancestors) = Self::original_source_view_labels(views)?;
        let inventory = Self::original_source_view_inventory(views, &publications, &ancestors);
        let mut result = Vec::new();

        let first = selected_views.first()?;
        let source = &first.draft.physical.source;
        let offset = tcl_lexer::LineIndex::new(source).offset_at_utf16(
            position.line,
            tcl_lexer::Utf16Col::new(position.character),
            source,
        );

        let selected_publications = tcl_lsp_core::original_source_views::cursor_publications(
            uri.as_str(),
            offset,
            &inventory,
        )?;
        for group in partial_definition_groups {
            let selected = views
                .iter()
                .zip(&inventory)
                .filter(|(view, row)| {
                    group.contains(&view.id) && selected_publications.contains(row.publication)
                })
                .collect::<Vec<_>>();
            if selected.is_empty()
                || selected.iter().any(|(_, row)| {
                    row.region.is_none()
                        || tcl_lsp_core::original_source_views::region_owns_cursor(row, offset)
                            != Some(true)
                })
            {
                return None;
            }
        }
        for publication in selected_publications {
            let selected =
                tcl_lsp_core::original_source_views::prepare(&publication, offset, &inventory);
            let selected = selected?;
            if references != Some(false) {
                let target = inventory
                    .iter()
                    .find(|row| row.publication == &selected.publication)?;
                let range = tcl_lsp_core::definition::span_to_range(
                    target.document.source,
                    &tcl_lexer::LineIndex::new_lsp(target.document.source),
                    selected.identity.span(),
                );
                result.push(tower_lsp_server::ls_types::Location {
                    uri: selected.identity.uri().parse().ok()?,
                    range: super::super::lift_lsp_range(range),
                });
            }
            if references.is_some() {
                for reference in
                    tcl_lsp_core::original_source_views::references(&selected, &inventory)
                {
                    let caller = inventory
                        .iter()
                        .find(|row| row.publication == &reference.publication)?;
                    let range = tcl_lsp_core::definition::span_to_range(
                        caller.document.source,
                        &tcl_lexer::LineIndex::new_lsp(caller.document.source),
                        reference.span,
                    );
                    result.push(tower_lsp_server::ls_types::Location {
                        uri: caller.document.uri.parse().ok()?,
                        range: super::super::lift_lsp_range(range),
                    });
                }
            }
        }
        let mut unique = Vec::new();
        for location in result {
            if !unique.contains(&location) {
                unique.push(location);
            }
        }

        Some(std::ops::ControlFlow::Break(Some(unique)))
    }
}

impl PublishedOriginalSourceView {
    fn is_indexed(&self, index: &tcl_lsp_core::workspace_index::WorkspaceIndex) -> bool {
        let stamp =
            tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(self.id);
        let mut rows = index
            .original_source_views(self.draft.physical.uri.as_str())
            .iter()
            .filter(|row| row.publication() == &stamp);
        let Some(row) = rows.next() else {
            return false;
        };
        rows.next().is_none()
            && Arc::ptr_eq(row.analysis(), &self.draft.producer.seed.analysis)
            && row.source() == &self.draft.physical.source
    }
}

impl Backend {
    /// Obtain ownership for pure worker computation. This returns no durable
    /// currentness: every result must rejoin this exact inventory at rendering.
    async fn compute_original_source_views<T: Send + Unpin + 'static>(
        &self,
        projection: &OriginalSourceScopeProjection,
        calculate: impl FnOnce(&[Arc<PublishedOriginalSourceView>]) -> Option<T> + Send + 'static,
    ) -> DocumentSourceResult<Option<(Vec<Arc<PublishedOriginalSourceView>>, T)>> {
        let Some(owners) = self
            .with_settled_original_source_scope(projection, |views| Some(views.to_vec()))
            .await?
        else {
            return Ok(None);
        };
        let Some(first) = owners.first() else {
            return Ok(None);
        };
        let uri = first.draft.physical.uri.clone();
        let worker_owners = owners.clone();
        let result = super::super::rt::spawn_blocking(move || calculate(&worker_owners))
            .await
            .map_err(|_| super::super::DocumentSourceUnavailable::Worker { uri })?;
        Ok(result.map(|result| (owners, result)))
    }

    fn same_original_source_view_inventory(
        saved: &[Arc<PublishedOriginalSourceView>],
        current: &[Arc<PublishedOriginalSourceView>],
    ) -> bool {
        saved.len() == current.len()
            && saved.iter().all(|old| {
                current
                    .iter()
                    .any(|new| old.id == new.id && old.draft.same_live_producer(&new.draft))
            })
    }
}

/// Independent current provider header for conditional deferred-body advice.
/// It is not installed in a source view or promoted to a runtime binding.
struct ConditionalAutoloadProviderSource {
    uri: Uri,
    source: Arc<str>,
    admission: Arc<super::super::DocumentPathAdmission>,
    publication: super::super::LibraryPublicationSource,
}

struct ConditionalAutoloadProvider {
    source: ConditionalAutoloadProviderSource,
    owner: WorkspaceSymbolSourceAnalysis,
}

impl Backend {
    fn conditional_autoload_saved_sources(
        publications: &[&super::super::LibraryPublicationSource],
        admissions: &[&super::super::DocumentPathAdmission],
    ) -> Option<Vec<(Arc<super::super::DocumentPathAdmission>, Arc<[u8]>)>> {
        (publications.len() == admissions.len()).then_some(())?;
        let mut saved = Vec::new();
        for (publication, expected) in publications.iter().zip(admissions) {
            if publication.open_revision.is_some() {
                continue;
            }
            // Only this publication's original bytes can lease its producer.
            // Equal normalized text from a later read cannot renew the receipt.
            let admission = publication.document.path_admission.as_ref()?;
            let bytes = publication.document.saved_source_bytes.as_ref()?;
            if admission.as_ref() != *expected || admission.uri() != &publication.uri {
                return None;
            }
            saved.push((Arc::clone(admission), Arc::clone(bytes)));
        }
        Some(saved)
    }

    async fn capture_conditional_autoload_providers(
        &self,
        files: Vec<std::path::PathBuf>,
    ) -> DocumentSourceResult<Option<Vec<ConditionalAutoloadProvider>>> {
        // Keep each actual source snapshot and its existing complete readonly
        // owner. A closed, unindexed index target has no Salsa SourceFile; the
        // fresh-source owner retains its real input and currency instead.
        let mut providers = Vec::new();
        for file in files {
            let Some(uri) = crate::canonical_file_uri(&file) else {
                return Ok(None);
            };
            let Some(document) = self.read_document(&uri).await? else {
                return Ok(None);
            };
            let Some(owner) = self
                .capture_workspace_symbol_analysis(&uri, &document)
                .await?
            else {
                return Ok(None);
            };
            let Some(admission) = document.path_admission.clone() else {
                return Ok(None);
            };
            if owner.admission() != admission.as_ref() || !owner.matches_snapshot(&uri, &document) {
                return Ok(None);
            }
            let source = Arc::clone(&document.text);
            let Some(publication) = self.library_publication_source(&uri, document).await else {
                return Ok(None);
            };
            providers.push(ConditionalAutoloadProvider {
                source: ConditionalAutoloadProviderSource {
                    uri,
                    source,
                    admission,
                    publication,
                },
                owner,
            });
        }
        Ok(Some(providers))
    }

    fn conditional_autoload_provider_headers<'a>(
        providers: &'a [ConditionalAutoloadProvider],
        files: &[Uri],
    ) -> Vec<tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'a>> {
        providers
            .iter()
            .filter(|provider| files.contains(&provider.source.uri))
            .map(
                |provider| tcl_lsp_core::original_declaration::OriginalDeclarationDocument {
                    uri: provider.source.uri.as_str(),
                    source: &provider.source.source,
                    analysis: provider.owner.analysis(),
                },
            )
            .collect()
    }

    /// Read only real configured index candidates after exact view capture.
    /// Query and provider owners survive every await and rejoin atomically.
    async fn conditional_source_view_autoload_locations(
        &self,
        projection: &OriginalSourceScopeProjection,
        owners: &[Arc<PublishedOriginalSourceView>],
        requests: &[tcl_lsp_core::original_source_views::ConditionalExternalAutoloadRequest],
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::Location>>> {
        if requests.is_empty() {
            return Ok(None);
        }
        // Join actual configured-index discovery, as the ordinary library
        // request does, before retaining its resolver generation.
        drop(self.workspace_scan_gate.lock().await);
        let (files, request_files, revision) = {
            let resolver = self.package_resolver.read().await;
            let mut files = Vec::new();
            let mut request_files = Vec::new();
            for request in requests {
                let candidates = request.files(&resolver);
                if candidates.is_empty() {
                    return Ok(None);
                }
                request_files.push(
                    candidates
                        .iter()
                        .map(|file| crate::canonical_file_uri(file))
                        .collect::<Option<Vec<_>>>(),
                );
                for file in candidates {
                    if !files.contains(&file) {
                        files.push(file);
                    }
                }
            }
            let Some(request_files) = request_files.into_iter().collect::<Option<Vec<_>>>() else {
                return Ok(None);
            };
            (files, request_files, resolver.revision())
        };
        let Some(providers) = self.capture_conditional_autoload_providers(files).await? else {
            return Ok(None);
        };
        self.with_selected_original_source_views_and_provider_headers(
            &projection.ids,
            Some(projection.revision),
            &providers,
            |views| {
                Self::same_original_source_view_inventory(owners, views).then_some(())?;
                let resolver = self.package_resolver.try_read().ok()?;
                (resolver.revision() == revision).then_some(())?;
                let (publications, ancestors) = Self::original_source_view_labels(views)?;
                let inventory =
                    Self::original_source_view_inventory(views, &publications, &ancestors);
                let mut selected = None;
                for (request, files) in requests.iter().zip(&request_files) {
                    // Joint physical leasing does not let an interpretation
                    // borrow a header from another request's indexed file.
                    let headers = Self::conditional_autoload_provider_headers(&providers, files);
                    let header =
                        tcl_lsp_core::original_source_views::conditional_external_autoload_header(
                            request, &inventory, &headers,
                        )?;
                    if selected.as_ref().is_some_and(|old: &tcl_lsp_core::original_source_views::ConditionalExternalSourceHeader| old.declaration() != header.declaration()) {
                        return None;
                    }
                    selected = Some(header);
                }
                let header = selected?;
                let identity = header.declaration();
                let provider = providers
                    .iter()
                    .find(|provider| provider.source.uri.as_str() == identity.uri())?;
                identity
                    .is_current(identity.uri(), &provider.source.source, provider.owner.analysis())
                    .then_some(())?;
                let range = tcl_lsp_core::definition::span_to_range(
                    &provider.source.source,
                    &tcl_lexer::LineIndex::new_lsp(&provider.source.source),
                    identity.span(),
                );
                Some(vec![tower_lsp_server::ls_types::Location {
                    uri: provider.source.uri.clone(),
                    range: super::super::lift_lsp_range(range),
                }])
            },
        )
        .await
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod completed_world_owner_controls {
    use super::*;
    use crate::{DocumentPublication, DocumentState};

    struct Fixture {
        root: std::path::PathBuf,
        main: Uri,
        host: Uri,
        source: String,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    async fn open(backend: &Backend, uri: &Uri, source: &str) {
        backend
            .db_set_source(uri, source, "tcl8.6".to_owned())
            .await
            .unwrap();
        let mut document = DocumentState::with_version(source.to_owned(), "tcl8.6".to_owned(), 1);
        document.path_admission = Some(backend.document_path_admission(uri).await.unwrap());
        document.publication = DocumentPublication::Indexed;
        backend
            .documents
            .lock("completed world owner fixture")
            .await
            .insert(uri.clone(), document);
    }

    async fn fixture(opaque: bool) -> (Fixture, Backend, Vec<crate::OriginalDocumentSnapshot>) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "tcl-lsp-completed-owner-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let child = root.join("child.tcl");
        std::fs::write(&child, "proc helper {value} {return $value}\n").unwrap();
        let source = if opaque {
            let source =
                "proc host_old {value} { return $value }\nputs [nop \"🧪\"]; puts [host_old 7]\n";
            assert_eq!(source.len(), 77);
            source.to_owned()
        } else {
            format!(
                "source {{{}}}\nproc independent {{}} {{return [helper 21]}}\n",
                child.display()
            )
        };
        let host_source = "proc selected {} {return 42}\n";
        std::fs::write(root.join("main.tcl"), &source).unwrap();
        std::fs::write(root.join("host.tcl"), host_source).unwrap();
        let main = Uri::from_file_path(root.join("main.tcl")).unwrap();
        let host = Uri::from_file_path(root.join("host.tcl")).unwrap();
        let backend = crate::tests::test_backend();
        // Production admits closed workspace sources through the folder scan.
        *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
        backend.scan_workspace_folders().await.unwrap();
        open(&backend, &main, &source).await;
        open(&backend, &host, host_source).await;
        let documents = backend.original_declaration_documents(&host).await.unwrap();
        (
            Fixture {
                root,
                main,
                host,
                source,
            },
            backend,
            documents,
        )
    }

    #[tokio::test]
    async fn actual_prefix_owner_is_selected_without_renewing_the_old_analysis() {
        let (fixture, backend, documents) = fixture(false).await;
        let original = documents
            .iter()
            .find(|row| row.0 == fixture.main.as_str())
            .unwrap()
            .2
            .clone();
        assert!(original.original_completed_command_world().is_none());
        let (owned, capture) = backend
            .command_source_documents(false, documents, fixture.host.as_str())
            .await
            .unwrap();
        let capture = capture.unwrap();
        let analysis = &owned
            .iter()
            .find(|row| row.0 == fixture.main.as_str())
            .unwrap()
            .2;
        let worlds = capture.worlds();
        let outcome = worlds
            .iter()
            .find(|row| row.uri == fixture.main.as_str())
            .unwrap()
            .outcome;
        assert!(outcome.original_completed_command_world(analysis).is_some());
        assert!(
            outcome
                .original_completed_command_world(&original)
                .is_none()
        );
        assert!(!Arc::ptr_eq(&original, analysis));
        assert!(original.original_completed_command_world().is_none());
        assert!(
            backend
                .original_command_source_worlds_are_current(&capture)
                .await
                .unwrap()
        );
        std::fs::write(
            fixture.root.join("child.tcl"),
            "proc changed {} {return OTHER}\n",
        )
        .unwrap();
        assert!(
            !backend
                .original_command_source_worlds_are_current(&capture)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn foreign_input_source_uri_and_selected_owner_keep_their_refusals() {
        let (fixture, backend, documents) = fixture(false).await;
        let index = documents
            .iter()
            .position(|row| row.0 == fixture.main.as_str())
            .unwrap();
        let original = Arc::clone(&documents[index].2);
        let (selected, _) = backend
            .command_source_documents(false, documents.clone(), fixture.main.as_str())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&selected[index].2, &original));
        let (members, capture) = backend
            .command_source_documents(true, documents.clone(), fixture.host.as_str())
            .await
            .unwrap();
        assert!(capture.is_none() && Arc::ptr_eq(&members[index].2, &original));
        let mut foreign = documents.clone();
        foreign[index].2 =
            Arc::new(tcl_compiler::analyser::Analyser::new().analyse(&fixture.source, "tcl9.0"));
        let retained = Arc::clone(&foreign[index].2);
        let (refused, capture) = backend
            .command_source_documents(false, foreign, fixture.host.as_str())
            .await
            .unwrap();
        assert!(capture.is_none() && Arc::ptr_eq(&refused[index].2, &retained));
        let mut wrong_source = documents.clone();
        wrong_source[index].1 = Arc::from("different source");
        let (refused, capture) = backend
            .command_source_documents(false, wrong_source, fixture.host.as_str())
            .await
            .unwrap();
        assert!(capture.is_none() && Arc::ptr_eq(&refused[index].2, &original));
        let mut wrong_uri = documents;
        wrong_uri[index].0 = Uri::from_file_path(fixture.root.join("other.tcl"))
            .unwrap()
            .as_str()
            .to_owned();
        let (refused, _) = backend
            .command_source_documents(false, wrong_uri, fixture.host.as_str())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&refused[index].2, &original));
    }

    #[tokio::test]
    async fn exact_original_opaque_source_never_borrows_a_peer_world() {
        let (fixture, backend, documents) = fixture(true).await;
        let original = documents
            .iter()
            .find(|row| row.0 == fixture.main.as_str())
            .unwrap()
            .2
            .clone();
        let (owned, capture) = backend
            .command_source_documents(false, documents, fixture.host.as_str())
            .await
            .unwrap();
        let analysis = &owned
            .iter()
            .find(|row| row.0 == fixture.main.as_str())
            .unwrap()
            .2;
        assert!(Arc::ptr_eq(analysis, &original));
        assert!(analysis.original_completed_command_world().is_none());
        if let Some(capture) = capture {
            assert!(
                capture
                    .worlds()
                    .iter()
                    .all(|row| row.uri != fixture.main.as_str())
            );
        }
    }

    const REFERENCE_SOURCE: &str = "proc host_old {value} { return $value }\nset host_marker \"🧪 café\"; proc host_caller {} {return [host_old 7]}\n";

    async fn reference_fixture(
        source_host: bool,
        tail: &str,
        host_source: &str,
    ) -> (Fixture, Backend) {
        let (mut fixture, backend, documents) = fixture(false).await;
        drop(documents);
        let sourced = fixture
            .root
            .join(if source_host { "host.tcl" } else { "child.tcl" });
        fixture.source = format!("source {{{}}}\n{tail}", sourced.display());
        std::fs::write(fixture.root.join("main.tcl"), &fixture.source).unwrap();
        std::fs::write(fixture.root.join("host.tcl"), host_source).unwrap();
        open(&backend, &fixture.main, &fixture.source).await;
        open(&backend, &fixture.host, host_source).await;
        // Match the live navigation fixture: Indexed alone supplies no root.
        // Publish each actual current source/input analysis to the real index.
        for (uri, source) in [
            (&fixture.main, fixture.source.as_str()),
            (&fixture.host, host_source),
        ] {
            let analysis = backend
                .analysis_for(uri, Arc::from(source), "tcl8.6".to_owned())
                .await
                .unwrap();
            backend
                .workspace_index
                .write()
                .await
                .add_document(uri.as_str(), &analysis);
        }
        (fixture, backend)
    }

    type ReferenceScopeCensusRow = (bool, bool, bool, bool, bool, usize, bool, usize, usize);
    fn reference_scope_census(
        settlement: &OriginalSourceViewSettlement,
        fixture: &Fixture,
    ) -> Vec<ReferenceScopeCensusRow> {
        settlement
            .scopes
            .iter()
            .take(8)
            .map(|scope| {
                (
                    scope.root_uri == fixture.main,
                    scope.root_uri == fixture.host,
                    scope.root.is_some(),
                    scope.complete,
                    scope.conditional_obligations,
                    scope.ids.len(),
                    scope.required.contains(&fixture.host),
                    scope.required.len(),
                    scope.association_owners.len(),
                )
            })
            .collect()
    }

    fn assert_reference_locations(host: &Uri, locations: &[tower_lsp_server::ls_types::Location]) {
        assert_eq!(locations.len(), 2);
        for (line, start, end) in [(0, 5, 13), (1, 56, 64)] {
            assert!(locations.iter().any(|location| &location.uri == host
                && location.range.start.line == line
                && location.range.end.line == line
                && location.range.start.character == start
                && location.range.end.character == end));
        }
    }

    #[tokio::test]
    async fn unfinished_suffix_definition_uses_only_its_owned_region() {
        let (fixture, backend) = reference_fixture(
            true,
            "proc main {} {helper}\nmain\n",
            "proc helper {} {return 8}\n",
        )
        .await;
        let settlement = backend
            .refresh_original_source_view_graph()
            .await
            .unwrap()
            .unwrap();
        let scope = settlement
            .scopes
            .iter()
            .find(|scope| scope.root_uri == fixture.main)
            .unwrap();
        assert!(!scope.complete);
        assert!(scope.partial_definition.is_some());
        assert!(settlement.projection(&fixture.main, false).is_none());
        assert!(
            settlement
                .positive_reference_projection(&fixture.main)
                .is_none()
        );
        let partial_projection = settlement.definition_projection(&fixture.main).unwrap();
        let rejected_prefix = backend
            .with_settled_original_source_scope(&partial_projection, |views| {
                Backend::calculate_partial_original_procedure_hover(
                    views,
                    &fixture.main,
                    tower_lsp_server::ls_types::Position {
                        line: 0,
                        character: 0,
                    },
                    &partial_projection.partial_definition_groups,
                )
            })
            .await
            .unwrap();
        assert!(rejected_prefix.is_none());
        let position = tower_lsp_server::ls_types::Position {
            line: 1,
            character: 15,
        };
        let result = backend
            .original_source_view_navigation_route(&fixture.main, position, None)
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(locations)) = result else {
            panic!("definition unavailable")
        };
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri, fixture.host);
        assert_eq!(locations[0].range.start.line, 0);
        assert_eq!(locations[0].range.start.character, 5);
        let hover = backend
            .original_source_view_hover_route(&fixture.main, position)
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(hover)) = hover else {
            panic!("partial procedure hover unavailable")
        };
        assert!(hover.value.contains("proc ::helper {}"));
        assert!(hover.value.contains("Conditional source declaration"));
        assert!(matches!(
            backend
                .original_source_view_navigation_route(&fixture.main, position, Some(true))
                .await
                .unwrap(),
            std::ops::ControlFlow::Break(None)
        ));
        // A cursor on the source prefix is not owned by the issued suffix.
        let prefix = tower_lsp_server::ls_types::Position {
            line: 0,
            character: 0,
        };
        assert!(matches!(
            backend
                .original_source_view_navigation_route(&fixture.main, prefix, None)
                .await
                .unwrap(),
            std::ops::ControlFlow::Break(None)
        ));
    }

    #[tokio::test]
    async fn unfinished_suffix_hover_rejoins_current_source_inputs() {
        let (fixture, backend) = reference_fixture(
            true,
            "proc main {} {helper}\nmain\n",
            "proc helper {} {return 8}\n",
        )
        .await;
        let settlement = backend
            .refresh_original_source_view_graph()
            .await
            .unwrap()
            .unwrap();
        let projection = settlement.definition_projection(&fixture.main).unwrap();
        let uri = fixture.main.clone();
        let groups = projection.partial_definition_groups.clone();
        let (owners, output) = backend
            .compute_original_source_views(&projection, move |views| {
                Some(Backend::calculate_partial_original_procedure_hover(
                    views,
                    &uri,
                    tower_lsp_server::ls_types::Position {
                        line: 1,
                        character: 15,
                    },
                    &groups,
                ))
            })
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            output,
            Some(std::ops::ControlFlow::Break(Some(_)))
        ));
        backend.invalidate_diag_inputs();
        assert!(
            backend
                .with_settled_original_source_scope(&projection, |views| {
                    Backend::same_original_source_view_inventory(&owners, views).then_some(())
                })
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn unfinished_suffix_definition_rejects_unknown_body_mutation() {
        let (fixture, backend) = reference_fixture(
            true,
            "proc main {} {opaque; helper}\nmain\n",
            "proc helper {} {return 8}\n",
        )
        .await;
        let position = tower_lsp_server::ls_types::Position {
            line: 1,
            character: 23,
        };
        assert!(matches!(
            backend
                .original_source_view_hover_route(&fixture.main, position)
                .await
                .unwrap(),
            std::ops::ControlFlow::Break(None)
        ));
        assert!(matches!(
            backend
                .original_source_view_navigation_route(&fixture.main, position, None)
                .await
                .unwrap(),
            std::ops::ControlFlow::Break(None)
        ));
    }

    #[tokio::test]
    async fn positive_references_from_root_follow_its_source_view() {
        let (fixture, backend) = reference_fixture(true, "host_old 9\n", REFERENCE_SOURCE).await;
        let route = backend
            .original_source_view_navigation_route(
                &fixture.main,
                tower_lsp_server::ls_types::Position {
                    line: 1,
                    character: 2,
                },
                Some(true),
            )
            .await
            .unwrap();
        let locations = match route {
            std::ops::ControlFlow::Break(Some(locations)) => locations,
            other => panic!("source-view references unavailable: {other:?}"),
        };
        assert!(
            locations
                .iter()
                .any(|location| location.uri == fixture.main && location.range.start.line == 1)
        );
        assert_reference_locations(
            &fixture.host,
            &locations
                .iter()
                .filter(|location| location.uri == fixture.host)
                .cloned()
                .collect::<Vec<_>>(),
        );
    }

    #[tokio::test]
    async fn positive_references_keep_all_live_views_without_unrelated_completion() {
        assert_eq!(REFERENCE_SOURCE.len(), 112);
        let (fixture, backend) = reference_fixture(false, "puts opaque\n", REFERENCE_SOURCE).await;
        let settlement = backend
            .refresh_original_source_view_graph()
            .await
            .unwrap()
            .unwrap();
        assert!(
            settlement
                .scopes
                .iter()
                .any(|scope| scope.root_uri == fixture.main
                    && !scope.complete
                    && scope.ids.len() > 1),
            "scope_count={} census(main,host,published,complete,conditional,ids,host_required,required,associations)={:?}",
            settlement.scopes.len(),
            reference_scope_census(&settlement, &fixture),
        );
        assert!(settlement.projection(&fixture.host, true).is_none());
        let projection = settlement
            .positive_reference_projection(&fixture.host)
            .unwrap();
        let expected = settlement
            .scopes
            .iter()
            .flat_map(|scope| scope.ids.iter().copied())
            .collect::<HashSet<_>>();
        assert_eq!(
            projection.ids.iter().copied().collect::<HashSet<_>>(),
            expected
        );
        assert_eq!(projection.revision, settlement.revision);
        let route = backend
            .original_source_view_navigation_route(
                &fixture.host,
                tower_lsp_server::ls_types::Position {
                    line: 0,
                    character: 6,
                },
                Some(true),
            )
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(locations)) = route else {
            panic!("positive reference inventory unavailable")
        };
        assert_reference_locations(&fixture.host, &locations);
    }

    #[tokio::test]
    async fn positive_references_refuse_selected_and_dependent_unknown_mutation() {
        for source_host in [false, true] {
            let host = if source_host {
                REFERENCE_SOURCE
            } else {
                "proc host_old {value} {return $value}\neval [gets stdin]\nhost_old 7\n"
            };
            let (fixture, backend) =
                reference_fixture(source_host, "eval [gets stdin]\nhost_old 9\n", host).await;
            let settlement = backend
                .refresh_original_source_view_graph()
                .await
                .unwrap()
                .unwrap();
            assert!(
                settlement
                    .scopes
                    .iter()
                    .any(|scope| scope.required.contains(&fixture.host)
                        && (!scope.complete || scope.root_world_uncertain)),
                "source_host={source_host} scope_count={} census(main,host,published,complete,conditional,ids,host_required,required,associations)={:?}",
                settlement.scopes.len(),
                reference_scope_census(&settlement, &fixture),
            );
            assert!(
                settlement
                    .positive_reference_projection(&fixture.host)
                    .is_none()
            );
            assert!(matches!(
                backend
                    .original_source_view_navigation_route(
                        &fixture.host,
                        tower_lsp_server::ls_types::Position {
                            line: 0,
                            character: 6
                        },
                        Some(true)
                    )
                    .await
                    .unwrap(),
                std::ops::ControlFlow::Break(None)
            ));
        }
    }

    #[tokio::test]
    async fn positive_references_refuse_foreign_publication_and_changed_input() {
        let (fixture, backend) = reference_fixture(false, "puts opaque\n", REFERENCE_SOURCE).await;
        let settlement = backend
            .refresh_original_source_view_graph()
            .await
            .unwrap()
            .unwrap();
        let projection = settlement
            .positive_reference_projection(&fixture.host)
            .unwrap();
        let checked = backend
            .with_settled_original_source_scope(&projection, |views| {
                let (publications, ancestors) = Backend::original_source_view_labels(views)?;
                let inventory =
                    Backend::original_source_view_inventory(views, &publications, &ancestors);
                let cursor = tcl_lsp_core::original_source_views::cursor_publications(
                    fixture.host.as_str(),
                    6,
                    &inventory,
                )?;
                let selected =
                    tcl_lsp_core::original_source_views::prepare(cursor.first()?, 6, &inventory)?;
                assert_eq!(
                    tcl_lsp_core::original_source_views::references(&selected, &inventory).len(),
                    1
                );
                let foreign = tcl_lsp_core::original_source_views::OriginalSourceViewDeclaration {
                    publication:
                        tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(
                            "foreign publication",
                        ),
                    identity: selected.identity.clone(),
                };
                assert!(
                    tcl_lsp_core::original_source_views::references(&foreign, &inventory)
                        .is_empty()
                );
                Some(())
            })
            .await
            .unwrap();
        assert!(checked.is_some());
        backend
            .db_set_source(&fixture.host, REFERENCE_SOURCE, "tcl9.0".to_owned())
            .await
            .unwrap();
        let mut document =
            DocumentState::with_version(REFERENCE_SOURCE.to_owned(), "tcl9.0".to_owned(), 1);
        document.path_admission = Some(
            backend
                .document_path_admission(&fixture.host)
                .await
                .unwrap(),
        );
        document.publication = DocumentPublication::Indexed;
        backend
            .documents
            .lock("foreign reference input")
            .await
            .insert(fixture.host.clone(), document);
        assert!(
            backend
                .with_settled_original_source_scope(&projection, |_| Some(()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn positive_references_keep_unrelated_child_physical_lease() {
        let (fixture, backend) = reference_fixture(false, "puts opaque\n", REFERENCE_SOURCE).await;
        let settlement = backend
            .refresh_original_source_view_graph()
            .await
            .unwrap()
            .unwrap();
        let projection = settlement
            .positive_reference_projection(&fixture.host)
            .unwrap();
        assert!(
            backend
                .with_settled_original_source_scope(&projection, |_| Some(()))
                .await
                .unwrap()
                .is_some()
        );
        std::fs::write(
            fixture.root.join("child.tcl"),
            "proc changed {} {return OTHER}\n",
        )
        .unwrap();
        assert!(
            backend
                .with_settled_original_source_scope(&projection, |_| Some(()))
                .await
                .unwrap()
                .is_none(),
            "projection_ids={} scope_count={} census(main,host,published,complete,conditional,ids,host_required,required,associations)={:?}",
            projection.ids.len(),
            settlement.scopes.len(),
            reference_scope_census(&settlement, &fixture),
        );
        assert_eq!(
            std::fs::read_to_string(fixture.root.join("host.tcl")).unwrap(),
            REFERENCE_SOURCE
        );
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod closed_source_child_ingress_tests {
    use super::*;
    use crate::{DocumentPublication, DocumentState, RehomingDocumentSnapshot};
    use futures_util::{SinkExt, StreamExt};
    use tower::{Service, ServiceExt};
    use tower_lsp_server::{
        ExitedError, LspService,
        jsonrpc::{Request, Response},
        ls_types::Position,
    };

    const CALLER: &str = "source child.tcl\nproc caller {} {return [helper 7]}\n";
    const CHILD: &str = "proc helper {value} {return $value}\n";
    const EXACT_APP: &str = include_str!("../../../tcl-lsp-server-wasi/test/fixture/app.tcl");
    const EXACT_LIB: &str = include_str!("../../../tcl-lsp-server-wasi/test/fixture/lib.tcl");

    struct Fixture {
        root: std::path::PathBuf,
        main: Uri,
        child: Uri,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    async fn open(backend: &Backend, uri: &Uri, source: &str) {
        backend
            .db_set_source(uri, source, "tcl8.6".to_owned())
            .await
            .unwrap();
        let admission = backend.document_path_admission(uri).await.unwrap();
        let mut document = DocumentState::with_version(source.to_owned(), "tcl8.6".to_owned(), 1);
        document.path_admission = Some(admission);
        document.publication = DocumentPublication::Indexed;
        backend
            .documents
            .lock("closed source child fixture")
            .await
            .insert(uri.clone(), document);
        let analysis = backend
            .analysis_for(uri, Arc::from(source), "tcl8.6".to_owned())
            .await
            .unwrap();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    async fn fixture() -> (Fixture, Backend) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "tcl-lsp-source-child-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("main.tcl"), CALLER).unwrap();
        std::fs::write(root.join("child.tcl"), CHILD).unwrap();
        let main = Uri::from_file_path(root.join("main.tcl")).unwrap();
        let child = Uri::from_file_path(root.join("child.tcl")).unwrap();
        let backend = crate::tests::test_backend();
        open(&backend, &main, CALLER).await;
        (Fixture { root, main, child }, backend)
    }

    async fn child_view(fixture: &Fixture, backend: &Backend) -> Arc<PublishedOriginalSourceView> {
        let route = backend
            .original_source_view_navigation_route(&fixture.main, Position::new(1, 24), None)
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(locations)) = route else {
            panic!("expected inherited source navigation")
        };
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri, fixture.child);
        let table = backend.original_source_views.lock().unwrap();
        let ids = table.by_uri.get(&fixture.child).unwrap();
        let mut views = ids
            .iter()
            .filter_map(|id| table.by_id.get(id))
            .filter(|view| view.draft.inherited.is_some());
        let view = Arc::clone(views.next().unwrap());
        assert!(views.next().is_none());
        view
    }

    async fn publication_parent(
        fixture: &Fixture,
        backend: &Backend,
    ) -> Arc<PublishedOriginalSourceView> {
        let document = backend.read_document(&fixture.main).await.unwrap().unwrap();
        let draft = backend
            .capture_initial_original_source_root(&fixture.main, document)
            .await
            .unwrap()
            .unwrap();
        let blocked = backend.rehoming_gate.lock().await;
        assert!(matches!(
            backend
                .publish_original_source_views(&fixture.main, vec![draft.clone()])
                .await
                .unwrap(),
            OriginalSourceViewAttempt::RetryCapture
        ));
        drop(blocked);
        let OriginalSourceViewAttempt::Ready(mut views) = backend
            .publish_original_source_views(&fixture.main, vec![draft])
            .await
            .unwrap()
        else {
            panic!("current root publication after contention")
        };
        assert_eq!(views.len(), 1);
        views.remove(0)
    }

    async fn inherited_draft(
        fixture: &Fixture,
        backend: &Backend,
        parent: &Arc<PublishedOriginalSourceView>,
    ) -> OriginalSourceViewDraft {
        let edges =
            tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView::source_edges_for_analysis(
                fixture.main.as_str(),
                &parent.draft.physical.source,
                &parent.draft.producer.seed.analysis,
                None,
            )
            .unwrap();
        let edge = edges
            .iter()
            .find(|edge| {
                matches!(
                    edge.original_source_frontier.as_deref(),
                    Some(SourceFrontier::CurrentCut(_))
                )
            })
            .unwrap();
        let Some(SourceFrontier::CurrentCut(cut)) = edge.original_source_frontier.as_deref() else {
            unreachable!()
        };
        let RehomingDocumentSnapshot::Ready(document) = backend
            .source_child_document_snapshot(&fixture.child)
            .await
            .unwrap()
        else {
            panic!("actual closed child publication")
        };
        let OriginalSourceViewAttempt::Ready(draft) = backend
            .capture_inherited_original_source_view(
                &fixture.child,
                document.text,
                Arc::clone(parent),
                Arc::new(cut.clone()),
            )
            .await
            .unwrap()
        else {
            panic!("genuine inherited draft")
        };
        draft
    }

    #[tokio::test]
    async fn source_publication_contention_retries_root_child_and_region_leases() {
        let (fixture, backend) = fixture().await;
        let parent = publication_parent(&fixture, &backend).await;
        let draft = inherited_draft(&fixture, &backend, &parent).await;
        let blocked = backend.rehoming_gate.lock().await;
        assert!(matches!(
            backend
                .append_inherited_original_source_view(&fixture.child, draft)
                .await
                .unwrap(),
            OriginalSourceViewAttempt::RetryCapture
        ));
        drop(blocked);
        let child = child_view(&fixture, &backend).await;
        assert!(child.draft.inherited.is_some());
        let request = {
            let table = backend.original_source_views.lock().unwrap();
            table
                .by_id
                .values()
                .find_map(|view| view.draft.region.clone())
                .expect("issued actual source region")
        };
        let OriginalSourceViewAttempt::Ready(draft) = backend
            .capture_conditional_region_draft(request)
            .await
            .unwrap()
        else {
            panic!("actual issued region draft")
        };
        let blocked = backend.rehoming_gate.lock().await;
        assert!(matches!(
            backend
                .append_original_source_regions(&fixture.main, vec![draft])
                .await
                .unwrap(),
            OriginalSourceViewAttempt::RetryCapture
        ));
        drop(blocked);
        let route = backend
            .original_source_view_navigation_route(&fixture.main, Position::new(1, 24), None)
            .await
            .unwrap();
        assert!(
            matches!(route, std::ops::ControlFlow::Break(Some(ref locations)) if locations.iter().any(|location| location.uri == fixture.child))
        );
    }

    #[tokio::test]
    async fn child_publication_retries_lost_parent_index_membership_without_relaxing_it() {
        let (fixture, backend) = fixture().await;
        let parent = publication_parent(&fixture, &backend).await;
        let draft = inherited_draft(&fixture, &backend, &parent).await;
        // The ordinary diagnostic publisher replaces this presentation through
        // the genuine index API. The source table still holds its old row.
        backend
            .workspace_index
            .write()
            .await
            .replace_document(fixture.main.as_str(), &parent.draft.producer.seed.analysis);
        assert!(
            backend
                .original_source_views
                .lock()
                .unwrap()
                .contains_actual(&parent)
        );
        assert!(!parent.is_indexed(&*backend.workspace_index.read().await));
        assert!(matches!(
            backend
                .append_inherited_original_source_view(&fixture.child, draft)
                .await
                .unwrap(),
            OriginalSourceViewAttempt::RetryCapture
        ));
        assert!(
            !backend
                .original_source_views
                .lock()
                .unwrap()
                .by_uri
                .contains_key(&fixture.child)
        );
        // Recapture through the graph's actual producers must restore the
        // parent index row before admitting a fresh child and its owned suffix.
        let child = child_view(&fixture, &backend).await;
        let current_parent = &child.draft.inherited.as_ref().unwrap().parent;
        assert!(current_parent.is_indexed(&*backend.workspace_index.read().await));
        assert_ne!(current_parent.id, parent.id);
    }

    async fn unrelated_shared_source_root(
        fixture: &Fixture,
        backend: &Backend,
    ) -> (Fixture, Arc<PublishedOriginalSourceView>) {
        let other_root = fixture.root.join("unrelated");
        std::fs::create_dir(&other_root).unwrap();
        let other_source = format!(
            "source {{{}}}\nproc other_caller {{}} {{return [helper 7]}}\n",
            fixture.root.join("child.tcl").display()
        );
        std::fs::write(other_root.join("main.tcl"), &other_source).unwrap();
        let other = Fixture {
            main: Uri::from_file_path(other_root.join("main.tcl")).unwrap(),
            child: fixture.child.clone(),
            root: other_root,
        };
        // Admit the closed physical input before issuing its independent root.
        assert!(matches!(
            backend
                .source_child_document_snapshot(&other.child)
                .await
                .unwrap(),
            RehomingDocumentSnapshot::Ready(_)
        ));
        open(backend, &other.main, &other_source).await;
        let other_parent = publication_parent(&other, backend).await;
        (other, other_parent)
    }

    fn assert_ordinary_scope(
        settlement: &OriginalSourceViewSettlement,
        selected: &Fixture,
        unrelated: &Fixture,
    ) {
        let selected_uri = &selected.main;
        let selected_definition = settlement.definition_projection(selected_uri).unwrap();
        assert!(
            settlement.ordinary_definition_scope_is_complete(selected_uri, &selected_definition)
        );
        assert!(
            !settlement
                .ordinary_definition_scope_is_complete(&unrelated.main, &selected_definition)
        );
        assert!(
            !settlement
                .ordinary_definition_scope_is_complete(&selected.child, &selected_definition)
        );
        let mut partial_definition = settlement.definition_projection(selected_uri).unwrap();
        partial_definition
            .partial_definition_groups
            .push(Vec::new());
        assert!(
            !settlement.ordinary_definition_scope_is_complete(selected_uri, &partial_definition)
        );
    }

    #[tokio::test]
    async fn unrelated_persistent_publication_refusal_preserves_selected_scope() {
        let (fixture, backend) = fixture().await;
        let selected_child = child_view(&fixture, &backend).await;
        let selected_parent = Arc::clone(&selected_child.draft.inherited.as_ref().unwrap().parent);
        let (other, other_parent) = unrelated_shared_source_root(&fixture, &backend).await;
        let work = backend.original_source_root_work().await.unwrap().unwrap();
        let bootstrap = work
            .candidates
            .iter()
            .map(|candidate| candidate.draft.clone())
            .collect::<Vec<_>>();
        let excluded = work
            .candidates
            .iter()
            .filter(|candidate| work.associated.contains(&candidate.draft.physical.uri))
            .cloned()
            .collect::<Vec<_>>();
        let incoming = work.incoming;
        backend.workspace_index.write().await.replace_document(
            other.main.as_str(),
            &other_parent.draft.producer.seed.analysis,
        );
        for _ in 0..2 {
            // No publisher restores this row between attempts: this is a
            // persistent refusal in a different, actual root interpretation.
            assert!(!other_parent.is_indexed(&*backend.workspace_index.read().await));
            let selected_scope = backend
                .run_original_source_view_work(Arc::clone(&selected_parent))
                .await
                .unwrap();
            assert!(selected_scope.complete);
            let mut other_scope = backend
                .run_original_source_view_work(Arc::clone(&other_parent))
                .await
                .unwrap();
            assert!(other_scope.recapture.is_some());
            other_scope.require_known_targets(&incoming);
            assert!(other_scope.required.contains(&other.child));
            assert!(
                other_scope.ids.is_empty()
                    && !other_scope.complete
                    && other_scope.partial_definition.is_none()
            );
            let settlement = backend
                .finish_original_root_reconciliation(OriginalSourceRootReconciliation {
                    bootstrap: bootstrap.clone(),
                    scopes: vec![selected_scope, other_scope],
                    excluded: excluded.clone(),
                    incoming: incoming.clone(),
                })
                .await
                .unwrap()
                .expect("unrelated retry debt must not discard the selected settlement");
            assert!(!settlement.requires_recapture(&fixture.main));
            assert!(settlement.requires_recapture(&other.main));
            assert!(settlement.requires_recapture(&other.child));
            assert!(!settlement.aggregate_complete());
            assert_ordinary_scope(&settlement, &fixture, &other);
            assert!(!backend.original_source_view_settlement_views_are_indexed(&settlement));
            assert!(settlement.projection(&fixture.main, true).is_none());
            assert!(settlement.definition_projection(&other.main).is_none());
            // Both real roots source the same physical helper. We make no
            // claim that their interpretation-specific target identities are
            // equivalent; the missing retained inventory alone blocks refs.
            assert!(settlement.definition_projection(&fixture.child).is_none());
            assert!(
                settlement
                    .positive_reference_projection(&fixture.main)
                    .is_none()
            );
            let projection = settlement
                .definition_projection(&fixture.main)
                .expect("current selected definition remains available");
            let uri = fixture.main.clone();
            let groups = projection.partial_definition_groups.clone();
            let (owners, route) = backend
                .compute_original_source_views(&projection, move |views| {
                    Some(Backend::calculate_original_source_navigation(
                        views,
                        &uri,
                        Position::new(1, 24),
                        None,
                        false,
                        &groups,
                    ))
                })
                .await
                .unwrap()
                .unwrap();
            assert!(
                matches!(route, Some(std::ops::ControlFlow::Break(Some(ref locations))) if locations.iter().any(|location| location.uri == fixture.child))
            );
            assert!(
                backend
                    .with_settled_original_source_scope(&projection, |views| {
                        Backend::same_original_source_view_inventory(&owners, views).then_some(())
                    })
                    .await
                    .unwrap()
                    .is_some()
            );
        }
    }

    #[tokio::test]
    async fn source_publication_never_renews_a_changed_or_deleted_saved_draft() {
        for deleted in [false, true] {
            let (fixture, backend) = fixture().await;
            let parent = publication_parent(&fixture, &backend).await;
            let draft = inherited_draft(&fixture, &backend, &parent).await;
            if deleted {
                std::fs::remove_file(fixture.root.join("child.tcl")).unwrap();
            } else {
                std::fs::write(fixture.root.join("child.tcl"), "proc replacement {} {}\n").unwrap();
            }
            assert!(matches!(
                backend
                    .append_inherited_original_source_view(&fixture.child, draft)
                    .await
                    .unwrap(),
                OriginalSourceViewAttempt::Unavailable
            ));
            assert!(
                !backend
                    .original_source_views
                    .lock()
                    .unwrap()
                    .by_uri
                    .contains_key(&fixture.child)
            );
        }
    }

    #[tokio::test]
    async fn unknown_source_frontier_remains_terminal_unavailable() {
        let (fixture, backend) = fixture().await;
        open(
            &backend,
            &fixture.main,
            "source $unresolved\nproc caller {} {return [helper 7]}\n",
        )
        .await;
        let settlement = backend
            .refresh_original_source_view_graph_once()
            .await
            .unwrap()
            .expect("semantic uncertainty is a pending scope, not publication contention");
        assert!(settlement.definition_projection(&fixture.main).is_none());
        let attempt = backend
            .original_source_view_navigation_attempt(&fixture.main, Position::new(1, 24), None)
            .await
            .unwrap();
        assert!(matches!(attempt, Some(std::ops::ControlFlow::Break(None))));
    }

    type SourceProtocolService = crate::transport_liveness::DeferredConcurrency<
        tower::util::MapRequest<LspService<Backend>, fn(Request) -> Request>,
    >;
    type SourceProtocolTask = tokio::task::JoinHandle<Result<Option<Response>, ExitedError>>;

    struct SourceProtocol {
        service: SourceProtocolService,
        backend: Backend,
        responder: tokio::task::JoinHandle<()>,
        startup: Vec<SourceProtocolTask>,
        serial: i64,
    }

    impl SourceProtocol {
        fn new() -> Self {
            let (service, socket) = LspService::new(Backend::new);
            let backend = service.inner().clone();
            let (mut outgoing, mut responses) = socket.split();
            let responder = tokio::spawn(async move {
                while let Some(request) = outgoing.next().await {
                    let (method, id, params) = request.into_parts();
                    if let Some(id) = id {
                        let result = if method == "workspace/configuration" {
                            serde_json::Value::Array(params.unwrap()["items"].as_array().unwrap().iter().map(|item| {
                                if item["section"] == "tclLsp.dialect" {serde_json::json!("tcl8.6")} else {serde_json::json!({"dialect":"tcl8.6","libraryPaths":[]})}
                            }).collect())
                        } else {
                            serde_json::Value::Null
                        };
                        responses.send(Response::from_ok(id, result)).await.unwrap();
                    }
                }
            });
            let service = service
                .map_request(crate::service::normalise_request_uris as fn(Request) -> Request);
            Self {
                service: crate::transport_liveness::DeferredConcurrency::new(
                    service,
                    crate::transport_liveness::DEFAULT_HANDLER_CONCURRENCY,
                ),
                backend,
                responder,
                startup: Vec::new(),
                serial: 0,
            }
        }

        async fn request(
            &mut self,
            method: &'static str,
            params: serde_json::Value,
        ) -> serde_json::Value {
            self.serial += 1;
            let request = Request::build(method)
                .id(self.serial)
                .params(params)
                .finish();
            self.service
                .ready()
                .await
                .unwrap()
                .call(request)
                .await
                .unwrap()
                .unwrap()
                .into_parts()
                .1
                .unwrap()
        }

        async fn notify(&mut self, method: &'static str, params: serde_json::Value) {
            let future = self
                .service
                .ready()
                .await
                .unwrap()
                .call(Request::build(method).params(params).finish());
            self.startup.push(tokio::spawn(future));
            tokio::task::yield_now().await;
        }

        async fn start(&mut self, fixture: &Fixture) {
            let root_uri = Uri::from_file_path(&fixture.root).unwrap();
            let _ = self.request("initialize", serde_json::json!({
                "processId":std::process::id(), "rootUri":null, "workspaceFolders":[],
                "initializationOptions":{"folderDialects":{(root_uri.as_str()):"tcl8.6"}},
                "capabilities":{"general":{"positionEncodings":["utf-16"]},"workspace":{"configuration":true,"workspaceFolders":true},"textDocument":{"hover":{"contentFormat":["markdown","plaintext"]},"definition":{"linkSupport":false},"publishDiagnostics":{}}}
            })).await;
            // Preserve the real concurrent notification/callback order.
            self.notify("initialized", serde_json::json!({})).await;
            self.notify(
                "workspace/didChangeConfiguration",
                serde_json::json!({"settings":{"tclLsp":{"dialect":"tcl8.6","libraryPaths":[]}}}),
            )
            .await;
            self.notify("textDocument/didOpen", serde_json::json!({"textDocument":{"uri":fixture.main,"languageId":"tcl","version":1,"text":EXACT_APP}})).await;
            let config = self.request("workspace/executeCommand", serde_json::json!({"command":"tcl-lsp.getEffectiveConfig","arguments":[fixture.main]})).await;
            assert_eq!(config["dialect"], "tcl8.6", "{config}");
        }

        async fn wait_current(&mut self, fixture: &Fixture) {
            tokio::time::timeout(std::time::Duration::from_secs(20), async {
                loop {
                    let report = self.request("workspace/executeCommand", serde_json::json!({"command":"tcl-lsp.currentDiagnostics","arguments":[{"uri":fixture.main,"version":1,"source":EXACT_APP}]})).await;
                    if report["status"] == "complete" {
                        assert_eq!(report["dialectId"], "tcl8.6");
                        assert_eq!(report["source"], EXACT_APP);
                        break;
                    }
                    assert_eq!(report["status"], "unavailable", "{report}");
                    assert!(report["reason"] == "pending" || report["reason"] == "stale", "{report}");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }).await.expect("protocol app currentDiagnostics must become complete before navigation");
        }

        async fn finish(&mut self) {
            for task in self.startup.drain(..) {
                task.await.unwrap().unwrap();
            }
            let _ = self.request("shutdown", serde_json::Value::Null).await;
            self.responder.abort();
        }
    }

    fn exact_source_fixture() -> Fixture {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "tcl-lsp-exact-wasi-first-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("app.tcl"), EXACT_APP).unwrap();
        std::fs::write(root.join("lib.tcl"), EXACT_LIB).unwrap();
        Fixture {
            main: Uri::from_file_path(root.join("app.tcl")).unwrap(),
            child: Uri::from_file_path(root.join("lib.tcl")).unwrap(),
            root,
        }
    }

    /// Genuine `Backend::new` initialization/configuration precedes one definition.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn exact_wasi_closed_child_first_public_definition_and_partial_hover() {
        let fixture = exact_source_fixture();
        let mut protocol = SourceProtocol::new();
        protocol.start(&fixture).await;
        protocol.wait_current(&fixture).await;
        assert!(protocol.backend.workspace_folders.lock().await.is_empty());
        assert!(
            !protocol
                .backend
                .documents
                .lock("exact child is closed")
                .await
                .contains_key(&fixture.child)
        );
        assert!(
            !protocol
                .backend
                .db_files
                .lock()
                .await
                .contains_key(&fixture.child)
        );
        assert!(
            !protocol
                .backend
                .workspace_index
                .read()
                .await
                .contains_document(fixture.child.as_str())
        );
        let response = protocol.request("textDocument/definition", serde_json::json!({"textDocument":{"uri":fixture.main},"position":{"line":9,"character":13}})).await;
        let expected_uri = fixture.child.as_str();
        let found = response.as_array().is_some_and(|locations| {
            locations.iter().any(|location| {
                location["uri"] == expected_uri && location["range"]["start"]["line"] == 8
            })
        });
        let hover = protocol.request("textDocument/hover", serde_json::json!({"textDocument":{"uri":fixture.main},"position":{"line":9,"character":13}})).await;
        let hover_text = serde_json::to_string(&hover).unwrap();
        // Cleanup still follows the preserved first response and separate hover.
        protocol.finish().await;
        assert!(
            found,
            "first public definition must locate actual helper declaration: {response}"
        );
        assert!(
            hover_text.contains("proc ::helper {value}")
                && hover_text.contains("Conditional source declaration"),
            "partial source hover must retain the actual helper metadata: {hover}"
        );
    }

    #[tokio::test]
    async fn navigation_compute_first_lease_refuses_contention_and_rejoins_after_release() {
        use futures_util::FutureExt as _;
        let (fixture, backend) = fixture().await;
        let settlement = backend
            .refresh_original_source_view_graph()
            .await
            .unwrap()
            .unwrap();
        let projection = settlement
            .navigation_projection(&fixture.main, None)
            .unwrap();
        let computed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        {
            let held_table = backend.original_source_views.lock().unwrap();
            let attempted = Arc::clone(&computed);
            // The first try_lock refuses synchronously. Do not await while
            // holding the standard mutex, or schedule a worker without a lease.
            let refused = backend
                .compute_original_source_views(&projection, move |_| {
                    attempted.store(true, std::sync::atomic::Ordering::SeqCst);
                    Some(())
                })
                .now_or_never()
                .expect("table contention must refuse the first lease immediately")
                .unwrap();
            assert!(refused.is_none());
            assert!(!computed.load(std::sync::atomic::Ordering::SeqCst));
            drop(held_table);
        }
        let attempted = Arc::clone(&computed);
        let leased = backend
            .compute_original_source_views(&projection, move |_| {
                attempted.store(true, std::sync::atomic::Ordering::SeqCst);
                Some(())
            })
            .await
            .unwrap();
        assert!(leased.is_some());
        assert!(computed.load(std::sync::atomic::Ordering::SeqCst));
        // Actual navigation still needs its own new settlement and final lease.
        let route = backend
            .original_source_view_navigation_attempt(&fixture.main, Position::new(1, 24), None)
            .await
            .unwrap();
        assert!(
            matches!(route, Some(std::ops::ControlFlow::Break(Some(ref locations))) if locations.iter().any(|location| location.uri == fixture.child))
        );
        // Successful computation of an unavailable semantic answer is terminal;
        // it must not be confused with an unavailable owner lease.
        let semantic = backend
            .original_source_view_navigation_attempt(&fixture.main, Position::new(0, 1), None)
            .await
            .unwrap();
        assert!(matches!(semantic, Some(std::ops::ControlFlow::Break(None))));
        // Establish currentness immediately before changing saved bytes, so
        // the withdrawal below cannot pass on an already-retired projection.
        assert!(
            backend
                .compute_original_source_views(&projection, |_| Some(()))
                .await
                .unwrap()
                .is_some()
        );
        // The previous lease cannot survive an unannounced physical change.
        std::fs::write(fixture.root.join("child.tcl"), "proc replacement {} {}\n").unwrap();
        assert!(
            backend
                .compute_original_source_views(&projection, |_| Some(()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn pending_parent_does_not_mint_an_interpretation_or_completion_receipt() {
        let (fixture, backend) = fixture().await;
        open(
            &backend,
            &fixture.main,
            "proc documented {value} {return $value}\n",
        )
        .await;
        let parent = Uri::from_file_path(fixture.root.join("pending.tcl")).unwrap();
        open(&backend, &parent, "source main.tcl\n").await;
        backend
            .documents
            .lock("pending source parent")
            .await
            .get_mut(&parent)
            .unwrap()
            .publication = DocumentPublication::Pending;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(0, 7))
            .await
            .unwrap();
        assert!(matches!(route, std::ops::ControlFlow::Continue(())));
        let table = backend.original_source_views.lock().unwrap();
        assert!(!table.by_uri.contains_key(&fixture.main));
        assert!(!table.by_uri.contains_key(&parent));
    }

    #[tokio::test]
    async fn ordinary_unknown_factory_hover_is_independent_of_unrelated_source_debt() {
        let (fixture, backend) = fixture().await;
        let caller = "UnknownFactory create Made {method go {} {return 1}}";
        open(&backend, &fixture.main, caller).await;
        let unrelated = Uri::from_file_path(fixture.root.join("unrelated.tcl")).unwrap();
        open(&backend, &unrelated, "source missing.tcl\n").await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(0, 36))
            .await
            .unwrap();
        assert!(matches!(route, std::ops::ControlFlow::Continue(())));
    }

    #[tokio::test]
    async fn incoming_source_association_keeps_child_hover_in_its_inherited_namespace() {
        let (fixture, backend) = fixture().await;
        open(
            &backend,
            &fixture.main,
            "namespace eval ::ns {source child.tcl}\n",
        )
        .await;
        let route = backend
            .original_source_view_hover_route(&fixture.child, Position::new(0, 7))
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(hover)) = route else {
            panic!("inherited child documentation")
        };
        assert!(hover.value.contains("proc ::ns::helper"));
        assert!(!hover.value.contains("proc ::helper"));
    }

    #[tokio::test]
    async fn sourced_class_declaration_hover_keeps_its_own_original_metadata() {
        let (fixture, backend) = fixture().await;
        open(
            &backend,
            &fixture.main,
            "source child.tcl\noo::class create Widget {\n    method run {} {return 1}\n}\n",
        )
        .await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(1, 18))
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(hover)) = route else {
            panic!("class source declaration card")
        };
        assert!(hover.value.contains("oo::class create ::Widget"));
        assert!(hover.value.contains("run"));
        assert!(hover.value.contains("availability depend on execution"));
    }

    #[tokio::test]
    async fn sourced_schema_hover_survives_complete_and_unavailable_suffixes() {
        let (fixture, backend) = fixture().await;
        open(&backend, &fixture.main, "source child.tcl\nset value 1\n").await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(1, 1))
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(hover)) = route else {
            panic!("source schema card")
        };
        assert!(hover.value.contains("**`set`**"));
        open(
            &backend,
            &fixture.main,
            "source child.tcl\nset value 1\nputs $value\n",
        )
        .await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(2, 1))
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(hover)) = route else {
            panic!("source schema despite incomplete execution")
        };
        assert!(hover.value.contains("**`puts`**"));
        assert!(hover.value.contains("command availability is unresolved"));
    }

    #[tokio::test]
    async fn sourced_special_variable_hover_keeps_exact_root_source_role() {
        let (fixture, backend) = fixture().await;
        open(
            &backend,
            &fixture.main,
            "source child.tcl\nputs $tcl_version\nputs $::env(PATH)\n",
        )
        .await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(2, 9))
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(hover)) = route else {
            panic!("root special source card")
        };
        assert!(hover.value.contains("env"));
        assert!(
            hover
                .value
                .contains("binding, availability and the current value are unresolved")
        );
        open(
            &backend,
            &fixture.main,
            "source child.tcl\nproc p {env} {puts $env(PATH)}\n",
        )
        .await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(1, 20))
            .await
            .unwrap();
        assert!(matches!(route, std::ops::ControlFlow::Break(None)));
    }

    #[tokio::test]
    async fn bounded_source_documentation_keeps_live_source_and_input_leases() {
        let (fixture, backend) = fixture().await;
        open(&backend, &fixture.main, "source missing.tcl\nputs 7\n").await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(1, 1))
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(hover)) = route else {
            panic!("unavailable source vocabulary")
        };
        assert!(hover.value.contains("**`puts`**"));
        assert!(hover.value.contains("command availability is unresolved"));
        let snapshot = backend.read_document(&fixture.main).await.unwrap().unwrap();
        let owner = backend
            .capture_workspace_symbol_analysis(&fixture.main, &snapshot)
            .await
            .unwrap()
            .unwrap();
        let epoch = backend.diag_inputs_epoch();
        let generation = *backend.class_factory_generation.read().await;
        backend.invalidate_diag_inputs();
        assert!(
            backend
                .with_current_readonly_source_state(
                    epoch,
                    generation,
                    &[owner.admission()],
                    None,
                    |state| owner
                        .matches_current_state(state, epoch, generation)
                        .then_some(()),
                    |_, ()| Some(())
                )
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn ordinary_complete_hover_keeps_its_existing_provider_route() {
        let (fixture, backend) = fixture().await;
        open(&backend, &fixture.main, "set counter 1\n").await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(0, 1))
            .await
            .unwrap();
        assert!(matches!(route, std::ops::ControlFlow::Continue(())));
    }

    #[tokio::test]
    async fn source_hover_withdraws_when_actual_analyser_inputs_change() {
        let (fixture, backend) = fixture().await;
        let settlement = backend
            .refresh_original_source_view_graph()
            .await
            .unwrap()
            .unwrap();
        let projection = settlement.projection(&fixture.main, false).unwrap();
        assert!(
            backend
                .with_settled_original_source_scope(&projection, |_| Some(()))
                .await
                .unwrap()
                .is_some()
        );
        backend.invalidate_diag_inputs();
        assert!(
            backend
                .with_settled_original_source_scope(&projection, |_| Some(()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn completed_source_hover_uses_inherited_unicode_parameter_metadata() {
        use tower_lsp_server::LanguageServer;
        use tower_lsp_server::ls_types::{
            HoverParams, TextDocumentIdentifier, TextDocumentPositionParams,
        };
        let (fixture, backend) = fixture().await;
        std::fs::write(
            fixture.root.join("child.tcl"),
            "# Documented helper\nproc helper {value {suffix 🧪}} {return $value}\n",
        )
        .unwrap();
        let hover = backend
            .hover(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: fixture.main.clone(),
                    },
                    position: Position::new(1, 24),
                },
                work_done_progress_params:
                    tower_lsp_server::ls_types::WorkDoneProgressParams::default(),
            })
            .await
            .unwrap()
            .unwrap();
        let text = serde_json::to_string(&hover).unwrap();
        assert!(text.contains("Conditional source declaration"));
        assert!(text.contains("proc ::helper {value {suffix 🧪}}"));
        assert!(text.contains("Documented helper"));
        let view = child_view(&fixture, &backend).await;
        assert!(
            view.draft
                .inherited
                .as_ref()
                .unwrap()
                .point
                .matches_child_analysis(&view.draft.producer.seed.analysis)
        );
    }

    #[tokio::test]
    async fn source_hover_rechecks_changed_and_deleted_target_before_publication() {
        for deleted in [false, true] {
            let (fixture, backend) = fixture().await;
            let settlement = backend
                .refresh_original_source_view_graph()
                .await
                .unwrap()
                .unwrap();
            let projection = settlement.projection(&fixture.main, false).unwrap();
            let uri = fixture.main.clone();
            let (owners, output) = backend
                .compute_original_source_views(&projection, move |views| {
                    Some(Backend::calculate_original_source_hover(
                        views,
                        &uri,
                        Position::new(1, 24),
                    ))
                })
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(
                output,
                Some(std::ops::ControlFlow::Break(Some(_)))
            ));
            if deleted {
                std::fs::remove_file(fixture.root.join("child.tcl")).unwrap();
            } else {
                std::fs::write(fixture.root.join("child.tcl"), "proc changed {} {}\n").unwrap();
            }
            assert!(
                backend
                    .with_settled_original_source_scope(&projection, |views| {
                        Backend::same_original_source_view_inventory(&owners, views).then_some(())
                    })
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[tokio::test]
    async fn source_hover_respects_known_replacement_in_the_same_suffix() {
        let (fixture, backend) = fixture().await;
        let source =
            "source child.tcl\nproc helper {local} {}\nproc caller {} {return [helper 7]}\n";
        open(&backend, &fixture.main, source).await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(2, 24))
            .await
            .unwrap();
        let std::ops::ControlFlow::Break(Some(hover)) = route else {
            panic!("expected selected replacement documentation")
        };
        assert!(hover.value.contains("proc ::helper {local}"));
        assert!(!hover.value.contains("proc ::helper {value}"));
    }

    #[tokio::test]
    async fn unavailable_source_hover_never_falls_through_to_lexical_names() {
        let (fixture, backend) = fixture().await;
        std::fs::remove_file(fixture.root.join("child.tcl")).unwrap();
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(1, 24))
            .await
            .unwrap();
        assert!(matches!(route, std::ops::ControlFlow::Break(None)));
        let other = Uri::from_file_path(fixture.root.join("other.tcl")).unwrap();
        open(&backend, &other, "proc helper {wrong} {}\n").await;
        let route = backend
            .original_source_view_hover_route(&fixture.main, Position::new(1, 24))
            .await
            .unwrap();
        assert!(matches!(route, std::ops::ControlFlow::Break(None)));
    }

    #[tokio::test]
    async fn closed_source_child_issues_inherited_view_from_published_physical_owner() {
        let (fixture, backend) = fixture().await;
        assert!(!backend.db_files.lock().await.contains_key(&fixture.child));
        assert!(
            !backend
                .documents
                .lock("unopened child")
                .await
                .contains_key(&fixture.child)
        );
        let view = child_view(&fixture, &backend).await;
        let inherited = view.draft.inherited.as_ref().unwrap();
        assert!(
            inherited
                .point
                .matches_parent_analysis(&inherited.parent.draft.producer.seed.analysis)
        );
        assert!(
            inherited
                .point
                .matches_child_analysis(&view.draft.producer.seed.analysis)
        );
        assert!(view.draft.matches_producer());
        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let file = files[&fixture.child];
        assert!(file == view.draft.physical.file);
        assert_eq!(file.text(&*db), CHILD);
        assert_eq!(
            file.path(&*db).as_deref(),
            view.draft.physical.admission.analyser_path()
        );
        let paths = backend.source_path_admissions.lock().unwrap();
        let published = &paths[&fixture.child];
        assert!(published.file == file);
        assert_eq!(
            published.admission.as_ref(),
            view.draft.physical.admission.as_ref()
        );
        assert!(view.draft.physical.publication.open_revision.is_none());
    }

    #[tokio::test]
    async fn changed_closed_source_child_withdraws_saved_interpretation() {
        let (fixture, backend) = fixture().await;
        let view = child_view(&fixture, &backend).await;
        std::fs::write(fixture.root.join("child.tcl"), "proc replacement {} {}\n").unwrap();
        assert!(
            backend
                .with_current_original_source_view(view.id, |_| Some(()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn deleted_closed_source_child_withdraws_saved_interpretation() {
        let (fixture, backend) = fixture().await;
        let view = child_view(&fixture, &backend).await;
        std::fs::remove_file(fixture.root.join("child.tcl")).unwrap();
        assert!(
            backend
                .with_current_original_source_view(view.id, |_| Some(()))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn refused_closed_source_child_path_creates_no_publication() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///source-%FF.tcl".parse().unwrap();
        assert!(matches!(
            backend.source_child_document_snapshot(&uri).await,
            Err(crate::DocumentSourceUnavailable::Path { .. })
        ));
        assert!(!backend.db_files.lock().await.contains_key(&uri));
        assert!(
            !backend
                .source_path_admissions
                .lock()
                .unwrap()
                .contains_key(&uri)
        );
        assert!(
            !backend
                .workspace_index
                .read()
                .await
                .contains_document(uri.as_str())
        );
    }

    #[tokio::test]
    async fn source_child_ingress_preserves_live_unsaved_document() {
        let (fixture, backend) = fixture().await;
        let unsaved = "proc helper {value} {return UNSAVED}\n";
        open(&backend, &fixture.child, unsaved).await;
        let RehomingDocumentSnapshot::Ready(snapshot) = backend
            .source_child_document_snapshot(&fixture.child)
            .await
            .unwrap()
        else {
            panic!("expected live child")
        };
        assert_eq!(snapshot.text.as_ref(), unsaved);
        let view = child_view(&fixture, &backend).await;
        assert_eq!(view.draft.physical.source.as_ref(), unsaved);
        assert!(view.draft.physical.publication.open_revision.is_some());
    }

    const AUTO_MAIN: &str = "source lib/helpers.tcl\n\nproc run {} {\n    puts [greet_helper world]\n    puts [mypkg_autoloaded]\n}\n";
    const AUTO_HELPER: &str = "proc greet_helper {who} {\n    return \"hi $who\"\n}\n";
    const AUTO_IMPL: &str = "proc mypkg_autoloaded {} {\n    return 42\n}\n";
    const AUTO_INDEX: &str = "# Tcl autoload index file, version 2.0\nset auto_index(mypkg_autoloaded) [list source [file join $dir impl.tcl]]\n";
    const AUTO_URI: &str = "file:///ws/main.tcl";
    const AUTO_PROVIDER: &str = "file:///ws/tmp/mypkg/impl.tcl";

    fn conditional_autoload_protocol(
        source: &str,
    ) -> (SourceProtocol, Arc<crate::vfs::MemoryStore>) {
        let store = Arc::new(crate::vfs::MemoryStore::new());
        for (uri, path, text) in [
            (AUTO_URI, "/ws/main.tcl", source),
            (
                "file:///ws/lib/helpers.tcl",
                "/ws/lib/helpers.tcl",
                AUTO_HELPER,
            ),
            (
                "file:///ws/tmp/mypkg/tclIndex",
                "/ws/tmp/mypkg/tclIndex",
                AUTO_INDEX,
            ),
            (AUTO_PROVIDER, "/ws/tmp/mypkg/impl.tcl", AUTO_IMPL),
        ] {
            store.upsert_uri(uri, path, text.as_bytes().to_vec());
        }
        assert_eq!(store.len(), 4);
        let backend_store: Arc<dyn crate::vfs::SourceStore> = store.clone();
        let (service, socket) =
            LspService::new(move |client| Backend::with_store(client, backend_store));
        let backend = service.inner().clone();
        let (mut outgoing, mut responses) = socket.split();
        let responder = tokio::spawn(async move {
            while let Some(request) = outgoing.next().await {
                let (method, id, params) = request.into_parts();
                if let Some(id) = id {
                    let result = if method == "workspace/configuration" {
                        serde_json::Value::Array(
                            params.unwrap()["items"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|_| serde_json::json!({}))
                                .collect(),
                        )
                    } else {
                        serde_json::Value::Null
                    };
                    responses.send(Response::from_ok(id, result)).await.unwrap();
                }
            }
        });
        let service =
            service.map_request(crate::service::normalise_request_uris as fn(Request) -> Request);
        (
            SourceProtocol {
                service: crate::transport_liveness::DeferredConcurrency::new(
                    service,
                    crate::transport_liveness::DEFAULT_HANDLER_CONCURRENCY,
                ),
                backend,
                responder,
                startup: Vec::new(),
                serial: 0,
            },
            store,
        )
    }

    async fn start_conditional_autoload(protocol: &mut SourceProtocol, source: &str) {
        let _=protocol.request("initialize",serde_json::json!({
            "processId":null,"rootUri":"file:///ws","workspaceFolders":[{"uri":"file:///ws","name":"ws"}],
            "capabilities":{"general":{"positionEncodings":["utf-16"]},"workspace":{"configuration":true,"symbol":{}},"textDocument":{"definition":{"linkSupport":false},"publishDiagnostics":{}}}
        })).await;
        protocol.notify("initialized", serde_json::json!({})).await;
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_secs(30),
                protocol.backend.workspace_scan_ready.wait()
            )
            .await
            .unwrap()
        );
        protocol.notify("textDocument/didOpen",serde_json::json!({"textDocument":{"uri":AUTO_URI,"languageId":"tcl","version":1,"text":source}})).await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(
            !protocol
                .backend
                .workspace_index
                .read()
                .await
                .contains_document(AUTO_PROVIDER)
        );
    }

    async fn conditional_autoload_definition(
        protocol: &mut SourceProtocol,
        source: &str,
    ) -> serde_json::Value {
        let offset = source.rfind("mypkg_autoloaded").unwrap();
        let position = tcl_lexer::LineIndex::new_lsp(source)
            .position_at_utf16(u32::try_from(offset + 4).unwrap(), source);
        protocol.request("textDocument/definition",serde_json::json!({"textDocument":{"uri":AUTO_URI},"position":{"line":position.line,"character":position.character.get()}})).await
    }

    fn has_conditional_autoload_provider(result: &serde_json::Value) -> bool {
        result.as_array().is_some_and(|rows| {
            rows.iter().any(|row| {
                row["uri"] == AUTO_PROVIDER
                    && row["range"]["start"]["line"] == 0
                    && row["range"]["start"]["character"] == 5
            })
        })
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn conditional_external_source_autoload_preserves_first_browser_sequence() {
        let (mut protocol, _store) = conditional_autoload_protocol(AUTO_MAIN);
        start_conditional_autoload(&mut protocol, AUTO_MAIN).await;
        let helper=protocol.request("textDocument/definition",serde_json::json!({"textDocument":{"uri":AUTO_URI},"position":{"line":3,"character":14}})).await;
        let _symbols = protocol
            .request(
                "workspace/symbol",
                serde_json::json!({"query":"greet_helper"}),
            )
            .await;
        let first = conditional_autoload_definition(&mut protocol, AUTO_MAIN).await;
        assert_eq!(protocol.serial, 4);
        protocol.finish().await;
        assert!(
            helper.as_array().is_some_and(|rows| rows
                .iter()
                .any(|row| row["uri"] == "file:///ws/lib/helpers.tcl")),
            "{helper}"
        );
        assert!(
            has_conditional_autoload_provider(&first),
            "first id4 external SOURCE candidate: {first}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn conditional_external_source_autoload_first_query_keeps_provider_currency() {
        let (mut protocol, store) = conditional_autoload_protocol(AUTO_MAIN);
        start_conditional_autoload(&mut protocol, AUTO_MAIN).await;
        let first = conditional_autoload_definition(&mut protocol, AUTO_MAIN).await;
        let unchanged = conditional_autoload_definition(&mut protocol, AUTO_MAIN).await;
        store.upsert_uri(
            AUTO_PROVIDER,
            "/ws/tmp/mypkg/impl.tcl",
            b"proc replaced_header {} {}\n".to_vec(),
        );
        let changed = conditional_autoload_definition(&mut protocol, AUTO_MAIN).await;
        protocol.finish().await;
        assert!(has_conditional_autoload_provider(&first), "{first}");
        assert!(
            has_conditional_autoload_provider(&unchanged),
            "unchanged repeat is the currency prerequisite: {unchanged}"
        );
        assert!(
            !has_conditional_autoload_provider(&changed),
            "changed provider: {changed}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn conditional_external_source_autoload_keeps_shadow_rename_unknown_and_dynamic_barriers()
    {
        for body in [
            "proc mypkg_autoloaded {} {}; puts [mypkg_autoloaded]",
            "proc held {} {}; rename held mypkg_autoloaded; puts [mypkg_autoloaded]",
            "rename mypkg_autoloaded removed; puts [mypkg_autoloaded]",
            "mystery; puts [mypkg_autoloaded]",
            "puts [mystery]; puts [mypkg_autoloaded]",
            "puts [mypkg_autoloaded $arg]",
        ] {
            let source = format!("source lib/helpers.tcl\nproc run {{arg}} {{{body}}}\n");
            let (mut protocol, _store) = conditional_autoload_protocol(&source);
            start_conditional_autoload(&mut protocol, &source).await;
            let result = conditional_autoload_definition(&mut protocol, &source).await;
            protocol.finish().await;
            assert!(
                !has_conditional_autoload_provider(&result),
                "{source}: {result}"
            );
        }
    }
    #[tokio::test]
    async fn conditional_external_source_autoload_preserves_original_saved_byte_receipts() {
        let (protocol, store) = conditional_autoload_protocol(AUTO_MAIN);
        let providers = protocol
            .backend
            .capture_conditional_autoload_providers(vec![std::path::PathBuf::from(
                "/ws/tmp/mypkg/impl.tcl",
            )])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(providers.len(), 1);
        let provider = &providers[0];
        let publications = [&provider.source.publication];
        let admissions = [provider.source.admission.as_ref()];
        let saved = Backend::conditional_autoload_saved_sources(&publications, &admissions)
            .expect("original closed-file byte receipt");
        assert_eq!(saved[0].1.as_ref(), AUTO_IMPL.as_bytes());
        let selected = saved
            .iter()
            .map(|(admission, bytes)| (admission.as_ref(), bytes.as_ref()))
            .collect::<Vec<_>>();
        assert!(
            super::super::super::DocumentPathAdmission::with_current_saved_sources(
                &selected,
                store.as_ref(),
                || true
            )
            .unwrap()
        );
        let changed = AUTO_IMPL.replace('\n', "\r");
        assert_eq!(tcl_lexer::normalise_lone_cr(&changed).as_ref(), AUTO_IMPL);
        store.upsert_uri(
            AUTO_PROVIDER,
            "/ws/tmp/mypkg/impl.tcl",
            changed.into_bytes(),
        );
        // The actual original publication keeps its original bytes, even when
        // a newer read would normalise to precisely the same analysis source.
        let retained =
            Backend::conditional_autoload_saved_sources(&publications, &admissions).unwrap();
        assert_eq!(retained[0].1.as_ref(), AUTO_IMPL.as_bytes());
        assert!(
            super::super::super::DocumentPathAdmission::with_current_saved_sources(
                &selected,
                store.as_ref(),
                || true
            )
            .is_err()
        );
        protocol.responder.abort();
    }

    #[tokio::test]
    async fn conditional_external_source_autoload_does_not_cross_donate_indexed_provider_headers() {
        let (protocol, store) = conditional_autoload_protocol(AUTO_MAIN);
        store.upsert_uri(
            "file:///ws/tmp/mypkg/other.tcl",
            "/ws/tmp/mypkg/other.tcl",
            b"proc unrelated_header {} {}\n".to_vec(),
        );
        let providers = protocol
            .backend
            .capture_conditional_autoload_providers(vec![
                std::path::PathBuf::from("/ws/tmp/mypkg/other.tcl"),
                std::path::PathBuf::from("/ws/tmp/mypkg/impl.tcl"),
            ])
            .await
            .unwrap()
            .unwrap();
        let first_files = ["file:///ws/tmp/mypkg/other.tcl".parse::<Uri>().unwrap()];
        let second_files = [AUTO_PROVIDER.parse::<Uri>().unwrap()];
        let first = Backend::conditional_autoload_provider_headers(&providers, &first_files);
        let second = Backend::conditional_autoload_provider_headers(&providers, &second_files);
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        let caller_source = "proc run {} {puts [mypkg_autoloaded]}";
        let caller = tcl_compiler::analyser::Analyser::new().analyse(caller_source, "tcl8.6");
        let publication =
            tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(91_u8);
        let documents = [
            tcl_lsp_core::original_source_views::OriginalSourceViewDocument {
                publication: &publication,
                document: tcl_lsp_core::original_declaration::OriginalDeclarationDocument {
                    uri: AUTO_URI,
                    source: caller_source,
                    analysis: &caller,
                },
                region: None,
                ancestors: &[],
            },
        ];
        let at = u32::try_from(caller_source.find("mypkg_autoloaded").unwrap() + 4).unwrap();
        let requests = tcl_lsp_core::original_source_views::conditional_external_autoload_requests(
            AUTO_URI, at, &documents,
        )
        .expect("genuine conditional body request");
        assert_eq!(requests.len(), 1);
        assert!(
            tcl_lsp_core::original_source_views::conditional_external_autoload_header(
                &requests[0],
                &documents,
                &second
            )
            .is_some(),
            "the other indexed provider genuinely has the target header"
        );
        assert!(
            tcl_lsp_core::original_source_views::conditional_external_autoload_header(
                &requests[0],
                &documents,
                &first
            )
            .is_none(),
            "this request's own indexed file cannot borrow that header"
        );
        assert_eq!(first[0].uri, first_files[0].as_str());
        assert_eq!(second[0].uri, second_files[0].as_str());
        assert!(
            !first[0]
                .analysis
                .original_procedure_declarations()
                .any(|row| &first[0].source[row.name_input().span().as_range()]
                    == "mypkg_autoloaded")
        );
        assert!(
            second[0]
                .analysis
                .original_procedure_declarations()
                .any(|row| &second[0].source[row.name_input().span().as_range()]
                    == "mypkg_autoloaded")
        );
        protocol.responder.abort();
    }
}
