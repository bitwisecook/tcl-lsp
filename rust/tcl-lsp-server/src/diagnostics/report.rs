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

//! The report every path publishes: policy layers, findings and the lift onto the wire.

use std::collections::HashMap;
use std::sync::Arc;

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::{diagnostic_policy as core_policy, diagnostic_report as core_report};

use crate::configuration::PolicyLayers;
use crate::protocol::{lift_span, lsp_severity};
use crate::rewrite_wire;

use super::command_refinement::{
    cross_file_arity_diagnostics, refine_workspace_index_w123, refine_workspace_w123,
};
use super::package_refinement::refine_workspace_w120;
use super::{LiftInputs, RefinementInputs};

/// The policy one document's report is decided under: `layers` plus the
/// document facts — the byte evidence, the dialect's overlap table and the
/// directives. Reporting is on and the file is not excluded, because the
/// diagnostics pipeline settles both before it produces anything
/// (`run_diagnostics_master_off`, `run_diagnostics_excluded`).
pub(in super::super) fn document_policy(
    layers: &PolicyLayers,
    decode_report: Option<&tcl_lsp_core::source_decode::DecodeReport>,
    dialect: &'static tcl_dialect::DialectProfile,
    directives: core_policy::Directives,
) -> core_policy::Policy {
    layers
        .builder()
        .reporting(true)
        .decode(decode_report)
        .dialect(dialect)
        .directives(directives)
        .build()
}

/// One document's LSP publish set: [`published_report`] through
/// [`lift_report`].
///
/// The one function every report path renders through — the fast and deep
/// pushes, the pull and the F5 model report — so the paths differ only in
/// what they produced (`docs/design/compiler/diagnostic-policy.md`
/// § Adapters: "the three publish paths become one call each on the same
/// function").
pub(in super::super) fn lifted_report(
    doc: &core_report::DocumentSource<'_>,
    produced: Vec<core_policy::Finding>,
    layers: &PolicyLayers,
    directives: core_policy::Directives,
    analysis: Option<&AnalysisResult>,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    lifted_report_with_rewrite_source(doc, produced, layers, directives, analysis, None)
}

fn lifted_report_with_rewrite_source(
    doc: &core_report::DocumentSource<'_>,
    produced: Vec<core_policy::Finding>,
    layers: &PolicyLayers,
    directives: core_policy::Directives,
    analysis: Option<&AnalysisResult>,
    source: Option<&rewrite_wire::RewriteWireSource>,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    let report = published_report(doc, produced, layers, directives, analysis);
    let payloads = rewrite_wire::original_rewrite_payloads(doc, &report, analysis, source);
    lift_report_with_payloads(doc.text, &report, &payloads)
}

/// `produced` plus the report's own producers under `layers` and
/// `directives`, with the analyser's skip declared when an analyser ran.
///
/// The report every adapter reads: [`lifted_report`] renders it to LSP
/// diagnostics, and the code-action handler reads it directly, so a fix is
/// decided against the exact set the editor shows. `analysed` is false
/// where no Tcl analyser ran, so no skip is declared for it.
pub(in super::super) fn published_report(
    doc: &core_report::DocumentSource<'_>,
    produced: Vec<core_policy::Finding>,
    layers: &PolicyLayers,
    directives: core_policy::Directives,
    analysis: Option<&AnalysisResult>,
) -> core_policy::Report {
    let policy = document_policy(layers, doc.decode, doc.dialect, directives);
    let mut report = core_report::document_report_with_analysis(doc, produced, &policy, analysis);
    if analysis.is_some() {
        report.declare_analyser_skip(&policy);
    }
    report
}

/// The LSP adapter (`docs/design/compiler/diagnostic-policy.md` § Adapters):
/// the report's shown findings on the wire, and nothing else. A `Span`
/// becomes a UTF-16 `Range` through [`lift_span`], the resolved severity
/// goes through [`lsp_severity`], the tag the code table declares becomes
/// `Diagnostic.tags`, and an applicable rewrite
/// ([`core_policy::Report::applicable_rewrites`]) becomes the `data` payload
/// an editor applies: `{replacement, startOffset, endOffset}` for an
/// ungrouped rewrite, and `{group, edits}` — every member's edit, on every
/// member's diagnostic — for an optimisation group, whose edits apply
/// all-or-nothing (#2123, #2149). A `hint_only` rewrite, whose span covers
/// the whole consuming statement, and a group that lost a member to the
/// policy carry no payload. Nothing here decides anything, and nothing here
/// may: the three publish paths, the pull provider and the F5 report reach it
/// through [`lifted_report`], one call each.
#[cfg(test)]
pub(in super::super) fn lift_report(
    text: &str,
    report: &core_policy::Report,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    lift_report_with_payloads(text, report, &rewrite_payloads(report))
}

fn lift_report_with_payloads(
    text: &str,
    report: &core_policy::Report,
    payloads: &HashMap<*const core_policy::Finding, serde_json::Value>,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    use tower_lsp_server::ls_types::{DiagnosticTag, NumberOrString};
    let line_index = tcl_lexer::LineIndex::new_lsp(text);
    let mut diagnostics: Vec<_> = report
        .shown()
        .map(|shown| {
            let finding = shown.finding;
            let data = payloads
                .get(&std::ptr::from_ref(finding))
                .cloned()
                .or_else(|| finding.structured_data());
            tower_lsp_server::ls_types::Diagnostic {
                range: lift_span(text, &line_index, finding.span),
                severity: Some(lsp_severity(shown.severity)),
                code: Some(NumberOrString::String(finding.code.to_string())),
                code_description: None,
                source: Some("tcl-lsp".to_string()),
                message: finding.message.clone(),
                related_information: None,
                tags: shown.tag.map(|tag| {
                    vec![match tag {
                        tcl_core_types::DiagTag::Unnecessary => DiagnosticTag::UNNECESSARY,
                        tcl_core_types::DiagTag::Deprecated => DiagnosticTag::DEPRECATED,
                    }]
                }),
                data,
            }
        })
        .collect();
    if let Some(miss) = report.analysis_context_unavailable() {
        diagnostics.push(tower_lsp_server::ls_types::Diagnostic {
            range: tower_lsp_server::ls_types::Range::default(),
            severity: Some(tower_lsp_server::ls_types::DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(
                "ANALYSIS_CONTEXT_UNAVAILABLE".to_owned(),
            )),
            source: Some("tcl-lsp".to_owned()),
            message: miss.to_string(),
            data: report.analysis_context_status_data(),
            ..tower_lsp_server::ls_types::Diagnostic::default()
        });
    }
    diagnostics
}

/// Each applicable rewrite's wire payload, keyed by the member finding it
/// rides on (the report's own element, so identity is the key): the flat
/// edit for an ungrouped rewrite, and the whole group's edits on every
/// member of a group.
#[cfg(test)]
fn rewrite_payloads(
    report: &core_policy::Report,
) -> std::collections::HashMap<*const core_policy::Finding, serde_json::Value> {
    let edit = |finding: &core_policy::Finding| match finding.rewrite_payload() {
        Some((replacement, _, _)) => serde_json::json!({
            "replacement": replacement,
            "startOffset": finding.span.start(),
            "endOffset": finding.span.end(),
        }),
        _ => serde_json::Value::Null,
    };
    let mut out = std::collections::HashMap::new();
    for rewrite in report.applicable_rewrites() {
        match rewrite.group {
            None => out.extend(
                rewrite
                    .members
                    .iter()
                    .map(|member| (std::ptr::from_ref(*member), edit(member))),
            ),
            Some(group) => {
                let payload = serde_json::json!({
                    "group": group,
                    "edits": rewrite.members.iter().map(|member| edit(member)).collect::<Vec<_>>(),
                });
                out.extend(
                    rewrite
                        .members
                        .iter()
                        .map(|member| (std::ptr::from_ref(*member), payload.clone())),
                );
            }
        }
    }
    out
}

/// The findings of the analyser's own family: `diagnostics` converted, in
/// order — a conversion, nothing more.
pub(in super::super) fn analyser_findings(
    diagnostics: &[tcl_compiler::analyser::Diagnostic],
) -> Vec<core_policy::Finding> {
    diagnostics
        .iter()
        .cloned()
        .map(core_policy::Finding::from)
        .collect()
}

/// The compiler-checks pipeline's findings and the optimiser's, in the
/// order the lifts always published them: checks first, then rewrites.
pub(in super::super) fn compiler_findings(
    diags: &tcl_lsp_db::CompilerDiagnostics,
) -> Vec<core_policy::Finding> {
    diags
        .checks
        .iter()
        .cloned()
        .map(core_policy::Finding::from)
        .chain(
            diags
                .optimisations
                .iter()
                .cloned()
                .map(core_policy::Finding::from),
        )
        .collect()
}

/// The opt-in `f5-xc` XC100–XC301 translatability findings for an
/// `f5-irules` document — a conversion; the policy step decides what shows.
///
/// The translation walks the document's own analysis, its retained registry
/// and a unit built from that analysis, never a reparse under the standalone
/// iRules defaults; an analysis that cannot describe `source` states no XC
/// findings.
pub(in super::super) fn xc_findings(
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<core_policy::Finding> {
    let Some(registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    let Some(unit) =
        tcl_lsp_db::compilation_unit_uncached_from_analysis(source, registry, analysis, None)
    else {
        return Vec::new();
    };
    f5_xc::get_xc_diagnostics_for_document(source, &unit, analysis)
        .unwrap_or_default()
        .into_iter()
        .map(core_policy::Finding::from)
        .collect()
}

/// The findings a document publishes: the analyser's own set, the O111
/// hints over its W100s, the compiler checks' and the optimiser's, and —
/// for an `f5-irules` document with `xcDiagnostics` on (`xc_document` is its
/// analysis) — the XC translatability findings over `analysis_text`, the
/// document's analysis form (a lone `\r` rewritten), which every producer
/// reads.
///
/// The deep push's, the pull provider's and the code-action handler's
/// assembly of `produced`, in one place, so the two deliveries cannot
/// disagree and a lightbulb decides every fix against the exact set the
/// document publishes (`docs/design/compiler/diagnostic-policy.md`
/// § Adapters, code actions).
pub(in super::super) fn published_findings(
    analyser_diags: &[tcl_compiler::analyser::Diagnostic],
    compiler_diags: &tcl_lsp_db::CompilerDiagnostics,
    xc_document: Option<&AnalysisResult>,
    analysis_text: &str,
) -> Vec<core_policy::Finding> {
    let mut produced = analyser_findings(analyser_diags);
    let hints = core_report::brace_expr_hints(&produced);
    produced.extend(hints);
    produced.extend(compiler_findings(compiler_diags));
    if let Some(analysis) = xc_document {
        produced.extend(xc_findings(analysis_text, analysis));
    }
    produced
}

/// Refine the analyser's single-file W120 against the workspace package
/// database, then lift the analyser / compiler / source-style / XC
/// diagnostics into LSP diagnostics on a `spawn_blocking` worker.  Returns the
/// join result so the caller distinguishes a worker panic from a clean set.
pub(super) async fn refine_and_lift_diagnostics(
    analysis: &Arc<AnalysisResult>,
    analyser_diags: Vec<tcl_compiler::analyser::Diagnostic>,
    compiler_diags: &Arc<tcl_lsp_db::CompilerDiagnostics>,
    refinement: &RefinementInputs<'_>,
    inputs: &LiftInputs<'_>,
) -> Result<Vec<tower_lsp_server::ls_types::Diagnostic>, crate::rt::JoinError> {
    // Refine the analyser's single-file W120 against the workspace package
    // database and the requires inherited from entry points / `source`
    // ancestors (shared with the pull path via `refine_workspace_w120`).
    let analyser_diags = refine_workspace_w120(
        analyser_diags,
        analysis.as_ref(),
        refinement.inheritance,
        refinement.package_resolver,
        refinement.provider_sources,
    )
    .await;
    // Drop any W123 (unknown command) the package database can resolve —
    // an auto-loaded library command (`tclIndex`) or a command an available
    // package's implementation defines. Always on, like the W120 refinement.
    let analyser_diags = refine_workspace_w123(
        analyser_diags,
        analysis.as_ref(),
        refinement.inheritance,
        refinement.package_resolver,
        refinement.provider_sources,
        inputs.dialect,
    )
    .await;
    // And any W123 backed by an exact source-owned sibling declaration.
    let mut analyser_diags =
        refine_workspace_index_w123(analyser_diags, analysis.as_ref(), &refinement.settled_calls);
    // A cross-file call that resolved to a workspace proc with a bad
    // argument count is the same error as the same-file one, reported with the
    // same codes. Emitted after the W123 refinement because it replaces that
    // W123 with something more specific — the command is not unknown, the call
    // is wrong.
    analyser_diags.extend(cross_file_arity_diagnostics(
        analysis.as_ref(),
        &refinement.settled_calls,
        |code| inputs.disabled.iter().any(|c| c == code),
    ));

    let rewrite_source = inputs.rewrite_source.clone();
    let analysis_lifts = Arc::clone(analysis);
    let lift_text = inputs.text.to_owned();
    let decode_report = inputs.decode_report;
    let policy_layers = inputs.policy_layers.clone();
    let style_line_length = inputs.style_line_length;
    let dialect = inputs.dialect;
    let xc_for_irules = inputs.xc_diagnostics && inputs.dialect.is_irules();
    let compiler_diags = Arc::clone(compiler_diags);
    let workspace_sources = Arc::clone(refinement.workspace_sources);
    crate::rt::spawn_blocking(move || {
        super::f5_models::refine_signed_data_group_diagnostics(
            &mut analyser_diags,
            workspace_sources.configuration_texts(),
        );
        // Every producer reads the *analysis* form — a lone `\r` terminates a
        // command for `tclsh` — while the spans lift against the client's
        // buffer, which the rewrite leaves byte-for-byte the same length.
        let analysis_text = tcl_lexer::normalise_lone_cr(&lift_text);
        // `analyser_diags` includes opt-in callback checks when enabled; direct
        // cross-file verdicts have already been settled by the workspace index.
        // The pull path's assembly, shared: the O111 hints over the analyser's
        // W100s, the compiler checks and the optimiser's rewrites, then —
        // opt-in — the XC100-301 translatability findings for `f5-irules`
        // documents. The source-style pass and, for a `.sslictcl` document,
        // the loader's findings are the report's own.
        let produced = published_findings(
            &analyser_diags,
            &compiler_diags,
            xc_for_irules.then_some(analysis_lifts.as_ref()),
            &analysis_text,
        );
        let doc = core_report::DocumentSource {
            text: &lift_text,
            analysis_text: &analysis_text,
            decode: decode_report.as_ref(),
            dialect,
            pass: core_report::SourcePass::Tcl {
                line_length: style_line_length as usize,
            },
        };
        lifted_report_with_rewrite_source(
            &doc,
            produced,
            &policy_layers,
            core_policy::Directives::from_analysis(&analysis_lifts, &lift_text),
            Some(&analysis_lifts),
            rewrite_source.as_ref(),
        )
    })
    .await
}
