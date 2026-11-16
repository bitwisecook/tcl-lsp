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

//! The warm of open documents.

use super::*;

/// [`Backend::warm_open_documents`] (the body
/// [`Backend::spawn_workspace_warm`] detaches) primes the deep salsa tier
/// for an **open** document but must not touch an unopened one the scan
/// only fed into `db_files` as a lightweight `SourceFile` input.
#[tokio::test]
async fn warm_open_documents_only_touches_open_files() {
    let executed: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let sink = Arc::clone(&executed);
    let backend = test_backend_over(tcl_lsp_db::TclDatabase::with_event_logger(move |key| {
        sink.lock().unwrap().push(key);
    }));
    let open_uri = Uri::from_str("file:///workspace/open.tcl").unwrap();
    let closed_uri = Uri::from_str("file:///workspace/closed.tcl").unwrap();
    backend.documents.lock("test").await.insert(
        open_uri.clone(),
        DocumentState::new("proc open_proc {} {}\n".to_owned(), "tcl8.6".to_owned()),
    );
    backend
        .db_set_source(&open_uri, "proc open_proc {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();
    // Never opened, but present in `db_files` — exactly the state a
    // workspace scan leaves an unopened file in.
    backend
        .db_set_source(&closed_uri, "proc closed_proc {} {}\n", "tcl8.6".to_owned())
        .await
        .unwrap();

    Backend::warm_open_documents(
        Arc::clone(&backend.db),
        Arc::clone(&backend.db_files),
        Arc::clone(&backend.documents),
        Arc::clone(&backend.db_config),
        Arc::clone(&backend.folder_db_configs),
    )
    .await;

    let deep_runs = executed
        .lock()
        .unwrap()
        .iter()
        .filter(|key| key.contains("file_analysis_incremental"))
        .count();
    assert_eq!(
        deep_runs, 1,
        "the warm must run the deep tier exactly once, for the one open \
             document, never for the unopened one",
    );
}
