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

//! Logical call lenses and the call hierarchy: the publication stamp a session keeps, and the prepare, walk and resolve answers.

use super::super::{Backend, DocumentPathAdmission, DocumentSourceResult};
use super::{
    OriginalReadonlyReferenceDocument, OriginalReadonlyReferenceReport, ReadonlyTransaction,
    coverage_tag, retry_readonly_attempt,
};
use std::ops::ControlFlow;
use std::sync::Arc;
use tcl_compiler::analyser::ResolvedAnalysisInput;
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};

// Actual host stamps are kept session-side, beside the CALL receipt in the
// existing bounded registry. Source equality cannot recreate these epochs.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct LogicalCallPublicationStamp {
    pub(super) index_generation: u64,
    pub(super) inputs_epoch: u64,
    pub(super) factory_generation: u64,
    pub(super) documents: Vec<LogicalCallDocumentStamp>,
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct LogicalCallDocumentStamp {
    pub(super) uri: Uri,
    pub(super) source: Arc<str>,
    pub(super) file: tcl_lsp_db::SourceFile,
    pub(super) config: tcl_lsp_db::AnalyserConfig,
    pub(super) input: Arc<ResolvedAnalysisInput>,
    pub(super) admission: Arc<DocumentPathAdmission>,
    pub(super) database_revision: salsa::Revision,
    pub(super) open_revision: Option<u64>,
    pub(super) uri_generation: u64,
    document_revision: u64,
    pub(super) version: Option<i32>,
    raw_text: Option<Arc<str>>,
    pub(super) dialect: String,
    pub(super) dialect_resolution_generation: u64,
    pub(super) resolved_dialect_generation: u64,
    index_revision: u64,
    index_context: tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext,
}

impl std::fmt::Debug for LogicalCallDocumentStamp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LogicalCallDocumentStamp")
            .field("uri", &self.uri)
            .field("database_revision", &self.database_revision)
            .field("document_revision", &self.document_revision)
            .field("index_revision", &self.index_revision)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for LogicalCallPublicationStamp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LogicalCallPublicationStamp")
            .field("index_generation", &self.index_generation)
            .field("inputs_epoch", &self.inputs_epoch)
            .field("factory_generation", &self.factory_generation)
            .field("document_count", &self.documents.len())
            .finish_non_exhaustive()
    }
}

impl LogicalCallPublicationStamp {
    pub(super) fn capture(
        documents: &[OriginalReadonlyReferenceDocument],
        workspace: &tcl_lsp_core::workspace_index::WorkspaceIndex,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> Option<Self> {
        let mut stamps = documents
            .iter()
            .map(|document| {
                let index_context = workspace.diagnostic_source_context(document.uri.as_str())?;
                let expected =
                    tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(
                        &document.analysis,
                    )?;
                if index_context != &expected {
                    return None;
                }
                Some(LogicalCallDocumentStamp {
                    uri: document.uri.clone(),
                    source: document.source.clone(),
                    file: document.file,
                    config: document.config,
                    input: document.input.clone(),
                    admission: document.admission.clone(),
                    database_revision: document.database_revision,
                    open_revision: document.publication.open_revision,
                    uri_generation: document.publication.uri_generation,
                    document_revision: document.publication.document.revision,
                    version: document.publication.document.version,
                    raw_text: document.publication.document.raw_text.clone(),
                    dialect: document.publication.document.dialect.clone(),
                    dialect_resolution_generation: document
                        .publication
                        .document
                        .dialect_resolution_generation,
                    resolved_dialect_generation: document
                        .publication
                        .document
                        .resolved_dialect_generation,
                    index_revision: workspace.document_revision(document.uri.as_str())?,
                    index_context: index_context.clone(),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        stamps.sort_by(|a, b| a.uri.as_str().cmp(b.uri.as_str()));
        Some(Self {
            index_generation: workspace.generation(),
            inputs_epoch,
            factory_generation,
            documents: stamps,
        })
    }
}

pub(super) struct LogicalCallProjection {
    pub(super) items: Vec<tower_lsp_server::ls_types::CallHierarchyItem>,
    pub(super) edges: Vec<(tower_lsp_server::ls_types::CallHierarchyItem, Vec<Range>)>,
}

pub(super) fn logical_call_inventory(
    documents: &[OriginalReadonlyReferenceDocument],
) -> Vec<tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>> {
    documents
        .iter()
        .map(
            |document| tcl_lsp_core::original_declaration::OriginalDeclarationDocument {
                uri: document.uri.as_str(),
                source: &document.source,
                analysis: &document.analysis,
            },
        )
        .collect()
}

fn lift_logical_call_item(
    selected: &tcl_lsp_core::call_hierarchy::LogicalCallHierarchyItem,
    registry: &mut tcl_lsp_core::original_declaration::OriginalDeclarationRegistry,
    publication: &tcl_lsp_core::original_declaration::LogicalCallHostPublication,
) -> Option<tower_lsp_server::ls_types::CallHierarchyItem> {
    let identity = selected.identity();
    let handle = registry.issue_logical_call(identity.clone(), publication.clone())?;
    let item = selected.presentation();
    Some(tower_lsp_server::ls_types::CallHierarchyItem {
        name: item.name.clone(),
        kind: tower_lsp_server::ls_types::SymbolKind::FUNCTION,
        tags: None,
        detail: item.detail.clone(),
        uri: identity.uri().parse().ok()?,
        range: super::super::lift_lsp_range(item.range),
        selection_range: super::super::lift_lsp_range(item.selection_range),
        data: Some(serde_json::json!({ "logicalCallHierarchy": handle })),
    })
}

impl Backend {
    pub(super) async fn logical_call_documents(
        &self,
        selected: &Uri,
    ) -> DocumentSourceResult<Option<Vec<OriginalReadonlyReferenceDocument>>> {
        let documents = self.original_declaration_documents(selected).await?;
        let mut retained = Vec::with_capacity(documents.len());
        for (uri, source, _) in documents {
            let Some(document) = self
                .capture_readonly_reference_document(&uri, source)
                .await?
            else {
                return Ok(None);
            };
            retained.push(document);
        }
        if !retained.iter().any(|document| document.uri == *selected) {
            return Ok(None);
        }
        Ok(Some(retained))
    }

    pub(in super::super) async fn readonly_logical_lenses(
        &self,
        selected: &Uri,
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::CodeLens>>> {
        retry_readonly_attempt(None, move || self.readonly_logical_lenses_attempt(selected)).await
    }

    async fn readonly_logical_lenses_attempt(
        &self,
        selected: &Uri,
    ) -> DocumentSourceResult<ControlFlow<Option<Vec<tower_lsp_server::ls_types::CodeLens>>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(documents) = self.logical_call_documents(selected).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &documents,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(lenses) = tcl_lsp_core::code_lens::logical_source_lenses(
            selected.as_str(),
            &logical_call_inventory(&documents),
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let mut registry = self.original_declaration_registry.lock().await;
        Ok(self.readonly_documents_transaction(&documents, epoch, generation, || {
            let workspace = self.workspace_index.try_read().ok()?;
            if LogicalCallPublicationStamp::capture(&documents, &workspace, epoch, generation).as_ref() != Some(&stamp) {
                return None;
            }
            let inventory = logical_call_inventory(&documents);
            let publication = tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(stamp.clone());
            lenses.iter().map(|lens| {
                lens.identity().is_current_in(&inventory).then_some(())?;
                let handle = registry.issue_logical_lens(lens.identity().clone(), publication.clone())?;
                Some(tower_lsp_server::ls_types::CodeLens {
                    range: super::super::lift_lsp_range(lens.range()), command: None,
                    data: Some(serde_json::json!({ "logicalSourceLens": handle, "uri": selected.as_str() })),
                })
            }).collect::<Option<Vec<_>>>()
        }).await?.into_attempt())
    }

    pub(in super::super) async fn readonly_logical_lens_resolve(
        &self,
        selected: &Uri,
        handle: tcl_lsp_core::original_declaration::OriginalDeclarationHandle,
    ) -> DocumentSourceResult<Option<tower_lsp_server::ls_types::CodeLens>> {
        retry_readonly_attempt(None, move || {
            self.readonly_logical_lens_resolve_attempt(selected, handle)
        })
        .await
    }

    async fn readonly_logical_lens_resolve_attempt(
        &self,
        selected: &Uri,
        handle: tcl_lsp_core::original_declaration::OriginalDeclarationHandle,
    ) -> DocumentSourceResult<ControlFlow<Option<tower_lsp_server::ls_types::CodeLens>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(documents) = self.logical_call_documents(selected).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &documents,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let publication =
            tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(stamp.clone());
        let Some(identity) = self
            .original_declaration_registry
            .lock()
            .await
            .resolve_logical_lens(
                handle,
                selected.as_str(),
                &logical_call_inventory(&documents),
                &publication,
            )
        else {
            return Ok(ControlFlow::Break(None));
        };
        let uri = selected.clone();
        let (documents, selection, report) = crate::rt::spawn_blocking(move || {
            let inventory = logical_call_inventory(&documents);
            let selection =
                tcl_lsp_core::code_lens::logical_source_lenses(uri.as_str(), &inventory).and_then(
                    |lenses| lenses.into_iter().find(|lens| lens.identity() == &identity),
                );
            let report =
                tcl_lsp_core::code_lens::logical_lens_reference_advice(&identity, &inventory);
            (documents, selection, report)
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let (Some(selection), Some(report)) = (selection, report) else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(self
            .readonly_documents_transaction(&documents, epoch, generation, || {
                let workspace = self.workspace_index.try_read().ok()?;
                if LogicalCallPublicationStamp::capture(&documents, &workspace, epoch, generation)
                    .as_ref()
                    != Some(&stamp)
                    || !report.is_current(&logical_call_inventory(&documents))
                {
                    return None;
                }
                let locations = report
                    .possible_source_targets()
                    .iter()
                    .map(|(owner, range)| {
                        documents
                            .iter()
                            .find(|document| document.uri.as_str() == owner)
                            .map(|document| Location {
                                uri: document.uri.clone(),
                                range: super::super::lift_lsp_range(*range),
                            })
                    })
                    .collect::<Option<Vec<_>>>()?;
                let range = super::super::lift_lsp_range(selection.range());
                let command = OriginalReadonlyReferenceReport {
                    locations,
                    coverage: report.coverage(),
                }
                .lens_command(selected, range.start);
                Some(tower_lsp_server::ls_types::CodeLens {
                    range,
                    command: Some(command),
                    data: Some(serde_json::json!({
                        "logicalSourceLens": handle, "uri": selected.as_str(),
                        "sourceReferenceCoverage": coverage_tag(report.coverage()),
                    })),
                })
            })
            .await?
            .into_attempt())
    }

    pub(in super::super) async fn readonly_logical_call_prepare(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::CallHierarchyItem>>> {
        retry_readonly_attempt(None, move || {
            self.readonly_logical_call_prepare_attempt(selected, position)
        })
        .await
    }

    async fn readonly_logical_call_prepare_attempt(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<ControlFlow<Option<Vec<tower_lsp_server::ls_types::CallHierarchyItem>>>>
    {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(retained) = self.logical_call_documents(selected).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let uri = selected.clone();
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &retained,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let (retained, advice) = crate::rt::spawn_blocking(move || {
            let inventory = logical_call_inventory(&retained);
            let advice = tcl_lsp_core::call_hierarchy::prepare_logical_from_inventory(
                uri.as_str(),
                position.line,
                position.character,
                &inventory,
            );
            (retained, advice)
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(self
            .finish_logical_call_advice(&retained, &advice, &stamp)
            .await?
            .into_attempt()
            .map_break(|projection| projection.map(|projection| projection.items)))
    }

    pub(in super::super) async fn readonly_logical_call_walk(
        &self,
        item: &tower_lsp_server::ls_types::CallHierarchyItem,
        incoming: bool,
    ) -> DocumentSourceResult<
        Option<Vec<(tower_lsp_server::ls_types::CallHierarchyItem, Vec<Range>)>>,
    > {
        retry_readonly_attempt(None, move || {
            self.readonly_logical_call_walk_attempt(item, incoming)
        })
        .await
    }

    async fn readonly_logical_call_walk_attempt(
        &self,
        item: &tower_lsp_server::ls_types::CallHierarchyItem,
        incoming: bool,
    ) -> DocumentSourceResult<
        ControlFlow<Option<Vec<(tower_lsp_server::ls_types::CallHierarchyItem, Vec<Range>)>>>,
    > {
        let Some(handle) = item
            .data
            .as_ref()
            .and_then(|data| data.get("logicalCallHierarchy"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
        else {
            return Ok(ControlFlow::Break(None));
        };
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(retained) = self.logical_call_documents(&item.uri).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &retained,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let publication =
            tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(stamp.clone());
        let Some(identity) = self
            .original_declaration_registry
            .lock()
            .await
            .resolve_logical_call(
                handle,
                item.uri.as_str(),
                &logical_call_inventory(&retained),
                &publication,
            )
        else {
            return Ok(ControlFlow::Break(None));
        };
        let (retained, advice) = crate::rt::spawn_blocking(move || {
            let inventory = logical_call_inventory(&retained);
            let advice = if incoming {
                tcl_lsp_core::call_hierarchy::logical_incoming_from_inventory(&identity, &inventory)
            } else {
                tcl_lsp_core::call_hierarchy::logical_outgoing_from_inventory(&identity, &inventory)
            };
            (retained, advice)
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
            uri: item.uri.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(self
            .finish_logical_call_advice(&retained, &advice, &stamp)
            .await?
            .into_attempt()
            .map_break(|projection| projection.map(|projection| projection.edges)))
    }

    pub(super) async fn finish_logical_call_advice(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        advice: &tcl_lsp_core::call_hierarchy::LogicalCallAdvice,
        stamp: &LogicalCallPublicationStamp,
    ) -> DocumentSourceResult<ReadonlyTransaction<LogicalCallProjection>> {
        let mut registry = self.original_declaration_registry.lock().await;
        self.readonly_documents_transaction(
            documents,
            stamp.inputs_epoch,
            stamp.factory_generation,
            || {
                // Nonblocking acquisition inside the existing final lease preserves
                // its lock ordering and holds the actual indexed revision through
                // issuance. Missing/busy index is unavailable, never fresh by bytes.
                let workspace = self.workspace_index.try_read().ok()?;
                if LogicalCallPublicationStamp::capture(
                    documents,
                    &workspace,
                    stamp.inputs_epoch,
                    stamp.factory_generation,
                )
                .as_ref()
                    != Some(stamp)
                {
                    return None;
                }
                let publication =
                    tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(
                        stamp.clone(),
                    );
                if !advice.is_current(&logical_call_inventory(documents)) {
                    return None;
                }
                // Retain typed debt and every independent original source receipt
                // through this lease. Standard LSP lists project possible positives;
                // neither an empty list nor an item grants call-graph absence.
                let items = advice
                    .possible_source_items()
                    .iter()
                    .map(|item| lift_logical_call_item(item, &mut registry, &publication))
                    .collect::<Option<Vec<_>>>()?;
                let edges = advice
                    .possible_source_edges()
                    .iter()
                    .map(|edge| {
                        Some((
                            lift_logical_call_item(edge.item(), &mut registry, &publication)?,
                            edge.possible_source_ranges()
                                .iter()
                                .copied()
                                .map(super::super::lift_lsp_range)
                                .collect(),
                        ))
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(LogicalCallProjection { items, edges })
            },
        )
        .await
    }
}
