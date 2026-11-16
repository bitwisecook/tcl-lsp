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

//! Closed-file badges.

use super::*;

/// The closed-file badge cache is bounded.  A closed file keeps its
/// diagnostics, so without a cap browsing a large tree would retain a
/// `Vec<Diagnostic>` per file ever opened for the process's life.  Past
/// [`CLOSED_DIAG_BADGE_CAP`] the least-recently-published
/// closed entry is evicted from both `pull_diag_cache` and `closed_diag_gen`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closed_diag_badge_cache_is_capped() {
    let backend = test_backend();
    let uri_at = |i: usize| Uri::from_str(&format!("file:///browse/f{i}.tcl")).unwrap();
    let total = CLOSED_DIAG_BADGE_CAP + 64;
    for i in 0..total {
        let uri = uri_at(i);
        backend.pull_diag_cache.lock().await.insert(
            uri.clone(),
            PullDiagEntry {
                currency: None,
                result_id: next_pull_diag_result_id(),
                revision: 0,
                diagnostics: Vec::new(),
            },
        );
        backend.closed_diag_gen.lock().await.insert(uri.clone(), 1);
        backend.record_closed_diag_badge(&uri).await;
    }

    let cache = backend.pull_diag_cache.lock().await;
    assert_eq!(
        cache.len(),
        CLOSED_DIAG_BADGE_CAP,
        "the closed-file badge cache must stop at the cap",
    );
    let gens = backend.closed_diag_gen.lock().await;
    assert_eq!(
        gens.len(),
        CLOSED_DIAG_BADGE_CAP,
        "the closed-run generations share the cache entry's lifetime",
    );
    // The oldest badge is gone; the newest is kept (least-recently-published
    // eviction, not arbitrary).
    assert!(!cache.contains_key(&uri_at(0)), "oldest badge evicted");
    assert!(!gens.contains_key(&uri_at(0)), "oldest generation evicted");
    assert!(cache.contains_key(&uri_at(total - 1)), "newest badge kept");
}

/// The inline closed-file republish must not memoise the deep compiler
/// tier.  `did_close` runs `run_diagnostics_core` synchronously for every
/// file the editor ever opened; routing that through the salsa `SourceFile`
/// would pin the file's `compilation_unit` (IR/CFG/SSA) and
/// `compiler_check_diagnostics` memos, which salsa cannot evict — the
/// unbounded term that exhausts memory on a workspace browse.  The badge
/// itself is unaffected: the same diagnostics, computed uncached and thrown
/// away.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
pub(in super::super) async fn closed_file_republish_does_not_memoise_the_deep_compiler_tier() {
    let root = unique_scratch_dir("close-no-deep-memo");
    let on_disk = root.join("warn.tcl");
    let src = "proc foo {} { set y 1 }\n";
    std::fs::write(&on_disk, src).unwrap();
    let executed: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let sink = Arc::clone(&executed);
    let backend = test_backend_over(tcl_lsp_db::TclDatabase::with_event_logger(move |key| {
        sink.lock().unwrap().push(key);
    }));
    let uri = Uri::from_file_path(&on_disk).unwrap();
    register(&backend, &uri, src).await;
    backend
        .db_set_source(&uri, src, "tcl8.6".to_owned())
        .await
        .unwrap();

    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;

    // The lightweight signature/decl queries a closed file still answers for
    // open documents' cross-file resolution are fine; the deep tier is not.
    let deep: Vec<String> = executed
        .lock()
        .unwrap()
        .iter()
        .filter(|key| {
            [
                "compilation_unit",
                "compiler_check_diagnostics",
                "file_analysis_incremental",
            ]
            .iter()
            .any(|query| key.contains(query))
        })
        .cloned()
        .collect();
    assert!(
        deep.is_empty(),
        "a closed file's republish must not build memoised deep-tier queries: {deep:?}",
    );
    // …and it still produces the closed-file badge.
    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache.get(&uri).expect("closed on-disk file keeps a badge");
    assert!(
        entry.diagnostics.iter().any(|d| matches!(
            &d.code,
            Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W211"
        )),
        "the uncached closed tier must still carry the file's W211: {:?}",
        entry.diagnostics,
    );
    drop(cache);
    std::fs::remove_dir_all(&root).ok();
}

/// An **open** document is exempt from the closed-badge cap — its
/// cache entry is owned by the open pipeline and must survive an eviction
/// sweep that happens to reach its URI.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closed_diag_badge_cap_spares_reopened_documents() {
    let backend = test_backend();
    let reopened = Uri::from_str("file:///browse/reopened.tcl").unwrap();
    backend.pull_diag_cache.lock().await.insert(
        reopened.clone(),
        PullDiagEntry {
            currency: None,
            result_id: next_pull_diag_result_id(),
            revision: 0,
            diagnostics: Vec::new(),
        },
    );
    // Queued while closed, then reopened before the cap pushed it out.
    backend.record_closed_diag_badge(&reopened).await;
    backend.documents.lock("test").await.insert(
        reopened.clone(),
        DocumentState::new("set x 1\n".to_owned(), "tcl8.6".to_owned()),
    );
    for i in 0..CLOSED_DIAG_BADGE_CAP {
        let uri = Uri::from_str(&format!("file:///browse/g{i}.tcl")).unwrap();
        backend.record_closed_diag_badge(&uri).await;
    }

    assert!(
        backend.pull_diag_cache.lock().await.contains_key(&reopened),
        "an open document's cache entry must survive the closed-badge sweep",
    );
}

// A workspace file that was opened and then had its editor tab closed
// must keep its Problems / File-Explorer badge (the diagnostics computed from
// its on-disk contents), rather than an unconditional empty publish that
// would drop the badge until the file was reopened.  Observed through the
// pull-cache the push path keeps in lock-step (the test client's socket is
// detached, so the notification itself is a no-op).

/// TP: a closed on-disk file that has a problem keeps a non-empty badge.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_close_retains_diagnostics_for_on_disk_file_with_problems() {
    let root = unique_scratch_dir("close-retain");
    let on_disk = root.join("warn.tcl");
    // `y` is assigned but never read → W211, a stable Tcl-dialect diagnostic.
    std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
    let backend = test_backend();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    register(&backend, &uri, "proc foo {} { set y 1 }\n").await;
    backend
        .db_set_source(&uri, "proc foo {} { set y 1 }\n", "tcl8.6".to_owned())
        .await
        .unwrap();

    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;

    assert!(
        !backend.documents.lock("test").await.contains_key(&uri),
        "did_close still removes the open buffer",
    );
    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache
        .get(&uri)
        .expect("a closed on-disk file with problems must keep a pull-cache entry");
    assert!(
        entry.diagnostics.iter().any(|d| matches!(
            &d.code,
            Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W211"
        )),
        "the retained badge must carry the file's W211: {:?}",
        entry.diagnostics,
    );
    drop(cache);
    std::fs::remove_dir_all(&root).ok();
}

/// TP + disk-accuracy: the retained badge reflects the *on-disk* file, not the
/// (possibly discarded) buffer that was open — a buffer edited clean then
/// closed-without-saving still shows the on-disk problem.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_close_badge_reflects_on_disk_not_discarded_buffer() {
    let root = unique_scratch_dir("close-disk-accurate");
    let on_disk = root.join("warn.tcl");
    std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
    let backend = test_backend();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    // The open buffer is clean; only disk has the W211.  The retained badge
    // must come from disk.
    register(&backend, &uri, "proc foo {} { return 1 }\n").await;
    backend
        .db_set_source(&uri, "proc foo {} { return 1 }\n", "tcl8.6".to_owned())
        .await
        .unwrap();

    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;

    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache.get(&uri).expect("closed on-disk file keeps an entry");
    assert!(
        entry.diagnostics.iter().any(|d| matches!(
            &d.code,
            Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W211"
        )),
        "badge must reflect the on-disk W211, not the clean discarded buffer: {:?}",
        entry.diagnostics,
    );
    drop(cache);
    std::fs::remove_dir_all(&root).ok();
}

/// TN: a closed on-disk file with no problems gets an empty badge (no false
/// File-Explorer decoration).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_close_publishes_empty_badge_for_clean_on_disk_file() {
    let root = unique_scratch_dir("close-clean");
    let on_disk = root.join("clean.tcl");
    // A defined-and-called proc with no unused variables — no diagnostics.
    let clean = "proc greet {} { puts hello }\ngreet\n";
    std::fs::write(&on_disk, clean).unwrap();
    let backend = test_backend();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    register(&backend, &uri, clean).await;
    backend
        .db_set_source(&uri, clean, "tcl8.6".to_owned())
        .await
        .unwrap();

    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;

    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache
        .get(&uri)
        .expect("a clean closed on-disk file still primes the cache (empty)");
    assert!(
        entry.diagnostics.is_empty(),
        "a clean file must not gain a badge: {:?}",
        entry.diagnostics,
    );
    drop(cache);
    std::fs::remove_dir_all(&root).ok();
}

/// FP-guard: a file deleted before its tab closes loses its badge (no stale
/// decoration for a file that is gone).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_close_clears_badge_when_file_deleted_before_close() {
    let root = unique_scratch_dir("close-deleted");
    let on_disk = root.join("gone.tcl");
    std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
    let backend = test_backend();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    register(&backend, &uri, "proc foo {} { set y 1 }\n").await;
    backend
        .db_set_source(&uri, "proc foo {} { set y 1 }\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.pull_diag_cache.lock().await.insert(
        uri.clone(),
        PullDiagEntry {
            currency: None,
            result_id: next_pull_diag_result_id(),
            revision: 0,
            diagnostics: Vec::new(),
        },
    );
    // The file vanishes from disk before the tab closes.
    std::fs::remove_file(&on_disk).unwrap();

    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;

    assert!(
        !backend.pull_diag_cache.lock().await.contains_key(&uri),
        "a deleted file must not keep a stale badge",
    );
    std::fs::remove_dir_all(&root).ok();
}

/// FN-guard: the closed-file publish never shadows a document that is open
/// again — a `did_open` racing in front of the closed run keeps authority, so
/// a closed publish is a no-op for an open buffer and cannot blank or
/// overwrite it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_closed_is_noop_for_open_document() {
    let root = unique_scratch_dir("close-reopen");
    let on_disk = root.join("buf.tcl");
    std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
    let backend = test_backend();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    register(&backend, &uri, "proc foo {} { set y 1 }\n").await;
    backend
        .db_set_source(&uri, "proc foo {} { set y 1 }\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    // A sentinel cache entry standing in for the open buffer's own published
    // set. The closed publish must leave it untouched (the doc is open).
    let sentinel = next_pull_diag_result_id();
    backend.pull_diag_cache.lock().await.insert(
        uri.clone(),
        PullDiagEntry {
            currency: None,
            result_id: sentinel.clone(),
            revision: 7,
            diagnostics: Vec::new(),
        },
    );

    backend.publish_closed_file_diagnostics(&uri).await;

    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache
        .get(&uri)
        .expect("open doc's cache entry must survive");
    assert_eq!(
        entry.result_id, sentinel,
        "a closed publish must not overwrite an open document's diagnostics",
    );
    drop(cache);
    std::fs::remove_dir_all(&root).ok();
}

/// A closed run whose generation has been superseded by a newer
/// close / watched-change refresh must not publish — so an older run
/// finishing late cannot overwrite the current set with stale diagnostics.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn superseded_closed_run_does_not_publish() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///gen.tcl").unwrap();
    // The URI is closed (absent from `documents`); the latest generation is 5.
    backend.closed_diag_gen.lock().await.insert(uri.clone(), 5);

    let diag = vec![tower_lsp_server::ls_types::Diagnostic::default()];
    let ctx = |generation: u64| DeliveryCtx {
        sidecar_revision: backend.fresh_analysis_cache.lock().unwrap().revision(),
        inputs_epoch: Some(backend.diag_inputs_epoch()),
        configuration_epoch: backend.diag_inputs_epoch(),
        source_inputs_epoch: Arc::clone(&backend.diag_inputs_epoch),
        source_inputs_gate: Arc::clone(&backend.analyser_inputs_gate),
        diagnostic_pass: None,
        client: &backend.client,
        diagnostic_publisher: &backend.diagnostic_publisher,
        documents: &backend.documents,
        store: &backend.store,
        diag_slots: &backend.diag_slots,
        pull_diag_cache: &backend.pull_diag_cache,
        closed_diag_gen: &backend.closed_diag_gen,
        source: "",
        pack_seq: 0,
        pack_set: None,
        registry: None,
        spec_packs: None,
        uri: &uri,
        currency: DiagCurrency::ClosedFromDisk(generation),
        version: None,
        client_supports_pull: false,
    };

    // A stale run (generation 3 < 5) is dropped.
    ctx(3).deliver_if_current(diag.clone()).await;
    assert!(
        backend.pull_diag_cache.lock().await.get(&uri).is_none(),
        "a superseded closed run must not publish",
    );
    // The current run (generation 5) publishes.
    ctx(5).deliver_if_current(diag).await;
    assert!(
        backend.pull_diag_cache.lock().await.get(&uri).is_some(),
        "the latest closed run publishes",
    );
}

/// Master switch off must clear a closed file's retained badge too — the
/// open-document reschedule alone would freeze it at its pre-toggle set.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reschedule_closed_file_diagnostics_clears_badge_when_master_off() {
    let root = unique_scratch_dir("close-master-off");
    let on_disk = root.join("warn.tcl");
    std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
    let backend = test_backend();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    // Prime the on-disk salsa source + a non-empty badge, as a prior close would.
    backend
        .db_set_source(&uri, "proc foo {} { set y 1 }\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend.pull_diag_cache.lock().await.insert(
        uri.clone(),
        PullDiagEntry {
            currency: None,
            result_id: next_pull_diag_result_id(),
            revision: u64::MAX,
            diagnostics: vec![tower_lsp_server::ls_types::Diagnostic {
                range: tower_lsp_server::ls_types::Range::default(),
                code: Some(tower_lsp_server::ls_types::NumberOrString::String(
                    "W211".to_owned(),
                )),
                ..Default::default()
            }],
        },
    );

    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "diagnostics": false })
            .as_object()
            .unwrap(),
    );
    backend.reschedule_closed_file_diagnostics().await;

    let cache = backend.pull_diag_cache.lock().await;
    let entry = cache.get(&uri).expect("entry stays, now emptied");
    assert!(
        entry.diagnostics.is_empty(),
        "master-switch-off must clear the closed file's badge: {:?}",
        entry.diagnostics,
    );
    drop(cache);
    std::fs::remove_dir_all(&root).ok();
}
