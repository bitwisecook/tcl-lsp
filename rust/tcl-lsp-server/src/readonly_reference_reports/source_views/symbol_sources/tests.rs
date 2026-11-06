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

use super::super::super::{
    DocumentSourceResult, ReadonlyTransaction, WorkspaceSymbolRow, WorkspaceSymbolSourceAnalysis,
};
use super::*;
use std::collections::HashMap;
use tower_lsp_server::LanguageServer;
use tower_lsp_server::ls_types::{
    DidOpenTextDocumentParams, Position, Range, SymbolKind, TextDocumentItem, Uri,
    WorkspaceSymbolParams, WorkspaceSymbolResponse,
};

const CHILD_URI: &str = "file:///ws/lib/helpers.tcl";
const PARENT_URI: &str = "file:///ws/main.tcl";
const HELPER: &str = "proc greet_helper {who} {\n    return \"hi $who\"\n}\n";
const MAIN: &str = "source lib/helpers.tcl\n\nproc run {} {\n    puts [greet_helper world]\n    puts [mypkg_autoloaded]\n}\n";

async fn fixture() -> (Backend, Arc<crate::vfs::MemoryStore>) {
    let mut backend = crate::tests::test_backend();
    let store = Arc::new(crate::vfs::MemoryStore::new());
    for (uri, path, source) in [
        (CHILD_URI, "/ws/lib/helpers.tcl", HELPER),
        (PARENT_URI, "/ws/main.tcl", MAIN),
        (
            "file:///ws/tmp/mypkg/tclIndex",
            "/ws/tmp/mypkg/tclIndex",
            "# Tcl autoload index file, version 2.0\nset auto_index(mypkg_autoloaded) [list source [file join $dir impl.tcl]]\n",
        ),
        (
            "file:///ws/tmp/mypkg/impl.tcl",
            "/ws/tmp/mypkg/impl.tcl",
            "proc mypkg_autoloaded {} {\n    return 42\n}\n",
        ),
    ] {
        store.upsert_uri(uri, path, source.as_bytes().to_vec());
    }
    backend.state_mut_for_test().store = store.clone();
    *backend.workspace_folders.lock().await = vec!["file:///ws".parse().unwrap()];
    backend.scan_workspace_folders().await.unwrap();
    (backend, store)
}

struct Captured {
    analyses: HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
    rows: Vec<WorkspaceSymbolRow>,
    views: Vec<Option<WorkspaceSymbolSourceView>>,
    epochs: (u64, u64),
}

impl Captured {
    async fn from_scan(backend: &Backend) -> Self {
        let uri: Uri = CHILD_URI.parse().unwrap();
        let (hit, revision) = {
            let index = backend.workspace_index.read().await;
            let mut hits = index.symbols_matching("greet_helper", 1000);
            assert_eq!(hits.len(), 1, "one actual scanned declaration");
            (hits.pop().unwrap(), index.document_revision(CHILD_URI))
        };
        let snapshot = backend
            .closed_workspace_symbol_source(&uri)
            .await
            .unwrap()
            .unwrap();
        let analysis = backend
            .capture_workspace_symbol_analysis(&uri, &snapshot)
            .await
            .unwrap()
            .unwrap();
        assert!(
            !Backend::workspace_symbol_hit_is_current(
                &hit,
                &snapshot,
                &analysis,
                &HashMap::from([(CHILD_URI.to_owned(), revision.unwrap())]),
            ),
            "standalone analysis must not borrow an inherited declaration site"
        );
        let OriginalSourceViewAttempt::Ready(Some(view)) = backend
            .capture_workspace_symbol_source_view(
                &hit,
                &snapshot,
                &mut WorkspaceSymbolSourceViewCache::default(),
            )
        else {
            panic!("actual scanned fact has exactly one retained interpretation")
        };
        assert!(
            view.scope.sources.len() >= 2,
            "child retains its real source parent"
        );
        let symbol = Backend::workspace_symbol_information(
            hit.name,
            hit.kind,
            hit.container_name,
            hit.name_span,
            &uri,
            &snapshot,
        );
        Self {
            analyses: HashMap::from([(CHILD_URI.to_owned(), Some(analysis))]),
            rows: vec![(symbol, uri, snapshot, revision)],
            views: vec![Some(view)],
            epochs: (
                backend.diag_inputs_epoch(),
                *backend.class_factory_generation.read().await,
            ),
        }
    }

    async fn publish(
        &self,
        backend: &Backend,
    ) -> DocumentSourceResult<ReadonlyTransaction<WorkspaceSymbolResponse>> {
        backend
            .workspace_symbol_response_transaction_with_views(
                &self.analyses,
                &self.rows,
                &self.views,
                self.epochs,
                &crate::WorkspaceSymbolDiagnostic::default(),
            )
            .await
    }
}

#[tokio::test]
async fn first_scanned_closed_symbol_uses_its_actual_inherited_source_view() {
    let (backend, _) = fixture().await;
    // No definition/hover/symbol warmup or diagnostic capture precedes this.
    let first = backend
        .symbol(WorkspaceSymbolParams {
            query: "greet_helper".into(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();
    let Some(WorkspaceSymbolResponse::Flat(symbols)) = first else {
        panic!("first actual public workspace/symbol response must contain the scanned declaration")
    };
    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "greet_helper");
    assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
    assert_eq!(symbols[0].location.uri.as_str(), CHILD_URI);
    assert_eq!(
        symbols[0].location.range,
        Range {
            start: Position::new(0, 5),
            end: Position::new(0, 17),
        }
    );
    let captured = Captured::from_scan(&backend).await;
    assert!(matches!(
        captured.publish(&backend).await.unwrap(),
        ReadonlyTransaction::Published(_)
    ));
}

#[tokio::test]
async fn captured_symbol_leases_parent_and_child_source_bytes_jointly() {
    for parent in [true, false] {
        let (backend, store) = fixture().await;
        let captured = Captured::from_scan(&backend).await;
        assert!(matches!(
            captured.publish(&backend).await.unwrap(),
            ReadonlyTransaction::Published(_)
        ));
        let (uri, path, source) = if parent {
            (PARENT_URI, "/ws/main.tcl", MAIN)
        } else {
            (CHILD_URI, "/ws/lib/helpers.tcl", HELPER)
        };
        store.upsert_uri(
            uri,
            path,
            format!("{source}# changed saved image\n").into_bytes(),
        );
        for admission in captured.views[0].as_ref().unwrap().admissions() {
            admission.check_current(store.as_ref()).unwrap();
        }
        assert!(
            matches!(
                captured.publish(&backend).await,
                Err(crate::DocumentSourceUnavailable::Store {
                    cause: crate::vfs::SourceUriPathUnavailable::SourceImageUnavailable,
                    ..
                })
            ),
            "same binding cannot renew the captured child or parent bytes"
        );
    }
}

#[tokio::test]
async fn parent_byte_only_change_before_capture_cannot_renew_an_interpretation() {
    let (backend, store) = fixture().await;
    let before = Captured::from_scan(&backend).await;
    assert!(matches!(
        before.publish(&backend).await.unwrap(),
        ReadonlyTransaction::Published(_)
    ));
    let changed = MAIN.replace('\n', "\r");
    assert_ne!(changed.as_bytes(), MAIN.as_bytes());
    assert_eq!(tcl_lexer::normalise_lone_cr(&changed).as_ref(), MAIN);
    store.upsert_uri(PARENT_URI, "/ws/main.tcl", changed.into_bytes());
    for admission in before.views[0].as_ref().unwrap().admissions() {
        admission.check_current(store.as_ref()).unwrap();
    }
    // This is a fresh capture after the mutation, not just an older query's
    // saved-image lease. The actual producer must retain its original bytes.
    let after = Captured::from_scan(&backend).await;
    assert!(matches!(
        after.publish(&backend).await,
        Err(crate::DocumentSourceUnavailable::Store {
            cause: crate::vfs::SourceUriPathUnavailable::SourceImageUnavailable,
            ..
        })
    ));
}

#[tokio::test]
async fn exact_physical_fact_does_not_require_an_unrelated_view_table_lease() {
    let (backend, store) = fixture().await;
    let uri: Uri = "file:///ws/other.tcl".parse().unwrap();
    store.upsert_uri(
        uri.as_str(),
        "/ws/other.tcl",
        b"proc greet_other {} {return OTHER}\n".to_vec(),
    );
    backend.scan_workspace_folders().await.unwrap();
    let (hit, revision) = {
        let index = backend.workspace_index.read().await;
        (
            index.symbols_matching("greet_other", 1000).pop().unwrap(),
            index.document_revision(uri.as_str()).unwrap(),
        )
    };
    let snapshot = backend
        .closed_workspace_symbol_source(&uri)
        .await
        .unwrap()
        .unwrap();
    let analysis = backend
        .capture_workspace_symbol_analysis(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    let revisions = HashMap::from([(uri.as_str().to_owned(), revision)]);
    assert!(Backend::workspace_symbol_hit_is_current(
        &hit, &snapshot, &analysis, &revisions
    ));
    let _unrelated_table = backend.original_source_views.lock().unwrap();
    assert!(matches!(
        backend.workspace_symbol_hit_source_view(
            &hit,
            &snapshot,
            &analysis,
            &revisions,
            &mut WorkspaceSymbolSourceViewCache::default(),
            &crate::WorkspaceSymbolDiagnostic::default(),
        ),
        OriginalSourceViewAttempt::Ready(None)
    ));
}

#[tokio::test]
async fn retired_interpretation_keeps_an_independent_current_physical_row() {
    let (backend, store) = fixture().await;
    let other: Uri = "file:///ws/other.tcl".parse().unwrap();
    store.upsert_uri(
        other.as_str(),
        "/ws/other.tcl",
        b"proc greet_other {} {return OTHER}\n".to_vec(),
    );
    backend.scan_workspace_folders().await.unwrap();
    let mut captured = Captured::from_scan(&backend).await;
    let (hit, revision) = {
        let index = backend.workspace_index.read().await;
        (
            index.symbols_matching("greet_other", 1000).pop().unwrap(),
            index.document_revision(other.as_str()).unwrap(),
        )
    };
    let snapshot = backend
        .closed_workspace_symbol_source(&other)
        .await
        .unwrap()
        .unwrap();
    let analysis = backend
        .capture_workspace_symbol_analysis(&other, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(Backend::workspace_symbol_hit_is_current(
        &hit,
        &snapshot,
        &analysis,
        &HashMap::from([(other.as_str().to_owned(), revision)])
    ));
    let symbol = Backend::workspace_symbol_information(
        hit.name,
        hit.kind,
        hit.container_name,
        hit.name_span,
        &other,
        &snapshot,
    );
    captured
        .analyses
        .insert(other.as_str().to_owned(), Some(analysis));
    captured
        .rows
        .push((symbol, other.clone(), snapshot, Some(revision)));
    captured.views.push(None);
    let ReadonlyTransaction::Published(WorkspaceSymbolResponse::Flat(before)) =
        captured.publish(&backend).await.unwrap()
    else {
        panic!("both genuine current rows must publish")
    };
    assert_eq!(before.len(), 2);
    backend.invalidate_live_publication([&PARENT_URI.parse().unwrap()]);
    let ReadonlyTransaction::Published(WorkspaceSymbolResponse::Flat(after)) =
        captured.publish(&backend).await.unwrap()
    else {
        panic!("retired child interpretation cannot remove the independent physical row")
    };
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].name, "greet_other");
    assert_eq!(after[0].location.uri, other);
}

#[tokio::test]
async fn captured_symbol_refuses_changed_parent_publication() {
    let (backend, _) = fixture().await;
    let captured = Captured::from_scan(&backend).await;
    assert!(matches!(
        captured.publish(&backend).await.unwrap(),
        ReadonlyTransaction::Published(_)
    ));
    backend
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(
                PARENT_URI.parse().unwrap(),
                "tcl".into(),
                1,
                MAIN.into(),
            ),
        })
        .await;
    assert!(matches!(
        captured.publish(&backend).await.unwrap(),
        ReadonlyTransaction::CurrencyRefused
    ));
}

#[tokio::test]
async fn retired_ancestor_cannot_supply_a_captured_symbol_interpretation() {
    let (backend, _) = fixture().await;
    let captured = Captured::from_scan(&backend).await;
    assert!(matches!(
        captured.publish(&backend).await.unwrap(),
        ReadonlyTransaction::Published(_)
    ));
    backend.invalidate_live_publication([&PARENT_URI.parse().unwrap()]);
    assert!(matches!(
        captured.publish(&backend).await.unwrap(),
        ReadonlyTransaction::CurrencyRefused
    ));
    let hit = &captured.views[0].as_ref().unwrap().hit;
    assert!(matches!(
        backend.capture_workspace_symbol_source_view(
            hit,
            &captured.rows[0].2,
            &mut WorkspaceSymbolSourceViewCache::default()
        ),
        OriginalSourceViewAttempt::Unavailable
    ));
}

#[tokio::test]
async fn retired_index_cannot_reconstruct_a_captured_symbol_interpretation() {
    let (backend, _) = fixture().await;
    let captured = Captured::from_scan(&backend).await;
    assert!(matches!(
        captured.publish(&backend).await.unwrap(),
        ReadonlyTransaction::Published(_)
    ));
    backend
        .workspace_index
        .write()
        .await
        .remove_document(CHILD_URI);
    assert!(matches!(
        captured.publish(&backend).await.unwrap(),
        ReadonlyTransaction::CurrencyRefused
    ));
    let hit = &captured.views[0].as_ref().unwrap().hit;
    let analysis = captured.analyses[CHILD_URI].as_ref().unwrap();
    assert!(matches!(
        backend.workspace_symbol_hit_source_view(
            hit,
            &captured.rows[0].2,
            analysis,
            &HashMap::from([(CHILD_URI.to_owned(), captured.rows[0].3.unwrap())]),
            &mut WorkspaceSymbolSourceViewCache::default(),
            &crate::WorkspaceSymbolDiagnostic::default(),
        ),
        OriginalSourceViewAttempt::Unavailable
    ));
}
