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

//! Fix all safe issues: repeated passes of the provably safe fixes.

use super::{
    Arc, Backend, HashSet, analyser_findings, core_code_actions, core_policy, document_policy,
    document_transform, jsonrpc,
};

/// How many re-analyse-and-apply rounds `tcl-lsp.fixAllSafeIssues` runs.
///
/// Each round applies a batch of non-overlapping fixes and re-analyses, so a
/// fix a previous edit unblocked (or a diagnostic a previous edit revealed)
/// is picked up next time round.  The loop also stops early as soon as a
/// round changes nothing, so this is the guard against a pathological
/// oscillation — two fixes that keep undoing each other — not the normal
/// exit.
const FIX_ALL_MAX_PASSES: usize = 4;

/// One fix selected for a `tcl-lsp.fixAllSafeIssues` pass.
///
/// A named struct rather than a tuple: five same-typed fields indexed
/// positionally (`f.0`, `f.2`) in the apply loop is exactly where getting one
/// wrong silently rewrites the wrong bytes.
struct BulkFix {
    /// Inclusive start byte offset of the replaced range.
    pub(in super::super) start: u32,
    /// Exclusive end byte offset of the replaced range.
    pub(in super::super) end: u32,
    /// Replacement text.
    pub(in super::super) new_text: String,
    /// The diagnostic code the fix came from, for the `applied` report.
    pub(in super::super) code: String,
    /// The fix's own description, for the `applied` report.
    description: String,
    /// The fix's safety class, for the `applied` report — always the
    /// semantics-equivalent one here, reported so a caller can see *why* the
    /// fix qualified rather than having to trust the command's name.
    safety: &'static str,
}

impl Backend {
    /// Handle `tcl-lsp.fixAllSafeIssues`: apply every non-overlapping
    /// **provably semantics-preserving** diagnostic fix, iteratively, until
    /// the source stabilises.
    ///
    /// "Safe" is a property of the individual fix, read from the
    /// [`FixSafety`](tcl_compiler::analyser::FixSafety) the emitter attached
    /// to it — not of the diagnostic's code.  A whitelist of codes (`W100`,
    /// `W110`, …) would promise a guarantee it cannot establish: the same code
    /// emits an equivalent rewrite for one call and a behaviour-changing one
    /// for the next.  Under C Tcl 9.0.3:
    ///
    /// ```tcl
    /// set a {$x}; set x 3; set b 2
    /// puts [expr $a + $b]          ;# 5 — W100's brace fix makes this an error
    /// puts [expr {"1" == "01"}]    ;# 1 — W110's `eq` rewrite makes this 0
    /// ```
    ///
    /// Both are valuable fixes — they are offered as their own named code
    /// actions — but neither belongs in an unattended bulk pass.
    ///
    /// Per pass: re-analyse (so a fix whose proof depended on text an earlier
    /// pass rewrote is re-derived rather than re-used), decide the findings
    /// under the document's policy, take each *shown* finding's first
    /// bulk-applicable fix, drop any whose span overlaps one already chosen,
    /// and apply the rest from the end of the buffer backwards so earlier
    /// offsets stay valid.  A fix is applied for a shown finding and for no
    /// other (`docs/design/compiler/diagnostic-policy.md` § Adapters, code
    /// actions): a code a layer turns off — computed or not — a `# noqa` over
    /// the command, and an abstaining document's findings all apply nothing.
    pub(in super::super) async fn fix_all_safe_issues_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(request) = document_transform::TransformRequest::parse(args.first()) else {
            return Ok(None);
        };
        let uri = request.uri().clone();
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let source_receipt = self.capture_transform_source(&request, &doc).await?;
        let na_mode = *self.non_ascii_mode.lock().await;
        // The document's own layers decide what shows, and their production
        // skip is what the analyser leaves uncomputed, so a code a folder
        // turns back on is computed for it and a gap is one the report can
        // explain.
        let layers = self.resolved_policy_layers(&uri).await;
        let disabled = layers.production_skip();
        let decode_report = doc.decode_report;
        let extra: HashSet<String> = self.extra_commands.lock().await.iter().cloned().collect();
        let packs = self.spec_packs().await;
        let resource = self.resource_analyser_inputs(Some(&uri)).await;
        let dialect = doc.dialect.clone();
        let worker_configuration = Arc::new(document_transform::FixAllWorkerConfiguration {
            disabled,
            na_mode,
            extra,
            packs,
            resource,
            policy: layers,
            dialect,
            decode_report,
        });
        // The one place that legitimately owns a copy: fix-all *rewrites* the
        // source in a loop, so it needs a mutable buffer of its own rather than
        // a share of the immutable snapshot.  It copies from `raw()` because
        // the fixed-up buffer is handed back to the client verbatim, so it
        // carries the document's real terminators; the analyser that locates
        // the fixes still sees the normalised form.  The rewrite is
        // byte-length preserving, so a fix span resolved against one applies
        // unchanged to the other.
        let mut source = doc.raw().to_owned();
        let (value, workers) = crate::rt::spawn_blocking(move || {
            let mut workers = Vec::new();
            let mut worker_sources_complete = true;
            let mut applied: Vec<serde_json::Value> = Vec::new();
            for _ in 0..FIX_ALL_MAX_PASSES {
                let analysis_source = tcl_lexer::normalise_lone_cr(&source);
                let (analysis, worker) = document_transform::FixAllWorkerConfiguration::analyse(
                    &worker_configuration,
                    &analysis_source,
                );
                if let Some(worker) = worker {
                    workers.push(worker);
                } else {
                    worker_sources_complete = false;
                }
                let policy = document_policy(
                    &worker_configuration.policy,
                    worker_configuration.decode_report.as_ref(),
                    tcl_lsp_core::profile_for_dialect(&worker_configuration.dialect),
                    core_policy::Directives::from_analysis(&analysis, &source),
                );
                let report = core_policy::apply(analyser_findings(&analysis.diagnostics), &policy);
                let Some(current) = core_code_actions::DiagnosticEditSource::for_analysis(
                    &analysis_source,
                    &analysis,
                ) else {
                    break;
                };
                let chosen = Self::bulk_applicable_fixes(&report, &current);
                if chosen.is_empty() {
                    break;
                }
                // Apply from the end so earlier byte offsets stay valid.
                let mut new_source = source.clone();
                for fix in chosen.iter().rev() {
                    let (start, end) = (fix.start as usize, fix.end as usize);
                    if start <= end && end <= new_source.len() {
                        new_source.replace_range(start..end, &fix.new_text);
                    }
                }
                if new_source == source {
                    break;
                }
                for fix in &chosen {
                    applied.push(serde_json::json!({
                        "code": fix.code,
                        "description": fix.description,
                        "safety": fix.safety,
                    }));
                }
                source = new_source;
            }
            (
                serde_json::json!({ "source": source, "applied": applied }),
                worker_sources_complete.then_some(workers),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("fix-all worker panicked: {err}").into(),
            data: None,
        })?;
        self.finish_document_transform(source_receipt, workers.unwrap_or_default(), value)
            .await
    }

    /// The fixes one "Fix All Safe Issues" pass may apply from `report`:
    /// every shown finding's first [`FixSafety::is_bulk_applicable`] fix, in
    /// ascending start order, with overlapping fixes dropped.
    ///
    /// Three filters, in order, over what the report shows:
    ///
    /// 1. **Provably equivalent only.** A fix is taken only when its emitter
    ///    classified *that instance* as semantics-preserving.  A diagnostic
    ///    whose only fixes are behaviour-changing contributes nothing — its
    ///    fixes stay available as individually-named code actions.
    /// 2. **Non-empty edit.** A zero-width fix inserting nothing cannot make
    ///    progress and would keep the pass loop spinning.
    /// 3. **Non-overlapping.** Two fixes whose spans touch cannot both be
    ///    applied against the same coordinate space; the earlier-starting one
    ///    wins and the other is left for the next pass, where it is
    ///    re-derived from the rewritten source (so a fix whose proof the
    ///    first edit invalidated is simply not re-offered).
    fn bulk_applicable_fixes(
        report: &core_policy::Report,
        current: &core_code_actions::DiagnosticEditSource<'_>,
    ) -> Vec<BulkFix> {
        let mut candidates: Vec<BulkFix> = report
            .shown()
            .filter_map(|shown| {
                let finding = shown.finding;
                if !current.matches_finding(finding) {
                    return None;
                }
                let fix = finding.fixes.iter().find(|fix| {
                    fix.safety.is_bulk_applicable() && current.contains_span(fix.span)
                })?;
                (fix.span.end() > fix.span.start() || !fix.new_text.is_empty()).then(|| BulkFix {
                    start: fix.span.start(),
                    end: fix.span.end(),
                    new_text: fix.new_text.clone(),
                    code: finding.code.to_string(),
                    description: fix.description.clone(),
                    safety: fix.safety.as_str(),
                })
            })
            .collect();
        candidates.sort_by_key(|fix| (fix.start, fix.end));
        let mut chosen: Vec<BulkFix> = Vec::new();
        let mut last_end = 0u32;
        for fix in candidates {
            if fix.start >= last_end {
                last_end = fix.end.max(fix.start);
                chosen.push(fix);
            }
        }
        chosen
    }
}
