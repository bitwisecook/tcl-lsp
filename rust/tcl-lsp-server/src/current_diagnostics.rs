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
//! Source-associated completion of the existing push diagnostic pipeline.

#[cfg(test)]
use super::{
    AnalysisResult, DeliveryCtx, DiagInputs, EvidenceHandles, Path, TrackedRwLock,
    core_workspace_index, next_pull_diag_result_id,
};
use super::{
    Arc, Backend, DiagSlot, DocumentPathAdmission, DocumentPublication, DocumentState, HashMap,
    Mutex, PublishedSourcePath, PullDiagEntry, Uri, WorkspaceDiagnosticSources, is_apl_source,
    jsonrpc,
};
#[cfg(test)]
use super::{
    DiagnosticPublishOutcome, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, FromStr, FullDocumentDiagnosticReport, LanguageServer, PathBuf,
    PublishTiming, TrackedMutexGuard, WorkspaceDiagnosticParams, WorkspaceDiagnosticReportResult,
    WorkspaceDocumentDiagnosticReport, run_diagnostics_core, sync_evaluator_epoch,
    sync_overlay_epoch, vfs,
};
use serde::{Deserialize, Serialize};

mod diagnostic_pass;
mod direct_report;
mod proposal;
mod published_reports;

#[derive(Clone)]
pub(super) struct DiagnosticPass {
    configuration: tcl_lsp_db::DiagnosticConfigurationSnapshot,
    source_file: Option<tcl_lsp_db::SourceFile>,
    path_admission: Arc<DocumentPathAdmission>,
    source: Arc<str>,
    dialect: String,
    revision: u64,
    version: Option<i32>,
    inputs_epoch: u64,
    source_inputs_epoch: Arc<std::sync::atomic::AtomicU64>,
    source_inputs_gate: Arc<tokio::sync::RwLock<()>>,
    source_slots: std::sync::Weak<Mutex<HashMap<Uri, DiagSlot>>>,
    source_factory_generation: Arc<tokio::sync::RwLock<u64>>,
    slot_generation: Option<u64>,
    class_factory_generation: u64,
    input: Option<Arc<tcl_compiler::analyser::ResolvedAnalysisInput>>,
    registry: tcl_registry::RegistrySnapshot,
    /// The document is an F5 model document (BIG-IP configuration or iApp
    /// APL): its report is the F5 model report, which reads no Tcl analysis
    /// input and applies the byte-integrity abstention itself.
    f5_model: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum DiagnosticUnavailable {
    Pending,
    Stale,
    Context,
    ProviderError,
    Cancelled,
    Unsupported,
    NotOpen,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum DiagnosticSuppression {
    Disabled,
    Excluded,
}

enum DiagnosticPassState {
    Unavailable(DiagnosticUnavailable),
    Complete {
        result_id: String,
    },
    Suppressed {
        result_id: String,
        reason: DiagnosticSuppression,
    },
}

pub(super) struct DiagnosticCompletionEntry {
    pass: Arc<DiagnosticPass>,
    state: DiagnosticPassState,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticSourceExpectation {
    uri: Uri,
    version: i32,
    source: String,
}

struct DiagnosticReportExpectation {
    uri: Uri,
    version: Option<i32>,
    source: Arc<str>,
}

/// A validated report for its retained direct-query or acknowledged publisher purpose.
/// Optional version is retained source data, never filled from a later buffer.
pub(super) struct PublishedDiagnosticReport {
    pub(super) uri: Uri,
    pub(super) version: Option<i32>,
    source: Arc<str>,
    dialect: String,
    revision: u64,
    inputs_epoch: u64,
    pub(super) result_id: String,
    pub(super) diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
    suppression: Option<DiagnosticSuppression>,
}

/// Exact source-file/input/publication metadata, without diagnostic completion.
/// Transform results reborrow this existing metadata owner independently of
/// push delivery, diagnostic suppression or the direct-report pipeline.
pub(super) struct CurrentDiagnosticInput {
    file: tcl_lsp_db::SourceFile,
    pub(super) input: Arc<tcl_compiler::analyser::ResolvedAnalysisInput>,
    database_revision: salsa::Revision,
}

#[cfg(test)]
struct CurrentDiagnosticRead<'a> {
    _database: TrackedMutexGuard<'a, tcl_lsp_db::TclDatabase>,
    _files: TrackedMutexGuard<'a, HashMap<Uri, tcl_lsp_db::SourceFile>>,
}

impl CurrentDiagnosticInput {
    pub(super) fn matches_locked(
        &self,
        uri: &Uri,
        admission: &DocumentPathAdmission,
        db: &tcl_lsp_db::TclDatabase,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &HashMap<Uri, PublishedSourcePath>,
    ) -> Result<(), DiagnosticUnavailable> {
        if tcl_lsp_db::database_revision(db) != self.database_revision
            || files.get(uri) != Some(&self.file)
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        let retained = published.get(uri).ok_or(DiagnosticUnavailable::Context)?;
        if retained.file != self.file
            || retained.admission.as_ref() != admission
            || self.file.path(db).as_deref() != admission.analyser_path()
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        Ok(())
    }

    #[cfg(test)]
    fn lock_current<'a>(
        &self,
        backend: &'a Backend,
        uri: &Uri,
        admission: &DocumentPathAdmission,
    ) -> Result<CurrentDiagnosticRead<'a>, DiagnosticUnavailable> {
        let db = backend
            .db
            .try_lock()
            .map_err(|_| DiagnosticUnavailable::Pending)?;
        let files = backend
            .db_files
            .try_lock()
            .map_err(|_| DiagnosticUnavailable::Pending)?;
        let published = backend
            .source_path_admissions
            .lock()
            .map_err(|_| DiagnosticUnavailable::Context)?;
        self.matches_locked(uri, admission, &db, &files, &published)?;
        drop(published);
        Ok(CurrentDiagnosticRead {
            _database: db,
            _files: files,
        })
    }
}

impl DiagnosticCompletionEntry {
    fn current_result(
        &self,
        source: &str,
        version: Option<i32>,
        doc: &DocumentState,
        epoch: u64,
        slot: Option<&DiagSlot>,
    ) -> Result<(&str, Option<DiagnosticSuppression>), DiagnosticUnavailable> {
        self.pass
            .matches_current_source(source, version, doc, epoch, slot)?;
        match &self.state {
            DiagnosticPassState::Unavailable(reason) => Err(*reason),
            DiagnosticPassState::Complete { result_id } => Ok((result_id, None)),
            DiagnosticPassState::Suppressed { result_id, reason } => Ok((result_id, Some(*reason))),
        }
    }
}

#[derive(Serialize)]
struct CurrentDiagnosticReport {
    uri: Uri,
    version: i32,
    source: String,
    #[serde(rename = "dialectId")]
    dialect_id: String,
    revision: String,
    #[serde(rename = "inputsEpoch")]
    inputs_epoch: String,
    #[serde(rename = "resultId")]
    result_id: String,
    diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum CurrentDiagnostics {
    Complete {
        #[serde(flatten)]
        report: CurrentDiagnosticReport,
    },
    Suppressed {
        reason: DiagnosticSuppression,
        #[serde(flatten)]
        report: CurrentDiagnosticReport,
    },
    Unavailable {
        reason: DiagnosticUnavailable,
    },
}

impl Backend {
    /// Read a source-associated result from the actual completed push ledger.
    /// The single argument must retain the open document URI, client version
    /// and complete source. This never schedules analysis or grants execution.
    pub async fn current_diagnostics_command(
        &self,
        arguments: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let [argument] = arguments else {
            return Err(jsonrpc::Error::invalid_params(
                "Expected one diagnostic source expectation",
            ));
        };
        let expected: DiagnosticSourceExpectation = serde_json::from_value(argument.clone())
            .map_err(|_| jsonrpc::Error::invalid_params("Expected {uri, version, source}"))?;
        let answer = self.current_diagnostics(&expected).await?;
        serde_json::to_value(answer)
            .map(Some)
            .map_err(|_| jsonrpc::Error::internal_error())
    }

    #[cfg(test)]
    async fn current_diagnostic_input(
        &self,
        expected: &DiagnosticSourceExpectation,
        admission: &DocumentPathAdmission,
    ) -> Result<CurrentDiagnosticInput, DiagnosticUnavailable> {
        self.current_diagnostic_input_parts(&expected.uri, &expected.source, admission)
            .await
    }

    /// Retain the checked document input's actual command store across
    /// diagnostic metadata computation and pass capture.
    pub(super) async fn diagnostic_registry(
        &self,
        uri: &Uri,
        dialect: &'static tcl_dialect::DialectProfile,
        packs: &tcl_spectcl::pack::PackSet,
    ) -> (Arc<tcl_registry::CommandRegistry>, bool) {
        match self.checked_open_diagnostic_registry(uri).await {
            Ok(Some(registry)) => (registry, true),
            other => (
                Self::fallback_diagnostic_registry(dialect, packs),
                other.is_ok(),
            ),
        }
    }

    /// The dialect and pack registry used when no checked open input applies.
    pub(super) fn fallback_diagnostic_registry(
        dialect: &'static tcl_dialect::DialectProfile,
        packs: &tcl_spectcl::pack::PackSet,
    ) -> Arc<tcl_registry::CommandRegistry> {
        if packs.is_empty() {
            Arc::clone(
                tcl_lsp_core::environment_for_dialect(dialect.name)
                    .default_context_registry()
                    .commands(),
            )
        } else {
            tcl_spectcl::install::registry_for_dialect_with_packs(dialect.name, packs)
        }
    }

    async fn checked_open_diagnostic_registry(
        &self,
        uri: &Uri,
    ) -> Result<Option<Arc<tcl_registry::CommandRegistry>>, DiagnosticUnavailable> {
        let (source, dialect) = {
            let docs = self.documents.lock("diagnostic_registry_source").await;
            let Some(doc) = docs.get(uri) else {
                return Ok(None);
            };
            if Self::is_bigip_dialect(&doc.dialect) || is_apl_source(uri, &doc.language_id) {
                return Ok(None);
            }
            if doc.publication != DocumentPublication::Indexed {
                return Err(DiagnosticUnavailable::Pending);
            }
            (Arc::clone(&doc.text), doc.dialect.clone())
        };
        let admission = self
            .document_path_admission(uri)
            .await
            .map_err(|_| DiagnosticUnavailable::Context)?;
        let current = self
            .current_diagnostic_input_parts(uri, &source, &admission)
            .await?;
        let db = self.db.lock().await;
        let files = self.db_files.lock().await;
        let published = self
            .source_path_admissions
            .lock()
            .map_err(|_| DiagnosticUnavailable::Context)?;
        current.matches_locked(uri, &admission, &db, &files, &published)?;
        if current.file.dialect(&*db) != &dialect {
            return Err(DiagnosticUnavailable::Stale);
        }
        Ok(Some(Arc::clone(
            current.input.borrowed_context_registry().commands(),
        )))
    }

    pub(super) async fn current_diagnostic_input_parts(
        &self,
        uri: &Uri,
        source: &str,
        admission: &DocumentPathAdmission,
    ) -> Result<CurrentDiagnosticInput, DiagnosticUnavailable> {
        let config = self.resolved_db_config(uri).await;
        let db = self.db.lock().await;
        let files = self.db_files.lock().await;
        let file = files
            .get(uri)
            .copied()
            .ok_or(DiagnosticUnavailable::Context)?;
        let published = self
            .source_path_admissions
            .lock()
            .map_err(|_| DiagnosticUnavailable::Context)?;
        let retained = published.get(uri).ok_or(DiagnosticUnavailable::Context)?;
        if retained.file != file
            || retained.admission.as_ref() != admission
            || file.path(&*db).as_deref() != admission.analyser_path()
        {
            return Err(DiagnosticUnavailable::Context);
        }
        drop(published);
        if file.text(&*db) != tcl_lexer::normalise_lone_cr(source).as_ref() {
            return Err(DiagnosticUnavailable::Stale);
        }
        let input = tcl_lsp_db::document_analysis_input(&*db, file, config)
            .map_err(|_| DiagnosticUnavailable::Context)?;
        Ok(CurrentDiagnosticInput {
            file,
            input,
            database_revision: tcl_lsp_db::database_revision(&db),
        })
    }

    async fn current_diagnostics(
        &self,
        expected: &DiagnosticSourceExpectation,
    ) -> jsonrpc::Result<CurrentDiagnostics> {
        // Workspace refinements (W120/W123 against the package database and
        // source graph) are unsettled until the startup scan has published.
        if !self.workspace_scan_ready.is_complete() {
            return Ok(CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Pending,
            });
        }
        let source = DiagnosticReportExpectation {
            uri: expected.uri.clone(),
            version: Some(expected.version),
            source: Arc::from(expected.source.as_str()),
        };
        match self
            .current_published_diagnostic_reports(&[source], DiagnosticCacheQuery::PublisherOnly)
            .await?
        {
            Err(reason) => Ok(CurrentDiagnostics::Unavailable { reason }),
            Ok(mut reports) => {
                let report = reports
                    .pop()
                    .expect("one requested source yields one report");
                let suppression = report.suppression;
                let report = CurrentDiagnosticReport {
                    uri: report.uri,
                    version: expected.version,
                    source: report.source.to_string(),
                    dialect_id: report.dialect.clone(),
                    revision: report.revision.to_string(),
                    inputs_epoch: report.inputs_epoch.to_string(),
                    result_id: report.result_id,
                    diagnostics: report.diagnostics,
                };
                Ok(match suppression {
                    Some(reason) => CurrentDiagnostics::Suppressed { reason, report },
                    None => CurrentDiagnostics::Complete { report },
                })
            }
        }
    }
}

#[derive(Clone, Copy)]
enum DiagnosticCacheQuery<'a> {
    PublisherOnly,
    Reborrow,
    PublishDirect(&'a PullDiagEntry),
}

#[derive(Clone)]
pub(super) struct DiagnosticCacheReceipt {
    workspace_sources: Option<Arc<WorkspaceDiagnosticSources>>,
    source: Arc<DiagnosticPass>,
    origin: DiagnosticCacheOrigin,
}

#[derive(Clone, Copy)]
enum DiagnosticCacheOrigin {
    Publisher,
    Direct {
        policy: Option<DiagnosticSuppression>,
    },
}

impl DiagnosticCacheReceipt {
    pub(super) fn from_publisher(source: &Arc<DiagnosticPass>) -> Self {
        Self {
            workspace_sources: None,
            source: Arc::clone(source),
            origin: DiagnosticCacheOrigin::Publisher,
        }
    }
    pub(super) fn with_workspace_sources(
        mut self,
        sources: Option<Arc<WorkspaceDiagnosticSources>>,
    ) -> Self {
        self.workspace_sources = sources;
        self
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod proposal_endpoint_tests;
