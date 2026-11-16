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

//! A settled scope: its projections for definitions and references, and the views selected from it while the physical leases hold.

use super::super::{Arc, Backend, DocumentSourceResult};
use super::{
    ConditionalAutoloadProvider, OriginalSourceScopeProjection, OriginalSourceViewId,
    OriginalSourceViewSettlement, PublishedOriginalSourceView,
};
use std::collections::HashSet;
use tower_lsp_server::ls_types::Uri;

impl OriginalSourceViewSettlement {
    pub(super) fn requires_recapture(&self, uri: &Uri) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.recapture.is_some() && scope.required.contains(uri))
    }

    /// Documents a settled scope interprets as a sourced child. Their callers
    /// belong to that scope's views, never to the ordinary route.
    pub(super) fn settled_sourced_documents(&self) -> HashSet<Uri> {
        self.scopes
            .iter()
            .filter(|scope| scope.complete && !scope.conditional_obligations)
            .flat_map(|scope| {
                scope
                    .required
                    .iter()
                    .filter(move |uri| **uri != scope.root_uri)
            })
            .cloned()
            .collect()
    }

    pub(super) fn aggregate_complete(&self) -> bool {
        self.scopes
            .iter()
            .all(|scope| scope.complete && !scope.conditional_obligations)
    }

    pub(super) fn ordinary_definition_scope_is_complete(
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

    pub(super) fn navigation_projection(
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

    pub(super) fn definition_projection(&self, uri: &Uri) -> Option<OriginalSourceScopeProjection> {
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
    pub(super) fn positive_reference_projection(
        &self,
        uri: &Uri,
    ) -> Option<OriginalSourceScopeProjection> {
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

    pub(super) fn projection(
        &self,
        uri: &Uri,
        aggregate: bool,
    ) -> Option<OriginalSourceScopeProjection> {
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
    pub(super) async fn with_original_source_scope<T>(
        &self,
        ids: &[OriginalSourceViewId],
        render: impl FnMut(&[Arc<PublishedOriginalSourceView>]) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.with_selected_original_source_views(ids, None, render)
            .await
    }

    pub(super) async fn with_settled_original_source_scope<T>(
        &self,
        projection: &OriginalSourceScopeProjection,
        render: impl FnMut(&[Arc<PublishedOriginalSourceView>]) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.with_selected_original_source_views(&projection.ids, Some(projection.revision), render)
            .await
    }

    pub(super) async fn with_selected_original_source_views<T>(
        &self,
        ids: &[OriginalSourceViewId],
        revision: Option<u64>,
        render: impl FnMut(&[Arc<PublishedOriginalSourceView>]) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.with_selected_original_source_views_and_provider_headers(ids, revision, &[], render)
            .await
    }

    pub(super) async fn with_selected_original_source_views_and_provider_headers<T>(
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

    pub(super) fn original_source_view_inventory<'a>(
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

    pub(super) fn original_source_view_labels(
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
}

impl Backend {
    /// Obtain ownership for pure worker computation. This returns no durable
    /// currentness: every result must rejoin this exact inventory at rendering.
    pub(super) async fn compute_original_source_views<T: Send + Unpin + 'static>(
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
        let result = super::super::super::rt::spawn_blocking(move || calculate(&worker_owners))
            .await
            .map_err(|_| super::super::super::DocumentSourceUnavailable::Worker { uri })?;
        Ok(result.map(|result| (owners, result)))
    }

    pub(super) fn same_original_source_view_inventory(
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
