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

//! Whether a captured document, analysis or fresh input still describes the current snapshot.

use super::super::{DocumentPathAdmission, LibraryPublicationSource};
use super::{
    FreshSourceAnalysisPurpose, OriginalReadonlyReferenceDocument, ReadonlySourceDocument,
    ReadonlySourceState, WorkspaceSymbolAnalysisKind, WorkspaceSymbolFreshInput,
    WorkspaceSymbolSourceAnalysis,
};
use tcl_compiler::analyser::{AnalysisResult, ResolvedAnalysisInput};
use tower_lsp_server::ls_types::Uri;

impl OriginalReadonlyReferenceDocument {
    pub(super) fn matches_current_snapshot(
        &self,
        uri_generations: &std::collections::HashMap<Uri, u64>,
        open: &std::collections::HashMap<Uri, super::super::DocumentState>,
        db: &tcl_lsp_db::TclDatabase,
        files: &std::collections::HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &std::collections::HashMap<Uri, super::super::PublishedSourcePath>,
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
            configuration: self.configuration.clone(),
            input: self.input.clone(),
            external_call_sites: self.external_call_sites.clone(),
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

    pub(super) fn matches_original_input(&self) -> bool {
        original_readonly_analysis_matches_input(&self.source, &self.analysis, &self.input)
    }

    /// Workspace-symbol snapshot validity (#1854). The Salsa handle, provider
    /// receipt, path, configuration values and analysis input must be the
    /// captured ones; the database revision need not, so an unrelated write
    /// or a later edit of this buffer keeps the captured answer. An unchanged
    /// buffer, or a closed file, must still be the exact captured publication.
    pub(super) fn matches_request_snapshot(
        &self,
        state: &ReadonlySourceState<'_>,
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
    ) -> bool {
        let current = state.open.get(&self.uri);
        let advanced = self.publication.open_revision.is_some()
            && current
                .is_some_and(|current| open_document_advanced(&self.publication.document, current));
        let publication_matches = if advanced {
            current.is_some_and(|current| !current.backing_file_deleted)
        } else {
            self.publication.matches_open_documents(state.open)
                && state
                    .uri_generations
                    .get(&self.uri)
                    .copied()
                    .unwrap_or_default()
                    == self.publication.uri_generation
                && self.file.text(state.db).as_str() == self.source.as_ref()
                && self.file.dialect(state.db) == &self.publication.document.dialect
        };
        diagnostic.check("lease.db.publication", publication_matches)
            && diagnostic.check("lease.db.original_input", self.matches_original_input())
            && diagnostic.check(
                "lease.db.file",
                state.files.get(&self.uri) == Some(&self.file),
            )
            && diagnostic.check(
                "lease.db.path_receipt",
                state.published.get(&self.uri).is_some_and(|retained| {
                    retained.file == self.file
                        && retained.admission.as_ref() == self.admission.as_ref()
                }),
            )
            && diagnostic.check(
                "lease.db.path",
                self.file.path(state.db).as_deref() == self.admission.analyser_path(),
            )
            && diagnostic.check(
                "lease.db.configuration",
                self.config.diagnostic_report_configuration(state.db) == self.configuration,
            )
            && diagnostic.check(
                "lease.db.analysis_input",
                { tcl_lsp_db::document_analysis_input(state.db, self.file, self.config) }
                    .is_ok_and(|input| input.as_ref() == self.input.as_ref()),
            )
    }
}

/// Whether `live` is a later client version of the open buffer `captured`
/// named, without its local revision going backwards. Such a request answers
/// from its captured revision (#1854); equal or regressed versions must still
/// be that exact revision. A close and reopen at a higher version is accepted
/// when the captured revision was 0, since reopening restarts at revision 0:
/// the captured text and its own analysis still answer together, and the
/// provider receipt is checked independently. A closed capture has no client
/// version, so it never advances: a changed disk read or index revision for
/// it withdraws the row deliberately.
pub(super) fn open_document_advanced(
    captured: &super::super::DocumentState,
    live: &super::super::DocumentState,
) -> bool {
    matches!(
        (captured.version, live.version),
        (Some(captured_version), Some(live_version)) if live_version > captured_version
    ) && live.revision >= captured.revision
}

pub(super) fn original_readonly_analysis_matches_input(
    source: &str,
    analysis: &AnalysisResult,
    input: &ResolvedAnalysisInput,
) -> bool {
    let config = input.lexer_config();
    analysis.resolved_input.as_ref() == Some(input)
        && analysis.analysis_context_unavailable.is_none()
        && analysis.body_lexer_config == Some(config)
        && analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        && analysis
            .retained_command_realm()
            .is_some_and(|realm| realm.matches_resolved_analysis_input(input))
}

impl WorkspaceSymbolSourceAnalysis {
    pub(in super::super) fn analysis(&self) -> &AnalysisResult {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => &document.analysis,
            WorkspaceSymbolAnalysisKind::Fresh(document) => &document.seed.analysis,
        }
    }

    pub(super) fn matches_snapshot(
        &self,
        uri: &Uri,
        snapshot: &super::super::DocumentState,
    ) -> bool {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => {
                &document.uri == uri
                    && document.source == snapshot.text
                    && document.publication.document.raw() == snapshot.raw()
            }
            WorkspaceSymbolAnalysisKind::Fresh(document) => {
                &document.uri == uri
                    && document.snapshot.text == snapshot.text
                    && document.snapshot.raw() == snapshot.raw()
            }
        }
    }

    pub(super) fn admission(&self) -> &DocumentPathAdmission {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => &document.admission,
            WorkspaceSymbolAnalysisKind::Fresh(document) => &document.seed.path_admission,
        }
    }

    pub(super) fn matches_current_state(
        &self,
        state: &ReadonlySourceState<'_>,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> bool {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => document.matches_current_snapshot(
                state.uri_generations,
                state.open,
                state.db,
                state.files,
                state.published,
            ),
            WorkspaceSymbolAnalysisKind::Fresh(document) => {
                let open_matches = match state.open.get(&document.uri) {
                    Some(current) if document.was_open => document.open_snapshot_matches(current),
                    None => !document.was_open,
                    Some(_) => false,
                };
                open_matches
                    && document.uri_generation_matches(state)
                    // A fresh syntax worker retains no Salsa query result.
                    && (matches!(document.purpose, FreshSourceAnalysisPurpose::SemanticStructure)
                        || tcl_lsp_db::database_revision(state.db) == document.database_revision)
                    && document.matches_input_state(state, inputs_epoch, factory_generation)
            }
        }
    }

    /// Validity of this request's captured snapshot (#1854). Unlike
    /// [`Self::matches_current_state`], a buffer whose client version advanced
    /// after capture keeps its captured analysis, and unrelated database writes
    /// do not withdraw it. A replaced, closed or deleted captured revision, and
    /// any configuration, factory, epoch, provider or input change, still do.
    pub(super) fn matches_request_snapshot(
        &self,
        state: &ReadonlySourceState<'_>,
        inputs_epoch: u64,
        factory_generation: u64,
        diagnostic: &super::super::WorkspaceSymbolDiagnostic,
    ) -> bool {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => {
                document.matches_request_snapshot(state, diagnostic)
            }
            WorkspaceSymbolAnalysisKind::Fresh(document) => {
                let current = state.open.get(&document.uri);
                let advanced = document.was_open
                    && matches!(
                        document.purpose,
                        FreshSourceAnalysisPurpose::ReadonlyComplete
                    )
                    && current
                        .is_some_and(|current| open_document_advanced(&document.snapshot, current));
                let open_matches = match current {
                    Some(current) if advanced => !current.backing_file_deleted,
                    Some(current) if document.was_open => document.open_snapshot_matches(current),
                    None => !document.was_open,
                    Some(_) => false,
                };
                diagnostic.check("lease.fresh.open_snapshot", open_matches)
                    && diagnostic.check(
                        "lease.fresh.uri_generation",
                        advanced || document.uri_generation_matches(state),
                    )
                    && diagnostic.check(
                        "lease.fresh.inputs",
                        document.matches_input_state(state, inputs_epoch, factory_generation),
                    )
            }
        }
    }
}

impl WorkspaceSymbolFreshInput {
    /// Eligibility for discarding old data and recapturing, never publication.
    pub(super) fn recompute_snapshot_matches(&self, current: &super::super::DocumentState) -> bool {
        matches!(self.purpose, FreshSourceAnalysisPurpose::SemanticStructure)
            && current.backing_file_deleted == self.snapshot.backing_file_deleted
            && current.revision == self.snapshot.revision
            && current.version == self.snapshot.version
            && current.raw() == self.snapshot.raw()
            && current.dialect == self.snapshot.dialect
            && (current.publication == self.snapshot.publication
                || (self.snapshot.publication == super::super::DocumentPublication::Salsa
                    && current.publication == super::super::DocumentPublication::Indexed))
            && current.path_admission == self.snapshot.path_admission
            && !current.dialect_resolution_pending()
    }

    pub(super) fn open_snapshot_matches(&self, current: &super::super::DocumentState) -> bool {
        (match self.purpose {
            FreshSourceAnalysisPurpose::ReadonlyComplete
            | FreshSourceAnalysisPurpose::ConditionalSourceRegion => !current.backing_file_deleted,
            FreshSourceAnalysisPurpose::SemanticStructure => {
                current.backing_file_deleted == self.snapshot.backing_file_deleted
            }
        }) && current.revision == self.snapshot.revision
            && current.version == self.snapshot.version
            && current.raw() == self.snapshot.raw()
            && current.dialect == self.snapshot.dialect
            && (current.publication == self.snapshot.publication
                || (matches!(self.purpose, FreshSourceAnalysisPurpose::SemanticStructure)
                    && self.snapshot.publication == super::super::DocumentPublication::Salsa
                    && current.publication == super::super::DocumentPublication::Indexed))
            && current.path_admission == self.snapshot.path_admission
            && current.dialect_resolution_generation == self.snapshot.dialect_resolution_generation
            && current.resolved_dialect_generation == self.snapshot.resolved_dialect_generation
    }

    pub(super) fn uri_generation_matches(&self, state: &ReadonlySourceState<'_>) -> bool {
        self.uri_generation
            == state
                .uri_generations
                .get(&self.uri)
                .copied()
                .unwrap_or_default()
    }

    /// The analysis still describes its own captured source under the
    /// request's analyser inputs, configuration and provider receipt.
    pub(super) fn matches_input_state(
        &self,
        state: &ReadonlySourceState<'_>,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> bool {
        self.seed.analyser_inputs_epoch == inputs_epoch
            && self.seed.class_factory_generation == factory_generation
            && self.config.diagnostic_report_configuration(state.db) == self.configuration
            && self.seed.path_admission.uri() == &self.uri
            && original_readonly_analysis_matches_input(
                &self.snapshot.text,
                &self.seed.analysis,
                &self.input,
            )
    }
}

impl ReadonlySourceDocument {
    pub(super) fn matches_current_snapshot(
        &self,
        uri_generations: &std::collections::HashMap<Uri, u64>,
        open: &std::collections::HashMap<Uri, super::super::DocumentState>,
        db: &tcl_lsp_db::TclDatabase,
        files: &std::collections::HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &std::collections::HashMap<Uri, super::super::PublishedSourcePath>,
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
            && self.file.external_call_sites(db) == &self.external_call_sites
            && self.file.dialect(db) == &self.publication.document.dialect
            && tcl_lsp_db::document_analysis_input(db, self.file, self.config)
                .is_ok_and(|input| input.as_ref() == self.input.as_ref())
    }
}
