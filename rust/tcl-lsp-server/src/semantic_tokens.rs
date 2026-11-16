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

//! Semantic tokens: the budgeted full and range requests, their convergence continuations and the refresh they ask for.

use super::{Backend, readonly_reference_reports};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp_server::Client;
use tower_lsp_server::ls_types::Uri;

mod full;
mod range;
mod refresh;

use full::log_full_convergence_settled;
#[cfg(test)]
pub(super) use full::{EnrichedDelivery, configured_coarse_semantic_tokens};
pub(super) use full::{FullSettleOutcome, coarse_semantic_source_changed};
pub(super) use range::{
    RangeConvergenceInputs, RangeSettleOutcome, core_semantic_range, log_range_convergence_settled,
    range_token_data,
};
#[cfg(test)]
pub(super) use range::{unindexed_semantic_tokens, unindexed_semantic_tokens_for_range};
pub(super) use refresh::{ConvergenceGuard, ConvergenceInFlight, SemanticTokensRefreshReason};
#[cfg(test)]
pub(super) use refresh::{
    SEMANTIC_TOKENS_REFRESH_DEBOUNCE, describe_refresh_reasons, refresh_if_coalesced,
};

pub(super) struct SemanticTokenCoreData {
    pub(super) data: Vec<u32>,
    coarse_source: Option<readonly_reference_reports::CoarseSemanticTokenSource>,
}

impl From<Vec<u32>> for SemanticTokenCoreData {
    fn from(data: Vec<u32>) -> Self {
        Self {
            data,
            coarse_source: None,
        }
    }
}

/// Upper bound `semantic_tokens_full`/`_delta` wait for the fully enriched
/// (SSA/SCCP-informed: regex-source retagging, user-class object-method
/// resolution) token stream before falling back to the cheap coarse tier
/// (segmenter + registry only — no `CompilationUnit`/analysis) and letting the
/// enriched computation finish in the background: semantic tokens must not
/// block indefinitely on a large/cold file's whole-file analysis.
/// Comparable to [`DIAGNOSTICS_DEBOUNCE`]: in the common case the diagnostics
/// worker has already primed the shared per-item analysis for this revision,
/// so the enriched query is a cache hit well inside this budget and callers
/// never see the fallback.
pub(super) const SEMANTIC_TOKENS_FAST_PATH_BUDGET: std::time::Duration =
    std::time::Duration::from_millis(40);

/// One-shot test seam that chooses the semantic-token coarse branch without a
/// timing race. Integration tests use it to exercise the real continuation on
/// a compact document, then make the refresh-driven re-request await the real
/// enriched arm; separate latency tests still cover the 40 ms race.
/// Unset or any value other than `1` is a no-op in every real session.
pub(super) const FORCE_SEMANTIC_TOKENS_COARSE_ENV: &str =
    "TCL_LSP_TEST_FORCE_SEMANTIC_TOKENS_COARSE";

// Process-wide state for the one-shot semantic-token override. A convergence
// re-request must observe the now-memoised enriched result normally rather
// than being forced through the coarse path forever.
static FORCE_SEMANTIC_TOKENS_COARSE_ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

static FORCE_SEMANTIC_TOKENS_COARSE_TAKEN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn semantic_tokens_test_seam_enabled() -> bool {
    *FORCE_SEMANTIC_TOKENS_COARSE_ENABLED
        .get_or_init(|| std::env::var(FORCE_SEMANTIC_TOKENS_COARSE_ENV).as_deref() == Ok("1"))
}

/// Take the configured semantic-token branch override once per process.
fn take_force_semantic_tokens_coarse() -> bool {
    semantic_tokens_test_seam_enabled()
        && !FORCE_SEMANTIC_TOKENS_COARSE_TAKEN.swap(true, std::sync::atomic::Ordering::Relaxed)
}

/// Let a test retry the forced branch when a normal salsa cancellation meant
/// its first continuation never reached the comparison.
fn rearm_semantic_tokens_test_seam() {
    if semantic_tokens_test_seam_enabled() {
        FORCE_SEMANTIC_TOKENS_COARSE_TAKEN.store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

pub(super) struct SemanticTokensRefreshCtx {
    pub(super) client: Client,
    pub(super) last_semantic_tokens: SemanticTokensCache,
    pub(super) refresh_pending: Arc<std::sync::atomic::AtomicU8>,
    pub(super) refresh_asked: EnrichedRefreshAsked,
}

/// Per-URI hash of the enriched token stream a convergence continuation has
/// already asked the client to re-pull.
///
/// The termination bound on the converge → refresh → re-request cycle.  A
/// request that overruns [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`] serves the coarse
/// tier and caches *that* as the URI's last stream, so the continuation's
/// enriched result differs from the cache and asks for a refresh.  The client
/// re-requests — and if that request also overruns the budget (a loaded
/// machine misses it even on a warm memo, since the budget races
/// `spawn_blocking` dispatch and the `db` lock), the coarse tier is served and
/// cached again, and the identical enriched stream asks for the identical
/// refresh.  Unbounded, that cycle never breaks: the server fires a
/// workspace-wide `workspace/semanticTokens/refresh` every debounce window,
/// which cancels and re-issues the editor's in-flight token request forever.
/// Recording the stream already asked about means a *repeat* ask for the same
/// bytes is dropped: the client has been told, and telling it again cannot
/// change the answer.  Any real change — an edit, a cross-file class landing
/// — produces different enriched bytes and re-arms the ask.
///
/// Entries are dropped with the token cache on close / rename, so this is
/// bounded by the open-document set.
pub(super) type EnrichedRefreshAsked = Arc<Mutex<HashMap<Uri, u64>>>;

/// Per-URI cache of the last semantic-token stream served: `(resultId, packed
/// integer data)`. Shared type alias for [`crate::BackendState::last_semantic_tokens`] and
/// [`SemanticTokensRefreshCtx::last_semantic_tokens`] — the same cache, read
/// and written from both the request-handling and the detached-continuation
/// side.
pub(super) type SemanticTokensCache = Arc<Mutex<HashMap<Uri, (String, Vec<u32>)>>>;

/// What a timed-out `semantic_tokens_range` request hands to its
/// convergence continuation: the partial CU / analysis results plus their reads.
/// Each `Option<Option<..>>` slot is `Some` iff that read landed within the
/// budget — in which case it is reused directly and its `JoinHandle` (still
/// carried, but spent) is never awaited again; a `None` slot means the read is
/// still un-consumed and the continuation awaits its handle. Capturing the slots
/// is what stops a CU that landed before the timeout from being lost (and its
/// spent handle re-polled) when the analysis read is the one that overran.
pub(super) type RangeConvergencePending = (
    Option<Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>>,
    Option<Option<Arc<tcl_compiler::analyser::AnalysisResult>>>,
    crate::rt::JoinHandle<Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>>,
    crate::rt::JoinHandle<Option<Arc<tcl_compiler::analyser::AnalysisResult>>>,
);

impl Backend {
    /// Deterministically select both sides of the semantic-token budget race
    /// for an integration-test child process.
    async fn semantic_tokens_test_override(
        &self,
        uri: &Uri,
        enriched: crate::rt::JoinHandle<Option<tcl_lsp_core::semantic_tokens::SemanticTokens>>,
    ) -> Option<Vec<u32>> {
        if take_force_semantic_tokens_coarse() {
            self.spawn_full_convergence(uri, enriched).await;
            return None;
        }

        // After the first forced-coarse response, make the refresh-driven
        // re-request await the genuine enriched arm without a timing race.
        if let Ok(Some(tokens)) = enriched.await {
            self.rearm_enriched_refresh(uri).await;
            log_full_convergence_settled(
                &self.client,
                uri,
                false,
                FullSettleOutcome::ServedEnriched,
            )
            .await;
            return Some(tokens.data);
        }
        log_full_convergence_settled(&self.client, uri, false, FullSettleOutcome::Cancelled).await;
        None
    }
}

fn next_semantic_tokens_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("st-{id}")
}

/// Convert the core's `Vec<u32>` packed semantic-tokens
/// stream (`[deltaLine, deltaCol, length, type, modifiers]`
/// per token) into a `Vec<SemanticToken>` for the wire layer.
pub(super) fn lift_semantic_token_data(
    data: &[u32],
) -> Vec<tower_lsp_server::ls_types::SemanticToken> {
    let mut tokens: Vec<tower_lsp_server::ls_types::SemanticToken> =
        Vec::with_capacity(data.len() / 5);
    for chunk in data.as_chunks::<5>().0 {
        tokens.push(tower_lsp_server::ls_types::SemanticToken {
            delta_line: chunk[0],
            delta_start: chunk[1],
            length: chunk[2],
            token_type: chunk[3],
            token_modifiers_bitset: chunk[4],
        });
    }
    tokens
}
