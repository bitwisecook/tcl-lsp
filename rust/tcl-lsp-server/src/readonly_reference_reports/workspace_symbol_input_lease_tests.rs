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
use crate::DocumentState;
use std::collections::HashMap;
use tower_lsp_server::LanguageServer;
use tower_lsp_server::ls_types::{
    SymbolInformation, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};

const SOURCE: &str = "proc first_symbol {} {}\nproc second_symbol {} {}\n";

async fn register(backend: &Backend, uri: &Uri, database: bool) {
    backend
        .documents
        .lock("workspace symbol input test")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(SOURCE.to_owned(), "tcl8.6".to_owned()),
        );
    if database {
        backend
            .db_set_source(uri, SOURCE, "tcl8.6".to_owned())
            .await
            .unwrap();
    }
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(SOURCE, "tcl8.6");
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
}

async fn symbols(backend: &Backend, query: &str) -> Vec<SymbolInformation> {
    let response = backend
        .symbol(WorkspaceSymbolParams {
            query: query.to_owned(),
            work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(
            ),
            partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
        })
        .await
        .unwrap();
    let Some(WorkspaceSymbolResponse::Flat(symbols)) = response else {
        panic!("expected genuine current symbols, got {response:?}");
    };
    symbols
}

struct Prepared {
    analyses: HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
    rows: Vec<(SymbolInformation, Uri, DocumentState, Option<u64>)>,
    inputs_epoch: u64,
    factory_generation: u64,
}

async fn prepare(backend: &Backend, uri: &Uri) -> Prepared {
    // Presentation rows come from the actual guarded endpoint, not from
    // fabricated names, geometry or declaration/Native identities.
    let found = symbols(backend, "symbol").await;
    assert_eq!(found.len(), 2);
    let inputs_epoch = backend.diag_inputs_epoch();
    let factory_generation = *backend.class_factory_generation.read().await;
    let snapshot = backend
        .documents
        .lock("workspace symbol proof source")
        .await
        .get(uri)
        .unwrap()
        .clone()
        .normalised_for_analysis();
    let analysis = backend
        .capture_workspace_symbol_analysis(uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    let revision = {
        let _documents = backend.documents.lock("workspace symbol proof index").await;
        backend
            .workspace_index
            .read()
            .await
            .document_revision(uri.as_str())
    };
    let rows = found
        .into_iter()
        .map(|symbol| (symbol, uri.clone(), snapshot.clone(), revision))
        .collect();
    Prepared {
        analyses: HashMap::from([(uri.as_str().to_owned(), Some(analysis))]),
        rows,
        inputs_epoch,
        factory_generation,
    }
}

async fn finish(backend: &Backend, prepared: &Prepared) -> Option<WorkspaceSymbolResponse> {
    backend
        .finish_workspace_symbol_response(
            &prepared.analyses,
            &prepared.rows,
            prepared.inputs_epoch,
            prepared.factory_generation,
        )
        .await
        .unwrap()
}

async fn transaction(
    backend: &Backend,
    prepared: &Prepared,
) -> ReadonlyTransaction<WorkspaceSymbolResponse> {
    backend
        .workspace_symbol_response_transaction(
            &prepared.analyses,
            &prepared.rows,
            prepared.inputs_epoch,
            prepared.factory_generation,
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn workspace_symbol_index_contention_recaptures_the_whole_query() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/workspace-symbol-retry-index.tcl"
        .parse()
        .unwrap();
    register(&backend, &uri, false).await;
    let prepared = prepare(&backend, &uri).await;
    let held = std::cell::RefCell::new(Some(backend.workspace_index.write().await));
    assert!(finish(&backend, &prepared).await.is_none());
    let attempts = std::cell::Cell::new(0);
    let response = retry_readonly_attempt(None, || async {
        let attempt = attempts.get();
        attempts.set(attempt + 1);
        if attempt == 0 {
            let refused = transaction(&backend, &prepared).await;
            assert!(matches!(refused, ReadonlyTransaction::CurrencyRefused));
            drop(held.borrow_mut().take());
            Ok::<_, super::super::DocumentSourceUnavailable>(refused.into_attempt())
        } else {
            backend.workspace_symbol_attempt("symbol").await
        }
    })
    .await
    .unwrap();
    assert_eq!(attempts.get(), 2);
    let Some(WorkspaceSymbolResponse::Flat(rows)) = response else {
        panic!("the recaptured query publishes current source symbols");
    };
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.location.uri == uri));
}

#[tokio::test]
async fn workspace_symbol_currency_withdrawal_recaptures_current_source() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/workspace-symbol-retry-source.tcl"
        .parse()
        .unwrap();
    register(&backend, &uri, false).await;
    let prepared = prepare(&backend, &uri).await;
    backend
        .documents
        .lock("replace symbol retry source")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(
                "proc current_symbol {} {}\n".to_owned(),
                "tcl8.6".to_owned(),
            )
            .with_publication_pending(),
        );
    backend
        .workspace_index
        .write()
        .await
        .remove_document(uri.as_str());
    let attempts = std::cell::Cell::new(0);
    let response = retry_readonly_attempt(None, || async {
        let attempt = attempts.get();
        attempts.set(attempt + 1);
        if attempt == 0 {
            let refused = transaction(&backend, &prepared).await;
            assert!(matches!(refused, ReadonlyTransaction::CurrencyRefused));
            Ok::<_, super::super::DocumentSourceUnavailable>(refused.into_attempt())
        } else {
            backend.workspace_symbol_attempt("symbol").await
        }
    })
    .await
    .unwrap();
    assert_eq!(attempts.get(), 2);
    let Some(WorkspaceSymbolResponse::Flat(rows)) = response else {
        panic!("the query must recapture its actual replacement source");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "current_symbol");
    assert_eq!(rows[0].location.uri, uri);
    assert!(matches!(
        transaction(&backend, &prepared).await,
        ReadonlyTransaction::CurrencyRefused
    ));
}

#[tokio::test]
async fn workspace_symbol_persistent_index_contention_exhausts_existing_budget() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/workspace-symbol-retry-exhausted.tcl"
        .parse()
        .unwrap();
    register(&backend, &uri, false).await;
    let prepared = prepare(&backend, &uri).await;
    let held = backend.workspace_index.write().await;
    let attempts = std::cell::Cell::new(0);
    let response = retry_readonly_attempt(None, || async {
        attempts.set(attempts.get() + 1);
        let refused = transaction(&backend, &prepared).await;
        assert!(matches!(refused, ReadonlyTransaction::CurrencyRefused));
        Ok::<_, super::super::DocumentSourceUnavailable>(refused.into_attempt())
    })
    .await
    .unwrap();
    assert!(response.is_none());
    assert_eq!(attempts.get(), READONLY_CURRENCY_ATTEMPTS);
    assert!(backend.workspace_index.try_read().is_err());
    drop(held);
    assert!(transaction(&backend, &prepared).await.published().is_some());
}

#[tokio::test]
async fn workspace_symbol_missing_indexed_fact_remains_terminal() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/workspace-symbol-retry-fact.tcl"
        .parse()
        .unwrap();
    register(&backend, &uri, false).await;
    backend
        .documents
        .lock("withdraw indexed symbol fact")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(
                SOURCE.replace("first_symbol", "replacement_symbol"),
                "tcl8.6".to_owned(),
            ),
        );
    assert_eq!(
        backend
            .workspace_index
            .read()
            .await
            .symbols_matching("first_symbol", 10)
            .len(),
        1,
        "the old indexed fact remains present"
    );
    let attempts = std::cell::Cell::new(0);
    let response = retry_readonly_attempt(None, || async {
        attempts.set(attempts.get() + 1);
        let attempt = backend.workspace_symbol_attempt("first_symbol").await?;
        assert!(matches!(attempt, ControlFlow::Break(None)));
        Ok::<_, super::super::DocumentSourceUnavailable>(attempt)
    })
    .await
    .unwrap();
    assert!(response.is_none());
    assert_eq!(attempts.get(), 1);
}

#[tokio::test]
async fn workspace_symbol_terminal_snapshot_row_keeps_other_current_rows() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/workspace-symbol-retry-partial.tcl"
        .parse()
        .unwrap();
    register(&backend, &uri, false).await;
    let mut prepared = prepare(&backend, &uri).await;
    let mismatched = DocumentState::new("proc unrelated {} {}\n".to_owned(), "tcl8.6".to_owned());
    prepared.rows[0].2 = mismatched.clone();
    let ReadonlyTransaction::Published(WorkspaceSymbolResponse::Flat(rows)) =
        transaction(&backend, &prepared).await
    else {
        panic!("the independent current row remains available");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, prepared.rows[1].0.name);
    prepared.rows[1].2 = mismatched;
    assert!(matches!(
        transaction(&backend, &prepared).await,
        ReadonlyTransaction::PublishRefused
    ));
}

#[tokio::test]
async fn workspace_symbol_missing_source_analysis_remains_terminal() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/workspace-symbol-retry-terminal.tcl"
        .parse()
        .unwrap();
    register(&backend, &uri, false).await;
    let mut prepared = prepare(&backend, &uri).await;
    prepared.analyses.insert(uri.as_str().to_owned(), None);
    let attempts = std::cell::Cell::new(0);
    let response = retry_readonly_attempt(None, || async {
        attempts.set(attempts.get() + 1);
        let refused = transaction(&backend, &prepared).await;
        assert!(matches!(refused, ReadonlyTransaction::PublishRefused));
        Ok::<_, super::super::DocumentSourceUnavailable>(refused.into_attempt())
    })
    .await
    .unwrap();
    assert!(response.is_none());
    assert_eq!(attempts.get(), 1);
}

#[tokio::test]
async fn native_indexed_and_pending_pathless_symbols_keep_genuine_no_database_inputs() {
    let backend = crate::tests::test_backend();
    let native: Uri = "file:///tmp/workspace-symbol-native-no-db.tcl"
        .parse()
        .unwrap();
    register(&backend, &native, false).await;
    assert!(!backend.db_files.lock().await.contains_key(&native));
    let native_rows = symbols(&backend, "symbol").await;
    assert_eq!(native_rows.len(), 2);
    assert!(
        native_rows
            .iter()
            .all(|symbol| symbol.location.uri == native)
    );

    let pending: Uri = "untitled:workspace-symbol-unicode-pending".parse().unwrap();
    backend
        .documents
        .lock("workspace symbol pending test")
        .await
        .insert(
            pending.clone(),
            DocumentState::with_version(
                "proc 茶 {} {}\rproc 雪 {} {}\r".to_owned(),
                "tcl8.6".to_owned(),
                1,
            )
            .with_publication_pending(),
        );
    assert!(!backend.db_files.lock().await.contains_key(&pending));
    let pending_rows = symbols(&backend, "雪").await;
    assert_eq!(pending_rows.len(), 1);
    assert_eq!(pending_rows[0].location.uri, pending);
    assert_eq!(pending_rows[0].location.range.start, Position::new(1, 5));
    assert_eq!(pending_rows[0].location.range.end, Position::new(1, 6));
}

#[tokio::test]
async fn fresh_symbol_analysis_cannot_survive_equal_source_configuration_mutation() {
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/workspace-symbol-fresh-config.tcl"
        .parse()
        .unwrap();
    register(&backend, &uri, false).await;
    let prepared = prepare(&backend, &uri).await;
    assert!(matches!(
        &prepared.analyses[uri.as_str()].as_ref().unwrap().kind,
        WorkspaceSymbolAnalysisKind::Fresh(_)
    ));
    assert!(finish(&backend, &prepared).await.is_some());
    let revision = backend
        .workspace_index
        .read()
        .await
        .document_revision(uri.as_str());
    let config = backend.resolved_db_config(&uri).await;
    {
        let mut db = backend.db.lock().await;
        config
            .set_extra_commands(&mut *db)
            .to(vec!["new_configuration_command".to_owned()]);
    }
    assert_eq!(
        backend.diag_inputs_epoch(),
        prepared.inputs_epoch,
        "isolate actual full configuration/DB currency from the epoch guard"
    );
    assert_eq!(
        backend
            .workspace_index
            .read()
            .await
            .document_revision(uri.as_str()),
        revision
    );
    assert_eq!(
        backend
            .documents
            .lock("workspace symbol unchanged source")
            .await[&uri]
            .text
            .as_ref(),
        SOURCE
    );
    assert!(finish(&backend, &prepared).await.is_none());
}

#[tokio::test]
async fn database_symbol_analysis_keeps_its_actual_full_input_and_publication_owner() {
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/workspace-symbol-database-config.tcl"
        .parse()
        .unwrap();
    register(&backend, &uri, true).await;
    let prepared = prepare(&backend, &uri).await;
    assert!(matches!(
        &prepared.analyses[uri.as_str()].as_ref().unwrap().kind,
        WorkspaceSymbolAnalysisKind::Database(_)
    ));
    assert!(finish(&backend, &prepared).await.is_some());
    let config = backend.resolved_db_config(&uri).await;
    {
        let mut db = backend.db.lock().await;
        config
            .set_extra_commands(&mut *db)
            .to(vec!["database_configuration_command".to_owned()]);
    }
    assert!(finish(&backend, &prepared).await.is_none());
}

#[tokio::test]
async fn symbol_final_joint_lease_retains_real_index_input_epoch_and_open_source_guards() {
    let backend = crate::tests::test_backend();
    let uri: Uri = "file:///tmp/workspace-symbol-joint-lease.tcl"
        .parse()
        .unwrap();
    register(&backend, &uri, false).await;
    let prepared = prepare(&backend, &uri).await;
    assert!(finish(&backend, &prepared).await.is_some());
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(SOURCE, "tcl8.6");
    backend
        .workspace_index
        .write()
        .await
        .replace_document(uri.as_str(), &analysis);
    assert!(
        finish(&backend, &prepared).await.is_none(),
        "equal source reindex cannot renew this request's captured index revision"
    );

    let prepared = prepare(&backend, &uri).await;
    backend.invalidate_diag_inputs();
    assert!(finish(&backend, &prepared).await.is_none());

    let prepared = prepare(&backend, &uri).await;
    backend
        .documents
        .lock("workspace symbol real source replacement")
        .await
        .insert(
            uri.clone(),
            DocumentState::new("proc other {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );
    assert!(finish(&backend, &prepared).await.is_none());
}
