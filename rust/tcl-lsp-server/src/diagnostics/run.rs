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

//! One diagnostics run: the master switch, exclusions and the progressive analyser path with its fast tier.

use std::ops::ControlFlow;
use std::sync::Arc;

use futures_util::future::FutureExt;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::compiler_checks::DiagCode;
use tcl_lsp_core::{diagnostic_policy as core_policy, diagnostic_report as core_report};
use tower_lsp_server::ls_types::{MessageType, Uri};

use crate::current_diagnostics::{DiagnosticSuppression, DiagnosticUnavailable};
use crate::recovery_source::RecoveryWidenCtx;
use crate::{provider_source, rewrite_wire};

use super::analysis_passes::compute_base_analysis;
use super::deep_pass::run_deep_diagnostics;
use super::delivery::DeliveryCtx;
use super::f5_models::run_diagnostics_f5_dialect;
use super::publisher::DiagnosticPublishOutcome;
use super::report::{analyser_findings, lifted_report};
use super::{AnalyserPathInputs, DiagInputs, DiagJob, LiftInputs, PublishTiming, SalsaAnalysisCtx};

/// Upper bound the diagnostics pipeline waits for the full deep pass — the
/// compiler / optimiser checks, the cross-file resolution, the W120 / W123
/// workspace refinement, and the lift — before publishing the cheap,
/// flicker-safe **fast tier**: the workspace-independent syntax /
/// structural / style diagnostics, computed from the per-file analyser walk
/// alone.  A document whose whole pipeline settles inside this budget — a small
/// or warm file — never reaches the fast tier, so it costs no redundant publish
/// round-trip; only a large / cold document, whose deep pass overruns the
/// budget, gets the early tier while the deep pass keeps running and later
/// replaces it for the same version.  Sized like
/// [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`] (the two paths share the memoised
/// per-item analysis) and comfortably under [`DIAGNOSTICS_DEBOUNCE`].
///
/// The budget is a wall-clock race and so is only the *warm-file* half of the
/// debounce-skip; [`DIAGNOSTICS_FAST_TIER_MIN_LINES`] is the timing-independent
/// half that keeps trivial files a single publish regardless of cold-start.
const DIAGNOSTICS_FAST_TIER_BUDGET: std::time::Duration = std::time::Duration::from_millis(40);

/// One-shot test seam that chooses the progressive diagnostics fast branch.
/// See [`FORCE_SEMANTIC_TOKENS_COARSE_ENV`].
const FORCE_DIAGNOSTICS_FAST_TIER_ENV: &str = "TCL_LSP_TEST_FORCE_DIAGNOSTICS_FAST_TIER";

// Process-wide state for the one-shot diagnostics override.
static FORCE_DIAGNOSTICS_FAST_TIER_ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

static FORCE_DIAGNOSTICS_FAST_TIER_TAKEN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Whether the diagnostics override is still available. It is consumed only
/// after the shared base analysis succeeds, so a normal salsa cancellation and
/// retry cannot spend the one shot without publishing the fast tier.
fn diagnostics_fast_tier_force_pending() -> bool {
    *FORCE_DIAGNOSTICS_FAST_TIER_ENABLED
        .get_or_init(|| std::env::var(FORCE_DIAGNOSTICS_FAST_TIER_ENV).as_deref() == Ok("1"))
        && !FORCE_DIAGNOSTICS_FAST_TIER_TAKEN.load(std::sync::atomic::Ordering::Relaxed)
}

/// Consume the diagnostics override after the base analysis has succeeded.
fn take_force_diagnostics_fast_tier() -> bool {
    diagnostics_fast_tier_force_pending()
        && !FORCE_DIAGNOSTICS_FAST_TIER_TAKEN.swap(true, std::sync::atomic::Ordering::Relaxed)
}

/// Documents below this line count never get a fast tier — they go straight to
/// the single deep publish.  This is the **timing-independent** floor of the
/// debounce-skip: a trivial document's deep-pass wall-clock is dominated by
/// one-time warm-up (registry construction, the first salsa query) rather than
/// per-file work, so even its first analysis can overrun
/// [`DIAGNOSTICS_FAST_TIER_BUDGET`] on a cold server — yet a fast tier there is
/// a pure redundant round-trip, since the deep result is milliseconds behind.
/// Gating on size as well as the elapsed budget keeps small files a single
/// publish regardless of machine speed or cold-start jitter (the property the
/// `diagnostics_delivery_smoke` tests pin), while the budget race still
/// suppresses the fast tier for *large but warm* files whose memoised deep pass
/// lands inside the budget.  Set well above any trivial file yet far below the
/// multi-thousand-line documents the fast tier targets.
const DIAGNOSTICS_FAST_TIER_MIN_LINES: usize = 500;

/// Master switch (`tclLsp.features.diagnostics = false`): publish an empty set
/// so any existing squiggles clear, then settle — the analyser, compiler
/// checks, and F5 validators are all skipped.  Always settled (`true`).
pub(super) async fn run_diagnostics_master_off(delivery: &DeliveryCtx<'_>) -> bool {
    if delivery
        .deliver_suppressed_if_current(DiagnosticSuppression::Disabled)
        .await
    {
        // Observability parity with the deep path's `[timing] deep diagnostics`
        // marker (emitted only after a real publish).  An empty-set publish
        // onto an already-empty collection may fire no client-side
        // diagnostics-changed event, so this marker is the reliable signal
        // that the master-switch-off pass actually ran and cleared this URI —
        // the VS Code harness' `waitForMasterOffDiagnostics` keys on this exact
        // line + `uri=`, and it aids field debugging of "why are my squiggles
        // gone" the same way the deep marker does.
        delivery
            .client
            .log_message(
                MessageType::LOG,
                format!(
                    "[timing] diagnostics master-off 0ms (uri={uri}, diags=0)",
                    uri = delivery.uri.as_str(),
                ),
            )
            .await;
    }
    true
}

/// `tclLsp.diagnostics.exclude` matched this document: publish an
/// empty set so any existing squiggles clear, then settle — the master
/// switch's exact shape, with its own timing marker so "why does this file
/// show no diagnostics" is answerable from the log.  The e2e harness'
/// `await_diagnostics_excluded` keys on this exact line + `uri=`.
pub(super) async fn run_diagnostics_excluded(delivery: &DeliveryCtx<'_>) -> bool {
    if delivery
        .deliver_suppressed_if_current(DiagnosticSuppression::Excluded)
        .await
    {
        delivery
            .client
            .log_message(
                MessageType::LOG,
                format!(
                    "[timing] diagnostics excluded 0ms (uri={uri}, diags=0)",
                    uri = delivery.uri.as_str(),
                ),
            )
            .await;
    }
    true
}

/// Returns whether this version is **settled** (published, intentionally
/// skipped as superseded, or a BIG-IP no-op).  `false` means the run was
/// cancelled mid-flight and the caller should retry the document's latest
/// state.
pub(in super::super) async fn run_diagnostics_core(
    inputs: DiagInputs,
    uri: &Uri,
    job: DiagJob,
) -> bool {
    // Read through `inputs` rather than destructuring it: the two context
    // structs below borrow most of its fields, and a by-value destructure of
    // two dozen names buys nothing but length. The salsa handles it also
    // carries were already consumed by the worker to capture `job`.
    let toggles = &inputs.toggles;
    let DiagJob {
        diagnostic_pass,
        text,
        decode_report,
        dialect,
        language_id,
        currency,
        version,
        file,
        config,
    } = job;
    // `text` is the client's exact buffer; `analysis_text` is what the parser
    // must see — a lone `\r` is a command terminator to `tclsh` (its script
    // channel's `-translation auto` rewrites it before the parser runs), not
    // the horizontal whitespace our lexer would make of it.  The two are the
    // same string for every LF / CRLF document and are always the same length,
    // so a span from one lifts unchanged against the other.  The *style* lints
    // keep the raw buffer: W118 exists precisely to report the real
    // terminators.
    let analysis_text = tcl_lexer::normalise_lone_cr(&text);

    let delivery = inputs.delivery_for_job(diagnostic_pass.as_ref(), &text, uri, currency, version);

    if let Some(pass) = &diagnostic_pass {
        let docs = inputs.documents.lock("begin_diagnostic_pass").await;
        if delivery.is_current(&docs).await {
            inputs
                .diagnostic_publisher
                .begin_diagnostic_pass(uri, pass)
                .await;
        }
    }
    if !toggles.diagnostics_enabled {
        return run_diagnostics_master_off(&delivery).await;
    }
    if toggles.excluded {
        return run_diagnostics_excluded(&delivery).await;
    }

    let lift_inputs = LiftInputs {
        rewrite_source: diagnostic_pass
            .as_ref()
            .map(|pass| rewrite_wire::RewriteWireSource::Document(Arc::clone(pass))),
        text: &text,
        decode_report,
        dialect: tcl_lsp_core::profile_for_dialect(&dialect),
        disabled: &inputs.disabled,
        policy_layers: &inputs.policy_layers,
        style_line_length: inputs.style_line_length,
        xc_diagnostics: toggles.xc.xc_diagnostics,
    };
    if let Some(settled) =
        run_diagnostics_f5_dialect(&delivery, &lift_inputs, &analysis_text, &language_id).await
    {
        return settled;
    }

    let salsa_ctx = SalsaAnalysisCtx {
        db: &inputs.db,
        uri,
        file,
        config,
        text: &analysis_text,
        dialect: tcl_lsp_core::profile_for_dialect(&dialect),
    };
    let settled = run_diagnostics_analyser_path(
        &delivery,
        &salsa_ctx,
        &lift_inputs,
        &AnalyserPathInputs {
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
            cross_file_resolution: toggles.xc.cross_file_resolution,
        },
    )
    .await;
    if !settled {
        delivery
            .diagnostic_pass_unavailable(DiagnosticUnavailable::Cancelled)
            .await;
    }
    settled
}

/// The **progressive** Tcl analyser path: the deep pass — base
/// analysis, compiler checks, and cross-file resolution (all cancellable, off
/// the event loop), the W120/W123 workspace refinement, the diagnostic lifts,
/// and the final currency-guarded publish — runs as one future, raced against
/// [`DIAGNOSTICS_FAST_TIER_BUDGET`].  If it settles inside the budget (a small
/// or warm file) the client sees a single publish.  If it
/// overruns (a large or cold file), the workspace-independent **fast tier**
/// (syntax / structural / style diagnostics, everything but W120/W123) is
/// published first so the user gets the bulk of the diagnostics without waiting
/// on the compiler/optimiser + cross-file + refinement passes; the deep pass
/// then finishes and replaces it for the same version.
///
/// Returns `false` only on a genuine salsa cancellation (the caller retries the
/// document's latest state); the fast tier never changes that verdict — the deep
/// pass is always the authority on whether the version settled.
async fn run_diagnostics_analyser_path(
    delivery: &DeliveryCtx<'_>,
    salsa_ctx: &SalsaAnalysisCtx<'_>,
    lift_inputs: &LiftInputs<'_>,
    inputs: &AnalyserPathInputs<'_>,
) -> bool {
    let started = crate::rt::Instant::now();
    let uri_str = delivery.uri.to_string();
    let line_count = salsa_ctx.text.lines().count();

    // The `Project` handle for the cross-file pass (stable across re-sets); `None`
    // ⇒ no cross-file pass (status-quo per-file diagnostics).
    let project = if inputs.cross_file_resolution {
        *inputs.db_project.lock().await
    } else {
        None
    };
    let timing = PublishTiming {
        started,
        uri_str: &uri_str,
        line_count,
    };

    // Compute the base per-file analysis once and share it (`.shared()`): the deep
    // pass awaits it inside its `join!` (concurrently with the compiler and
    // cross-file passes), and — when the budget elapses — the fast tier
    // awaits the *same* future for its coarse publish. `Shared` lets any polled
    // clone drive the read, so the fast-tier await below still makes progress while
    // the deep future is parked. This is the explicit form of what salsa's
    // block-and-share gave implicitly, but without the second `compute_base_analysis`
    // call parking a blocking-pool thread on the already-in-flight base read.
    let recovery = RecoveryWidenCtx {
        cache: inputs.recovery_names,
        registry: &inputs.registry,
        workspace_index: inputs.workspace_index,
        package_resolver: inputs.package_resolver,
        provider_capture: &inputs.provider_capture,
        prefer: inputs.package_prefer,
    };
    let base = Box::pin(compute_base_analysis(
        delivery.client,
        salsa_ctx,
        lift_inputs.disabled,
        inputs.extra_commands,
        inputs.non_ascii_mode,
        &recovery,
    ))
    .shared();

    // Trivial documents skip the progressive machinery entirely and take the
    // single-publish path, so cold-start warm-up can never turn a one-line file
    // into two publishes (see [`DIAGNOSTICS_FAST_TIER_MIN_LINES`]).
    if line_count < DIAGNOSTICS_FAST_TIER_MIN_LINES {
        return run_deep_diagnostics(
            delivery,
            salsa_ctx,
            lift_inputs,
            inputs,
            project,
            timing,
            (base, None),
        )
        .await;
    }

    // The authoritative deep pass performs the full publish and reports whether
    // the version settled.  Race it against the fast-tier budget.
    let fast_tier_deadline = crate::rt::Instant::now() + DIAGNOSTICS_FAST_TIER_BUDGET;
    let gate = FastTierGate::default();
    let deep = run_deep_diagnostics(
        delivery,
        salsa_ctx,
        lift_inputs,
        inputs,
        project,
        timing,
        (base.clone(), Some(&gate)),
    );
    tokio::pin!(deep);

    let force_fast_tier = diagnostics_fast_tier_force_pending();
    if !force_fast_tier {
        tokio::select! {
            biased;
            // The whole pipeline finished inside the budget: the deep publish is the
            // one and only publish, so the fast tier is never sent (no redundant
            // round-trip on small / warm files).  `biased` prefers this arm so a
            // deep pass that lands right on the deadline still skips the fast
            // tier.
            settled = &mut deep => return settled,
            () = crate::rt::sleep_until(fast_tier_deadline) => {}
        }
    }

    // Budget elapsed with the deep pass still running: publish the flicker-safe
    // fast tier now, from the shared base future the deep pass is already awaiting
    // — one in-flight read, never a second whole-file walk. A cancellation here is
    // harmless: the deep pass observes the same edit and settles it.
    //
    // The deep pass keeps being polled meanwhile. Left parked, a lock acquire
    // inside it keeps its place in that lock's fair queue; once the lock is
    // released to it, nothing polls it to take the guard, so the lock stays
    // assigned to a future that only runs after the fast tier, which itself
    // needs the document map and the salsa store. That stopped the whole
    // server with every lock free and every waiter parked (#1657). The gate
    // keeps the deep publication behind the fast tier instead.
    let Some(held) = gate.hold() else {
        // The deep pass is already publishing; it is the authoritative set.
        return deep.await;
    };
    let fast = async move {
        let _held = held;
        let ControlFlow::Continue(analysis) = base.await else {
            return;
        };
        // Spend the one shot only once there is a real fast tier to publish. A
        // cancelled base analysis returns above with it still available for
        // the scheduler's retry.
        if force_fast_tier && !take_force_diagnostics_fast_tier() {
            return;
        }
        let fast_receipt = publish_fast_tier(delivery, &analysis, lift_inputs).await;
        // The test seam waits until the actual push reaches the reference
        // client before allowing the deep future to replace it. Production
        // drops this receipt immediately and retains the fully concurrent
        // progressive path above.
        if force_fast_tier && let Some(receipt) = fast_receipt {
            let _ = receipt.await;
        }
    };
    tokio::pin!(fast);
    tokio::select! {
        biased;
        settled = &mut deep => return settled,
        () = &mut fast => {}
    }
    deep.await
}

/// Orders the deep publication after the fast tier while both futures are
/// polled by one task. The deep pass passes the gate before it publishes; the
/// fast tier holds it until its own publication is delivered or abandoned.
#[derive(Default)]
pub(in super::super) struct FastTierGate {
    pub(in super::super) state: std::sync::Mutex<FastTierGateState>,
    pub(in super::super) released: tokio::sync::Notify,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum FastTierGateState {
    #[default]
    Open,
    Held,
    Passed,
}

impl FastTierGate {
    pub(in super::super) fn state(&self) -> std::sync::MutexGuard<'_, FastTierGateState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Hold the gate for the fast tier, or `None` once the deep pass has
    /// already passed it.
    pub(in super::super) fn hold(&self) -> Option<FastTierHold<'_>> {
        let mut state = self.state();
        if *state == FastTierGateState::Passed {
            return None;
        }
        *state = FastTierGateState::Held;
        Some(FastTierHold { gate: self })
    }

    /// Wait while the fast tier holds the gate, then pass it. Awaited with no
    /// lock held, so the fast tier can always finish.
    pub(in super::super) async fn pass(&self) {
        loop {
            let released = self.released.notified();
            tokio::pin!(released);
            released.as_mut().enable();
            {
                let mut state = self.state();
                if *state != FastTierGateState::Held {
                    *state = FastTierGateState::Passed;
                    return;
                }
            }
            released.await;
        }
    }
}

/// The fast tier's hold on a [`FastTierGate`]; releases it when dropped.
pub(in super::super) struct FastTierHold<'a> {
    pub(in super::super) gate: &'a FastTierGate,
}

impl Drop for FastTierHold<'_> {
    fn drop(&mut self) {
        *self.gate.state() = FastTierGateState::Open;
        self.gate.released.notify_waiters();
    }
}

/// Whether a diagnostic code belongs to the workspace-independent **fast tier**
/// that [`publish_fast_tier`] delivers ahead of the deep pass.
///
/// The classification lives on [`DiagCode::refined_by_workspace`] (the single
/// source of truth for which codes a workspace / cross-file pass can retract) —
/// this consumer only *asks* it, never re-encodes the set.  A code is fast
/// unless the deep pass might refine it away; the deep pass only ever *removes*
/// those (currently W120/W123) and *adds* new diagnostics (compiler/optimiser,
/// synthesised cross-file arity), so the fast tier is a strict subset of the
/// deep tier — no fast-tier diagnostic is ever contradicted (no false-positive
/// flicker).
pub(in super::super) const fn is_fast_tier(code: DiagCode) -> bool {
    !code.refined_by_workspace()
}

/// Publish the flicker-safe **fast tier**: the workspace-independent
/// analyser diagnostics ([`is_fast_tier`]) plus the pure source-style lints,
/// lifted off the event loop.  Delivered push-only through
/// [`DeliveryCtx::deliver_fast_tier_if_current`] (never priming the pull cache,
/// see that method), currency-guarded so a superseding edit can never let this
/// land after the deep tier for the same version.  A lift-worker panic just
/// means the deep tier is the first thing the client sees, which is no worse
/// than publishing no fast tier at all.
async fn publish_fast_tier(
    delivery: &DeliveryCtx<'_>,
    analysis: &Arc<AnalysisResult>,
    lift_inputs: &LiftInputs<'_>,
) -> Option<tokio::sync::oneshot::Receiver<DiagnosticPublishOutcome>> {
    let fast: Vec<tcl_compiler::analyser::Diagnostic> = analysis
        .diagnostics
        .iter()
        .filter(|d| is_fast_tier(d.code))
        .cloned()
        .collect();
    let analysis_lifts = Arc::clone(analysis);
    let text = lift_inputs.text.to_owned();
    let decode_report = lift_inputs.decode_report;
    let policy_layers = lift_inputs.policy_layers.clone();
    let style_line_length = lift_inputs.style_line_length;
    let dialect = lift_inputs.dialect;
    let lifted = crate::rt::spawn_blocking(move || {
        let analysis_text = tcl_lexer::normalise_lone_cr(&text);
        let doc = core_report::DocumentSource {
            text: &text,
            analysis_text: &analysis_text,
            decode: decode_report.as_ref(),
            dialect,
            pass: core_report::SourcePass::Tcl {
                line_length: style_line_length as usize,
            },
        };
        // No O111: it stays deep-tier (tier membership is `is_fast_tier`'s
        // scheduling, not policy), and the deep publish carries it.
        lifted_report(
            &doc,
            analyser_findings(&fast),
            &policy_layers,
            core_policy::Directives::from_analysis(&analysis_lifts, &text),
            Some(&analysis_lifts),
        )
    })
    .await;
    if let Ok(diagnostics) = lifted {
        delivery.deliver_fast_tier_if_current(diagnostics).await
    } else {
        None
    }
}
