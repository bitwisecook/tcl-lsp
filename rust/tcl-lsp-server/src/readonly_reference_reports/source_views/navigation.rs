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

//! View-aware navigation for inherited interpretations: definitions, references and the reference supplement.

use super::super::{Arc, Backend, DocumentSourceResult};
use super::{
    OriginalSourceViewId, PublishedOriginalSourceView, SOURCE_VIEW_CONTENTION_ATTEMPTS,
    SOURCE_VIEW_CONTENTION_BACKOFF,
};
use std::collections::HashSet;
use tower_lsp_server::ls_types::Uri;

impl Backend {
    pub(super) async fn await_original_source_navigation_readiness(
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

    /// Positive source-view references cannot survey an independent root while
    /// another root is incomplete. When the cursor's own scope is complete, the
    /// ordinary route answers the documents no settled scope sources, as it does
    /// for definition; those it does source stay with the views.
    pub(crate) async fn original_source_reference_supplement(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<Option<HashSet<Uri>>> {
        let Some(settlement) = self.refresh_original_source_view_graph_for_uri(uri).await? else {
            return Ok(None);
        };
        if settlement.aggregate_complete() {
            return Ok(None);
        }
        let Some(projection) = settlement.definition_projection(uri) else {
            return Ok(None);
        };
        if !settlement.ordinary_definition_scope_is_complete(uri, &projection) {
            return Ok(None);
        }
        let sourced = settlement.settled_sourced_documents();
        Ok((!sourced.contains(uri)).then_some(sourced))
    }

    /// One navigation attempt; `None` when the settled scope could not be read
    /// consistently and the attempt may be repeated.
    pub(super) async fn original_source_view_navigation_attempt(
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
        let Some(output) = ordinary else {
            let Some(requests) = conditional else {
                return Ok(Some(std::ops::ControlFlow::Break(None)));
            };
            let Some(locations) = self
                .conditional_source_view_autoload_locations(&ids, &owners, &requests)
                .await?
            else {
                return Ok(Some(std::ops::ControlFlow::Break(None)));
            };
            // The provider and view leases have already been joined together.
            return Ok(Some(std::ops::ControlFlow::Break(Some(locations))));
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

type HostNavigationRoute = std::ops::ControlFlow<Option<Vec<tower_lsp_server::ls_types::Location>>>;

impl Backend {
    pub(super) fn calculate_original_source_navigation(
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
                    range: super::super::super::lift_lsp_range(range),
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
                        range: super::super::super::lift_lsp_range(range),
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
