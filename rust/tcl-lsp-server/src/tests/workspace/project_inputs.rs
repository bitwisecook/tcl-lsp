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

//! Cross-file evidence and coverage published to project files.

use super::*;

/// The project-wide evidence read runs on a cloned salsa snapshot off the
/// `db` mutex, so the write section has to re-check
/// currency against the live database rather than trusting the snapshot's
/// view.  A second pass over an unchanged project must therefore write
/// nothing — no input moves, nothing downstream is invalidated, and no peer
/// is rescheduled — which is also what stops the worker/peer loop from
/// running forever.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cross_file_evidence_sync_settles_after_one_pass() {
    let backend = test_backend();
    let lib = Uri::from_str("file:///w/lib.tcl").unwrap();
    let main = Uri::from_str("file:///w/main.tcl").unwrap();
    backend
        .db_set_source(
            &lib,
            "proc helper {mode} { return $mode }\n",
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();
    backend
        .db_set_source(&main, "helper dev\n", "tcl8.6".to_owned())
        .await
        .unwrap();
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

    let first = sync_cross_file_evidence(&handles).await;
    assert_eq!(
        first.changed.len(),
        2,
        "the cold pass gives both covered files a view: {:?}",
        first.changed,
    );
    let second = sync_cross_file_evidence(&handles).await;
    assert!(
        second.changed.is_empty(),
        "an unchanged project must write no input: {:?}",
        second.changed,
    );

    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    let evidence = files[&lib]
        .external_call_sites(&*db)
        .clone()
        .expect("a covered file gets a view");
    assert_eq!(
        evidence.callees().collect::<Vec<_>>(),
        vec!["::helper"],
        "main.tcl's call reaches lib.tcl's proc"
    );
}

/// Coverage and cross-file evidence must read
/// the same Salsa revision. Ordinary edits intentionally retain the prior
/// workspace-index slot until diagnostics republishes it, so an old
/// literal target there must not hide a new external target in Salsa.
#[tokio::test]
async fn covered_load_targets_use_the_captured_salsa_revision_1854() {
    use salsa::Setter as _;

    let backend = test_backend();
    let main = Uri::from_str("file:///w/main.tcl").unwrap();
    let target = Uri::from_str("file:///w/target.tcl").unwrap();
    backend
        .db_set_source(&main, "source target.tcl\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .db_set_source(&target, "proc target {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = Analyser::new()
        .analyse("source target.tcl\n", "tcl8.6")
        .clone();
    backend
        .workspace_index
        .write()
        .await
        .replace_document(main.as_str(), &analysis);
    {
        let mut db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        files[&main]
            .set_text(&mut *db)
            .to("source ../outside.tcl\n".to_owned());
    }
    assert_eq!(
        backend.workspace_index.read().await.sources().count(),
        1,
        "the regression requires the previous covered index view",
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
    let captured = capture_cross_file_evidence_snapshot(&handles)
        .await
        .expect("the project must remain initialised");
    let covered = files_with_covered_load_targets(
        &captured.snapshot,
        &captured.files,
        &captured.members,
        captured.project,
    );
    assert!(
        !covered.contains(&main),
        "the current external target must defeat the stale covered index view",
    );
}

/// Diagnostics for the driving document are computed before evidence
/// refresh. If its new text turns a covered
/// `source` into an external one, losing `Some(evidence)` must schedule the
/// second pass that withdraws any fold made from that stale closed world.
#[tokio::test]
async fn coverage_loss_marks_the_driving_diagnostics_dirty_1854() {
    use salsa::Setter as _;

    let backend = test_backend();
    let main = Uri::from_str("file:///w/coverage-loss-main-1854.tcl").unwrap();
    let target = Uri::from_str("file:///w/coverage-loss-target-1854.tcl").unwrap();
    backend
        .db_set_source(
            &main,
            "source coverage-loss-target-1854.tcl\n",
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();
    backend
        .db_set_source(&target, "proc target {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
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
    let initial = sync_cross_file_evidence(&handles).await;
    assert!(initial.changed.contains(&main));
    {
        let mut db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        assert!(files[&main].external_call_sites(&*db).is_some());
        files[&main]
            .set_text(&mut *db)
            .to("source ../outside.tcl\n".to_owned());
    }
    let slots = Arc::new(Mutex::new(HashMap::from([(
        main.clone(),
        DiagSlot::default(),
    )])));

    refresh_cross_file_evidence(&handles, &slots, &main).await;

    assert!(
        slots.lock().await[&main].dirty,
        "coverage loss must schedule the driving document's soundness pass",
    );
}

/// An edited orphan keeps a local Salsa
/// handle in `db_files`, but must lose the workspace evidence it carried
/// before leaving the admitted project set.
#[tokio::test]
async fn cross_file_evidence_clears_local_only_orphans_1854() {
    let backend = test_backend();
    let main = Uri::from_str("file:///w/covered-main-1854.tcl").unwrap();
    let orphan = Uri::from_str("file:///w/covered-orphan-1854.tcl").unwrap();
    backend
        .db_set_source(&main, "local\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .db_set_source(&orphan, "proc local {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
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
    assert!(
        sync_cross_file_evidence(&handles)
            .await
            .changed
            .contains(&orphan),
        "the admitted file must first receive call-site evidence",
    );
    {
        let mut db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let mut members = backend.db_project_members.lock().await;
        assert!(members.remove(&orphan));
        let mut project = backend.db_project.lock().await;
        Backend::sync_db_project(&mut db, &files, &members, &mut project);
    }
    assert!(
        backend.db_files.lock().await.contains_key(&orphan),
        "the regression requires the orphan local handle to remain",
    );
    assert!(
        sync_cross_file_evidence(&handles)
            .await
            .changed
            .contains(&orphan),
        "retiring the orphan's evidence must be reported as a change",
    );
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert!(
        files[&orphan].external_call_sites(&*db).is_none(),
        "a local-only orphan must not retain project evidence",
    );
}

#[tokio::test]
async fn cross_file_evidence_rejects_a_stale_write_revision_1854() {
    let backend = test_backend();
    let main = Uri::from_str("file:///w/stale-evidence-main-1854.tcl").unwrap();
    let lib = Uri::from_str("file:///w/stale-evidence-lib-1854.tcl").unwrap();
    backend
        .db_set_source(&main, "helper dev\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .db_set_source(
            &lib,
            "proc helper {mode} { return $mode }\n",
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();
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
    assert!(
        sync_cross_file_evidence(&handles)
            .await
            .changed
            .contains(&lib),
    );
    let (captured_revision, lib_file, stale_want) = {
        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let file = files[&lib];
        (
            tcl_lsp_db::database_revision(&db),
            file,
            file.external_call_sites(&*db).clone(),
        )
    };
    assert!(stale_want.is_some(), "the control requires stale evidence");

    backend.db_remove_source(&lib).await;
    let changed = apply_cross_file_evidence_if_current(
        &handles,
        captured_revision,
        vec![(lib.clone(), lib_file, stale_want, false)],
    )
    .await;
    assert!(
        changed.changed.is_empty(),
        "a result from an older Salsa revision must be discarded",
    );
    let db = backend.db.lock().await;
    let tombstones = backend.db_tombstones.lock().await;
    assert!(
        tombstones[&lib].external_call_sites(&*db).is_none(),
        "the stale worker must not restore evidence onto the tombstone",
    );
}

/// Membership, project, and source text must
/// come from one Salsa snapshot. A live orphan publication can change all
/// three while the evidence reader waits for `db`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cross_file_evidence_snapshot_captures_queued_membership_change_1854() {
    let backend = Arc::new(test_backend());
    let main = Uri::from_str("file:///w/membership-main-1854.tcl").unwrap();
    let orphan = Uri::from_str("file:///w/membership-orphan-1854.tcl").unwrap();
    backend
        .db_set_source(
            &main,
            "source membership-orphan-1854.tcl\n",
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();
    backend
        .db_set_source(&orphan, "proc local {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = Analyser::new()
        .analyse("source membership-orphan-1854.tcl\n", "tcl8.6")
        .clone();
    backend
        .workspace_index
        .write()
        .await
        .replace_document(main.as_str(), &analysis);
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

    let database = backend.db.lock().await;
    let retiring = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        let orphan = orphan.clone();
        async move {
            let mut db = backend.db.lock().await;
            let files = backend.db_files.lock().await;
            let mut members = backend.db_project_members.lock().await;
            assert!(members.remove(&orphan));
            let mut project = backend.db_project.lock().await;
            Backend::sync_db_project(&mut db, &files, &members, &mut project);
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        while !backend.db.contention().contains("1 queued waiter") {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the membership writer must queue first");
    let capturing =
        crate::rt::spawn(async move { capture_cross_file_evidence_snapshot(&handles).await });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        while !backend.db.contention().contains("2 queued waiter(s)") {
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the evidence snapshot must queue behind the membership writer");

    drop(database);
    retiring
        .await
        .expect("the membership writer must not panic");
    let captured = crate::rt::timeout(std::time::Duration::from_secs(5), capturing)
        .await
        .expect("the evidence snapshot must finish after the writer")
        .expect("the evidence snapshot task must not panic")
        .expect("the project must remain initialised");
    let covered = files_with_covered_load_targets(
        &captured.snapshot,
        &captured.files,
        &captured.members,
        captured.project,
    );
    assert!(
        !covered.contains(&main),
        "pre-removal coverage must not survive beside the post-removal project",
    );
    assert_eq!(
        captured.project.files(&*captured.snapshot).len(),
        1,
        "the captured Salsa project must contain only the admitted source",
    );
}

/// Coverage membership and the
/// Salsa project must come from one live-publication generation. An orphan
/// edit removes its URI from both, so the evidence snapshot cannot retain
/// the old covered set while observing the new project.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cross_file_evidence_snapshot_waits_for_live_membership_commit_1854() {
    let backend = Arc::new(test_backend());
    let main = Uri::from_str("file:///w/evidence-main-1854.tcl").unwrap();
    let orphan = Uri::from_str("file:///w/evidence-orphan-1854.tcl").unwrap();
    let main_text = "source evidence-orphan-1854.tcl\nproc main_proc {} {}\n";
    backend
        .db_set_source(&main, main_text, "tcl8.6".to_owned())
        .await
        .unwrap();
    backend
        .db_set_source(&orphan, "proc orphan_proc {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    let analysis = Analyser::new().analyse(main_text, "tcl8.6").clone();
    backend
        .workspace_index
        .write()
        .await
        .replace_document(main.as_str(), &analysis);
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

    let publication = backend.live_publication_gate.lock().await;
    let mut db = backend.db.lock().await;
    let syncing = crate::rt::spawn(async move { sync_cross_file_evidence(&handles).await });
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if backend.rehoming_gate.try_lock().is_err() {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the evidence sync must reach the live-publication boundary");
    assert!(
        crate::rt::timeout(std::time::Duration::from_millis(25), async {
            loop {
                if backend.db.contention().contains("1 queued waiter") {
                    break;
                }
                crate::rt::yield_now().await;
            }
        })
        .await
        .is_err(),
        "evidence sync must not capture coverage before the live membership commit",
    );

    let files = backend.db_files.lock().await;
    let mut members = backend.db_project_members.lock().await;
    assert!(members.remove(&orphan));
    let mut project = backend.db_project.lock().await;
    Backend::sync_db_project(&mut db, &files, &members, &mut project);
    drop(project);
    drop(members);
    drop(files);
    drop(db);
    drop(publication);

    crate::rt::timeout(std::time::Duration::from_secs(5), syncing)
        .await
        .expect("evidence sync must finish after the live commit")
        .expect("evidence sync must not panic");
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    assert!(
        files[&main].external_call_sites(&*db).is_none(),
        "a source of the removed member must remain conservatively uncovered",
    );
}
