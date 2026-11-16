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
    f5_model_document_report, is_apl_source, jsonrpc, longest_folder_match,
    next_pull_diag_result_id, proposal_diagnostics, provider_source, rewrite_wire,
    sha256_digest_hex,
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
    currentness: ProposalCurrentnessData,
}

struct ProposalAnalysisResult<'a> {
    report: ProducedDiagnosticReport,
    keepalive: ProposalAnalysisKeepalive<'a>,
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

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
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
    checked_policy_sources: Vec<Arc<DiagnosticPass>>,
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
                tcl_lsp_db::document_analysis_input(&*db, file, job.config).ok()
            }
        } else {
            None
        };
        drop(db);
        let f5_model =
            Backend::is_bigip_dialect(&job.dialect) || is_apl_source(uri, &job.language_id);
        // A scheduled run defers the checked open-document registry to this
        // drain: an analysis pass requires the document's current input.
        // Policy-clear receipts retain source currency independently.
        let registry = if self.registry_at_drain
            && !f5_model
            && self.toggles.diagnostics_enabled
            && !self.toggles.excluded
        {
            input
                .as_ref()?
                .borrowed_context_registry()
                .commands()
                .snapshot()
        } else {
            self.registry.snapshot()
        };
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
            registry,
            f5_model,
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
        let Some(result_id) = self.delivered_non_tcl_report(Vec::new()).await else {
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
        let currentness = &analysis.keepalive.currentness;
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
            let (index, generation) = {
                let index = inputs.workspace_index.read().await;
                (index.clone(), index.generation())
            };
            let folder_configs = inputs.folder_db_configs.lock().await.clone();
            let global_config = *inputs.db_config.lock().await;
            let rehomed_source_seeds = inputs.rehomed_source_seeds.lock().await.clone();
            let package_revision = inputs.package_resolver.read().await.revision();
            let factory_generation = inputs.class_factory_generation.read().await;
            // A live Salsa snapshot must not survive an unrelated await.
            // Keep the factory generation stable while taking the final snapshot.
            let captured = capture_cross_file_evidence_snapshot(handles)
                .await
                .ok_or(DiagnosticUnavailable::Context)?;
            (
                captured,
                index,
                generation,
                *factory_generation,
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
            workspace_declared_commands: file
                .workspace_declared_commands(&*context.captured.snapshot)
                .clone(),
            workspace_source_modules: file
                .workspace_source_modules(&*context.captured.snapshot)
                .clone(),
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
        if !inputs.sidecar_revision.is_published_current() {
            return Err(DiagnosticUnavailable::Stale);
        }
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
            sidecar_revision: &inputs.sidecar_revision,
            registry: Arc::clone(&inputs.registry),
            extra_commands: &inputs.extra_commands,
            generic_variable_patterns: inputs.generic_variable_patterns.as_deref(),
            non_ascii_mode: inputs.non_ascii_mode,
            script_encoding: inputs.script_encoding,
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
            keepalive: ProposalAnalysisKeepalive {
                _projection: projection,
                _detached_db: detached_db,
                _detached_project: detached_project,
                _detached_workspace_index: detached_workspace_index,
                _projected_rehomed_source_seeds: projected_rehomed_source_seeds,
                _proposal_recovery_names: proposal_recovery_names,
                _analysis_text: analysis_text,
                _lift: lift,
                _registry: path_registry,
                currentness,
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
            if !inputs.sidecar_revision.is_published_current() {
                return Err(DiagnosticUnavailable::Stale);
            }
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
            if !inputs.sidecar_revision.is_published_current() {
                return Err(DiagnosticUnavailable::Stale);
            }
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
        // One typed PendingWrite-only recovery across both publication
        // attempts: at most three productions, no new deadline or sleep.
        let mut pending_write_retry_available = true;
        for attempt in 0..2 {
            // A scheduler admission can supersede the captured slot while the
            // provider runs. Recompute once with a fresh receipt; never relabel
            // the stale result or restart the caller's deadline/cancellation.
            if attempt > 0
                && let Ok(mut reports) = self
                    .current_cached_diagnostic_reports(std::slice::from_ref(uri))
                    .await?
            {
                return Ok(reports.remove(0));
            }
            let produced = self.produce_direct_diagnostic_report(uri).await;
            let produced = if matches!(
                &produced,
                Err(DirectDiagnosticProductionFailure::PendingWrite)
            ) && pending_write_retry_available
            {
                pending_write_retry_available = false;
                if let Ok(mut reports) = self
                    .current_cached_diagnostic_reports(std::slice::from_ref(uri))
                    .await?
                {
                    return Ok(reports.remove(0));
                }
                self.produce_direct_diagnostic_report(uri).await
            } else {
                produced
            }
            .map_err(|failure| Self::diagnostic_report_unavailable(failure.reason()))?;
            match self.publish_direct_diagnostic_report(uri, produced).await? {
                Ok(report) => return Ok(report),
                Err(DiagnosticUnavailable::Stale) if attempt == 0 => {}
                Err(reason) => {
                    return Err(Self::diagnostic_report_unavailable(reason));
                }
            }
        }
        unreachable!("the final attempt returns its actual publication result")
    }

    async fn publish_direct_diagnostic_report(
        &self,
        uri: &Uri,
        produced: ProducedDirectDiagnosticReport,
    ) -> jsonrpc::Result<Result<PublishedDiagnosticReport, DiagnosticUnavailable>> {
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
            Ok(mut reports) => Ok(Ok(reports.remove(0))),
            Err(reason) => Ok(Err(reason)),
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
            checked_policy_sources: Vec::new(),
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

    fn validate_current_published_report(
        &self,
        item: CurrentPublishedReportItem<'_, '_>,
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
        let suppression = Self::current_published_suppression(CurrentPublishedSuppression {
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
        if suppression.is_some() {
            checks.checked_policy_sources.push(Arc::clone(pass));
        }
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

    fn finish_current_published_reports(
        &self,
        finalization: CurrentPublishedReportFinalization<'_, '_>,
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
                    checked_policy_sources,
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
            for pass in checked_policy_sources {
                if !pass.matches_source_publication(&db, &files, &published) {
                    return Ok(Err(DiagnosticUnavailable::Stale));
                }
            }
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
                .any(|sources| !sources.matches(docs, &index))
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

#[derive(Debug)]
enum DirectDiagnosticProductionFailure {
    PendingWrite,
    Unavailable(DiagnosticUnavailable),
}

impl From<DiagnosticUnavailable> for DirectDiagnosticProductionFailure {
    fn from(reason: DiagnosticUnavailable) -> Self {
        Self::Unavailable(reason)
    }
}

impl DirectDiagnosticProductionFailure {
    fn reason(self) -> DiagnosticUnavailable {
        match self {
            Self::PendingWrite => DiagnosticUnavailable::Cancelled,
            Self::Unavailable(reason) => reason,
        }
    }
}

struct ProducedDirectDiagnosticReport {
    receipt: DiagnosticCacheReceipt,
    diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
}

impl DiagnosticPass {
    fn matches_source_publication(
        &self,
        db: &tcl_lsp_db::TclDatabase,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &HashMap<Uri, PublishedSourcePath>,
    ) -> bool {
        let Some(file) = self.source_file else {
            return self.f5_model;
        };
        let uri = self.path_admission.uri();
        files.get(uri) == Some(&file)
            && published.get(uri).is_some_and(|retained| {
                retained.file == file && retained.admission.as_ref() == self.path_admission.as_ref()
            })
            && file.path(db).as_deref() == self.path_admission.analyser_path()
            && file.text(db) == tcl_lexer::normalise_lone_cr(&self.source).as_ref()
            && file.dialect(db) == &self.dialect
    }

    /// The checked open-document registry this pass's input carries.
    pub(super) fn checked_registry(&self) -> Option<Arc<CommandRegistry>> {
        if self.f5_model {
            return None;
        }
        self.input
            .as_ref()
            .map(|input| Arc::clone(input.borrowed_context_registry().commands()))
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

fn direct_diagnostic_analyser_path(inputs: &DiagInputs) -> AnalyserPathInputs<'_> {
    AnalyserPathInputs {
        sidecar_revision: &inputs.sidecar_revision,
        registry: Arc::clone(&inputs.registry),
        extra_commands: &inputs.extra_commands,
        generic_variable_patterns: inputs.generic_variable_patterns.as_deref(),
        non_ascii_mode: inputs.non_ascii_mode,
        script_encoding: inputs.script_encoding,
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
    ) -> Result<ProducedDirectDiagnosticReport, DirectDiagnosticProductionFailure> {
        self.edits_settled().await;
        let is_open = self
            .documents
            .lock("direct_diagnostic_readiness")
            .await
            .contains_key(uri);
        if is_open {
            let _ = Box::pin(self.read_document(uri))
                .await
                .map_err(|_| DiagnosticUnavailable::Context)?;
        }
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
    ) -> Result<Vec<tower_lsp_server::ls_types::Diagnostic>, DirectDiagnosticProductionFailure>
    {
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
        let cancellations =
            super::diagnostic_read_cancellation::DiagnosticReadCancellations::default();
        let base = Box::pin(super::compute_base_analysis_with_read_cancellations(
            &inputs.client,
            &salsa_ctx,
            &inputs.disabled,
            &inputs.extra_commands,
            inputs.non_ascii_mode,
            &recovery,
            Some(&cancellations),
        ));
        let project = if path.cross_file_resolution {
            *path.db_project.lock().await
        } else {
            None
        };
        let produced = match Box::pin(
            super::compute_deep_diagnostic_report_with_read_cancellations(
                uri,
                &self.documents,
                &salsa_ctx,
                &lift,
                &path,
                project,
                (base, Some(&cancellations)),
            ),
        )
        .await
        {
            ControlFlow::Continue(produced) => produced,
            ControlFlow::Break((_, reason)) => {
                return Err(
                    if matches!(reason, DiagnosticUnavailable::Cancelled)
                        && cancellations.pending_write_only()
                    {
                        DirectDiagnosticProductionFailure::PendingWrite
                    } else {
                        reason.into()
                    },
                );
            }
        };
        produced.provider_completion?;
        if !produced
            .provider_sources
            .is_current(&*inputs.package_resolver.read().await)
        {
            return Err(DiagnosticUnavailable::Stale.into());
        }
        if !source.matches_completed_analysis(&produced.analysis) {
            return Err(DiagnosticUnavailable::Context.into());
        }
        *workspace_sources = Some(Arc::clone(&produced.workspace_sources));
        produced.result.map_err(|_| {
            DirectDiagnosticProductionFailure::Unavailable(DiagnosticUnavailable::ProviderError)
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod proposal_endpoint_tests;

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
