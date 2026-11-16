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

//! The cancellable query-database passes: base analysis, project diagnostics and compiler checks.

use std::collections::HashSet;
use std::ops::ControlFlow;
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, NonAsciiMode};
use tcl_registry::CommandRegistry;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::{MessageType, Uri};

use super::SalsaAnalysisCtx;
use crate::configuration::ResourceAnalyserInputs;
use crate::recovery_source::{RecoveryWidenCtx, compute_recovery_analysis};
use crate::tracked_locks::{TrackedMutex, cancellable_catch};
use crate::uri_norm::DocumentPath as _;
use crate::{Backend, diagnostic_read_cancellation};

pub(in super::super) type CheckedCompletenessInput =
    Result<Arc<tcl_compiler::analyser::ResolvedAnalysisInput>, tcl_registry::model::OverlayMiss>;

pub(in super::super) struct PreparedCompletenessInput {
    pub(in super::super) checked_input: CheckedCompletenessInput,
    pub(in super::super) untracked_analyser: Option<Analyser>,
}

/// Resolve the same producer used by the subsequent base or recovery walk.
/// An indexed input comes from its actual salsa key; an untracked analyser is
/// retained, including its checked input, rather than rebuilt after the gate.
pub(in super::super) async fn prepare_base_completeness_input(
    ctx: &SalsaAnalysisCtx<'_>,
    disabled: &HashSet<String>,
    extra: &HashSet<String>,
    mode: NonAsciiMode,
) -> PreparedCompletenessInput {
    let (supplied, packs, resource) = {
        let db = ctx.db.lock().await;
        (
            ctx.file
                .map(|file| tcl_lsp_db::document_analysis_input(&*db, file, ctx.config)),
            ctx.config.spec_pack_key(&*db),
            ResourceAnalyserInputs::from_db_config(ctx.config, &*db),
        )
    };
    if let Some(input) = supplied {
        return PreparedCompletenessInput {
            checked_input: input,
            untracked_analyser: None,
        };
    }
    let mut analyser =
        Backend::configured_analyser(disabled.clone(), mode, extra.clone(), packs, resource)
            .with_file_path(
                ctx.uri
                    .document_path()
                    .map(|path| path.display().to_string()),
            );
    let input = {
        analyser
            .prepare_analysis_input(ctx.dialect.name)
            .map(Arc::new)
    };
    PreparedCompletenessInput {
        checked_input: input,
        untracked_analyser: Some(analyser),
    }
}

pub(in super::super) fn original_document_is_complete(
    text: &str,
    input: &CheckedCompletenessInput,
) -> bool {
    input.as_ref().is_ok_and(|input| {
        tcl_lexer::script_image_is_complete(
            &tcl_lexer::SourceImage::document(text),
            input.lexer_config(),
        ) == Ok(true)
    })
}

/// Report an analysis-worker panic to **both** the server log and the editor.
///
/// A panic in the analysis pipeline costs the document every diagnostic it
/// would have had, and the document is then presented as clean.  Silence there
/// is the worst possible failure mode: a reviewer reads "no findings" as "no
/// problems" — a zero-width-space iRule can lose its `WARNING IRULE3102`
/// URL-evasion finding exactly this way, with nothing shown anywhere the user
/// could see.  `eprintln!` alone reaches only the server's stderr, which no
/// editor surfaces.
///
/// `window/logMessage` at `ERROR` puts it in the language-server output channel
/// every LSP client exposes, which is what [`publish_diagnostics_result`] does
/// for a *lift*-worker panic; the analysis worker says the same thing rather
/// than failing quietly.
pub(in super::super) async fn report_analysis_worker_panic(
    client: &Client,
    uri: &Uri,
    stage: &str,
    err: &crate::rt::JoinError,
) {
    let message = format!(
        "tcl-lsp: the {stage} diagnostics worker panicked analysing {} (is_panic={}). \
         No diagnostics are published for this document — it is NOT known to be clean. \
         Please report this with the file that triggered it.",
        uri.as_str(),
        err.is_panic(),
    );
    eprintln!("{message}");
    client.log_message(MessageType::ERROR, message).await;
}

/// Take the process's evaluator epoch into the query database.
///
/// The memoised lattices are shared by every worker while each worker's hook
/// host is its own (built on its first hook dispatch), so a hook quarantined on whichever
/// worker ran it changes what an analysis there would answer without changing
/// anything a query reads, and a plan [`Backend::reload_spec_packs`]
/// publishes reaches the workers before the pack key it installs reaches the
/// database. `tcl_registry::pack_hooks::evaluator_epoch` moves for both, and
/// this makes it the salsa input `tcl_lsp_db::EvaluatorEpoch`, which every
/// memoised lattice is keyed by. Compare-then-set: a sync that finds the
/// evaluators unchanged writes nothing and invalidates nothing, so the steady
/// state costs one lock.
pub(in super::super) async fn sync_evaluator_epoch(db: &TrackedMutex<tcl_lsp_db::TclDatabase>) {
    let epoch = tcl_registry::pack_hooks::evaluator_epoch();
    let mut db = db.lock().await;
    tcl_lsp_db::set_evaluator_epoch(&mut db, epoch);
}

/// Base analysis: the cancellable salsa `file_analysis_incremental` query, off
/// the LSP event loop — the whole-file per-item walk that dominates the deep
/// pass and feeds *both* the workspace-independent fast tier and the deep tier.
/// `ctx.file` is `None` only if the salsa input is somehow absent; then fall
/// back to a direct (uncached) analyse.  The cross-file callback-arity pass is
/// a separate query ([`compute_project_diags_with_read_cancellations`]) so it can run concurrently with
/// this one and so the fast tier can publish before it.
///
/// `Continue(analysis)` carries the result; `Break(settled)` is the early return
/// for the deep pass — `Break(false)` on a genuine salsa cancellation (retry the
/// latest state), `Break(true)` on a deterministic worker panic (settle rather
/// than livelock the debounce loop).
pub(in super::super) async fn compute_base_analysis(
    client: &Client,
    ctx: &SalsaAnalysisCtx<'_>,
    disabled: &HashSet<String>,
    extra_commands: &HashSet<String>,
    non_ascii_mode: NonAsciiMode,
    recovery: &RecoveryWidenCtx<'_>,
) -> ControlFlow<bool, Arc<AnalysisResult>> {
    compute_base_analysis_with_read_cancellations(
        client,
        ctx,
        disabled,
        extra_commands,
        non_ascii_mode,
        recovery,
        None,
    )
    .await
}

pub(in super::super) async fn compute_base_analysis_with_read_cancellations(
    client: &Client,
    ctx: &SalsaAnalysisCtx<'_>,
    disabled: &HashSet<String>,
    extra_commands: &HashSet<String>,
    non_ascii_mode: NonAsciiMode,
    recovery: &RecoveryWidenCtx<'_>,
    cancellations: Option<&diagnostic_read_cancellation::DiagnosticReadCancellations>,
) -> ControlFlow<bool, Arc<AnalysisResult>> {
    let &SalsaAnalysisCtx {
        db,
        uri,
        file,
        config,
        text,
        dialect,
    } = ctx;

    let prepared =
        prepare_base_completeness_input(ctx, disabled, extra_commands, non_ascii_mode).await;

    // A document with an unclosed delimiter needs a wider "known command"
    // universe than the folder/global-scoped `extra_commands` salsa input
    // carries: the workspace's own procs/classes and the commands *this*
    // document's `package require`s resolve to (the recovery known-command
    // hierarchy — see `tcl_compiler::analyser::utils::recovery_known_commands`,
    // which this widens one layer up). Resolving that is real work (a
    // workspace-index read, package-database lookup + file reads) and is
    // inherently per-document — a document's own `package require`s decide
    // which package commands apply, which the shared, folder/global-scoped
    // salsa `AnalyserConfig` cannot express — so this bypasses the cached path
    // entirely and runs only when the original configured gate does not prove
    // completeness. Unknown parser/input is retained on the recovery path.
    if !original_document_is_complete(text, &prepared.checked_input) {
        return Box::pin(compute_recovery_analysis(
            client,
            ctx,
            disabled,
            extra_commands,
            non_ascii_mode,
            recovery,
            prepared,
        ))
        .await;
    }

    if let Some(file) = file {
        // Clone a fresh, short-lived snapshot for just this read and move it
        // into the worker; it drops when the read finishes, so it never holds
        // exclusive-access-blocking references across the debounce sleep.
        let snapshot = db.snapshot("compute_base_analysis").await;
        let cancellations = cancellations.cloned();
        let worker_cancellations = cancellations.clone();
        match crate::rt::spawn_blocking(move || {
            {
                cancellable_catch(&snapshot, || {
                    tcl_lsp_db::file_analysis_incremental(&*snapshot, file, config)
                })
                .inspect_err(|reason| {
                    diagnostic_read_cancellation::observe(worker_cancellations.as_ref(), reason);
                })
                .ok()
            }
        })
        .await
        {
            Ok(Some(analysis)) => ControlFlow::Continue(analysis),
            // A genuine salsa cancellation (a concurrent `set_text` on the
            // shared db) — don't publish; signal a retry of the document's
            // latest state.
            Ok(None) => ControlFlow::Break(false),
            // The worker PANICKED. A panic in the analysis pipeline is
            // deterministic for this document, so retrying it livelocks the
            // debounce loop (~20 failed analyses/second, diagnostics never
            // published). Treat the run as *settled* so the scheduler stops
            // re-marking the slot dirty; the document keeps its prior
            // diagnostics rather than spinning the CPU.
            Err(e) => {
                diagnostic_read_cancellation::terminal(cancellations.as_ref());
                report_analysis_worker_panic(client, uri, "incremental", &e).await;
                ControlFlow::Break(true)
            }
        }
    } else {
        let a_text = text.to_owned();
        let a_dialect = dialect;
        // The gate and this actual untracked walk share one prepared owner.
        let PreparedCompletenessInput {
            checked_input,
            untracked_analyser,
        } = prepared;
        let (Some(analyser), Ok(input)) = (untracked_analyser, checked_input) else {
            return ControlFlow::Break(true);
        };
        let analysis = crate::rt::spawn_blocking(move || {
            {
                Arc::new(
                    analyser
                        .with_resolved_input(input.as_ref().clone())
                        .analyse(&a_text, a_dialect.name),
                )
            }
        })
        .await
        .unwrap_or_default();
        ControlFlow::Continue(analysis)
    }
}

/// The opt-in project callback-arity diagnostics for the deep tier, when a
/// `Project` is indexed and this document has a salsa input. Direct cross-file
/// command existence and arity remain with the workspace-index settlement
/// below, which runs for either setting of `crossFileResolution`.
/// `Continue(None)` means no opt-in callback pass (`crossFileResolution` off,
/// or no salsa input): the caller falls back to the per-file diagnostics.
///
/// Split out of [`compute_base_analysis`] so it can run concurrently with the
/// base walk and the compiler checks and so the workspace-
/// independent fast tier can publish before the callback check lands.
/// It reuses the memoised per-item analysis rather than re-walking the file, so
/// the split adds no second whole-file analysis.  `Break(settled)` mirrors
/// [`compute_base_analysis`] — `Break(false)` on cancellation (retry),
/// `Break(true)` on a deterministic worker panic (settle).
pub(super) async fn compute_project_diags_with_read_cancellations(
    ctx: &SalsaAnalysisCtx<'_>,
    project: Option<tcl_lsp_db::Project>,
    cancellations: Option<&diagnostic_read_cancellation::DiagnosticReadCancellations>,
) -> ControlFlow<bool, Option<Vec<tcl_compiler::analyser::Diagnostic>>> {
    let &SalsaAnalysisCtx {
        db,
        uri,
        file,
        config,
        ..
    } = ctx;
    let (Some(project), Some(file)) = (project, file) else {
        return ControlFlow::Continue(None);
    };
    let snapshot = db.snapshot("compute_project_diags").await;
    let cancellations = cancellations.cloned();
    let worker_cancellations = cancellations.clone();
    match crate::rt::spawn_blocking(move || {
        cancellable_catch(&snapshot, || {
            (*tcl_lsp_db::project_callback_diagnostics_for_inputs(
                &*snapshot, file, config, project,
            ))
            .clone()
        })
        .inspect_err(|reason| {
            diagnostic_read_cancellation::observe(worker_cancellations.as_ref(), reason);
        })
        .ok()
    })
    .await
    {
        Ok(Some(d)) => ControlFlow::Continue(Some(d)),
        Ok(None) => ControlFlow::Break(false),
        Err(e) => {
            diagnostic_read_cancellation::terminal(cancellations.as_ref());
            eprintln!(
                "tcl-lsp: cross-file diagnostics worker panicked for {} (is_panic={}); \
                 skipping this document's diagnostics to avoid a retry livelock",
                uri.as_str(),
                e.is_panic(),
            );
            ControlFlow::Break(true)
        }
    }
}

/// Say once, on the server's log, that a workspace's packs are not installed
/// where a query needed them: the compiler checks and the optimiser's rewrites
/// wait for the packs, and analysis and highlighting read the plain registry
/// meanwhile. Each distinct miss the database has recorded is reported one
/// time, however many queries hit it.
fn report_overlay_misses() {
    for miss in tcl_lsp_db::take_overlay_misses() {
        eprintln!("{}", overlay_miss_message(&miss));
    }
}

/// The log line for one overlay miss: which overlay, and what waits for it.
pub(in super::super) fn overlay_miss_message(miss: &tcl_registry::model::OverlayMiss) -> String {
    format!(
        "tcl-lsp: {miss}; the compiler checks and optimisations wait for the packs to \
         install, and analysis and highlighting read the plain registry meanwhile"
    )
}

pub(super) fn unavailable_compiler_diagnostics() -> Arc<tcl_lsp_db::CompilerDiagnostics> {
    Arc::new(tcl_lsp_db::CompilerDiagnostics {
        checks: Vec::new(),
        optimisations: Vec::new(),
        taint_replays: tcl_compiler::taint::TaintReplayAnalysis::unavailable(),
    })
}

/// Optimiser / compiler-checks diagnostics, also off the event loop via the
/// cancellable salsa `compiler_check_diagnostics` query: the unit's
/// per-procedure lattices are memoised by `function_lattice` and shared with
/// the analyser tail.  `file` is `None` only if the salsa input is absent; then
/// build directly (uncached).
///
/// `Break(false)` on cancellation (retry), `Break(true)` on a deterministic
/// worker panic (settle) — matching [`compute_base_analysis`].
#[cfg(test)]
pub(in super::super) async fn compute_compiler_diags(
    ctx: &SalsaAnalysisCtx<'_>,
    registry: &Arc<CommandRegistry>,
    generic_variable_patterns: Option<&[String]>,
    analysis: Option<&Arc<AnalysisResult>>,
) -> ControlFlow<bool, Option<Arc<tcl_lsp_db::CompilerDiagnostics>>> {
    compute_compiler_diags_with_read_cancellations(
        ctx,
        registry,
        generic_variable_patterns,
        analysis,
        None,
    )
    .await
}

async fn compute_compiler_diags_with_read_cancellations(
    ctx: &SalsaAnalysisCtx<'_>,
    registry: &Arc<CommandRegistry>,
    generic_variable_patterns: Option<&[String]>,
    analysis: Option<&Arc<AnalysisResult>>,
    cancellations: Option<&diagnostic_read_cancellation::DiagnosticReadCancellations>,
) -> ControlFlow<bool, Option<Arc<tcl_lsp_db::CompilerDiagnostics>>> {
    let &SalsaAnalysisCtx {
        db,
        uri,
        file,
        config,
        text,
        ..
    } = ctx;
    if let Some(file) = file {
        let snapshot = db.snapshot("compute_compiler_diags").await;
        let cancellations = cancellations.cloned();
        let worker_cancellations = cancellations.clone();
        match crate::rt::spawn_blocking(move || {
            let diagnostics = cancellable_catch(&snapshot, || {
                tcl_lsp_db::compiler_check_diagnostics_if_available(&*snapshot, file, config)
            })
            .inspect_err(|reason| {
                diagnostic_read_cancellation::observe(worker_cancellations.as_ref(), reason);
            })
            .ok();
            report_overlay_misses();
            diagnostics
        })
        .await
        {
            Ok(Some(d)) => ControlFlow::Continue(d),
            // Genuine cancellation — retry the latest state.
            Ok(None) => ControlFlow::Break(false),
            // Deterministic worker panic — settle instead of livelocking.
            Err(e) => {
                diagnostic_read_cancellation::terminal(cancellations.as_ref());
                eprintln!(
                    "tcl-lsp: compiler-check worker panicked for {} (is_panic={}); \
                     skipping this document's diagnostics to avoid a retry livelock",
                    uri.as_str(),
                    e.is_panic(),
                );
                ControlFlow::Break(true)
            }
        }
    } else {
        let Some(c_analysis) = analysis.map(Arc::clone) else {
            return ControlFlow::Continue(Some(unavailable_compiler_diagnostics()));
        };
        let c_text = text.to_owned();
        let c_registry = Arc::clone(registry);
        let c_generic = generic_variable_patterns.map(<[String]>::to_vec);
        ControlFlow::Continue(Some(
            crate::rt::spawn_blocking(move || {
                {
                    Arc::new(
                        tcl_lsp_db::compiler_check_diagnostics_uncached_from_analysis(
                            &c_text,
                            &c_registry,
                            &c_analysis,
                            c_generic.as_deref(),
                            // No SourceFile means no stored cross-file evidence;
                            // the configured base still owns this source's input.
                            None,
                        ),
                    )
                }
            })
            .await
            .unwrap_or_else(|_| unavailable_compiler_diagnostics()),
        ))
    }
}

/// Indexed compiler work is independent of the base walk. An unindexed
/// document borrows the configured base result, including terminal refusal,
/// rather than reconstructing an input from the registry's display profile.
#[cfg(test)]
pub(in super::super) async fn compute_compiler_diags_after_base(
    ctx: &SalsaAnalysisCtx<'_>,
    registry: &Arc<CommandRegistry>,
    generic_variable_patterns: Option<&[String]>,
    base: impl std::future::Future<Output = ControlFlow<bool, Arc<AnalysisResult>>>,
) -> ControlFlow<bool, Option<Arc<tcl_lsp_db::CompilerDiagnostics>>> {
    compute_compiler_diags_after_base_with_read_cancellations(
        ctx,
        registry,
        generic_variable_patterns,
        base,
        None,
    )
    .await
}

pub(super) async fn compute_compiler_diags_after_base_with_read_cancellations(
    ctx: &SalsaAnalysisCtx<'_>,
    registry: &Arc<CommandRegistry>,
    generic_variable_patterns: Option<&[String]>,
    base: impl std::future::Future<Output = ControlFlow<bool, Arc<AnalysisResult>>>,
    cancellations: Option<&diagnostic_read_cancellation::DiagnosticReadCancellations>,
) -> ControlFlow<bool, Option<Arc<tcl_lsp_db::CompilerDiagnostics>>> {
    if ctx.file.is_some() {
        return compute_compiler_diags_with_read_cancellations(
            ctx,
            registry,
            generic_variable_patterns,
            None,
            cancellations,
        )
        .await;
    }
    match base.await {
        ControlFlow::Continue(analysis) => {
            compute_compiler_diags_with_read_cancellations(
                ctx,
                registry,
                generic_variable_patterns,
                Some(&analysis),
                cancellations,
            )
            .await
        }
        ControlFlow::Break(settled) => ControlFlow::Break(settled),
    }
}
