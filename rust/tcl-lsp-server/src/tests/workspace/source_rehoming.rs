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

//! Sourced files rehomed under their source-site namespaces.

use super::*;

/// A source-site snapshot may need the autoload tier, which waits for an
/// in-flight workspace scan. It must perform that wait *before* taking
/// `rehoming_gate`: the scan holds `workspace_scan_gate` while its merge
/// takes `rehoming_gate`, so the reverse nesting deadlocks both tasks.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rehomed_symbol_snapshot_does_not_hold_its_gate_while_waiting_for_scan() {
    let backend = Arc::new(test_backend());
    seed_autoload_library(&backend, "autoload-rehoming-order").await;
    let app = Uri::from_str("file:///app.tcl").unwrap();
    let app_src = "Rbc_ActiveLegend .g\n";
    register(&backend, &app, app_src).await;
    let analysis = Arc::new(Analyser::new().analyse(app_src, "tcl8.6").clone());

    let scan_guard = backend.workspace_scan_gate.lock().await;
    let query_backend = Arc::clone(&backend);
    let query_app = app.clone();
    let query = crate::rt::spawn(async move {
        let (guard, symbols) = query_backend
            .resolved_symbol_index_snapshot(&query_app, app_src, &analysis, Position::new(0, 3))
            .await
            .unwrap();
        drop(guard);
        symbols
    });
    crate::rt::yield_now().await;
    assert!(
        !query.is_finished(),
        "autoload preflight must wait for the in-flight workspace scan"
    );

    let rehoming_guard = crate::rt::timeout(
        std::time::Duration::from_secs(1),
        backend.rehoming_gate.lock(),
    )
    .await
    .expect("the waiting query must not hold rehoming_gate");
    drop(rehoming_guard);
    drop(scan_guard);

    let symbols = crate::rt::timeout(std::time::Duration::from_secs(2), query)
        .await
        .expect("query must complete after the scan releases its gate")
        .expect("query task panicked");
    assert_eq!(symbols, vec!["::Rbc_ActiveLegend"]);
}

/// `source` runs a file in the caller's namespace, so a bare
/// `proc helper` in a file sourced inside `namespace eval ::x` is really
/// `::x::helper` — cross-document references from a correctly-qualified
/// call site must reach the sourced file's declaration.
#[tokio::test]
async fn sourced_file_defs_rehome_under_the_source_site_namespace_m9() {
    let backend = test_backend();
    let a = Uri::from_str("file:///proj/a.tcl").unwrap();
    let b = Uri::from_str("file:///proj/b.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\nhelper\n").await;
    let a_src = "namespace eval ::x { source b.tcl }\n::x::helper\n";
    register(&backend, &a, a_src).await;
    let analysis = {
        let mut an = Analyser::new();
        an.analyse(a_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&a, a_src, &analysis, Position::new(1, 3), true)
        .await
        .unwrap();
    assert!(
        refs.iter().any(|l| l.uri == b && l.range.start.line == 0),
        "the sourced declaration must be a reference target: {refs:?}"
    );
    assert!(
        refs.iter().any(|l| l.uri == b && l.range.start.line == 1),
        "the sourced file's own bare call re-homes too: {refs:?}"
    );
}

/// The same re-homing must also fire when the `source` target is a
/// same-file constant variable rather than a literal path (`set b
/// "b.tcl"; source $b`) — otherwise the whole `source` call goes
/// untracked and re-homing silently abstains.
#[tokio::test]
async fn sourced_file_defs_rehome_through_a_same_file_constant_variable_idx_46() {
    let backend = test_backend();
    let a = Uri::from_str("file:///proj/a.tcl").unwrap();
    let b = Uri::from_str("file:///proj/b.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\nhelper\n").await;
    let a_src = "namespace eval ::x { set b \"b.tcl\"; source $b }\n::x::helper\n";
    register(&backend, &a, a_src).await;
    let analysis = {
        let mut an = Analyser::new();
        an.analyse(a_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&a, a_src, &analysis, Position::new(1, 3), true)
        .await
        .unwrap();
    assert!(
        refs.iter().any(|l| l.uri == b && l.range.start.line == 0),
        "the sourced declaration must be a reference target: {refs:?}"
    );
    assert!(
        refs.iter().any(|l| l.uri == b && l.range.start.line == 1),
        "the sourced file's own bare call re-homes too: {refs:?}"
    );
}

/// Declaration side: references from the sourced file's own
/// declaration cursor reach the sourcing document's qualified call.
#[tokio::test]
async fn sourced_file_declaration_finds_qualified_callers_m9() {
    let backend = test_backend();
    let a = Uri::from_str("file:///proj/a.tcl").unwrap();
    let b = Uri::from_str("file:///proj/b.tcl").unwrap();
    let b_src = "proc helper {} {}\n";
    register(&backend, &b, b_src).await;
    register(
        &backend,
        &a,
        "namespace eval ::x { source b.tcl }\n::x::helper\n",
    )
    .await;
    // Reconcile, then resolve from b.tcl's declaration.
    backend.refresh_source_rehoming().await.unwrap();
    let analysis = {
        let mut an = Analyser::new();
        an.analyse(b_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&b, b_src, &analysis, Position::new(0, 6), false)
        .await
        .unwrap();
    assert!(
        refs.iter().any(|l| l.uri == a && l.range.start.line == 1),
        "the qualified call in the sourcing document is a reference: {refs:?}"
    );
}

/// An ordinary top-level `source b.tcl` — no namespace, the
/// normal way to write Tcl — must reconcile to a fixed point.
///
/// A document sourced only from the global namespace wants exactly the
/// standalone analysis the index already holds, so
/// [`crate::BackendState::rehomed_source_seeds`] records it as *absence*.  A work
/// queue comparing against `Some(["::"])` instead — a value the store never
/// writes — would queue, re-analyse and re-index the document on every
/// round of every call, for ever.
///
/// The generation assertion is the one with the user-visible teeth: every
/// index mutation bumps it, and every `Derived` memo on the index (settled
/// invocations, defined names, command links, run order) is dropped when it
/// moves, so a non-converging pass throws the whole memo tier away on every
/// navigation request.
#[tokio::test]
async fn a_global_source_site_converges_without_touching_the_index_1297() {
    let backend = test_backend();
    let a = Uri::from_str("file:///global-source/a.tcl").unwrap();
    let b = Uri::from_str("file:///global-source/b.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\n").await;
    register(&backend, &a, "source b.tcl\nhelper\n").await;
    let before = backend.workspace_index.read().await.generation();

    backend.refresh_source_rehoming().await.unwrap();

    assert!(
        backend.rehomed_source_seeds.lock().await.is_empty(),
        "a document sourced only from the global namespace is not re-homed, \
             so the map holds no entry for it",
    );
    let after_first = backend.workspace_index.read().await.generation();
    assert_eq!(
        before, after_first,
        "there is nothing to re-home, so the pass must not re-index anything",
    );
    backend.refresh_source_rehoming().await.unwrap();
    assert_eq!(
        after_first,
        backend.workspace_index.read().await.generation(),
        "a second pass over converged state must be a no-op — while it was \
             not, every index-generation memo was invalidated on every \
             navigation request",
    );
}

/// Mixed shape: one document genuinely re-homed under `::ns`
/// and another sourced from global scope, in the same workspace.  Both must
/// reach a fixed point, and the re-homed one must keep its seed.
#[tokio::test]
async fn mixed_global_and_namespaced_source_sites_reach_a_fixed_point_1297() {
    let backend = test_backend();
    let a = Uri::from_str("file:///mixed-source/a.tcl").unwrap();
    let b = Uri::from_str("file:///mixed-source/b.tcl").unwrap();
    let c = Uri::from_str("file:///mixed-source/c.tcl").unwrap();
    let d = Uri::from_str("file:///mixed-source/d.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\n").await;
    register(&backend, &c, "proc plain {} {}\n").await;
    register(&backend, &a, "namespace eval ::ns { source b.tcl }\n").await;
    register(&backend, &d, "source c.tcl\n").await;

    backend.refresh_source_rehoming().await.unwrap();

    assert_eq!(
        backend
            .rehomed_source_seeds
            .lock()
            .await
            .iter()
            .map(|(uri, seeds)| (uri.clone(), seeds.clone()))
            .collect::<std::collections::BTreeMap<_, _>>(),
        [(b.as_str().to_owned(), vec!["::ns".to_owned()])]
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>(),
        "only the namespaced source site is re-homed; the global one is not",
    );
    {
        let index = backend.workspace_index.read().await;
        assert!(
            index.workspace_command_exists("::ns::helper"),
            "the re-homed twin is indexed",
        );
        assert!(
            index.workspace_command_exists("::plain"),
            "the globally-sourced document keeps its standalone view",
        );
    }

    let settled = backend.workspace_index.read().await.generation();
    backend.refresh_source_rehoming().await.unwrap();
    backend.refresh_source_rehoming().await.unwrap();
    assert_eq!(
        settled,
        backend.workspace_index.read().await.generation(),
        "both documents are converged, so further passes do no work",
    );
}

/// Transition: a document re-homed under `::ns` whose source
/// site later moves to global scope must end up standalone, lose its record,
/// and converge from there.  This is the direction the `recorded` loop
/// serves, and the one that proves filtering the standalone view out of the
/// desired set has not stranded anything.
#[tokio::test]
async fn a_source_site_moving_to_global_scope_restores_the_standalone_view_1297() {
    let backend = test_backend();
    let a = Uri::from_str("file:///unwrapped-source/a.tcl").unwrap();
    let b = Uri::from_str("file:///unwrapped-source/b.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\n").await;
    register(&backend, &a, "namespace eval ::ns { source b.tcl }\n").await;
    backend.refresh_source_rehoming().await.unwrap();
    assert_eq!(
        backend
            .rehomed_source_seeds
            .lock()
            .await
            .get(b.as_str())
            .cloned(),
        Some(vec!["::ns".to_owned()]),
        "precondition: b.tcl is re-homed under ::ns",
    );

    // The user unwraps the `namespace eval`: the same `source` now runs in
    // the global namespace.
    backend
        .workspace_index
        .write()
        .await
        .remove_document(a.as_str());
    register(&backend, &a, "source b.tcl\n").await;
    backend.refresh_source_rehoming().await.unwrap();

    assert!(
        backend.rehomed_source_seeds.lock().await.is_empty(),
        "the document is standalone again, so its record goes with the seed",
    );
    {
        let index = backend.workspace_index.read().await;
        assert!(
            index.workspace_command_exists("::helper"),
            "the standalone view is back in the index",
        );
        assert!(
            !index.workspace_command_exists("::ns::helper"),
            "and the re-homed twin is gone",
        );
    }
    let settled = backend.workspace_index.read().await.generation();
    backend.refresh_source_rehoming().await.unwrap();
    assert_eq!(
        settled,
        backend.workspace_index.read().await.generation(),
        "the restored state is itself a fixed point",
    );
}

/// A file sourced into **several** namespaces is one
/// physical declaration with one runtime identity per source site
/// (tclsh 9.0.4: `namespace eval ::x {source b.tcl}` + `namespace eval
/// ::y {source b.tcl}` yields both `::x::helper` and `::y::helper`).
/// Declaration-side navigation must union every view — never map the
/// cursor to the first sorted seed and lose the other callers.
#[tokio::test]
async fn multi_seeded_declaration_unions_every_runtime_identity_945() {
    let backend = test_backend();
    let a = Uri::from_str("file:///proj/a.tcl").unwrap();
    let b = Uri::from_str("file:///proj/b.tcl").unwrap();
    let b_src = "proc helper {} {namespace current}\n";
    register(&backend, &b, b_src).await;
    register(
            &backend,
            &a,
            "namespace eval ::x { source b.tcl }\nnamespace eval ::y { source b.tcl }\n::x::helper\n::y::helper\n",
        )
        .await;
    backend.refresh_source_rehoming().await.unwrap();
    let analysis = {
        let mut an = Analyser::new();
        an.analyse(b_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&b, b_src, &analysis, Position::new(0, 6), false)
        .await
        .unwrap();
    let caller_lines: std::collections::BTreeSet<u32> = refs
        .iter()
        .filter(|l| l.uri == a)
        .map(|l| l.range.start.line)
        .collect();
    assert_eq!(
        caller_lines,
        [2, 3].into_iter().collect(),
        "both seeded views' callers must surface: {refs:?}"
    );
    // Rename from the declaration is an explicit multi-symbol edit:
    // the one physical token changes, and *every* view's callers
    // follow — leaving no runtime identity dispatching the old name.
    let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
        std::collections::HashMap::new();
    let blocked = backend
        .add_cross_document_rename_edits(
            &b,
            b_src,
            &analysis,
            Position::new(0, 6),
            "assist",
            &mut changes,
        )
        .await
        .unwrap();
    assert!(!blocked, "nothing blocks this rename");
    let a_edit_lines: std::collections::BTreeSet<u32> = changes
        .get(&a)
        .map_or_default(|edits| edits.iter().map(|e| e.range.start.line).collect());
    assert_eq!(
        a_edit_lines,
        [2, 3].into_iter().collect(),
        "both views' callers must be rewritten: {changes:?}"
    );
}

/// Concurrent `refresh_source_rehoming` callers (every navigation handler
/// calls it at its own entry) must not each run their own full re-analysis
/// of the same re-homed document — that duplicates disk reads / analyses
/// and serialises on `workspace_index`'s write lock, turning one caller's
/// O(sourced files) convergence cost into O(concurrent callers × that
/// cost) and head-of-line-blocking unrelated requests behind it.
///
/// Proxy for "did the real reconciliation work run more than once":
/// each genuine reconciliation of a document is one `remove_document` +
/// one `add_document`, so it bumps [`core_workspace_index::WorkspaceIndex::generation`]
/// by a fixed amount (measured here on a solo backend as `single_pass`, since
/// the source-view graph publishes more than the document's remove and add);
/// a second, redundant pass over the same document would bump it again. Run on a real multi-thread runtime with genuine
/// `tokio::spawn` tasks (not `tokio::join!` on one task, which never
/// gives two `.await`-separated bodies true concurrent access to the
/// gate) so the two calls can actually race for
/// [`crate::BackendState::rehoming_gate`] — with the gate, the outcome is
/// deterministic regardless of scheduling: whichever call acquires it
/// second always finds the reconciliation already converged, because the
/// first call fully completes (and releases the gate) before the second
/// can proceed. A build without the gate could still pass this test on an
/// unlucky schedule (the two spawned tasks not actually overlapping), so
/// it is a genuine, non-flaky proof that the gate dedups rather than a
/// guaranteed catch of its absence — the reconciliation
/// path's several `.await` points (disk read, `spawn_blocking`, index
/// write) give a wide window for the race to manifest in practice.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rehoming_gate_dedups_concurrent_reconciliation_passes() {
    let single_pass = {
        let solo = test_backend();
        let a = Uri::from_str("file:///proj/a.tcl").unwrap();
        let b = Uri::from_str("file:///proj/b.tcl").unwrap();
        register(&solo, &b, "proc helper {} {}\n").await;
        register(
            &solo,
            &a,
            "namespace eval ::x { source b.tcl }\n::x::helper\n",
        )
        .await;
        let before = solo.workspace_index.read().await.generation();
        solo.refresh_source_rehoming().await.unwrap();
        solo.workspace_index
            .read()
            .await
            .generation()
            .wrapping_sub(before)
    };
    assert!(
        single_pass > 0,
        "one reconciliation must publish into the index"
    );
    let backend = Arc::new(test_backend());
    let a = Uri::from_str("file:///proj/a.tcl").unwrap();
    let b = Uri::from_str("file:///proj/b.tcl").unwrap();
    register(&backend, &b, "proc helper {} {}\n").await;
    register(
        &backend,
        &a,
        "namespace eval ::x { source b.tcl }\n::x::helper\n",
    )
    .await;

    let before = backend.workspace_index.read().await.generation();
    let (b1, b2) = (Arc::clone(&backend), Arc::clone(&backend));
    let (r1, r2) = tokio::join!(
        crate::rt::spawn(async move { b1.refresh_source_rehoming().await.unwrap() }),
        crate::rt::spawn(async move { b2.refresh_source_rehoming().await.unwrap() }),
    );
    r1.expect("first pass must not panic");
    r2.expect("second pass must not panic");
    let after = backend.workspace_index.read().await.generation();
    assert_eq!(
        after.wrapping_sub(before),
        single_pass,
        "two concurrent reconciliation passes must not each re-index the same document",
    );

    // The result is still correct: b.tcl's declaration is reachable
    // under its seeded identity from a.tcl's qualified call.
    let b_src = "proc helper {} {}\n";
    let analysis = {
        let mut an = Analyser::new();
        an.analyse(b_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&b, b_src, &analysis, Position::new(0, 6), false)
        .await
        .unwrap();
    assert!(
        refs.iter().any(|l| l.uri == a),
        "the sourcing document's qualified call must still resolve: {refs:?}"
    );
}

/// Reconciliation owns `rehoming_gate`, while
/// a pending live publication needs that gate to mark its document ready.
/// Waiting through `read_document` here therefore formed a direct cycle.
/// A pass that encounters the pending sourced document must retire, release
/// the gate, and let the next pass reconcile the published revision.
#[tokio::test]
async fn rehoming_never_waits_for_live_publication_that_needs_its_gate_1854() {
    let backend = test_backend();
    let parent = Uri::from_str("file:///rehoming-publication/a.tcl").unwrap();
    let sourced = Uri::from_str("file:///rehoming-publication/b.tcl").unwrap();
    register(&backend, &sourced, "proc helper {} {}\n").await;
    register(
        &backend,
        &parent,
        "namespace eval ::live { source b.tcl }\n",
    )
    .await;
    backend
        .documents
        .lock("test")
        .await
        .get_mut(&sourced)
        .expect("the sourced document is open")
        .publication = DocumentPublication::Pending;

    // A pass over a sourced document whose publication stays pending retries
    // a bounded number of times (SOURCE_VIEW_CONTENTION_ATTEMPTS), then
    // refuses; a deadlock would never return, so the bound is generous.
    let refused = crate::rt::timeout(
        std::time::Duration::from_secs(60),
        backend.refresh_source_rehoming(),
    )
    .await
    .expect("rehoming must release its gate instead of waiting for publication");
    assert!(
        matches!(
            refused,
            Err(crate::DocumentSourceUnavailable::ConditionalAdvice { ref uri }) if *uri == parent || *uri == sourced
        ),
        "a persistently pending sourced document refuses the pass: {refused:?}"
    );
    assert!(
        backend.rehoming_gate.try_lock().is_ok(),
        "the pending publisher must be able to acquire the released gate",
    );
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::live::helper"),
        "a pending revision must not publish a stale re-homed view",
    );

    backend
        .documents
        .lock("test")
        .await
        .get_mut(&sourced)
        .expect("the sourced document remains open")
        .publication = DocumentPublication::Indexed;
    backend.live_publication_advanced.notify_waiters();
    backend.refresh_source_rehoming().await.unwrap();
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::live::helper"),
        "the next pass must reconcile the published live revision",
    );
}

/// A closed-file rehoming snapshot must release
/// the global document map before waiting for Salsa and must reject the
/// captured disk source if the editor opens the URI in that interval.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rehoming_snapshot_releases_documents_and_rechecks_open_1854() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///rehoming-snapshot-1854.tcl").unwrap();
    backend
        .db_set_source(&uri, "proc disk {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();

    let database = backend.db.lock().await;
    let snapshot = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move { backend.rehoming_document_snapshot(&uri).await }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend.db.contention().contains("1 queued waiter") {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the rehoming snapshot must reach the held Salsa database");
    let mut documents = crate::rt::timeout(
        std::time::Duration::from_millis(500),
        backend.documents.lock("rehoming-review"),
    )
    .await
    .expect("a rehoming Salsa wait retained the global document map");
    documents.insert(
        uri.clone(),
        DocumentState::with_version("proc live {} {}\n".to_owned(), "tcl8.6".to_owned(), 1),
    );
    drop(documents);
    drop(database);

    assert!(matches!(
        snapshot.await.expect("snapshot task panicked").unwrap(),
        RehomingDocumentSnapshot::Pending
    ));
}
