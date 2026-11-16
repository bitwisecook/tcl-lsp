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

//! Coalesced `workspace/semanticTokens/refresh` requests and the convergence claims behind them.

use super::super::Backend;
use super::super::protocol::bounded_client_request;
use super::SemanticTokensRefreshCtx;
use super::full::{EnrichedDelivery, FullSettleOutcome, log_full_convergence_settled};
use std::collections::HashMap;
use std::sync::Arc;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::{MessageType, Uri};

/// Debounce window for coalescing `workspace/semanticTokens/refresh` pushes
/// (see [`SemanticTokensRefreshCtx::request_refresh_coalesced`]). Comparable
/// to [`DIAGNOSTICS_DEBOUNCE`]: many cold large documents finishing their
/// enriched computation around the same time (e.g. right after `initialized`
/// restores several tabs) would otherwise each fire their own workspace-wide
/// refresh; this collapses a burst into one fire per window.
pub(in super::super) const SEMANTIC_TOKENS_REFRESH_DEBOUNCE: std::time::Duration =
    std::time::Duration::from_millis(50);

/// Owned handles for the detached semantic-tokens background continuation —
/// the enriched (SSA/SCCP-informed) computation [`Backend::semantic_tokens_core_data`]
/// keeps running after [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`] elapses, so it can
/// still tell the client once it lands.  A small owned-handles struct (the
/// [`DiagInputs`] pattern) rather than borrowing `Backend`, since the
/// continuation runs on a detached `tokio::spawn` task with no lifetime tied to
/// the originating request.
/// Which subsystem asked for a `workspace/semanticTokens/refresh`.
///
/// The refresh carries no payload, so a client — and a test watching one —
/// cannot tell a viewport-convergence refresh from a registry-wide one. The
/// asks are also coalesced, so the answer is a *set*, not a single value.
/// [`SemanticTokensRefreshCtx::request_refresh_coalesced`] accumulates the
/// reasons that rode along and names every one of them on the fired marker, so
/// an observer that owns one reason can tell its own refresh from another's.
#[derive(Clone, Copy)]
pub(in super::super) enum SemanticTokensRefreshReason {
    /// A viewport's enriched stream disagreed with the coarse tier served, or a
    /// cancelled comparison asked for its retry.
    Convergence,
    /// The command registry moved under every open document — a spec-pack
    /// reload. Unrelated to any one viewport, and workspace-wide by nature.
    PackReload,
}

impl SemanticTokensRefreshReason {
    /// This reason's bit in the accumulated pending set.
    pub(in super::super) const fn bit(self) -> u8 {
        match self {
            Self::Convergence => 0b01,
            Self::PackReload => 0b10,
        }
    }

    /// The word the fired marker prints.
    pub(in super::super) const fn label(self) -> &'static str {
        match self {
            Self::Convergence => "convergence",
            Self::PackReload => "pack-reload",
        }
    }
}

/// Render an accumulated reason set for the fired marker.
///
/// Every reason that rode along is named, `+`-joined in declaration order, so
/// the marker never attributes a coalesced fire to whichever ask happened to
/// win the race to schedule it. An empty set cannot reach the marker (a fire is
/// only scheduled by an ask), but is rendered rather than asserted on: a
/// diagnostic line must never be the thing that panics the server.
pub(in super::super) fn describe_refresh_reasons(bits: u8) -> String {
    let named: Vec<&str> = [
        SemanticTokensRefreshReason::Convergence,
        SemanticTokensRefreshReason::PackReload,
    ]
    .into_iter()
    .filter(|reason| bits & reason.bit() != 0)
    .map(SemanticTokensRefreshReason::label)
    .collect();
    if named.is_empty() {
        return "none".to_owned();
    }
    named.join("+")
}

/// Emit the marker for a `workspace/semanticTokens/refresh` that is about to be
/// sent.
///
/// The twin of the convergence settled markers, and it exists for the same
/// reason: an observer needs a message-passing signal for *which* subsystem's
/// refresh reached the client, rather than a wall-clock guess that any refresh
/// arriving in its window must be its own.
async fn log_semantic_tokens_refresh_fired(client: &Client, reasons: u8) {
    client
        .log_message(
            MessageType::LOG,
            format!(
                "[timing] semantic_tokens.refresh.fired (reason={})",
                describe_refresh_reasons(reasons)
            ),
        )
        .await;
}

/// URIs whose detached semantic-token convergence continuation is still in
/// flight — the dedup map behind [`ConvergenceGuard`].
///
/// The value records whether any later request was **coalesced** onto that
/// claim, so the claim holder knows it is answering for more than itself (see
/// [`ConvergenceGuard::release`]).
///
/// A `std::sync::Mutex`: every critical section is a single map insert, lookup,
/// or remove, never held across an await, and the remove has to happen from
/// [`ConvergenceGuard`]'s `Drop`, which cannot await.
pub(in super::super) type ConvergenceInFlight = Arc<std::sync::Mutex<HashMap<Uri, bool>>>;

/// A claim on one URI's convergence continuation: at most one may be detached
/// per document at a time.
///
/// A `semanticTokens/range` or `semanticTokens/full` request that overruns
/// [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`] detaches a continuation that holds the
/// document's text and its coarse token stream and queues a blocking recompute.
/// Without this bound, an editor scrolling a cold file issues a viewport request
/// per frame, so N in-flight range requests on one document mean N live document
/// copies and N queued jobs; coalescing the resulting notification
/// ([`SemanticTokensRefreshCtx::request_refresh_coalesced`]) collapses only the
/// notification, never the work behind it.
///
/// The continuations are redundant with each other by construction: the only
/// output is a *workspace-scoped* `workspace/semanticTokens/refresh`, which asks
/// the client to re-pull tokens for every open document. So a pending
/// continuation already covers every later request for the same URI, and the
/// `range` and `full` paths share one set for that reason.
///
/// A guard rather than a bare flag so an aborted or panicking continuation
/// releases the claim as its frame unwinds — a stuck marker would silence the
/// URI's convergence for the rest of the session.
pub(in super::super) struct ConvergenceGuard {
    pub(in super::super) uri: Uri,
    in_flight: ConvergenceInFlight,
    /// Set by [`Self::release`], so `Drop` does not remove an entry a *later*
    /// claim has since installed for the same URI.
    pub(in super::super) released: bool,
}

impl ConvergenceGuard {
    /// Claim `uri`, or `None` when a continuation for it is already pending — in
    /// which case the caller skips detaching and settles as `coalesced`, and
    /// the pending claim is marked as now answering for that request too.
    pub(in super::super) fn claim(in_flight: &ConvergenceInFlight, uri: &Uri) -> Option<Self> {
        let mut in_flight_map = in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(coalesced) = in_flight_map.get_mut(uri) {
            *coalesced = true;
            return None;
        }
        in_flight_map.insert(uri.clone(), false);
        drop(in_flight_map);
        Some(Self {
            uri: uri.clone(),
            in_flight: Arc::clone(in_flight),
            released: false,
        })
    }

    /// Release the claim, reporting whether any request was coalesced onto it
    /// while it was held.
    ///
    /// A coalesced request skips its own continuation on the strength of this
    /// one's refresh — but this one only refreshes when *its own* comparison
    /// found a difference.  When the claim holder's viewport happened to match
    /// the coarse tier (or its reads were cancelled outright) while a skipped
    /// viewport would have differed, the skipped one stays coarse until
    /// something unrelated re-requests it.  Reading the flag as part of the
    /// same critical section that removes the entry is what makes acting on it
    /// sound: a request arriving after this returns finds no claim and takes
    /// out a fresh one rather than being silently dropped into a claim that is
    /// already gone.
    pub(in super::super) fn release(&mut self) -> bool {
        if self.released {
            return false;
        }
        self.released = true;
        self.in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.uri)
            .unwrap_or(false)
    }
}

impl Drop for ConvergenceGuard {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        // Recovering from a poisoned lock rather than skipping the removal: the
        // map holds no invariant a panic could have broken, and leaving a marker
        // behind would permanently silence this URI's convergence.
        self.in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.uri);
    }
}

/// Digest of an enriched token stream, for the repeat-ask bound (see
/// [`EnrichedRefreshAsked`]).  A hash rather than the stream itself: the value
/// is only ever compared for equality, and a token stream is one `u32` per
/// five per token — far too much to retain per open document just to notice a
/// repeat.
fn enriched_token_digest(data: &[u32]) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Release `guard` and, when the continuation is finishing **without** having
/// asked for a refresh but requests were coalesced onto its claim, ask for one
/// anyway.  Returns what the settled marker should record as `refresh=`.
///
/// Shared by the `range` and `full` continuations because the stranding is the
/// same on both: they claim from one set precisely because either one's
/// workspace-scoped `workspace/semanticTokens/refresh` covers the other's
/// skipped requests, which only holds if the claim holder actually fires one.
pub(in super::super) fn refresh_if_coalesced(
    refresh_ctx: &SemanticTokensRefreshCtx,
    guard: &mut ConvergenceGuard,
    refreshed: bool,
) -> bool {
    let coalesced = guard.release();
    if refreshed || !coalesced {
        return refreshed;
    }
    refresh_ctx.request_refresh_coalesced(SemanticTokensRefreshReason::Convergence);
    true
}

impl SemanticTokensRefreshCtx {
    /// Compare the just-landed enriched token stream against whatever `uri`'s
    /// cache currently holds (the coarse tier served while the enriched
    /// computation was still running, or an earlier enriched stream); if it
    /// differs, ask the client to re-request semantic tokens so the
    /// enrichment (regex-source retagging, user-class object-method
    /// resolution) reaches the editor without waiting for the next edit.
    /// Deliberately does **not** write the cache itself — only a served
    /// `full`/`full/delta` response does that (see [`crate::BackendState::last_semantic_tokens`]);
    /// this only decides whether a refresh is worth asking for. Best-effort:
    /// a client without `workspace/semanticTokens/refresh` support rejects the
    /// request, which is harmless.
    ///
    /// Returns the comparison's distinct outcomes so a caller can tell an
    /// enriched response that was already served from a coarse response whose
    /// detached comparison either matched or had its repeat refresh suppressed.
    pub(in super::super) async fn deliver_if_changed(
        &self,
        uri: &Uri,
        data: &[u32],
    ) -> EnrichedDelivery {
        let changed = {
            let cache = self.last_semantic_tokens.lock().await;
            cache.get(uri).is_none_or(|(_, cached)| cached != data)
        };
        if !changed {
            EnrichedDelivery::ComparedEqual
        } else if self.request_refresh_for_enriched(uri, data).await {
            EnrichedDelivery::RefreshRequested
        } else {
            EnrichedDelivery::RefreshSuppressed
        }
    }

    /// Ask the client to re-pull tokens because `uri`'s enriched stream
    /// differs from what was served — **once** per distinct enriched stream
    /// (see [`EnrichedRefreshAsked`]).  Returns whether an ask was actually
    /// made, so the settled marker records what happened rather than what was
    /// wanted.
    pub(super) async fn request_refresh_for_enriched(&self, uri: &Uri, data: &[u32]) -> bool {
        let digest = enriched_token_digest(data);
        {
            let mut asked = self.refresh_asked.lock().await;
            if asked.get(uri) == Some(&digest) {
                return false;
            }
            asked.insert(uri.clone(), digest);
        }
        self.request_refresh_coalesced(SemanticTokensRefreshReason::Convergence);
        true
    }

    /// Emit the `full` convergence continuation's settled marker.
    pub(super) async fn log_full_convergence_settled(
        &self,
        uri: &Uri,
        refreshed: bool,
        enriched: FullSettleOutcome,
    ) {
        log_full_convergence_settled(&self.client, uri, refreshed, enriched).await;
    }

    /// Coalesce concurrent refresh asks into one debounced
    /// `workspace/semanticTokens/refresh` per [`SEMANTIC_TOKENS_REFRESH_DEBOUNCE`]
    /// window. The request carries no URI — it asks the client to re-pull
    /// tokens for every document it has open — so collapsing N pending
    /// refresh intents into one fire loses nothing; the single re-request
    /// covers every URI. Guards against many cold large documents finishing
    /// their enriched computation near-simultaneously (e.g. right after
    /// `initialized` restores several tabs) each firing their own
    /// workspace-wide refresh.
    ///
    /// The first caller to flip `refresh_pending` false→true owns the fire:
    /// it spawns a task that sleeps out the debounce window, clears the flag,
    /// then sends the refresh. Every other caller during that window sees the
    /// flag already `true` and returns immediately — absorbed into the
    /// upcoming fire. Clearing the flag *before* the RPC (not after) means a
    /// result landing while the RPC itself is in flight schedules a fresh
    /// debounced fire rather than being silently dropped: at worst one extra
    /// fire, never a missed one.
    pub(in super::super) fn request_refresh_coalesced(&self, reason: SemanticTokensRefreshReason) {
        if self
            .refresh_pending
            .fetch_or(reason.bit(), std::sync::atomic::Ordering::AcqRel)
            != 0
        {
            return; // a fire is already scheduled — this result rides along.
        }
        let client = self.client.clone();
        let pending = Arc::clone(&self.refresh_pending);
        crate::rt::spawn(async move {
            crate::rt::sleep(SEMANTIC_TOKENS_REFRESH_DEBOUNCE).await;
            // Taking the set is the same clear-before-send the flag did, and it
            // is also what makes the marker complete: every reason that rode
            // along during the window is named, and a reason arriving after
            // this point schedules its own fire rather than being folded into a
            // marker already written.
            let reasons = pending.swap(0, std::sync::atomic::Ordering::AcqRel);
            log_semantic_tokens_refresh_fired(&client, reasons).await;
            let _ = bounded_client_request(client.semantic_tokens_refresh()).await;
        });
    }
}

impl Backend {
    /// The client now holds `uri`'s enriched stream, so a later coarse
    /// response (a file switch or window reload that misses the budget again)
    /// must be able to ask for the same enriched stream once more. Only an
    /// enriched serve re-arms it, so a client stuck on the coarse tier still
    /// gets at most one ask per distinct stream.
    pub(super) async fn rearm_enriched_refresh(&self, uri: &Uri) {
        self.semantic_tokens_refresh_asked.lock().await.remove(uri);
    }
}
