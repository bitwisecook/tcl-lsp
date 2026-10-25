// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retained source interpretations and their independent physical publications.
//!
//! Source-only interpretations retain physical leases independently of loading,
//! whole-program completeness, Native execution and edit authority.
//! A view key is a locator; only a live table join under the source lease answers.

use super::{
    Arc, Backend, DocumentSourceResult, ReadonlySourceDocument, ReadonlySourceState,
    WorkspaceSymbolFreshInput, original_readonly_analysis_matches_input,
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
            && super::with_pack_hooks(|| {
                tcl_lsp_db::document_analysis_input(state.db, physical.file, physical.config)
            })
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
    ) -> DocumentSourceResult<Option<Vec<Arc<PublishedOriginalSourceView>>>> {
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
    ) -> DocumentSourceResult<Option<Arc<PublishedOriginalSourceView>>> {
        Ok(self
            .publish_original_source_view_batch(
                uri,
                vec![draft],
                SourceViewPublicationMode::AppendInherited,
            )
            .await?
            .and_then(|mut views| (views.len() == 1).then(|| views.pop()).flatten()))
    }

    async fn append_original_source_regions(
        &self,
        uri: &Uri,
        drafts: Vec<OriginalSourceViewDraft>,
    ) -> DocumentSourceResult<Option<Vec<Arc<PublishedOriginalSourceView>>>> {
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

    async fn publish_original_source_view_batch(
        &self,
        uri: &Uri,
        drafts: Vec<OriginalSourceViewDraft>,
        mode: SourceViewPublicationMode,
    ) -> DocumentSourceResult<Option<Vec<Arc<PublishedOriginalSourceView>>>> {
        let Some(first) = drafts.first() else {
            return Ok(None);
        };
        let epoch = first.producer.seed.analyser_inputs_epoch;
        let factory = first.producer.seed.class_factory_generation;
        let Some(ancestors) =
            Self::original_source_view_batch_ancestors(uri, &drafts, mode, epoch, factory)
        else {
            return Ok(None);
        };
        let sources = Self::original_source_view_batch_sources(&drafts, &ancestors);
        let publications = Self::original_source_view_publications(&sources);
        let Some(saved) = self.readonly_saved_sources(&publications).await? else {
            return Ok(None);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        let admissions = Self::original_source_view_admissions(&sources);
        let validation_drafts = drafts.clone();
        let mut drafts = Some(drafts);
        self.with_current_readonly_source_state(
            epoch,
            factory,
            &admissions,
            Some(&saved),
            |state| {
                (validation_drafts.iter().all(|draft| {
                    draft.matches_physical_state(state, epoch, factory)
                }) && ancestors.iter().flatten().all(|view| {
                    view.draft.matches_physical_state(state, epoch, factory)
                })).then_some(())
            },
            |_state, ()| {
                let _rehoming = self.rehoming_gate.try_lock().ok()?;
                let mut table = self.original_source_views.try_lock().ok()?;
                let mut index = self.workspace_index.try_write().ok()?;
                let drafts = drafts.take()?;
                if ancestors.iter().flatten().any(|view| {
                        !table.contains_actual(view)
                            || !view.is_indexed(&index)
                    })
                {
                    return None;
                }
                if mode == SourceViewPublicationMode::ReplaceFirstEdge
                    && validation_drafts.iter().all(|draft| !draft.is_interpreted())
                    && table.by_uri.get(uri).into_iter().flatten().any(|id| {
                        table.by_id.get(id).is_some_and(|view| view.draft.is_interpreted())
                    })
                {
                    return None;
                }
                if mode == SourceViewPublicationMode::ReplaceFirstEdge
                    && let Some(existing) = table.reuse_current_batch(uri, &drafts, &index)
                {
                    return Some(existing);
                }
                let (next_id, staged) = table.stage(drafts)?;
                if mode != SourceViewPublicationMode::ReplaceFirstEdge
                    && (staged.iter().any(|(id, _)| table.by_id.contains_key(id))
                        || (mode == SourceViewPublicationMode::AppendRegions
                            && table.by_uri.get(uri).is_none_or(Vec::is_empty))) {
                    return None;
                }
                let indexed = staged.iter().map(|(id, draft)| {
                    let publication = tcl_lsp_core::original_declaration::OriginalSourceViewPublication::retain(*id);
                    let source = Arc::clone(&draft.physical.source);
                    let analysis = Arc::clone(&draft.producer.seed.analysis);
                    match &draft.region {
                        None => tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView::capture(publication, source, analysis),
                        Some(owned) => tcl_lsp_core::workspace_index::WorkspaceOriginalSourceView::capture_region(publication, source, analysis, owned.region.clone()),
                    }
                }).collect::<Option<Vec<_>>>()?;
                match mode {
                    SourceViewPublicationMode::ReplaceFirstEdge => index.replace_original_source_views(uri.as_str(), indexed)?,
                    SourceViewPublicationMode::AppendInherited => index.append_original_source_interpretations(uri.as_str(), indexed)?,
                    SourceViewPublicationMode::AppendRegions => index.append_original_source_regions(uri.as_str(), indexed)?,
                };
                let views = staged
                    .into_iter()
                    .map(|(id, draft)| {
                        Arc::new(PublishedOriginalSourceView {
                            id,
                            draft,
                        })
                    })
                    .collect::<Vec<_>>();
                match mode {
                    SourceViewPublicationMode::ReplaceFirstEdge => table.replace(uri, next_id, &views),
                    SourceViewPublicationMode::AppendInherited | SourceViewPublicationMode::AppendRegions => table.append_regions(uri, next_id, &views),
                }
                Some(views)
            },
        )
        .await
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
    ) -> DocumentSourceResult<Option<OriginalSourceViewDraft>> {
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
            return Ok(None);
        }
        let Some(physical) = self
            .capture_readonly_source_document(uri.as_str(), source)
            .await?
        else {
            return Ok(None);
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
            return Ok(None);
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
            return Ok(None);
        };
        let Some(interpretation_input) = seed.analysis.resolved_input.clone().map(Arc::new) else {
            return Ok(None);
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
        Ok(draft.matches_producer().then_some(draft))
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
    ) -> DocumentSourceResult<Option<OriginalSourceViewDraft>> {
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
            return Ok(None);
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
            return Ok(None);
        };
        let Some(input) = seed.analysis.resolved_input.clone().map(Arc::new) else {
            return Ok(None);
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
        Ok(draft.matches_producer().then_some(draft))
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

    /// Ask the existing kernel for the exact ordered suffix chain. There is no
    /// fresh whole-parent analysis or URI/text reconstruction on this route.
    async fn materialize_conditional_source_regions(
        &self,
        parent: Arc<PublishedOriginalSourceView>,
        child_prefix: Arc<PublishedOriginalSourceView>,
        load: Arc<tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad>,
        completed_child: Option<&OriginalSourceViewCompletion>,
    ) -> DocumentSourceResult<Option<OriginalSourceRegionWork>> {
        let outcome = if let Some(completed) = completed_child {
            if completed.prefix.id != child_prefix.id
                || !completed
                    .prefix
                    .draft
                    .same_live_producer(&child_prefix.draft)
                || !completed
                    .outcome
                    .matches_prefix_analysis(&child_prefix.draft.producer.seed.analysis)
            {
                return Ok(None);
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
            return Ok(None);
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
                let Some(draft) = self.capture_conditional_region_draft(request).await? else {
                    return Ok(None);
                };
                pending.push((views.len(), draft));
                views.push(None);
            }
        }
        if !pending.is_empty() {
            let (positions, drafts): (Vec<_>, Vec<_>) = pending.into_iter().unzip();
            let Some(published) = self
                .append_original_source_regions(&parent.draft.physical.uri, drafts)
                .await?
            else {
                return Ok(None);
            };
            if published.len() != positions.len() {
                return Ok(None);
            }
            for (position, view) in positions.into_iter().zip(published) {
                views[position] = Some(view);
            }
        }
        let Some(views) = views.into_iter().collect::<Option<Vec<_>>>() else {
            return Ok(None);
        };
        Ok(Some(OriginalSourceRegionWork {
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
            root: Some(root),
            root_uri,
            required,
            association_owners: self.association_owners,
            ids: self.membership.into_iter().collect(),
            complete,
            completion,
            eligibility: HashSet::new(),
            conditional_obligations: self.conditional_obligations,
        }
    }
}

enum OriginalSourceViewFrontierStep {
    NoCut,
    Pending,
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
struct OriginalSourceRootScope {
    root: Option<OriginalSourceViewId>,
    root_uri: Uri,
    required: HashSet<Uri>,
    association_owners: Vec<OriginalSourceViewAssociation>,
    ids: Vec<OriginalSourceViewId>,
    complete: bool,
    completion: Option<OriginalSourceViewCompletion>,
    eligibility: HashSet<Uri>,
    conditional_obligations: bool,
}

impl OriginalSourceRootScope {
    fn pending(uri: Uri) -> Self {
        Self {
            root: None,
            root_uri: uri.clone(),
            required: HashSet::from([uri]),
            association_owners: Vec::new(),
            ids: Vec::new(),
            complete: false,
            completion: None,
            eligibility: HashSet::new(),
            conditional_obligations: false,
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
}

impl OriginalSourceViewSettlement {
    fn host_navigation_scope_detail(&self, uri: &Uri) -> String {
        let mut scopes = self
            .scopes
            .iter()
            .map(|scope| {
                format!(
                    "root={} complete={} conditional={} selected_required={} owners={}",
                    scope.root_uri.as_str(),
                    scope.complete,
                    scope.conditional_obligations,
                    scope.required.contains(uri),
                    scope.ids.len(),
                )
            })
            .collect::<Vec<_>>();
        scopes.sort();
        scopes.join("; ")
    }

    fn aggregate_complete(&self) -> bool {
        self.scopes
            .iter()
            .all(|scope| scope.complete && !scope.conditional_obligations)
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
        let _build = self.original_source_view_build_gate.lock().await;
        if !self
            .prune_retired_original_source_view_presentations()
            .await
        {
            self.trace_host_navigation_stage("graph.prune-refused")
                .await;
            return Ok(None);
        }
        let Some(work) = self.original_source_root_work().await? else {
            self.trace_host_navigation_stage("graph.root-work-unavailable")
                .await;
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
            reconciliation
                .scopes
                .push(self.run_original_source_view_work(root).await?);
        }
        if !self
            .reconcile_excluded_original_roots(&mut reconciliation)
            .await?
        {
            self.trace_host_navigation_stage("graph.excluded-reconciliation-refused")
                .await;
            return Ok(None);
        }
        let result = self
            .finish_original_root_reconciliation(reconciliation)
            .await?;
        if result.is_none() {
            self.trace_host_navigation_stage("graph.final-reconciliation-refused")
                .await;
        }
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
            let Some(views) = self
                .publish_original_source_views(&uri, vec![candidate.draft])
                .await?
            else {
                reconciliation
                    .scopes
                    .push(OriginalSourceRootScope::pending(uri));
                continue;
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
                    let Some(mut views) = self
                        .publish_original_source_views(&uri, vec![candidate.draft])
                        .await?
                    else {
                        reconciliation
                            .scopes
                            .push(OriginalSourceRootScope::pending(uri));
                        continue;
                    };
                    if views.len() != 1 {
                        return Ok(false);
                    }
                    views.remove(0)
                };
                let mut scope = self.run_original_source_view_work(view).await?;
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
                if scope.complete && !scope.eligibility.is_subset(&completed) {
                    scope.complete = false;
                    changed = true;
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
                        .is_some_and(|view| view.draft.is_interpreted())
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
                                .is_some_and(|view| view.draft.is_interpreted())
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
    ) -> DocumentSourceResult<Option<OriginalSourceViewEdges>> {
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
            return Ok(None);
        };
        self.with_current_original_source_view(view.id, |current| {
            current
                .draft
                .same_live_producer(&view.draft)
                .then(|| edges.clone())
        })
        .await
    }

    /// A genuine current association removes competing independent authoring
    /// roots, never sibling inherited interpretations. Retirement is atomic
    /// with compatibility-row withdrawal under the publication transaction.
    async fn retire_associated_original_source_roots(
        &self,
        parent: &PublishedOriginalSourceView,
        target: &Uri,
    ) -> DocumentSourceResult<bool> {
        let draft = &parent.draft;
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
        let result = self
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
        Ok(result.is_some())
    }

    async fn source_view_child(
        &self,
        parent: &Arc<PublishedOriginalSourceView>,
        edge: &tcl_lsp_core::workspace_index::WorkspaceSource,
    ) -> DocumentSourceResult<Option<Arc<PublishedOriginalSourceView>>> {
        let Some(uri) = Self::source_view_target(parent, edge) else {
            return Ok(None);
        };
        let Some(
            tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceFrontier::CurrentCut(cut),
        ) = edge.original_source_frontier.as_deref() else {
            return Ok(None);
        };
        if !cut.matches_parent_analysis(&parent.draft.producer.seed.analysis)
            || edge.original_project_source_load.as_deref() != Some(cut.source_load())
        {
            return Ok(None);
        }
        if let Some(cached) = self
            .cached_original_source_view(&uri, Some((parent, cut)))
            .await?
        {
            return Ok(Some(cached));
        }
        let snapshot = self.rehoming_document_snapshot(&uri).await?;
        let super::super::RehomingDocumentSnapshot::Ready(document) = snapshot else {
            return Ok(None);
        };
        let Some(draft) = self
            .capture_inherited_original_source_view(
                &uri,
                document.text,
                Arc::clone(parent),
                Arc::new(cut.clone()),
            )
            .await?
        else {
            return Ok(None);
        };
        self.append_inherited_original_source_view(&uri, draft)
            .await
    }

    async fn original_source_view_frontier_step(
        &self,
        parent: &Arc<PublishedOriginalSourceView>,
        state: &mut OriginalSourceViewWorkState,
    ) -> DocumentSourceResult<OriginalSourceViewFrontierStep> {
        let Some(batch) = self
            .original_source_view_edges(Arc::clone(parent), &state.unknown)
            .await?
        else {
            return Ok(OriginalSourceViewFrontierStep::Pending);
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
                if !self
                    .retire_associated_original_source_roots(parent, &target)
                    .await?
                {
                    return Ok(OriginalSourceViewFrontierStep::Pending);
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
    ) -> DocumentSourceResult<bool> {
        if state.finished.contains_key(&parent.id) {
            return Ok(true);
        }
        if !state.active.insert(parent.id) {
            return Ok(false);
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
                return Ok(true);
            }
            OriginalSourceViewFrontierStep::Pending => return Ok(false),
            OriginalSourceViewFrontierStep::Cut(edge) => edge,
        };
        let Some(child) = self.source_view_child(&parent, &edge).await? else {
            return Ok(false);
        };
        let Some(SourceFrontier::CurrentCut(cut)) = edge.original_source_frontier.as_deref() else {
            return Ok(false);
        };
        let load = Arc::new(cut.source_load().clone());
        state.tasks.push(SourceViewWork::AfterChild {
            parent,
            child: Arc::clone(&child),
            load,
        });
        state.tasks.push(SourceViewWork::Enter(child));
        Ok(true)
    }

    /// Every issued region must keep its source-frontier debts even when the
    /// kernel already owns a conditional normal route. Nonfinal frame regions
    /// cannot hide a cut or be reordered behind a later outer suffix.
    async fn reconcile_issued_region_frontiers(
        &self,
        work: &OriginalSourceRegionWork,
        state: &mut OriginalSourceViewWorkState,
    ) -> DocumentSourceResult<bool> {
        for (index, region) in work.views.iter().enumerate() {
            let is_unfinished_tail = !work.chain.complete() && index + 1 == work.views.len();
            if is_unfinished_tail {
                continue;
            }
            if !matches!(
                self.original_source_view_frontier_step(region, state)
                    .await?,
                OriginalSourceViewFrontierStep::NoCut
            ) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn run_original_source_view_work(
        &self,
        root: Arc<PublishedOriginalSourceView>,
    ) -> DocumentSourceResult<OriginalSourceRootScope> {
        let root_id = root.id;
        let root_uri = root.draft.physical.uri.clone();
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
                    if !self.visit_original_source_view(parent, &mut state).await? {
                        return Ok(state.finish_scope(root_id, root_uri, false));
                    }
                }
                SourceViewWork::AfterChild {
                    parent,
                    child,
                    load,
                } => {
                    let Some(completed_child) = state.finished.get(&child.id) else {
                        return Ok(state.finish_scope(root_id, root_uri, false));
                    };
                    let Some(work) = self
                        .materialize_conditional_source_regions(
                            Arc::clone(&parent),
                            child,
                            load,
                            completed_child.as_ref(),
                        )
                        .await?
                    else {
                        return Ok(state.finish_scope(root_id, root_uri, false));
                    };
                    state
                        .membership
                        .extend(work.views.iter().map(|view| view.id));
                    if !self
                        .reconcile_issued_region_frontiers(&work, &mut state)
                        .await?
                    {
                        return Ok(state.finish_scope(root_id, root_uri, false));
                    }
                    if let Some(completion) = work.completion() {
                        state.active.remove(&parent.id);
                        state.finished.insert(parent.id, Some(completion));
                    } else {
                        // An incomplete ordered kernel chain ends at its actual
                        // last issued frame region. It cannot skip to an outer
                        // suffix or manufacture whole-prefix completion.
                        let Some(last) = work.views.last().cloned() else {
                            return Ok(state.finish_scope(root_id, root_uri, false));
                        };
                        state.tasks.push(SourceViewWork::AfterRegion {
                            work,
                            region: Arc::clone(&last),
                        });
                        state.tasks.push(SourceViewWork::Enter(last));
                    }
                }
                SourceViewWork::AfterRegion { work, region } => {
                    let Some(Some(completed)) = state.finished.get(&region.id) else {
                        return Ok(state.finish_scope(root_id, root_uri, false));
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
        Ok(state.finish_scope(root_id, root_uri, true))
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
        let Ok(table) = self.original_source_views.try_lock() else {
            return false;
        };
        let Ok(index) = self.workspace_index.try_read() else {
            return false;
        };
        table.current_revision() == Some(settlement.revision)
            && settlement.aggregate_complete() && settlement.scopes.iter().all(|scope| {
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
        let publications = views
            .iter()
            .map(|view| &view.draft.physical.publication)
            .collect::<Vec<_>>();
        let Some(saved) = self.readonly_saved_sources(&publications).await? else {
            return Ok(None);
        };
        let saved = saved
            .iter()
            .map(|source| (source.admission.as_ref(), source.bytes.as_slice()))
            .collect::<Vec<_>>();
        let admissions = views
            .iter()
            .map(|view| view.draft.physical.admission.as_ref())
            .collect::<Vec<_>>();
        self.with_current_readonly_source_state(epoch, factory, &admissions, Some(&saved),
            |state| views.iter().all(|view| view.draft.matches_physical_state(state, epoch, factory)).then_some(()),
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
    async fn trace_host_navigation_stage(&self, stage: &str) {
        if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
            self.client
                .log_message(
                    tower_lsp_server::ls_types::MessageType::LOG,
                    format!("HOST_NAV_STAGE uri=* stage={stage}"),
                )
                .await;
        }
    }

    pub(crate) async fn trace_host_navigation(&self, uri: &Uri, stage: &str, detail: &str) {
        if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
            self.client
                .log_message(
                    tower_lsp_server::ls_types::MessageType::LOG,
                    format!("HOST_NAV_STAGE uri={} stage={stage} {detail}", uri.as_str()),
                )
                .await;
        }
    }

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
        let Some(settlement) = self.refresh_original_source_view_graph().await? else {
            self.trace_host_navigation(uri, "graph.unavailable", "")
                .await;
            return Ok(std::ops::ControlFlow::Break(None));
        };
        let Some(ids) = settlement.projection(uri, references.is_some()) else {
            if matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
                let scopes = settlement.host_navigation_scope_detail(uri);
                self.trace_host_navigation(uri, "projection.unavailable", &scopes)
                    .await;
            }
            return Ok(std::ops::ControlFlow::Break(None));
        };
        let aggregate_complete = settlement.aggregate_complete();
        let trace_uri = uri.clone();
        let uri = uri.clone();
        let computed = self
            .compute_original_source_views(&ids, move |views| {
                Some(Self::calculate_original_source_navigation(
                    views,
                    &uri,
                    position,
                    references,
                    aggregate_complete,
                ))
            })
            .await?;
        let Some((
            owners,
            HostNavigationCalculation {
                output,
                stage,
                detail,
            },
        )) = computed
        else {
            self.trace_host_navigation(&trace_uri, "views.current-scope-unavailable", "")
                .await;
            return Ok(std::ops::ControlFlow::Break(None));
        };
        self.trace_host_navigation(&trace_uri, stage, &detail).await;
        let Some(output) = output else {
            return Ok(std::ops::ControlFlow::Break(None));
        };
        let mut output = Some(output);
        let result = self
            .with_settled_original_source_scope(&ids, |views| {
                Self::same_original_source_view_inventory(&owners, views).then_some(())?;
                output.take()
            })
            .await?;
        if result.is_none() {
            self.trace_host_navigation(&trace_uri, "views.final-current-scope-unavailable", "")
                .await;
        }
        Ok(result.unwrap_or(std::ops::ControlFlow::Break(None)))
    }
}

type HostNavigationRoute = std::ops::ControlFlow<Option<Vec<tower_lsp_server::ls_types::Location>>>;
struct HostNavigationCalculation {
    output: Option<HostNavigationRoute>,
    stage: &'static str,
    detail: String,
}

fn host_navigation_owner_detail(views: &[&Arc<PublishedOriginalSourceView>]) -> String {
    if !matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1")) {
        return String::new();
    }
    views
        .iter()
        .map(|view| {
            format!(
                "owner={:?} analysis={:p} interpreted={} region={} bytes={}",
                view.id,
                view.draft.producer.seed.analysis.as_ref(),
                view.draft.is_interpreted(),
                view.draft.region.is_some(),
                view.draft.physical.source.len(),
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

impl Backend {
    fn calculate_original_source_navigation(
        views: &[Arc<PublishedOriginalSourceView>],
        uri: &Uri,
        position: tower_lsp_server::ls_types::Position,
        references: Option<bool>,
        aggregate_complete: bool,
    ) -> HostNavigationCalculation {
        let selected_views = views
            .iter()
            .filter(|view| &view.draft.physical.uri == uri)
            .collect::<Vec<_>>();
        let mut detail = host_navigation_owner_detail(&selected_views);
        let mut stage = "views.labels";
        let output = (|| {
            if aggregate_complete
                && !selected_views
                    .iter()
                    .any(|view| view.draft.is_interpreted())
            {
                stage = "ordinary.continue";
                return Some(std::ops::ControlFlow::Continue(()));
            }
            let (publications, ancestors) = Self::original_source_view_labels(views)?;
            let inventory = Self::original_source_view_inventory(views, &publications, &ancestors);
            let mut result = Vec::new();
            stage = "views.selected-first";
            let first = selected_views.first()?;
            let source = &first.draft.physical.source;
            let offset = tcl_lexer::LineIndex::new(source).offset_at_utf16(
                position.line,
                tcl_lexer::Utf16Col::new(position.character),
                source,
            );
            stage = "core.cursor-publications";
            for publication in tcl_lsp_core::original_source_views::cursor_publications(
                uri.as_str(),
                offset,
                &inventory,
            )? {
                stage = "core.prepare";
                let selected =
                    tcl_lsp_core::original_source_views::prepare(&publication, offset, &inventory);
                if selected.is_none()
                    && matches!(option_env!("TCL_LSP_TRACE_COARSE_SOURCE"), Some("1"))
                    && let Some(refusal) =
                        tcl_lsp_core::original_source_views::take_prepare_refusal_trace()
                {
                    detail.push_str(" core_prepare_refusal=");
                    detail.push_str(&refusal);
                }
                let selected = selected?;
                if references != Some(false) {
                    stage = "core.target-owner";
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
                        stage = "core.reference-owner";
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
            stage = "core.complete";
            Some(std::ops::ControlFlow::Break(Some(unique)))
        })();
        HostNavigationCalculation {
            output,
            stage,
            detail,
        }
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
                "proc host_old {value} { return $value }\nputs \"🧪 café\"; puts [host_old 7]\n";
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
}
