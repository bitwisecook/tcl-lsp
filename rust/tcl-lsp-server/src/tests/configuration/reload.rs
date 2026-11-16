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

//! The configuration pull, its failure modes and the coalesced reload.

use super::*;

/// The session's analyser skip is the production skip of its layers: the
/// per-code decisions and the default-off seed a layer did not turn on,
/// and never a family gate — the shimmer switch is the policy's, not the
/// skip's.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_session_skip_is_the_layers_production_skip() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({
            "diagnostics": { "W210": false, "W242": true }
        }))
        .await
        .unwrap();
    let skip = backend.disabled_diagnostics.lock().await.clone();
    assert_eq!(
        skip,
        std::iter::once("W210".to_owned()).collect::<HashSet<String>>(),
        "the layer's `false` is in it and its `true` lifts the seed",
    );

    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "shimmer": { "enabled": false } }))
        .await
        .unwrap();
    let uri = Uri::from_str("file:///s.tcl").unwrap();
    let (disabled, ..) = backend.resolved_analysis_settings(&uri).await;
    assert!(
        disabled.iter().all(|code| !code.starts_with('S')),
        "no shimmer code is skipped: {disabled:?}",
    );
    let policy = backend.resolved_policy_layers(&uri).await.builder().build();
    assert_eq!(
        policy.code_reason(DiagCode::S100),
        Some(core_policy::Reason::ShimmerOff),
        "the switch hides the family in the policy step",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_change_configuration_applies_inline_settings() {
    let backend = test_backend();
    let params = DidChangeConfigurationParams {
        settings: serde_json::json!({
            "dialect": "tcl9.0",
            "tclLsp.style.nonAscii": "strict",
            "tclLsp.diagnostics.W211": false,
        }),
    };
    // `did_change_configuration` ends by pulling config from the client,
    // which errors fast against the test's detached socket; the timeout is
    // a backstop so a future transport change can't hang the suite.
    crate::rt::timeout(
        std::time::Duration::from_secs(10),
        backend.did_change_configuration(params),
    )
    .await
    .expect("did_change_configuration should not hang");
    assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
    assert_eq!(*backend.non_ascii_mode.lock().await, NonAsciiMode::Strict);
    assert!(
        backend.disabled_diagnostics.lock().await.contains("W211"),
        "W211 should be disabled by the inline settings",
    );
}

/// A burst of `didChangeConfiguration` notifications must
/// coalesce into a single reload.  The leader is the only handler that runs
/// the pipeline; every other notification in the window returns straight
/// away after applying its inline settings.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_configuration_burst_coalesces_into_one_reload() {
    let backend = test_backend();
    let notify = |dialect: &str| DidChangeConfigurationParams {
        settings: serde_json::json!({ "dialect": dialect }),
    };
    // `tower_lsp_server` drives notifications concurrently
    // (`buffer_unordered`, see `EditOrder`), so the burst is modelled that
    // way: one handler becomes the leader and parks for the debounce
    // window, and the other fifteen land while it is parked.
    let leader = backend.did_change_configuration(notify("tcl8.4"));
    let dialects = ["tcl8.5", "tcl8.6", "tcl9.0"];
    let followers = async {
        // Let the leader claim the slot before the followers arrive.
        crate::rt::sleep(std::time::Duration::from_millis(10)).await;
        let start = std::time::Instant::now();
        for i in 0..15 {
            backend
                .did_change_configuration(notify(dialects[i % dialects.len()]))
                .await;
        }
        // The whole point: a follower applies its inline settings and
        // returns — it never waits for a pull, let alone its own window.
        assert!(
            start.elapsed() < CONFIG_RELOAD_DEBOUNCE,
            "followers must not each wait out a debounce window: {:?}",
            start.elapsed(),
        );
        // All fifteen are folded into the single pending generation.
        let slot = backend.config_reload.lock().await;
        assert!(slot.running, "exactly one leader owns the reload");
        assert!(slot.dirty, "the burst is pending for that leader");
    };
    tokio::join!(leader, followers);

    // The leader has run the pipeline and retired, and the state it settled
    // on is the *last* notification's — a superseded generation may be
    // dropped, the final one never may.
    {
        let slot = backend.config_reload.lock().await;
        assert!(
            !slot.running,
            "the leader must retire once the burst drains"
        );
        assert!(!slot.dirty);
    }
    assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
}

/// The coalescing must not swallow the final generation: after the window
/// closes the leader runs the pipeline and retires.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_last_generation_of_a_burst_still_runs_to_completion() {
    let backend = test_backend();
    for _ in 0..4 {
        backend
            .did_change_configuration(DidChangeConfigurationParams {
                settings: serde_json::json!({ "dialect": "tcl9.0" }),
            })
            .await;
    }
    // The leader is still sleeping out its window; wait for it to run and
    // then retire. `client.configuration` errors fast against the test's
    // detached socket, so the pipeline completes rather than hanging.
    let retired = wait_for(|| async {
        let slot = backend.config_reload.lock().await;
        !slot.running && !slot.dirty
    })
    .await;
    assert!(retired, "the leader must run the reload and then retire");
    assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
}

/// A failed scoped configuration pull must not freeze what a folder
/// *inherits*. The unscoped pull that preceded it may have moved a global,
/// and a folder handle snapshots every input it does not itself override —
/// so the fallback re-applies the last-known chain and the handle picks the
/// new global up, instead of answering `resolved_db_config` with the old
/// one while the global handle already carries the new.
#[tokio::test]
async fn a_failed_scoped_pull_refreshes_what_a_folder_inherits() {
    async fn inherited_bigip(backend: &Backend, uri: &Uri) -> Option<String> {
        let config = backend.resolved_db_config(uri).await;
        let db = backend.db.lock().await;
        ResourceAnalyserInputs::from_db_config(config, &*db).bigip_version
    }

    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "bigipVersion": "17.1.0" }))
        .await
        .unwrap();

    let folder = Uri::from_str("file:///w/").unwrap();
    backend
        .apply_folder_configs(vec![(
            folder,
            // Overrides an unrelated knob, so it gets its own handle and
            // inherits `bigipVersion` by snapshot.
            parse_folder_config(&serde_json::json!({ "extraCommands": ["zzz"] }))
                .expect("folder config parses"),
        )])
        .await;

    let uri = Uri::from_str("file:///w/a.tcl").unwrap();
    assert_eq!(
        inherited_bigip(&backend, &uri).await.as_deref(),
        Some("17.1.0")
    );

    // The unscoped pull lands a new global, then the scoped one fails. The
    // primary root carries no folder settings of its own, so it has no
    // entry to patch and no signature override — the shape that used to
    // return without touching the chain at all.
    backend
        .apply_global_config(&serde_json::json!({ "bigipVersion": "16.1.0" }))
        .await
        .unwrap();
    let primary = Uri::from_str("file:///primary/").unwrap();
    backend
        .apply_primary_project_signature_fallback(Some(&primary), &serde_json::json!({}))
        .await;

    assert_eq!(
        inherited_bigip(&backend, &uri).await.as_deref(),
        Some("16.1.0"),
        "an inherited input follows the global it inherits"
    );
}

#[tokio::test]
async fn primary_project_signature_exclusions_do_not_leak_into_sibling_root() {
    let backend = test_backend();
    let folder_a = Uri::from_str("file:///proj-a").unwrap();
    let folder_b = Uri::from_str("file:///proj-b").unwrap();
    let file_a = Uri::from_str("file:///proj-a/file.tcl").unwrap();
    let file_b = Uri::from_str("file:///proj-b/file.tcl").unwrap();
    *backend.workspace_folders.lock().await = vec![folder_a.clone(), folder_b.clone()];

    let global_editor = serde_json::json!({});
    let primary_merged = serde_json::json!({
        "signatureHelp": { "disabledCommands": ["set"] }
    });
    backend
        .apply_global_config_with_signature_fallback(&primary_merged, &global_editor)
        .await
        .unwrap();
    backend
        .apply_folder_configs(vec![
            (
                folder_a,
                parse_folder_config(&primary_merged).expect("primary config"),
            ),
            (
                folder_b,
                parse_folder_config(&global_editor).expect("sibling config"),
            ),
        ])
        .await;

    assert_eq!(
        backend
            .resolved_signature_help_disabled_commands(&file_a)
            .await,
        vec!["::set".to_owned()],
        "the primary project keeps its own exclusion",
    );
    assert!(
        backend
            .resolved_signature_help_disabled_commands(&file_b)
            .await
            .is_empty(),
        "a sibling without an exclusion inherits only global/editor layers",
    );
}

#[tokio::test]
async fn scoped_pull_failure_keeps_primary_project_signature_exclusions_isolated() {
    let backend = test_backend();
    let folder_a = Uri::from_str("file:///proj-a").unwrap();
    let folder_b = Uri::from_str("file:///proj-b").unwrap();
    let file_a = Uri::from_str("file:///proj-a/file.tcl").unwrap();
    let file_b = Uri::from_str("file:///proj-b/file.tcl").unwrap();
    *backend.workspace_folders.lock().await = vec![folder_a.clone(), folder_b.clone()];
    *backend.signature_help_disabled_commands.lock().await = vec!["::format".to_owned()];

    // A prior successful scoped pull for the sibling must survive a later
    // scoped-request failure while the primary's project INI is recovered.
    backend
        .apply_folder_configs(vec![(
            folder_b,
            FolderConfig {
                signature_help_disabled_commands: Some(vec!["::incr".to_owned()]),
                ..FolderConfig::default()
            },
        )])
        .await;
    backend
        .apply_primary_project_signature_fallback(
            Some(&folder_a),
            &serde_json::json!({
                "signatureHelp": { "disabledCommands": ["set"] }
            }),
        )
        .await;

    assert_eq!(
        backend
            .resolved_signature_help_disabled_commands(&file_a)
            .await,
        vec!["::set".to_owned()],
        "the primary root keeps its project exclusion after a scoped pull failure",
    );
    assert_eq!(
        backend
            .resolved_signature_help_disabled_commands(&file_b)
            .await,
        vec!["::incr".to_owned()],
        "the fallback must preserve and not overwrite a sibling's last scoped value",
    );

    backend
        .apply_primary_project_signature_fallback(Some(&folder_a), &serde_json::json!({}))
        .await;
    assert_eq!(
        backend
            .resolved_signature_help_disabled_commands(&file_a)
            .await,
        vec!["::format".to_owned()],
        "removing the project setting restores global/editor inheritance",
    );
    assert_eq!(
        backend
            .resolved_signature_help_disabled_commands(&file_b)
            .await,
        vec!["::incr".to_owned()],
        "clearing the primary override must still leave the sibling untouched",
    );
}
