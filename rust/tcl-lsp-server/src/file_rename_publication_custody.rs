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

use super::*;
use std::future::Future;
use std::task::Poll;
use std::time::Duration;

struct Fixture {
    backend: Backend,
    source: FileRenameSourcePublication,
    inputs: FileRenameInputSnapshot,
    generation: u64,
    index_revision: Option<u64>,
}

impl Fixture {
    async fn new() -> Self {
        let backend = crate::tests::test_backend();
        let uri: Uri = "file:///rename-publication/main.tcl".parse().unwrap();
        let text = "proc current {} {}\nsource lib/old.tcl\n";
        backend
            .documents
            .lock("rename publication fixture")
            .await
            .insert(
                uri.clone(),
                DocumentState::with_version(text.to_owned(), "tcl8.6".to_owned(), 3),
            );
        backend
            .db_set_source(&uri, text, "tcl8.6".to_owned())
            .await
            .unwrap();
        let inputs = backend.file_rename_input_snapshot().await.unwrap();
        let document = backend.read_document(&uri).await.unwrap().unwrap();
        let analysis = backend
            .analysis_for(&uri, Arc::clone(&document.text), document.dialect.clone())
            .await
            .unwrap();
        let (generation, index_revision) = {
            let mut index = backend.workspace_index.write().await;
            index.replace_document(uri.as_str(), &analysis);
            (index.generation(), index.document_revision(uri.as_str()))
        };
        Self {
            backend,
            source: FileRenameSourcePublication {
                uri,
                document,
                analysis,
            },
            inputs,
            generation,
            index_revision,
        }
    }

    fn source(&self) -> FileRenameSourcePublication {
        FileRenameSourcePublication {
            uri: self.source.uri.clone(),
            document: self.source.document.clone(),
            analysis: Arc::clone(&self.source.analysis),
        }
    }

    async fn held_edits(&self) -> Vec<HeldFileRename> {
        let old = PathBuf::from("/rename-publication/lib/old.tcl");
        let new = PathBuf::from("/rename-publication/lib/new.tcl");
        let action = core_file_ops::FileRenameSourceAction::new(&old, &new, &[]).unwrap();
        let edits = {
            let index = self.backend.workspace_index.read().await;
            core_file_ops::compute_rename_edits(&action, &index, &|owner| {
                uri_norm::file_action_path(&owner.parse::<Uri>().ok()?)
            })
        };
        assert!(
            !edits.is_empty(),
            "the real source action must supply edit owners"
        );
        let mut sources = FileRenameSources::from([(
            self.source.uri.clone(),
            (
                self.source.document.clone(),
                Arc::clone(&self.source.analysis),
            ),
        )]);
        self.backend
            .hold_file_rename_edits(
                HashMap::from([(self.source.uri.clone(), edits)]),
                &mut sources,
            )
            .await
            .unwrap()
            .expect("the source action must retain actual source/index joins")
    }

    async fn revision(&self) -> Option<u64> {
        self.backend
            .workspace_index
            .read()
            .await
            .document_revision(self.source.uri.as_str())
    }

    async fn publish(
        &self,
        sources: &mut FileRenameSources,
        generation: &mut u64,
    ) -> ControlFlow<()> {
        crate::rt::timeout(
            Duration::from_secs(3),
            self.backend.publish_file_rename_source(
                self.source(),
                sources,
                generation,
                &self.inputs,
            ),
        )
        .await
        .unwrap()
        .unwrap()
    }
}

async fn poll_once<F: Future>(future: std::pin::Pin<&mut F>) -> Poll<F::Output> {
    let mut future = future;
    std::future::poll_fn(|context| Poll::Ready(future.as_mut().poll(context))).await
}

#[tokio::test]
async fn current_source_publication_updates_the_actual_index_and_retains_its_owner() {
    let fixture = Fixture::new().await;
    let mut sources = FileRenameSources::new();
    let mut generation = fixture.generation;
    assert!(
        fixture
            .publish(&mut sources, &mut generation)
            .await
            .is_continue()
    );
    assert_ne!(fixture.revision().await, fixture.index_revision);
    assert_ne!(generation, fixture.generation);
    let (document, analysis) = &sources[&fixture.source.uri];
    assert_eq!(document.version, Some(3));
    assert!(Arc::ptr_eq(analysis, &fixture.source.analysis));
}

#[tokio::test]
async fn queued_input_writer_blocks_old_source_mutation_before_its_epoch_bump() {
    let fixture = Fixture::new().await;
    let writer = fixture.backend.analyser_inputs_gate.write().await;
    *fixture.backend.non_ascii_mode.lock().await = NonAsciiMode::Strict;
    let mut sources = FileRenameSources::new();
    let mut generation = fixture.generation;
    let mut publication = Box::pin(fixture.backend.publish_file_rename_source(
        fixture.source(),
        &mut sources,
        &mut generation,
        &fixture.inputs,
    ));
    let first = poll_once(publication.as_mut()).await;
    // This checks the actual index before inspecting the future's readiness.
    assert_eq!(fixture.revision().await, fixture.index_revision);
    assert!(first.is_pending());
    fixture.backend.invalidate_diag_inputs();
    drop(writer);
    assert!(
        crate::rt::timeout(Duration::from_secs(3), publication)
            .await
            .unwrap()
            .unwrap()
            .is_break()
    );
    assert_eq!(fixture.revision().await, fixture.index_revision);
    assert_eq!(generation, fixture.generation);
    assert!(sources.is_empty());
}

#[tokio::test]
async fn captured_sidecar_identity_cannot_revive_after_equal_epoch_recovery() {
    let fixture = Fixture::new().await;
    let epoch = fixture.backend.diag_inputs_epoch();
    let new_revision = {
        let _writer = fixture.backend.analyser_inputs_gate.write().await;
        let mut cache = fixture.backend.fresh_analysis_cache.lock().unwrap();
        cache.clear();
        cache.revision()
    };
    let mut sources = FileRenameSources::new();
    let mut generation = fixture.generation;
    assert!(
        fixture
            .publish(&mut sources, &mut generation)
            .await
            .is_break()
    );
    assert_eq!(fixture.revision().await, fixture.index_revision);
    {
        let _writer = fixture.backend.analyser_inputs_gate.write().await;
        fixture
            .backend
            .fresh_analysis_cache
            .lock()
            .unwrap()
            .complete_sidecar_invalidation(&new_revision);
    }
    assert_eq!(fixture.backend.diag_inputs_epoch(), epoch);
    assert!(fixture.backend.file_rename_input_snapshot().await.is_some());
    assert!(
        fixture
            .publish(&mut sources, &mut generation)
            .await
            .is_break()
    );
    assert_eq!(fixture.revision().await, fixture.index_revision);
    assert!(sources.is_empty());
}

#[tokio::test]
async fn changed_factory_generation_refuses_old_source_without_an_input_epoch_change() {
    let fixture = Fixture::new().await;
    *fixture.backend.class_factory_generation.write().await += 1;
    assert_eq!(
        fixture.backend.diag_inputs_epoch(),
        fixture.inputs.analyser_inputs_epoch
    );
    let mut sources = FileRenameSources::new();
    let mut generation = fixture.generation;
    assert!(
        fixture
            .publish(&mut sources, &mut generation)
            .await
            .is_break()
    );
    assert_eq!(fixture.revision().await, fixture.index_revision);
    assert!(sources.is_empty());
}

#[tokio::test]
async fn pending_sidecar_has_no_capturable_file_rename_input() {
    let fixture = Fixture::new().await;
    let revision = {
        let _writer = fixture.backend.analyser_inputs_gate.write().await;
        let mut cache = fixture.backend.fresh_analysis_cache.lock().unwrap();
        cache.clear();
        cache.revision()
    };
    assert!(fixture.backend.file_rename_input_snapshot().await.is_none());
    {
        let _writer = fixture.backend.analyser_inputs_gate.write().await;
        fixture
            .backend
            .fresh_analysis_cache
            .lock()
            .unwrap()
            .complete_sidecar_invalidation(&revision);
    }
    assert!(fixture.backend.file_rename_input_snapshot().await.is_some());
}

#[tokio::test]
async fn contended_source_publication_releases_other_stores_and_rechecks_original_inputs() {
    let fixture = Fixture::new().await;
    let index = fixture.backend.workspace_index.read().await;
    let mut sources = FileRenameSources::new();
    let mut generation = fixture.generation;
    let mut publication = Box::pin(fixture.backend.publish_file_rename_source(
        fixture.source(),
        &mut sources,
        &mut generation,
        &fixture.inputs,
    ));
    assert!(poll_once(publication.as_mut()).await.is_pending());
    drop(
        fixture
            .backend
            .documents
            .try_lock("source contention control")
            .expect("index wait must release documents"),
    );
    drop(
        fixture
            .backend
            .analyser_inputs_gate
            .try_write()
            .expect("index wait must release input reader"),
    );
    drop(
        fixture
            .backend
            .class_factory_generation
            .try_write()
            .expect("index wait must release factory reader"),
    );
    let document = crate::rt::timeout(
        Duration::from_secs(3),
        fixture.backend.read_document(&fixture.source.uri),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    assert_eq!(document.version, Some(3));
    {
        let _writer = fixture.backend.analyser_inputs_gate.write().await;
        fixture.backend.invalidate_diag_inputs();
    }
    assert_eq!(
        index.document_revision(fixture.source.uri.as_str()),
        fixture.index_revision
    );
    drop(index);
    assert!(
        crate::rt::timeout(Duration::from_secs(3), publication)
            .await
            .unwrap()
            .unwrap()
            .is_break()
    );
    assert_eq!(fixture.revision().await, fixture.index_revision);
    assert_eq!(generation, fixture.generation);
    assert!(sources.is_empty());
}

#[tokio::test]
async fn superseded_open_document_still_refuses_the_captured_source() {
    let fixture = Fixture::new().await;
    fixture
        .backend
        .documents
        .lock("rename publication superseded")
        .await
        .insert(
            fixture.source.uri.clone(),
            DocumentState::with_version("proc newer {} {}\n".to_owned(), "tcl8.6".to_owned(), 4),
        );
    let mut sources = FileRenameSources::new();
    let mut generation = fixture.generation;
    assert!(
        fixture
            .publish(&mut sources, &mut generation)
            .await
            .is_break()
    );
    assert_eq!(fixture.revision().await, fixture.index_revision);
    assert!(sources.is_empty());
}

#[tokio::test]
async fn newer_index_generation_is_not_rebased_onto_the_old_source() {
    let fixture = Fixture::new().await;
    fixture
        .backend
        .workspace_index
        .write()
        .await
        .replace_document(
            "file:///rename-publication/other.tcl",
            &fixture.source.analysis,
        );
    let mut sources = FileRenameSources::new();
    let mut generation = fixture.generation;
    assert!(
        fixture
            .publish(&mut sources, &mut generation)
            .await
            .is_break()
    );
    assert_eq!(fixture.revision().await, fixture.index_revision);
    assert_eq!(generation, fixture.generation);
    assert!(sources.is_empty());
}

#[tokio::test]
async fn cancelled_real_sidecar_invalidation_refuses_capture_until_database_recovery() {
    let fixture = Fixture::new().await;
    let database = fixture.backend.db.lock().await;
    let mut invalidation = Box::pin(fixture.backend.invalidate_sidecar_stubs());
    assert!(poll_once(invalidation.as_mut()).await.is_pending());
    assert_ne!(
        fixture.backend.diag_inputs_epoch(),
        fixture.inputs.analyser_inputs_epoch
    );
    drop(invalidation);
    drop(database);
    assert!(fixture.backend.file_rename_input_snapshot().await.is_none());
    let mut sources = FileRenameSources::new();
    let mut generation = fixture.generation;
    assert!(
        fixture
            .publish(&mut sources, &mut generation)
            .await
            .is_break()
    );
    assert_eq!(fixture.revision().await, fixture.index_revision);
    fixture.backend.invalidate_sidecar_stubs().await;
    assert!(fixture.backend.file_rename_input_snapshot().await.is_some());
    assert!(
        fixture
            .publish(&mut sources, &mut generation)
            .await
            .is_break()
    );
    assert!(sources.is_empty());
}

#[tokio::test]
async fn current_final_edit_retains_real_source_edits_and_client_version() {
    let fixture = Fixture::new().await;
    let held = fixture.held_edits().await;
    let edit = fixture
        .backend
        .file_rename_edit_if_current(held, fixture.generation, &fixture.inputs)
        .await
        .unwrap();
    let Some(DocumentChanges::Edits(edits)) = edit.document_changes else {
        panic!("versioned source edits");
    };
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].text_document.version, Some(3));
    assert_ne!(
        edits[0].edits,
        [] as [tower_lsp_server::ls_types::OneOf<
            TextEdit,
            tower_lsp_server::ls_types::AnnotatedTextEdit,
        >; 0]
    );
}

#[tokio::test]
async fn final_edit_refuses_retired_sidecar_after_the_source_index_join_was_captured() {
    let fixture = Fixture::new().await;
    let held = fixture.held_edits().await;
    {
        let _writer = fixture.backend.analyser_inputs_gate.write().await;
        let mut cache = fixture.backend.fresh_analysis_cache.lock().unwrap();
        cache.clear();
        let current = cache.revision();
        cache.complete_sidecar_invalidation(&current);
    }
    assert_eq!(
        fixture.backend.diag_inputs_epoch(),
        fixture.inputs.analyser_inputs_epoch
    );
    assert!(
        fixture
            .backend
            .file_rename_edit_if_current(held, fixture.generation, &fixture.inputs)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn queued_input_writer_blocks_final_edit_before_its_epoch_bump() {
    let fixture = Fixture::new().await;
    let held = fixture.held_edits().await;
    let writer = fixture.backend.analyser_inputs_gate.write().await;
    let mut publication = Box::pin(fixture.backend.file_rename_edit_if_current(
        held,
        fixture.generation,
        &fixture.inputs,
    ));
    let first = poll_once(publication.as_mut()).await;
    assert!(
        first.is_pending(),
        "no WorkspaceEdit may escape an active input writer"
    );
    fixture.backend.invalidate_diag_inputs();
    drop(writer);
    assert!(
        crate::rt::timeout(Duration::from_secs(3), publication)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn contended_final_edit_releases_other_stores_and_rechecks_original_inputs() {
    let fixture = Fixture::new().await;
    let held = fixture.held_edits().await;
    let index = fixture.backend.workspace_index.write().await;
    let mut publication = Box::pin(fixture.backend.file_rename_edit_if_current(
        held,
        fixture.generation,
        &fixture.inputs,
    ));
    assert!(poll_once(publication.as_mut()).await.is_pending());
    drop(
        fixture
            .backend
            .documents
            .try_lock("edit contention control")
            .expect("index wait must release documents"),
    );
    drop(
        fixture
            .backend
            .analyser_inputs_gate
            .try_write()
            .expect("index wait must release input reader"),
    );
    drop(
        fixture
            .backend
            .class_factory_generation
            .try_write()
            .expect("index wait must release factory reader"),
    );
    let document = crate::rt::timeout(
        Duration::from_secs(3),
        fixture.backend.read_document(&fixture.source.uri),
    )
    .await
    .unwrap()
    .unwrap()
    .unwrap();
    assert_eq!(document.version, Some(3));
    {
        let _writer = fixture.backend.analyser_inputs_gate.write().await;
        fixture.backend.invalidate_diag_inputs();
    }
    drop(index);
    assert!(
        crate::rt::timeout(Duration::from_secs(3), publication)
            .await
            .unwrap()
            .is_none()
    );
}
