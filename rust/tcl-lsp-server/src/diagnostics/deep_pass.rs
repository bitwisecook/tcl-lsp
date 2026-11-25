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

//! The deep pass: analyser, compiler checks and cross-file resolution joined into one report.

use std::ops::ControlFlow;
use std::sync::Arc;

use futures_util::future::FutureExt;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;
use tower_lsp_server::ls_types::Uri;

use crate::current_diagnostics::DiagnosticUnavailable;
use crate::documents::DocumentStore;
use crate::workspace_diagnostic_sources::WorkspaceDiagnosticSources;
use crate::{diagnostic_read_cancellation, provider_source};

use super::analysis_passes::{
    compute_compiler_diags_after_base_with_read_cancellations,
    compute_project_diags_with_read_cancellations, unavailable_compiler_diagnostics,
};
use super::command_refinement::settle_cross_file_calls;
use super::consumers::WorkspaceCommandQueries;
use super::delivery::DeliveryCtx;
use super::package_refinement::compute_source_inheritance;
use super::publication::publish_diagnostics_result_with_sources;
use super::report::refine_and_lift_diagnostics;
use super::run::FastTierGate;
use super::{
    AnalyserPathInputs, CrossFileCalls, DiagnosticPublicationSources, LiftInputs, PublishTiming,
    RefinementInputs, SalsaAnalysisCtx, SourceInheritance,
};

pub(in super::super) fn deep_provider_completion(
    _uri: &Uri,
    cross_file_resolution: bool,
    project: Option<tcl_lsp_db::Project>,
    compiler_result: &ControlFlow<bool, Option<Arc<tcl_lsp_db::CompilerDiagnostics>>>,
    project_result: &ControlFlow<bool, Option<Vec<tcl_compiler::analyser::Diagnostic>>>,
) -> Result<(), DiagnosticUnavailable> {
    if cross_file_resolution && project.is_none() {
        Err(DiagnosticUnavailable::Context)
    } else {
        match (compiler_result, project_result) {
            (ControlFlow::Continue(None), _) => Err(DiagnosticUnavailable::Context),
            (ControlFlow::Continue(Some(_)), ControlFlow::Continue(_)) => Ok(()),
            _ => Err(DiagnosticUnavailable::ProviderError),
        }
    }
}

type DeepWorkspaceDiagnostics = (
    bool,
    SourceInheritance,
    CrossFileCalls,
    Arc<WorkspaceDiagnosticSources>,
);

async fn deep_workspace_diagnostics(
    uri: &Uri,
    documents: &Arc<DocumentStore>,
    analysis: &AnalysisResult,
    analyser_diags: &[tcl_compiler::analyser::Diagnostic],
    inputs: &AnalyserPathInputs<'_>,
) -> ControlFlow<(bool, DiagnosticUnavailable), DeepWorkspaceDiagnostics> {
    let needs_inheritance = analyser_diags
        .iter()
        .any(|d| d.code == DiagCode::W120 || d.code == DiagCode::W123);
    let needs_settling = !analysis.unresolved_command_sites.is_empty()
        || (analyser_diags.iter().any(|d| d.code == DiagCode::W123)
            && analysis
                .command_invocations
                .iter()
                .any(|invocation| invocation.is_mathfunc_call));
    let data_group_root = inputs.data_group_root.filter(|_| {
        analyser_diags
            .iter()
            .any(|diagnostic| diagnostic.signed_data_group_comparison.is_some())
    });
    let (inheritance, settled_calls, workspace_sources) =
        if needs_inheritance || needs_settling || data_group_root.is_some() {
            let (docs, index) = loop {
                let docs = documents.lock("workspace_diagnostic_source_query").await;
                if let Ok(index) = inputs.workspace_index.try_read() {
                    break (docs, index);
                }
                drop(docs);
                drop(inputs.workspace_index.read().await);
            };
            let settled_calls = if needs_settling {
                settle_cross_file_calls(&index, analysis, &inputs.registry, uri.as_str())
            } else {
                CrossFileCalls::default()
            };
            let published = inputs
                .provider_capture
                .source_path_admissions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let mut workspace_sources = match WorkspaceDiagnosticSources::capture(
                &settled_calls.source_candidates,
                &index,
                &docs,
                &published,
                inputs.provider_capture.store.as_ref(),
            ) {
                Ok(sources) => sources,
                Err(reason) => {
                    return ControlFlow::Break((
                        !matches!(reason, DiagnosticUnavailable::Pending),
                        reason,
                    ));
                }
            };
            if let Some(root) = data_group_root
                && let Err(reason) = workspace_sources.capture_configurations(
                    &docs,
                    root,
                    inputs.provider_capture.store.as_ref(),
                )
            {
                return ControlFlow::Break((true, reason));
            }
            (
                if needs_inheritance {
                    compute_source_inheritance(
                        &index,
                        uri,
                        analysis,
                        inputs.entry_points,
                        inputs.folder_root,
                        inputs.package_prefer,
                    )
                } else {
                    SourceInheritance::default()
                },
                settled_calls,
                workspace_sources,
            )
        } else {
            (
                SourceInheritance::default(),
                CrossFileCalls::default(),
                WorkspaceDiagnosticSources::default(),
            )
        };
    let mut workspace_sources = workspace_sources;
    if let Err(reason) = workspace_sources
        .read_saved(inputs.provider_capture.store, inputs.script_encoding)
        .await
    {
        return ControlFlow::Break((true, reason));
    }
    let workspace_sources = Arc::new(workspace_sources);
    ControlFlow::Continue((
        needs_inheritance,
        inheritance,
        settled_calls,
        workspace_sources,
    ))
}

/// The indexed deep diagnostics pass runs the analyser, compiler checks and
/// cross-file resolution concurrently, then refines and publishes their joint
/// findings. An unindexed compiler build depends on the shared base analysis's
/// complete input; it does not start a separate profile-derived analysis.
/// Indexed secondary work can finish after base cancellation, but publication
/// remains guarded by the same document currency.
///
/// `base` is the per-file analyser walk, supplied by the caller (as a `Shared`
/// future) rather than started here, so the progressive fast tier can await the
/// *same* computation instead of issuing a second one; it is simply the first arm
/// of the `join!`. The optional [`FastTierGate`] holds the publication behind a
/// fast tier the same task is still delivering.
///
/// Returns whether the version **settled** (published or superseded) —
/// `false` only on a genuine salsa cancellation. A deterministic worker panic
/// in a *secondary* pass (compiler / cross-file) degrades that pass to its
/// empty / per-file fallback and still publishes the currency-guarded deep
/// tier, so the fast tier — which may already have replaced the client's set
/// with its reduced subset — is never left as the terminal state.
pub(super) async fn run_deep_diagnostics(
    delivery: &DeliveryCtx<'_>,
    salsa_ctx: &SalsaAnalysisCtx<'_>,
    lift_inputs: &LiftInputs<'_>,
    inputs: &AnalyserPathInputs<'_>,
    project: Option<tcl_lsp_db::Project>,
    timing: PublishTiming<'_>,
    (base, gate): (
        impl std::future::Future<Output = ControlFlow<bool, Arc<AnalysisResult>>>,
        Option<&FastTierGate>,
    ),
) -> bool {
    let produced = Box::pin(compute_deep_diagnostic_report(
        delivery.uri,
        delivery.documents,
        salsa_ctx,
        lift_inputs,
        inputs,
        project,
        base,
    ))
    .await;
    if let Some(gate) = gate {
        gate.pass().await;
    }
    let produced = match produced {
        ControlFlow::Continue(produced) => produced,
        ControlFlow::Break((settled, reason)) => {
            delivery.diagnostic_pass_unavailable(reason).await;
            return settled;
        }
    };
    publish_diagnostics_result_with_sources(
        delivery,
        inputs.workspace_index,
        inputs.rehomed_source_seeds,
        inputs.rehoming_gate,
        &produced.analysis,
        DiagnosticPublicationSources {
            result: (
                produced.provider_completion,
                produced.result,
                Some(produced.command_queries),
            ),
            workspace_sources: Some(produced.workspace_sources),
        },
        timing,
    )
    .await
}

/// One actual configured provider pass, before either push publication or a
/// direct cache response. Availability remains separate from display fallback.
pub(in super::super) struct ProducedDiagnosticReport {
    pub(in super::super) workspace_sources: Arc<WorkspaceDiagnosticSources>,
    pub(in super::super) analysis: Arc<AnalysisResult>,
    pub(in super::super) command_queries: WorkspaceCommandQueries,
    pub(in super::super) provider_completion: Result<(), DiagnosticUnavailable>,
    pub(in super::super) result:
        Result<Vec<tower_lsp_server::ls_types::Diagnostic>, crate::rt::JoinError>,
    pub(in super::super) provider_sources: provider_source::ConfiguredProviderSourceReport,
}

pub(in super::super) async fn compute_deep_diagnostic_report(
    uri: &Uri,
    documents: &Arc<DocumentStore>,
    salsa_ctx: &SalsaAnalysisCtx<'_>,
    lift_inputs: &LiftInputs<'_>,
    inputs: &AnalyserPathInputs<'_>,
    project: Option<tcl_lsp_db::Project>,
    base: impl std::future::Future<Output = ControlFlow<bool, Arc<AnalysisResult>>>,
) -> ControlFlow<(bool, DiagnosticUnavailable), ProducedDiagnosticReport> {
    compute_deep_diagnostic_report_with_read_cancellations(
        uri,
        documents,
        salsa_ctx,
        lift_inputs,
        inputs,
        project,
        (base, None),
    )
    .await
}

pub(in super::super) async fn compute_deep_diagnostic_report_with_read_cancellations(
    uri: &Uri,
    documents: &Arc<DocumentStore>,
    salsa_ctx: &SalsaAnalysisCtx<'_>,
    lift_inputs: &LiftInputs<'_>,
    inputs: &AnalyserPathInputs<'_>,
    project: Option<tcl_lsp_db::Project>,
    production: (
        impl std::future::Future<Output = ControlFlow<bool, Arc<AnalysisResult>>>,
        Option<&diagnostic_read_cancellation::DiagnosticReadCancellations>,
    ),
) -> ControlFlow<(bool, DiagnosticUnavailable), ProducedDiagnosticReport> {
    if !inputs.sidecar_revision.is_published_current() {
        return ControlFlow::Break((true, DiagnosticUnavailable::Stale));
    }
    let (base, cancellations) = production;
    let base = base.shared();
    let (base_result, compiler_result, project_result) = tokio::join!(
        base.clone(),
        compute_compiler_diags_after_base_with_read_cancellations(
            salsa_ctx,
            &inputs.registry,
            inputs.generic_variable_patterns,
            base,
            cancellations,
        ),
        compute_project_diags_with_read_cancellations(salsa_ctx, project, cancellations),
    );
    if !inputs.sidecar_revision.is_published_current() {
        return ControlFlow::Break((true, DiagnosticUnavailable::Stale));
    }
    let provider_completion = deep_provider_completion(
        uri,
        inputs.cross_file_resolution,
        project,
        &compiler_result,
        &project_result,
    );
    diagnostic_read_cancellation::terminal_if_context(cancellations, provider_completion);
    let analysis: Arc<AnalysisResult> = match base_result {
        ControlFlow::Continue(a) => a,
        ControlFlow::Break(settled) => {
            return ControlFlow::Break((
                settled,
                if settled {
                    DiagnosticUnavailable::ProviderError
                } else {
                    DiagnosticUnavailable::Cancelled
                },
            ));
        }
    };
    let compiler_diags: Arc<tcl_lsp_db::CompilerDiagnostics> = match compiler_result {
        ControlFlow::Continue(Some(d)) => d,
        // Cancellation → retry the latest state.
        ControlFlow::Break(false) => {
            return ControlFlow::Break((false, DiagnosticUnavailable::Cancelled));
        }
        // Deterministic worker panic → degrade to no compiler diags but STILL
        // publish below. The fast tier may already have replaced the client's
        // complete set with its reduced subset; an early return here would leave
        // that reduced set as the terminal state (settled ⇒ no retry) —
        // stripping the O1xx/refined-W12x/cross-file findings the user had. The
        // degraded deep publish is still a strict superset of the fast tier, just
        // without the compiler/optimiser hints.
        ControlFlow::Continue(None) | ControlFlow::Break(true) => {
            unavailable_compiler_diagnostics()
        }
    };
    // `Some` adds the opt-in project callback-arity checks
    // (`crossFileResolution` on, salsa input present). Direct cross-file calls
    // are settled once against the workspace index below. `None` falls back to
    // the per-file set, as does a deterministic worker panic (`Break(true)`),
    // which still publishes rather than stranding the fast tier. `Break(false)`
    // is cancellation.
    let project_command_queries = match &project_result {
        ControlFlow::Continue(None) if project.is_none() => WorkspaceCommandQueries::NoRead,
        _ => WorkspaceCommandQueries::Broad,
    };
    let analyser_diags: Vec<_> = match project_result {
        ControlFlow::Continue(Some(d)) => d,
        ControlFlow::Continue(None) | ControlFlow::Break(true) => analysis.diagnostics.clone(),
        ControlFlow::Break(false) => {
            return ControlFlow::Break((false, DiagnosticUnavailable::Cancelled));
        }
    };

    // What the workspace `source` graph contributes to this document (in both
    // the ancestor and the `source`-descendant direction), and its call sites
    // settled against the workspace index. Both read the index, so they share
    // one lock acquisition.
    //
    // Demanded only when they can change a verdict: the inheritance when there
    // is a W120 or W123 to refine, the settled calls when the analyser
    // recorded any unresolved site at all. A document with neither pays
    // nothing.
    let (needs_inheritance, inheritance, settled_calls, workspace_sources) =
        match deep_workspace_diagnostics(uri, documents, &analysis, &analyser_diags, inputs).await {
            ControlFlow::Continue(sources) => sources,
            ControlFlow::Break(reason) => return ControlFlow::Break(reason),
        };
    let command_queries = project_command_queries.join(settled_calls.command_queries);
    let provider_sources = inputs
        .provider_capture
        .capture(inputs.package_resolver, needs_inheritance)
        .await;
    let provider_access = provider_source::ProviderSourceAccess::Supplied(&provider_sources);
    let result = refine_and_lift_diagnostics(
        &analysis,
        analyser_diags,
        &compiler_diags,
        &RefinementInputs {
            inheritance: &inheritance,
            package_resolver: inputs.package_resolver,
            provider_sources: &provider_access,
            settled_calls,
            workspace_sources: &workspace_sources,
        },
        lift_inputs,
    )
    .await;

    ControlFlow::Continue(ProducedDiagnosticReport {
        workspace_sources,
        analysis,
        command_queries,
        provider_completion,
        result,
        provider_sources: provider_source::ConfiguredProviderSourceReport::from_capture(
            provider_sources,
        ),
    })
}
