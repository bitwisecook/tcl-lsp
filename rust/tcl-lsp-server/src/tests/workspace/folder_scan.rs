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

//! The folder scan: what it indexes, skips and builds, and the requests that wait for it.

use super::*;

/// The discovered-installation leg of the workspace scan, exercised over a
/// pinned fixture install (the harness never surveys the host's own Tcl,
/// user decision 2026-10-19T11:00Z `product_fixes`): a package under the
/// installation's `auto_path` and a `tclIndex` autoload entry both reach
/// the resolver, and a rescan reuses the memoised index survey.
#[tokio::test]
async fn scan_workspace_folders_surveys_a_pinned_discovered_installation() {
    let sys = TmpWs::new("pinned-install");
    sys.write(
        "lib/fixpkg1.0/pkgIndex.tcl",
        "package ifneeded fixpkg 1.0 [list source [file join $dir fixpkg.tcl]]\n",
    );
    sys.write("lib/fixpkg1.0/fixpkg.tcl", "package provide fixpkg 1.0\n");
    sys.write(
        "lib/fixpkg1.0/tclIndex",
        "# Tcl autoload index file, version 2.0\n\
             set auto_index(fix_helper) [list source [file join $dir fixpkg.tcl]]\n",
    );
    let lib_root = sys.0.join("lib");
    let mut backend = test_backend();
    assert!(
        backend.discovered_tcl.get().is_some_and(Vec::is_empty),
        "the harness presets an empty host-installation survey",
    );
    backend.state_mut_for_test().discovered_tcl = Arc::new(std::sync::OnceLock::from(vec![
        core_tcl_install::TclInstallation {
            version: "8.6".to_owned(),
            tcl_library: lib_root.join("tcl8.6"),
            auto_path: vec![lib_root.clone()],
        },
    ]));
    backend.scan_workspace_folders().await.unwrap();
    {
        let resolver = backend.package_resolver.read().await;
        assert!(resolver.provides("fixpkg"));
        assert!(
            resolver.auto_command_names().contains(&"fix_helper"),
            "{:?}",
            resolver.auto_command_names()
        );
    }
    let first = backend.package_resolver.read().await.revision();
    backend.scan_workspace_folders().await.unwrap();
    let resolver = backend.package_resolver.read().await;
    assert!(resolver.provides("fixpkg"), "a rescan keeps the survey");
    assert_ne!(resolver.revision(), first, "a rescan rebuilds the database");
}

/// Fix #3: `scan_workspace_folders` must (re)build the package resolver from
/// the editor library paths / discovered installations EVEN with no workspace
/// roots, so a single-file / no-folder session's W120 refinement sees the
/// user's "Select Tcl Installation" library paths.  POSITIVE: with no roots
/// but a library path that provides a package, the resolver is populated.
/// NEGATIVE (control): with roots and no library paths, it scans the tree.
#[tokio::test]
async fn scan_workspace_folders_builds_resolver_with_no_roots() {
    // POSITIVE: no workspace folders, but a `tclLsp.libraryPaths` directory
    // containing a package.  An early return on an empty root set would
    // leave the resolver empty.
    let lib_ws = TmpWs::new("noroots-lib");
    lib_ws.write(
        "mypkg/pkgIndex.tcl",
        "package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
    );
    lib_ws.write("mypkg/mypkg.tcl", "package provide mypkg 1.0\n");

    let backend = test_backend();
    // No workspace folders at all (single-file / no-folder session).
    assert!(backend.workspace_folders.lock().await.is_empty());
    *backend.editor_library_paths.lock().await = vec![
        lib_ws
            .0
            .to_str()
            .expect("library path configuration requires UTF-8")
            .to_owned(),
    ];

    backend.scan_workspace_folders().await.unwrap();

    assert!(
        backend.package_resolver.read().await.provides("mypkg"),
        "with no roots, the resolver must still be built from the editor \
             library paths so single-file sessions resolve packages",
    );

    // NEGATIVE / control: a fresh backend with a root tree and no library
    // paths still scans the workspace tree (the indexing path is unaffected).
    let root_ws = TmpWs::new("withroot");
    root_ws.write(
        "pkgIndex.tcl",
        "package ifneeded rootpkg 1.0 [list source [file join $dir r.tcl]]\n",
    );
    root_ws.write("r.tcl", "package provide rootpkg 1.0\n");
    let backend2 = test_backend();
    let root_uri = Uri::from_file_path(&root_ws.0).unwrap();
    *backend2.workspace_folders.lock().await = vec![root_uri];
    backend2.scan_workspace_folders().await.unwrap();
    assert!(
        backend2.package_resolver.read().await.provides("rootpkg"),
        "with a root and no library paths, the tree scan must still populate \
             the resolver",
    );
}

/// The startup race: `initialized()` kicks off `scan_workspace_folders`,
/// which can take a while on a real workspace, and must reschedule
/// already-open documents once it completes.  Otherwise a document opened
/// at the same time as `initialized` fires (a client's normal startup
/// sequence: `initialize` -> `initialized` with `didOpen` for restored
/// tabs arriving concurrently, see `edit_serialize`'s doc comment) has its
/// first diagnostics published against the still-empty `workspace_index` /
/// `package_resolver` and nothing ever corrects it. Asserting the document
/// is rescheduled after `initialized()` is the whole of this test; that
/// the refinement itself is correct once rescheduled is proven separately
/// by
/// `source_graph_inheritance_suppresses_w120_in_sourced_module`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn initialized_reschedules_open_documents_after_workspace_scan() {
    let backend = test_backend();
    // A document referencing a package-gated command with no local
    // `package require` — the same shape as the reported
    // `::report::defstyle` false positive.
    let caller = Uri::from_str("file:///startup-caller.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        caller.clone(),
        DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
    );
    assert!(
        backend.diag_slots.lock().await.is_empty(),
        "sanity: no diagnostics scheduled before initialized() runs",
    );

    backend.initialized(InitializedParams {}).await;

    assert!(
        backend.diag_slots.lock().await.contains_key(&caller),
        "an open document must be rescheduled once initialized()'s \
             scan_workspace_folders completes, so a diagnostics run that \
             raced ahead of the scan gets corrected instead of standing \
             indefinitely",
    );
}

/// Resolver publication is a separate currency boundary from per-batch
/// row publication. A resolver carrying an accepted row's auto-path fact
/// is refused when its configured analysis inputs changed before the final
/// resolver write.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_scan_resolver_publication_requires_current_inputs() {
    let root = TmpWs::new("scan-resolver-currentness");
    root.write(
        "pkgIndex.tcl",
        "package ifneeded stale 1.0 {package provide stale 1.0}\n",
    );
    let mut candidate = PackageResolver::new();
    candidate.scan_path(&root.0);
    assert!(candidate.provides("stale"));

    let backend = test_backend();
    let epoch = backend.diag_inputs_epoch();
    let factory_generation = *backend.class_factory_generation.read().await;
    backend
        .set_session_dialect_override_command(&[serde_json::json!("f5-irules")])
        .await
        .expect("advance the analyzer-input epoch");

    assert!(
        !backend
            .publish_workspace_scan_resolver_if_current(candidate, epoch, factory_generation, true,)
            .await
    );
    assert!(
        !backend.package_resolver.read().await.provides("stale"),
        "a stale resolver must not replace the current package database",
    );

    let current = test_backend();
    let current_epoch = current.diag_inputs_epoch();
    let current_factory_generation = *current.class_factory_generation.read().await;
    let mut current_candidate = PackageResolver::new();
    current_candidate.scan_path(&root.0);
    assert!(
        current
            .publish_workspace_scan_resolver_if_current(
                current_candidate,
                current_epoch,
                current_factory_generation,
                true,
            )
            .await
    );
    assert!(current.package_resolver.read().await.provides("stale"));
}

#[tokio::test]
async fn scan_workspace_folders_indexes_unopened_files() {
    let root = unique_scratch_dir("scan");
    std::fs::write(
        root.join("lib.tcl"),
        "proc greet {name} { puts \"hi $name\" }\n",
    )
    .unwrap();
    std::fs::write(root.join("main.tcl"), "greet world\n").unwrap();

    let backend = test_backend();
    let root_url = Uri::from_file_path(&root).unwrap();
    *backend.workspace_folders.lock().await = vec![root_url];

    backend.scan_workspace_folders().await.unwrap();

    let index = backend.workspace_index.read().await;
    let defs = index.proc_definitions("greet", "greet");
    assert!(
        !defs.is_empty(),
        "expected the unopened lib.tcl proc to be indexed",
    );
    // The call site in main.tcl should be indexed too (no
    // current-URI exclusion here).
    let invs = index.invocations_of("greet", "");
    assert!(
        !invs.is_empty(),
        "expected the unopened main.tcl call site to be indexed",
    );
    drop(index);

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn scan_workspace_folders_skips_open_documents() {
    let root = unique_scratch_dir("scan-open");
    let on_disk = root.join("buf.tcl");
    // The on-disk copy defines `stale`; the live buffer (below)
    // defines `fresh`. After the scan the index must reflect the
    // live buffer, not the on-disk copy.
    std::fs::write(&on_disk, "proc stale {} {}\n").unwrap();

    let backend = test_backend();
    let root_url = Uri::from_file_path(&root).unwrap();
    let uri = Uri::from_file_path(&on_disk).unwrap();
    *backend.workspace_folders.lock().await = vec![root_url];
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
    );
    // Seed the index from the live buffer the way did_open would.
    {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    backend.scan_workspace_folders().await.unwrap();

    let index = backend.workspace_index.read().await;
    assert!(
        !index.proc_definitions("fresh", "fresh").is_empty(),
        "live buffer's proc must survive the scan",
    );
    assert!(
        index.proc_definitions("stale", "stale").is_empty(),
        "on-disk copy must not clobber the open document",
    );
    drop(index);

    std::fs::remove_dir_all(&root).ok();
}

/// A workspace scan of unopened files must build only the
/// lightweight tier — `workspace_index` (from the scan's own analyser
/// pass) and the salsa `SourceFile` inputs — never the deep salsa tier
/// (`file_analysis_incremental` / `compilation_unit` /
/// `compiler_check_diagnostics`).  Uses the same
/// `TclDatabase::with_event_logger` technique as
/// `closed_file_republish_does_not_memoise_the_deep_compiler_tier` to
/// observe exactly which salsa queries a scan with no open documents
/// executes.
#[tokio::test]
async fn scan_workspace_folders_does_not_build_deep_tier_for_unopened_files() {
    let root = unique_scratch_dir("scan-no-deep-tier");
    std::fs::write(
        root.join("lib.tcl"),
        "proc greet {name} { puts \"hi $name\" }\n",
    )
    .unwrap();
    std::fs::write(root.join("main.tcl"), "greet world\n").unwrap();

    let executed: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let sink = Arc::clone(&executed);
    let backend = test_backend_over(tcl_lsp_db::TclDatabase::with_event_logger(move |key| {
        sink.lock().unwrap().push(key);
    }));
    let root_url = Uri::from_file_path(&root).unwrap();
    *backend.workspace_folders.lock().await = vec![root_url];

    backend.scan_workspace_folders().await.unwrap();

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
        "a workspace scan with no open documents must not build any \
             deep-tier salsa query for the files it discovers: {deep:?}",
    );

    // The lightweight index the scan's own analyser pass builds must still
    // cover both files — cross-file resolution for open documents is not
    // lost, only the eager deep-tier warm.
    let index = backend.workspace_index.read().await;
    assert!(
        !index.proc_definitions("greet", "greet").is_empty(),
        "the scan must still index the unopened file's declarations",
    );
    assert!(
        !index.invocations_of("greet", "").is_empty(),
        "the scan must still index the unopened file's call sites",
    );
    drop(index);

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn workspace_scan_merge_does_not_clobber_open_document() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///workspace/live.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
    );

    let mut analyser = Analyser::new();
    let fresh_analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &fresh_analysis);

    let mut analyser = Analyser::new();
    let stale_analysis = analyser.analyse("proc stale {} {}\n", "tcl8.6").clone();
    let scan_results = vec![(
        uri.clone(),
        "proc stale {} {}\n".to_owned(),
        "tcl8.6".to_owned(),
        stale_analysis,
    )];

    backend
        .merge_workspace_scan_results(&scan_results)
        .await
        .unwrap();

    let index = backend.workspace_index.read().await;
    assert!(
        !index.proc_definitions("fresh", "other").is_empty(),
        "open-buffer analysis must survive stale workspace scan results",
    );
    assert!(
        index.proc_definitions("stale", "other").is_empty(),
        "workspace scan must not overwrite a live buffer",
    );
}

#[tokio::test(start_paused = true)]
async fn workspace_symbol_waits_past_previous_deadlines_and_scan_gate() {
    let backend = test_backend();
    *backend.workspace_folders.lock().await = vec![Uri::from_str("file:///workspace").unwrap()];
    let scan = backend.workspace_scan_gate.lock().await;
    let mut waiting = Box::pin(backend.wait_for_workspace_scan());
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(std::future::Future::poll(
            waiting.as_mut(),
            cx
        )))
        .await
        .is_pending()
    );
    tokio::time::advance(std::time::Duration::from_secs(31)).await;
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(std::future::Future::poll(
            waiting.as_mut(),
            cx
        )))
        .await
        .is_pending()
    );
    tokio::time::advance(std::time::Duration::from_secs(11)).await;
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(std::future::Future::poll(
            waiting.as_mut(),
            cx
        )))
        .await
        .is_pending(),
        "a delayed scan must not produce an empty answer"
    );
    backend.workspace_scan_ready.mark_complete();
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(std::future::Future::poll(
            waiting.as_mut(),
            cx
        )))
        .await
        .is_pending(),
        "the in-flight scan gate must also be released"
    );
    drop(scan);
    crate::rt::timeout(std::time::Duration::from_secs(5), waiting)
        .await
        .expect("the query must resume after the scan releases its gate")
        .unwrap();
}

#[tokio::test]
async fn workspace_symbol_readiness_waits_for_completion_and_releases_on_failure() {
    let backend = test_backend();
    *backend.workspace_folders.lock().await = vec![Uri::from_str("file:///workspace").unwrap()];
    let mut waiting = Box::pin(backend.wait_for_workspace_scan());
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(20), &mut waiting)
            .await
            .is_err()
    );
    backend.workspace_scan_ready.mark_complete();
    crate::rt::timeout(std::time::Duration::from_secs(5), waiting)
        .await
        .unwrap()
        .unwrap();

    let failed = test_backend();
    *failed.workspace_folders.lock().await = vec![Uri::from_str("file:///workspace").unwrap()];
    let guard = WorkspaceStartupGuard(&failed.workspace_scan_ready);
    let mut waiting = Box::pin(failed.wait_for_workspace_scan());
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(20), &mut waiting)
            .await
            .is_err()
    );
    drop(guard);
    let error = crate::rt::timeout(std::time::Duration::from_secs(5), waiting)
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(error.message, "Workspace startup scan failed");
}

#[tokio::test]
async fn workspace_symbol_readiness_releases_on_shutdown_and_cancellation() {
    let backend = test_backend();
    *backend.workspace_folders.lock().await = vec![Uri::from_str("file:///workspace").unwrap()];
    let mut cancelled = Box::pin(backend.wait_for_workspace_scan());
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(20), &mut cancelled)
            .await
            .is_err()
    );
    drop(cancelled);
    let mut waiting = Box::pin(backend.wait_for_workspace_scan());
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(20), &mut waiting)
            .await
            .is_err()
    );
    backend.shutdown().await.unwrap();
    let error = crate::rt::timeout(std::time::Duration::from_secs(5), waiting)
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(error.message, "Workspace scan stopped during shutdown");
}

/// `ensure_autoload_indexed` must wait out an
/// in-flight `scan_workspace_folders` rather than reading
/// `package_resolver` while it is still being rebuilt — otherwise a
/// rename / go-to-definition request landing during a scan (startup, a
/// workspace-folder change, a config change) could race it and wrongly
/// conclude the command isn't auto-loadable, even though the very same
/// request moments later would have resolved correctly. Simulates the
/// race directly by holding `workspace_scan_gate` (as a real scan would
/// for its whole duration) and asserting a concurrent query blocks until
/// it is released, rather than trusting incidental timing.
#[tokio::test]
async fn autoload_indexed_query_waits_out_an_in_flight_workspace_scan() {
    let backend = Arc::new(test_backend());
    seed_autoload_library(&backend, "autoload-race").await;
    let guard = backend.workspace_scan_gate.lock().await;
    let query_backend = Arc::clone(&backend);
    let query = crate::rt::spawn(async move {
        query_backend
            .ensure_autoload_indexed("Rbc_ActiveLegend", "::")
            .await
            .unwrap()
    });
    crate::rt::yield_now().await;
    assert!(
        !query.is_finished(),
        "must wait for the in-flight scan rather than racing it"
    );
    drop(guard);
    let resolved = query.await.expect("query task panicked");
    assert!(
        resolved.is_some(),
        "resolves once the in-flight scan's gate is released"
    );
}
