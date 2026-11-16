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

//! Hover over source views: the route, the authoring hover of an unavailable graph and the procedure cards.

use super::super::{Arc, Backend, DocumentSourceResult};
use super::{OriginalSourceViewId, PublishedOriginalSourceView};
use tower_lsp_server::ls_types::Uri;

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
            super::super::WorkspaceSymbolAnalysisKind::Database(document) => {
                Arc::clone(&document.analysis)
            }
            super::super::WorkspaceSymbolAnalysisKind::Fresh(document) => {
                Arc::clone(&document.seed.analysis)
            }
        };
        let text = Arc::clone(&snapshot.text);
        let request_uri = uri.clone();
        let mut hover = super::super::super::rt::spawn_blocking(move || {
            tcl_lsp_core::hover::source_documentation_hover(
                &text,
                position.line,
                position.character,
                &analysis,
            )
        })
        .await
        .map_err(|_| super::super::super::DocumentSourceUnavailable::Worker { uri: request_uri })?;
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

    pub(super) fn calculate_partial_original_procedure_hover(
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

    pub(super) fn calculate_original_source_hover(
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
