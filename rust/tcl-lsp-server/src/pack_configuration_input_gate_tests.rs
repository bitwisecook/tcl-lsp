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
use tcl_dialect::model::WorkspaceTrust;

#[tokio::test]
async fn original_folder_and_trust_setters_wait_for_the_report_input_owner() {
    let backend = crate::tests::test_backend();
    let folder: Uri = "file:///pack-configuration-source".parse().unwrap();
    let params = InitializeParams {
        workspace_folders: Some(vec![tower_lsp_server::ls_types::WorkspaceFolder {
            uri: folder.clone(),
            name: "pack".to_owned(),
        }]),
        ..InitializeParams::default()
    };
    let inputs = backend.analyser_inputs_gate.read().await;
    let mut folders = Box::pin(backend.apply_workspace_folders(&params));
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(folders.as_mut().poll(&mut context).is_pending());
    assert!(backend.workspace_folders.lock().await.is_empty());
    drop(inputs);
    folders.await;
    assert_eq!(*backend.workspace_folders.lock().await, vec![folder]);
    let params = InitializeParams {
        initialization_options: Some(serde_json::json!({"workspaceTrust": "untrusted"})),
        ..InitializeParams::default()
    };
    let inputs = backend.analyser_inputs_gate.read().await;
    let mut trust = Box::pin(backend.apply_initialization_options(&params));
    assert!(trust.as_mut().poll(&mut context).is_pending());
    assert_eq!(
        *backend.workspace_trust.lock().await,
        WorkspaceTrust::Trusted
    );
    drop(inputs);
    trust.await;
    assert_eq!(
        *backend.workspace_trust.lock().await,
        WorkspaceTrust::Untrusted
    );
}

#[tokio::test]
async fn pack_notice_union_requires_matching_source_and_publication_owners() {
    let backend = crate::tests::test_backend();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is after the epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "tcl-lsp-pack-notice-{}-{nonce}.tclspec",
        std::process::id(),
    ));
    let source = "speclib sample 1.0 {}\n";
    std::fs::write(&path, source).expect("write the exact loader source");
    let origin = backend
        .store
        .source_path_origin(&path)
        .expect("the native provider issues the selected source path");
    let uri = canonical_file_uri(&path).expect("the selected path has a file URI");
    let packs = Arc::new(tcl_spectcl::PackSet::default());
    let registry = backend.registry_for_dialect("tcl8.6").await;
    let diagnostic = tower_lsp_server::ls_types::Diagnostic {
        message: "source-bound notice".to_owned(),
        ..Default::default()
    };
    install_saved_pack_notice(
        &backend,
        &uri,
        source,
        &packs,
        &registry,
        &diagnostic,
        &origin,
    );
    let merged = backend.diagnostic_publisher.with_pack_notices(
        PackNoticeContext {
            uri: &uri,
            source,
            pack_seq: 7,
            packs: &packs,
            registry: &registry,
            store: backend.store.as_ref(),
        },
        Vec::new(),
    );
    assert_eq!(merged, std::slice::from_ref(&diagnostic));
    // A retained Studio image has its own real admission owner; it does
    // not need a fabricated loader SourcePathOrigin to survive the union.
    let studio_admission = Arc::new(
        DocumentPathAdmission::capture(&uri, backend.store.as_ref())
            .expect("capture the selected Studio source path"),
    );
    install_studio_pack_notice(
        &backend,
        &uri,
        source,
        &packs,
        &registry,
        &diagnostic,
        studio_admission,
    );
    assert_studio_and_stale_pack_notice_rejections(
        &backend,
        &uri,
        source,
        &packs,
        &registry,
        &diagnostic,
    );
    let replacement = Arc::new(tcl_spectcl::PackSet::default());
    assert_pack_notice_wrong_publication_owner(&backend, &uri, source, &replacement, &registry);
    let other_registry = backend.registry_for_dialect("tcl9.0").await;
    assert_pack_notice_wrong_publication_owner(&backend, &uri, source, &packs, &other_registry);
    std::fs::write(&path, "speclib changed 1.0 {}\n").expect("change the actual provider source");
    assert_eq!(
        backend.diagnostic_publisher.with_pack_notices(
            PackNoticeContext {
                uri: &uri,
                source,
                pack_seq: 7,
                packs: &packs,
                registry: &registry,
                store: backend.store.as_ref()
            },
            Vec::new()
        ),
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
    std::fs::remove_file(path).expect("remove the temporary pack source");
}

fn install_saved_pack_notice(
    backend: &Backend,
    uri: &Uri,
    source: &str,
    packs: &Arc<tcl_spectcl::PackSet>,
    registry: &Arc<CommandRegistry>,
    diagnostic: &tower_lsp_server::ls_types::Diagnostic,
    origin: &vfs::SourcePathOrigin,
) {
    let mut by_uri = HashMap::new();
    by_uri.insert(
        uri.clone(),
        vec![PackNoticeEntry {
            diagnostic: diagnostic.clone(),
            source: Arc::from(source),
            owner: PackNoticeSourceOwner::Saved(origin.clone()),
        }],
    );
    backend.diagnostic_publisher.set_pack_notices(
        7,
        Arc::clone(packs),
        vec![Arc::clone(registry)],
        by_uri,
        Vec::new(),
        tcl_dialect::model::WorkspaceTrust::Trusted,
    );
}

fn install_studio_pack_notice(
    backend: &Backend,
    uri: &Uri,
    source: &str,
    packs: &Arc<tcl_spectcl::PackSet>,
    registry: &Arc<CommandRegistry>,
    diagnostic: &tower_lsp_server::ls_types::Diagnostic,
    studio_admission: Arc<DocumentPathAdmission>,
) {
    backend.diagnostic_publisher.set_pack_notices(
        7,
        Arc::clone(packs),
        vec![Arc::clone(registry)],
        HashMap::from([(
            uri.clone(),
            vec![PackNoticeEntry {
                diagnostic: diagnostic.clone(),
                source: Arc::from(source),
                owner: PackNoticeSourceOwner::Studio(studio_admission),
            }],
        )]),
        Vec::new(),
        tcl_dialect::model::WorkspaceTrust::Trusted,
    );
}

fn assert_studio_and_stale_pack_notice_rejections(
    backend: &Backend,
    uri: &Uri,
    source: &str,
    packs: &Arc<tcl_spectcl::PackSet>,
    registry: &Arc<CommandRegistry>,
    diagnostic: &tower_lsp_server::ls_types::Diagnostic,
) {
    assert_eq!(
        backend.diagnostic_publisher.with_pack_notices(
            PackNoticeContext {
                uri,
                source,
                pack_seq: 7,
                packs,
                registry,
                store: backend.store.as_ref()
            },
            Vec::new()
        ),
        std::slice::from_ref(diagnostic)
    );
    assert_eq!(
        backend.diagnostic_publisher.with_pack_notices(
            PackNoticeContext {
                uri,
                source: "speclib changed 1.0 {}\n",
                pack_seq: 7,
                packs,
                registry,
                store: backend.store.as_ref()
            },
            Vec::new()
        ),
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
    assert_eq!(
        backend.diagnostic_publisher.with_pack_notices(
            PackNoticeContext {
                uri,
                source,
                pack_seq: 8,
                packs,
                registry,
                store: backend.store.as_ref()
            },
            Vec::new()
        ),
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
}

fn assert_pack_notice_wrong_publication_owner(
    backend: &Backend,
    uri: &Uri,
    source: &str,
    packs: &Arc<tcl_spectcl::PackSet>,
    registry: &Arc<CommandRegistry>,
) {
    assert_eq!(
        backend.diagnostic_publisher.with_pack_notices(
            PackNoticeContext {
                uri,
                source,
                pack_seq: 7,
                packs,
                registry,
                store: backend.store.as_ref()
            },
            Vec::new()
        ),
        [] as [tower_lsp_server::ls_types::Diagnostic; 0]
    );
}
