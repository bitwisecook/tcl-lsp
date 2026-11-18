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

//! Publishing a batch of views inside the index transaction, and rejoining a saved view to its live table row.

use super::super::{Arc, Backend, DocumentSourceResult, ReadonlySourceDocument};
use super::{
    OriginalSourceViewAttempt, OriginalSourceViewDraft, OriginalSourceViewId,
    OriginalSourceViewTable, PublishedOriginalSourceView, SourceViewPublicationMode,
};
use tower_lsp_server::ls_types::Uri;

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

    pub(super) async fn append_inherited_original_source_view(
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

    pub(super) async fn append_original_source_regions(
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
    ) -> Vec<&super::super::super::LibraryPublicationSource> {
        sources
            .iter()
            .map(|source| &source.publication)
            .collect::<Vec<_>>()
    }

    fn original_source_view_admissions(
        sources: &[Arc<ReadonlySourceDocument>],
    ) -> Vec<&super::super::super::DocumentPathAdmission> {
        sources
            .iter()
            .map(|source| source.admission.as_ref())
            .collect::<Vec<_>>()
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
                    && (draft.ancestors().is_none() || draft.call_chain_executes(uri)))
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
            super::super::ReadonlyTransaction::Published(outcome) => outcome,
            super::super::ReadonlyTransaction::CurrencyRefused
            | super::super::ReadonlyTransaction::PublishRefused => {
                OriginalSourceViewAttempt::RetryCapture
            }
        })
    }
}

impl Backend {
    /// A saved locator rejoins its own still-live table row before any metadata
    /// lookup. It never analyses a URI/text pair to reconstruct a missing view.
    pub(super) async fn with_current_original_source_view<T>(
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
