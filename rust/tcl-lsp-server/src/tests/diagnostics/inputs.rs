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

//! Diagnostics inputs re-resolved across configuration changes.

use super::*;

/// Reopening a document after the diagnostics master switch
/// (`tclLsp.features.diagnostics`) was turned off *while the file was
/// closed* must analyse under the current (off) switch and clear the
/// squiggles — not republish the file's pre-toggle diagnostics.
///
/// A retained [`DiagSlot`] keeps the `diagnostics_enabled = true` captured
/// on the pre-close analysis, and the reuse-cached-inputs fast path
/// (`schedule_diagnostics`, `force_refresh = false`) would drain the
/// reopen's worker under those stale on-switch inputs.  Opening a document
/// is a config-context boundary, so it force-refreshes the inputs; the
/// slot's post-reopen `diagnostics_enabled` reflecting the *current* toggle
/// proves it.  Closing also releases the slot, so the stale inputs are gone
/// by then too.
///
/// Asserted on the slot's captured inputs (set synchronously by
/// `schedule_diagnostics_impl` before the worker spawns) rather than a
/// published set, since the test client's socket is detached.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reopen_after_master_switch_off_reresolves_diagnostics_inputs() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///reopen-master-off.tcl").unwrap();
    let open = |version| DidOpenTextDocumentParams {
        text_document: tower_lsp_server::ls_types::TextDocumentItem {
            uri: uri.clone(),
            language_id: "tcl".to_owned(),
            version,
            text: "set y 1\n".to_owned(),
        },
    };
    let captured_switch = || async {
        backend
            .diag_slots
            .lock()
            .await
            .get(&uri)
            .and_then(|s| s.latest_inputs.as_ref())
            .map(|i| i.toggles.diagnostics_enabled)
    };

    // 1. Open with the master switch on (the default): the slot captures
    //    `diagnostics_enabled = true`.
    Box::pin(backend.did_open(open(1))).await;
    assert_eq!(
        captured_switch().await,
        Some(true),
        "sanity: the initial open captures the master switch as on",
    );

    // 2. Close the tab. The slot's captured inputs are released —
    //    either with the slot itself or, if its worker is still draining,
    //    as a cleared `latest_inputs`. Nothing stale survives to be reused.
    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;
    assert_eq!(
        captured_switch().await,
        None,
        "did_close must release the slot's captured inputs (#1144)",
    );

    // 3. Turn the diagnostics master switch off while the file is closed —
    //    the toggle store changes, and the closed URI holds no resolved
    //    inputs to go stale.
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({ "diagnostics": false })
            .as_object()
            .unwrap(),
    );

    // 4. Reopen. The open path must re-resolve the config-sensitive inputs
    //    rather than reuse the stale on-switch ones, so the worker drains
    //    under the master switch's *current* (off) state.
    Box::pin(backend.did_open(open(2))).await;
    assert_eq!(
        captured_switch().await,
        Some(false),
        "reopening after the master switch went off must re-resolve the \
             diagnostics inputs to the current (off) toggle, not republish the \
             file's pre-toggle squiggles from the retained slot (#104)",
    );
}

/// An edit that lands *between* a config apply and the reload's own
/// `reschedule_all_open_documents` must analyse under the config that has
/// been applied, not the one its slot cached before the apply.
///
/// That window is real and observable: `run_config_reload` applies the
/// switches first (which is the instant `tcl-lsp.getEffectiveConfig` starts
/// reporting them), then walks the disk for `SpecTcl` packs, and only then
/// reschedules.  A client that treats the command as the settle signal and
/// then types lands inside it — and without the epoch stamp, that keystroke
/// reuses inputs resolved before the apply, so a just-disabled optimiser
/// still emitted its O-codes on that publish.
///
/// Pinned without racing the debounce: the worker's first act after the
/// debounce is `capture_job`, which takes `documents`, so holding that lock
/// parks it there.  It cannot retire the slot from there (retirement needs a
/// *clean* slot, and it has already taken the dirty flag), so `latest_inputs`
/// stays observable for the whole test.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_edit_during_a_config_apply_re_resolves_its_diagnostics_inputs() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///w/epoch.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;

    let optimiser_in_slot = async || -> bool {
        backend
            .diag_slots
            .lock()
            .await
            .get(&uri)
            .expect("the slot must still exist")
            .latest_inputs
            .as_ref()
            .expect("a scheduled slot carries its inputs")
            .policy_layers
            .builder()
            .build()
            .optimiser
            .enabled
    };

    // The first edit primes the cache, exactly as a keystroke does.
    backend
        .schedule_diagnostics(uri.clone(), "tcl8.6".to_owned())
        .await;
    let docs_held = backend.documents.lock("test").await;
    assert!(
        optimiser_in_slot().await,
        "the optimiser starts enabled, so the primed inputs must say so",
    );

    // The apply half of `run_config_reload`: the switch is live and the
    // command already reports it, but no reschedule has run yet.
    backend
        .apply_global_config(&serde_json::json!({"optimiser": {"enabled": false}}))
        .await
        .unwrap();
    assert!(
        !*backend.optimiser_enabled.lock().await,
        "the apply must have landed, or this test proves nothing",
    );

    // …and the user types.  The edit path asks for no refresh; the epoch
    // stamp is the only thing that can make this correct.
    backend
        .schedule_diagnostics(uri.clone(), "tcl8.6".to_owned())
        .await;
    assert!(
        !optimiser_in_slot().await,
        "an edit arriving after a config apply must re-resolve its analyser \
             inputs — reusing the pre-apply cache republishes the O-codes the \
             user just disabled (#1651)",
    );
    drop(docs_held);
}

/// The epoch stamp is only worth anything if it describes a snapshot that
/// saw *one* configuration. `diag_inputs` is a long chain of awaits over a
/// dozen config mutexes, so an apply landing inside it produces a snapshot
/// that is half pre-change and half post-change; committing that — under
/// either epoch — puts that window straight back, because the worker
/// takes `latest_inputs` at drain time and never re-checks it.
///
/// Interleaved deterministically rather than by luck. The resolve is parked
/// on `spec_packs`, which `registry_for_dialect` takes *after*
/// `resolved_analysis_settings` has already read the optimiser switch — so
/// the parked task is holding a snapshot that says "enabled" while the test
/// flips it to disabled underneath. The park is observable because the
/// URI is fresh: `schedule_diagnostics_impl`'s first act is a
/// `slots.entry(uri).or_default()` peek, so the slot appearing in the map
/// proves the task is past it and inside the resolve, and it cannot leave
/// the resolve while the test holds `spec_packs`.
///
/// The flip is made directly rather than through `apply_global_config`
/// because that path takes `spec_packs` itself (via `sync_db_config`) and
/// would deadlock against the park. The wiring from a real apply to the
/// invalidation is pinned by
/// `an_edit_during_a_config_apply_re_resolves_its_diagnostics_inputs`; what
/// is under test here is what `schedule_diagnostics_impl` does with an
/// epoch that moves mid-resolve.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_snapshot_resolved_across_a_config_change_is_discarded_not_committed() {
    let backend = Arc::new(test_backend());
    let uri = Uri::from_str("file:///w/mid-resolve.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;

    // Park the diagnostics worker, so it cannot retire the slot out from
    // under the assertions.
    let docs_held = backend.documents.lock("test").await;
    // Park the resolve, after the optimiser switch has been read.
    let packs_held = backend.spec_packs.lock().await;

    let scheduling = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let uri = uri.clone();
        async move {
            backend.schedule_diagnostics(uri, "tcl8.6".to_owned()).await;
        }
    });

    assert!(
        wait_for(|| async { backend.diag_slots.lock().await.contains_key(&uri) }).await,
        "the scheduler must reach its slot peek, or nothing is being interleaved",
    );

    // The apply half: the switch is now off and the epoch has moved, while
    // the parked resolve still holds a snapshot that says it is on.
    backend.policy_layers.lock().await.editor =
        serde_json::json!({ "optimiser": { "enabled": false } });
    backend.invalidate_diag_inputs();
    let epoch_after_apply = backend.diag_inputs_epoch();

    drop(packs_held);
    crate::rt::timeout(std::time::Duration::from_secs(30), scheduling)
        .await
        .expect("the schedule must finish once the pack lock is free")
        .expect("schedule_diagnostics panicked");

    let slots = backend.diag_slots.lock().await;
    let slot = slots.get(&uri).expect("the slot must still exist");
    let inputs = slot
        .latest_inputs
        .as_ref()
        .expect("a scheduled slot carries its inputs");
    assert!(
        !inputs.policy_layers.builder().build().optimiser.enabled,
        "a snapshot read across the apply must be discarded and re-read, not \
             committed — publishing it republishes the O-codes the user just disabled",
    );
    assert_eq!(
        slot.inputs_epoch, epoch_after_apply,
        "the committed stamp must name the configuration the snapshot actually saw",
    );
    drop(slots);
    drop(docs_held);
}

/// Two schedules for one URI can be in flight at once and nothing makes
/// them finish in the order they started, so a resolve that read epoch N
/// can land *after* one that read N+1 and has already committed. Letting it
/// win would replace a current snapshot with an older one and stamp the slot
/// backwards — and the reuse check would then call the older one current,
/// which is worse than having no stamp at all.
///
/// The faster commit is installed directly. That is not a shortcut around a
/// race: `(latest_inputs, inputs_epoch)` at a later epoch than the one the
/// next resolve will read *is* the state that interleaving leaves behind,
/// and building it outright is what makes the assertion deterministic
/// instead of dependent on which task the runtime happens to poll first.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_late_resolve_cannot_stamp_the_slot_backwards() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///w/late-resolve.tcl").unwrap();
    register(&backend, &uri, "set x 1\n").await;
    let docs_held = backend.documents.lock("test").await;

    // What the faster scheduler committed: the optimiser off, under an
    // epoch later than anything a resolve starting now could read.
    backend.policy_layers.lock().await.editor =
        serde_json::json!({ "optimiser": { "enabled": false } });
    backend.invalidate_diag_inputs();
    backend
        .schedule_diagnostics(uri.clone(), "tcl8.6".to_owned())
        .await;
    let newer_epoch = backend.diag_inputs_epoch() + 5;
    {
        let mut slots = backend.diag_slots.lock().await;
        let slot = slots.get_mut(&uri).expect("the slot must exist");
        assert!(
            !slot
                .latest_inputs
                .as_ref()
                .expect("inputs")
                .policy_layers
                .builder()
                .build()
                .optimiser
                .enabled,
            "the newer snapshot must start out with the optimiser off",
        );
        slot.inputs_epoch = newer_epoch;
    }

    // The slower resolve now lands. It reads the configuration as it stands
    // — which the test moves back to "enabled", so an overwrite is visible —
    // and stamps it with today's epoch, which is behind the slot's.
    backend.policy_layers.lock().await.editor =
        serde_json::json!({ "optimiser": { "enabled": true } });
    backend
        .schedule_diagnostics(uri.clone(), "tcl8.6".to_owned())
        .await;

    let slots = backend.diag_slots.lock().await;
    let slot = slots.get(&uri).expect("the slot must still exist");
    assert!(
        !slot
            .latest_inputs
            .as_ref()
            .expect("inputs")
            .policy_layers
            .builder()
            .build()
            .optimiser
            .enabled,
        "a resolve stamped behind the slot must not overwrite the newer snapshot",
    );
    assert_eq!(
        slot.inputs_epoch, newer_epoch,
        "and it must not stamp the slot backwards, which would make the reuse \
             check call the older snapshot current",
    );
    drop(slots);
    drop(docs_held);
}
