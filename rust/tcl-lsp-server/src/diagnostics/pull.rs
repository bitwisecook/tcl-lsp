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

//! Reports computed on request: the pull path and the synchronous publish.

#[cfg(test)]
use super::DocumentSourceResult;
#[cfg(test)]
use super::F5ModelDocument;
#[cfg(test)]
use super::F5PullInputs;
use super::command_refinement::{
    cross_file_arity_diagnostics, refine_workspace_index_w123, refine_workspace_w123,
    settle_cross_file_calls,
};
#[cfg(test)]
use super::core_policy;
#[cfg(test)]
use super::core_report;
#[cfg(test)]
use super::f5_models::f5_dialect_diagnostics;
#[cfg(test)]
use super::f5_models::f5_model_report;
#[cfg(test)]
use super::f5_models::is_apl_source;
use super::package_refinement::refine_workspace_w120;
#[cfg(test)]
use super::publisher::PackNoticeContext;
#[cfg(test)]
use super::report::lifted_report;
#[cfg(test)]
use super::report::published_findings;
use super::run::run_diagnostics_core;
use super::{
    AnalysisResult, Arc, Backend, CommandRegistry, CrossFileCalls, DiagCode, HashSet,
    PullRefinementInputs, SourceInheritance, Uri, provider_source,
};

/// Monotonic `result_id` for the pull-diagnostic cache.  A fresh id each time a
/// document's diagnostics change lets a client's `previousResultId` be compared
/// for the `Unchanged` short-circuit.
pub(in super::super) fn next_pull_diag_result_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("pd-{id}")
}

impl Backend {
    /// Build the **complete** diagnostic set for a document — analyser +
    /// compiler/optimiser + source-style — exactly as the push path
    /// ([`run_diagnostics_core`]) assembles it. The pull handler
    /// (`textDocument/diagnostic`) uses this so an editor on the pull path is
    /// not silently missing the compiler, optimiser, and source-style
    /// warnings that `publish_diagnostics` adds (the capability is advertised
    /// unconditionally, so `vscode-languageclient` and peers may choose pull).
    ///
    /// Compiler checks run **uncached** here: pull requests are on demand
    /// (hover/save), not per-keystroke, so they need no salsa file handle, and
    /// the analyser result still comes through the shared [`Self::analysis_for`]
    /// cache.
    /// Project callback signature advice for this exact supplied source and
    /// analysis, or `None` when `crossFileResolution` is off. The shared
    /// database projection retains original prefix/target evidence and source
    /// barriers; a missing producer cannot use nominal arities as a fallback.
    /// Header queries run on a worker because a cold project can demand each
    /// file's signature inventory.
    pub(in super::super) async fn project_callback_diagnostics_if(
        &self,
        cross_file_on: bool,
        source: &str,
        analysis: &Arc<AnalysisResult>,
        disabled: &HashSet<String>,
    ) -> Option<Vec<tcl_compiler::analyser::Diagnostic>> {
        // naming.database.original-project-callback-projection
        // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
        if !cross_file_on {
            return None;
        }
        let sidecar_revision = self
            .fresh_analysis_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .published_revision()?;
        // Resolve everything else *before* cloning the database, and clone it
        // only on the branch that immediately hands it to a worker.
        //
        // A salsa snapshot is not an ordinary handle. `set_text` (and creating a
        // `SourceFile`) goes through `Storage::cancel_others`, which parks on a
        // condvar until every **other** clone has been dropped — a blocking
        // wait, taken on a Tokio worker. Deferred publication waits for the
        // tracked census to become empty first, so a clone retained past its
        // query holds publication back and can stall every request handler in
        // the server.
        //
        // Holding one across an unrelated `.await` — the `db_project` lock, or
        // the whole function on the `None` branch — widens that window for no
        // benefit. Cloning last keeps the snapshot's life exactly the worker's.
        let project = *self.db_project.lock().await;
        let project = project?;
        let source = source.to_owned();
        let analysis = Arc::clone(analysis);
        let disabled = disabled.clone();
        let db = self.db.snapshot("project_callback_diagnostics_if").await;
        crate::rt::spawn_blocking(move || {
            // The snapshot and retained analysis are read-only. Cancellation
            // discards this projection before any diagnostics are published.
            salsa::Cancelled::catch(std::panic::AssertUnwindSafe(|| {
                tcl_lsp_db::project_callback_diagnostics_for_analysis_with_inputs(
                    &*db,
                    project,
                    &source,
                    &analysis,
                    |code| disabled.contains(code),
                )
            }))
            .ok()
        })
        .await
        .ok()
        .flatten()
        .filter(|_| sidecar_revision.is_published_current())
    }

    /// Run the compiler-check diagnostics for `text` off the event-loop thread.
    #[cfg(test)]
    pub(in super::super) async fn compiler_diagnostics_for(
        &self,
        uri: &Uri,
        text: &str,
        analysis: &Arc<AnalysisResult>,
        registry: &Arc<CommandRegistry>,
    ) -> tcl_lsp_db::CompilerDiagnostics {
        let (c_text, c_analysis, c_registry) =
            (text.to_owned(), Arc::clone(analysis), Arc::clone(registry));
        // URI-scoped (folder/project override aware), matching the push
        // path's `resolved_generic_variable_patterns(uri)` so IRULE4002
        // honours a folder's `diagnostics.genericVariablePatterns` override.
        let c_generic = self.resolved_generic_variable_patterns(uri).await;
        // The uncached supplied-input build retains the document's analysis;
        // project call-site evidence remains an independent producer input.
        let c_evidence = self.cross_file_evidence_for(uri).await;
        crate::rt::spawn_blocking(move || {
            tcl_lsp_db::compiler_check_diagnostics_uncached_from_analysis(
                &c_text,
                &c_registry,
                &c_analysis,
                c_generic.as_deref(),
                c_evidence.as_deref(),
            )
        })
        .await
        .unwrap_or_else(|_| tcl_lsp_db::CompilerDiagnostics {
            checks: Vec::new(),
            optimisations: Vec::new(),
            taint_replays: tcl_compiler::taint::TaintReplayAnalysis::unavailable(),
        })
    }

    /// The pull-path report for an F5 **model** dialect — BIG-IP config or
    /// iApp APL presentation text, which is not Tcl source and has its own
    /// validators rather than the analyser.
    ///
    /// Split out of [`Self::full_diagnostics_for`] because it is a complete
    /// alternative report rather than a step of one: it shares only the
    /// resolved settings, and keeping it inline made the Tcl path harder to
    /// read than either is on its own.
    #[cfg(test)]
    pub(super) async fn f5_pull_report(
        &self,
        uri: &Uri,
        text: &str,
        analysis_text: &str,
        language_id: &str,
        inputs: &F5PullInputs<'_>,
    ) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
        let encoding_abstains =
            tcl_lsp_core::source_decode::should_abstain(inputs.decode_report.as_ref());
        let produced = if encoding_abstains {
            Vec::new()
        } else {
            f5_dialect_diagnostics(
                uri,
                analysis_text,
                inputs.profile,
                language_id,
                &self.documents,
                self.store.as_ref(),
            )
            .await
            .unwrap_or_default()
        };
        f5_model_report(
            &F5ModelDocument {
                text,
                analysis_text,
                decode_report: inputs.decode_report.as_ref(),
                dialect: inputs.profile,
            },
            inputs.policy_layers,
            produced,
        )
    }

    /// The pull path's whole report for `uri` — the analysed set, unioned with
    /// any pack-load notices standing on the file.
    ///
    /// The union is what the push path publishes (see
    /// [`DiagnosticPublisher::with_pack_notices`]), and a pulled report has to
    /// match a pushed one diagnostic for diagnostic.
    #[cfg(test)]
    pub(in super::super) async fn full_diagnostics_for(
        &self,
        uri: &Uri,
        text: Arc<str>,
        dialect: String,
        language_id: &str,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::Diagnostic>> {
        let analysed = self
            .analysed_diagnostics_for(uri, Arc::clone(&text), dialect.clone(), language_id)
            .await?;
        let (pack_generation, pack_set) = {
            let published = self.spec_packs.lock().await;
            (published.seq, Arc::clone(&published.packs))
        };
        let registry = self.registry_for_dialect(&dialect).await;
        Ok(self.diagnostic_publisher.with_pack_notices(
            PackNoticeContext {
                uri,
                source: &text,
                pack_seq: pack_generation,
                packs: &pack_set,
                registry: &registry,
                store: self.store.as_ref(),
            },
            analysed,
        ))
    }

    #[cfg(test)]
    async fn analysed_diagnostics_for(
        &self,
        uri: &Uri,
        text: Arc<str>,
        dialect: String,
        language_id: &str,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::Diagnostic>> {
        // Master switch off (`tclLsp.features.diagnostics = false`): the pull
        // path returns an empty report, matching the push path's empty-publish.
        if !self.feature_enabled("diagnostics", uri).await {
            return Ok(Vec::new());
        }
        // `tclLsp.diagnostics.exclude`: an excluded file's pull report
        // is empty too, matching the push path's empty publish.
        if self.diagnostics_excluded(uri).await {
            return Ok(Vec::new());
        }
        let (disabled, _non_ascii_mode) = self.resolved_analysis_settings(uri).await;
        let policy_layers = self.resolved_policy_layers(uri).await;
        // Resolve the document's dialect once for the whole report rather than
        // at each provider call below.
        let profile = tcl_lsp_core::profile_for_dialect(&dialect);
        // Match the push worker's exact snapshot contract. A pull can race the
        // debounced push before its cache is primed, so it must carry the same
        // byte evidence itself rather than relying on a prior publish.
        let decode_report = {
            let docs = self.documents.lock("full_diagnostics_for").await;
            docs.get(uri)
                .filter(|doc| doc.text.as_ref() == text.as_ref())
                .and_then(|doc| doc.decode_report)
        };
        // `text` is the client's exact buffer and `analysis_text` its analysis
        // form (lone `\r` → `\n`), exactly as `run_diagnostics_core` splits
        // them on the push path: the parser sees the script `tclsh` would, the
        // W118 line-ending lint sees the real terminators, and the two agree
        // on every offset because the rewrite preserves byte length.
        // In the overwhelmingly common no-bare-CR case the analysis form *is*
        // the client buffer, so reuse its `Arc` rather than copying.
        let analysis_text: Arc<str> = match tcl_lexer::normalise_lone_cr(&text) {
            std::borrow::Cow::Borrowed(_) => text.clone(),
            std::borrow::Cow::Owned(s) => s.into(),
        };
        // F5 dialect dispatch: BIG-IP config / iApp APL
        // presentation documents have model-level validators, not the Tcl
        // analyser — mirror the push path's dispatch so a pull-mode editor
        // receives the same BIGIP/IAPP diagnostics.
        if Self::is_bigip_dialect(&dialect) || is_apl_source(uri, language_id) {
            return Ok(self
                .f5_pull_report(
                    uri,
                    &text,
                    &analysis_text,
                    language_id,
                    &F5PullInputs {
                        profile,
                        policy_layers: &policy_layers,
                        decode_report,
                    },
                )
                .await);
        }
        let analysis = self
            .analysis_for(uri, analysis_text.clone(), dialect.clone())
            .await?;
        let registry = self.registry_for_dialect(&dialect).await;

        let compiler_diags = self
            .compiler_diagnostics_for(uri, &analysis_text, &analysis, &registry)
            .await;

        // XC100-301 translatability lints — independent toggle, f5-irules only.
        let xc_on = self.xc_diagnostics_enabled(uri).await;
        let xc_for_irules = tcl_lsp_core::profile_for_dialect(&dialect).is_irules() && xc_on;
        // Cross-file resolution + the workspace W120 / W123 refinements,
        // matching the push path — and shared verbatim with
        // `textDocument/codeAction`, which lifts its quick-fixes from this
        // exact set (see `published_analyser_diagnostics`).
        let analyser_diags = self
            .published_analyser_diagnostics(
                uri,
                &analysis_text,
                &analysis,
                profile,
                &registry,
                &disabled,
            )
            .await;
        let style_line_length = self.resolved_style_line_length(uri).await;
        Ok(crate::rt::spawn_blocking(move || {
            // The push path's producer set, in the push path's order.
            let produced = published_findings(
                &analyser_diags,
                &compiler_diags,
                xc_for_irules.then_some(analysis.as_ref()),
                &analysis_text,
            );
            let doc = core_report::DocumentSource {
                text: &text,
                analysis_text: &analysis_text,
                decode: decode_report.as_ref(),
                dialect: profile,
                pass: core_report::SourcePass::Tcl {
                    line_length: style_line_length as usize,
                },
            };
            lifted_report(
                &doc,
                produced,
                &policy_layers,
                core_policy::Directives::from_analysis(&analysis, &analysis_text),
                Some(&analysis),
            )
        })
        .await
        .unwrap_or_default())
    }

    /// Apply the workspace refinements the push path's
    /// `refine_and_lift_diagnostics` applies, so a pulled report matches a
    /// pushed one diagnostic for diagnostic:
    ///
    /// * **W120** — a package the workspace's
    ///   `pkgIndex.tcl` / `libraryPaths` prove is transitively provided, or
    ///   that an entry file / `source` ancestor already required.
    /// * **W123 from the package database** — an auto-loaded library command,
    ///   or an available package's defined command.
    /// * **W123 from the workspace index** — an exact source-owned Logical
    ///   command candidate at the current external header coordinate.
    ///
    /// The inherited-requires walk and the workspace-index read are both
    /// gated on there being something to refine, so a clean document pays
    /// neither the lock nor the `source`-graph walk.
    async fn refine_pull_analyser_diagnostics(
        &self,
        uri: &Uri,
        analyser_diags: Vec<tcl_compiler::analyser::Diagnostic>,
        analysis: &AnalysisResult,
        inputs: &PullRefinementInputs<'_>,
    ) -> Vec<tcl_compiler::analyser::Diagnostic> {
        let inheritance = if analyser_diags
            .iter()
            .any(|d| d.code == DiagCode::W120 || d.code == DiagCode::W123)
        {
            self.source_inheritance_for(uri, analysis).await
        } else {
            SourceInheritance::default()
        };
        // Settle this document's unresolved call sites against the
        // workspace index — the same lookup navigation uses — so the pull path
        // suppresses the same W123s and raises the same arity errors the push
        // path does. Both surfaces must agree; see
        // `published_analyser_diagnostics`.
        let has_math_w123 = analyser_diags.iter().any(|d| d.code == DiagCode::W123)
            && analysis
                .command_invocations
                .iter()
                .any(|invocation| invocation.is_mathfunc_call);
        let settled_calls = if analysis.unresolved_command_sites.is_empty() && !has_math_w123 {
            CrossFileCalls::default()
        } else {
            let index = self.workspace_index.read().await;
            let registry = self.registry_for_dialect(inputs.dialect.name).await;
            settle_cross_file_calls(&index, analysis, &registry, uri.as_str())
        };
        let dialects = provider_source::ProviderDialectSources::of(self);
        let provider_sources = provider_source::ProviderCaptureInputs {
            db: &self.db,
            files: &self.db_files,
            global: &self.db_config,
            folders: &self.folder_db_configs,
            store: &self.store,
            source_path_admissions: &self.source_path_admissions,
            dialects: &dialects,
            proposal: None,
        }
        .capture(
            &self.package_resolver,
            analyser_diags
                .iter()
                .any(|diagnostic| matches!(diagnostic.code, DiagCode::W120 | DiagCode::W123)),
        )
        .await;
        let provider_access = provider_source::ProviderSourceAccess::Supplied(&provider_sources);
        let analyser_diags = refine_workspace_w120(
            analyser_diags,
            analysis,
            &inheritance,
            &self.package_resolver,
            &provider_access,
            inputs.registry,
        )
        .await;
        let analyser_diags = refine_workspace_w123(
            analyser_diags,
            analysis,
            &inheritance,
            &self.package_resolver,
            &provider_access,
            inputs.dialect,
        )
        .await;
        let mut out = refine_workspace_index_w123(analyser_diags, analysis, &settled_calls);
        out.extend(cross_file_arity_diagnostics(
            analysis,
            &settled_calls,
            |code| inputs.disabled.contains(code),
        ));
        out
    }

    /// The analyser diagnostics this document actually **publishes**: the
    /// analyser's own set put through every workspace pass the diagnostics
    /// pipeline applies — cross-file resolution when `crossFileResolution` is
    /// on, then the W120 and W123 refinements.
    ///
    /// This is the single answer both diagnostic-consuming surfaces must use.
    /// `textDocument/diagnostic` publishes it; `textDocument/codeAction` lifts
    /// quick-fixes from it.  Having code actions read `analysis.diagnostics`
    /// instead would let the server offer "Replace with 'ni'" over a perfectly
    /// good cross-file `Pi()` call *whose diagnostic it had already decided to
    /// suppress* — a quick-fix that rewrites working code into a syntax error
    /// (tclsh 8.6.16 / 9.0.4: `expr {ni() / acos(-1.0)}` → `missing operand`).
    /// One function, one verdict, so the two surfaces cannot drift apart.
    pub(in super::super) async fn published_analyser_diagnostics(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &Arc<AnalysisResult>,
        dialect: &'static tcl_dialect::DialectProfile,
        registry: &CommandRegistry,
        disabled: &HashSet<String>,
    ) -> Vec<tcl_compiler::analyser::Diagnostic> {
        // Opt-in project callback arity: direct cross-file calls are settled
        // below against the workspace index for either toggle state. This pass
        // remains for callback metadata, which has no direct-call site to
        // settle. It is gathered only when `crossFileResolution` is enabled.
        // This is the same supplied-analysis projection used by the tracked
        // push query. Original source identity and known target barriers stay
        // with the shared database owner rather than a reporting arity map.
        let cross_file_on = self.cross_file_resolution_enabled(uri).await;
        let analyser_diags = self
            .project_callback_diagnostics_if(cross_file_on, source, analysis, disabled)
            .await
            .unwrap_or_else(|| analysis.diagnostics.clone());
        // Dialect overlap decisions belong to the shared diagnostic policy,
        // which retains the typed finding and its suppression reason for both
        // published reports and applicable code actions.
        self.refine_pull_analyser_diagnostics(
            uri,
            analyser_diags,
            analysis,
            &PullRefinementInputs {
                dialect,
                registry,
                disabled,
            },
        )
        .await
    }

    /// Compute and publish diagnostics synchronously (awaited).  Used by the
    /// on-close/disk paths and tests; the interactive open/change paths use the
    /// detached [`Self::schedule_diagnostics`] instead so they never block the
    /// LSP message loop.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(in super::super) async fn publish_analyser_diagnostics(
        &self,
        uri: Uri,
        _text: String,
        dialect: String,
        _revision: u64,
        _version: Option<i32>,
    ) {
        let inputs = self
            .diag_inputs(&uri, tcl_lsp_core::profile_for_dialect(&dialect))
            .await;
        let Some(job) = inputs.capture_job(&uri).await else {
            return;
        };
        run_diagnostics_core(inputs, &uri, job).await;
    }
}
