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

#[tokio::test]
async fn clones_share_actual_inputs_and_state_releases_after_the_final_owner() {
    let mut backend = tests::test_backend();
    backend.state_mut_for_test().store = Arc::new(vfs::MemoryStore::new());
    let weak = Arc::downgrade(&backend.state);
    let clone = backend.clone();
    assert!(Arc::ptr_eq(&backend.state, &clone.state));
    assert!(Arc::ptr_eq(&backend.db, &clone.db));
    assert!(Arc::ptr_eq(&backend.documents, &clone.documents));
    *clone.default_dialect.lock().await = "tcl9.0".to_owned();
    assert_eq!(backend.default_dialect.lock().await.as_str(), "tcl9.0");
    drop(clone);
    assert_eq!(Arc::strong_count(&backend.state), 1);
    drop(backend);
    assert!(
        weak.upgrade().is_none(),
        "background client publishers must not retain BackendState"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_owned_task_retains_the_real_backend_only_until_its_full_teardown() {
    let backend = tests::test_backend();
    let weak = Arc::downgrade(&backend.state);
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let task = crate::rt::spawn({
        let backend = backend.clone();
        let started = Arc::clone(&started);
        let release = Arc::clone(&release);
        async move {
            started.notify_one();
            release.notified().await;
            assert!(backend.db_files.lock().await.is_empty());
        }
    });
    crate::rt::timeout(std::time::Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    drop(backend);
    assert!(weak.upgrade().is_some());
    release.notify_one();
    crate::rt::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert!(
        weak.upgrade().is_none(),
        "the task must not leave a Backend/readiness/task cycle"
    );
}
#[tokio::test]
async fn native_browser_and_wasi_constructor_forms_keep_the_same_service_contract() {
    let (_native, _native_socket) = tower_lsp_server::LspService::new(Backend::new);
    let browser_store: Arc<dyn vfs::SourceStore> = Arc::new(vfs::MemoryStore::new());
    let (_browser, _browser_socket) =
        tower_lsp_server::LspService::new(move |client| Backend::with_store(client, browser_store));
    let wasi_store: Arc<dyn vfs::SourceStore> = Arc::new(vfs::NativeStore);
    let (_wasi, _wasi_socket) =
        tower_lsp_server::LspService::new(move |client| Backend::with_store(client, wasi_store));
}
