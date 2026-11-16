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

//! The logical type hierarchy and implementation answers.

use super::super::{Backend, DocumentSourceResult};
use super::{OriginalReadonlyReferenceDocument, ReadonlyTransaction, retry_readonly_attempt};
use std::ops::ControlFlow;
use tower_lsp_server::ls_types::{Location, Position, Uri};

impl Backend {
    async fn finish_logical_hierarchy_projection<'a>(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        projection: impl Fn() -> &'a [tcl_lsp_core::type_hierarchy::TypeHierarchyItem],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<ReadonlyTransaction<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>>>
    {
        let mut registry = self.original_declaration_registry.lock().await;
        self.readonly_documents_transaction(documents, inputs_epoch, factory_generation, || {
            projection()
                .iter()
                .map(|item| {
                    let identity = item.logical_declaration.as_ref()?;
                    let document = documents
                        .iter()
                        .find(|document| document.uri.as_str() == identity.uri())?;
                    if !identity.is_current(identity.uri(), &document.source, &document.analysis) {
                        return None;
                    }
                    let handle = registry.issue_logical_class(identity.clone())?;
                    Some(tower_lsp_server::ls_types::TypeHierarchyItem {
                        name: item.name.clone(),
                        kind: tower_lsp_server::ls_types::SymbolKind::CLASS,
                        tags: None,
                        detail: item.detail.clone(),
                        uri: document.uri.clone(),
                        range: super::super::lift_lsp_range(item.range),
                        selection_range: super::super::lift_lsp_range(item.selection_range),
                        data: Some(serde_json::json!({ "logicalHierarchy": handle })),
                    })
                })
                .collect::<Option<Vec<_>>>()
        })
        .await
    }

    pub(super) async fn finish_logical_hierarchy_advice(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        advice: &tcl_lsp_core::type_hierarchy::LogicalHierarchyAdvice,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<ReadonlyTransaction<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>>>
    {
        self.finish_logical_hierarchy_projection(
            documents,
            || advice.possible_source_items(),
            inputs_epoch,
            factory_generation,
        )
        .await
    }

    #[cfg(test)]
    pub(super) async fn finish_logical_hierarchy_items(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        items: &[tcl_lsp_core::type_hierarchy::TypeHierarchyItem],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        self.finish_logical_hierarchy_projection(
            documents,
            || items,
            inputs_epoch,
            factory_generation,
        )
        .await
        .map(|transaction| transaction.published().unwrap_or_default())
    }

    pub(in super::super) async fn readonly_logical_hierarchy_prepare(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        retry_readonly_attempt(Vec::new(), move || {
            self.readonly_logical_hierarchy_prepare_attempt(selected, position)
        })
        .await
    }

    async fn readonly_logical_hierarchy_prepare_attempt(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<ControlFlow<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(current) = self.read_document(selected).await? else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let Some(document) = self
            .capture_readonly_reference_document(selected.as_str(), current.text)
            .await?
        else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let (document, advice) = crate::rt::spawn_blocking(move || {
            let advice = tcl_lsp_core::type_hierarchy::prepare_logical_advice_for_document(
                document.uri.as_str(),
                &document.source,
                position.line,
                position.character,
                &document.analysis,
            );
            (document, advice)
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        Ok(self
            .finish_logical_hierarchy_advice(&[document], &advice, epoch, generation)
            .await?
            .into_attempt()
            .map_break(Option::unwrap_or_default))
    }

    pub(in super::super) async fn readonly_logical_hierarchy_walk(
        &self,
        item: &tower_lsp_server::ls_types::TypeHierarchyItem,
        subtypes: bool,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        retry_readonly_attempt(Vec::new(), move || {
            self.readonly_logical_hierarchy_walk_attempt(item, subtypes)
        })
        .await
    }

    async fn readonly_logical_hierarchy_walk_attempt(
        &self,
        item: &tower_lsp_server::ls_types::TypeHierarchyItem,
        subtypes: bool,
    ) -> DocumentSourceResult<ControlFlow<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>>> {
        let Some(handle) = item
            .data
            .as_ref()
            .and_then(|data| data.get("logicalHierarchy"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
        else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let documents = self.original_declaration_documents(&item.uri).await?;
        let mut retained = Vec::with_capacity(documents.len());
        for (uri, source, _) in documents {
            let Some(document) = self
                .capture_readonly_reference_document(&uri, source)
                .await?
            else {
                return Ok(ControlFlow::Break(Vec::new()));
            };
            retained.push(document);
        }
        let Some(owner) = retained.iter().find(|document| document.uri == item.uri) else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let Some(identity) = self
            .original_declaration_registry
            .lock()
            .await
            .resolve_logical_class(handle, item.uri.as_str(), &owner.source, &owner.analysis)
        else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let (retained, advice) = crate::rt::spawn_blocking(move || {
            let inventory = retained
                .iter()
                .map(
                    |document| tcl_lsp_core::type_hierarchy::OriginalHierarchyDocument {
                        uri: document.uri.as_str(),
                        source: &document.source,
                        analysis: &document.analysis,
                    },
                )
                .collect::<Vec<_>>();
            let advice = tcl_lsp_core::type_hierarchy::related_logical_from_inventory(
                &identity, &inventory, subtypes,
            );
            (retained, advice)
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
            uri: item.uri.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        Ok(self
            .finish_logical_hierarchy_advice(&retained, &advice, epoch, generation)
            .await?
            .into_attempt()
            .map_break(Option::unwrap_or_default))
    }

    pub(in super::super) async fn readonly_logical_implementation(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<Option<Vec<Location>>> {
        retry_readonly_attempt(None, move || {
            self.readonly_logical_implementation_attempt(selected, position)
        })
        .await
    }

    async fn readonly_logical_implementation_attempt(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<ControlFlow<Option<Vec<Location>>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(current) = self.read_document(selected).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(document) = self
            .capture_readonly_reference_document(selected.as_str(), current.text)
            .await?
        else {
            return Ok(ControlFlow::Break(None));
        };
        let (document, advice) = crate::rt::spawn_blocking(move || {
            let advice = tcl_lsp_core::implementation::logical_implementation_advice(
                document.uri.as_str(),
                &document.source,
                position.line,
                position.character,
                &document.analysis,
            );
            (document, advice)
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(self
            .finish_logical_implementation_advice(&[document], &advice, epoch, generation)
            .await?
            .into_attempt())
    }

    pub(super) async fn finish_logical_implementation_advice(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        advice: &tcl_lsp_core::implementation::LogicalImplementationAdvice,
        epoch: u64,
        generation: u64,
    ) -> DocumentSourceResult<ReadonlyTransaction<Vec<Location>>> {
        self.readonly_documents_transaction(documents, epoch, generation, || {
            let owner = documents.first()?;
            if !advice.is_current(owner.uri.as_str(), &owner.source, &owner.analysis) {
                return None;
            }
            // Standard Locations carry possible positives only; the typed
            // source result and its debt stay held through this final lease.
            Some(
                advice
                    .possible_source_ranges()
                    .iter()
                    .map(|range| Location {
                        uri: owner.uri.clone(),
                        range: super::super::lift_lsp_range(*range),
                    })
                    .collect(),
            )
        })
        .await
    }
}
