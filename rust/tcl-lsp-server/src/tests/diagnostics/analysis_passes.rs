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

//! Evaluator epochs and overlay misses in the analysis passes.

use super::*;

/// The evaluator epoch reaches the query database after a diagnostics
/// pass. A hook quarantined on a worker thread moves the process's
/// epoch, and nothing a query reads says so; the refresh that follows the
/// pass takes it, compare-then-set, and reschedules nothing for it. That
/// the input re-keys the memoised lattices is `tcl-lsp-db`'s
/// `an_evaluator_epoch_re_keys_the_memoised_lattices`.
#[tokio::test]
async fn the_pass_after_a_quarantine_takes_the_evaluator_epoch() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///w/evaluator-epoch-d104.tcl").unwrap();
    backend
        .db_set_source(&uri, "proc p {} {return 1}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    let taken = || async {
        let db = backend.db.lock().await;
        tcl_lsp_db::EvaluatorEpoch::try_get(&*db).map(|epoch| epoch.generation(&*db))
    };
    std::thread::spawn(tcl_registry::pack_hooks::note_quarantine)
        .join()
        .expect("the worker quarantined");
    let moved = tcl_registry::pack_hooks::evaluator_epoch();
    assert!(
        taken().await.is_some_and(|epoch| epoch < moved),
        "the database has not seen the quarantine"
    );
    let handles = EvidenceHandles {
        db_config: Arc::clone(&backend.db_config),
        folder_db_configs: Arc::clone(&backend.folder_db_configs),
        db: Arc::clone(&backend.db),
        db_files: Arc::clone(&backend.db_files),
        db_project_members: Arc::clone(&backend.db_project_members),
        db_project: Arc::clone(&backend.db_project),
        workspace_index: Arc::clone(&backend.workspace_index),
        documents: Arc::clone(&backend.documents),
        rehoming_gate: Arc::clone(&backend.rehoming_gate),
        live_publication_gate: Arc::clone(&backend.live_publication_gate),
        class_factory_generation: Arc::clone(&backend.class_factory_generation),
    };
    let slots = Arc::new(Mutex::new(HashMap::from([(
        uri.clone(),
        DiagSlot::default(),
    )])));

    refresh_cross_file_evidence(&handles, &slots, &uri).await;

    assert!(
        taken().await.is_some_and(|epoch| epoch >= moved),
        "the refresh after the pass takes the moved epoch"
    );
    assert!(
        !slots.lock().await[&uri].dirty,
        "and reschedules nothing for it"
    );
}

/// A workspace whose packs are not installed is one line on the log: the
/// overlay and the dialect it was asked for, and what waits for it.
#[test]
fn an_overlay_miss_is_reported_with_its_key_and_what_waits() {
    let message = overlay_miss_message(&tcl_registry::model::OverlayMiss {
        environment: "tcl9.0".to_owned(),
        overlay: 0xC0FF_EE01,
    });
    assert!(message.contains("0xc0ffee01"), "{message}");
    assert!(message.contains("`tcl9.0`"), "{message}");
    assert!(
        message.contains("compiler checks and optimisations wait"),
        "{message}"
    );
    assert!(
        message.contains("highlighting read the plain registry"),
        "{message}"
    );
}
