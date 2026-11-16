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

//! Capturing what a readonly answer reads: a source document, a reference document with its analysis, a workspace-symbol analysis and a fresh-source analysis.

use super::super::{Backend, DocumentSourceResult};
use super::snapshot_currency::original_readonly_analysis_matches_input;
use super::{
    FreshSourceAnalysisPurpose, OriginalReadonlyReferenceDocument, ReadonlySourceDocument,
    WorkspaceSymbolAnalysisKind, WorkspaceSymbolFreshInput, WorkspaceSymbolSourceAnalysis,
};
use std::sync::Arc;
use tower_lsp_server::ls_types::Uri;

impl Backend {
    pub(in super::super) async fn capture_readonly_source_document(
        &self,
        owner: &str,
        source: Arc<str>,
    ) -> DocumentSourceResult<Option<ReadonlySourceDocument>> {
        self.capture_readonly_source_document_diagnostic(
            owner,
            source,
            &super::super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    pub(in super::super) async fn capture_readonly_source_document_diagnostic(
        &self,
        owner: &str,
        source: Arc<str>,
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<Option<ReadonlySourceDocument>> {
        let Ok(uri) = owner.parse::<Uri>() else {
            diagnostic.record("source.uri_parse", &[]);
            return Ok(None);
        };
        let Some(document) = self.read_document(&uri).await? else {
            diagnostic.record("source.read_absent", &[]);
            return Ok(None);
        };
        if document.text != source {
            diagnostic.record("source.text_changed", &[]);
            return Ok(None);
        }
        let config = self.resolved_db_config(&uri).await;
        let Some(publication) = self.library_publication_source(&uri, document).await else {
            diagnostic.record("source.publication_unavailable", &[]);
            return Ok(None);
        };
        let (file, configuration, input, external_call_sites, admission, database_revision) = {
            let db = self.db.lock().await;
            let files = self.db_files.lock().await;
            let Some(&file) = files.get(&uri) else {
                diagnostic.record("source.no_db_file", &[]);
                return Ok(None);
            };
            let published = self
                .source_path_admissions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(retained) = published.get(&uri) else {
                diagnostic.record("source.no_retained_path", &[]);
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
                diagnostic.record("source.retained_publication_mismatch", &[]);
                return Ok(None);
            }
            let Ok(input) = tcl_lsp_db::document_analysis_input(&*db, file, config) else {
                diagnostic.record("source.analysis_input_unavailable", &[]);
                return Ok(None);
            };
            (
                file,
                config.diagnostic_report_configuration(&*db),
                input,
                file.external_call_sites(&*db).clone(),
                retained.admission.clone(),
                tcl_lsp_db::database_revision(&db),
            )
        };
        Ok(Some(ReadonlySourceDocument {
            uri,
            source,
            file,
            config,
            configuration,
            input,
            external_call_sites,
            admission,
            publication,
            database_revision,
        }))
    }

    pub(super) async fn capture_readonly_reference_document(
        &self,
        owner: &str,
        source: Arc<str>,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceDocument>> {
        self.capture_readonly_reference_document_diagnostic(
            owner,
            source,
            &super::super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    async fn capture_readonly_reference_document_diagnostic(
        &self,
        owner: &str,
        source: Arc<str>,
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceDocument>> {
        let Some(retained) = self
            .capture_readonly_source_document_diagnostic(owner, source, diagnostic)
            .await?
        else {
            diagnostic.record("metadata.no_source_owner", &[]);
            return Ok(None);
        };
        let ReadonlySourceDocument {
            uri,
            source,
            file,
            config,
            configuration,
            input,
            external_call_sites,
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
        let producer_input = Arc::clone(&input);
        let producer_evidence = external_call_sites.clone();
        let seed = crate::rt::spawn_blocking(move || {
            driver.analyse_readonly_document(
                &producer_source,
                &dialect,
                &producer_input,
                producer_evidence.as_deref(),
            )
        })
        .await
        .map_err(|_| super::super::DocumentSourceUnavailable::Worker { uri: original_uri })??;
        if seed.path_admission.as_ref() != admission.as_ref() {
            diagnostic.record("metadata.path_admission_changed", &[]);
            return Ok(None);
        }
        let current = OriginalReadonlyReferenceDocument {
            uri,
            source,
            analysis: seed.analysis,
            file,
            config,
            configuration,
            input,
            external_call_sites,
            admission,
            publication,
            database_revision,
        };
        let accepted = current.matches_original_input();
        if !accepted && diagnostic.enabled() {
            let analysis = &current.analysis;
            let input = &current.input;
            let config = input.lexer_config();
            diagnostic.record(
                "metadata_match",
                &[
                    usize::from(analysis.resolved_input.as_ref() == Some(input.as_ref())),
                    usize::from(analysis.analysis_context_unavailable.is_none()),
                    usize::from(analysis.body_lexer_config == Some(config)),
                    usize::from(analysis.matches_original_source_image(
                        &tcl_lexer::SourceImage::document(&current.source),
                        config,
                    )),
                    usize::from(
                        analysis
                            .retained_command_realm()
                            .is_some_and(|realm| realm.matches_resolved_analysis_input(input)),
                    ),
                ],
            );
        }
        Ok(accepted.then_some(current))
    }

    /// Preserve the source snapshot's own semantic input. Published database
    /// documents use the existing genuine readonly owner; Pending and genuine
    /// no-handle sources retain their actual fresh driver/seed instead.
    pub(in super::super) async fn capture_workspace_symbol_analysis(
        &self,
        uri: &Uri,
        snapshot: &super::super::DocumentState,
    ) -> DocumentSourceResult<Option<WorkspaceSymbolSourceAnalysis>> {
        self.capture_workspace_symbol_analysis_diagnostic(
            uri,
            snapshot,
            &super::super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    pub(in super::super) async fn capture_workspace_symbol_analysis_diagnostic(
        &self,
        uri: &Uri,
        snapshot: &super::super::DocumentState,
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<Option<WorkspaceSymbolSourceAnalysis>> {
        let snapshot = if snapshot.dialect.is_empty() {
            let Some(current) = self.read_document(uri).await? else {
                diagnostic.record("capture.closed_read_absent", &[]);
                return Ok(None);
            };
            if current.clone().normalised_for_analysis().text != snapshot.text {
                diagnostic.record("capture.closed_text_changed", &[]);
                return Ok(None);
            }
            current.normalised_for_analysis()
        } else {
            snapshot.clone().normalised_for_analysis()
        };
        if snapshot.publication == super::super::DocumentPublication::Indexed
            && self.db_files.lock().await.contains_key(uri)
        {
            let Some(document) = self
                .capture_readonly_reference_document_diagnostic(
                    uri.as_str(),
                    Arc::clone(&snapshot.text),
                    diagnostic,
                )
                .await?
            else {
                diagnostic.record("capture.no_metadata_owner", &[]);
                return Ok(None);
            };
            let accepted = document.publication.document.raw() == snapshot.raw()
                && document.publication.document.dialect == snapshot.dialect;
            if !accepted {
                diagnostic.record("symbol_capture_document_snapshot", &[]);
            }
            return Ok(accepted.then_some(WorkspaceSymbolSourceAnalysis {
                kind: WorkspaceSymbolAnalysisKind::Database(Box::new(document)),
            }));
        }

        self.capture_fresh_source_analysis_diagnostic(
            uri,
            snapshot,
            FreshSourceAnalysisPurpose::ReadonlyComplete,
            diagnostic,
        )
        .await
    }

    // One genuine fresh-source capture/matcher for complete readonly analysis
    // and the independently selected coarse syntax-only worker purpose.
    pub(super) async fn capture_fresh_source_analysis(
        &self,
        uri: &Uri,
        snapshot: super::super::DocumentState,
        purpose: FreshSourceAnalysisPurpose,
    ) -> DocumentSourceResult<Option<WorkspaceSymbolSourceAnalysis>> {
        self.capture_fresh_source_analysis_diagnostic(
            uri,
            snapshot,
            purpose,
            &super::super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    #[allow(clippy::too_many_lines)] // one report, step by step
    async fn capture_fresh_source_analysis_diagnostic(
        &self,
        uri: &Uri,
        snapshot: super::super::DocumentState,
        purpose: FreshSourceAnalysisPurpose,
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<Option<WorkspaceSymbolSourceAnalysis>> {
        let config = self.resolved_db_config(uri).await;
        let (configuration, database_revision) = {
            let db = self.db.lock().await;
            (
                config.diagnostic_report_configuration(&*db),
                tcl_lsp_db::database_revision(&db),
            )
        };
        let was_open = self
            .documents
            .lock("workspace_symbol_source_capture")
            .await
            .contains_key(uri);
        if !was_open && matches!(purpose, FreshSourceAnalysisPurpose::SemanticStructure) {
            // A path receipt authenticates the selected provider address, not
            // unchanged closed-file bytes. This new publication purpose needs
            // the actual retained open-document source owner.
            diagnostic.record("fresh.structure_without_open", &[]);
            return Ok(None);
        }
        let uri_generation = self.live_publication_uri_generation(uri);
        let seed = match purpose {
            FreshSourceAnalysisPurpose::ConditionalSourceRegion => return Ok(None),
            FreshSourceAnalysisPurpose::ReadonlyComplete => {
                self.fresh_analysis_seed(uri, Arc::clone(&snapshot.text), snapshot.dialect.clone())
                    .await?
            }
            FreshSourceAnalysisPurpose::SemanticStructure => {
                let Some(admission) = snapshot.path_admission.clone() else {
                    diagnostic.record("fresh.structure_without_path", &[]);
                    return Ok(None);
                };

                admission.check_for(uri, self.store.as_ref())?;
                let sidecar_revision = self
                    .fresh_analysis_cache
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .revision();
                let mut driver = self
                    .fresh_analysis_driver(admission)
                    .await
                    .into_source_shell();
                // Every capture attempt of every coarse token request used to
                // re-analyse the same text; key it exactly as the seed cache.
                let key = super::super::fresh_analysis_cache::FreshAnalysisKey {
                    text: Arc::clone(&snapshot.text),
                    dialect: snapshot.dialect.clone(),
                    path_admission: Arc::clone(&driver.path_admission),
                    analyser_inputs_epoch: driver.analyser_inputs_epoch,
                    class_factory_generation: driver.class_factory_generation,
                    inputs: driver.inputs.clone(),
                    resolved_input: driver
                        .analyser
                        .prepare_analysis_input(&snapshot.dialect)
                        .ok(),
                };
                let cached = key.resolved_input.as_ref().and_then(|_| {
                    self.fresh_analysis_cache
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get_structure(uri, &key)
                });
                if let Some(seed) = cached {
                    seed
                } else {
                    let text = Arc::clone(&snapshot.text);
                    let dialect = snapshot.dialect.clone();
                    let original_uri = uri.clone();
                    let seed =
                        crate::rt::spawn_blocking(move || driver.analyse(&text, &dialect, None))
                            .await
                            .map_err(|_| super::super::DocumentSourceUnavailable::Worker {
                                uri: original_uri,
                            })??;
                    let mut cache = self
                        .fresh_analysis_cache
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if key.resolved_input.is_some() && cache.is_current(&sidecar_revision) {
                        cache.insert_structure(uri, key, seed.clone());
                    }
                    seed
                }
            }
        };
        let Some(input) = seed.analysis.resolved_input.clone().map(Arc::new) else {
            diagnostic.record("fresh.no_resolved_input", &[]);
            return Ok(None);
        };
        let matches_input =
            original_readonly_analysis_matches_input(&snapshot.text, &seed.analysis, &input);
        if !matches_input {
            if diagnostic.enabled() {
                diagnostic.record(
                    "fresh_match",
                    &[
                        usize::from(seed.analysis.resolved_input.as_ref() == Some(input.as_ref())),
                        usize::from(seed.analysis.analysis_context_unavailable.is_none()),
                        usize::from(seed.analysis.body_lexer_config == Some(input.lexer_config())),
                        usize::from(seed.analysis.matches_original_source_image(
                            &tcl_lexer::SourceImage::document(&snapshot.text),
                            input.lexer_config(),
                        )),
                        usize::from(
                            seed.analysis
                                .retained_command_realm()
                                .is_some_and(|realm| realm.matches_resolved_analysis_input(&input)),
                        ),
                    ],
                );
            }
            diagnostic.record("fresh.source_input_mismatch", &[]);
            return Ok(None);
        }
        Ok(Some(WorkspaceSymbolSourceAnalysis {
            kind: WorkspaceSymbolAnalysisKind::Fresh(Box::new(WorkspaceSymbolFreshInput {
                purpose,
                uri: uri.clone(),
                snapshot,
                was_open,
                seed,
                input,
                config,
                configuration,
                database_revision,
                uri_generation,
            })),
        }))
    }
}
