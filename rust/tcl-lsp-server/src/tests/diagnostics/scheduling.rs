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

//! The debounce and the per-document slot.

use super::*;

/// A typing burst must reset the diagnostics debounce window on every edit.
/// A delay fixed from the burst's first edit launches fresh salsa reads
/// throughout it, so `set_text` repeatedly waits for their cancellation and
/// the request-side edit barrier can exceed its budget.
#[tokio::test(start_paused = true)]
async fn diagnostics_debounce_waits_for_a_quiet_window() {
    let uri = Uri::from_str("file:///debounce.tcl").unwrap();
    let slots = Arc::new(Mutex::new(HashMap::new()));
    {
        let mut guard = slots.lock().await;
        let slot = guard.entry(uri.clone()).or_insert_with(DiagSlot::default);
        slot.mark_dirty();
    }

    let waiter = crate::rt::spawn({
        let slots = Arc::clone(&slots);
        let uri = uri.clone();
        async move { stable_diagnostics_generation(&slots, &uri).await }
    });

    // Virtual time makes the producer cadence exact. With wall-clock
    // sleeps a loaded suite can delay this task past the 50 ms deadline,
    // correctly letting the waiter settle before the next nominal 30 ms
    // edit and making the test report a false early-return failure.
    crate::rt::yield_now().await;
    for _ in 0..2 {
        tokio::time::advance(std::time::Duration::from_millis(30)).await;
        slots.lock().await.get_mut(&uri).unwrap().mark_dirty();
    }

    tokio::time::advance(std::time::Duration::from_millis(49)).await;
    crate::rt::yield_now().await;
    assert!(
        !waiter.is_finished(),
        "the waiter resolved before a full quiet debounce window elapsed"
    );
    tokio::time::advance(DIAGNOSTICS_DEBOUNCE * 2).await;
    let generation = waiter.await.expect("debounce waiter task panicked");
    assert_eq!(generation, Some(3));
}

/// A clean slot must be retired before the debounce waiter yields.  A
/// subsequent edit then sees no active worker claim and can start one;
/// retiring only after the waiter returned left a gap where that edit set
/// `dirty` behind the still-running worker and no future worker was spawned.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn diagnostics_clean_slot_retires_before_waiter_returns() {
    let uri = Uri::from_str("file:///retire-clean-slot.tcl").unwrap();
    let slots = Arc::new(Mutex::new(HashMap::new()));
    slots.lock().await.insert(
        uri.clone(),
        DiagSlot {
            running: true,
            ..DiagSlot::default()
        },
    );

    assert_eq!(stable_diagnostics_generation(&slots, &uri).await, None);

    // This mirrors `schedule_diagnostics_impl`'s worker-start decision for
    // an edit arriving immediately after the clean observation.
    let starts_worker = {
        let mut guard = slots.lock().await;
        let slot = guard.entry(uri).or_insert_with(DiagSlot::default);
        slot.mark_dirty();
        if slot.running {
            false
        } else {
            slot.running = true;
            true
        }
    };
    assert!(
        starts_worker,
        "an edit after clean-slot retirement must start a diagnostics worker"
    );
}

/// `diag_slots` must shrink when a document closes.  Keeping the slot for
/// every URI ever scheduled would retain a [`DiagInputs`] — per-URI clones
/// of the disabled / extra-command / severity-override / entry-point sets —
/// for the process's life, so browsing a workspace would grow the map
/// without bound.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn did_close_drops_the_diagnostics_slot() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///slot-drop.tcl").unwrap();
    Box::pin(backend.did_open(DidOpenTextDocumentParams {
        text_document: tower_lsp_server::ls_types::TextDocumentItem {
            uri: uri.clone(),
            language_id: "tcl".to_owned(),
            version: 1,
            text: "proc foo {} { set y 1 }\n".to_owned(),
        },
    }))
    .await;
    assert!(
        backend.diag_slots.lock().await.contains_key(&uri),
        "sanity: opening a document schedules it, creating the slot",
    );

    backend
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;

    // The close releases the inputs immediately; the slot itself goes with
    // them once the debounced worker that was draining the open buffer
    // retires (`retire_slot`), so poll rather than assume an instant drop.
    assert!(
        wait_for(|| async { !backend.diag_slots.lock().await.contains_key(&uri) }).await,
        "a closed document must not keep its diagnostics slot (#1144)",
    );
}
