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

//! The direct report: produced by the checked provider computation for a pull, under its own provider receipt.

use super::super::{
    AnalyserPathInputs, Arc, Backend, ControlFlow, DiagInputs, DiagJob, F5ReportInputs, LiftInputs,
    PackNoticeContext, PullDiagEntry, RecoveryWidenCtx, SalsaAnalysisCtx, Uri,
    WorkspaceDiagnosticSources, f5_model_document_report, jsonrpc, next_pull_diag_result_id,
    provider_source, rewrite_wire,
};
use super::{
    DiagnosticCacheOrigin, DiagnosticCacheQuery, DiagnosticCacheReceipt, DiagnosticPass,
    DiagnosticReportExpectation, DiagnosticSuppression, DiagnosticUnavailable,
    PublishedDiagnosticReport,
};

impl Backend {
    pub(in super::super) fn diagnostic_report_unavailable(
        reason: DiagnosticUnavailable,
    ) -> jsonrpc::Error {
        jsonrpc::Error {
            code: jsonrpc::ErrorCode::ContentModified,
            message: "Current diagnostic report is unavailable".into(),
            data: Some(serde_json::json!({ "reason": reason })),
        }
    }

    /// The direct response has its own actual provider receipt. It cannot
    /// replace a publisher completion or borrow a delivery acknowledgement.
    pub(in super::super) async fn ensure_current_diagnostic_report(
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

    pub(super) async fn publish_direct_diagnostic_report(
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
}

#[derive(Debug)]
pub(super) enum DirectDiagnosticProductionFailure {
    PendingWrite,
    Unavailable(DiagnosticUnavailable),
}

impl From<DiagnosticUnavailable> for DirectDiagnosticProductionFailure {
    fn from(reason: DiagnosticUnavailable) -> Self {
        Self::Unavailable(reason)
    }
}

impl DirectDiagnosticProductionFailure {
    pub(super) fn reason(self) -> DiagnosticUnavailable {
        match self {
            Self::PendingWrite => DiagnosticUnavailable::Cancelled,
            Self::Unavailable(reason) => reason,
        }
    }
}

pub(super) struct ProducedDirectDiagnosticReport {
    pub(super) receipt: DiagnosticCacheReceipt,
    pub(super) diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
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
        data_group_root: inputs.data_group_root.as_deref(),
        cross_file_resolution: inputs.toggles.xc.cross_file_resolution,
    }
}

impl Backend {
    /// Execute the existing checked provider computation for the direct report
    /// purpose. This never enqueues a push or issues a Delivered receipt.
    pub(super) async fn produce_direct_diagnostic_report(
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
            super::super::diagnostic_read_cancellation::DiagnosticReadCancellations::default();
        let base = Box::pin(super::super::compute_base_analysis_with_read_cancellations(
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
            super::super::compute_deep_diagnostic_report_with_read_cancellations(
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
