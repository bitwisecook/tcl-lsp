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

//! `semanticTokens/range`: the viewport race and its convergence continuation.

use super::super::diagnostics::is_apl_source;
use super::super::dialects::{IAPPS_DIALECT, IRULES_DIALECT};
use super::super::{Backend, DocumentState};
use super::refresh::{ConvergenceGuard, SemanticTokensRefreshReason, refresh_if_coalesced};
use super::{
    RangeConvergencePending, SEMANTIC_TOKENS_FAST_PATH_BUDGET, SemanticTokensRefreshCtx,
    rearm_semantic_tokens_test_seam, semantic_tokens_test_seam_enabled,
    take_force_semantic_tokens_coarse,
};
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::definition::LspRange as CoreLspRange;
use tcl_lsp_core::semantic_tokens as core_semantic_tokens;
use tcl_registry::CommandRegistry;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::{MessageType, Range, Uri};

/// The document-side inputs `spawn_range_convergence` needs to recompute the
/// enriched viewport off the LSP event loop and diff it against the coarse tier
/// already served. Bundled so the detach helper stays under the
/// argument-count lint rather than threading six positional clones.
pub(in super::super) struct RangeConvergenceInputs {
    /// The document URI, for the settled-marker log line tests key on.
    pub(in super::super) uri: String,
    /// The coarse token stream already served, to diff the enriched recompute against.
    pub(in super::super) served: Vec<u32>,
    pub(in super::super) registry: Arc<CommandRegistry>,
    /// The document text, shared with the request's own coarse tokenisation
    /// rather than cloned for the continuation: one buffer per request,
    /// not one per tier.
    pub(in super::super) text: Arc<str>,
    pub(in super::super) dialect: String,
    pub(in super::super) range: CoreLspRange,
}

impl Backend {
    /// Compute the packed semantic-token stream for `uri`, prioritising a
    /// prompt response over waiting for the fully enriched result.
    ///
    /// Races the memoised, SSA/SCCP-enriched `semantic_tokens` salsa query
    /// against [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`]. When it lands in time —
    /// the common case, since the diagnostics worker has usually already
    /// primed the shared per-item analysis for this revision — it is returned
    /// directly, identical to a bare `db_semantic_tokens` call. When it does
    /// not (a cold or very large file whose whole-file analysis is still
    /// running, or a concurrent edit that cancelled the read), this returns
    /// the cheap segmenter+registry-only tier
    /// (`core_semantic_tokens::full`, no `CompilationUnit`/analysis) — the
    /// bulk of the highlighting, immediately — while the enriched computation
    /// keeps running in the background: it is salsa-memoised and shared with
    /// the diagnostics worker regardless of who consumes it, so nothing is
    /// wasted, and a `workspace/semanticTokens/refresh` follows once it lands
    /// and actually differs from what was served, so the enrichment (regex
    /// -source retagging, user-class object-method resolution) reaches the
    /// editor without waiting for the next edit.
    ///
    /// When a `Project` is indexed, the background computation is
    /// `semantic_tokens_project_for_inputs`, whose configured cross-file
    /// class/proc-role indices read every project file under its own input.
    /// They read it at the light `file_token_facts_for_config`
    /// tier and behind a per-file backdating firewall, so on a warm
    /// workspace this is an aggregation, not a walk; a cold one still pays a
    /// first pass, which this fast path keeps off the token response — it only
    /// delays how soon the *refresh* follows.
    /// Viewport tokens for a document that is **not Tcl** — an APL presentation
    /// or a BIG-IP config — or `None` when the document *is* Tcl and belongs on
    /// the normal pipeline.
    ///
    /// Each carries *embedded* Tcl — a `ltm rule { … }` body, an APL `[ … ]`
    /// bracket expression — so each takes the registry for the dialect that code
    /// is written in.  Fetched lazily, so an ordinary Tcl document pays for
    /// neither.
    pub(in super::super) async fn non_tcl_range_tokens(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        range: CoreLspRange,
    ) -> Option<core_semantic_tokens::SemanticTokens> {
        if is_apl_source(uri, &doc.language_id) {
            let registry = self.registry_for_dialect(IAPPS_DIALECT).await;
            return Some(core_semantic_tokens::apl_range(&doc.text, range, &registry));
        }
        if Self::is_bigip_dialect(&doc.dialect) {
            let registry = self.registry_for_dialect(IRULES_DIALECT).await;
            return Some(core_semantic_tokens::bigip_conf_range(
                &doc.text, range, &registry,
            ));
        }
        None
    }

    /// Race the memoised CU + analysis reads for `uri` against
    /// [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`] for a viewport (`range`) request.
    ///
    /// On a warm/small document both reads land well inside the budget and the
    /// caller colours the viewport with the enriched tier (`pending` is `None`);
    /// on a cold/large one the budget wins and the caller serves the cheap
    /// segmenter+registry-only tier immediately (`cached_cu`/`cached_analysis`
    /// both `None`) rather than blocking the viewport on a whole-file
    /// analysis. The reads are taken as `JoinHandle`s so the ones the budget
    /// drops are **not** lost: on timeout they ride out to the detached
    /// convergence continuation via `pending`, which keeps awaiting
    /// the enriched unit/analysis and pushes a coalesced
    /// `workspace/semanticTokens/refresh` once the enriched viewport genuinely
    /// differs from the coarse tier served — the range analogue of
    /// `semantic_tokens_full`'s converge-later behaviour.
    pub(in super::super) async fn race_range_enriched_reads(
        &self,
        uri: &Uri,
    ) -> (
        Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>,
        Option<Arc<tcl_compiler::analyser::AnalysisResult>>,
        Option<RangeConvergencePending>,
        bool,
    ) {
        // Normally both handles are `Some` (indexed document) or `None` (unindexed
        // buffer → coarse only, nothing to converge to). They are read under
        // *separate* `db_files` locks, though, so a concurrent
        // `did_close`/`db_remove_source` between the two can leave one `Some` and
        // the other `None`; the `_ => None` arm then degrades to coarse-only
        // (`pending = None`) and the orphaned read detaches but self-cancels on the
        // same `Project`-input bump.
        let handles = match (
            self.db_compilation_unit_handle(uri).await,
            self.db_file_analysis_handle(uri).await,
        ) {
            (Some(cu), Some(analysis)) => Some((cu, analysis)),
            _ => None,
        };
        match handles {
            Some((mut cu_handle, mut analysis_handle)) => {
                // Race the enriched reads against the budget, capturing each result
                // into an outer slot *as it lands* so a partial completion survives
                // the dropped race future. The two reads are polled concurrently
                // (`join!`), so each slot fills independently of the other's landing
                // order. A slot is `Some` iff its handle was awaited to completion —
                // so a `None` slot means the handle is still un-consumed (the
                // continuation awaits it), while a `Some` slot is reused directly,
                // never re-awaiting a spent handle. Without the slots, a read that
                // landed within budget while its sibling overran would be consumed
                // and then lost when the timeout drops the race future, and its spent
                // handle re-awaited in the continuation (a re-poll of a completed
                // `JoinHandle`) — losing the enrichment and the convergence refresh.
                let mut cu_slot: Option<
                    Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>,
                > = None;
                let mut analysis_slot: Option<Option<Arc<tcl_compiler::analyser::AnalysisResult>>> =
                    None;
                let both_ready = if take_force_semantic_tokens_coarse() {
                    false
                } else if semantic_tokens_test_seam_enabled() {
                    // The refresh-driven re-request takes the real enriched
                    // arm deterministically; see the full-token twin above.
                    tokio::join!(
                        async { cu_slot = Some((&mut cu_handle).await.ok().flatten()) },
                        async {
                            analysis_slot = Some((&mut analysis_handle).await.ok().flatten());
                        },
                    );
                    true
                } else {
                    tokio::select! {
                        biased;
                        () = async {
                            tokio::join!(
                                async { cu_slot = Some((&mut cu_handle).await.ok().flatten()) },
                                async {
                                    analysis_slot = Some((&mut analysis_handle).await.ok().flatten());
                                },
                            );
                        } => true,
                        () = crate::rt::sleep(SEMANTIC_TOKENS_FAST_PATH_BUDGET) => false,
                    }
                };
                if both_ready {
                    // Both landed inside the budget — serve the enriched viewport.
                    (cu_slot.flatten(), analysis_slot.flatten(), None, true)
                } else {
                    // Budget won: serve coarse, handing whatever partial result
                    // landed plus the still-running (un-consumed) reads to the
                    // convergence continuation.
                    (
                        None,
                        None,
                        Some((cu_slot, analysis_slot, cu_handle, analysis_handle)),
                        true,
                    )
                }
            }
            None => (None, None, None, false),
        }
    }

    /// Detach the convergence continuation for a range request served
    /// the coarse tier: await the enriched CU / analysis (reusing any that landed
    /// within the budget via its slot, never re-awaiting a spent handle),
    /// recompute the viewport-filtered range, and fire a coalesced
    /// `workspace/semanticTokens/refresh` if it genuinely differs from the coarse
    /// `served` stream. The range stream is never in `last_semantic_tokens`, so
    /// the diff is against the exact `served` bytes rather than the token cache.
    ///
    /// `guard` is the caller's per-URI claim; it is held for the
    /// continuation's whole life so a later request for the same document skips
    /// detaching a redundant second one.
    pub(in super::super) fn spawn_range_convergence(
        &self,
        inputs: RangeConvergenceInputs,
        pending: RangeConvergencePending,
        guard: ConvergenceGuard,
    ) {
        let RangeConvergenceInputs {
            uri,
            served,
            registry,
            text,
            dialect,
            range,
        } = inputs;
        let (cu_slot, analysis_slot, cu_handle, analysis_handle) = pending;
        let mut guard = guard;
        // `uri` here is the log-line string; the claim carries the typed URI the
        // repeat-ask bound is keyed by.
        let claim_uri = guard.uri.clone();
        let refresh_ctx = SemanticTokensRefreshCtx {
            client: self.client.clone(),
            last_semantic_tokens: Arc::clone(&self.last_semantic_tokens),
            refresh_pending: Arc::clone(&self.semantic_tokens_refresh_pending),
            refresh_asked: Arc::clone(&self.semantic_tokens_refresh_asked),
        };
        crate::rt::spawn(async move {
            // Reuse a result that already landed within the budget; otherwise await
            // the still-running read. A `None` slot means the handle was never
            // awaited to completion, so awaiting it here is safe — never a re-poll
            // of a spent handle.
            let cu = match cu_slot {
                Some(cu) => cu,
                None => cu_handle.await.ok().flatten(),
            };
            let analysis = match analysis_slot {
                Some(analysis) => analysis,
                None => analysis_handle.await.ok().flatten(),
            };
            // Both results are required for an enriched viewport. A concurrent
            // setter can cancel either read after its sibling finishes; treating
            // that partial tier as enriched can compare equal to coarse tokens
            // and strand the viewport. Schedule a coalesced client re-pull so
            // cancellation is retryable rather than merely observable.
            let cancelled = cu.is_none() || analysis.is_none();
            let (refreshed, outcome) = if cancelled {
                refresh_ctx.request_refresh_coalesced(SemanticTokensRefreshReason::Convergence);
                (true, RangeSettleOutcome::Cancelled)
            } else {
                let enriched = crate::rt::spawn_blocking(move || {
                    core_semantic_tokens::range_with_cu_and_analysis(
                        &text,
                        tcl_lsp_core::profile_for_dialect(&dialect),
                        range,
                        &registry,
                        cu.as_deref(),
                        analysis.as_deref(),
                    )
                    .data
                })
                .await;
                match enriched {
                    Ok(enriched) if enriched == served => {
                        (false, RangeSettleOutcome::ComparedEqual)
                    }
                    // Same repeat-ask bound as the `full` path
                    // ([`EnrichedRefreshAsked`]): a viewport whose enriched
                    // recompute keeps landing identical must not keep asking
                    // the client to re-pull the tokens it already asked for.
                    Ok(enriched) => {
                        let refreshed = refresh_ctx
                            .request_refresh_for_enriched(&claim_uri, &enriched)
                            .await;
                        let outcome = if refreshed {
                            RangeSettleOutcome::RefreshRequested
                        } else {
                            RangeSettleOutcome::RefreshSuppressed
                        };
                        (refreshed, outcome)
                    }
                    Err(_) => {
                        refresh_ctx
                            .request_refresh_coalesced(SemanticTokensRefreshReason::Convergence);
                        (true, RangeSettleOutcome::Cancelled)
                    }
                }
            };
            // The convergence decision is now made (a refresh was asked for, or the
            // enriched viewport matched the coarse tier we served, or the reads were
            // cancelled). Emit the settled marker so a waiter can key on this exact
            // line + `uri=` to know the compare finished — the message-passing
            // signal that lets a test assert on the *absence* of a
            // `workspace/semanticTokens/refresh` deterministically instead of racing
            // a fixed sleep. `refresh=` records the outcome for observability.
            if matches!(outcome, RangeSettleOutcome::Cancelled) {
                rearm_semantic_tokens_test_seam();
            }
            // Release the per-URI claim only now the decision is made, so every
            // viewport request that arrived while this ran rode along with it —
            // and honour what rode along: a viewport skipped as `coalesced` has
            // no continuation of its own, so if this one's comparison found no
            // difference (or its reads were cancelled) the skipped viewports
            // would stay coarse until an unrelated re-request.  The refresh is
            // workspace-scoped, so one covers every one of them.
            let refreshed = refresh_if_coalesced(&refresh_ctx, &mut guard, refreshed);
            log_range_convergence_settled(&refresh_ctx.client, &uri, refreshed, outcome).await;
        });
    }

    pub(in super::super) fn request_semantic_tokens_retry(
        &self,
        reason: SemanticTokensRefreshReason,
    ) {
        SemanticTokensRefreshCtx {
            client: self.client.clone(),
            last_semantic_tokens: Arc::clone(&self.last_semantic_tokens),
            refresh_pending: Arc::clone(&self.semantic_tokens_refresh_pending),
            refresh_asked: Arc::clone(&self.semantic_tokens_refresh_asked),
        }
        .request_refresh_coalesced(reason);
    }

    pub(in super::super) fn retry_cancelled_range(&self, outcome: RangeSettleOutcome) -> bool {
        if !matches!(outcome, RangeSettleOutcome::Cancelled) {
            return false;
        }
        self.request_semantic_tokens_retry(SemanticTokensRefreshReason::Convergence);
        true
    }

    pub(in super::super) fn inline_range_settlement(
        &self,
        served_enriched: bool,
        had_analysis_handles: bool,
    ) -> (bool, RangeSettleOutcome) {
        let outcome = if served_enriched {
            RangeSettleOutcome::ServedEnriched
        } else if had_analysis_handles {
            // Both handles existed, but a concurrent Salsa write cancelled
            // their reads. This is retryable, not the terminal unindexed case.
            RangeSettleOutcome::Cancelled
        } else {
            RangeSettleOutcome::NoAnalysis
        };
        (self.retry_cancelled_range(outcome), outcome)
    }
}

/// What a `semanticTokens/range` request's convergence decision amounted to,
/// recorded in the settled marker (see [`log_range_convergence_settled`]).
#[derive(Clone, Copy)]
pub(in super::super) enum RangeSettleOutcome {
    /// A coarse response's recomputed enriched viewport matched the bytes served.
    ComparedEqual,
    /// A coarse response's differing enriched viewport scheduled a re-request.
    RefreshRequested,
    /// A coarse response's differing enriched viewport had already asked once.
    RefreshSuppressed,
    /// The coarse tier was served but the enriched reads were cancelled (a
    /// concurrent edit, or another document mutating the salsa `Project`), so
    /// nothing was compared.
    Cancelled,
    /// Both enriched reads landed inside the fast-path budget, so the response
    /// *was* the enriched viewport and there is nothing to converge to.
    ServedEnriched,
    /// The document has no database compilation-unit / analysis handles (an
    /// unindexed buffer, or a `did_close` racing the two reads). Its configured
    /// source projection has no pending database tier to converge to.
    NoAnalysis,
    /// A convergence continuation for this document was already in flight, so
    /// no second one was detached: the pending one's workspace-scoped
    /// refresh covers this viewport too.
    Coalesced,
}

impl RangeSettleOutcome {
    pub(in super::super) fn as_str(self) -> &'static str {
        match self {
            Self::ComparedEqual => "compared-equal",
            Self::RefreshRequested => "refresh-requested",
            Self::RefreshSuppressed => "refresh-suppressed",
            Self::Cancelled => "cancelled",
            Self::ServedEnriched => "served-enriched",
            Self::NoAnalysis => "no-analysis",
            Self::Coalesced => "coalesced",
        }
    }
}

/// Emit the settled marker for a `semanticTokens/range` request.
///
/// Logged for **every** range request, not just the ones that detach a
/// convergence continuation. A waiter that keys on this marker to learn a
/// request's convergence decision would otherwise wait forever on the paths
/// that never detach one — a viewport served the enriched tier directly, or an
/// unindexed buffer with no analysis to converge to — which are exactly the
/// paths a cold or heavily-loaded server takes. `outcome=` says which case it
/// was, so an observer can tell "compared and found equal" from "never
/// compared".
pub(in super::super) async fn log_range_convergence_settled(
    client: &Client,
    uri: &str,
    refreshed: bool,
    outcome: RangeSettleOutcome,
) {
    client
        .log_message(
            MessageType::LOG,
            format!(
                "[timing] semantic_tokens.range_convergence.settled \
                 (uri={uri}, refresh={refreshed}, outcome={})",
                outcome.as_str()
            ),
        )
        .await;
}

/// The unindexed token path uses the existing analysis owner for both source
/// schemas and its unit. Supplied refusal never rebuilds a standalone profile.
pub(in super::super) fn unindexed_semantic_tokens(
    text: &str,
    analysis: &AnalysisResult,
) -> core_semantic_tokens::SemanticTokens {
    unindexed_semantic_tokens_for_range(text, analysis, None)
}

pub(in super::super) fn unindexed_semantic_tokens_for_range(
    text: &str,
    analysis: &AnalysisResult,
    range: Option<CoreLspRange>,
) -> core_semantic_tokens::SemanticTokens {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return core_semantic_tokens::SemanticTokens::default();
    };
    let context = input.context_registry();
    let Some(cu) = tcl_lsp_db::compilation_unit_uncached_from_analysis(
        text,
        context.commands(),
        analysis,
        None,
    ) else {
        return core_semantic_tokens::SemanticTokens::default();
    };
    if let Some(range) = range {
        core_semantic_tokens::range_with_cu_and_analysis(
            text,
            input.analyser_profile(),
            range,
            context.commands(),
            Some(&cu),
            Some(analysis),
        )
    } else {
        core_semantic_tokens::full_with_cu_and_analysis(
            text,
            input.analyser_profile(),
            context.commands(),
            Some(&cu),
            Some(analysis),
        )
    }
}

pub(in super::super) fn core_semantic_range(range: Range) -> CoreLspRange {
    CoreLspRange {
        start_line: range.start.line,
        start_character: range.start.character,
        end_line: range.end.line,
        end_character: range.end.character,
    }
}

pub(in super::super) fn range_token_data(
    had_analysis_handles: bool,
    analysis: Option<&AnalysisResult>,
    text: &str,
    core_range: CoreLspRange,
    dialect: &str,
    registry: &CommandRegistry,
    cu: Option<&tcl_compiler::compilation_unit::CompilationUnit>,
) -> Vec<u32> {
    if !had_analysis_handles {
        return analysis.map_or_else(Vec::new, |analysis| {
            unindexed_semantic_tokens_for_range(text, analysis, Some(core_range)).data
        });
    }
    let profile = tcl_lsp_core::profile_for_dialect(dialect);
    let Some(analysis) = analysis else {
        // The analysed read missed the budget: serve the first-paint tier,
        // which interprets nothing, and let the continuation converge.
        return core_semantic_tokens::coarse_range(text, profile, core_range, registry).data;
    };
    core_semantic_tokens::range_with_cu_and_analysis(
        text,
        profile,
        core_range,
        registry,
        cu,
        Some(analysis),
    )
    .data
}
