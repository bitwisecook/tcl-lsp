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

//! Call hierarchy over source views: the prepare route, the walk from a saved handle and the wire locators issued from live views.

use super::super::{Arc, Backend, DocumentSourceResult};
use super::{OriginalSourceScopeProjection, OriginalSourceViewId, PublishedOriginalSourceView};
use tower_lsp_server::ls_types::Uri;

impl Backend {
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
            let range = super::super::super::lift_lsp_range(range);
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
}

impl Backend {
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
            range: super::super::super::lift_lsp_range(core.range),
            selection_range: super::super::super::lift_lsp_range(core.selection_range),
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
