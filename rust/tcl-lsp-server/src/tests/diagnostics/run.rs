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

//! The master switch, exclusions and the fast tier.

use super::*;

/// The progressive fast tier must exclude exactly the two
/// workspace-refined analyser codes — W120 (missing `package require`) and
/// W123 (unresolved command) — and nothing else. Publishing either
/// un-refined resurfaces a startup false-positive W120.
#[test]
fn fast_tier_excludes_only_workspace_refined_codes() {
    // The two deferred codes are the whole exclusion set.
    assert!(!is_fast_tier(DiagCode::W120));
    assert!(!is_fast_tier(DiagCode::W123));

    // A representative sweep of the codes the analyser walk produces —
    // syntax errors, structural errors, local arity, style, and variable
    // lints — must all be fast: they are workspace-independent and the deep
    // pass never removes them.  Local arity (E002/E003) is fast; the deep
    // pass only ever *synthesises* additional cross-file arity, never a
    // per-file one the fast tier already showed.
    for code in [
        DiagCode::E002,
        DiagCode::E003,
        DiagCode::W100,
        DiagCode::W111,
        DiagCode::W112,
        DiagCode::W210,
        DiagCode::W211,
        DiagCode::W121,
        DiagCode::W124,
    ] {
        assert!(is_fast_tier(code), "{code} should be in the fast tier");
    }

    // Belt and braces: iterate the entire code catalogue and assert the
    // exclusion set never silently grows to something the deep pass does not
    // actually refine (which would delay a stable diagnostic for no reason).
    for code in DiagCode::ALL {
        let excluded = !is_fast_tier(*code);
        let is_workspace_refined = matches!(code, DiagCode::W120 | DiagCode::W123);
        assert_eq!(
            excluded, is_workspace_refined,
            "{code}: fast-tier exclusion must match exactly the W120/W123 \
                 workspace-refined set",
        );
    }
}

/// The progressive diagnostics path once awaited the fast tier while its
/// own deep pass sat unpolled. A lock acquire parked inside the deep pass
/// keeps its place in the lock's fair queue, so when the holder released
/// the lock it was assigned to that unpolled acquire, and the fast tier,
/// waiting for the same lock, could never get it. The lock then read as
/// free while every later waiter stayed parked (#1657).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_parked_deep_pass_does_not_strand_the_lock_the_fast_tier_needs_1657() {
    let short = std::time::Duration::from_millis(200);
    let lock = tokio::sync::Mutex::new(());
    let gate = FastTierGate::default();
    let elsewhere = lock.lock().await;
    let deep = async {
        drop(lock.lock().await);
        gate.pass().await;
    };
    tokio::pin!(deep);
    assert!(
        crate::rt::timeout(short, &mut deep).await.is_err(),
        "the deep pass must queue behind the current holder",
    );
    let hold = gate
        .hold()
        .expect("the deep pass has not reached its publication");
    drop(elsewhere);
    assert!(
        crate::rt::timeout(short, lock.lock()).await.is_err(),
        "control: with the deep pass unpolled, its queued acquire keeps the released lock",
    );
    let lock = &lock;
    let fast = async move {
        let _hold = hold;
        drop(lock.lock().await);
    };
    tokio::pin!(fast);
    let both = async {
        tokio::select! {
            biased;
            () = &mut deep => panic!("the deep pass published before the fast tier"),
            () = &mut fast => {}
        }
        deep.await;
    };
    assert!(
        crate::rt::timeout(std::time::Duration::from_secs(5), both)
            .await
            .is_ok(),
        "polling the deep pass beside the fast tier must let both finish",
    );
}

#[tokio::test]
async fn the_fast_tier_gate_orders_the_deep_publication_after_the_fast_tier() {
    let short = std::time::Duration::from_millis(50);
    let gate = FastTierGate::default();
    let hold = gate.hold().expect("an untouched gate can be held");
    let pass = gate.pass();
    tokio::pin!(pass);
    assert!(
        crate::rt::timeout(short, &mut pass).await.is_err(),
        "the deep pass must wait while the fast tier holds the gate",
    );
    drop(hold);
    assert!(
        crate::rt::timeout(short, &mut pass).await.is_ok(),
        "releasing the hold must let the deep pass through",
    );
    assert!(
        gate.hold().is_none(),
        "once the deep pass is publishing, no fast tier may follow it",
    );
}

#[tokio::test]
async fn diagnostics_master_switch_clears_the_report() {
    // `tclLsp.features.diagnostics = false` yields an empty diagnostic
    // report (clearing squiggles).
    let backend = test_backend();
    let uri = Uri::from_str("file:///d.tcl").unwrap();
    // Trailing whitespace → W112 (a source-style hint, on by default and
    // not opt-in) gives a guaranteed diagnostic to suppress.
    let src = "set x 1  \n";
    register(&backend, &uri, src).await;

    let on = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !on.is_empty(),
        "expected at least one diagnostic with the feature on: {on:?}",
    );

    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "diagnostics": false })
            .as_object()
            .unwrap(),
    );
    let off = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        off.is_empty(),
        "diagnostics disabled must yield an empty report: {off:?}",
    );
}

#[tokio::test]
async fn diagnostics_exclude_name_glob_clears_the_report() {
    // `tclLsp.diagnostics.exclude`: a name pattern empties the
    // matching document's report, leaves a non-matching sibling alone,
    // and a re-apply without the key restores the diagnostics (the
    // global list is reset unconditionally on every apply).
    let backend = test_backend();
    let excluded_uri = Uri::from_str("file:///w/gen/api.ruff").unwrap();
    let kept_uri = Uri::from_str("file:///w/gen/api.tcl").unwrap();
    let src = "set x 1  \n"; // trailing whitespace → W112
    register(&backend, &excluded_uri, src).await;
    register(&backend, &kept_uri, src).await;

    backend
        .apply_global_config(&serde_json::json!({ "diagnostics": { "exclude": ["*.ruff"] } }))
        .await
        .unwrap();
    let excluded = backend
        .full_diagnostics_for(&excluded_uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        excluded.is_empty(),
        "an excluded file must yield an empty report: {excluded:?}",
    );
    let kept = backend
        .full_diagnostics_for(&kept_uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !kept.is_empty(),
        "a non-matching sibling must keep its diagnostics",
    );

    backend
        .apply_global_config(&serde_json::json!({}))
        .await
        .unwrap();
    let restored = backend
        .full_diagnostics_for(&excluded_uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !restored.is_empty(),
        "dropping the exclude key must restore the diagnostics",
    );
}

#[tokio::test]
async fn diagnostics_exclude_folder_path_glob_is_root_relative() {
    // A folder-scoped path pattern matches against the folder-relative
    // path — `docs/**` covers the folder's `docs/` tree, not a file that
    // merely mentions `docs` elsewhere in its path or name.
    let backend = test_backend();
    let folder = crate::canonical_file_uri("/w").unwrap();
    let fc = FolderConfig {
        diagnostics_exclude: Some(vec!["docs/**".to_owned()]),
        ..FolderConfig::default()
    };
    *backend.folder_configs.lock().await = vec![(folder, fc)];

    let excluded_uri = Uri::from_str("file:///w/docs/gen.tcl").unwrap();
    let kept_uri = Uri::from_str("file:///w/src/docs.tcl").unwrap();
    let src = "set x 1  \n";
    register(&backend, &excluded_uri, src).await;
    register(&backend, &kept_uri, src).await;

    let excluded = backend
        .full_diagnostics_for(&excluded_uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        excluded.is_empty(),
        "a folder path glob must exclude its subtree: {excluded:?}",
    );
    let kept = backend
        .full_diagnostics_for(&kept_uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !kept.is_empty(),
        "`docs/**` must not match a file outside the folder's docs/ tree",
    );
}

#[tokio::test]
async fn diagnostics_exclude_folder_list_replaces_the_global_one() {
    // A folder that sets `diagnostics.exclude` replaces the global list
    // wholesale — a file matching only the global pattern keeps its
    // diagnostics inside that folder.
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "diagnostics": { "exclude": ["*.ruff"] } }))
        .await
        .unwrap();
    let folder = crate::canonical_file_uri("/w").unwrap();
    let fc = FolderConfig {
        diagnostics_exclude: Some(vec!["docs/**".to_owned()]),
        ..FolderConfig::default()
    };
    *backend.folder_configs.lock().await = vec![(folder, fc)];

    let uri = Uri::from_str("file:///w/src/api.ruff").unwrap();
    let src = "set x 1  \n";
    register(&backend, &uri, src).await;
    let diags = backend
        .full_diagnostics_for(&uri, Arc::from(src), "tcl8.6".to_owned(), "tcl")
        .await
        .unwrap();
    assert!(
        !diags.is_empty(),
        "the folder's own list must replace the global `*.ruff` pattern",
    );
}
