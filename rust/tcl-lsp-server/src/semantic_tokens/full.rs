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

//! `semanticTokens/full` and its delta: the budget race and the convergence continuation.

use super::super::diagnostics::is_apl_source;
use super::super::dialects::{IAPPS_DIALECT, IRULES_DIALECT};
use super::super::{Backend, DocumentState, readonly_reference_reports};
use super::range::unindexed_semantic_tokens;
use super::refresh::{ConvergenceGuard, refresh_if_coalesced};
use super::{
    SEMANTIC_TOKENS_FAST_PATH_BUDGET, SemanticTokenCoreData, SemanticTokensRefreshCtx,
    lift_semantic_token_data, next_semantic_tokens_id, rearm_semantic_tokens_test_seam,
    semantic_tokens_test_seam_enabled,
};
use std::ops::ControlFlow;
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::semantic_tokens as core_semantic_tokens;
use tower_lsp_server::ls_types::{
    MessageType, SemanticTokens as LspSemanticTokens, SemanticTokensDelta,
    SemanticTokensDeltaParams, SemanticTokensEdit, SemanticTokensFullDeltaResult,
    SemanticTokensResult, Uri,
};
use tower_lsp_server::{Client, jsonrpc};

pub(in super::super) fn configured_coarse_semantic_tokens(
    text: &str,
    analysis: &AnalysisResult,
) -> jsonrpc::Result<Vec<u32>> {
    let input = analysis
        .resolved_input
        .as_ref()
        .ok_or_else(coarse_semantic_source_changed)?;

    if tcl_compiler::source_graph::current_analysis(text, analysis).is_none() {
        return Err(coarse_semantic_source_changed());
    }
    Ok(core_semantic_tokens::full_with_cu_and_analysis(
        text,
        input.analyser_profile(),
        input.context_registry().commands(),
        None,
        Some(analysis),
    )
    .data)
}

pub(in super::super) fn coarse_semantic_source_changed() -> jsonrpc::Error {
    jsonrpc::Error {
        code: jsonrpc::ErrorCode::ContentModified,
        message: "Coarse semantic token source or configured input changed".into(),
        data: None,
    }
}

/// What comparing a detached enriched stream against the last served stream
/// decided.  This stays separate from [`FullSettleOutcome`]: the latter also
/// represents paths that never detached a comparison at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in super::super) enum EnrichedDelivery {
    /// The enriched bytes equal the response already served for this URI.
    ComparedEqual,
    /// The enriched bytes differ and scheduled a client re-request.
    RefreshRequested,
    /// The enriched bytes differ, but that exact stream was already asked for.
    RefreshSuppressed,
}

/// Emit the settled marker for a `semanticTokens/full` request's convergence
/// decision.
///
/// The twin of the `semantic_tokens.range_convergence.settled` line
/// [`Backend::spawn_range_convergence`] logs, and it exists for the same
/// reason: an observer needs a message-passing signal that the
/// coarse-vs-enriched decision for *this* request has been made, rather than a
/// fixed sleep or a wall-clock race against the debounced
/// `workspace/semanticTokens/refresh` that only *sometimes* follows it.
/// `refresh=` records the outcome, so "decided not to refresh" is
/// distinguishable from "the continuation has not run yet".
///
/// **Every** path that serves the coarse tier emits it, including the one where
/// the enriched read was cancelled outright (a concurrent edit, or another
/// document mutating the salsa `Project`) and no continuation is detached at
/// all — otherwise the signal would be missing exactly when a waiter most needs
/// to learn that nothing further is coming.
///
/// `outcome=` says exactly how the response settled. In particular,
/// `served-enriched` is distinct from `compared-equal`: the latter describes a
/// coarse response whose detached enriched comparison matched it. A repeated
/// differing stream is likewise recorded as `refresh-suppressed`, so a test
/// client can re-request without causing the server to repeat its bounded
/// workspace refresh.
pub(super) async fn log_full_convergence_settled(
    client: &Client,
    uri: &Uri,
    refreshed: bool,
    enriched: FullSettleOutcome,
) {
    client
        .log_message(
            MessageType::LOG,
            format!(
                "[timing] semantic_tokens.full_convergence.settled \
                 (uri={}, refresh={refreshed}, outcome={})",
                uri.as_str(),
                enriched.as_str(),
            ),
        )
        .await;
}

/// What became of the enriched stream behind a `semanticTokens/full` response
/// (see [`log_full_convergence_settled`]).
#[derive(Clone, Copy)]
pub(in super::super) enum FullSettleOutcome {
    /// The enriched stream won the fast-path race and was served directly.
    ServedEnriched,
    /// A coarse response's detached enriched stream matched the bytes served.
    ComparedEqual,
    /// A coarse response's differing enriched stream scheduled a re-request.
    RefreshRequested,
    /// A coarse response's differing enriched stream had already asked once.
    RefreshSuppressed,
    /// The enriched read was cancelled by a concurrent salsa write, so nothing
    /// was compared; a re-request re-races it.
    Cancelled,
    /// The document has no salsa input, so the response was computed enriched
    /// from a freshly-built unit + analysis.
    NoAnalysis,
    /// A convergence continuation for this document was already in flight, so
    /// no second one was detached: the pending one's workspace-scoped
    /// refresh covers this request too.
    Coalesced,
}

impl FullSettleOutcome {
    pub(in super::super) fn as_str(self) -> &'static str {
        match self {
            Self::ServedEnriched => "served-enriched",
            Self::ComparedEqual => "compared-equal",
            Self::RefreshRequested => "refresh-requested",
            Self::RefreshSuppressed => "refresh-suppressed",
            Self::Cancelled => "cancelled",
            Self::NoAnalysis => "no-analysis",
            Self::Coalesced => "coalesced",
        }
    }
}

impl Backend {
    pub(in super::super) async fn semantic_tokens_core_data(
        &self,
        uri: &Uri,
        doc: &DocumentState,
    ) -> jsonrpc::Result<SemanticTokenCoreData> {
        // APL (iApp presentation) and BIG-IP config are not Tcl — each has its
        // own declarative grammar and its own token set, so both bypass the Tcl
        // pipeline (segmenter / compilation unit / analyser) entirely.  Without
        // these branches the Tcl tokeniser reads each braced block as one
        // literal word and emits whole *lines* as `string` tokens, which
        // mis-colours the file rather than merely under-colouring it.  Both
        // lexers are cheap and pure — line-oriented — so neither needs salsa
        // memoisation.
        //
        // Both settle immediately with `no-analysis`: neither goes through the
        // Tcl compilation unit, so there is no enriched tier to converge to and
        // an observer waiting on the marker must not be left waiting. *Every*
        // exit from this function emits the marker exactly once — that
        // completeness is the whole value of the signal.
        if is_apl_source(uri, &doc.language_id) {
            // The iApps registry: an APL `[ … ]` bracket expression is iApp Tcl.
            let registry = self.registry_for_dialect(IAPPS_DIALECT).await;
            let data = core_semantic_tokens::apl_full(&doc.text, &registry).data;
            log_full_convergence_settled(&self.client, uri, false, FullSettleOutcome::NoAnalysis)
                .await;
            return Ok(data.into());
        }
        if Self::is_bigip_dialect(&doc.dialect) {
            // The iRules registry, not the BIG-IP one: a `ltm rule { … }` body is
            // iRules code embedded in the config, and is walked as such.
            let registry = self.registry_for_dialect(IRULES_DIALECT).await;
            let data = core_semantic_tokens::bigip_conf_full(&doc.text, &registry).data;
            log_full_convergence_settled(&self.client, uri, false, FullSettleOutcome::NoAnalysis)
                .await;
            return Ok(data.into());
        }

        // Install (and retain) this workspace pack generation before the salsa
        // query resolves its opaque `spec_pack_key`. The query database sits
        // below `tcl-spectcl`, so it can look up an installed overlay but must
        // never manufacture one without the pack contents.
        let _registry_generation = self.registry_for_dialect(&doc.dialect).await;
        let Some(mut enriched) = self.db_semantic_tokens(uri).await else {
            let analysis = self
                .fresh_analysis_for(uri, Arc::clone(&doc.text), doc.dialect.clone())
                .await?;
            let text = Arc::clone(&doc.text);
            // The configured fresh driver retains pack generation, resource
            // availability and grammar. The source unit borrows that same input.
            let data =
                crate::rt::spawn_blocking(move || unindexed_semantic_tokens(&text, &analysis).data)
                    .await
                    .map_err(|err| jsonrpc::Error {
                        code: jsonrpc::ErrorCode::InternalError,
                        message: format!("semantic_tokens worker panicked: {err}").into(),
                        data: None,
                    })?;
            log_full_convergence_settled(&self.client, uri, false, FullSettleOutcome::NoAnalysis)
                .await;
            return Ok(data.into());
        };

        if semantic_tokens_test_seam_enabled() {
            if let Some(tokens) = self.semantic_tokens_test_override(uri, enriched).await {
                return Ok(tokens.into());
            }
        } else {
            tokio::select! {
                biased;
                result = &mut enriched => {
                    if let Ok(Some(tokens)) = result {
                        // The enriched tier won the race and is being served as-is:
                        // settled, nothing to converge to.
                        self.rearm_enriched_refresh(uri).await;
                        log_full_convergence_settled(
                            &self.client,
                            uri,
                            false,
                            FullSettleOutcome::ServedEnriched,
                        )
                        .await;
                        return Ok(tokens.data.into());
                    }
                    // A genuine salsa cancellation (a concurrent edit landed) or a
                    // worker panic — either way, don't retry inline or surface an
                    // error: fall through to the cheap coarse tier below so the
                    // caller still gets a prompt result. The edit that cancelled
                    // this read has already scheduled its own diagnostics run,
                    // which will prime a fresh enriched result for the next
                    // request.
                    //
                    // Settle explicitly: the coarse tier is about to be served and
                    // *no* continuation is detached on this path, so without the
                    // marker an observer waiting for the convergence decision would
                    // wait for something that is never coming.
                    log_full_convergence_settled(
                        &self.client,
                        uri,
                        false,
                        FullSettleOutcome::Cancelled,
                    )
                    .await;
                }
                () = crate::rt::sleep(SEMANTIC_TOKENS_FAST_PATH_BUDGET) => {
                    // Too slow for a synchronous first paint: hand the still-running
                    // enriched read to a detached continuation rather than blocking
                    // this response on it, and serve the coarse tier below.
                    self.spawn_full_convergence(uri, enriched).await;
                }
            }
        }

        let mut snapshot = doc.clone();
        for attempt in 0..2 {
            let source = self
                .capture_coarse_semantic_source(uri, &snapshot)
                .await?
                .ok_or_else(coarse_semantic_source_changed)?;
            let analysis = Arc::clone(
                readonly_reference_reports::CoarseSemanticTokenSource::analysis(&source),
            );
            let text = Arc::clone(&snapshot.text);
            let data = crate::rt::spawn_blocking(move || {
                configured_coarse_semantic_tokens(&text, &analysis)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("semantic_tokens worker panicked: {err}").into(),
                data: None,
            })??;
            if attempt == 0
                && let Some(current) = self.coarse_source_recompute_snapshot(&source).await?
            {
                snapshot = current;
                continue;
            }
            return Ok(SemanticTokenCoreData {
                data,
                coarse_source: Some(source),
            });
        }
        Err(coarse_semantic_source_changed())
    }

    /// Detach the `semanticTokens/full` convergence continuation for a request
    /// whose enriched read overran [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`]: keep
    /// awaiting the read and ask the client to re-request once it lands, so the
    /// enrichment reaches the editor without waiting for the next edit.
    ///
    /// At most one continuation per document, on the same claim set the
    /// `range` path uses: both end in the same workspace-scoped
    /// `workspace/semanticTokens/refresh`, so a continuation already pending for
    /// this URI covers this request too and a second one would only hold a
    /// second live read for the same answer. The skipped request still settles,
    /// as `coalesced` — the marker is emitted on *every* path by contract.
    pub(super) async fn spawn_full_convergence(
        &self,
        uri: &Uri,
        enriched: crate::rt::JoinHandle<Option<tcl_lsp_core::semantic_tokens::SemanticTokens>>,
    ) {
        let Some(guard) = ConvergenceGuard::claim(&self.semantic_tokens_convergence, uri) else {
            log_full_convergence_settled(&self.client, uri, false, FullSettleOutcome::Coalesced)
                .await;
            return;
        };
        let refresh_ctx = SemanticTokensRefreshCtx {
            client: self.client.clone(),
            last_semantic_tokens: Arc::clone(&self.last_semantic_tokens),
            refresh_pending: Arc::clone(&self.semantic_tokens_refresh_pending),
            refresh_asked: Arc::clone(&self.semantic_tokens_refresh_asked),
        };
        let uri = uri.clone();
        let mut guard = guard;
        crate::rt::spawn(async move {
            // Settle either way — including on a cancelled read — so a waiter can
            // key on the marker instead of racing the debounced refresh that only
            // *sometimes* follows.
            let outcome = match enriched.await {
                Ok(Some(tokens)) => {
                    match refresh_ctx.deliver_if_changed(&uri, &tokens.data).await {
                        EnrichedDelivery::ComparedEqual => FullSettleOutcome::ComparedEqual,
                        EnrichedDelivery::RefreshRequested => FullSettleOutcome::RefreshRequested,
                        EnrichedDelivery::RefreshSuppressed => FullSettleOutcome::RefreshSuppressed,
                    }
                }
                _ => FullSettleOutcome::Cancelled,
            };
            if matches!(outcome, FullSettleOutcome::Cancelled) {
                rearm_semantic_tokens_test_seam();
            }
            // Release the per-URI claim only now the decision is made, so every
            // request that arrived while this ran rode along with it — and honour
            // what rode along: a request skipped as `coalesced` has no refresh of
            // its own, so if this one decided against refreshing (its own read
            // matched the cache, or was cancelled) the coalesced ones would be
            // stranded.  The refresh is workspace-scoped, so one covers them all.
            let refreshed = matches!(outcome, FullSettleOutcome::RefreshRequested);
            let refreshed = refresh_if_coalesced(&refresh_ctx, &mut guard, refreshed);
            let outcome = if refreshed && !matches!(outcome, FullSettleOutcome::RefreshRequested) {
                FullSettleOutcome::Coalesced
            } else {
                outcome
            };
            refresh_ctx
                .log_full_convergence_settled(&uri, refreshed, outcome)
                .await;
        });
    }
}

impl Backend {
    /// Settle a dialect hint an edit left pending before reading tokens.
    ///
    /// The coarse tier refuses a document whose dialect is unresolved, and the
    /// edit's own resolution runs only after it has published the text and
    /// scheduled diagnostics. A token request that does not resolve it itself
    /// is refused until that tail runs, so the editor repaints late. The same
    /// generation-checked resolution, and a no-op when nothing is pending.
    async fn resolve_pending_token_dialect(&self, uri: &Uri) {
        let pending_tcl = {
            let docs = self.documents.lock("semantic_tokens_dialect").await;
            docs.get(uri)
                .is_some_and(|doc| doc.language_id == "tcl" && doc.dialect_resolution_pending())
        };
        if pending_tcl
            && let Err(error) = self
                .resolve_dirty_open_document_dialect(uri, "semantic_tokens_dialect")
                .await
        {
            self.client
                .log_message(MessageType::WARNING, error.to_string())
                .await;
        }
    }

    pub(in super::super) async fn semantic_tokens_full_attempt(
        &self,
        uri: &Uri,
    ) -> jsonrpc::Result<ControlFlow<jsonrpc::Result<Option<SemanticTokensResult>>>> {
        self.resolve_pending_token_dialect(uri).await;
        let Some(doc) = self.read_document_for_semantic_tokens(uri).await? else {
            return Ok(ControlFlow::Break(Ok(None)));
        };
        // `S-semantic-tokens-rich`: real classification, served from the
        // memoised `semantic_tokens` query (packed integer stream, 5 ints per
        // token `[deltaLine, deltaCol, length, type, modifiers]`).
        let core = self.semantic_tokens_core_data(uri, &doc).await?;
        if let Some(source) = core.coarse_source {
            let mut wire = Some(LspSemanticTokens {
                result_id: None,
                data: lift_semantic_token_data(&core.data),
            });
            let mut data = Some(core.data);
            return Ok(self
                .coarse_semantic_source_transaction(&source, || {
                    let Ok(mut cache) = self.last_semantic_tokens.try_lock() else {
                        return None;
                    };

                    let data = data.take()?;
                    let mut wire = wire.take()?;
                    let result_id = next_semantic_tokens_id();
                    wire.result_id = Some(result_id.clone());
                    cache.insert(uri.clone(), (result_id, data));
                    Some(Some(SemanticTokensResult::Tokens(wire)))
                })
                .await?
                .into_coarse_token_attempt());
        }
        let core_data = core.data;
        let result_id = next_semantic_tokens_id();
        // Remember this stream so a later `full/delta` with this `resultId` can
        // diff against it.
        self.last_semantic_tokens
            .lock()
            .await
            .insert(uri.clone(), (result_id.clone(), core_data.clone()));
        Ok(ControlFlow::Break(Ok(Some(SemanticTokensResult::Tokens(
            LspSemanticTokens {
                result_id: Some(result_id),
                data: lift_semantic_token_data(&core_data),
            },
        )))))
    }

    pub(in super::super) async fn semantic_tokens_full_delta_attempt(
        &self,
        uri: &Uri,
        params: &SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<ControlFlow<jsonrpc::Result<Option<SemanticTokensFullDeltaResult>>>> {
        self.resolve_pending_token_dialect(uri).await;
        let Some(doc) = self.read_document_for_semantic_tokens(uri).await? else {
            return Ok(ControlFlow::Break(Ok(None)));
        };
        let core = self.semantic_tokens_core_data(uri, &doc).await?;
        if let Some(source) = core.coarse_source {
            // Pure lift/diff work stays outside the source publication lease.
            // The same cache's unique issued id revalidates this baseline; a
            // concurrent cache change simply selects the full-stream response.
            let baseline = self
                .last_semantic_tokens
                .lock()
                .await
                .get(uri)
                .filter(|(id, _)| *id == params.previous_result_id)
                .cloned();
            let mut edits = baseline.as_ref().map(|(_, old)| {
                core_semantic_tokens::diff(old, &core.data)
                    .map(|edit| SemanticTokensEdit {
                        start: edit.start,
                        delete_count: edit.delete_count,
                        data: Some(lift_semantic_token_data(&edit.data)),
                    })
                    .into_iter()
                    .collect::<Vec<_>>()
            });
            let mut full = Some(lift_semantic_token_data(&core.data));
            let mut data = Some(core.data);
            return Ok(self
                .coarse_semantic_source_transaction(&source, || {
                    let Ok(mut cache) = self.last_semantic_tokens.try_lock() else {
                        return None;
                    };

                    let data = data.take()?;
                    let result_id = next_semantic_tokens_id();
                    let baseline_current = baseline.as_ref().is_some_and(|(id, _)| {
                        cache.get(uri).is_some_and(|(current, _)| current == id)
                    });
                    let wire = if baseline_current {
                        SemanticTokensFullDeltaResult::TokensDelta(SemanticTokensDelta {
                            result_id: Some(result_id.clone()),
                            edits: edits.take()?,
                        })
                    } else {
                        SemanticTokensFullDeltaResult::Tokens(LspSemanticTokens {
                            result_id: Some(result_id.clone()),
                            data: full.take()?,
                        })
                    };
                    cache.insert(uri.clone(), (result_id, data));
                    Some(Some(wire))
                })
                .await?
                .into_coarse_token_attempt());
        }
        let new_data = core.data;
        let result_id = next_semantic_tokens_id();
        // Swap the cache to the freshly-served stream and grab the previous one
        // *only if* its `resultId` matches the client's `previousResultId` — a
        // stale / unknown id means we can't safely diff, so fall back to a full
        // stream.  Doing the take-and-replace under one lock keeps concurrent
        // requests for the same URI consistent.
        let baseline = {
            let mut cache = self.last_semantic_tokens.lock().await;
            let prev = cache
                .remove(uri)
                .filter(|(id, _)| *id == params.previous_result_id);
            cache.insert(uri.clone(), (result_id.clone(), new_data.clone()));
            prev
        };
        if let Some((_, old_data)) = baseline {
            // Matching baseline: answer with the minimal token-aligned edit
            // (an empty edit list when nothing changed) rather than the whole
            // stream — the incremental path every `full/delta`-capable editor
            // benefits from.
            let edits = semantic_token_delta_edits(&old_data, &new_data);
            Ok(ControlFlow::Break(Ok(Some(
                SemanticTokensFullDeltaResult::TokensDelta(SemanticTokensDelta {
                    result_id: Some(result_id),
                    edits,
                }),
            ))))
        } else {
            Ok(ControlFlow::Break(Ok(Some(
                SemanticTokensFullDeltaResult::Tokens(LspSemanticTokens {
                    result_id: Some(result_id),
                    data: lift_semantic_token_data(&new_data),
                }),
            ))))
        }
    }
}

fn semantic_token_delta_edits(old_data: &[u32], new_data: &[u32]) -> Vec<SemanticTokensEdit> {
    match core_semantic_tokens::diff(old_data, new_data) {
        Some(edit) => vec![SemanticTokensEdit {
            start: edit.start,
            delete_count: edit.delete_count,
            data: Some(lift_semantic_token_data(&edit.data)),
        }],
        None => Vec::new(),
    }
}
