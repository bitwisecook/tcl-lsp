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

//! Semantic tokens: tiers, convergence claims and refresh requests.

use super::*;

/// For an *indexed* document (a real `didOpen`-style session,
/// via `db_set_source`), `semantic_tokens_full` must always serve one of
/// the two well-defined tiers `semantic_tokens_core_data` can produce —
/// the enriched result (when the race in `SEMANTIC_TOKENS_FAST_PATH_BUDGET`
/// is won) or the cheap coarse tier (when it is not) — never anything
/// else (garbage, empty, or a third shape). Deliberately does not assert
/// *which* tier wins: `spawn_blocking` scheduling latency under a loaded
/// test runtime can legitimately push even a trivial document past the
/// budget, which is the fast-path/fallback design working as intended,
/// not a bug — `semantic_tokens_retags_constant_regex_source_true_positive`
/// (tcl-lsp-db) is the deterministic proof that the enriched tier itself
/// produces the richer result when computed directly.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn semantic_tokens_full_serves_a_well_defined_tier_when_indexed() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///indexed.tcl").unwrap();
    let src = "set my_re \".*abc\"\nregexp $my_re $s\n";
    backend
        .db_set_source(&uri, src, "tcl9.0".to_owned())
        .await
        .unwrap();
    let admission = backend.document_path_admission(&uri).await.unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(src.to_owned(), "tcl9.0".to_owned()).with_path_admission(admission),
    );

    let served = backend
        .semantic_tokens_full(SemanticTokensParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        })
        .await
        .expect("ok")
        .expect("semanticTokens enabled must yield a token set");
    let SemanticTokensResult::Tokens(served) = served else {
        panic!("expected a full token stream, not a delta");
    };

    let enriched = backend
        .db_semantic_tokens(&uri)
        .await
        .expect("indexed document has a JoinHandle")
        .await
        .expect("worker did not panic")
        .expect("not cancelled");
    let registry = backend.registry_for_dialect("tcl9.0").await;
    let coarse =
        core_semantic_tokens::full(src, tcl_lsp_core::profile_for_dialect("tcl9.0"), &registry);

    assert!(!served.data.is_empty(), "must never serve an empty stream");
    assert!(
        served.data == lift_semantic_token_data(&enriched.data)
            || served.data == lift_semantic_token_data(&coarse.data),
        "served tokens must be exactly the enriched tier or exactly the \
             coarse tier, never a third shape",
    );
}

/// The viewport's enriched tier reads the unit under the workspace's pack
/// overlay, so it colours exactly what the full-document query colours.
/// `mylib::peek pat` is a pack command that only reads `pat`
/// (`arg 0 -role VarRead`): under the overlay the `regexp` reads the
/// version `set pat` wrote, so that literal is coloured as the pattern's
/// source. A unit built without the overlay sees a call to code the module
/// cannot see, which may rewrite any name live past it, so the literal is
/// no proven source and stays a string — a viewport that disagrees with
/// the document it is a window on, resolved against a registry that knows
/// the command it ignored.
///
/// Compared on the tier's own inputs — the unit and analysis handles
/// `race_range_enriched_reads` awaits and the registry
/// `semantic_tokens_range` resolves against — rather than through the
/// handler, whose 40 ms race may legitimately serve the coarse tier.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn range_tokens_read_the_unit_under_the_workspace_pack_overlay() {
    let backend = test_backend();
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from("/workspace/.tcl-lsp/mylib.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        "speclib mylib 1.0 {\n    command mylib::peek {\n        arity 1\n        \
             arg 0 -role VarRead\n    }\n}\n"
            .to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    // What a reload does before the key goes live: the overlaid registry
    // for the document's dialect exists, the set is the backend's, and
    // the salsa config carries its key.
    let _registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &packs);
    *backend.spec_packs.lock().await = PublishedPackSet {
        seq: 1,
        packs: Arc::new(packs),
    };
    backend.sync_db_config().await;

    let uri = Uri::from_str("file:///workspace/viewport.tcl").unwrap();
    let src = "set pat {^a+$}\nmylib::peek pat\nregexp $pat $s\n";
    backend
        .db_set_source(&uri, src, "tcl9.0".to_owned())
        .await
        .unwrap();
    let profile = tcl_lsp_core::profile_for_dialect("tcl9.0");
    let whole = CoreLspRange {
        start_line: 0,
        start_character: 0,
        end_line: 3,
        end_character: 0,
    };

    let unit = backend
        .db_compilation_unit_handle(&uri)
        .await
        .expect("an indexed document has a unit handle")
        .await
        .expect("worker did not panic")
        .expect("not cancelled");
    let analysis = backend
        .db_file_analysis_handle(&uri)
        .await
        .expect("an indexed document has an analysis handle")
        .await
        .expect("worker did not panic")
        .expect("not cancelled");
    let registry = backend.registry_for_dialect("tcl9.0").await;
    let viewport = core_semantic_tokens::range_with_cu_and_analysis(
        src,
        profile,
        whole,
        &registry,
        Some(&unit),
        Some(&analysis),
    );
    let full = backend
        .db_semantic_tokens(&uri)
        .await
        .expect("an indexed document has a token handle")
        .await
        .expect("worker did not panic")
        .expect("not cancelled");
    assert_eq!(
        viewport.data, full.data,
        "a viewport over the whole document colours what the document does",
    );

    // The overlay is what decides it: the same tier over the unit built
    // without the packs leaves the literal a string.
    let file = (*backend.db_files.lock().await)
        .get(&uri)
        .copied()
        .expect("the document is indexed");
    let snapshot = backend.db.snapshot("test").await;
    let bare =
        crate::rt::spawn_blocking(move || tcl_lsp_db::document_compilation_unit(&*snapshot, file))
            .await
            .expect("worker did not panic");
    let unaware = core_semantic_tokens::range_with_cu_and_analysis(
        src,
        profile,
        whole,
        &registry,
        Some(&bare),
        Some(&analysis),
    );
    assert!(
        first_line_has_a_regex_quantifier(&full.data),
        "under the overlay `set pat`'s literal is the pattern's source: {:?}",
        full.data,
    );
    assert!(
        !first_line_has_a_regex_quantifier(&unaware.data),
        "without the overlay the call may rewrite `pat`, or this test proves \
             nothing: {:?}",
        unaware.data,
    );
}

/// Whether the packed token stream colours a regex quantifier on line 0.
fn first_line_has_a_regex_quantifier(data: &[u32]) -> bool {
    let quantifier = core_semantic_tokens::legend_token_types()
        .iter()
        .position(|kind| *kind == "regexpQuantifier")
        .and_then(|index| u32::try_from(index).ok())
        .expect("the legend has a regex quantifier");
    let mut line = 0;
    data.chunks(5).any(|token| {
        line += token[0];
        line == 0 && token[3] == quantifier
    })
}

/// Direct unit coverage of `SemanticTokensRefreshCtx::deliver_if_changed`
/// — the detached continuation's only logic. It must never write the
/// shared cache (only a served `full`/`full/delta` response does that),
/// regardless of whether the landed result matches what is cached.
#[tokio::test]
async fn semantic_tokens_refresh_ctx_never_mutates_the_cache() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///refresh-ctx.tcl").unwrap();
    let ctx = SemanticTokensRefreshCtx {
        client: backend.client.clone(),
        last_semantic_tokens: Arc::clone(&backend.last_semantic_tokens),
        refresh_pending: Arc::clone(&backend.semantic_tokens_refresh_pending),
        refresh_asked: Arc::clone(&backend.semantic_tokens_refresh_asked),
    };

    // No cache entry yet: "changed" (there is nothing to match), but still
    // must not write the cache itself.
    assert_eq!(
        ctx.deliver_if_changed(&uri, &[1, 2, 3]).await,
        EnrichedDelivery::RefreshRequested,
        "a missing served entry must request a refresh",
    );
    assert!(
        backend
            .last_semantic_tokens
            .lock()
            .await
            .get(&uri)
            .is_none(),
        "deliver_if_changed must never write the semantic-tokens cache",
    );

    // Seed a cache entry as if a coarse response had just been served,
    // then land an enriched result that matches it exactly.
    backend
        .last_semantic_tokens
        .lock()
        .await
        .insert(uri.clone(), ("coarse-result-id".to_owned(), vec![1, 2, 3]));
    assert_eq!(
        ctx.deliver_if_changed(&uri, &[1, 2, 3]).await,
        EnrichedDelivery::ComparedEqual,
        "a matching detached stream must be reported distinctly",
    );
    assert_eq!(
        backend.last_semantic_tokens.lock().await.get(&uri).cloned(),
        Some(("coarse-result-id".to_owned(), vec![1, 2, 3])),
        "an unchanged landed result must leave the served cache entry \
             exactly as it was",
    );

    // A landed result that differs from what was served: still must not
    // overwrite the cache — only the next served response does that.
    assert_eq!(
        ctx.deliver_if_changed(&uri, &[9, 9, 9]).await,
        EnrichedDelivery::RefreshRequested,
        "a changed stream must report its refresh request",
    );
    assert_eq!(
        backend.last_semantic_tokens.lock().await.get(&uri).cloned(),
        Some(("coarse-result-id".to_owned(), vec![1, 2, 3])),
        "deliver_if_changed must not overwrite the cache even when the \
             landed result differs from what was served",
    );
}

/// The converge → refresh → re-request cycle must terminate.
///
/// A request that overruns the fast-path budget serves the coarse tier and
/// caches *that*, so the continuation's enriched stream differs and asks
/// for a refresh.  If the client's re-request also overruns the budget —
/// which a loaded machine does even on a warm memo — the coarse tier is
/// served and cached again, and the identical enriched stream asks for the
/// identical refresh.  Left unbounded the server fires a workspace-wide
/// `workspace/semanticTokens/refresh` every debounce window forever,
/// cancelling and re-issuing the editor's in-flight token request each
/// time, so a `provideDocumentSemanticTokens` call never settles.  The
/// second ask for the same enriched bytes must be dropped; a genuinely
/// different enriched stream must still get through.
#[tokio::test]
async fn a_repeated_identical_enriched_stream_asks_for_one_refresh_only() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///storm.tcl").unwrap();
    let other = Uri::from_str("file:///storm-other.tcl").unwrap();
    let ctx = SemanticTokensRefreshCtx {
        client: backend.client.clone(),
        last_semantic_tokens: Arc::clone(&backend.last_semantic_tokens),
        refresh_pending: Arc::clone(&backend.semantic_tokens_refresh_pending),
        refresh_asked: Arc::clone(&backend.semantic_tokens_refresh_asked),
    };
    // Round 1: the coarse tier was served and cached; the enriched stream
    // differs, so the client is asked to re-pull.
    let coarse = ("coarse".to_owned(), vec![0, 0, 3, 1, 0]);
    let enriched = vec![0, 0, 3, 7, 0];
    backend
        .last_semantic_tokens
        .lock()
        .await
        .insert(uri.clone(), coarse.clone());
    assert_eq!(
        ctx.deliver_if_changed(&uri, &enriched).await,
        EnrichedDelivery::RefreshRequested,
        "the first enriched stream that differs must ask for a refresh",
    );
    // Round 2: the re-request overran the budget again, so the same coarse
    // stream is cached and the same enriched stream lands. Asking again
    // could only repeat what the client has already been told.
    backend
        .last_semantic_tokens
        .lock()
        .await
        .insert(uri.clone(), coarse.clone());
    assert_eq!(
        ctx.deliver_if_changed(&uri, &enriched).await,
        EnrichedDelivery::RefreshSuppressed,
        "a repeat ask for the identical enriched stream must be dropped",
    );
    // A genuinely different enriched stream (an edit landed, a cross-file
    // class resolved) re-arms the ask.
    assert_eq!(
        ctx.deliver_if_changed(&uri, &[0, 0, 3, 9, 0]).await,
        EnrichedDelivery::RefreshRequested,
        "a changed enriched stream must ask again",
    );
    // The bound is per URI, not global.
    backend
        .last_semantic_tokens
        .lock()
        .await
        .insert(other.clone(), coarse);
    assert_eq!(
        ctx.deliver_if_changed(&other, &enriched).await,
        EnrichedDelivery::RefreshRequested,
        "another document's identical stream must ask on its own account",
    );
}

/// The full-token settled marker is a test-client contract: only these
/// three outcomes permit a caller to accept the response it already has.
/// In particular, a repeat-suppressed stream still differs from that
/// response even though the server correctly declines to issue another
/// workspace-wide refresh.
#[test]
fn full_settle_outcomes_keep_final_and_retryable_states_distinct() {
    assert_eq!(
        FullSettleOutcome::ServedEnriched.as_str(),
        "served-enriched"
    );
    assert_eq!(FullSettleOutcome::ComparedEqual.as_str(), "compared-equal");
    assert_eq!(FullSettleOutcome::NoAnalysis.as_str(), "no-analysis");
    assert_eq!(
        FullSettleOutcome::RefreshRequested.as_str(),
        "refresh-requested"
    );
    assert_eq!(
        FullSettleOutcome::RefreshSuppressed.as_str(),
        "refresh-suppressed"
    );
    assert_eq!(FullSettleOutcome::Cancelled.as_str(), "cancelled");
    assert_eq!(FullSettleOutcome::Coalesced.as_str(), "coalesced");
}

/// Range convergence must expose the same distinction as the full-token
/// path. In particular, `refresh-suppressed` is not `compared-equal`: the
/// bytes still differ even though the bounded repeat ask correctly did not
/// schedule another workspace refresh.
#[test]
fn range_settle_outcomes_keep_equal_and_suppressed_states_distinct() {
    assert_eq!(
        RangeSettleOutcome::ServedEnriched.as_str(),
        "served-enriched"
    );
    assert_eq!(RangeSettleOutcome::ComparedEqual.as_str(), "compared-equal");
    assert_eq!(
        RangeSettleOutcome::RefreshRequested.as_str(),
        "refresh-requested"
    );
    assert_eq!(
        RangeSettleOutcome::RefreshSuppressed.as_str(),
        "refresh-suppressed"
    );
    assert_eq!(RangeSettleOutcome::Cancelled.as_str(), "cancelled");
    assert_eq!(RangeSettleOutcome::NoAnalysis.as_str(), "no-analysis");
    assert_eq!(RangeSettleOutcome::Coalesced.as_str(), "coalesced");
}

/// A coalesced fire names every reason that rode along, never just the one
/// that happened to schedule it.
///
/// The whole point of the attribution is that an observer can
/// tell its own refresh from another subsystem's. If a convergence ask
/// riding along on a pack-reload-owned fire were reported as `pack-reload`
/// alone, a test counting convergence refreshes would miss a real one — the
/// attribution would be worse than none.
#[test]
fn a_coalesced_refresh_names_every_reason_that_rode_along() {
    assert_eq!(
        describe_refresh_reasons(SemanticTokensRefreshReason::Convergence.bit()),
        "convergence"
    );
    assert_eq!(
        describe_refresh_reasons(SemanticTokensRefreshReason::PackReload.bit()),
        "pack-reload"
    );
    assert_eq!(
        describe_refresh_reasons(
            SemanticTokensRefreshReason::Convergence.bit()
                | SemanticTokensRefreshReason::PackReload.bit()
        ),
        "convergence+pack-reload"
    );
    assert_eq!(describe_refresh_reasons(0), "none");
}

/// A registry-wide reload's refresh is not the convergence path's.
#[tokio::test]
async fn a_pack_reload_refresh_is_not_attributed_to_convergence() {
    let backend = test_backend();
    backend.request_semantic_tokens_retry(SemanticTokensRefreshReason::PackReload);
    assert_eq!(
        backend
            .semantic_tokens_refresh_pending
            .load(std::sync::atomic::Ordering::Acquire),
        SemanticTokensRefreshReason::PackReload.bit(),
        "a spec-pack reload must schedule its refresh under its own reason",
    );
}

/// A cancelled pair of Salsa enrichment reads
/// is not a terminal coarse result. It schedules the same bounded,
/// workspace-wide re-pull used by detached convergence.
#[tokio::test]
async fn cancelled_range_enrichment_schedules_retry_1854() {
    let backend = test_backend();
    assert!(backend.retry_cancelled_range(RangeSettleOutcome::Cancelled));
    assert_eq!(
        backend
            .semantic_tokens_refresh_pending
            .load(std::sync::atomic::Ordering::Acquire),
        SemanticTokensRefreshReason::Convergence.bit(),
        "cancelled range enrichment must schedule a client re-pull, \
             attributed to the convergence path",
    );
}

/// A fire already scheduled must absorb a second request rather than
/// scheduling a duplicate: the `workspace/semanticTokens/refresh`
/// notification carries no data, so a client that receives it re-pulls
/// current tokens for every open document — any other document's
/// enriched result landing during the debounce window rides along with
/// the fire already scheduled. Guards against the "many cold large tabs
/// finish around the same time" thundering-herd case.
#[tokio::test]
async fn semantic_tokens_refresh_ctx_dedupes_concurrent_refreshes() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///refresh-dedup.tcl").unwrap();
    let ctx = SemanticTokensRefreshCtx {
        client: backend.client.clone(),
        last_semantic_tokens: Arc::clone(&backend.last_semantic_tokens),
        refresh_pending: Arc::clone(&backend.semantic_tokens_refresh_pending),
        refresh_asked: Arc::clone(&backend.semantic_tokens_refresh_asked),
    };

    // Simulate a fire already scheduled (e.g. by a different document's
    // enriched result landing moments earlier).
    backend.semantic_tokens_refresh_pending.store(
        SemanticTokensRefreshReason::Convergence.bit(),
        std::sync::atomic::Ordering::Relaxed,
    );

    // A changed result must ride along with the fire already scheduled,
    // not schedule a second one.
    ctx.deliver_if_changed(&uri, &[1, 2, 3]).await;
    assert_ne!(
        backend
            .semantic_tokens_refresh_pending
            .load(std::sync::atomic::Ordering::Relaxed),
        0,
        "a fire already scheduled must be left untouched, not \
             scheduled a second time",
    );

    // Reset to simulate that fire having landed, then let a new changed
    // result schedule its own.
    backend
        .semantic_tokens_refresh_pending
        .store(0, std::sync::atomic::Ordering::Relaxed);
    ctx.deliver_if_changed(&uri, &[4, 5, 6]).await;
    assert_ne!(
        backend
            .semantic_tokens_refresh_pending
            .load(std::sync::atomic::Ordering::Relaxed),
        0,
        "scheduling a fire must record its reason immediately, before its \
             debounce window elapses",
    );

    // Once the debounce window elapses and the fire completes, the flag
    // must clear so a later change can schedule the next one.
    crate::rt::sleep(SEMANTIC_TOKENS_REFRESH_DEBOUNCE * 2).await;
    assert_eq!(
        backend
            .semantic_tokens_refresh_pending
            .load(std::sync::atomic::Ordering::Relaxed),
        0,
        "the reason set must clear once the debounced fire completes, so a \
             later change can schedule the next one",
    );
}

/// A continuation whose document closes while it is still running lands
/// against an evicted cache entry (`did_close` removes it). This pins the
/// current, deliberate behaviour: `deliver_if_changed` treats the missing
/// entry as "changed" and schedules a refresh for a now-closed document.
/// That refresh is a harmless no-op (there is nothing left to re-request
/// for this URI, and the workspace-wide push is dataless), so it is not
/// worth distinguishing from the alternative reading of a missing cache
/// entry — a genuine (if narrow) race where this continuation's result
/// lands *before* `semantic_tokens_full`'s own cache write completes,
/// where treating "missing" as "unchanged" would risk suppressing a
/// refresh a still-open document genuinely needs.
#[tokio::test]
async fn semantic_tokens_refresh_ctx_after_did_close_is_harmless() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///close-during-refresh.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;
    backend
        .last_semantic_tokens
        .lock()
        .await
        .insert(uri.clone(), ("coarse-result-id".to_owned(), vec![1, 2, 3]));

    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;
    assert!(
        backend
            .last_semantic_tokens
            .lock()
            .await
            .get(&uri)
            .is_none(),
        "did_close must have evicted the cache entry",
    );

    let ctx = SemanticTokensRefreshCtx {
        client: backend.client.clone(),
        last_semantic_tokens: Arc::clone(&backend.last_semantic_tokens),
        refresh_pending: Arc::clone(&backend.semantic_tokens_refresh_pending),
        refresh_asked: Arc::clone(&backend.semantic_tokens_refresh_asked),
    };
    ctx.deliver_if_changed(&uri, &[9, 9, 9]).await;
    assert_ne!(
        backend
            .semantic_tokens_refresh_pending
            .load(std::sync::atomic::Ordering::Relaxed),
        0,
        "a result landing for a closed document schedules a refresh -- \
             harmless (dataless, workspace-wide), not incorrect",
    );
    assert!(
        backend
            .last_semantic_tokens
            .lock()
            .await
            .get(&uri)
            .is_none(),
        "deliver_if_changed must not resurrect a cache entry for a \
             closed document",
    );
}

/// Only the first of several concurrent convergence attempts on one
/// URI may detach a continuation, and the claim is released once it
/// finishes, so the next request converges normally. Other URIs are
/// independent.
#[test]
fn convergence_guard_admits_one_continuation_per_uri() {
    let in_flight: ConvergenceInFlight = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let uri = Uri::from_str("file:///cold.tcl").unwrap();
    let other = Uri::from_str("file:///warm.tcl").unwrap();

    let first = ConvergenceGuard::claim(&in_flight, &uri).expect("the first attempt detaches");
    assert!(
        ConvergenceGuard::claim(&in_flight, &uri).is_none(),
        "a second attempt on the same URI must not detach a redundant \
             continuation",
    );
    let sibling =
        ConvergenceGuard::claim(&in_flight, &other).expect("a different URI is unaffected");

    // The continuation finishes: the URI converges again on the next request.
    drop(first);
    let again = ConvergenceGuard::claim(&in_flight, &uri)
        .expect("a completed continuation releases its URI");
    drop(again);
    drop(sibling);
    assert!(
        in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty(),
        "every released claim must leave the in-flight set",
    );
}

/// A claim reports whether anything was
/// coalesced onto it, and `refresh_if_coalesced` turns that into a refresh
/// exactly when the claim holder itself decided against one.
///
/// Without this a skipped viewport is stranded: it detached no
/// continuation of its own on the strength of this claim's refresh, but
/// the claim only refreshes when *its* comparison found a difference.
#[tokio::test]
async fn coalesced_claim_refreshes_even_when_the_holder_found_no_difference() {
    let backend = test_backend();
    let refresh_ctx = SemanticTokensRefreshCtx {
        client: backend.client.clone(),
        last_semantic_tokens: Arc::clone(&backend.last_semantic_tokens),
        refresh_pending: Arc::clone(&backend.semantic_tokens_refresh_pending),
        refresh_asked: Arc::clone(&backend.semantic_tokens_refresh_asked),
    };
    let in_flight: ConvergenceInFlight = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let uri = Uri::from_str("file:///coalesced.tcl").unwrap();

    // Nothing coalesced, no difference found: today's behaviour — no refresh.
    let mut lone = ConvergenceGuard::claim(&in_flight, &uri).expect("claimed");
    assert!(
        !refresh_if_coalesced(&refresh_ctx, &mut lone, false),
        "a solitary claim that found no difference must not refresh",
    );
    assert_eq!(
        backend
            .semantic_tokens_refresh_pending
            .load(std::sync::atomic::Ordering::Acquire),
        0,
        "no refresh may have been scheduled",
    );

    // A viewport coalesced onto the claim, and the holder still found no
    // difference: the coalesced one would otherwise stay coarse forever.
    let mut held = ConvergenceGuard::claim(&in_flight, &uri).expect("re-claimed after release");
    assert!(
        ConvergenceGuard::claim(&in_flight, &uri).is_none(),
        "the second request coalesces onto the held claim",
    );
    assert!(
        refresh_if_coalesced(&refresh_ctx, &mut held, false),
        "a coalesced claim must refresh on behalf of the requests it absorbed",
    );
    assert_eq!(
        backend
            .semantic_tokens_refresh_pending
            .load(std::sync::atomic::Ordering::Acquire),
        SemanticTokensRefreshReason::Convergence.bit(),
        "the coalesced refresh must actually have been scheduled, \
             attributed to the convergence path",
    );
    assert!(
        in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty(),
        "releasing must drop the claim so the next request converges normally",
    );
}

/// The cancelled/error arm of the same rule: the claim holder's reads dying
/// must not swallow the coalesced requests' only chance at a refresh.
#[test]
fn a_released_claims_coalesced_flag_survives_a_cancelled_holder() {
    let in_flight: ConvergenceInFlight = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let uri = Uri::from_str("file:///cancelled.tcl").unwrap();
    let mut held = ConvergenceGuard::claim(&in_flight, &uri).expect("claimed");
    assert!(ConvergenceGuard::claim(&in_flight, &uri).is_none());
    assert!(ConvergenceGuard::claim(&in_flight, &uri).is_none());
    assert!(
        held.release(),
        "two coalesced requests must be reported to the holder",
    );
    // Idempotent: a second release (or the guard's own `Drop`) must not
    // report again, nor remove a claim a later request has since taken.
    let later = ConvergenceGuard::claim(&in_flight, &uri).expect("re-claimed");
    assert!(!held.release());
    drop(held);
    assert!(
        in_flight
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(&uri),
        "the released guard must not evict the later claim",
    );
    drop(later);
}

/// The claim is a guard, not a flag, so a continuation that is
/// aborted mid-flight (or panics) still releases its URI — a stuck marker
/// would silence convergence for that document for the session.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn convergence_guard_releases_on_cancellation() {
    let in_flight: ConvergenceInFlight = Arc::new(std::sync::Mutex::new(HashMap::new()));
    let uri = Uri::from_str("file:///aborted.tcl").unwrap();
    let guard = ConvergenceGuard::claim(&in_flight, &uri).expect("claimed");

    let task = crate::rt::spawn(async move {
        // Never completes; the abort below drops the future — and the guard
        // it owns — instead.
        crate::rt::sleep(std::time::Duration::from_hours(1)).await;
        drop(guard);
    });
    task.abort();
    assert!(task.await.is_err(), "the continuation was cancelled");

    assert!(
        ConvergenceGuard::claim(&in_flight, &uri).is_some(),
        "a cancelled continuation must release its URI",
    );
}
