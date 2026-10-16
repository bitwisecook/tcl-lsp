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

use super::*;
use serde::{Deserialize, Serialize};

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
    slot_generation: Option<u64>,
    class_factory_generation: u64,
    input: Option<Arc<tcl_compiler::analyser::ResolvedAnalysisInput>>,
    registry: tcl_registry::RegistrySnapshot,
}

#[derive(Clone, Copy, Serialize)]
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
    revision: u64,
    inputs_epoch: u64,
    pub(super) result_id: String,
    pub(super) diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
    suppression: Option<DiagnosticSuppression>,
}

struct CurrentDiagnosticInput {
    file: tcl_lsp_db::SourceFile,
    input: Arc<tcl_compiler::analyser::ResolvedAnalysisInput>,
    database_revision: salsa::Revision,
}

#[cfg(test)]
struct CurrentDiagnosticRead<'a> {
    _database: TrackedMutexGuard<'a, tcl_lsp_db::TclDatabase>,
    _files: TrackedMutexGuard<'a, HashMap<Uri, tcl_lsp_db::SourceFile>>,
}

impl CurrentDiagnosticInput {
    fn matches_locked(
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

impl DiagInputs {
    pub(super) async fn capture_diagnostic_pass(
        &self,
        uri: &Uri,
        job: &DiagJob,
    ) -> Option<Arc<DiagnosticPass>> {
        let DiagCurrency::Open(revision) = job.currency else {
            return None;
        };
        let epoch = self.inputs_epoch?;
        // Retain the original publication, rather than recapturing a mapping
        // that might have replaced the provider while this pass was running.
        let path_admission = if let Some(file) = job.file {
            let retained = self.source_path_admissions.lock().ok()?.get(uri)?.clone();
            if retained.file != file {
                return None;
            }
            retained.admission
        } else {
            self.documents
                .lock("diagnostic_pass_source_owner")
                .await
                .get(uri)?
                .path_admission
                .clone()?
        };
        path_admission.check_for(uri, self.store.as_ref()).ok()?;
        let generation = *self.class_factory_generation.read().await;
        let db = self.db.lock().await;
        let configuration = job.config.diagnostic_report_configuration(&*db);
        let input = if let Some(file) = job.file {
            if file.text(&*db) != tcl_lexer::normalise_lone_cr(&job.text).as_ref()
                || file.dialect(&*db) != &job.dialect
            {
                None
            } else {
                with_pack_hooks(|| tcl_lsp_db::document_analysis_input(&*db, file, job.config)).ok()
            }
        } else {
            None
        };
        drop(db);
        let slot_generation = self
            .diag_slots
            .lock()
            .await
            .get(uri)
            .map(|slot| slot.generation);
        Some(Arc::new(DiagnosticPass {
            configuration,
            source_file: job.file,
            path_admission,
            source: Arc::clone(&job.text),
            dialect: job.dialect.clone(),
            revision,
            version: job.version,
            inputs_epoch: epoch,
            slot_generation,
            class_factory_generation: generation,
            input,
            registry: self.registry.snapshot(),
        }))
    }
}

impl DiagnosticPublisher {
    pub(super) async fn begin_diagnostic_pass(&self, uri: &Uri, pass: &Arc<DiagnosticPass>) {
        self.completion.lock().await.insert(
            uri.clone(),
            DiagnosticCompletionEntry {
                pass: Arc::clone(pass),
                state: DiagnosticPassState::Unavailable(DiagnosticUnavailable::Pending),
            },
        );
    }

    async fn finish_diagnostic_pass(
        &self,
        uri: &Uri,
        pass: &Arc<DiagnosticPass>,
        state: DiagnosticPassState,
    ) {
        let mut ledger = self.completion.lock().await;
        if let Some(entry) = ledger.get_mut(uri)
            && Arc::ptr_eq(&entry.pass, pass)
        {
            entry.state = state;
        }
    }
}

impl DeliveryCtx<'_> {
    pub(super) async fn deliver_suppressed_if_current(
        &self,
        reason: DiagnosticSuppression,
    ) -> bool {
        let Some(result_id) = self.delivered_current_report(Vec::new()).await else {
            return false;
        };
        if let Some(pass) = self.diagnostic_pass {
            self.diagnostic_publisher
                .finish_diagnostic_pass(
                    self.uri,
                    pass,
                    DiagnosticPassState::Suppressed { result_id, reason },
                )
                .await;
        }
        true
    }

    pub(super) async fn diagnostic_pass_unavailable(&self, reason: DiagnosticUnavailable) {
        if let Some(pass) = self.diagnostic_pass {
            self.diagnostic_publisher
                .finish_diagnostic_pass(self.uri, pass, DiagnosticPassState::Unavailable(reason))
                .await;
        }
    }

    pub(super) async fn complete_diagnostic_pass(
        &self,
        analysis: &AnalysisResult,
        result_id: String,
    ) {
        let Some(pass) = self.diagnostic_pass else {
            return;
        };
        if !pass.matches_completed_analysis(analysis) {
            self.diagnostic_pass_unavailable(DiagnosticUnavailable::Context)
                .await;
            return;
        }
        self.diagnostic_publisher
            .finish_diagnostic_pass(self.uri, pass, DiagnosticPassState::Complete { result_id })
            .await;
    }
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

    async fn current_diagnostic_input_parts(
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
        let input = with_pack_hooks(|| tcl_lsp_db::document_analysis_input(&*db, file, config))
            .map_err(|_| DiagnosticUnavailable::Context)?;
        Ok(CurrentDiagnosticInput {
            file,
            input,
            database_revision: tcl_lsp_db::database_revision(&db),
        })
    }

    pub(super) fn diagnostic_report_unavailable(reason: DiagnosticUnavailable) -> jsonrpc::Error {
        jsonrpc::Error {
            code: jsonrpc::ErrorCode::ContentModified,
            message: "Current diagnostic report is unavailable".into(),
            data: Some(serde_json::json!({ "reason": reason })),
        }
    }

    /// The direct response has its own actual provider receipt. It cannot
    /// replace a publisher completion or borrow a delivery acknowledgement.
    pub(super) async fn ensure_current_diagnostic_report(
        &self,
        uri: &Uri,
    ) -> jsonrpc::Result<PublishedDiagnosticReport> {
        if let Ok(mut reports) = self
            .current_cached_diagnostic_reports(std::slice::from_ref(uri))
            .await?
        {
            return Ok(reports.remove(0));
        }
        let produced = self
            .produce_direct_diagnostic_report(uri)
            .await
            .map_err(Self::diagnostic_report_unavailable)?;
        let source = &produced.receipt.source;
        let expected = DiagnosticReportExpectation {
            uri: uri.clone(),
            version: source.version,
            source: Arc::clone(&source.source),
        };
        let candidate = PullDiagEntry {
            result_id: next_pull_diag_result_id(),
            revision: source.revision,
            diagnostics: produced.diagnostics,
            currency: Some(produced.receipt),
        };
        match self
            .current_published_diagnostic_reports(
                &[expected],
                DiagnosticCacheQuery::PublishDirect(&candidate),
            )
            .await?
        {
            Ok(mut reports) => Ok(reports.remove(0)),
            Err(reason) => Err(Self::diagnostic_report_unavailable(reason)),
        }
    }
    /// Reborrow acknowledged publisher reports for their actual current sources.
    /// An unavailable participant refuses the batch rather than donating absence.
    pub(super) async fn current_cached_diagnostic_reports(
        &self,
        uris: &[Uri],
    ) -> jsonrpc::Result<Result<Vec<PublishedDiagnosticReport>, DiagnosticUnavailable>> {
        let expected = {
            let docs = self.documents.lock("cached_diagnostic_sources").await;
            let mut expected = Vec::with_capacity(uris.len());
            for uri in uris {
                let Some(doc) = docs.get(uri) else {
                    return Ok(Err(DiagnosticUnavailable::NotOpen));
                };
                expected.push(DiagnosticReportExpectation {
                    uri: uri.clone(),
                    version: doc.version,
                    source: Arc::from(doc.raw()),
                });
            }
            expected
        };
        self.current_published_diagnostic_reports(&expected, DiagnosticCacheQuery::Reborrow)
            .await
    }

    async fn current_published_diagnostic_reports(
        &self,
        expected: &[DiagnosticReportExpectation],
        purpose: DiagnosticCacheQuery<'_>,
    ) -> jsonrpc::Result<Result<Vec<PublishedDiagnosticReport>, DiagnosticUnavailable>> {
        if matches!(purpose, DiagnosticCacheQuery::PublishDirect(_)) && expected.len() != 1 {
            return Ok(Err(DiagnosticUnavailable::Context));
        }
        let mut admissions = Vec::with_capacity(expected.len());
        for source in expected {
            admissions.push(self.document_path_admission(&source.uri).await?);
        }
        let _inputs = self.analyser_inputs_gate.read().await;
        let epoch = self.diag_inputs_epoch();
        let factory_generation = self.class_factory_generation.read().await;
        let mut current = Vec::with_capacity(expected.len());
        let mut configurations = Vec::with_capacity(expected.len());
        for (source, admission) in expected.iter().zip(&admissions) {
            let config = self.resolved_db_config(&source.uri).await;
            configurations.push(config);
            current.push(
                self.current_diagnostic_input_parts(&source.uri, &source.source, admission)
                    .await,
            );
        }
        let docs = self.documents.lock("current_diagnostic_reports").await;
        let slots = self.diag_slots.lock().await;
        let ledger = self.diagnostic_publisher.completion.lock().await;
        let mut cache = self.pull_diag_cache.lock().await;
        let mut reports = Vec::with_capacity(expected.len());
        let mut checked_inputs = Vec::new();
        let mut checked_configurations = Vec::with_capacity(expected.len());
        for (((source, admission), current), configuration) in expected
            .iter()
            .zip(&admissions)
            .zip(&current)
            .zip(&configurations)
        {
            let Some(doc) = docs.get(&source.uri) else {
                return Ok(Err(DiagnosticUnavailable::NotOpen));
            };
            if doc.version != source.version || doc.raw() != source.source.as_ref() {
                return Ok(Err(DiagnosticUnavailable::Stale));
            }
            if tcl_lsp_core::source_decode::should_abstain(doc.decode_report.as_ref()) {
                return Ok(Err(DiagnosticUnavailable::Context));
            }
            if doc.publication != DocumentPublication::Indexed || self.diag_inputs_epoch() != epoch
            {
                return Ok(Err(DiagnosticUnavailable::Pending));
            }
            let report = match purpose {
                DiagnosticCacheQuery::PublishDirect(candidate) => candidate,
                _ => match cache.get(&source.uri) {
                    Some(report) => report,
                    None => return Ok(Err(DiagnosticUnavailable::Pending)),
                },
            };
            let Some(receipt) = report.currency.as_ref() else {
                return Ok(Err(DiagnosticUnavailable::Stale));
            };
            let pass = &receipt.source;
            let suppression = match receipt.origin {
                DiagnosticCacheOrigin::Publisher => {
                    let Some(entry) = ledger.get(&source.uri) else {
                        return Ok(Err(DiagnosticUnavailable::Pending));
                    };
                    let (result_id, suppression) = match entry.current_result(
                        &source.source,
                        source.version,
                        doc,
                        epoch,
                        slots.get(&source.uri),
                    ) {
                        Ok(result) => result,
                        Err(reason) => return Ok(Err(reason)),
                    };
                    // The authentic current publisher pass owns unavailable
                    // outcomes before any older cached report is considered.
                    // Success still requires this exact pass and issued ID.
                    if !Arc::ptr_eq(&entry.pass, pass) || result_id != report.result_id {
                        return Ok(Err(DiagnosticUnavailable::Stale));
                    }
                    suppression
                }
                DiagnosticCacheOrigin::Direct { policy }
                    if !matches!(purpose, DiagnosticCacheQuery::PublisherOnly) =>
                {
                    if let Err(reason) = pass.matches_current_source(
                        &source.source,
                        source.version,
                        doc,
                        epoch,
                        slots.get(&source.uri),
                    ) {
                        return Ok(Err(reason));
                    }
                    policy
                }
                DiagnosticCacheOrigin::Direct { .. } => {
                    return Ok(Err(DiagnosticUnavailable::Stale));
                }
            };
            if report.revision != pass.revision
                || pass.path_admission.as_ref() != admission.as_ref()
            {
                return Ok(Err(DiagnosticUnavailable::Stale));
            }
            checked_configurations.push((*configuration, pass.configuration.clone()));
            // A delivered policy clear does not claim analysis preparation.
            if suppression.is_none() {
                let current = match current {
                    Ok(current) => current,
                    Err(reason) => return Ok(Err(*reason)),
                };
                if pass.source_file != Some(current.file)
                    || pass.input.as_deref() != Some(current.input.as_ref())
                    || pass.class_factory_generation != *factory_generation
                {
                    return Ok(Err(DiagnosticUnavailable::Stale));
                }
                checked_inputs.push((current, source, admission));
            }
            reports.push(PublishedDiagnosticReport {
                uri: source.uri.clone(),
                version: pass.version,
                source: Arc::clone(&pass.source),
                revision: pass.revision,
                inputs_epoch: epoch,
                result_id: report.result_id.clone(),
                diagnostics: report.diagnostics.clone(),
                suppression,
            });
        }
        // No participant can change its SourceFile/input while the complete
        // batch is projected. Never wait for DB writers under documents.
        let guards = if expected.is_empty() {
            None
        } else {
            let Ok(db) = self.db.try_lock() else {
                return Ok(Err(DiagnosticUnavailable::Pending));
            };
            // Configuration values are checked under this same final database
            // guard, including policy clears that require no prepared analysis.
            // A prior snapshot or editing-input equality cannot cover a later
            // diagnostic-only option mutation such as declared target ranges.
            for (config, retained) in checked_configurations {
                if config.diagnostic_report_configuration(&*db) != retained {
                    return Ok(Err(DiagnosticUnavailable::Stale));
                }
            }
            let Ok(files) = self.db_files.try_lock() else {
                return Ok(Err(DiagnosticUnavailable::Pending));
            };
            let published = self
                .source_path_admissions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for (current, source, admission) in checked_inputs {
                if let Err(reason) =
                    current.matches_locked(&source.uri, admission, &db, &files, &published)
                {
                    return Ok(Err(reason));
                }
            }
            drop(published);
            Some((db, files))
        };
        let admissions = admissions.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        let mut reports = Some(reports);
        let result = DocumentPathAdmission::with_current(&admissions, self.store.as_ref(), || {
            let Some(reports) = reports.take() else {
                return Err(DiagnosticUnavailable::Stale);
            };
            if let DiagnosticCacheQuery::PublishDirect(candidate) = purpose {
                cache.insert(expected[0].uri.clone(), candidate.clone());
            }
            Ok(reports)
        })?;
        drop(guards);
        Ok(result)
    }

    async fn current_diagnostics(
        &self,
        expected: &DiagnosticSourceExpectation,
    ) -> jsonrpc::Result<CurrentDiagnostics> {
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
            source: Arc::clone(source),
            origin: DiagnosticCacheOrigin::Publisher,
        }
    }
}

struct ProducedDirectDiagnosticReport {
    receipt: DiagnosticCacheReceipt,
    diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
}

impl DiagnosticPass {
    fn matches_current_source(
        &self,
        source: &str,
        version: Option<i32>,
        doc: &DocumentState,
        epoch: u64,
        slot: Option<&DiagSlot>,
    ) -> Result<(), DiagnosticUnavailable> {
        if self.source.as_ref() != source
            || self.version != version
            || self.revision != doc.revision
            || self.dialect != doc.dialect
            || self.inputs_epoch != epoch
            || self.slot_generation != slot.map(|slot| slot.generation)
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        if slot.is_some_and(|slot| slot.dirty) {
            return Err(DiagnosticUnavailable::Pending);
        }
        Ok(())
    }

    fn matches_completed_analysis(&self, analysis: &AnalysisResult) -> bool {
        let Some(input) = self.input.as_ref() else {
            return false;
        };
        let source = tcl_lexer::normalise_lone_cr(&self.source);
        tcl_compiler::source_graph::current_analysis(&source, analysis).is_some()
            && analysis.resolved_input.as_ref() == Some(input.as_ref())
            && analysis.resolved_registry().is_some_and(|registry| {
                registry.snapshot().semantic_key() == self.registry.semantic_key()
            })
    }
}

impl Backend {
    /// Execute the existing checked provider computation for the direct report
    /// purpose. This never enqueues a push or issues a Delivered receipt.
    async fn produce_direct_diagnostic_report(
        &self,
        uri: &Uri,
    ) -> Result<ProducedDirectDiagnosticReport, DiagnosticUnavailable> {
        let dialect = {
            let docs = self.documents.lock("direct_diagnostic_source").await;
            docs.get(uri)
                .ok_or(DiagnosticUnavailable::NotOpen)?
                .dialect
                .clone()
        };
        let inputs = self
            .diag_inputs(uri, tcl_lsp_core::profile_for_dialect(&dialect))
            .await;
        let job = inputs
            .capture_job(uri)
            .await
            .ok_or(DiagnosticUnavailable::Pending)?;
        let source = job
            .diagnostic_pass
            .as_ref()
            .ok_or(DiagnosticUnavailable::Context)?;
        let policy = if !inputs.toggles.diagnostics_enabled {
            Some(DiagnosticSuppression::Disabled)
        } else if inputs.toggles.excluded {
            Some(DiagnosticSuppression::Excluded)
        } else {
            None
        };
        let diagnostics = if policy.is_some() {
            Vec::new()
        } else {
            if Self::is_bigip_dialect(&job.dialect) || is_apl_source(uri, &job.language_id) {
                return Err(DiagnosticUnavailable::Unsupported);
            }
            let analysis_text = tcl_lexer::normalise_lone_cr(&job.text);
            let profile = tcl_lsp_core::profile_for_dialect(&job.dialect);
            let salsa_ctx = SalsaAnalysisCtx {
                db: &inputs.db,
                uri,
                file: job.file,
                config: job.config,
                text: &analysis_text,
                dialect: profile,
            };
            let lift = LiftInputs {
                text: &job.text,
                decode_report: job.decode_report,
                dialect: profile,
                disabled: &inputs.disabled,
                policy_layers: &inputs.policy_layers,
                style_line_length: inputs.style_line_length,
                xc_diagnostics: inputs.toggles.xc.xc_diagnostics,
            };
            let path = AnalyserPathInputs {
                registry: Arc::clone(&inputs.registry),
                extra_commands: &inputs.extra_commands,
                generic_variable_patterns: inputs.generic_variable_patterns.as_deref(),
                non_ascii_mode: inputs.non_ascii_mode,
                db_project: &inputs.db_project,
                workspace_index: &inputs.workspace_index,
                rehomed_source_seeds: &inputs.rehomed_source_seeds,
                rehoming_gate: &inputs.rehoming_gate,
                package_resolver: &inputs.package_resolver,
                package_prefer: inputs.package_prefer,
                provider_capture: provider_source::ProviderCaptureInputs {
                    db: &inputs.db,
                    files: &inputs.db_files,
                    global: &inputs.db_config,
                    folders: &inputs.folder_db_configs,
                    store: &inputs.store,
                    dialects: &inputs.provider_dialects,
                },
                recovery_names: &inputs.recovery_names,
                entry_points: &inputs.entry_points,
                folder_root: inputs.folder_root.as_deref(),
                cross_file_resolution: inputs.toggles.xc.cross_file_resolution,
            };
            let recovery = RecoveryWidenCtx {
                cache: path.recovery_names,
                registry: &path.registry,
                workspace_index: path.workspace_index,
                package_resolver: path.package_resolver,
                provider_capture: &path.provider_capture,
                prefer: path.package_prefer,
            };
            let base = compute_base_analysis(
                &inputs.client,
                &salsa_ctx,
                &inputs.disabled,
                &inputs.extra_commands,
                inputs.non_ascii_mode,
                &recovery,
            );
            let project = if path.cross_file_resolution {
                *path.db_project.lock().await
            } else {
                None
            };
            let produced =
                match compute_deep_diagnostic_report(uri, &salsa_ctx, &lift, &path, project, base)
                    .await
                {
                    ControlFlow::Continue(produced) => produced,
                    ControlFlow::Break((_, reason)) => return Err(reason),
                };
            produced.provider_completion?;
            if !source.matches_completed_analysis(&produced.analysis) {
                return Err(DiagnosticUnavailable::Context);
            }
            produced
                .result
                .map_err(|_| DiagnosticUnavailable::ProviderError)?
        };
        Ok(ProducedDirectDiagnosticReport {
            receipt: DiagnosticCacheReceipt {
                source: Arc::clone(source),
                origin: DiagnosticCacheOrigin::Direct { policy },
            },
            diagnostics: self
                .diagnostic_publisher
                .with_pack_notices(uri, diagnostics),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn open_source(backend: &Backend, expected: &DiagnosticSourceExpectation) {
        backend
            .documents
            .lock("diagnostic_completion_fixture")
            .await
            .insert(
                expected.uri.clone(),
                DocumentState::with_version(
                    expected.source.clone(),
                    "tcl8.6".into(),
                    expected.version,
                ),
            );
        backend
            .db_set_source(&expected.uri, &expected.source, "tcl8.6".into())
            .await
            .unwrap();
    }

    async fn run_provider(backend: &Backend, uri: &Uri) {
        let inputs = backend
            .diag_inputs(uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let job = inputs.capture_job(uri).await.unwrap();
        assert!(
            job.diagnostic_pass.is_some(),
            "fixture retains an actual source input"
        );
        assert!(run_diagnostics_core(inputs, uri, job).await);
    }

    #[tokio::test]
    async fn current_diagnostics_requires_an_actual_completed_provider_even_for_empty_reports() {
        let backend = crate::tests::test_backend();
        let mut expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-completion.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Pending
            }
        ));
        // A published/displayed empty collection supplies no provider receipt.
        backend.pull_diag_cache.lock().await.insert(
            expected.uri.clone(),
            PullDiagEntry {
                currency: None,
                result_id: next_pull_diag_result_id(),
                revision: 0,
                diagnostics: Vec::new(),
            },
        );
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(
            matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { report } if report.diagnostics.is_empty())
        );

        // A fresh admission cannot authenticate an unretained publication.
        let admission = backend
            .document_path_admission(&expected.uri)
            .await
            .unwrap();
        let Ok(current) = backend
            .current_diagnostic_input(&expected, &admission)
            .await
        else {
            panic!("fixture retains the actual published SourceFile input");
        };
        let retained = backend
            .source_path_admissions
            .lock()
            .unwrap()
            .remove(&expected.uri)
            .unwrap();
        assert!(matches!(
            current.lock_current(&backend, &expected.uri, &admission),
            Err(DiagnosticUnavailable::Context)
        ));
        assert!(matches!(
            backend
                .current_diagnostic_input(&expected, &admission)
                .await,
            Err(DiagnosticUnavailable::Context)
        ));
        assert!(backend.current_diagnostics(&expected).await.is_err());
        backend
            .source_path_admissions
            .lock()
            .unwrap()
            .insert(expected.uri.clone(), retained);
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));

        let old = DiagnosticSourceExpectation {
            uri: expected.uri.clone(),
            version: 1,
            source: String::new(),
        };
        expected.version = 2;
        expected.source = "proc check {} {}\ncheck unexpected\n".into();
        open_source(&backend, &expected).await;
        assert!(matches!(
            backend.current_diagnostics(&old).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(
            matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { report } if !report.diagnostics.is_empty())
        );
    }

    #[tokio::test]
    async fn current_diagnostics_cannot_reborrow_a_replaced_equal_path_publication() {
        let mut backend = crate::tests::test_backend();
        let store = Arc::new(vfs::MemoryStore::new());
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-publication-aba.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        let path = "/same/diagnostic-publication-aba.tcl";
        store.upsert_uri(expected.uri.as_str(), path, Vec::new());
        backend.store = store.clone();
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        let old_pass = backend
            .diagnostic_publisher
            .completion
            .lock()
            .await
            .get(&expected.uri)
            .unwrap()
            .pass
            .clone();
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
        store.upsert_uri(
            expected.uri.as_str(),
            "/different/diagnostic-publication-aba.tcl",
            Vec::new(),
        );
        store.upsert_uri(expected.uri.as_str(), path, Vec::new());
        // Republish the same bytes/version/configuration under the replacement
        // provider binding without completing another diagnostic pass.
        open_source(&backend, &expected).await;
        let admission = backend
            .document_path_admission(&expected.uri)
            .await
            .unwrap();
        assert_ne!(old_pass.path_admission.as_ref(), admission.as_ref());
        let Ok(current) = backend
            .current_diagnostic_input(&expected, &admission)
            .await
        else {
            panic!("replacement publication has an actual current input");
        };
        assert_eq!(old_pass.input.as_deref(), Some(current.input.as_ref()));
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Stale
            }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
    }

    #[tokio::test]
    async fn current_diagnostics_withdraws_changed_inputs_generation_and_source() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-input-currency.tcl").unwrap(),
            version: 4,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
        // An unchanged editing input/revision/epoch cannot donate changed report configuration.
        let (prior_input, prior_configuration) = {
            let config = backend.resolved_db_config(&expected.uri).await;
            let db = backend.db.lock().await;
            let file = *backend.db_files.lock().await.get(&expected.uri).unwrap();
            (
                tcl_lsp_db::document_analysis_input(&*db, file, config).unwrap(),
                config.diagnostic_report_configuration(&*db),
            )
        };
        let prior_epoch = backend.diag_inputs_epoch();
        {
            use salsa::Setter as _;
            let config = backend.resolved_db_config(&expected.uri).await;
            let mut db = backend.db.lock().await;
            config
                .set_targets(&mut *db)
                .to(vec![("tcl".into(), "8.4-9.0".into())]);
        }
        {
            let config = backend.resolved_db_config(&expected.uri).await;
            let db = backend.db.lock().await;
            let file = *backend.db_files.lock().await.get(&expected.uri).unwrap();
            assert_eq!(
                prior_input,
                tcl_lsp_db::document_analysis_input(&*db, file, config).unwrap()
            );
            assert_ne!(
                prior_configuration,
                config.diagnostic_report_configuration(&*db)
            );
        }
        assert_eq!(prior_epoch, backend.diag_inputs_epoch());
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
        *backend.class_factory_generation.write().await += 1;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), false);
        backend.invalidate_diag_inputs();
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed {
                reason: DiagnosticSuppression::Disabled,
                ..
            }
        ));
        let admission = backend
            .document_path_admission(&expected.uri)
            .await
            .unwrap();
        backend
            .documents
            .lock("completion-close")
            .await
            .remove(&expected.uri);
        backend
            .evict_diag_slot(&expected.uri, &admission)
            .await
            .unwrap();
        assert!(
            backend
                .diagnostic_publisher
                .completion
                .lock()
                .await
                .get(&expected.uri)
                .is_none()
        );
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
    }

    #[tokio::test]
    async fn cancelled_and_failed_passes_cannot_reuse_a_prior_complete_report() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-pass-outcomes.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        let inputs = backend
            .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        let pass = job.diagnostic_pass.as_ref().unwrap();
        backend
            .diagnostic_publisher
            .begin_diagnostic_pass(&expected.uri, pass)
            .await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Pending
            }
        ));
        for reason in [
            DiagnosticUnavailable::Cancelled,
            DiagnosticUnavailable::ProviderError,
        ] {
            backend
                .diagnostic_publisher
                .finish_diagnostic_pass(
                    &expected.uri,
                    pass,
                    DiagnosticPassState::Unavailable(reason),
                )
                .await;
            assert!(matches!(
                backend.current_diagnostics(&expected).await.unwrap(),
                CurrentDiagnostics::Unavailable { .. }
            ));
        }
        // A superseded pass cannot overwrite the actual newer pass state.
        let successor = inputs.capture_job(&expected.uri).await.unwrap();
        let successor = successor.diagnostic_pass.as_ref().unwrap();
        backend
            .diagnostic_publisher
            .begin_diagnostic_pass(&expected.uri, successor)
            .await;
        backend
            .diagnostic_publisher
            .finish_diagnostic_pass(
                &expected.uri,
                pass,
                DiagnosticPassState::Unavailable(DiagnosticUnavailable::ProviderError),
            )
            .await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Pending
            }
        ));
    }

    async fn await_pending_notification(backend: &Backend, uri: &Uri) {
        crate::rt::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let pending = backend
                    .diagnostic_publisher
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .pending
                    .contains_key(uri);
                if pending {
                    break;
                }
                crate::rt::yield_now().await;
            }
        })
        .await
        .expect("the actual provider must enqueue its notification");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn suppression_requires_delivered_clear_and_superseded_clear_cannot_settle() {
        use std::sync::atomic::Ordering;
        let backend = Arc::new(crate::tests::test_backend());
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-clear-receipt.tcl").unwrap(),
            version: 7,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), false);
        backend.invalidate_diag_inputs();
        // Reuse the existing held-mailbox seam: no new production instrumentation.
        backend
            .diagnostic_publisher
            .started
            .store(true, Ordering::Release);
        let runner = Arc::clone(&backend);
        let uri = expected.uri.clone();
        let first = crate::rt::spawn(async move { run_provider(&runner, &uri).await });
        await_pending_notification(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Pending
            }
        ));
        // This real mailbox replacement settles the first clear as Superseded.
        let replacement = backend.diagnostic_publisher.enqueue_assembled(
            expected.uri.clone(),
            Vec::new(),
            Some(expected.version),
            false,
            false,
        );
        first.await.unwrap();
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        let runner = Arc::clone(&backend);
        let uri = expected.uri.clone();
        let second = crate::rt::spawn(async move { run_provider(&runner, &uri).await });
        // The second actual clear must supersede this queued notification.
        assert_eq!(
            crate::rt::timeout(std::time::Duration::from_secs(5), replacement)
                .await
                .expect("the second clear must reach the mailbox")
                .unwrap(),
            DiagnosticPublishOutcome::Superseded
        );
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        backend
            .diagnostic_publisher
            .started
            .store(false, Ordering::Release);
        backend.diagnostic_publisher.ensure_started();
        second.await.unwrap();
        assert!(
            matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed { reason: DiagnosticSuppression::Disabled, report } if report.diagnostics.is_empty())
        );

        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), true);
        *backend.diagnostics_exclude.lock().await = vec!["*.tcl".into()];
        backend.invalidate_diag_inputs();
        run_provider(&backend, &expected.uri).await;
        assert!(
            matches!(backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed { reason: DiagnosticSuppression::Excluded, report } if report.diagnostics.is_empty())
        );
    }

    #[tokio::test]
    async fn issued_publication_id_cannot_be_replaced_by_the_actual_legacy_pull_handler() {
        use std::sync::atomic::Ordering;
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-issued-publication.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        let inputs = backend
            .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        let pass = job.diagnostic_pass.as_ref().unwrap();
        let analysis = backend
            .analysis_for(&expected.uri, Arc::clone(&job.text), job.dialect.clone())
            .await
            .unwrap();
        backend
            .diagnostic_publisher
            .begin_diagnostic_pass(&expected.uri, pass)
            .await;
        let delivery = DeliveryCtx {
            diagnostic_pass: Some(pass),
            client: &backend.client,
            diagnostic_publisher: &backend.diagnostic_publisher,
            documents: &backend.documents,
            store: &backend.store,
            diag_slots: &backend.diag_slots,
            pull_diag_cache: &backend.pull_diag_cache,
            closed_diag_gen: &backend.closed_diag_gen,
            uri: &expected.uri,
            currency: job.currency,
            version: job.version,
            client_supports_pull: false,
        };
        // Hold actual delivery while the legacy handler independently replaces
        // the cache. No production scheduling or publisher hook is added.
        backend
            .diagnostic_publisher
            .started
            .store(true, Ordering::Release);
        let publication = {
            let docs = backend.documents.lock("issued-publication-control").await;
            assert!(delivery.is_current(&docs).await);
            delivery.cache_and_enqueue(Vec::new()).await
        };
        assert_eq!(
            backend
                .pull_diag_cache
                .lock()
                .await
                .get(&expected.uri)
                .unwrap()
                .result_id,
            publication.result_id
        );
        backend.pull_diag_cache.lock().await.remove(&expected.uri);
        let params = serde_json::from_value(serde_json::json!({
            "textDocument": {"uri": expected.uri}
        }))
        .unwrap();
        let legacy = backend.diagnostic(params).await.unwrap();
        let legacy_id = match legacy {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(report)) => {
                report.full_document_diagnostic_report.result_id.unwrap()
            }
            _ => panic!("a real cache-miss pull must publish a new full report"),
        };
        assert_ne!(publication.result_id, legacy_id);
        backend
            .diagnostic_publisher
            .started
            .store(false, Ordering::Release);
        backend.diagnostic_publisher.ensure_started();
        assert_eq!(
            publication.receipt.await.unwrap(),
            DiagnosticPublishOutcome::Delivered
        );
        // Exercise the actual post-ack issuer facade with its own transported
        // publication ID. This control isolates publication identity; it does
        // not claim another complete provider pass was run by this direct call.
        delivery
            .complete_diagnostic_pass(&analysis, publication.result_id.clone())
            .await;
        {
            let ledger = backend.diagnostic_publisher.completion.lock().await;
            let entry = ledger.get(&expected.uri).unwrap();
            assert!(matches!(
                &entry.state,
                DiagnosticPassState::Complete { result_id }
                    if result_id == &publication.result_id && result_id != &legacy_id
            ));
        }
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Stale
            }
        ));
        // A new genuine provider pass remains able to issue current completion.
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
    }

    #[tokio::test]
    async fn actual_failed_lift_worker_cannot_issue_completed_empty_diagnostics() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-failed-lift.tcl").unwrap(),
            version: 2,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        let inputs = backend
            .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        let pass = job.diagnostic_pass.as_ref().unwrap();
        backend
            .diagnostic_publisher
            .begin_diagnostic_pass(&expected.uri, pass)
            .await;
        let analysis = backend
            .analysis_for(&expected.uri, Arc::clone(&job.text), job.dialect.clone())
            .await
            .unwrap();
        let delivery = DeliveryCtx {
            diagnostic_pass: Some(pass),
            client: &backend.client,
            diagnostic_publisher: &backend.diagnostic_publisher,
            documents: &backend.documents,
            store: &backend.store,
            diag_slots: &backend.diag_slots,
            pull_diag_cache: &backend.pull_diag_cache,
            closed_diag_gen: &backend.closed_diag_gen,
            uri: &expected.uri,
            currency: job.currency,
            version: job.version,
            client_supports_pull: false,
        };
        let failed =
            crate::rt::spawn_blocking(|| -> Vec<tower_lsp_server::ls_types::Diagnostic> {
                panic!("actual diagnostic lift worker failure control")
            })
            .await;
        assert!(failed.is_err());
        assert!(
            publish_diagnostics_result(
                &delivery,
                &backend.workspace_index,
                &backend.rehomed_source_seeds,
                &backend.rehoming_gate,
                &analysis,
                (Ok(()), failed),
                PublishTiming {
                    started: crate::rt::Instant::now(),
                    uri_str: expected.uri.as_str(),
                    line_count: 1
                }
            )
            .await
        );
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::ProviderError
            }
        ));
    }

    #[tokio::test]
    async fn actual_foreign_provider_registry_cannot_borrow_completed_analysis_currency() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-provider-registry.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
        let mut inputs = backend
            .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect("tcl8.6"))
            .await;
        let foreign = inputs
            .registry
            .project_for_profile(tcl_lsp_core::profile_for_dialect("tcl8.4"));
        assert_ne!(
            inputs.registry.snapshot().semantic_key(),
            foreign.snapshot().semantic_key()
        );
        inputs.registry = Arc::new(foreign);
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        assert!(run_diagnostics_core(inputs, &expected.uri, job).await);
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable {
                reason: DiagnosticUnavailable::Context
            }
        ));
    }

    #[tokio::test]
    async fn delivered_policy_suppression_does_not_claim_missing_analysis_context() {
        use salsa::Setter as _;
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-disabled-missing-context.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        {
            let config = backend.resolved_db_config(&expected.uri).await;
            let mut db = backend.db.lock().await;
            config.set_spec_pack_key(&mut *db).to(u64::MAX - 390_037);
        }
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), false);
        backend.invalidate_diag_inputs();
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed {
                reason: DiagnosticSuppression::Disabled,
                ..
            }
        ));
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), true);
        backend.invalidate_diag_inputs();
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
    }
    fn pull_params(uri: &Uri, previous: Option<&str>) -> DocumentDiagnosticParams {
        serde_json::from_value(serde_json::json!({
            "textDocument": { "uri": uri },
            "previousResultId": previous,
        }))
        .unwrap()
    }

    fn full_pull(report: DocumentDiagnosticReportResult) -> FullDocumentDiagnosticReport {
        match report {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(report)) => {
                report.full_document_diagnostic_report
            }
            _ => panic!("a changed original report requires a Full response"),
        }
    }

    fn workspace_params(uri: &Uri, previous: &str) -> WorkspaceDiagnosticParams {
        serde_json::from_value(serde_json::json!({
            "previousResultIds": [{ "uri": uri, "value": previous }]
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn legacy_cache_withdraws_config_only_changes_before_unchanged() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///legacy-config-currency.tcl").unwrap(),
            version: 3,
            source: "proc check {} {}\ncheck unexpected\n".into(),
        };
        open_source(&backend, &expected).await;
        let original = full_pull(
            backend
                .diagnostic(pull_params(&expected.uri, None))
                .await
                .unwrap(),
        );
        assert!(!original.items.is_empty());
        let previous = original.result_id.unwrap();
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), false);
        backend.invalidate_diag_inputs();
        let cleared = full_pull(
            backend
                .diagnostic(pull_params(&expected.uri, Some(&previous)))
                .await
                .unwrap(),
        );
        assert_ne!(cleared.result_id.as_deref(), Some(previous.as_str()));
        assert!(cleared.items.is_empty());
        {
            let cache = backend.pull_diag_cache.lock().await;
            assert!(matches!(
                cache
                    .get(&expected.uri)
                    .unwrap()
                    .currency
                    .as_ref()
                    .unwrap()
                    .origin,
                DiagnosticCacheOrigin::Direct {
                    policy: Some(DiagnosticSuppression::Disabled)
                }
            ));
        }
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Unavailable { .. }
        ));
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Suppressed {
                reason: DiagnosticSuppression::Disabled,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn workspace_cache_preserves_actual_report_version_and_whole_source() {
        let backend = crate::tests::test_backend();
        let mut expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///workspace-cache-currency.tcl").unwrap(),
            version: 1,
            source: "proc check {} {}\ncheck unexpected\n".into(),
        };
        open_source(&backend, &expected).await;
        let original = full_pull(
            backend
                .diagnostic(pull_params(&expected.uri, None))
                .await
                .unwrap(),
        );
        assert!(!original.items.is_empty());
        let previous = original.result_id.unwrap();
        expected.version = 2;
        expected.source.clear();
        open_source(&backend, &expected).await;
        let report = backend
            .workspace_diagnostic(workspace_params(&expected.uri, &previous))
            .await
            .unwrap();
        let WorkspaceDiagnosticReportResult::Report(report) = report else {
            panic!("workspace report retains actual participants");
        };
        let [WorkspaceDocumentDiagnosticReport::Full(report)] = report.items.as_slice() else {
            panic!("a changed source cannot borrow Unchanged from the older payload");
        };
        assert_eq!(report.version, Some(2));
        assert_ne!(
            report.full_document_diagnostic_report.result_id.as_deref(),
            Some(previous.as_str())
        );
        assert!(report.full_document_diagnostic_report.items.is_empty());
    }

    #[tokio::test]
    async fn legacy_cache_missing_context_or_closed_owner_cannot_issue_empty_current() {
        use salsa::Setter as _;
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///legacy-unavailable-currency.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        open_source(&backend, &expected).await;
        let original = full_pull(
            backend
                .diagnostic(pull_params(&expected.uri, None))
                .await
                .unwrap(),
        );
        let previous = original.result_id.unwrap();
        {
            let config = backend.resolved_db_config(&expected.uri).await;
            let mut db = backend.db.lock().await;
            config.set_spec_pack_key(&mut *db).to(u64::MAX - 391_111);
        }
        backend.invalidate_diag_inputs();
        assert!(
            backend
                .diagnostic(pull_params(&expected.uri, Some(&previous)))
                .await
                .is_err()
        );
        assert!(
            backend
                .workspace_diagnostic(workspace_params(&expected.uri, &previous))
                .await
                .is_err()
        );
        backend
            .documents
            .lock("legacy-cache-close-control")
            .await
            .remove(&expected.uri);
        assert!(
            backend
                .diagnostic(pull_params(&expected.uri, None))
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn legacy_cache_equal_path_replacement_cannot_reuse_completed_payload() {
        let mut backend = crate::tests::test_backend();
        let store = Arc::new(vfs::MemoryStore::new());
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///legacy-cache-aba.tcl").unwrap(),
            version: 1,
            source: String::new(),
        };
        let path = "/same/legacy-cache-aba.tcl";
        store.upsert_uri(expected.uri.as_str(), path, Vec::new());
        backend.store = store.clone();
        open_source(&backend, &expected).await;
        let original = full_pull(
            backend
                .diagnostic(pull_params(&expected.uri, None))
                .await
                .unwrap(),
        );
        let previous = original.result_id.unwrap();
        store.upsert_uri(
            expected.uri.as_str(),
            "/different/legacy-cache-aba.tcl",
            Vec::new(),
        );
        store.upsert_uri(expected.uri.as_str(), path, Vec::new());
        assert!(
            backend
                .current_cached_diagnostic_reports(std::slice::from_ref(&expected.uri))
                .await
                .is_err()
        );
        open_source(&backend, &expected).await;
        let current = full_pull(
            backend
                .diagnostic(pull_params(&expected.uri, Some(&previous)))
                .await
                .unwrap(),
        );
        assert_ne!(current.result_id.as_deref(), Some(previous.as_str()));
    }
}
