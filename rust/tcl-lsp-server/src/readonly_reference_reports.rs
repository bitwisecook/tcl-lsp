// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reporting-only source reference locations and their independent coverage.

use super::{
    Backend, DocumentPathAdmission, DocumentSourceResult, LibraryPublicationSource, with_pack_hooks,
};
use std::sync::Arc;
use tcl_compiler::analyser::{AnalysisResult, ResolvedAnalysisInput};
use tcl_lsp_core::original_declaration::{OriginalDeclarationIdentity, ReadonlyReferenceCoverage};
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};

struct OriginalReadonlyReferenceDocument {
    uri: Uri,
    source: Arc<str>,
    analysis: Arc<AnalysisResult>,
    file: tcl_lsp_db::SourceFile,
    config: tcl_lsp_db::AnalyserConfig,
    input: Arc<ResolvedAnalysisInput>,
    admission: Arc<DocumentPathAdmission>,
    publication: LibraryPublicationSource,
    database_revision: salsa::Revision,
}

struct ReadonlySourceDocument {
    uri: Uri,
    source: Arc<str>,
    file: tcl_lsp_db::SourceFile,
    config: tcl_lsp_db::AnalyserConfig,
    input: Arc<ResolvedAnalysisInput>,
    admission: Arc<DocumentPathAdmission>,
    publication: LibraryPublicationSource,
    database_revision: salsa::Revision,
}

pub(super) struct OriginalReadonlyReferenceReport {
    pub(super) locations: Vec<Location>,
    pub(super) coverage: ReadonlyReferenceCoverage,
}

pub(super) const fn coverage_tag(coverage: ReadonlyReferenceCoverage) -> &'static str {
    match coverage {
        ReadonlyReferenceCoverage::ExistingInventory => "existing-inventory",
        ReadonlyReferenceCoverage::Partial => "partial",
        ReadonlyReferenceCoverage::Unavailable => "unavailable",
    }
}

impl OriginalReadonlyReferenceReport {
    pub(super) fn unavailable() -> Self {
        Self {
            locations: Vec::new(),
            coverage: ReadonlyReferenceCoverage::Unavailable,
        }
    }

    pub(super) fn lens_command(
        self,
        uri: Uri,
        position: Position,
    ) -> tower_lsp_server::ls_types::Command {
        let title = tcl_lsp_core::code_lens::readonly_reference_count_title(
            self.locations.len(),
            self.coverage,
        );
        let clickable = self.coverage == ReadonlyReferenceCoverage::ExistingInventory
            || self.coverage == ReadonlyReferenceCoverage::Partial && !self.locations.is_empty();
        tower_lsp_server::ls_types::Command {
            title,
            command: if clickable {
                "tcl-lsp.showReferences".to_owned()
            } else {
                String::new()
            },
            arguments: clickable.then(|| {
                vec![
                    serde_json::Value::String(uri.to_string()),
                    serde_json::to_value(position).unwrap_or(serde_json::Value::Null),
                    serde_json::to_value(self.locations).unwrap_or(serde_json::Value::Null),
                ]
            }),
        }
    }
}

// Actual host stamps are kept session-side, beside the CALL receipt in the
// existing bounded registry. Source equality cannot recreate these epochs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LogicalCallPublicationStamp {
    index_generation: u64,
    inputs_epoch: u64,
    factory_generation: u64,
    documents: Vec<LogicalCallDocumentStamp>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct LogicalCallDocumentStamp {
    uri: Uri,
    source: Arc<str>,
    file: tcl_lsp_db::SourceFile,
    config: tcl_lsp_db::AnalyserConfig,
    input: Arc<ResolvedAnalysisInput>,
    admission: Arc<DocumentPathAdmission>,
    database_revision: salsa::Revision,
    open_revision: Option<u64>,
    uri_generation: u64,
    document_revision: u64,
    version: Option<i32>,
    raw_text: Option<Arc<str>>,
    dialect: String,
    dialect_resolution_generation: u64,
    resolved_dialect_generation: u64,
    index_revision: u64,
    index_context: tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext,
}
impl LogicalCallPublicationStamp {
    fn capture(
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

struct LogicalCallProjection {
    items: Vec<tower_lsp_server::ls_types::CallHierarchyItem>,
    edges: Vec<(tower_lsp_server::ls_types::CallHierarchyItem, Vec<Range>)>,
}

fn logical_call_inventory(
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
        range: super::lift_lsp_range(item.range),
        selection_range: super::lift_lsp_range(item.selection_range),
        data: Some(serde_json::json!({ "logicalCallHierarchy": handle })),
    })
}

impl Backend {
    async fn capture_readonly_source_document(
        &self,
        owner: &str,
        source: Arc<str>,
    ) -> DocumentSourceResult<Option<ReadonlySourceDocument>> {
        let Ok(uri) = owner.parse::<Uri>() else {
            return Ok(None);
        };
        let Some(document) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        if document.text != source {
            return Ok(None);
        }
        let config = self.resolved_db_config(&uri).await;
        let Some(publication) = self.library_publication_source(&uri, document).await else {
            return Ok(None);
        };
        let (file, input, admission, database_revision) = {
            let db = self.db.lock().await;
            let files = self.db_files.lock().await;
            let Some(&file) = files.get(&uri) else {
                return Ok(None);
            };
            let published = self
                .source_path_admissions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(retained) = published.get(&uri) else {
                return Ok(None);
            };
            if retained.file != file
                || file.path(&*db).as_deref() != retained.admission.analyser_path()
                || file.text(&*db).as_str() != source.as_ref()
                || file.dialect(&*db) != &publication.document.dialect
                || publication
                    .document
                    .path_admission
                    .as_deref()
                    .is_some_and(|admission| admission != retained.admission.as_ref())
            {
                return Ok(None);
            }
            let Ok(input) =
                with_pack_hooks(|| tcl_lsp_db::document_analysis_input(&*db, file, config))
            else {
                return Ok(None);
            };
            (
                file,
                input,
                retained.admission.clone(),
                tcl_lsp_db::database_revision(&db),
            )
        };
        Ok(Some(ReadonlySourceDocument {
            uri,
            source,
            file,
            config,
            input,
            admission,
            publication,
            database_revision,
        }))
    }

    async fn capture_readonly_reference_document(
        &self,
        owner: &str,
        source: Arc<str>,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceDocument>> {
        let Some(retained) = self.capture_readonly_source_document(owner, source).await? else {
            return Ok(None);
        };
        let ReadonlySourceDocument {
            uri,
            source,
            file,
            config,
            input,
            admission,
            publication,
            database_revision,
        } = retained;
        // Reissue this report's metadata from the genuine retained provider;
        // a later equal-path publication cannot relabel an old analysis.
        let mut driver = self.fresh_analysis_driver(admission.clone()).await;
        driver.analyser = driver.analyser.with_resolved_input(input.as_ref().clone());
        let producer_source = source.clone();
        let dialect = publication.document.dialect.clone();
        let original_uri = uri.clone();
        let seed =
            crate::rt::spawn_blocking(move || driver.analyse(&producer_source, &dialect, None))
                .await
                .map_err(|_| super::DocumentSourceUnavailable::Worker { uri: original_uri })??;
        if seed.path_admission.as_ref() != admission.as_ref() {
            return Ok(None);
        }
        let current = OriginalReadonlyReferenceDocument {
            uri,
            source,
            analysis: seed.analysis,
            file,
            config,
            input,
            admission,
            publication,
            database_revision,
        };
        Ok(current.matches_original_input().then_some(current))
    }

    async fn finish_readonly_reference_report(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        targets: Vec<(String, tcl_lexer::Span)>,
        coverage: ReadonlyReferenceCoverage,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceReport>> {
        self.with_current_readonly_documents(documents, inputs_epoch, factory_generation, || {
            lift_readonly_reference_targets(&targets, documents).map(|locations| {
                OriginalReadonlyReferenceReport {
                    locations,
                    coverage,
                }
            })
        })
        .await
    }

    async fn with_current_readonly_documents<T>(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        inputs_epoch: u64,
        factory_generation: u64,
        mut publish: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        let _inputs = self.analyser_inputs_gate.read().await;
        let generation = self.class_factory_generation.read().await;
        let open = self
            .documents
            .lock("readonly_reference_final_snapshot")
            .await;
        let Ok(db) = self.db.try_lock() else {
            return Ok(None);
        };
        let Ok(files) = self.db_files.try_lock() else {
            return Ok(None);
        };
        let published = self
            .source_path_admissions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let uri_generations = self
            .live_publication_uri_generations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.diag_inputs_epoch() != inputs_epoch
            || *generation != factory_generation
            || documents.iter().any(|document| {
                !document.matches_current_snapshot(&uri_generations, &open, &db, &files, &published)
            })
        {
            return Ok(None);
        }
        let admissions = documents
            .iter()
            .map(|document| document.admission.as_ref())
            .collect::<Vec<_>>();
        DocumentPathAdmission::with_current(&admissions, self.store.as_ref(), &mut publish)
    }

    /// Project namespace relationships only under their original input/index
    /// and publication owners. Missing is unavailable, never namespace absence.
    pub(super) async fn finish_conditional_namespace_advice(
        &self,
        advice: &tcl_lsp_core::workspace_index::ConditionalNamespaceImportAdvice,
    ) -> DocumentSourceResult<Option<Vec<String>>> {
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let mut documents = Vec::new();
        for source in advice.documents() {
            let Some(text) = source.source() else {
                return Ok(None);
            };
            let Some(document) = self
                .capture_readonly_source_document(source.uri(), Arc::from(text))
                .await?
            else {
                return Ok(None);
            };
            if !source.matches_input(document.input.as_ref()) {
                return Ok(None);
            }
            documents.push(document);
        }
        let _inputs = self.analyser_inputs_gate.read().await;
        let generation = self.class_factory_generation.read().await;
        let open = self.documents.lock("namespace_advice_final_snapshot").await;
        let index = self.workspace_index.read().await;
        let Ok(db) = self.db.try_lock() else {
            return Ok(None);
        };
        let Ok(files) = self.db_files.try_lock() else {
            return Ok(None);
        };
        let published = self
            .source_path_admissions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let uri_generations = self
            .live_publication_uri_generations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.diag_inputs_epoch() != inputs_epoch
            || *generation != factory_generation
            || !advice.matches_index(&index)
            || documents.iter().any(|document| {
                !document.matches_current_snapshot(&uri_generations, &open, &db, &files, &published)
            })
        {
            return Ok(None);
        }
        let admissions = documents
            .iter()
            .map(|document| document.admission.as_ref())
            .collect::<Vec<_>>();
        DocumentPathAdmission::with_current(&admissions, self.store.as_ref(), || {
            Some(
                advice
                    .compatibility_report()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
        })
    }

    async fn logical_call_documents(
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

    pub(super) async fn readonly_logical_lenses(
        &self,
        selected: &Uri,
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::CodeLens>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(documents) = self.logical_call_documents(selected).await? else {
            return Ok(None);
        };
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &documents,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(None);
        };
        let Some(lenses) = tcl_lsp_core::code_lens::logical_source_lenses(
            selected.as_str(),
            &logical_call_inventory(&documents),
        ) else {
            return Ok(None);
        };
        let mut registry = self.original_declaration_registry.lock().await;
        self.with_current_readonly_documents(&documents, epoch, generation, || {
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
                    range: super::lift_lsp_range(lens.range()), command: None,
                    data: Some(serde_json::json!({ "logicalSourceLens": handle, "uri": selected.as_str() })),
                })
            }).collect::<Option<Vec<_>>>()
        }).await
    }

    pub(super) async fn readonly_logical_lens_resolve(
        &self,
        selected: &Uri,
        handle: tcl_lsp_core::original_declaration::OriginalDeclarationHandle,
    ) -> DocumentSourceResult<Option<tower_lsp_server::ls_types::CodeLens>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(documents) = self.logical_call_documents(selected).await? else {
            return Ok(None);
        };
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &documents,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(None);
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
            return Ok(None);
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
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let (Some(selection), Some(report)) = (selection, report) else {
            return Ok(None);
        };
        self.with_current_readonly_documents(&documents, epoch, generation, || {
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
                            range: super::lift_lsp_range(*range),
                        })
                })
                .collect::<Option<Vec<_>>>()?;
            let range = super::lift_lsp_range(selection.range());
            let command = OriginalReadonlyReferenceReport {
                locations,
                coverage: report.coverage(),
            }
            .lens_command(selected.clone(), range.start);
            Some(tower_lsp_server::ls_types::CodeLens {
                range,
                command: Some(command),
                data: Some(serde_json::json!({
                    "logicalSourceLens": handle, "uri": selected.as_str(),
                    "sourceReferenceCoverage": coverage_tag(report.coverage()),
                })),
            })
        })
        .await
    }

    pub(super) async fn readonly_logical_call_prepare(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::CallHierarchyItem>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(retained) = self.logical_call_documents(selected).await? else {
            return Ok(None);
        };
        let uri = selected.clone();
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &retained,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(None);
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
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(None);
        };
        self.finish_logical_call_advice(&retained, &advice, &stamp)
            .await
            .map(|projection| projection.map(|projection| projection.items))
    }

    pub(super) async fn readonly_logical_call_walk(
        &self,
        item: &tower_lsp_server::ls_types::CallHierarchyItem,
        incoming: bool,
    ) -> DocumentSourceResult<
        Option<Vec<(tower_lsp_server::ls_types::CallHierarchyItem, Vec<Range>)>>,
    > {
        let Some(handle) = item
            .data
            .as_ref()
            .and_then(|data| data.get("logicalCallHierarchy"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
        else {
            return Ok(None);
        };
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(retained) = self.logical_call_documents(&item.uri).await? else {
            return Ok(None);
        };
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &retained,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(None);
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
            return Ok(None);
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
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: item.uri.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(None);
        };
        self.finish_logical_call_advice(&retained, &advice, &stamp)
            .await
            .map(|projection| projection.map(|projection| projection.edges))
    }

    async fn finish_logical_call_advice(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        advice: &tcl_lsp_core::call_hierarchy::LogicalCallAdvice,
        stamp: &LogicalCallPublicationStamp,
    ) -> DocumentSourceResult<Option<LogicalCallProjection>> {
        let mut registry = self.original_declaration_registry.lock().await;
        self.with_current_readonly_documents(
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
                                .map(super::lift_lsp_range)
                                .collect(),
                        ))
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(LogicalCallProjection { items, edges })
            },
        )
        .await
    }

    async fn finish_logical_hierarchy_projection<'a>(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        projection: impl Fn() -> &'a [tcl_lsp_core::type_hierarchy::TypeHierarchyItem],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        let mut registry = self.original_declaration_registry.lock().await;
        self.with_current_readonly_documents(documents, inputs_epoch, factory_generation, || {
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
                        range: super::lift_lsp_range(item.range),
                        selection_range: super::lift_lsp_range(item.selection_range),
                        data: Some(serde_json::json!({ "logicalHierarchy": handle })),
                    })
                })
                .collect::<Option<Vec<_>>>()
        })
        .await
        .map(Option::unwrap_or_default)
    }

    async fn finish_logical_hierarchy_advice(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        advice: &tcl_lsp_core::type_hierarchy::LogicalHierarchyAdvice,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        self.finish_logical_hierarchy_projection(
            documents,
            || advice.possible_source_items(),
            inputs_epoch,
            factory_generation,
        )
        .await
    }

    #[cfg(test)]
    async fn finish_logical_hierarchy_items(
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
    }

    pub(super) async fn readonly_logical_hierarchy_prepare(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(current) = self.read_document(selected).await? else {
            return Ok(Vec::new());
        };
        let Some(document) = self
            .capture_readonly_reference_document(selected.as_str(), current.text)
            .await?
        else {
            return Ok(Vec::new());
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
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(Vec::new());
        };
        self.finish_logical_hierarchy_advice(&[document], &advice, epoch, generation)
            .await
    }

    pub(super) async fn readonly_logical_hierarchy_walk(
        &self,
        item: &tower_lsp_server::ls_types::TypeHierarchyItem,
        subtypes: bool,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        let Some(handle) = item
            .data
            .as_ref()
            .and_then(|data| data.get("logicalHierarchy"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
        else {
            return Ok(Vec::new());
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
                return Ok(Vec::new());
            };
            retained.push(document);
        }
        let Some(owner) = retained.iter().find(|document| document.uri == item.uri) else {
            return Ok(Vec::new());
        };
        let Some(identity) = self
            .original_declaration_registry
            .lock()
            .await
            .resolve_logical_class(handle, item.uri.as_str(), &owner.source, &owner.analysis)
        else {
            return Ok(Vec::new());
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
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: item.uri.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(Vec::new());
        };
        self.finish_logical_hierarchy_advice(&retained, &advice, epoch, generation)
            .await
    }

    pub(super) async fn readonly_logical_implementation(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<Option<Vec<Location>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(current) = self.read_document(selected).await? else {
            return Ok(None);
        };
        let Some(document) = self
            .capture_readonly_reference_document(selected.as_str(), current.text)
            .await?
        else {
            return Ok(None);
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
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(None);
        };
        self.finish_logical_implementation_advice(&[document], &advice, epoch, generation)
            .await
    }

    async fn finish_logical_implementation_advice(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        advice: &tcl_lsp_core::implementation::LogicalImplementationAdvice,
        epoch: u64,
        generation: u64,
    ) -> DocumentSourceResult<Option<Vec<Location>>> {
        self.with_current_readonly_documents(documents, epoch, generation, || {
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
                        range: super::lift_lsp_range(*range),
                    })
                    .collect(),
            )
        })
        .await
    }

    /// Source-only occurrences join the original declaration in its real URI.
    /// All participants retain their actual SourceFile and full checked input;
    /// this report cannot be consumed as call edges or editable occurrences.
    pub(super) async fn readonly_original_declaration_reference_report(
        &self,
        identity: &OriginalDeclarationIdentity,
        selected: &Uri,
        include_declaration: bool,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceReport>> {
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let documents = self.original_declaration_documents(selected).await?;
        let mut retained = Vec::with_capacity(documents.len());
        for (owner, source, _) in documents {
            let Some(document) = self
                .capture_readonly_reference_document(&owner, source)
                .await?
            else {
                return Ok(None);
            };
            retained.push(document);
        }
        let Some(owner) = retained
            .iter()
            .find(|document| document.uri.as_str() == identity.uri())
        else {
            return Ok(None);
        };
        if !identity.is_current(identity.uri(), &owner.source, &owner.analysis) {
            return Ok(None);
        }
        if !original_declaration_owner_is_unique(identity, &retained) {
            return Ok(None);
        }
        let Some(report) = tcl_lsp_core::original_declaration::readonly_reference_report(
            identity,
            &owner.source,
            &owner.analysis,
            true,
        ) else {
            return Ok(None);
        };
        let inventory = retained
            .iter()
            .map(
                |document| tcl_lsp_core::original_call_hierarchy::OriginalCallDocument {
                    uri: document.uri.as_str(),
                    source: &document.source,
                    analysis: &document.analysis,
                },
            )
            .collect::<Vec<_>>();
        let mut targets = {
            let workspace = self.workspace_index.read().await;
            tcl_lsp_core::original_call_hierarchy::references(
                identity,
                &inventory,
                Some(&workspace),
            )
        };
        targets.extend(
            report
                .spans()
                .iter()
                .map(|span| (identity.uri().to_owned(), *span)),
        );
        if include_declaration {
            targets.push((identity.uri().to_owned(), identity.span()));
        }
        super::dedup_span_targets(&mut targets);
        self.finish_readonly_reference_report(
            &retained,
            targets,
            report.coverage(),
            inputs_epoch,
            factory_generation,
        )
        .await
    }

    pub(super) async fn readonly_original_method_reference_locations_at(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
        include_declaration: bool,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Vec<Location>>> {
        use std::ops::ControlFlow;
        let target = match self
            .original_method_candidate_at(uri, source, analysis, position)
            .await?
        {
            ControlFlow::Continue(()) => return Ok(ControlFlow::Continue(())),
            ControlFlow::Break(None) => return Ok(ControlFlow::Break(Vec::new())),
            ControlFlow::Break(Some(target)) => target,
        };
        let Some(owner) = target.uri().parse::<Uri>().ok() else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let Some(document) = self.read_document(&owner).await? else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let current = self
            .analysis_for(&owner, document.text.clone(), document.dialect)
            .await?;
        let Some(identity) = OriginalDeclarationIdentity::for_member_candidate(
            owner.as_str(),
            &document.text,
            &current,
            &target,
        ) else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        Ok(ControlFlow::Break(
            self.readonly_original_declaration_reference_report(
                &identity,
                uri,
                include_declaration,
            )
            .await?
            .map_or_else(Vec::new, |report| report.locations),
        ))
    }
}

fn lift_readonly_reference_targets(
    targets: &[(String, tcl_lexer::Span)],
    documents: &[OriginalReadonlyReferenceDocument],
) -> Option<Vec<Location>> {
    targets
        .iter()
        .map(|(uri, span)| {
            let document = documents
                .iter()
                .find(|document| document.uri.as_str() == uri.as_str())?;
            if span.start() > span.end() || span.end() as usize > document.source.len() {
                return None;
            }
            let index = tcl_lexer::LineIndex::new_lsp(&document.source);
            let start = index.position_at_utf16(span.start(), &document.source);
            let end = index.position_at_utf16(span.end(), &document.source);
            Some(Location {
                uri: document.uri.clone(),
                range: Range {
                    start: Position::new(start.line, start.character.get()),
                    end: Position::new(end.line, end.character.get()),
                },
            })
        })
        .collect()
}

fn original_declaration_owner_is_unique(
    identity: &OriginalDeclarationIdentity,
    documents: &[OriginalReadonlyReferenceDocument],
) -> bool {
    documents
        .iter()
        .filter(|document| {
            tcl_lsp_core::original_declaration::declarations(
                document.uri.as_str(),
                &document.source,
                &document.analysis,
            )
            .is_some_and(|rows| {
                rows.iter().any(|row| {
                    row.role() == identity.role()
                        && row.site() == identity.site()
                        && row.image() == identity.image()
                        && row.config() == identity.config()
                })
            })
        })
        .count()
        == 1
}

impl OriginalReadonlyReferenceDocument {
    fn matches_current_snapshot(
        &self,
        uri_generations: &std::collections::HashMap<Uri, u64>,
        open: &std::collections::HashMap<Uri, super::DocumentState>,
        db: &tcl_lsp_db::TclDatabase,
        files: &std::collections::HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &std::collections::HashMap<Uri, super::PublishedSourcePath>,
    ) -> bool {
        self.matches_original_input()
            && self.source_currency().matches_current_snapshot(
                uri_generations,
                open,
                db,
                files,
                published,
            )
    }

    fn source_currency(&self) -> ReadonlySourceDocument {
        ReadonlySourceDocument {
            uri: self.uri.clone(),
            source: self.source.clone(),
            file: self.file,
            config: self.config,
            input: self.input.clone(),
            admission: self.admission.clone(),
            publication: LibraryPublicationSource {
                uri: self.publication.uri.clone(),
                document: self.publication.document.clone(),
                open_revision: self.publication.open_revision,
                uri_generation: self.publication.uri_generation,
            },
            database_revision: self.database_revision,
        }
    }

    fn matches_original_input(&self) -> bool {
        let config = self.input.lexer_config();
        self.analysis.resolved_input.as_ref() == Some(self.input.as_ref())
            && self.analysis.analysis_context_unavailable.is_none()
            && self.analysis.body_lexer_config == Some(config)
            && self.analysis.matches_original_source_image(
                &tcl_lexer::SourceImage::document(&self.source),
                config,
            )
            && self
                .analysis
                .retained_command_realm()
                .is_some_and(|realm| realm.matches_resolved_analysis_input(self.input.as_ref()))
    }
}

impl ReadonlySourceDocument {
    fn matches_current_snapshot(
        &self,
        uri_generations: &std::collections::HashMap<Uri, u64>,
        open: &std::collections::HashMap<Uri, super::DocumentState>,
        db: &tcl_lsp_db::TclDatabase,
        files: &std::collections::HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &std::collections::HashMap<Uri, super::PublishedSourcePath>,
    ) -> bool {
        self.publication.matches_open_documents(open)
            && uri_generations.get(&self.uri).copied().unwrap_or_default()
                == self.publication.uri_generation
            && tcl_lsp_db::database_revision(db) == self.database_revision
            && files.get(&self.uri) == Some(&self.file)
            && published.get(&self.uri).is_some_and(|retained| {
                retained.file == self.file && retained.admission.as_ref() == self.admission.as_ref()
            })
            && self.file.path(db).as_deref() == self.admission.analyser_path()
            && self.file.text(db).as_str() == self.source.as_ref()
            && self.file.dialect(db) == &self.publication.document.dialect
            && with_pack_hooks(|| tcl_lsp_db::document_analysis_input(db, self.file, self.config))
                .is_ok_and(|input| input.as_ref() == self.input.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DocumentState;
    use tower_lsp_server::LanguageServer;
    use tower_lsp_server::ls_types::{CodeLensParams, TextDocumentIdentifier};

    const SOURCE: &str =
        "oo::class create C {method ping {} {}; method body {} {my ping; my ping}}";

    async fn register_current(backend: &Backend, uri: &Uri, source: &str) {
        backend
            .documents
            .lock("readonly reference test")
            .await
            .insert(
                uri.clone(),
                DocumentState::new(source.to_owned(), "tcl8.6".to_owned()),
            );
        backend
            .db_set_source(uri, source, "tcl8.6".to_owned())
            .await
            .unwrap();
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

    async fn ping_identity(backend: &Backend, uri: &Uri) -> OriginalDeclarationIdentity {
        let document = backend.read_document(uri).await.unwrap().unwrap();
        let analysis = backend
            .analysis_for(uri, document.text.clone(), document.dialect)
            .await
            .unwrap();
        let class = analysis.original_class_declarations().next().unwrap();
        let method = class
            .metadata()
            .original_members
            .declarations()
            .find(|method| method.original_name_input().bytes() == b"ping")
            .unwrap();
        OriginalDeclarationIdentity::for_method(
            uri.as_str(),
            &document.text,
            &analysis,
            class,
            method,
        )
        .unwrap()
    }

    async fn snapshot_still_current(
        backend: &Backend,
        retained: &OriginalReadonlyReferenceDocument,
    ) -> bool {
        let generation = *backend.class_factory_generation.read().await;
        backend
            .finish_readonly_reference_report(
                std::slice::from_ref(retained),
                Vec::new(),
                ReadonlyReferenceCoverage::Partial,
                backend.diag_inputs_epoch(),
                generation,
            )
            .await
            .unwrap()
            .is_some()
    }

    #[tokio::test]
    async fn readonly_member_wire_retains_owner_and_partial_coverage() {
        // naming.server.original-reference-and-lens-source-selection
        // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///readonly-member-wire.tcl".parse().unwrap();
        register_current(&backend, &uri, SOURCE).await;
        let identity = ping_identity(&backend, &uri).await;
        let report = backend
            .readonly_original_declaration_reference_report(&identity, &uri, false)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(report.coverage, ReadonlyReferenceCoverage::Partial);
        assert_eq!(report.locations.len(), 2);
        assert!(report.locations.iter().all(|location| location.uri == uri));
        assert!(
            backend
                .original_declaration_reference_locations(&identity, &uri, false)
                .await
                .unwrap()
                .is_empty()
        );
        let lenses = backend
            .code_lens(CodeLensParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            })
            .await
            .unwrap()
            .unwrap();
        let mut lens = lenses
            .into_iter()
            .find(|lens| {
                lens.range.start.character == u32::try_from(SOURCE.find("ping").unwrap()).unwrap()
            })
            .unwrap();
        assert_eq!(
            lens.data.as_ref().unwrap()["sourceReferenceCoverage"],
            "partial"
        );
        let data = lens.data.as_mut().unwrap();
        data["sourceReferenceCoverage"] = serde_json::json!("existing-inventory");
        data["qname"] = serde_json::json!("::unrelated");
        let lens = backend.code_lens_resolve(lens).await.unwrap();
        assert_eq!(
            lens.data.as_ref().unwrap()["sourceReferenceCoverage"],
            "partial"
        );
        let command = lens.command.unwrap();
        assert_eq!(command.title, "at least 2 references");
        assert_eq!(command.command, "tcl-lsp.showReferences");
        let locations: Vec<Location> =
            serde_json::from_value(command.arguments.unwrap()[2].clone()).unwrap();
        assert_eq!(locations, report.locations);
    }

    #[tokio::test]
    async fn readonly_member_wire_withdraws_missing_or_replaced_source_owners() {
        // naming.server.original-reference-and-lens-source-selection
        // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///readonly-member-current.tcl".parse().unwrap();
        let foreign: Uri = "file:///readonly-member-foreign.tcl".parse().unwrap();
        register_current(&backend, &uri, SOURCE).await;
        let identity = ping_identity(&backend, &uri).await;
        let document = backend.read_document(&uri).await.unwrap().unwrap();
        let retained = backend
            .capture_readonly_reference_document(uri.as_str(), document.text.clone())
            .await
            .unwrap()
            .unwrap();
        assert!(snapshot_still_current(&backend, &retained).await);
        register_current(&backend, &foreign, SOURCE).await;
        let foreign_identity = ping_identity(&backend, &foreign).await;
        assert_ne!(identity, foreign_identity);
        assert!(
            backend
                .readonly_original_declaration_reference_report(&identity, &uri, false)
                .await
                .unwrap()
                .is_none()
        );
        backend.db_files.lock().await.remove(&uri);
        assert!(!snapshot_still_current(&backend, &retained).await);
        assert!(
            backend
                .readonly_original_declaration_reference_report(&identity, &uri, false)
                .await
                .unwrap()
                .is_none()
        );
        register_current(&backend, &uri, "oo::class create C {method changed {} {}}").await;
        assert!(!snapshot_still_current(&backend, &retained).await);
        assert!(
            backend
                .readonly_original_declaration_reference_report(&identity, &uri, false)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn readonly_member_wire_rejects_equal_path_provider_republication() {
        // naming.server.original-reference-and-lens-source-selection
        // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
        let mut backend = crate::tests::test_backend();
        let store = Arc::new(crate::vfs::MemoryStore::new());
        let uri: Uri = "file:///readonly-provider-aba.tcl".parse().unwrap();
        let path = "/same/readonly-provider-aba.tcl";
        store.upsert_uri(uri.as_str(), path, SOURCE.as_bytes().to_vec());
        backend.store = store.clone();
        register_current(&backend, &uri, SOURCE).await;
        let retained = backend
            .capture_readonly_reference_document(uri.as_str(), Arc::from(SOURCE))
            .await
            .unwrap()
            .unwrap();
        assert!(snapshot_still_current(&backend, &retained).await);
        store.upsert_uri(
            uri.as_str(),
            "/other/readonly-provider-aba.tcl",
            SOURCE.as_bytes().to_vec(),
        );
        store.upsert_uri(uri.as_str(), path, SOURCE.as_bytes().to_vec());
        register_current(&backend, &uri, SOURCE).await;
        let current = backend
            .capture_readonly_reference_document(uri.as_str(), Arc::from(SOURCE))
            .await
            .unwrap()
            .unwrap();
        assert!(current.file == retained.file);
        assert_eq!(current.input, retained.input);
        assert_ne!(current.admission.as_ref(), retained.admission.as_ref());
        assert!(!snapshot_still_current(&backend, &retained).await);
        assert!(snapshot_still_current(&backend, &current).await);
        backend.invalidate_live_publication(std::iter::once(&uri));
        assert!(!snapshot_still_current(&backend, &current).await);
        let current = backend
            .capture_readonly_reference_document(uri.as_str(), Arc::from(SOURCE))
            .await
            .unwrap()
            .unwrap();
        assert!(snapshot_still_current(&backend, &current).await);
        backend
            .documents
            .lock("readonly_same_bytes_revision")
            .await
            .get_mut(&uri)
            .unwrap()
            .revision += 1;
        assert!(!snapshot_still_current(&backend, &current).await);
    }

    #[test]
    fn unavailable_member_wire_has_no_clickable_empty_census() {
        // naming.server.original-reference-and-lens-source-selection
        // docs/design/analysis/name-resolution-proofs/server-original-reference-and-lens-source-selection.md
        for coverage in [
            ReadonlyReferenceCoverage::Partial,
            ReadonlyReferenceCoverage::Unavailable,
        ] {
            let report = OriginalReadonlyReferenceReport {
                locations: Vec::new(),
                coverage,
            };
            let command = report.lens_command(
                "file:///readonly-empty.tcl".parse().unwrap(),
                Position::new(0, 0),
            );
            assert_eq!(command.title, "References unavailable");
            assert!(command.command.is_empty());
            assert!(command.arguments.is_none());
        }
        let report = OriginalReadonlyReferenceReport {
            locations: Vec::new(),
            coverage: ReadonlyReferenceCoverage::ExistingInventory,
        };
        let command = report.lens_command(
            "file:///readonly-empty.tcl".parse().unwrap(),
            Position::new(0, 0),
        );
        assert_eq!(command.title, "0 references");
        assert_eq!(command.command, "tcl-lsp.showReferences");
    }
}

#[cfg(test)]
mod logical_hierarchy_tests {
    use super::*;
    use crate::DocumentState;

    async fn register(backend: &Backend, uri: &Uri, source: &str) {
        backend
            .documents
            .lock("logical hierarchy test")
            .await
            .insert(
                uri.clone(),
                DocumentState::new(source.to_owned(), "tcl".to_owned()),
            );
        backend
            .db_set_source(uri, source, "tcl".to_owned())
            .await
            .unwrap();
        let analysis = backend
            .analysis_for(uri, Arc::from(source), "tcl".to_owned())
            .await
            .unwrap();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    #[tokio::test]
    async fn logical_hierarchy_wire_uses_issued_class_and_real_cross_document_owners() {
        let backend = crate::tests::test_backend();
        let base: Uri = "file:///tmp/logical-hierarchy-base.tcl".parse().unwrap();
        let child: Uri = "file:///tmp/logical-hierarchy-child.tcl".parse().unwrap();
        let base_source = "namespace eval N {oo::class create Base {}}";
        let child_source = "namespace eval N {oo::class create Child {superclass Base}}";
        register(&backend, &base, base_source).await;
        register(&backend, &child, child_source).await;
        let offset = u32::try_from(child_source.find("Child").unwrap()).unwrap();
        let mut items = backend
            .readonly_logical_hierarchy_prepare(&child, Position::new(0, offset))
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        let mut item = items.remove(0);
        assert!(item.data.is_some());
        item.name = "untrusted label".into();
        item.selection_range = Range::default();
        let parents = backend
            .type_hierarchy_walk(item.clone(), false)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0].name, "::N::Base");
        assert_eq!(parents[0].uri, base);
        assert!(parents[0].data.is_some());
        let children = backend
            .type_hierarchy_walk(parents[0].clone(), true)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].uri, child);
        let mut unissued = item.clone();
        unissued.data = None;
        assert!(
            backend
                .type_hierarchy_walk(unissued, false)
                .await
                .unwrap()
                .unwrap()
                .is_empty()
        );
        let mut foreign = item.clone();
        foreign.uri = base.clone();
        assert!(
            backend
                .type_hierarchy_walk(foreign, false)
                .await
                .unwrap()
                .unwrap()
                .is_empty()
        );
        register(&backend, &child, &format!("{child_source}\n# changed")).await;
        assert!(
            backend
                .type_hierarchy_walk(item, false)
                .await
                .unwrap()
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn logical_hierarchy_final_snapshot_rejects_changed_relation_parent_with_current_target()
    {
        let backend = crate::tests::test_backend();
        let base: Uri = "file:///tmp/logical-hierarchy-final-base.tcl"
            .parse()
            .unwrap();
        let child: Uri = "file:///tmp/logical-hierarchy-final-child.tcl"
            .parse()
            .unwrap();
        let base_source = "oo::class create Base {}";
        let child_source = "oo::class create Child {superclass Base}";
        register(&backend, &base, base_source).await;
        register(&backend, &child, child_source).await;
        let epoch = backend.diag_inputs_epoch();
        let generation = *backend.class_factory_generation.read().await;
        let documents = vec![
            backend
                .capture_readonly_reference_document(base.as_str(), Arc::from(base_source))
                .await
                .unwrap()
                .unwrap(),
            backend
                .capture_readonly_reference_document(child.as_str(), Arc::from(child_source))
                .await
                .unwrap()
                .unwrap(),
        ];
        let position = u32::try_from(child_source.find("Child").unwrap()).unwrap();
        let root = tcl_lsp_core::type_hierarchy::prepare_for_document(
            child.as_str(),
            child_source,
            0,
            position,
            &documents[1].analysis,
        )
        .remove(0)
        .logical_declaration
        .unwrap();
        let inventory = documents
            .iter()
            .map(
                |document| tcl_lsp_core::type_hierarchy::OriginalHierarchyDocument {
                    uri: document.uri.as_str(),
                    source: &document.source,
                    analysis: &document.analysis,
                },
            )
            .collect::<Vec<_>>();
        let items =
            tcl_lsp_core::type_hierarchy::related_logical_from_inventory(&root, &inventory, false)
                .unwrap()
                .into_items();
        assert_eq!(items.len(), 1);
        register(&backend, &child, "oo::class create Child {}").await;
        assert!(
            backend
                .finish_logical_hierarchy_items(&documents, &items, epoch, generation)
                .await
                .unwrap()
                .is_empty()
        );
    }
}

#[cfg(test)]
mod conditional_navigation_publication_tests {
    use super::*;
    use crate::DocumentState;

    async fn register(backend: &Backend, uri: &Uri, source: &str) {
        backend
            .documents
            .lock("conditional_navigation_test")
            .await
            .insert(
                uri.clone(),
                DocumentState::new(source.to_owned(), "tcl".to_owned()),
            );
        backend
            .db_set_source(uri, source, "tcl".to_owned())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn typed_hierarchy_debt_survives_wire_publication_and_parent_currentness() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/conditional-hierarchy-debt.tcl"
            .parse()
            .unwrap();
        let source = "oo::class create Base {}; mystery; oo::class create Child {superclass Base}";
        register(&backend, &uri, source).await;
        let offset = u32::try_from(source.rfind("Base").unwrap()).unwrap();
        let items = backend
            .readonly_logical_hierarchy_prepare(&uri, Position::new(0, offset))
            .await
            .unwrap();
        assert_eq!(items.len(), 1);
        let document = backend
            .capture_readonly_reference_document(uri.as_str(), Arc::from(source))
            .await
            .unwrap()
            .unwrap();
        let handle =
            serde_json::from_value(items[0].data.as_ref().unwrap()["logicalHierarchy"].clone())
                .unwrap();
        let identity = backend
            .original_declaration_registry
            .lock()
            .await
            .resolve_logical_class(handle, uri.as_str(), &document.source, &document.analysis)
            .unwrap();
        assert!(identity.has_unresolved_applicability());
        let advice = tcl_lsp_core::type_hierarchy::prepare_logical_advice_for_document(
            uri.as_str(),
            source,
            0,
            offset,
            &document.analysis,
        )
        .unwrap();
        assert!(advice.has_unresolved_applicability());
        let epoch = backend.diag_inputs_epoch();
        let generation = *backend.class_factory_generation.read().await;
        register(
            &backend,
            &uri,
            "oo::class create Base {}; oo::class create Child {}",
        )
        .await;
        assert!(
            backend
                .finish_logical_hierarchy_advice(&[document], &advice, epoch, generation)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn implementation_possible_locations_keep_typed_debt_until_final_source_lease() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/conditional-implementation-debt.tcl"
            .parse()
            .unwrap();
        let source = "oo::class create Base {method run {} {}}\nmystery\noo::class create Child {superclass Base; method run {} {}}\noo::class create Other {superclass $missing}";
        register(&backend, &uri, source).await;
        let offset = u32::try_from(source.find("run").unwrap()).unwrap();
        let locations = backend
            .readonly_logical_implementation(&uri, Position::new(0, offset))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            locations
                .iter()
                .map(|location| location.range.start.line)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
        let document = backend
            .capture_readonly_reference_document(uri.as_str(), Arc::from(source))
            .await
            .unwrap()
            .unwrap();
        let advice = tcl_lsp_core::implementation::logical_implementation_advice(
            uri.as_str(),
            source,
            0,
            offset,
            &document.analysis,
        )
        .unwrap();
        assert!(advice.has_unresolved_applicability());
        let epoch = backend.diag_inputs_epoch();
        let generation = *backend.class_factory_generation.read().await;
        register(&backend, &uri, &format!("{source}\n# changed")).await;
        assert!(
            backend
                .finish_logical_implementation_advice(&[document], &advice, epoch, generation)
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[cfg(test)]
mod logical_call_wire_tests {
    use super::*;
    use crate::DocumentState;
    use tower_lsp_server::LanguageServer;

    async fn register(backend: &Backend, uri: &Uri, source: &str) {
        backend
            .documents
            .lock("logical call wire test")
            .await
            .insert(
                uri.clone(),
                DocumentState::new(source.to_owned(), "tcl".to_owned()),
            );
        backend
            .db_set_source(uri, source, "tcl".to_owned())
            .await
            .unwrap();
        let analysis = backend
            .analysis_for(uri, Arc::from(source), "tcl".to_owned())
            .await
            .unwrap();
        backend
            .workspace_index
            .write()
            .await
            .replace_document(uri.as_str(), &analysis);
    }
    async fn prepare(
        backend: &Backend,
        uri: &Uri,
        source: &str,
        name: &str,
    ) -> tower_lsp_server::ls_types::CallHierarchyItem {
        let at = u32::try_from(source.find(name).unwrap()).unwrap();
        let point = tcl_lexer::LineIndex::new_lsp(source).position_at_utf16(at, source);
        let params = serde_json::from_value(serde_json::json!({"textDocument":{"uri":uri},"position":{"line":point.line,"character":point.character.get()}})).unwrap();
        let mut items = backend
            .prepare_call_hierarchy(params)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(items.len(), 1);
        items.remove(0)
    }

    #[tokio::test]
    async fn actual_call_handlers_use_opaque_current_source_selection_not_client_fields() {
        let backend = crate::tests::test_backend();
        let caller: Uri = "file:///tmp/issued-call-caller.tcl".parse().unwrap();
        let target: Uri = "file:///tmp/issued-call-target.tcl".parse().unwrap();
        let caller_source = "proc caller {} { helper }\n";
        register(&backend, &caller, caller_source).await;
        register(&backend, &target, "proc helper {} {}\n").await;
        let mut item = prepare(&backend, &caller, caller_source, "helper").await;
        assert_eq!(item.uri, target);
        assert!(
            item.data
                .as_ref()
                .unwrap()
                .get("logicalCallHierarchy")
                .is_some()
        );
        item.name = "client label has no identity".to_owned();
        item.detail = Some("untrusted".to_owned());
        item.range = Range::default();
        item.selection_range = Range::default();
        let params = serde_json::from_value(serde_json::json!({"item":item})).unwrap();
        let incoming = backend.incoming_calls(params).await.unwrap().unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].from.uri, caller);
        assert_eq!(incoming[0].from.name, "::caller");
        let params = serde_json::from_value(serde_json::json!({"item":incoming[0].from})).unwrap();
        let outgoing = backend.outgoing_calls(params).await.unwrap().unwrap();
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].to.uri, target);
        assert!(outgoing[0].to.data.is_some());
        let mut unissued = item.clone();
        unissued.data = None;
        assert!(
            backend
                .readonly_logical_call_walk(&unissued, true)
                .await
                .unwrap()
                .is_none()
        );
        let mut wrong = item.clone();
        wrong.uri = caller;
        assert!(
            backend
                .readonly_logical_call_walk(&wrong, true)
                .await
                .unwrap()
                .is_none()
        );
        for schema in [1, 2] {
            let mut wrong = item.clone();
            wrong.data.as_mut().unwrap()["logicalCallHierarchy"]["schema"] =
                serde_json::json!(schema);
            assert!(
                backend
                    .readonly_logical_call_walk(&wrong, true)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[tokio::test]
    async fn equal_bytes_republication_and_reindex_cannot_renew_prepared_call_stamp() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/issued-call-epochs.tcl".parse().unwrap();
        let source = "proc caller {} { helper }\nproc helper {} {}\n";
        register(&backend, &uri, source).await;
        let item = prepare(&backend, &uri, source, "helper").await;
        let analysis = backend
            .analysis_for(&uri, Arc::from(source), "tcl".to_owned())
            .await
            .unwrap();
        backend
            .workspace_index
            .write()
            .await
            .replace_document(uri.as_str(), &analysis);
        assert!(
            backend
                .readonly_logical_call_walk(&item, true)
                .await
                .unwrap()
                .is_none()
        );
        let item = prepare(&backend, &uri, source, "helper").await;
        backend.invalidate_live_publication(std::iter::once(&uri));
        assert!(
            backend
                .readonly_logical_call_walk(&item, true)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn typed_call_debt_and_parent_target_stamps_survive_until_final_source_lease() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/issued-call-final-lease.tcl".parse().unwrap();
        let source = "mystery\nproc caller {} {helper; $head}\nproc helper {} {}\n";
        register(&backend, &uri, source).await;
        let documents = backend.logical_call_documents(&uri).await.unwrap().unwrap();
        let epoch = backend.diag_inputs_epoch();
        let generation = *backend.class_factory_generation.read().await;
        let stamp = LogicalCallPublicationStamp::capture(
            &documents,
            &*backend.workspace_index.read().await,
            epoch,
            generation,
        )
        .unwrap();
        let inventory = logical_call_inventory(&documents);
        let advice = tcl_lsp_core::call_hierarchy::prepare_logical_from_inventory(
            uri.as_str(),
            1,
            16,
            &inventory,
        )
        .unwrap();
        assert!(advice.has_unresolved_applicability());
        assert_eq!(advice.possible_source_items().len(), 1);
        assert!(
            backend
                .finish_logical_call_advice(&documents, &advice, &stamp)
                .await
                .unwrap()
                .is_some()
        );
        register(&backend, &uri, &format!("{source}# changed\n")).await;
        assert!(
            backend
                .finish_logical_call_advice(&documents, &advice, &stamp)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn missing_actual_index_stamp_is_unavailable_even_with_equal_source_and_analysis() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/issued-call-missing-index.tcl".parse().unwrap();
        let source = "proc helper {} {}\nhelper\n";
        register(&backend, &uri, source).await;
        let item = prepare(&backend, &uri, source, "helper").await;
        backend
            .workspace_index
            .write()
            .await
            .remove_document(uri.as_str());
        assert!(
            backend
                .readonly_logical_call_walk(&item, true)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            backend
                .readonly_logical_call_prepare(&uri, Position::new(0, 5))
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[cfg(test)]
mod logical_call_provider_generation_tests {
    use super::*;
    use crate::DocumentState;

    #[tokio::test]
    async fn newly_indexed_provider_invalidates_old_call_stamp_with_all_retained_rows_unchanged() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/call-provider-generation.tcl".parse().unwrap();
        let source = "proc caller {} {helper}\nproc helper {} {}\n";
        backend
            .documents
            .lock("call provider generation test")
            .await
            .insert(
                uri.clone(),
                DocumentState::new(source.to_owned(), "tcl".to_owned()),
            );
        backend
            .db_set_source(&uri, source, "tcl".to_owned())
            .await
            .unwrap();
        let analysis = backend
            .analysis_for(&uri, Arc::from(source), "tcl".to_owned())
            .await
            .unwrap();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
        let documents = backend.logical_call_documents(&uri).await.unwrap().unwrap();
        let epoch = backend.diag_inputs_epoch();
        let generation = *backend.class_factory_generation.read().await;
        let stamp = LogicalCallPublicationStamp::capture(
            &documents,
            &*backend.workspace_index.read().await,
            epoch,
            generation,
        )
        .unwrap();
        let advice = tcl_lsp_core::call_hierarchy::prepare_logical_from_inventory(
            uri.as_str(),
            0,
            16,
            &logical_call_inventory(&documents),
        )
        .unwrap();
        let projection = backend
            .finish_logical_call_advice(&documents, &advice, &stamp)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(projection.items.len(), 1);
        let handle = serde_json::from_value(
            projection.items[0].data.as_ref().unwrap()["logicalCallHierarchy"].clone(),
        )
        .unwrap();
        let extra = tcl_compiler::analyser::Analyser::new().analyse("proc helper {} {}", "tcl");
        backend
            .workspace_index
            .write()
            .await
            .add_document("file:///tmp/new-call-provider.tcl", &extra);
        let current = LogicalCallPublicationStamp::capture(
            &documents,
            &*backend.workspace_index.read().await,
            epoch,
            generation,
        )
        .unwrap();
        // This isolates a provider-roster mutation: old row source, input,
        // admission, DB/publication and indexed revision/context stay exact.
        assert_eq!(stamp.documents, current.documents);
        assert_ne!(stamp.index_generation, current.index_generation);
        let publication =
            tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(current);
        assert!(
            backend
                .original_declaration_registry
                .lock()
                .await
                .resolve_logical_call(
                    handle,
                    uri.as_str(),
                    &logical_call_inventory(&documents),
                    &publication
                )
                .is_none()
        );
        assert!(
            backend
                .finish_logical_call_advice(&documents, &advice, &stamp)
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[cfg(test)]
mod logical_lens_wire_tests {
    use super::*;
    use crate::DocumentState;
    use tower_lsp_server::LanguageServer;
    use tower_lsp_server::ls_types::{CodeLensParams, TextDocumentIdentifier};

    async fn register(backend: &Backend, uri: &Uri, source: &str) {
        backend.documents.lock("logical lens test").await.insert(
            uri.clone(),
            DocumentState::new(source.to_owned(), "tcl".to_owned()),
        );
        backend
            .db_set_source(uri, source, "tcl".to_owned())
            .await
            .unwrap();
        let analysis = backend
            .analysis_for(uri, Arc::from(source), "tcl".to_owned())
            .await
            .unwrap();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    async fn helper_lens(backend: &Backend, uri: &Uri) -> tower_lsp_server::ls_types::CodeLens {
        backend
            .code_lens(CodeLensParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            })
            .await
            .unwrap()
            .unwrap()
            .into_iter()
            .find(|lens| lens.range.start.line == 0)
            .unwrap()
    }

    #[tokio::test]
    async fn logical_lens_reuses_opaque_source_selection_not_client_range_or_label() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/logical-lens-wire.tcl".parse().unwrap();
        register(
            &backend,
            &uri,
            "proc helper {} {}\nproc caller {} {helper}\n",
        )
        .await;
        let mut lens = helper_lens(&backend, &uri).await;
        let original = lens.range;
        lens.range = Range::new(Position::new(99, 99), Position::new(99, 100));
        lens.data.as_mut().unwrap()["qname"] = serde_json::json!("::caller");
        let resolved = backend.code_lens_resolve(lens).await.unwrap();
        assert_eq!(resolved.range, original);
        let command = resolved.command.unwrap();
        assert_eq!(command.title, "at least 1 reference");
        assert_eq!(command.command, "tcl-lsp.showReferences");
        assert_eq!(resolved.data.unwrap()["sourceReferenceCoverage"], "partial");
    }

    #[tokio::test]
    async fn logical_lens_old_publication_and_legacy_or_unknown_handles_stay_inert() {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/logical-lens-stale.tcl".parse().unwrap();
        register(
            &backend,
            &uri,
            "proc helper {} {}\nproc caller {} {helper}\n",
        )
        .await;
        let lens = helper_lens(&backend, &uri).await;
        let mut legacy = lens.clone();
        legacy.data = Some(serde_json::json!({"qname": "::helper", "uri": uri.as_str()}));
        assert!(
            backend
                .code_lens_resolve(legacy)
                .await
                .unwrap()
                .command
                .is_none()
        );
        let mut unknown = lens.clone();
        unknown.data.as_mut().unwrap()["logicalSourceLens"] =
            serde_json::json!({"schema": 4, "token": 18446744073709551615_u64});
        assert!(
            backend
                .code_lens_resolve(unknown)
                .await
                .unwrap()
                .command
                .is_none()
        );
        register(&backend, &uri, "proc other {} {}\nproc caller {} {other}\n").await;
        assert!(
            backend
                .code_lens_resolve(lens)
                .await
                .unwrap()
                .command
                .is_none()
        );
    }
    #[tokio::test]
    async fn logical_lens_equal_counter_from_another_backend_cannot_select_this_session() {
        let first = crate::tests::test_backend();
        let second = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/logical-lens-session.tcl".parse().unwrap();
        let source = "proc helper {} {}\nproc caller {} {helper}\n";
        register(&first, &uri, source).await;
        register(&second, &uri, source).await;
        let first_lens = helper_lens(&first, &uri).await;
        let second_lens = helper_lens(&second, &uri).await;
        let first_handle = &first_lens.data.as_ref().unwrap()["logicalSourceLens"];
        let second_handle = &second_lens.data.as_ref().unwrap()["logicalSourceLens"];
        assert_eq!(first_handle["schema"], second_handle["schema"]);
        assert_eq!(first_handle["token"], second_handle["token"]);
        assert_ne!(first_handle["session"], second_handle["session"]);
        assert!(
            first
                .code_lens_resolve(first_lens.clone())
                .await
                .unwrap()
                .command
                .is_some()
        );
        assert!(
            second
                .code_lens_resolve(second_lens)
                .await
                .unwrap()
                .command
                .is_some()
        );
        assert!(
            second
                .code_lens_resolve(first_lens)
                .await
                .unwrap()
                .command
                .is_none()
        );
    }
}

#[cfg(test)]
mod logical_call_backend_session_tests {
    use super::*;
    use crate::DocumentState;
    use tower_lsp_server::LanguageServer;

    async fn prepare(
        backend: &Backend,
        uri: &Uri,
        source: &str,
    ) -> tower_lsp_server::ls_types::CallHierarchyItem {
        backend
            .documents
            .lock("logical call Backend session test")
            .await
            .insert(
                uri.clone(),
                DocumentState::new(source.to_owned(), "tcl".to_owned()),
            );
        backend
            .db_set_source(uri, source, "tcl".to_owned())
            .await
            .unwrap();
        let analysis = backend
            .analysis_for(uri, Arc::from(source), "tcl".to_owned())
            .await
            .unwrap();
        backend
            .workspace_index
            .write()
            .await
            .replace_document(uri.as_str(), &analysis);
        let params = serde_json::from_value(
            serde_json::json!({"textDocument":{"uri":uri},"position":{"line":0,"character":5}}),
        )
        .unwrap();
        let mut items = backend
            .prepare_call_hierarchy(params)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(items.len(), 1);
        items.remove(0)
    }

    #[tokio::test]
    async fn fresh_backend_rejects_prior_session_call_on_both_real_endpoints() {
        let first = crate::tests::test_backend();
        let second = crate::tests::test_backend();
        let uri: Uri = "file:///tmp/logical-call-same-session-source.tcl"
            .parse()
            .unwrap();
        let source = "proc caller {} { helper }\nproc helper {} {}\ncaller\n";
        let old = prepare(&first, &uri, source).await;
        let current = prepare(&second, &uri, source).await;
        let old_wire = &old.data.as_ref().unwrap()["logicalCallHierarchy"];
        let current_wire = &current.data.as_ref().unwrap()["logicalCallHierarchy"];
        assert_eq!(old_wire["token"], current_wire["token"]);
        assert_eq!(old_wire["schema"], 3);
        assert_ne!(old_wire["session"], current_wire["session"]);
        // Each backend has a genuine fresh source/publication row and actual
        // incoming/outgoing positives before replaying the old wire on B.
        for (backend, item) in [(&first, &old), (&second, &current)] {
            let params = serde_json::from_value(serde_json::json!({"item":item})).unwrap();
            assert_eq!(
                backend.incoming_calls(params).await.unwrap().unwrap().len(),
                1
            );
            let params = serde_json::from_value(serde_json::json!({"item":item})).unwrap();
            assert_eq!(
                backend.outgoing_calls(params).await.unwrap().unwrap().len(),
                1
            );
        }
        let params = serde_json::from_value(serde_json::json!({"item":old})).unwrap();
        assert!(second.incoming_calls(params).await.unwrap().is_none());
        let params = serde_json::from_value(serde_json::json!({"item":old})).unwrap();
        assert!(second.outgoing_calls(params).await.unwrap().is_none());
    }
}
