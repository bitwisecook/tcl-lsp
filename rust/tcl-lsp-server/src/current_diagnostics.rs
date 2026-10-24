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

use super::{
    AnalyserPathInputs, AnalysisResult, Arc, Backend, CLASS_FACTORY_SYNC_ROUNDS, CommandRegistry,
    ControlFlow, DeliveryCtx, DiagCurrency, DiagInputs, DiagJob, DiagSlot, DiagnosticPublisher,
    Digest, DocumentPathAdmission, DocumentPublication, DocumentState, EvidenceHandles,
    F5ReportInputs, HashMap, HashSet, LiftInputs, Mutex, OrphanCommandDependencies,
    PackNoticeContext, Path, ProducedDiagnosticReport, PublishedSourcePath, PullDiagEntry,
    RecoveryNameCache, RecoveryWidenCtx, SalsaAnalysisCtx, Sha256, TrackedMutex, TrackedRwLock,
    Uri, WorkspaceCommandQueries, WorkspaceDiagnosticSources, capture_cross_file_evidence_snapshot,
    compute_base_analysis, compute_deep_diagnostic_report, core_workspace_index,
    current_diagnostics, f5_model_document_report, is_apl_source, jsonrpc, longest_folder_match,
    next_pull_diag_result_id, proposal_diagnostics, provider_source, rewrite_wire,
    sha256_digest_hex, with_pack_hooks,
};
#[cfg(test)]
use super::{
    DiagnosticPublishOutcome, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, FromStr, FullDocumentDiagnosticReport, LanguageServer, PathBuf,
    PublishTiming, TrackedMutexGuard, WorkspaceDiagnosticParams, WorkspaceDiagnosticReportResult,
    WorkspaceDocumentDiagnosticReport, publish_diagnostics_result, run_diagnostics_core,
    sync_evaluator_epoch, sync_overlay_epoch, vfs,
};
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

/// Caller-held identity for read-only reborrowing of the existing publisher ledger.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DiagnosticPublicationExpectation {
    uri: Uri,
    version: i32,
    source: String,
    dialect_id: String,
    revision: String,
    inputs_epoch: String,
    result_id: String,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalDiagnosticExpectation {
    uri: Uri,
    version: i32,
    source: String,
    proposal: String,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum ProposalDiagnostics {
    Complete {
        uri: Uri,
        version: i32,
        source: String,
        proposal: String,
        // Serialized from the captured effective resource dialect only after
        // the producer's final source-store currentness check succeeds.
        #[serde(rename = "dialectId")]
        dialect_id: String,
        revision: String,
        #[serde(rename = "inputsEpoch")]
        inputs_epoch: String,
        #[serde(rename = "workspaceIndexGeneration")]
        workspace_index_generation: String,
        #[serde(rename = "resultId")]
        result_id: String,
        diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
    },
    Suppressed {
        uri: Uri,
        version: i32,
        source: String,
        proposal: String,
        reason: DiagnosticSuppression,
    },
    Unavailable {
        reason: DiagnosticUnavailable,
    },
}

struct ProposalCurrentness<'a> {
    expected: &'a ProposalDiagnosticExpectation,
    document_revision: u64,
    dialect: &'a str,
    admission: &'a Arc<DocumentPathAdmission>,
    inputs_epoch: u64,
    database_revision: salsa::Revision,
    project: tcl_lsp_db::Project,
    members: &'a HashSet<Uri>,
    rehomed_source_seeds: &'a HashMap<String, Vec<String>>,
    token_configurations: &'a [(tcl_lsp_db::SourceFile, tcl_lsp_db::AnalyserConfig)],
    source_receipts: &'a [(Uri, tcl_lsp_db::SourceFile, Arc<DocumentPathAdmission>)],
    selected_original_file: tcl_lsp_db::SourceFile,
    index_generation: u64,
    class_factory_generation: u64,
    evaluator_epoch: u64,
    overlay_epoch: u64,
    package_revision: u64,
    folder_configs: &'a [(Uri, tcl_lsp_db::AnalyserConfig)],
    global_config: tcl_lsp_db::AnalyserConfig,
}

fn proposal_lock_result<T, E>(result: Result<T, E>) -> Result<T, DiagnosticUnavailable> {
    result.map_err(|_| DiagnosticUnavailable::Pending)
}

fn extend_proposal_admissions<'a>(
    receipts: &'a [(Uri, tcl_lsp_db::SourceFile, Arc<DocumentPathAdmission>)],
    published: &HashMap<Uri, PublishedSourcePath>,
    admissions: &mut Vec<&'a DocumentPathAdmission>,
) -> Result<(), DiagnosticUnavailable> {
    for (uri, file, admission) in receipts {
        let current = published.get(uri).ok_or(DiagnosticUnavailable::Stale)?;
        if current.file != *file || !Arc::ptr_eq(&current.admission, admission) {
            return Err(DiagnosticUnavailable::Stale);
        }
        admissions.push(admission.as_ref());
    }
    Ok(())
}

struct ProposalDocumentCapture {
    document_revision: u64,
    dialect: String,
    admission: Arc<DocumentPathAdmission>,
}

struct ProposalWorkspaceCapture {
    captured: super::CrossFileEvidenceSnapshot,
    original_index: core_workspace_index::WorkspaceIndex,
    index_generation: u64,
    factory_generation: u64,
    folder_configs: Vec<(Uri, tcl_lsp_db::AnalyserConfig)>,
    global_config: tcl_lsp_db::AnalyserConfig,
    rehomed_source_seeds: HashMap<String, Vec<String>>,
    package_revision: u64,
}

struct ProposalSourceRows {
    original_token_configurations: Vec<(tcl_lsp_db::SourceFile, tcl_lsp_db::AnalyserConfig)>,
    source_inputs: Vec<proposal_diagnostics::SourceInput>,
    source_receipts: Vec<(Uri, tcl_lsp_db::SourceFile, Arc<DocumentPathAdmission>)>,
    selected_original_file: tcl_lsp_db::SourceFile,
    project_uris: HashSet<Uri>,
    source_admissions: HashMap<Uri, super::PublishedSourcePath>,
}

struct ProposalSourceRowContext<'a> {
    expected: &'a ProposalDiagnosticExpectation,
    document: &'a ProposalDocumentCapture,
    captured: &'a super::CrossFileEvidenceSnapshot,
    token_configurations: &'a [(tcl_lsp_db::SourceFile, tcl_lsp_db::AnalyserConfig)],
    folder_configs: &'a [(Uri, tcl_lsp_db::AnalyserConfig)],
    global_config: tcl_lsp_db::AnalyserConfig,
    source_admissions: &'a HashMap<Uri, super::PublishedSourcePath>,
}

struct ProposalSourceRow {
    input: proposal_diagnostics::SourceInput,
    admission: Arc<DocumentPathAdmission>,
    is_selected: bool,
}

struct ProposalCurrentnessData {
    document_revision: u64,
    dialect: String,
    admission: Arc<DocumentPathAdmission>,
    inputs_epoch: u64,
    database_revision: salsa::Revision,
    project: tcl_lsp_db::Project,
    members: HashSet<Uri>,
    rehomed_source_seeds: HashMap<String, Vec<String>>,
    token_configurations: Vec<(tcl_lsp_db::SourceFile, tcl_lsp_db::AnalyserConfig)>,
    source_receipts: Vec<(Uri, tcl_lsp_db::SourceFile, Arc<DocumentPathAdmission>)>,
    selected_original_file: tcl_lsp_db::SourceFile,
    index_generation: u64,
    class_factory_generation: u64,
    evaluator_epoch: u64,
    overlay_epoch: u64,
    package_revision: u64,
    folder_configs: Vec<(Uri, tcl_lsp_db::AnalyserConfig)>,
    global_config: tcl_lsp_db::AnalyserConfig,
    _project_uris: HashSet<Uri>,
    _source_admissions: HashMap<Uri, super::PublishedSourcePath>,
}

struct ProposalProjectionBuild<'a> {
    expected: &'a ProposalDiagnosticExpectation,
    document: ProposalDocumentCapture,
    inputs_epoch: u64,
    workspace: ProposalWorkspaceCapture,
    rows: ProposalSourceRows,
}

struct ProposalProjectionState {
    projection: proposal_diagnostics::Projection,
    original_index: core_workspace_index::WorkspaceIndex,
    currentness: ProposalCurrentnessData,
}

struct ProposalAnalysisKeepalive<'a> {
    _lift: LiftInputs<'a>,
    _registry: Arc<CommandRegistry>,
    _proposal_recovery_names: Arc<Mutex<RecoveryNameCache>>,
    _analysis_text: std::borrow::Cow<'a, str>,
    _projected_rehomed_source_seeds: Arc<Mutex<HashMap<String, Vec<String>>>>,
    _detached_workspace_index: Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    _detached_project: Arc<Mutex<Option<tcl_lsp_db::Project>>>,
    _detached_db: Arc<TrackedMutex<tcl_lsp_db::TclDatabase>>,
    _projection: proposal_diagnostics::Projection,
    _currentness: ProposalCurrentnessData,
}

struct ProposalAnalysisResult<'a> {
    report: ProducedDiagnosticReport,
    _keepalive: ProposalAnalysisKeepalive<'a>,
}

struct ProposalSubclassSettlement<'a> {
    inputs: &'a DiagInputs,
    salsa_ctx: &'a SalsaAnalysisCtx<'a>,
    recovery: &'a RecoveryWidenCtx<'a>,
    workspace_index: &'a Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    projection: &'a mut proposal_diagnostics::Projection,
}

struct ProposalDeepAnalysis<'a> {
    expected: &'a ProposalDiagnosticExpectation,
    inputs: &'a DiagInputs,
    salsa_ctx: &'a SalsaAnalysisCtx<'a>,
    recovery: &'a RecoveryWidenCtx<'a>,
    lift: &'a LiftInputs<'a>,
    path_inputs: &'a AnalyserPathInputs<'a>,
    projection: &'a proposal_diagnostics::Projection,
}

fn proposal_diagnostic_result_id(
    expected: &ProposalDiagnosticExpectation,
    currentness: &ProposalCurrentness<'_>,
    registry: &CommandRegistry,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(expected.uri.as_str().as_bytes());
    hasher.update(expected.version.to_le_bytes());
    hasher.update(expected.source.as_bytes());
    hasher.update(expected.proposal.as_bytes());
    hasher.update(
        format!(
            "{:?}:{}:{}:{}:{}:{}:{}",
            currentness.database_revision,
            currentness.inputs_epoch,
            currentness.index_generation,
            currentness.package_revision,
            currentness.evaluator_epoch,
            currentness.overlay_epoch,
            currentness.class_factory_generation
        )
        .as_bytes(),
    );
    hasher.update(format!("{:?}", registry.snapshot().semantic_key()).as_bytes());
    sha256_digest_hex(&hasher.finalize())
}

struct DiagnosticReportExpectation {
    uri: Uri,
    version: Option<i32>,
    source: Arc<str>,
}

struct CurrentPublishedReportBatch<'a, 'b> {
    expected: &'a [DiagnosticReportExpectation],
    admissions: &'a [Arc<DocumentPathAdmission>],
    current: &'a [Result<CurrentDiagnosticInput, DiagnosticUnavailable>],
    configurations: &'b [tcl_lsp_db::AnalyserConfig],
    purpose: DiagnosticCacheQuery<'b>,
    epoch: u64,
    factory_generation: u64,
    docs: &'b HashMap<Uri, DocumentState>,
    slots: &'b HashMap<Uri, DiagSlot>,
    ledger: &'b HashMap<Uri, DiagnosticCompletionEntry>,
    cache: &'b HashMap<Uri, PullDiagEntry>,
}

struct CurrentPublishedReportItem<'a, 'b> {
    source: &'a DiagnosticReportExpectation,
    admission: &'a Arc<DocumentPathAdmission>,
    current: &'a Result<CurrentDiagnosticInput, DiagnosticUnavailable>,
    configuration: &'b tcl_lsp_db::AnalyserConfig,
    purpose: DiagnosticCacheQuery<'b>,
    epoch: u64,
    factory_generation: u64,
    doc: &'b DocumentState,
    slots: &'b HashMap<Uri, DiagSlot>,
    ledger: &'b HashMap<Uri, DiagnosticCompletionEntry>,
    cache: &'b HashMap<Uri, PullDiagEntry>,
    checks: &'b mut CurrentPublishedReportChecks<'a>,
}

struct CurrentPublishedSuppression<'a> {
    report: &'a PullDiagEntry,
    receipt: &'a DiagnosticCacheReceipt,
    source: &'a DiagnosticReportExpectation,
    purpose: DiagnosticCacheQuery<'a>,
    epoch: u64,
    doc: &'a DocumentState,
    slots: &'a HashMap<Uri, DiagSlot>,
    ledger: &'a HashMap<Uri, DiagnosticCompletionEntry>,
}

struct CurrentPublishedReportChecks<'a> {
    reports: Vec<PublishedDiagnosticReport>,
    workspace_sources: Vec<Arc<WorkspaceDiagnosticSources>>,
    checked_inputs: Vec<(
        &'a CurrentDiagnosticInput,
        &'a DiagnosticReportExpectation,
        &'a Arc<DocumentPathAdmission>,
    )>,
    checked_configurations: Vec<(
        tcl_lsp_db::AnalyserConfig,
        tcl_lsp_db::DiagnosticConfigurationSnapshot,
    )>,
}

struct CurrentPublishedReportFinalization<'a, 'b> {
    expected: &'a [DiagnosticReportExpectation],
    admissions: &'a [Arc<DocumentPathAdmission>],
    docs: &'b HashMap<Uri, DocumentState>,
    cache: &'b mut HashMap<Uri, PullDiagEntry>,
    purpose: DiagnosticCacheQuery<'b>,
    checks: CurrentPublishedReportChecks<'a>,
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

pub(super) fn trace_diagnostic_context_check(uri: &Uri, stage: &str, accepted: bool) -> bool {
    #[cfg(debug_assertions)]
    if std::env::var("TCL_LSP_TRACE_COARSE_SOURCE").as_deref() == Ok("1") {
        eprintln!(
            "DIAGNOSTIC_CONTEXT uri={} stage={stage} accepted={accepted}",
            uri.as_str()
        );
    }
    #[cfg(not(debug_assertions))]
    let _ = (uri, stage);
    accepted
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
        trace_diagnostic_context_check(uri, "capture.input_present", input.is_some());
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
            source_inputs_epoch: Arc::clone(&self.source_inputs_epoch),
            source_inputs_gate: Arc::clone(&self.source_inputs_gate),
            source_slots: Arc::downgrade(&self.diag_slots),
            source_factory_generation: Arc::clone(&self.class_factory_generation),
            slot_generation,
            class_factory_generation: generation,
            input,
            registry: self.registry.snapshot(),
            f5_model: Backend::is_bigip_dialect(&job.dialect)
                || is_apl_source(uri, &job.language_id),
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

    /// Complete this pass with the F5 model report delivered for `text`.
    /// Only an F5 model pass over exactly that source completes; anything else
    /// is a context mismatch, never a borrowed completion.
    pub(super) async fn complete_f5_model_pass(&self, text: &str, result_id: String) {
        let Some(pass) = self.diagnostic_pass else {
            return;
        };
        if !pass.f5_model || pass.source.as_ref() != text {
            self.diagnostic_pass_unavailable(DiagnosticUnavailable::Context)
                .await;
            return;
        }
        self.diagnostic_publisher
            .finish_diagnostic_pass(self.uri, pass, DiagnosticPassState::Complete { result_id })
            .await;
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

    /// Reborrow the authentic publisher ledger without scheduling analysis,
    /// publishing/reissuing a report, or accepting a direct pull-cache receipt.
    pub(super) async fn validate_diagnostic_publication_command(
        &self,
        arguments: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let [argument] = arguments else {
            return Err(jsonrpc::Error::invalid_params(
                "Expected one publication identity",
            ));
        };
        let expected: DiagnosticPublicationExpectation = serde_json::from_value(argument.clone())
            .map_err(|_| {
            jsonrpc::Error::invalid_params("Expected a complete publication identity")
        })?;
        let source = DiagnosticReportExpectation {
            uri: expected.uri.clone(),
            version: Some(expected.version),
            source: Arc::from(expected.source.as_str()),
        };
        let current = self
            .current_published_diagnostic_reports(&[source], DiagnosticCacheQuery::PublisherOnly)
            .await?;
        let valid = current.is_ok_and(|reports| {
            let [report] = reports.as_slice() else {
                return false;
            };
            report.suppression.is_none()
                && report.uri == expected.uri
                && report.version == Some(expected.version)
                && report.source.as_ref() == expected.source
                && report.dialect == expected.dialect_id
                && report.revision.to_string() == expected.revision
                && report.inputs_epoch.to_string() == expected.inputs_epoch
                && report.result_id == expected.result_id
        });
        if !valid {
            return Ok(Some(serde_json::json!({"status":"unavailable"})));
        }
        Ok(Some(serde_json::json!({
            "status":"current", "uri":expected.uri, "version":expected.version,
            "source":expected.source, "dialectId":expected.dialect_id,
            "revision":expected.revision, "inputsEpoch":expected.inputs_epoch,
            "resultId":expected.result_id,
        })))
    }

    /// Analyse proposed full source in an isolated project projection. This
    /// report is not a push acknowledgement or a grant to edit the document.
    pub(super) async fn proposal_diagnostics_command(
        &self,
        arguments: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let [argument] = arguments else {
            return Err(jsonrpc::Error::invalid_params(
                "Expected one proposal source expectation",
            ));
        };
        let expected: ProposalDiagnosticExpectation = serde_json::from_value(argument.clone())
            .map_err(|_| {
                jsonrpc::Error::invalid_params("Expected {uri, version, source, proposal}")
            })?;
        let answer = match self.produce_proposal_diagnostics(&expected).await {
            Ok(answer) => answer,
            Err(reason) => ProposalDiagnostics::Unavailable { reason },
        };
        serde_json::to_value(answer)
            .map(Some)
            .map_err(|_| jsonrpc::Error::internal_error())
    }

    async fn produce_proposal_diagnostics(
        &self,
        expected: &ProposalDiagnosticExpectation,
    ) -> Result<ProposalDiagnostics, DiagnosticUnavailable> {
        let (document, _language_id) = self.capture_proposal_document(expected).await?;
        let inputs = self
            .diag_inputs(
                &expected.uri,
                tcl_lsp_core::profile_for_dialect(&document.dialect),
            )
            .await;
        let inputs_epoch = inputs.inputs_epoch.ok_or(DiagnosticUnavailable::Stale)?;
        if let Some(suppressed) = self
            .suppressed_proposal_diagnostics(expected, &document, &inputs, inputs_epoch)
            .await?
        {
            return Ok(suppressed);
        }
        let handles = EvidenceHandles::from_inputs(&inputs);
        let workspace = self
            .capture_proposal_workspace(&inputs, inputs_epoch, &handles)
            .await?;
        let rows = self.capture_proposal_source_rows(expected, &document, &workspace)?;
        let proposal_sources = provider_source::ProposalProviderSources::new(
            expected.uri.clone(),
            rows.selected_original_file,
            Arc::clone(&document.admission),
        )
        .ok_or(DiagnosticUnavailable::Context)?;
        let state = self
            .build_proposal_projection(ProposalProjectionBuild {
                expected,
                document,
                inputs_epoch,
                workspace,
                rows,
            })
            .await?;
        let analysis = self
            .analyse_proposal_projection(expected, &inputs, &proposal_sources, state)
            .await?;
        let diagnostics = analysis
            .report
            .result
            .map_err(|_| DiagnosticUnavailable::ProviderError)?;
        let currentness = &analysis._keepalive._currentness;
        let currentness = ProposalCurrentness {
            expected,
            document_revision: currentness.document_revision,
            dialect: &currentness.dialect,
            admission: &currentness.admission,
            inputs_epoch: currentness.inputs_epoch,
            database_revision: currentness.database_revision,
            project: currentness.project,
            members: &currentness.members,
            rehomed_source_seeds: &currentness.rehomed_source_seeds,
            token_configurations: &currentness.token_configurations,
            source_receipts: &currentness.source_receipts,
            selected_original_file: currentness.selected_original_file,
            index_generation: currentness.index_generation,
            class_factory_generation: currentness.class_factory_generation,
            evaluator_epoch: currentness.evaluator_epoch,
            overlay_epoch: currentness.overlay_epoch,
            package_revision: currentness.package_revision,
            folder_configs: &currentness.folder_configs,
            global_config: currentness.global_config,
        };
        let result_id = proposal_diagnostic_result_id(expected, &currentness, &inputs.registry);
        self.revalidate_proposal_capture(
            &currentness,
            &inputs,
            &analysis.report.provider_sources,
            &analysis.report.workspace_sources,
            diagnostics,
            result_id,
        )
        .await
    }

    async fn capture_proposal_document(
        &self,
        expected: &ProposalDiagnosticExpectation,
    ) -> Result<(ProposalDocumentCapture, String), DiagnosticUnavailable> {
        let (document_revision, dialect, language_id, admission) = {
            let documents = self.documents.lock("proposal_diagnostic_source").await;
            let document = documents
                .get(&expected.uri)
                .ok_or(DiagnosticUnavailable::NotOpen)?;
            if document.publication != DocumentPublication::Indexed
                || document.version != Some(expected.version)
                || document.raw() != expected.source
            {
                return Err(DiagnosticUnavailable::Stale);
            }
            (
                document.revision,
                document.dialect.clone(),
                document.language_id.clone(),
                document
                    .path_admission
                    .clone()
                    .ok_or(DiagnosticUnavailable::Context)?,
            )
        };
        admission
            .check_for(&expected.uri, self.store.as_ref())
            .map_err(|_| DiagnosticUnavailable::Context)?;
        if Backend::is_bigip_dialect(&dialect) || is_apl_source(&expected.uri, &language_id) {
            return Err(DiagnosticUnavailable::Unsupported);
        }
        Ok((
            ProposalDocumentCapture {
                document_revision,
                dialect,
                admission,
            },
            language_id,
        ))
    }

    async fn suppressed_proposal_diagnostics(
        &self,
        expected: &ProposalDiagnosticExpectation,
        document: &ProposalDocumentCapture,
        inputs: &DiagInputs,
        inputs_epoch: u64,
    ) -> Result<Option<ProposalDiagnostics>, DiagnosticUnavailable> {
        let reason = if !inputs.toggles.diagnostics_enabled {
            Some(DiagnosticSuppression::Disabled)
        } else if inputs.toggles.excluded {
            Some(DiagnosticSuppression::Excluded)
        } else {
            None
        };
        let Some(reason) = reason else {
            return Ok(None);
        };
        self.revalidate_suppressed_proposal(
            expected,
            document.document_revision,
            &document.dialect,
            &document.admission,
            inputs_epoch,
        )
        .await?;
        Ok(Some(ProposalDiagnostics::Suppressed {
            uri: expected.uri.clone(),
            version: expected.version,
            source: expected.source.clone(),
            proposal: expected.proposal.clone(),
            reason,
        }))
    }

    async fn capture_proposal_workspace(
        &self,
        inputs: &DiagInputs,
        inputs_epoch: u64,
        handles: &EvidenceHandles,
    ) -> Result<ProposalWorkspaceCapture, DiagnosticUnavailable> {
        let (
            captured,
            original_index,
            index_generation,
            factory_generation,
            folder_configs,
            global_config,
            rehomed_source_seeds,
            package_revision,
        ) = {
            let _analyser_inputs = self.analyser_inputs_gate.read().await;
            if self.diag_inputs_epoch() != inputs_epoch {
                return Err(DiagnosticUnavailable::Stale);
            }
            let _rehoming = inputs.rehoming_gate.lock().await;
            let _live_publication = inputs.live_publication_gate.lock().await;
            let captured = capture_cross_file_evidence_snapshot(handles)
                .await
                .ok_or(DiagnosticUnavailable::Context)?;
            let (index, generation) = {
                let index = inputs.workspace_index.read().await;
                (index.clone(), index.generation())
            };
            let factory_generation = *inputs.class_factory_generation.read().await;
            let folder_configs = inputs.folder_db_configs.lock().await.clone();
            let global_config = *inputs.db_config.lock().await;
            let rehomed_source_seeds = inputs.rehomed_source_seeds.lock().await.clone();
            let package_revision = inputs.package_resolver.read().await.revision();
            (
                captured,
                index,
                generation,
                factory_generation,
                folder_configs,
                global_config,
                rehomed_source_seeds,
                package_revision,
            )
        };
        Ok(ProposalWorkspaceCapture {
            captured,
            original_index,
            index_generation,
            factory_generation,
            folder_configs,
            global_config,
            rehomed_source_seeds,
            package_revision,
        })
    }

    fn capture_proposal_source_rows(
        &self,
        expected: &ProposalDiagnosticExpectation,
        document: &ProposalDocumentCapture,
        workspace: &ProposalWorkspaceCapture,
    ) -> Result<ProposalSourceRows, DiagnosticUnavailable> {
        let captured = &workspace.captured;
        let projected_rows = captured.project.files(&*captured.snapshot);
        if projected_rows.len() != captured.members.len() {
            return Err(DiagnosticUnavailable::Context);
        }
        let original_token_configurations = captured
            .project
            .token_configurations(&*captured.snapshot)
            .clone()
            .ok_or(DiagnosticUnavailable::Context)?;
        if original_token_configurations.len() != projected_rows.len() {
            return Err(DiagnosticUnavailable::Context);
        }
        let source_admissions = self
            .source_path_admissions
            .lock()
            .map_err(|_| DiagnosticUnavailable::Context)?
            .clone();
        let context = ProposalSourceRowContext {
            expected,
            document,
            captured: &workspace.captured,
            token_configurations: &original_token_configurations,
            folder_configs: &workspace.folder_configs,
            global_config: workspace.global_config,
            source_admissions: &source_admissions,
        };
        let mut source_inputs = Vec::with_capacity(projected_rows.len());
        let mut source_receipts = Vec::with_capacity(projected_rows.len());
        let mut project_uris = HashSet::with_capacity(projected_rows.len());
        let mut selected_original_file = None;
        for &file in projected_rows {
            let Some((uri, _)) = workspace
                .captured
                .files
                .iter()
                .find(|(_, candidate)| *candidate == file)
            else {
                return Err(DiagnosticUnavailable::Context);
            };
            if !project_uris.insert(uri.clone()) || !workspace.captured.members.contains(uri) {
                return Err(DiagnosticUnavailable::Context);
            }
            let row = self.capture_proposal_source_row(&context, uri, file)?;
            if row.is_selected {
                selected_original_file = Some(file);
            }
            source_receipts.push((uri.clone(), file, row.admission));
            source_inputs.push(row.input);
        }
        if project_uris != workspace.captured.members {
            return Err(DiagnosticUnavailable::Context);
        }
        Ok(ProposalSourceRows {
            original_token_configurations,
            source_inputs,
            source_receipts,
            selected_original_file: selected_original_file.ok_or(DiagnosticUnavailable::Context)?,
            project_uris,
            source_admissions,
        })
    }

    fn capture_proposal_source_row(
        &self,
        context: &ProposalSourceRowContext<'_>,
        uri: &Uri,
        file: tcl_lsp_db::SourceFile,
    ) -> Result<ProposalSourceRow, DiagnosticUnavailable> {
        let published = context
            .source_admissions
            .get(uri)
            .ok_or(DiagnosticUnavailable::Context)?;
        if published.file != file {
            return Err(DiagnosticUnavailable::Stale);
        }
        published
            .admission
            .check_for(uri, self.store.as_ref())
            .map_err(|_| DiagnosticUnavailable::Context)?;
        let source_path = published.admission.path().map(Path::to_path_buf);
        let file_path = file.path(&*context.captured.snapshot).clone();
        if file_path.as_deref() != published.admission.analyser_path()
            || file_path.is_some() != source_path.is_some()
        {
            return Err(DiagnosticUnavailable::Context);
        }
        let mut mapped_configs = context
            .token_configurations
            .iter()
            .filter(|(mapped_file, _)| *mapped_file == file);
        let config = mapped_configs
            .next()
            .map(|(_, config)| *config)
            .ok_or(DiagnosticUnavailable::Context)?;
        if mapped_configs.next().is_some() {
            return Err(DiagnosticUnavailable::Context);
        }
        let resolved_config = longest_folder_match(context.folder_configs, uri)
            .copied()
            .unwrap_or(context.global_config);
        if config != resolved_config {
            return Err(DiagnosticUnavailable::Context);
        }
        let row = proposal_diagnostics::SourceInput {
            uri: uri.clone(),
            text: file.text(&*context.captured.snapshot).clone(),
            dialect: file.dialect(&*context.captured.snapshot).clone(),
            path: file_path,
            source_path,
            external_call_sites: file
                .external_call_sites(&*context.captured.snapshot)
                .clone(),
            workspace_class_factories: file
                .workspace_class_factories(&*context.captured.snapshot)
                .clone(),
            workspace_subclass_methods: file
                .workspace_subclass_methods(&*context.captured.snapshot)
                .clone(),
            sidecar_stubs_epoch: file.sidecar_stubs_epoch(&*context.captured.snapshot),
            config: proposal_diagnostics::ConfigInput {
                disabled_diagnostics: config
                    .disabled_diagnostics(&*context.captured.snapshot)
                    .clone(),
                non_ascii_mode: config.non_ascii_mode(&*context.captured.snapshot),
                extra_commands: config.extra_commands(&*context.captured.snapshot).clone(),
                generic_variable_patterns: config
                    .generic_variable_patterns(&*context.captured.snapshot)
                    .clone(),
                bigip_version: config.bigip_version(&*context.captured.snapshot).clone(),
                spec_pack_key: config.spec_pack_key(&*context.captured.snapshot),
                targets: config.targets(&*context.captured.snapshot).clone(),
                package_provides: config.package_provides(&*context.captured.snapshot).clone(),
            },
        };
        let is_selected = uri == &context.expected.uri;
        if is_selected
            && (row.dialect != context.document.dialect
                || !Arc::ptr_eq(&published.admission, &context.document.admission)
                || tcl_lexer::normalise_lone_cr(&row.text).as_ref()
                    != tcl_lexer::normalise_lone_cr(&context.expected.source).as_ref())
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        Ok(ProposalSourceRow {
            input: row,
            admission: Arc::clone(&published.admission),
            is_selected,
        })
    }

    async fn build_proposal_projection(
        &self,
        build: ProposalProjectionBuild<'_>,
    ) -> Result<ProposalProjectionState, DiagnosticUnavailable> {
        let ProposalProjectionBuild {
            expected,
            document,
            inputs_epoch,
            workspace,
            rows,
        } = build;
        let database_revision = workspace.captured.revision;
        let project = workspace.captured.project;
        let members = workspace.captured.members.clone();
        let evaluator_epoch = tcl_registry::pack_hooks::evaluator_epoch();
        let overlay_epoch = tcl_registry::overlay_epoch();
        let captured_evaluator_epoch =
            tcl_lsp_db::EvaluatorEpoch::try_get(&*workspace.captured.snapshot)
                .map_or(0, |epoch| epoch.generation(&*workspace.captured.snapshot));
        let captured_overlay_epoch =
            tcl_lsp_db::OverlayEpoch::try_get(&*workspace.captured.snapshot)
                .map_or(0, |epoch| epoch.generation(&*workspace.captured.snapshot));
        if evaluator_epoch != captured_evaluator_epoch || overlay_epoch != captured_overlay_epoch {
            return Err(DiagnosticUnavailable::Stale);
        }
        let ProposalWorkspaceCapture {
            captured,
            original_index,
            index_generation,
            factory_generation,
            folder_configs,
            global_config,
            rehomed_source_seeds,
            package_revision,
        } = workspace;
        drop(captured);
        let mut projection = proposal_diagnostics::build(
            rows.source_inputs,
            &expected.uri,
            tcl_lexer::normalise_lone_cr(&expected.proposal).as_ref(),
            evaluator_epoch,
            overlay_epoch,
        )
        .map_err(|_| DiagnosticUnavailable::Context)?;
        if !proposal_diagnostics::recompute_class_factories(&mut projection).await
            || !proposal_diagnostics::recompute_call_site_evidence(&mut projection).await
        {
            return Err(DiagnosticUnavailable::Context);
        }
        Ok(ProposalProjectionState {
            projection,
            original_index,
            currentness: ProposalCurrentnessData {
                document_revision: document.document_revision,
                dialect: document.dialect,
                admission: document.admission,
                inputs_epoch,
                database_revision,
                project,
                members,
                rehomed_source_seeds,
                token_configurations: rows.original_token_configurations,
                source_receipts: rows.source_receipts,
                selected_original_file: rows.selected_original_file,
                index_generation,
                class_factory_generation: factory_generation,
                evaluator_epoch,
                overlay_epoch,
                package_revision,
                folder_configs,
                global_config,
                _project_uris: rows.project_uris,
                _source_admissions: rows.source_admissions,
            },
        })
    }

    async fn analyse_proposal_projection<'a>(
        &self,
        expected: &'a ProposalDiagnosticExpectation,
        inputs: &'a DiagInputs,
        proposal_sources: &'a provider_source::ProposalProviderSources,
        state: ProposalProjectionState,
    ) -> Result<ProposalAnalysisResult<'a>, DiagnosticUnavailable> {
        let currentness = state.currentness;
        let mut projection = state.projection;
        let original_index = state.original_index;
        let detached_db = Arc::clone(&projection.db);
        let detached_project = Arc::new(Mutex::new(Some(projection.project)));
        let detached_workspace_index = Arc::new(TrackedRwLock::new(
            "proposal_workspace_index",
            original_index,
        ));
        let selected_file = projection.selected;
        let projected_rehomed_source_seeds =
            Arc::new(Mutex::new(currentness.rehomed_source_seeds.clone()));
        let dialect_profile = tcl_lsp_core::profile_for_dialect(&currentness.dialect);
        let analysis_text = tcl_lexer::normalise_lone_cr(&expected.proposal);
        let salsa_ctx = SalsaAnalysisCtx {
            db: &detached_db,
            uri: &expected.uri,
            file: Some(selected_file),
            config: projection.selected_config,
            text: &analysis_text,
            dialect: dialect_profile,
        };
        let proposal_recovery_names = Arc::new(Mutex::new(RecoveryNameCache::default()));
        let path_inputs = AnalyserPathInputs {
            registry: Arc::clone(&inputs.registry),
            extra_commands: &inputs.extra_commands,
            generic_variable_patterns: inputs.generic_variable_patterns.as_deref(),
            non_ascii_mode: inputs.non_ascii_mode,
            db_project: &detached_project,
            workspace_index: &detached_workspace_index,
            rehomed_source_seeds: &projected_rehomed_source_seeds,
            rehoming_gate: &inputs.rehoming_gate,
            package_resolver: &inputs.package_resolver,
            package_prefer: inputs.package_prefer,
            provider_capture: provider_source::ProviderCaptureInputs {
                db: &inputs.db,
                files: &inputs.db_files,
                global: &inputs.db_config,
                folders: &inputs.folder_db_configs,
                store: &inputs.store,
                source_path_admissions: &inputs.source_path_admissions,
                dialects: &inputs.provider_dialects,
                proposal: Some(proposal_sources),
            },
            recovery_names: &proposal_recovery_names,
            entry_points: &inputs.entry_points,
            folder_root: inputs.folder_root.as_deref(),
            cross_file_resolution: inputs.toggles.xc.cross_file_resolution,
        };
        let recovery = RecoveryWidenCtx {
            cache: path_inputs.recovery_names,
            registry: &path_inputs.registry,
            workspace_index: path_inputs.workspace_index,
            package_resolver: path_inputs.package_resolver,
            provider_capture: &path_inputs.provider_capture,
            prefer: path_inputs.package_prefer,
        };
        self.settle_proposal_subclasses(ProposalSubclassSettlement {
            inputs,
            salsa_ctx: &salsa_ctx,
            recovery: &recovery,
            workspace_index: &detached_workspace_index,
            projection: &mut projection,
        })
        .await?;
        let lift = Self::proposal_lift_inputs(expected, inputs, &currentness, dialect_profile);
        let report = self
            .produce_proposal_deep_diagnostics(ProposalDeepAnalysis {
                expected,
                inputs,
                salsa_ctx: &salsa_ctx,
                recovery: &recovery,
                lift: &lift,
                path_inputs: &path_inputs,
                projection: &projection,
            })
            .await?;
        let path_registry = path_inputs.registry;
        Ok(ProposalAnalysisResult {
            report,
            _keepalive: ProposalAnalysisKeepalive {
                _projection: projection,
                _detached_db: detached_db,
                _detached_project: detached_project,
                _detached_workspace_index: detached_workspace_index,
                _projected_rehomed_source_seeds: projected_rehomed_source_seeds,
                _proposal_recovery_names: proposal_recovery_names,
                _analysis_text: analysis_text,
                _lift: lift,
                _registry: path_registry,
                _currentness: currentness,
            },
        })
    }

    async fn settle_proposal_subclasses(
        &self,
        settlement: ProposalSubclassSettlement<'_>,
    ) -> Result<(), DiagnosticUnavailable> {
        let ProposalSubclassSettlement {
            inputs,
            salsa_ctx,
            recovery,
            workspace_index,
            projection,
        } = settlement;
        let mut subclass_settled = false;
        for _ in 0..CLASS_FACTORY_SYNC_ROUNDS {
            let analysis = Box::pin(compute_base_analysis(
                &inputs.client,
                salsa_ctx,
                &inputs.disabled,
                &inputs.extra_commands,
                inputs.non_ascii_mode,
                recovery,
            ))
            .await;
            let analysis = match analysis {
                ControlFlow::Continue(analysis) => analysis,
                ControlFlow::Break(false) => return Err(DiagnosticUnavailable::Cancelled),
                ControlFlow::Break(true) => return Err(DiagnosticUnavailable::ProviderError),
            };
            workspace_index
                .write()
                .await
                .replace_document(salsa_ctx.uri.as_str(), &analysis);
            let index = workspace_index.read().await.clone();
            let Some(changed) =
                proposal_diagnostics::update_subclass_methods_from_index(projection, &index).await
            else {
                return Err(DiagnosticUnavailable::Context);
            };
            if !changed {
                subclass_settled = true;
                break;
            }
        }
        if !subclass_settled {
            return Err(DiagnosticUnavailable::Context);
        }
        Ok(())
    }

    async fn produce_proposal_deep_diagnostics(
        &self,
        analysis: ProposalDeepAnalysis<'_>,
    ) -> Result<ProducedDiagnosticReport, DiagnosticUnavailable> {
        let ProposalDeepAnalysis {
            expected,
            inputs,
            salsa_ctx,
            recovery,
            lift,
            path_inputs,
            projection,
        } = analysis;
        let base = Box::pin(compute_base_analysis(
            &inputs.client,
            salsa_ctx,
            &inputs.disabled,
            &inputs.extra_commands,
            inputs.non_ascii_mode,
            recovery,
        ));
        let project = inputs
            .toggles
            .xc
            .cross_file_resolution
            .then_some(projection.project);
        let produced = match Box::pin(compute_deep_diagnostic_report(
            &expected.uri,
            &self.documents,
            salsa_ctx,
            lift,
            path_inputs,
            project,
            base,
        ))
        .await
        {
            ControlFlow::Continue(produced) => produced,
            ControlFlow::Break((_, reason)) => return Err(reason),
        };
        produced.provider_completion?;
        if produced.provider_sources.source_failure().is_some() {
            return Err(DiagnosticUnavailable::Context);
        }
        let resolver = inputs.package_resolver.read().await;
        if !produced.provider_sources.is_current(&resolver) {
            return Err(DiagnosticUnavailable::Stale);
        }
        drop(resolver);
        Ok(produced)
    }

    fn proposal_lift_inputs<'a>(
        expected: &'a ProposalDiagnosticExpectation,
        inputs: &'a DiagInputs,
        currentness: &ProposalCurrentnessData,
        dialect: &'static tcl_dialect::DialectProfile,
    ) -> LiftInputs<'a> {
        LiftInputs {
            rewrite_source: Some(rewrite_wire::RewriteWireSource::Proposal {
                uri: expected.uri.clone(),
                version: expected.version,
                original_source: Arc::from(expected.source.as_str()),
                proposal: Arc::from(expected.proposal.as_str()),
                revision: currentness.document_revision,
                inputs_epoch: currentness.inputs_epoch,
            }),
            text: &expected.proposal,
            decode_report: None,
            dialect,
            disabled: &inputs.disabled,
            policy_layers: &inputs.policy_layers,
            style_line_length: inputs.style_line_length,
            xc_diagnostics: inputs.toggles.xc.xc_diagnostics,
        }
    }

    async fn revalidate_proposal_capture(
        &self,
        currentness: &ProposalCurrentness<'_>,
        inputs: &DiagInputs,
        provider_sources: &provider_source::ConfiguredProviderSourceReport,
        workspace_sources: &WorkspaceDiagnosticSources,
        diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
        result_id: String,
    ) -> Result<ProposalDiagnostics, DiagnosticUnavailable> {
        let _analyser_inputs = self.analyser_inputs_gate.read().await;
        if self.diag_inputs_epoch() != currentness.inputs_epoch {
            return Err(DiagnosticUnavailable::Stale);
        }
        let _rehoming = inputs.rehoming_gate.lock().await;
        let _live_publication = inputs.live_publication_gate.lock().await;
        let provider_admissions = provider_sources
            .proposal_admissions()
            .ok_or(DiagnosticUnavailable::Context)?;
        let documents = self.documents.lock("proposal_diagnostic_revalidate").await;
        let db = proposal_lock_result(self.db.try_lock())?;
        let files = proposal_lock_result(self.db_files.try_lock())?;
        let members = proposal_lock_result(self.db_project_members.try_lock())?;
        let project = proposal_lock_result(self.db_project.try_lock())?;
        let factory_generation = proposal_lock_result(inputs.class_factory_generation.try_read())?;
        let current_folder_configs = proposal_lock_result(inputs.folder_db_configs.try_lock())?;
        let current_global_config = proposal_lock_result(inputs.db_config.try_lock())?;
        let current_rehomed_source_seeds =
            proposal_lock_result(inputs.rehomed_source_seeds.try_lock())?;
        let index = proposal_lock_result(inputs.workspace_index.try_read())?;
        let resolver = proposal_lock_result(inputs.package_resolver.try_read())?;
        let published = proposal_lock_result(self.source_path_admissions.try_lock())?;
        let document = documents
            .get(&currentness.expected.uri)
            .ok_or(DiagnosticUnavailable::Stale)?;
        if document.publication != DocumentPublication::Indexed
            || document.revision != currentness.document_revision
            || document.version != Some(currentness.expected.version)
            || document.raw() != currentness.expected.source
            || document.dialect != currentness.dialect
            || !document
                .path_admission
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, currentness.admission))
            || tcl_lsp_db::database_revision(&db) != currentness.database_revision
            || files.get(&currentness.expected.uri) != Some(&currentness.selected_original_file)
            || *members != *currentness.members
            || *project != Some(currentness.project)
            || currentness.project.token_configurations(&*db).as_deref()
                != Some(currentness.token_configurations)
            || *factory_generation != currentness.class_factory_generation
            || current_folder_configs.as_slice() != currentness.folder_configs
            || *current_global_config != currentness.global_config
            || *current_rehomed_source_seeds != *currentness.rehomed_source_seeds
            || index.generation() != currentness.index_generation
            || resolver.revision() != currentness.package_revision
            || tcl_registry::pack_hooks::evaluator_epoch() != currentness.evaluator_epoch
            || tcl_registry::overlay_epoch() != currentness.overlay_epoch
            || self.diag_inputs_epoch() != currentness.inputs_epoch
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        let mut admissions = Vec::with_capacity(currentness.source_receipts.len());
        extend_proposal_admissions(currentness.source_receipts, &published, &mut admissions)?;
        extend_proposal_admissions(&provider_admissions, &published, &mut admissions)?;
        if !workspace_sources.matches(&documents, &index) {
            return Err(DiagnosticUnavailable::Stale);
        }
        let mut diagnostics = Some(diagnostics);
        let mut outcome = None;
        let check = WorkspaceDiagnosticSources::with_current_batch(
            &[workspace_sources],
            &admissions,
            self.store.as_ref(),
            || {
                if self.diag_inputs_epoch() != currentness.inputs_epoch
                    || tcl_registry::pack_hooks::evaluator_epoch() != currentness.evaluator_epoch
                    || tcl_registry::overlay_epoch() != currentness.overlay_epoch
                    || !provider_sources.proposal_is_current(&resolver, &*db, &files, &published)
                {
                    return Err(DiagnosticUnavailable::Stale);
                }
                outcome = Some(ProposalDiagnostics::Complete {
                    uri: currentness.expected.uri.clone(),
                    version: currentness.expected.version,
                    source: currentness.expected.source.clone(),
                    proposal: currentness.expected.proposal.clone(),
                    // This outcome is built inside the same final store-currentness
                    // closure as the diagnostics and their input epoch.
                    dialect_id: currentness.dialect.to_owned(),
                    revision: format!(
                        "{:?}:{}",
                        currentness.database_revision, currentness.document_revision
                    ),
                    inputs_epoch: currentness.inputs_epoch.to_string(),
                    workspace_index_generation: currentness.index_generation.to_string(),
                    result_id: result_id.clone(),
                    diagnostics: diagnostics.take().ok_or(DiagnosticUnavailable::Context)?,
                });
                Ok(())
            },
        )
        .map_err(|_| DiagnosticUnavailable::Context)?;
        check?;
        outcome.ok_or(DiagnosticUnavailable::Context)
    }

    async fn revalidate_suppressed_proposal(
        &self,
        expected: &ProposalDiagnosticExpectation,
        document_revision: u64,
        dialect: &str,
        admission: &Arc<DocumentPathAdmission>,
        inputs_epoch: u64,
    ) -> Result<(), DiagnosticUnavailable> {
        let _analyser_inputs = self.analyser_inputs_gate.read().await;
        if self.diag_inputs_epoch() != inputs_epoch {
            return Err(DiagnosticUnavailable::Stale);
        }
        let documents = self
            .documents
            .lock("proposal_diagnostic_suppression_revalidate")
            .await;
        let document = documents
            .get(&expected.uri)
            .ok_or(DiagnosticUnavailable::Stale)?;
        if document.publication != DocumentPublication::Indexed
            || document.revision != document_revision
            || document.version != Some(expected.version)
            || document.raw() != expected.source
            || document.dialect != dialect
            || !document
                .path_admission
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, admission))
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        DocumentPathAdmission::with_current(&[admission.as_ref()], self.store.as_ref(), || {
            if self.diag_inputs_epoch() == inputs_epoch {
                Ok(())
            } else {
                Err(DiagnosticUnavailable::Stale)
            }
        })
        .map_err(|_| DiagnosticUnavailable::Context)?
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
        let checks = match self.validate_current_published_reports(CurrentPublishedReportBatch {
            expected,
            admissions: &admissions,
            current: &current,
            configurations: &configurations,
            purpose,
            epoch,
            factory_generation: *factory_generation,
            docs: &docs,
            slots: &slots,
            ledger: &ledger,
            cache: &cache,
        }) {
            Ok(checks) => checks,
            Err(reason) => return Ok(Err(reason)),
        };
        self.finish_current_published_reports(CurrentPublishedReportFinalization {
            expected,
            admissions: &admissions,
            docs: &docs,
            cache: &mut cache,
            purpose,
            checks,
        })
    }

    fn validate_current_published_reports<'a>(
        &self,
        batch: CurrentPublishedReportBatch<'a, '_>,
    ) -> Result<CurrentPublishedReportChecks<'a>, DiagnosticUnavailable> {
        let CurrentPublishedReportBatch {
            expected,
            admissions,
            current,
            configurations,
            purpose,
            epoch,
            factory_generation,
            docs,
            slots,
            ledger,
            cache,
        } = batch;
        let mut checks = CurrentPublishedReportChecks {
            reports: Vec::with_capacity(expected.len()),
            workspace_sources: Vec::new(),
            checked_inputs: Vec::new(),
            checked_configurations: Vec::with_capacity(expected.len()),
        };
        for (((source, admission), current), configuration) in expected
            .iter()
            .zip(admissions.iter())
            .zip(current.iter())
            .zip(configurations.iter())
        {
            let doc = docs
                .get(&source.uri)
                .ok_or(DiagnosticUnavailable::NotOpen)?;
            self.validate_current_published_report(CurrentPublishedReportItem {
                source,
                admission,
                current,
                configuration,
                purpose,
                epoch,
                factory_generation,
                doc,
                slots,
                ledger,
                cache,
                checks: &mut checks,
            })?;
        }
        Ok(checks)
    }

    fn validate_current_published_report<'a>(
        &self,
        item: CurrentPublishedReportItem<'a, '_>,
    ) -> Result<(), DiagnosticUnavailable> {
        let CurrentPublishedReportItem {
            source,
            admission,
            current,
            configuration,
            purpose,
            epoch,
            factory_generation,
            doc,
            slots,
            ledger,
            cache,
            checks,
        } = item;
        if doc.version != source.version || doc.raw() != source.source.as_ref() {
            return Err(DiagnosticUnavailable::Stale);
        }
        if doc.publication != DocumentPublication::Indexed || self.diag_inputs_epoch() != epoch {
            return Err(DiagnosticUnavailable::Pending);
        }
        let report = match purpose {
            DiagnosticCacheQuery::PublishDirect(candidate) => candidate,
            _ => cache
                .get(&source.uri)
                .ok_or(DiagnosticUnavailable::Pending)?,
        };
        let receipt = report
            .currency
            .as_ref()
            .ok_or(DiagnosticUnavailable::Stale)?;
        let pass = &receipt.source;
        // An abstaining (non-text) source has no analysis report. The F5
        // model report applies that abstention itself, so it stays the
        // document's answer.
        if !pass.f5_model && tcl_lsp_core::source_decode::should_abstain(doc.decode_report.as_ref())
        {
            return Err(DiagnosticUnavailable::Context);
        }
        if let Some(sources) = receipt.workspace_sources.as_ref() {
            checks.workspace_sources.push(Arc::clone(sources));
        }
        let suppression = self.current_published_suppression(CurrentPublishedSuppression {
            report,
            receipt,
            source,
            purpose,
            epoch,
            doc,
            slots,
            ledger,
        })?;
        if report.revision != pass.revision || pass.path_admission.as_ref() != admission.as_ref() {
            return Err(DiagnosticUnavailable::Stale);
        }
        checks
            .checked_configurations
            .push((*configuration, pass.configuration.clone()));
        if suppression.is_none() && !pass.f5_model {
            let current = current.as_ref().map_err(|reason| *reason)?;
            if pass.source_file != Some(current.file)
                || pass.input.as_deref() != Some(current.input.as_ref())
                || pass.class_factory_generation != factory_generation
            {
                return Err(DiagnosticUnavailable::Stale);
            }
            checks.checked_inputs.push((current, source, admission));
        }
        checks.reports.push(PublishedDiagnosticReport {
            uri: source.uri.clone(),
            version: pass.version,
            source: Arc::clone(&pass.source),
            dialect: pass.dialect.clone(),
            revision: pass.revision,
            inputs_epoch: epoch,
            result_id: report.result_id.clone(),
            diagnostics: report.diagnostics.clone(),
            suppression,
        });
        Ok(())
    }

    fn current_published_suppression(
        &self,
        input: CurrentPublishedSuppression<'_>,
    ) -> Result<Option<DiagnosticSuppression>, DiagnosticUnavailable> {
        let CurrentPublishedSuppression {
            report,
            receipt,
            source,
            purpose,
            epoch,
            doc,
            slots,
            ledger,
        } = input;
        let pass = &receipt.source;
        match receipt.origin {
            DiagnosticCacheOrigin::Publisher => {
                let entry = ledger
                    .get(&source.uri)
                    .ok_or(DiagnosticUnavailable::Pending)?;
                let (result_id, suppression) = entry.current_result(
                    &source.source,
                    source.version,
                    doc,
                    epoch,
                    slots.get(&source.uri),
                )?;
                // The authentic current publisher pass owns unavailable
                // outcomes before any older cached report is considered.
                // Success still requires this exact pass and issued ID.
                if !Arc::ptr_eq(&entry.pass, pass) || result_id != report.result_id {
                    return Err(DiagnosticUnavailable::Stale);
                }
                Ok(suppression)
            }
            DiagnosticCacheOrigin::Direct { policy }
                if !matches!(purpose, DiagnosticCacheQuery::PublisherOnly) =>
            {
                pass.matches_current_source(
                    &source.source,
                    source.version,
                    doc,
                    epoch,
                    slots.get(&source.uri),
                )?;
                Ok(policy)
            }
            DiagnosticCacheOrigin::Direct { .. } => Err(DiagnosticUnavailable::Stale),
        }
    }

    fn finish_current_published_reports<'a>(
        &self,
        finalization: CurrentPublishedReportFinalization<'a, '_>,
    ) -> jsonrpc::Result<Result<Vec<PublishedDiagnosticReport>, DiagnosticUnavailable>> {
        let CurrentPublishedReportFinalization {
            expected,
            admissions,
            docs,
            cache,
            purpose,
            checks:
                CurrentPublishedReportChecks {
                    reports,
                    workspace_sources,
                    checked_inputs,
                    checked_configurations,
                },
        } = finalization;
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
        let _workspace_source_index = if workspace_sources.iter().any(|sources| !sources.is_empty())
        {
            let Ok(index) = self.workspace_index.try_read() else {
                return Ok(Err(DiagnosticUnavailable::Pending));
            };
            if workspace_sources
                .iter()
                .any(|sources| !sources.matches(&docs, &index))
            {
                return Ok(Err(DiagnosticUnavailable::Stale));
            }
            Some(index)
        } else {
            None
        };
        let admissions = admissions.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        let workspace_sources = workspace_sources
            .iter()
            .map(Arc::as_ref)
            .collect::<Vec<_>>();
        let mut reports = Some(reports);
        let result = WorkspaceDiagnosticSources::with_current_batch(
            &workspace_sources,
            &admissions,
            self.store.as_ref(),
            || {
                let Some(reports) = reports.take() else {
                    return Err(DiagnosticUnavailable::Stale);
                };
                if let DiagnosticCacheQuery::PublishDirect(candidate) = purpose {
                    cache.insert(expected[0].uri.clone(), candidate.clone());
                }
                Ok(reports)
            },
        )?;
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

struct ProducedDirectDiagnosticReport {
    receipt: DiagnosticCacheReceipt,
    diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
}

impl DiagnosticPass {
    fn trace_context_check(&self, stage: &str, accepted: bool) -> bool {
        trace_diagnostic_context_check(self.path_admission.uri(), stage, accepted)
    }

    pub(super) async fn workspace_source_input_guards(
        &self,
    ) -> Option<(
        tokio::sync::RwLockReadGuard<'_, ()>,
        tokio::sync::RwLockReadGuard<'_, u64>,
    )> {
        let inputs = self.source_inputs_gate.read().await;
        let factory = self.source_factory_generation.read().await;
        if self
            .source_inputs_epoch
            .load(std::sync::atomic::Ordering::Acquire)
            != self.inputs_epoch
            || *factory != self.class_factory_generation
        {
            return None;
        }
        Some((inputs, factory))
    }

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

    pub(super) fn rewrite_source_association(
        &self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<serde_json::Value> {
        if self.source.as_ref() != source || !self.matches_completed_analysis(analysis) {
            return None;
        }
        Some(serde_json::json!({
            "schema": 1,
            "purpose": "original-document-optimiser-rewrite",
            "uri": self.path_admission.uri(),
            "version": self.version,
            "source": source,
            "analysisSource": tcl_lexer::normalise_lone_cr(source),
            "revision": self.revision.to_string(),
            "inputsEpoch": self.inputs_epoch.to_string(),
            "dialectId": self.dialect,
        }))
    }

    fn matches_completed_analysis(&self, analysis: &AnalysisResult) -> bool {
        let Some(input) = self.input.as_ref() else {
            return self.trace_context_check("completed.input_present", false);
        };
        let source = tcl_lexer::normalise_lone_cr(&self.source);
        self.trace_context_check(
            "completed.source_current",
            tcl_compiler::source_graph::current_analysis(&source, analysis).is_some(),
        ) && self.trace_context_check(
            "completed.resolved_input",
            analysis.resolved_input.as_ref() == Some(input.as_ref()),
        ) && self.trace_context_check(
            "completed.registry",
            analysis.resolved_registry().is_some_and(|registry| {
                registry.snapshot().semantic_key() == self.registry.semantic_key()
            }),
        )
    }
}

fn direct_diagnostic_analyser_path(inputs: &DiagInputs) -> AnalyserPathInputs<'_> {
    AnalyserPathInputs {
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
            source_path_admissions: &inputs.source_path_admissions,
            dialects: &inputs.provider_dialects,
            proposal: None,
        },
        recovery_names: &inputs.recovery_names,
        entry_points: &inputs.entry_points,
        folder_root: inputs.folder_root.as_deref(),
        cross_file_resolution: inputs.toggles.xc.cross_file_resolution,
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
        let mut workspace_sources = None;
        let diagnostics = if policy.is_some() {
            Vec::new()
        } else {
            if source.f5_model {
                let analysis_text = tcl_lexer::normalise_lone_cr(&job.text);
                let diagnostics = f5_model_document_report(
                    &F5ReportInputs {
                        uri,
                        text: &job.text,
                        analysis_text: &analysis_text,
                        decode_report: job.decode_report.as_ref(),
                        dialect: tcl_lsp_core::profile_for_dialect(&job.dialect),
                        language_id: &job.language_id,
                        policy_layers: &inputs.policy_layers,
                    },
                    &inputs.documents,
                    inputs.store.as_ref(),
                )
                .await;
                return Ok(ProducedDirectDiagnosticReport {
                    receipt: DiagnosticCacheReceipt {
                        workspace_sources: None,
                        source: Arc::clone(source),
                        origin: DiagnosticCacheOrigin::Direct { policy: None },
                    },
                    diagnostics,
                });
            }
            self.produce_direct_tcl_diagnostics(uri, &inputs, &job, source, &mut workspace_sources)
                .await?
        };
        Ok(ProducedDirectDiagnosticReport {
            receipt: DiagnosticCacheReceipt {
                workspace_sources,
                source: Arc::clone(source),
                origin: DiagnosticCacheOrigin::Direct { policy },
            },
            diagnostics: self.diagnostic_publisher.with_pack_notices(
                PackNoticeContext {
                    uri,
                    source: source.source.as_ref(),
                    pack_seq: inputs.pack_seq,
                    packs: &inputs.pack_set,
                    registry: &inputs.registry,
                    store: inputs.store.as_ref(),
                },
                diagnostics,
            ),
        })
    }

    async fn produce_direct_tcl_diagnostics(
        &self,
        uri: &Uri,
        inputs: &DiagInputs,
        job: &DiagJob,
        source: &Arc<DiagnosticPass>,
        workspace_sources: &mut Option<Arc<WorkspaceDiagnosticSources>>,
    ) -> Result<Vec<tower_lsp_server::ls_types::Diagnostic>, DiagnosticUnavailable> {
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
            rewrite_source: Some(rewrite_wire::RewriteWireSource::Document(Arc::clone(
                source,
            ))),
            text: &job.text,
            decode_report: job.decode_report,
            dialect: profile,
            disabled: &inputs.disabled,
            policy_layers: &inputs.policy_layers,
            style_line_length: inputs.style_line_length,
            xc_diagnostics: inputs.toggles.xc.xc_diagnostics,
        };
        let path = direct_diagnostic_analyser_path(inputs);
        let recovery = RecoveryWidenCtx {
            cache: path.recovery_names,
            registry: &path.registry,
            workspace_index: path.workspace_index,
            package_resolver: path.package_resolver,
            provider_capture: &path.provider_capture,
            prefer: path.package_prefer,
        };
        let base = Box::pin(compute_base_analysis(
            &inputs.client,
            &salsa_ctx,
            &inputs.disabled,
            &inputs.extra_commands,
            inputs.non_ascii_mode,
            &recovery,
        ));
        let project = if path.cross_file_resolution {
            *path.db_project.lock().await
        } else {
            None
        };
        let produced = match Box::pin(compute_deep_diagnostic_report(
            uri,
            &self.documents,
            &salsa_ctx,
            &lift,
            &path,
            project,
            base,
        ))
        .await
        {
            ControlFlow::Continue(produced) => produced,
            ControlFlow::Break((_, reason)) => return Err(reason),
        };
        produced.provider_completion?;
        if !produced
            .provider_sources
            .is_current(&*inputs.package_resolver.read().await)
        {
            return Err(DiagnosticUnavailable::Stale);
        }
        if !source.matches_completed_analysis(&produced.analysis) {
            return Err(DiagnosticUnavailable::Context);
        }
        *workspace_sources = Some(Arc::clone(&produced.workspace_sources));
        produced
            .result
            .map_err(|_| DiagnosticUnavailable::ProviderError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn actual_direct_and_push_rewrites_retain_their_captured_document_source() {
        let backend = crate::tests::test_backend();
        backend
            .apply_global_config(&serde_json::json!({"optimiser": {"profile": "full"}}))
            .await
            .expect("the fixture configuration is valid");
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///rewrite-wire-captured-source.tcl").unwrap(),
            version: 7,
            source: "proc p {y} {\n    set x [llength $y]\n    puts $x\n}\n".into(),
        };
        open_source(&backend, &expected).await;
        let direct = backend
            .produce_direct_diagnostic_report(&expected.uri)
            .await
            .unwrap_or_else(|_| panic!("direct report"));
        assert_document_rewrites(&direct.diagnostics, &expected);
        run_provider(&backend, &expected.uri).await;
        let CurrentDiagnostics::Complete { report } =
            backend.current_diagnostics(&expected).await.unwrap()
        else {
            panic!("actual completed push report");
        };
        assert_document_rewrites(&report.diagnostics, &expected);
    }

    fn assert_document_rewrites(
        diagnostics: &[tower_lsp_server::ls_types::Diagnostic],
        expected: &DiagnosticSourceExpectation,
    ) {
        let rewrites: Vec<_> = diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.data.as_ref())
            .filter(|data| data.get("sourceAssociation").is_some())
            .collect();
        assert!(
            !rewrites.is_empty(),
            "actual worker must retain an applicable rewrite"
        );
        for rewrite in rewrites {
            let source = &rewrite["sourceAssociation"];
            assert_eq!(source["purpose"], "original-document-optimiser-rewrite");
            assert_eq!(source["uri"], expected.uri.as_str());
            assert_eq!(source["version"], expected.version);
            assert_eq!(source["source"], expected.source);
            assert!(source["revision"].is_string());
            assert!(source["inputsEpoch"].is_string());
            assert!(source.get("originalSource").is_none());
        }
    }

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
        // Select the provider profile from the actual published Salsa source,
        // and require it to agree with the live document before dispatch.
        let document = backend
            .documents
            .lock("diagnostic_provider_fixture")
            .await
            .get(uri)
            .cloned()
            .expect("fixture has a live document");
        let file = backend
            .db_files
            .lock()
            .await
            .get(uri)
            .copied()
            .expect("fixture publishes its source to the database");
        let dialect = {
            let db = backend.db.lock().await;
            assert_eq!(file.dialect(&*db), &document.dialect);
            assert_eq!(
                file.text(&*db),
                tcl_lexer::normalise_lone_cr(&document.text).as_ref()
            );
            file.dialect(&*db).clone()
        };
        let inputs = backend
            .diag_inputs(uri, tcl_lsp_core::profile_for_dialect(&dialect))
            .await;
        let job = inputs.capture_job(uri).await.unwrap();
        assert!(
            job.diagnostic_pass.is_some(),
            "fixture retains an actual source input"
        );
        assert!(run_diagnostics_core(inputs, uri, job).await);
    }

    #[tokio::test]
    async fn publication_validation_reborrows_exact_publisher_without_reissuing() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-validation-purpose.tcl").unwrap(),
            version: 1,
            source: "set x 1".into(),
        };
        backend.documents.lock("validation_fixture").await.insert(
            expected.uri.clone(),
            DocumentState::with_version(expected.source.clone(), "tcl9.0".into(), 1),
        );
        backend
            .db_set_source(&expected.uri, &expected.source, "tcl9.0".into())
            .await
            .unwrap();
        run_provider(&backend, &expected.uri).await;
        let mut identity =
            serde_json::to_value(backend.current_diagnostics(&expected).await.unwrap()).unwrap();
        assert_eq!(identity["status"], "complete");
        identity.as_object_mut().unwrap().remove("status");
        identity.as_object_mut().unwrap().remove("diagnostics");
        let before = backend.pull_diag_cache.lock().await[&expected.uri]
            .result_id
            .clone();
        let validation = backend
            .validate_diagnostic_publication_command(&[identity.clone()])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(validation["status"], "current");
        assert_eq!(
            backend.pull_diag_cache.lock().await[&expected.uri].result_id,
            before
        );
        for field in ["resultId", "revision", "inputsEpoch", "dialectId", "source"] {
            let mut foreign = identity.clone();
            foreign[field] = serde_json::json!("foreign");
            let refused = backend
                .validate_diagnostic_publication_command(&[foreign])
                .await
                .unwrap()
                .unwrap();
            assert_eq!(refused["status"], "unavailable", "{field}");
        }
        backend
            .diagnostic_publisher
            .completion
            .lock()
            .await
            .remove(&expected.uri);
        assert_eq!(
            backend
                .validate_diagnostic_publication_command(&[identity])
                .await
                .unwrap()
                .unwrap()["status"],
            "unavailable"
        );
    }

    #[tokio::test]
    async fn current_diagnostics_serializes_dialect_from_the_completed_publisher_pass() {
        let backend = crate::tests::test_backend();
        let expected = DiagnosticSourceExpectation {
            uri: Uri::from_str("file:///diagnostic-published-dialect.tcl").unwrap(),
            version: 1,
            source: "set x 1".into(),
        };
        let dialect = "tcl9.0";
        backend
            .documents
            .lock("diagnostic_completed_dialect")
            .await
            .insert(
                expected.uri.clone(),
                DocumentState::with_version(
                    expected.source.clone(),
                    dialect.into(),
                    expected.version,
                ),
            );
        backend
            .db_set_source(&expected.uri, &expected.source, dialect.into())
            .await
            .unwrap();
        let inputs = backend
            .diag_inputs(&expected.uri, tcl_lsp_core::profile_for_dialect(dialect))
            .await;
        let job = inputs.capture_job(&expected.uri).await.unwrap();
        assert!(job.diagnostic_pass.is_some());
        assert!(run_diagnostics_core(inputs, &expected.uri, job).await);
        let response =
            serde_json::to_value(backend.current_diagnostics(&expected).await.unwrap()).unwrap();
        assert_eq!(response["status"], "complete");
        assert_eq!(response["uri"], expected.uri.to_string());
        assert_eq!(response["version"], expected.version);
        assert_eq!(response["source"], expected.source);
        assert_eq!(response["dialectId"], dialect);
        assert!(response["resultId"].is_string());
        assert!(response["inputsEpoch"].is_string());
    }

    /// A BIG-IP configuration or iApp APL document answers
    /// `textDocument/diagnostic` with its F5 model report — the same report
    /// code the push publisher runs — both directly before any push and by
    /// reborrowing the completed push afterwards.
    #[tokio::test]
    async fn f5_model_documents_answer_pull_with_their_model_report() {
        for (uri, dialect, source) in [
            (
                "file:///pulled-config.scf",
                "f5-bigip",
                "ltm pool /Common/p {\n    members {\n        /Common/10.0.0.1:80 { }\n    }\n}\n",
            ),
            (
                "file:///pulled.apl",
                "f5-iapps",
                "section basic {\n    string name\n}\n",
            ),
        ] {
            let backend = crate::tests::test_backend();
            let expected = DiagnosticSourceExpectation {
                uri: Uri::from_str(uri).unwrap(),
                version: 1,
                source: source.into(),
            };
            backend.documents.lock("f5_pull_fixture").await.insert(
                expected.uri.clone(),
                DocumentState::with_version(
                    expected.source.clone(),
                    dialect.into(),
                    expected.version,
                ),
            );
            backend
                .db_set_source(&expected.uri, &expected.source, dialect.into())
                .await
                .unwrap();
            let direct = full_pull(
                backend
                    .diagnostic(pull_params(&expected.uri, None))
                    .await
                    .unwrap_or_else(|error| panic!("{uri}: direct pull refused: {error:?}")),
            );
            run_provider(&backend, &expected.uri).await;
            let pushed = backend
                .pull_diag_cache
                .lock()
                .await
                .get(&expected.uri)
                .map(|entry| (entry.result_id.clone(), entry.diagnostics.clone()))
                .expect("the push primed the pull cache");
            let reborrowed = full_pull(
                backend
                    .diagnostic(pull_params(&expected.uri, None))
                    .await
                    .unwrap_or_else(|error| panic!("{uri}: pull after push refused: {error:?}")),
            );
            assert_eq!(
                reborrowed.result_id.as_deref(),
                Some(pushed.0.as_str()),
                "{uri}"
            );
            assert_eq!(reborrowed.items, pushed.1, "{uri}");
            assert_eq!(direct.items, pushed.1, "{uri}: one report code");
            let current =
                serde_json::to_value(backend.current_diagnostics(&expected).await.unwrap())
                    .unwrap();
            assert_eq!(current["status"], "complete", "{uri}: {current}");
            assert_eq!(current["dialectId"], dialect, "{uri}");
        }
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

    fn assert_diagnostics_unavailable(report: &CurrentDiagnostics) {
        assert!(matches!(report, CurrentDiagnostics::Unavailable { .. }));
    }

    fn legacy_full_report_id(legacy: DocumentDiagnosticReportResult) -> String {
        match legacy {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(report)) => {
                report.full_document_diagnostic_report.result_id.unwrap()
            }
            _ => panic!("a real cache-miss pull must publish a new full report"),
        }
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
        assert_diagnostics_unavailable(&backend.current_diagnostics(&expected).await.unwrap());
        run_provider(&backend, &expected.uri).await;
        assert!(matches!(
            backend.current_diagnostics(&expected).await.unwrap(),
            CurrentDiagnostics::Complete { .. }
        ));
        *backend.class_factory_generation.write().await += 1;
        assert_diagnostics_unavailable(&backend.current_diagnostics(&expected).await.unwrap());
        run_provider(&backend, &expected.uri).await;
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), false);
        backend.invalidate_diag_inputs();
        assert_diagnostics_unavailable(&backend.current_diagnostics(&expected).await.unwrap());
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
        assert_diagnostics_unavailable(&backend.current_diagnostics(&expected).await.unwrap());
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
            source: &job.text,
            pack_seq: inputs.pack_seq,
            pack_set: Some(&inputs.pack_set),
            registry: Some(&inputs.registry),
            spec_packs: Some(&inputs.spec_packs),
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
        let legacy_id = legacy_full_report_id(legacy);
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
        // Preserve the original failed-worker assertion surface. This adapter
        // delegates to the current publication owner; a failed worker produced
        // no command-query receipt and cannot enter dependency admission.
        async fn publish_diagnostics_result(
            delivery: &DeliveryCtx<'_>,
            workspace_index: &Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
            rehomed_source_seeds: &Arc<Mutex<HashMap<String, Vec<String>>>>,
            rehoming_gate: &Arc<tokio::sync::Mutex<()>>,
            analysis: &Arc<AnalysisResult>,
            result: (
                Result<(), DiagnosticUnavailable>,
                Result<Vec<tower_lsp_server::ls_types::Diagnostic>, crate::rt::JoinError>,
            ),
            timing: PublishTiming<'_>,
        ) -> bool {
            crate::publish_diagnostics_result(
                delivery,
                workspace_index,
                rehomed_source_seeds,
                rehoming_gate,
                analysis,
                (result.0, result.1, None),
                timing,
            )
            .await
        }
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
            source: &job.text,
            pack_seq: inputs.pack_seq,
            pack_set: Some(&inputs.pack_set),
            registry: Some(&inputs.registry),
            spec_packs: Some(&inputs.spec_packs),
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
        assert_ne!(original.items.len(), 0);
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
        assert_eq!(
            cleared.items,
            [] as [tower_lsp_server::ls_types::Diagnostic; 0]
        );
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
        assert_ne!(original.items.len(), 0);
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
        assert_eq!(
            report.full_document_diagnostic_report.items,
            [] as [tower_lsp_server::ls_types::Diagnostic; 0]
        );
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

#[cfg(test)]
mod proposal_endpoint_tests {
    use super::*;

    async fn publish_source(
        backend: &Backend,
        store: &vfs::MemoryStore,
        path: &Path,
        source: &str,
        version: i32,
    ) -> Uri {
        let uri = Uri::from_file_path(path).unwrap();
        store.upsert_uri(uri.as_str(), path, source.as_bytes().to_vec());
        backend
            .db_set_source(&uri, source, "tcl8.6".into())
            .await
            .expect("real configured SourceFile/project publication");
        let admission = backend
            .source_path_admissions
            .lock()
            .unwrap()
            .get(&uri)
            .unwrap()
            .admission
            .clone();
        backend
            .documents
            .lock("proposal_endpoint_fixture")
            .await
            .insert(
                uri.clone(),
                DocumentState::with_version(source.into(), "tcl8.6".into(), version)
                    .with_path_admission(admission)
                    .with_language_id("tcl".into()),
            );
        let analysis = backend
            .analysis_for(&uri, Arc::from(source), "tcl8.6".into())
            .await
            .expect("fixture retains the actual selected full analysis input");
        backend
            .workspace_index
            .write()
            .await
            .replace_document(uri.as_str(), &analysis);
        uri
    }

    async fn fixture() -> (Backend, Arc<vfs::MemoryStore>, Uri, Uri, PathBuf) {
        let store = Arc::new(vfs::MemoryStore::new());
        let mut backend = crate::tests::test_backend();
        backend.store = store.clone();
        let root = std::env::temp_dir().join("tcl-lsp-proposal-endpoint-controls");
        let selected = publish_source(&backend, &store, &root.join("selected.tcl"), "", 7).await;
        let other = publish_source(
            &backend,
            &store,
            &root.join("other.tcl"),
            "# unchanged second project input\n",
            2,
        )
        .await;
        // Use the real epoch issuers, after the actual selected registries and
        // indexed analysis have been prepared. These are not guessed labels.
        sync_evaluator_epoch(&backend.db).await;
        sync_overlay_epoch(&backend.db).await;
        (backend, store, selected, other, root)
    }

    async fn command(
        backend: &Backend,
        uri: &Uri,
        version: i32,
        source: &str,
        proposal: &str,
    ) -> serde_json::Value {
        backend
            .proposal_diagnostics_command(&[serde_json::json!({
                "uri": uri,
                "version": version,
                "source": source,
                "proposal": proposal,
            })])
            .await
            .expect("actual proposal endpoint response")
            .expect("serialized response")
    }

    fn same_optional_arc<T>(before: Option<&Arc<T>>, after: Option<&Arc<T>>) -> bool {
        match (before, after) {
            (None, None) => true,
            (Some(before), Some(after)) => Arc::ptr_eq(before, after),
            _ => false,
        }
    }

    fn assert_invalid_proposal(
        invalid: &serde_json::Value,
        selected: &Uri,
        clean: &serde_json::Value,
    ) {
        assert_eq!(invalid["status"], "complete", "{invalid}");
        assert_eq!(invalid["uri"], selected.as_str());
        assert_eq!(invalid["version"], 7);
        assert_eq!(invalid["source"], "");
        assert_eq!(invalid["proposal"], "set\n");
        assert!(
            invalid["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["code"] == "E002"),
            "{invalid}"
        );
        assert_ne!(invalid["resultId"], clean["resultId"]);
    }

    fn assert_clean_proposal(clean: &serde_json::Value) {
        assert_eq!(clean["status"], "complete", "{clean}");
        assert!(
            clean["diagnostics"].as_array().unwrap().is_empty(),
            "{clean}"
        );
    }

    fn proposal_file_values(
        db: &tcl_lsp_db::TclDatabase,
        file: tcl_lsp_db::SourceFile,
        config: tcl_lsp_db::AnalyserConfig,
    ) -> (
        String,
        String,
        Option<String>,
        u64,
        tcl_lsp_db::DiagnosticConfigurationSnapshot,
    ) {
        (
            file.text(db).clone(),
            file.dialect(db).clone(),
            file.path(db).clone(),
            file.sidecar_stubs_epoch(db),
            config.diagnostic_report_configuration(db),
        )
    }

    #[tokio::test]
    async fn actual_proposal_command_changes_diagnostics_without_mutating_live_project() {
        let (backend, _, selected, other, _) = fixture().await;
        let index_generation = backend.workspace_index.read().await.generation();
        let members = backend.db_project_members.lock().await.clone();
        let project = backend.db_project.lock().await.unwrap();
        let configurations = project
            .token_configurations(&*backend.db.lock().await)
            .clone();
        let mut before = Vec::new();
        let revision = {
            let db = backend.db.lock().await;
            let files = backend.db_files.lock().await;
            for uri in [&selected, &other] {
                let file = files[uri];
                let config = configurations
                    .as_ref()
                    .unwrap()
                    .iter()
                    .find(|(candidate, _)| *candidate == file)
                    .unwrap()
                    .1;
                before.push((
                    uri.clone(),
                    file,
                    proposal_file_values(&db, file, config),
                    (
                        file.external_call_sites(&*db).clone(),
                        file.workspace_class_factories(&*db).clone(),
                        file.workspace_subclass_methods(&*db).clone(),
                    ),
                ));
            }
            tcl_lsp_db::database_revision(&db)
        };
        let selected_admission = backend.source_path_admissions.lock().unwrap()[&selected]
            .admission
            .clone();
        let clean = command(&backend, &selected, 7, "", "").await;
        assert_clean_proposal(&clean);
        let invalid = command(&backend, &selected, 7, "", "set\n").await;
        assert_invalid_proposal(&invalid, &selected, &clean);

        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        assert_eq!(tcl_lsp_db::database_revision(&db), revision);
        assert!(project.token_configurations(&*db) == &configurations);
        for (uri, file, values, evidence) in &before {
            assert!(files.get(uri) == Some(file));
            let config = configurations
                .as_ref()
                .unwrap()
                .iter()
                .find(|(candidate, _)| candidate == file)
                .unwrap()
                .1;
            assert_eq!(proposal_file_values(&db, *file, config), *values);
            assert!(same_optional_arc(
                evidence.0.as_ref(),
                file.external_call_sites(&*db).as_ref()
            ));
            assert!(same_optional_arc(
                evidence.1.as_ref(),
                file.workspace_class_factories(&*db).as_ref()
            ));
            assert!(same_optional_arc(
                evidence.2.as_ref(),
                file.workspace_subclass_methods(&*db).as_ref()
            ));
        }
        drop(files);
        drop(db);
        assert!(*backend.db_project.lock().await == Some(project));
        assert_eq!(*backend.db_project_members.lock().await, members);
        assert_eq!(
            backend.workspace_index.read().await.generation(),
            index_generation
        );
        let documents = backend.documents.lock("proposal_endpoint_unchanged").await;
        let document = &documents[&selected];
        assert_eq!(document.raw(), "");
        assert_eq!(document.version, Some(7));
        assert!(Arc::ptr_eq(
            document.path_admission.as_ref().unwrap(),
            &selected_admission
        ));
        assert_eq!(
            documents[&other].raw(),
            "# unchanged second project input\n"
        );
        assert_eq!(documents[&other].version, Some(2));
        drop(documents);
        assert!(
            backend.pull_diag_cache.lock().await.is_empty(),
            "proposal is not a push/cache publication"
        );
    }

    #[tokio::test]
    async fn actual_proposal_command_refuses_stale_original_and_withdrawn_source_admission() {
        let (backend, store, selected, _, root) = fixture().await;
        let first = command(&backend, &selected, 7, "", "").await;
        assert_eq!(first["status"], "complete", "{first}");
        let current_source = "# edited original source\n";
        publish_source(
            &backend,
            &store,
            &root.join("selected.tcl"),
            current_source,
            8,
        )
        .await;
        let stale = command(&backend, &selected, 7, "", "set\n").await;
        assert_eq!(stale["status"], "unavailable", "{stale}");
        assert_eq!(stale["reason"], "stale");
        let current = command(&backend, &selected, 8, current_source, "").await;
        assert_eq!(current["status"], "complete", "{current}");
        assert!(store.remove_uri(selected.as_str()));
        let withdrawn = command(&backend, &selected, 8, current_source, "").await;
        assert_eq!(withdrawn["status"], "unavailable", "{withdrawn}");
        assert_eq!(withdrawn["reason"], "context");
    }

    #[tokio::test]
    async fn actual_proposal_command_refuses_selected_provider_and_unversioned_disk_provider() {
        for selected_is_provider in [true, false] {
            let (backend, store, selected, _, root) = fixture().await;
            let clean = command(&backend, &selected, 7, "", "").await;
            assert_eq!(clean["status"], "complete", "{clean}");
            let provider_path = if selected_is_provider {
                root.join("selected.tcl")
            } else {
                let path = root.join("disk-provider.tcl");
                // Readable SourceStore bytes are deliberately not a retained
                // indexed SourceFile or a byte-version lease.
                store.upsert(&path, b"proc provided {} {}\n".to_vec());
                assert!(backend.store.read_source(&path).is_ok());
                assert!(
                    !backend
                        .db_files
                        .lock()
                        .await
                        .contains_key(&Uri::from_file_path(&path).unwrap())
                );
                path
            };
            let provider_name = provider_path.file_name().unwrap().to_str().unwrap();
            backend
                .package_resolver
                .write()
                .await
                .add_original_pkg_index(
                    &format!("package ifneeded own 1.0 {{source {provider_name}}}"),
                    &root,
                    &root.join("pkgIndex.tcl"),
                    &|_| true,
                    &|_| Vec::new(),
                );
            assert!(
                backend
                    .package_resolver
                    .read()
                    .await
                    .provider_source_files()
                    .contains(&provider_path)
            );
            let answer = command(
                &backend,
                &selected,
                7,
                "",
                "package require own\nprovided\n",
            )
            .await;
            assert_eq!(answer["status"], "unavailable", "{answer}");
            assert_eq!(answer["reason"], "context");
            let db = backend.db.lock().await;
            let file = backend.db_files.lock().await[&selected];
            assert_eq!(file.text(&*db), "");
        }
    }

    #[tokio::test]
    async fn actual_proposal_command_never_turns_suppression_or_incomplete_source_into_clean() {
        let (backend, _, selected, _, _) = fixture().await;
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), false);
        backend.invalidate_diag_inputs();
        let suppressed = command(&backend, &selected, 7, "", "").await;
        assert_eq!(suppressed["status"], "suppressed", "{suppressed}");
        assert_eq!(suppressed["reason"], "disabled");
        assert!(suppressed.get("diagnostics").is_none());
        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("diagnostics".into(), true);
        backend.invalidate_diag_inputs();
        let incomplete = command(&backend, &selected, 7, "", "proc unfinished {").await;
        assert_eq!(incomplete["status"], "unavailable", "{incomplete}");
        assert_eq!(incomplete["reason"], "context");
        assert!(incomplete.get("diagnostics").is_none());
    }
}

impl OrphanCommandDependencies {
    pub(super) fn from_pass(
        uri: &Uri,
        pass: &Arc<DiagnosticPass>,
        analysis: &Arc<AnalysisResult>,
        queries: WorkspaceCommandQueries,
    ) -> Option<Self> {
        if pass.path_admission.uri() != uri || !pass.matches_completed_analysis(analysis) {
            return None;
        }
        let _inputs = pass.source_inputs_gate.try_read().ok()?;
        if pass
            .source_inputs_epoch
            .load(std::sync::atomic::Ordering::Acquire)
            != pass.inputs_epoch
        {
            return None;
        }
        Some(Self {
            uri: uri.clone(),
            source: Arc::clone(&pass.source),
            dialect: pass.dialect.clone(),
            revision: pass.revision,
            version: pass.version,
            analysis: Arc::clone(analysis),
            expected_epoch: pass.inputs_epoch,
            current_epoch: Arc::clone(&pass.source_inputs_epoch),
            inputs_gate: Arc::clone(&pass.source_inputs_gate),
            slots: pass.source_slots.clone(),
            slot_generation: pass.slot_generation,
            factory_generation: Arc::clone(&pass.source_factory_generation),
            expected_factory_generation: pass.class_factory_generation,
            queries,
        })
    }
}
