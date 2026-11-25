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

//! Diagnostics for proposed full source, analysed in an isolated project projection and revalidated against the live capture.

use super::super::{
    AnalyserPathInputs, Arc, Backend, CLASS_FACTORY_SYNC_ROUNDS, CommandRegistry, ControlFlow,
    DiagInputs, Digest, DocumentPathAdmission, DocumentPublication, EvidenceHandles, HashMap,
    HashSet, LiftInputs, Mutex, Path, ProducedDiagnosticReport, PublishedSourcePath,
    RecoveryNameCache, RecoveryWidenCtx, SalsaAnalysisCtx, Sha256, TrackedMutex, TrackedRwLock,
    Uri, WorkspaceDiagnosticSources, capture_cross_file_evidence_snapshot, compute_base_analysis,
    compute_deep_diagnostic_report, core_workspace_index, is_apl_source, jsonrpc,
    longest_folder_match, proposal_diagnostics, provider_source, rewrite_wire, sha256_digest_hex,
};
use super::{DiagnosticSuppression, DiagnosticUnavailable};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposalDiagnosticExpectation {
    pub(super) uri: Uri,
    pub(super) version: i32,
    pub(super) source: String,
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
    pub(super) dialect: &'a str,
    pub(super) admission: &'a Arc<DocumentPathAdmission>,
    pub(super) inputs_epoch: u64,
    pub(super) database_revision: salsa::Revision,
    project: tcl_lsp_db::Project,
    members: &'a HashSet<Uri>,
    pub(super) rehomed_source_seeds: &'a HashMap<String, Vec<String>>,
    token_configurations: &'a [(tcl_lsp_db::SourceFile, tcl_lsp_db::AnalyserConfig)],
    source_receipts: &'a [(Uri, tcl_lsp_db::SourceFile, Arc<DocumentPathAdmission>)],
    selected_original_file: tcl_lsp_db::SourceFile,
    index_generation: u64,
    pub(super) class_factory_generation: u64,
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
    pub(super) dialect: String,
    pub(super) admission: Arc<DocumentPathAdmission>,
}

pub(super) struct ProposalWorkspaceCapture {
    pub(super) captured: super::super::CrossFileEvidenceSnapshot,
    original_index: core_workspace_index::WorkspaceIndex,
    index_generation: u64,
    pub(super) factory_generation: u64,
    folder_configs: Vec<(Uri, tcl_lsp_db::AnalyserConfig)>,
    global_config: tcl_lsp_db::AnalyserConfig,
    pub(super) rehomed_source_seeds: HashMap<String, Vec<String>>,
    package_revision: u64,
}

struct ProposalSourceRows {
    original_token_configurations: Vec<(tcl_lsp_db::SourceFile, tcl_lsp_db::AnalyserConfig)>,
    source_inputs: Vec<proposal_diagnostics::SourceInput>,
    source_receipts: Vec<(Uri, tcl_lsp_db::SourceFile, Arc<DocumentPathAdmission>)>,
    selected_original_file: tcl_lsp_db::SourceFile,
    project_uris: HashSet<Uri>,
    source_admissions: HashMap<Uri, super::super::PublishedSourcePath>,
}

struct ProposalSourceRowContext<'a> {
    expected: &'a ProposalDiagnosticExpectation,
    document: &'a ProposalDocumentCapture,
    pub(super) captured: &'a super::super::CrossFileEvidenceSnapshot,
    token_configurations: &'a [(tcl_lsp_db::SourceFile, tcl_lsp_db::AnalyserConfig)],
    folder_configs: &'a [(Uri, tcl_lsp_db::AnalyserConfig)],
    global_config: tcl_lsp_db::AnalyserConfig,
    source_admissions: &'a HashMap<Uri, super::super::PublishedSourcePath>,
}

struct ProposalSourceRow {
    pub(super) input: proposal_diagnostics::SourceInput,
    pub(super) admission: Arc<DocumentPathAdmission>,
    is_selected: bool,
}

struct ProposalCurrentnessData {
    document_revision: u64,
    pub(super) dialect: String,
    pub(super) admission: Arc<DocumentPathAdmission>,
    pub(super) inputs_epoch: u64,
    pub(super) database_revision: salsa::Revision,
    project: tcl_lsp_db::Project,
    members: HashSet<Uri>,
    pub(super) rehomed_source_seeds: HashMap<String, Vec<String>>,
    token_configurations: Vec<(tcl_lsp_db::SourceFile, tcl_lsp_db::AnalyserConfig)>,
    source_receipts: Vec<(Uri, tcl_lsp_db::SourceFile, Arc<DocumentPathAdmission>)>,
    selected_original_file: tcl_lsp_db::SourceFile,
    index_generation: u64,
    pub(super) class_factory_generation: u64,
    evaluator_epoch: u64,
    overlay_epoch: u64,
    package_revision: u64,
    folder_configs: Vec<(Uri, tcl_lsp_db::AnalyserConfig)>,
    global_config: tcl_lsp_db::AnalyserConfig,
    _project_uris: HashSet<Uri>,
    _source_admissions: HashMap<Uri, super::super::PublishedSourcePath>,
}

struct ProposalProjectionBuild<'a> {
    expected: &'a ProposalDiagnosticExpectation,
    document: ProposalDocumentCapture,
    pub(super) inputs_epoch: u64,
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
    pub(super) workspace_index: &'a Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
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

impl Backend {
    /// Analyse proposed full source in an isolated project projection. This
    /// report is not a push acknowledgement or a grant to edit the document.
    pub(in super::super) async fn proposal_diagnostics_command(
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

    pub(super) async fn capture_proposal_workspace(
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

    // One detached analysis context assembled field by field; splitting it
    // would only move the keepalive bookkeeping.
    #[allow(clippy::too_many_lines)]
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
            data_group_root: inputs.data_group_root.as_deref(),
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
                _registry: path_inputs.registry,
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
}
