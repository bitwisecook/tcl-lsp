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

//! Autoload and package libraries indexed on demand, and their publication.

use super::*;

/// End to end across documents: which *release* of a
/// multi-version package go-to-definition lands in.
///
/// Three providers of `widget` (2.0, 2.3 and 1.5) each define
/// `::widget::init`, all outside the workspace root and reachable only
/// through the user file's own `auto_path` mutation, so the answer comes
/// from the package tier's version selection and nothing else.
///
/// Oracle (`tclsh8.6` 8.6.14 / `tclsh9.0` 9.0.4, byte-identical, the same
/// three versions registered as `package ifneeded widget V …`):
///
/// ```text
/// package require widget            -> 2.3
/// package require widget 2.0        -> 2.3
/// package require -exact widget 2.0 -> 2.0
/// package require widget 1.2        -> 1.5
/// ```
///
/// Dropping `-exact` would send row 3 into 2.3, and answering an
/// unconstrained require with the first provider discovered would send
/// row 1 into whichever directory sorted first — 1.5.
#[tokio::test]
async fn definition_lands_in_the_package_release_tcl_would_load() {
    let ws = TmpWs::new("pkgver-def");
    for ver in ["1.5", "2.0", "2.3"] {
        let dir = format!("v{}", ver.replace('.', ""));
        ws.write(
            &format!("reporoot/{dir}/pkgIndex.tcl"),
            &format!(
                "package ifneeded widget {ver} \
                     [list source [file join $dir widget.tcl]]\n"
            ),
        );
        ws.write(
            &format!("reporoot/{dir}/widget.tcl"),
            &format!(
                "package provide widget {ver}\n\
                     proc ::widget::init {{}} {{ return {ver} }}\n"
            ),
        );
    }

    // Each row: the `package require` line, and the basename of the
    // directory whose `widget.tcl` must be the definition target.
    for (require_line, want_dir) in [
        ("package require widget", "v23"),
        ("package require widget 2.0", "v23"),
        ("package require -exact widget 2.0", "v20"),
        ("package require widget 1.2", "v15"),
    ] {
        let user_src = format!(
            "lappend auto_path [file dirname [file dirname [info script]]]\n\
                 {require_line}\n\
                 puts [::widget::init]\n"
        );
        let name = format!("reporoot/examples/{want_dir}_user.tcl");
        ws.write(&name, &user_src);

        let backend = test_backend();
        let narrow_root = crate::canonical_file_uri(ws.0.join("reporoot/examples")).unwrap();
        *backend.workspace_folders.lock().await = vec![narrow_root];
        backend.scan_workspace_folders().await.unwrap();

        let user_uri = crate::canonical_file_uri(ws.0.join(&name)).unwrap();
        register(&backend, &user_uri, &user_src).await;
        let locs = Box::pin(backend.compute_definition(
            &user_uri,
            Position {
                line: 2,
                character: 12,
            },
        ))
        .await
        .unwrap();
        assert_eq!(
            locs.len(),
            1,
            "`{require_line}` must pull in exactly one release's \
                 implementation, got: {locs:?}",
        );
        let target = locs[0].uri.as_str();
        assert!(
            target.contains(&format!("/{want_dir}/")),
            "`{require_line}` must navigate into {want_dir}'s widget.tcl, \
                 got {target}",
        );
    }
}

#[tokio::test]
async fn package_definition_keeps_each_callers_release_in_one_backend() {
    let workspace = TmpWs::new("pkgver-domains");
    for (directory, version) in [("v1", "1.0"), ("v2", "2.0")] {
        workspace.write(
            &format!("reporoot/{directory}/pkgIndex.tcl"),
            &format!("package ifneeded P {version} [list source [file join $dir package.tcl]]\n"),
        );
        workspace.write(&format!("reporoot/{directory}/package.tcl"), &format!(
                "namespace eval ::P {{proc run {{}} {{return {version}}}; proc café {{}} {{return {version}}}}}\npackage provide P {version}\n",
            ));
    }
    workspace.write(
        "reporoot/hidden/pkgIndex.tcl",
        "if {0} {package ifneeded P 99.0 [list source [file join $dir hidden.tcl]]}\n",
    );
    workspace.write("reporoot/hidden/hidden.tcl",
            "namespace eval ::P {proc run {} {return HIDDEN}; proc café {} {return HIDDEN}}\npackage provide P 99.0\n");
    let mut callers = Vec::new();
    for (name, requirement) in [
        ("first", "-exact P 1.0"),
        ("second", "-exact P 2.0"),
        ("latest", "P"),
    ] {
        let source = format!(
            "lappend auto_path [file dirname [file dirname [info script]]]\npackage require {requirement}\nputs [::P::run]\nputs [::P::café]\n"
        );
        let path = format!("reporoot/examples/{name}.tcl");
        workspace.write(&path, &source);
        callers.push((
            crate::canonical_file_uri(workspace.0.join(path)).unwrap(),
            source,
        ));
    }
    for order in [[0, 1, 0, 2], [1, 0, 1, 2]] {
        let backend = test_backend();
        *backend.workspace_folders.lock().await =
            vec![crate::canonical_file_uri(workspace.0.join("reporoot/examples")).unwrap()];
        backend.scan_workspace_folders().await.unwrap();
        for (uri, source) in &callers {
            // Opening replaces the scanned standalone view. The generic
            // register helper intentionally accumulates source-site views.
            backend
                .workspace_index
                .write()
                .await
                .remove_document(uri.as_str());
            register(&backend, uri, source).await;
        }
        for caller in order {
            let expected = if caller == 0 { "v1" } else { "v2" };
            for (line, character) in [(2, 12), (3, 14)] {
                let locations = backend
                    .compute_definition(&callers[caller].0, Position { line, character })
                    .await
                    .unwrap();
                assert_eq!(
                    locations.len(),
                    1,
                    "caller {caller}, order {order:?}: {locations:?}"
                );
                assert!(
                    locations[0]
                        .uri
                        .as_str()
                        .contains(&format!("/{expected}/package.tcl")),
                    "caller {caller}, order {order:?}: {locations:?}"
                );
            }
        }
        let changed = callers[0].1.replace("-exact P 1.0", "-exact P 2.0");
        backend
            .workspace_index
            .write()
            .await
            .remove_document(callers[0].0.as_str());
        register(&backend, &callers[0].0, &changed).await;
        let locations = backend
            .compute_definition(
                &callers[0].0,
                Position {
                    line: 2,
                    character: 12,
                },
            )
            .await
            .unwrap();
        assert_eq!(locations.len(), 1, "changed requirement: {locations:?}");
        assert!(
            locations[0].uri.as_str().contains("/v2/package.tcl"),
            "changed requirement: {locations:?}"
        );
    }
}

/// TP: with two releases of one package on the search
/// path, the document's own `package require NAME VERSION` decides which
/// one go-to-definition navigates into.
///
/// Oracle (`tclsh8.6` / `tclsh9.0`, `widget` 1.5 and 2.3 both on
/// `auto_path`): `package require widget 2.0` loads **2.3**.
#[tokio::test]
async fn a_versioned_require_indexes_the_release_it_asks_for() {
    let ws = TmpWs::new("pkgver");
    for (dir, ver, marker) in [("v1", "1.5", "OLD"), ("v2", "2.3", "NEW")] {
        ws.write(
            &format!("reporoot/{dir}/pkgIndex.tcl"),
            &format!("package ifneeded widget {ver} [list source [file join $dir widget.tcl]]\n"),
        );
        ws.write(
            &format!("reporoot/{dir}/widget.tcl"),
            &format!(
                "package provide widget {ver}\n\
                     proc ::widget::render {{}} {{ return {marker} }}\n"
            ),
        );
    }
    let root = ws.0.join("reporoot");
    let root_s = root.to_str().unwrap();
    // `v1` is listed first, so a first-provider-wins resolve would answer
    // 1.5, which is what this pins against.
    let user_src = format!(
        "set auto_path {{{root_s}/v1 {root_s}/v2}}\n\
             package require widget 2.0\n\
             ::widget::render\n"
    );
    ws.write("reporoot/examples/user.tcl", &user_src);

    let backend = test_backend();
    let narrow_root = crate::canonical_file_uri(ws.0.join("reporoot/examples")).unwrap();
    *backend.workspace_folders.lock().await = vec![narrow_root];
    backend.scan_workspace_folders().await.unwrap();

    let user_uri = crate::canonical_file_uri(ws.0.join("reporoot/examples/user.tcl")).unwrap();
    register(&backend, &user_uri, &user_src).await;
    let locs = Box::pin(backend.compute_definition(
        &user_uri,
        Position {
            line: 2,
            character: 4,
        },
    ))
    .await
    .unwrap_or_default();
    assert_eq!(locs.len(), 1, "expected one definition, got: {locs:?}");
    assert!(
        locs[0].uri.as_str().contains("/v2/"),
        "`package require widget 2.0` selects 2.3, so definition must land \
             in v2/widget.tcl, got: {:?}",
        locs[0].uri,
    );
}

/// The autoload tier merges the defining library file
/// into the workspace index, so cross-document **references** reach the
/// library declaration and the library's own internal call sites — not
/// just go-to-definition.
#[tokio::test]
async fn autoload_merge_makes_references_reach_the_library_m8() {
    let backend = test_backend();
    let (_library_dir, lib_uri) = seed_autoload_library(&backend, "autoload-refs").await;
    // The workspace document only *calls* the library command.
    let app = Uri::from_str("file:///app.tcl").unwrap();
    let app_src = "Rbc_ActiveLegend .g\n";
    register(&backend, &app, app_src).await;
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(app_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&app, app_src, &analysis, Position::new(0, 3), true)
        .await
        .unwrap();
    assert!(
        refs.iter()
            .any(|l| l.uri == lib_uri && l.range.start.line == 0),
        "declaration in the library file: {refs:?}"
    );
    assert!(
        refs.iter()
            .any(|l| l.uri == lib_uri && l.range.start.line == 1),
        "library-internal call site: {refs:?}"
    );
}

#[tokio::test]
async fn original_autoload_navigation_joins_current_source_declaration_identity() {
    let backend = test_backend();
    let (_library_dir, lib_uri) =
        seed_autoload_library(&backend, "original-autoload-navigation").await;
    let app = Uri::from_str("file:///original-autoload-app.tcl").unwrap();
    let source = "Rbc_ActiveLegend .g\n";
    register(&backend, &app, source).await;
    let document = backend.read_document(&app).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(&app, document.text.clone(), document.dialect)
        .await
        .unwrap();
    assert!(!analysis.allows_retained_logical_declaration_advice());
    let position = Position::new(0, 3);
    let locations = backend
        .autoload_definition(&app, source, position, &analysis)
        .await
        .unwrap();
    assert_eq!(locations.len(), 1, "{locations:?}");
    assert_eq!(locations[0].uri, lib_uri);
    assert_eq!(locations[0].range.start.line, 0);
    let hover = backend
        .cross_document_hover(&app, source, position, &analysis)
        .await
        .unwrap()
        .expect("current original library declaration hover");
    assert!(hover.value.contains("Rbc_ActiveLegend"), "{hover:?}");
    assert_eq!(
        backend
            .autoload_definition(&app, &format!("{source} "), position, &analysis)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
    let mut missing = analysis.as_ref().clone();
    missing.resolved_input = None;
    assert_eq!(
        backend
            .autoload_definition(&app, source, position, &missing)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
    assert!(
        backend
            .cross_document_hover(&app, source, position, &missing)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        backend
            .autoload_definition(&app, source, Position::new(0, 17), &analysis)
            .await
            .unwrap(),
        [] as [tower_lsp_server::ls_types::Location; 0]
    );
}

#[tokio::test]
async fn native_bare_autoload_head_keeps_its_own_readonly_eligibility_and_source_currency() {
    for public in [true, false] {
        let backend = test_backend();
        let (_library_dir, provider) =
            seed_autoload_library(&backend, &format!("bare-autoload-{public}")).await;
        let provider_path = provider.to_file_path().unwrap().into_owned();
        std::fs::write(&provider_path, "proc Rbc_ActiveLegend {} {}\n").unwrap();
        let app: Uri = "file:///bare-autoload-caller.tcl".parse().unwrap();
        let source = "Rbc_ActiveLegend\n";
        register(&backend, &app, source).await;
        let document = backend.read_document(&app).await.unwrap().unwrap();
        let analysis = backend
            .analysis_for(&app, document.text.clone(), document.dialect.clone())
            .await
            .unwrap();
        let position = Position::new(0, 4);
        assert!(core_definition::original_autoload_request_at(source, &analysis, 0, 4).is_some());
        assert!(
            !backend
                .workspace_index
                .read()
                .await
                .contains_document(provider.as_str())
        );
        let locations = if public {
            let response = backend
                .goto_definition(
                    serde_json::from_value(serde_json::json!({
                        "textDocument":{"uri":app.as_str()}, "position":{"line":0,"character":4}
                    }))
                    .unwrap(),
                )
                .await
                .unwrap();
            match response {
                Some(GotoDefinitionResponse::Array(locations)) => locations,
                other => panic!("first bare public autoload response: {other:?}"),
            }
        } else {
            // The exact autoload owner independently proves head eligibility;
            // a declined generic geometry helper cannot erase that owner.
            backend
                .definition_workspace_fallback(
                    &app,
                    &document,
                    &analysis,
                    position,
                    (
                        backend.diag_inputs_epoch(),
                        backend.live_publication_uri_generation(&app),
                    ),
                    false,
                )
                .await
                .unwrap()
        };
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri, provider);
        assert_eq!(locations[0].range.start.line, 0);
        if public {
            for _ in 0..2 {
                let response = backend
                    .goto_definition(
                        serde_json::from_value(serde_json::json!({
                            "textDocument":{"uri":app.as_str()}, "position":{"line":0,"character":4}
                        }))
                        .unwrap(),
                    )
                    .await
                    .unwrap();
                assert!(
                    matches!(response, Some(GotoDefinitionResponse::Array(ref targets))
                        if targets.len() == 1 && targets[0].uri == provider && targets[0].range.start.line == 0),
                    "repeated publication must preserve the current target: {response:?}"
                );
            }
        }

        assert!(
            core_definition::original_autoload_request_at("Rbc_ActiveLegend \n", &analysis, 0, 4)
                .is_none()
        );
        let mut missing = analysis.as_ref().clone();
        missing.resolved_input = None;
        assert!(core_definition::original_autoload_request_at(source, &missing, 0, 4).is_none());
        std::fs::write(&provider_path, "proc replaced_header {} {}\n").unwrap();
        assert!(
            backend
                .compute_definition(&app, position)
                .await
                .unwrap()
                .is_empty(),
            "a changed closed provider cannot reuse its former header"
        );
    }
}

#[tokio::test]
async fn library_publication_refuses_actual_configuration_and_factory_changes() {
    use salsa::Setter as _;

    for factory_change in [false, true] {
        let backend = test_backend();
        let uri = Uri::from_str("untitled:library-full-input").unwrap();
        register(&backend, &uri, "proc library_full_input {} {}\n").await;
        let snapshot = backend.read_document(&uri).await.unwrap().unwrap();
        let epoch = backend.diag_inputs_epoch();
        let generation = *backend.class_factory_generation.read().await;
        let receipt = backend
            .capture_workspace_symbol_analysis(&uri, &snapshot)
            .await
            .unwrap()
            .unwrap();
        if factory_change {
            let mut analyser = Analyser::new();
            let oracle = Arc::new(
                analyser
                    .analyse("oo::class create LibraryFactory {}", "tcl8.6")
                    .class_factories(),
            );
            let mut generation = backend.class_factory_generation.write().await;
            let mut db = backend.db.lock().await;
            let files = backend.db_files.lock().await;
            files[&uri]
                .set_workspace_class_factories(&mut *db)
                .to(Some(oracle));
            *generation = generation.saturating_add(1);
        } else {
            let config = backend.resolved_db_config(&uri).await;
            let mut db = backend.db.lock().await;
            config
                .set_extra_commands(&mut *db)
                .to(vec!["library_new_command".to_owned()]);
        }
        assert!(
            !backend
                .publish_checked_library_analyses(
                    &[(uri.clone(), snapshot, receipt)],
                    epoch,
                    generation
                )
                .await
                .unwrap()
        );
        assert!(
            !backend
                .autoloaded_library_uris
                .lock()
                .await
                .contains(uri.as_str())
        );
    }
}

#[tokio::test]
async fn library_publication_retains_original_provider_mapping_across_equal_path_aba() {
    let mut backend = test_backend();
    let store = Arc::new(vfs::MemoryStore::new());
    let uri = Uri::from_str("file:///library-original-mapping.tcl").unwrap();
    let path = PathBuf::from("/library-provider-mount/source.tcl");
    let source = "proc library_provider_target {} {}\n";
    store.upsert_uri(uri.as_str(), &path, source.as_bytes().to_vec());
    backend.state_mut_for_test().store = store.clone();
    let snapshot = backend.read_document(&uri).await.unwrap().unwrap();
    let epoch = backend.diag_inputs_epoch();
    let generation = *backend.class_factory_generation.read().await;
    let receipt = backend
        .capture_workspace_symbol_analysis(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    assert!(store.remove_uri(uri.as_str()));
    store.upsert_uri(uri.as_str(), &path, source.as_bytes().to_vec());
    let result = backend
        .publish_checked_library_analyses(&[(uri.clone(), snapshot, receipt)], epoch, generation)
        .await;
    assert!(!result.unwrap_or(false));
    assert!(
        !backend
            .autoloaded_library_uris
            .lock()
            .await
            .contains(uri.as_str())
    );
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .contains_document(uri.as_str())
    );
}

#[tokio::test]
async fn library_publication_checks_closed_saved_source_and_commits_whole_batch() {
    for change_saved_source in [false, true] {
        let backend = test_backend();
        let (_library_dir, uri) =
            seed_autoload_library(&backend, "checked-library-saved-source").await;
        let snapshot = backend.read_document(&uri).await.unwrap().unwrap();
        let epoch = backend.diag_inputs_epoch();
        let generation = *backend.class_factory_generation.read().await;
        let receipt = backend
            .capture_workspace_symbol_analysis(&uri, &snapshot)
            .await
            .unwrap()
            .unwrap();
        if change_saved_source {
            std::fs::write(
                uri.to_file_path().unwrap(),
                "proc changed_library_target {} {}\n",
            )
            .unwrap();
        }
        assert_eq!(
            backend
                .publish_checked_library_analyses(
                    &[(uri.clone(), snapshot, receipt)],
                    epoch,
                    generation
                )
                .await
                .unwrap(),
            !change_saved_source
        );
        assert_eq!(
            backend
                .autoloaded_library_uris
                .lock()
                .await
                .contains(uri.as_str()),
            !change_saved_source
        );
        assert_eq!(
            backend
                .workspace_index
                .read()
                .await
                .contains_document(uri.as_str()),
            !change_saved_source
        );
    }
}

#[tokio::test]
async fn library_publication_reuses_genuine_full_input_and_factory_receipts() {
    let backend = test_backend();
    let uri = Uri::from_str("untitled:library-publication-owner").unwrap();
    register(&backend, &uri, "proc library_receipt_target {} {}\n").await;
    let snapshot = backend.read_document(&uri).await.unwrap().unwrap();
    let epoch = backend.diag_inputs_epoch();
    let generation = *backend.class_factory_generation.read().await;
    let receipt = backend
        .capture_workspace_symbol_analysis(&uri, &snapshot)
        .await
        .unwrap()
        .unwrap();
    let batch = vec![(uri.clone(), snapshot, receipt)];
    assert!(
        !backend
            .publish_checked_library_analyses(&batch, epoch.wrapping_add(1), generation)
            .await
            .unwrap()
    );
    assert!(
        !backend
            .publish_checked_library_analyses(&batch, epoch, generation.wrapping_add(1))
            .await
            .unwrap()
    );
    assert!(
        backend
            .publish_checked_library_analyses(&batch, epoch, generation)
            .await
            .unwrap()
    );
    assert!(
        backend
            .autoloaded_library_uris
            .lock()
            .await
            .contains(uri.as_str())
    );
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::library_receipt_target")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn original_library_publication_refuses_contended_caller_and_provider_edits() {
    for edit_provider in [false, true] {
        let backend = Arc::new(test_backend());
        let caller_uri = Uri::from_str("file:///library-publication-caller.tcl").unwrap();
        let provider_uri = Uri::from_str("file:///library-publication-provider.tcl").unwrap();
        register(&backend, &caller_uri, "original_library_target ARG\n").await;
        register(
            &backend,
            &provider_uri,
            "proc original_library_target {arg} {}\n",
        )
        .await;
        let caller_document = backend.read_document(&caller_uri).await.unwrap().unwrap();
        let caller = backend
            .library_publication_source(&caller_uri, caller_document)
            .await
            .unwrap();
        let provider_document = backend.read_document(&provider_uri).await.unwrap().unwrap();
        let analysis = backend
            .analysis_for(
                &provider_uri,
                provider_document.text.clone(),
                provider_document.dialect.clone(),
            )
            .await
            .unwrap();
        let provider = backend
            .library_publication_source(&provider_uri, provider_document)
            .await
            .unwrap();
        backend
            .workspace_index
            .write()
            .await
            .remove_document(provider_uri.as_str());
        let epoch = backend.diag_inputs_epoch();
        let gate = backend.rehoming_gate.lock().await;
        let (started, waiting) = tokio::sync::oneshot::channel();
        let publishing = crate::rt::spawn({
            let backend = Arc::clone(&backend);
            async move {
                started.send(()).unwrap();
                backend
                    .publish_original_library_analyses(epoch, &caller, &[(provider, analysis)])
                    .await
            }
        });
        waiting.await.unwrap();
        let edited_uri = if edit_provider {
            provider_uri.clone()
        } else {
            caller_uri.clone()
        };
        let edited_source = if edit_provider {
            "proc current_library_target {arg} {}\n"
        } else {
            "current_library_target ARG\n"
        };
        let editing = crate::rt::spawn({
            let backend = Arc::clone(&backend);
            let edited_uri = edited_uri.clone();
            async move {
                backend
                    .did_change(DidChangeTextDocumentParams {
                        text_document:
                            tower_lsp_server::ls_types::VersionedTextDocumentIdentifier {
                                uri: edited_uri,
                                version: 2,
                            },
                        content_changes: vec![
                            tower_lsp_server::ls_types::TextDocumentContentChangeEvent {
                                range: None,
                                range_length: None,
                                text: edited_source.into(),
                            },
                        ],
                    })
                    .await;
            }
        });
        wait_for_original_library_edit(&backend, &edited_uri, edited_source).await;
        assert_library_edit_keeps_configuration_epoch(&backend, epoch);
        drop(gate);
        assert!(
            !crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
                .await
                .unwrap()
                .unwrap()
        );
        assert_original_library_edit_result(
            &backend,
            &edited_uri,
            edited_source,
            edit_provider,
            editing,
        )
        .await;
    }
}

async fn wait_for_original_library_edit(backend: &Backend, edited_uri: &Uri, edited_source: &str) {
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let edited = backend
                .documents
                .lock("original-library-edit-visible")
                .await
                .get(edited_uri)
                .is_some_and(|document| {
                    document.text.as_ref() == edited_source && document.revision > 0
                });
            if edited {
                break;
            }
            crate::rt::yield_now().await;
        }
    })
    .await
    .expect("the edit must splice while publication waits for rehoming");
}

async fn assert_original_library_edit_result(
    backend: &Backend,
    edited_uri: &Uri,
    edited_source: &str,
    edit_provider: bool,
    editing: crate::rt::JoinHandle<()>,
) {
    crate::rt::timeout(std::time::Duration::from_secs(5), editing)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        backend
            .read_document(edited_uri)
            .await
            .unwrap()
            .unwrap()
            .text
            .as_ref(),
        edited_source
    );
    let index = backend.workspace_index.read().await;
    assert!(!index.workspace_command_exists("::original_library_target"));
    if edit_provider {
        assert!(index.workspace_command_exists("::current_library_target"));
    }
}

fn assert_library_edit_keeps_configuration_epoch(backend: &Backend, epoch: u64) {
    assert_eq!(
        backend.diag_inputs_epoch(),
        epoch,
        "document edits are independent of configuration epochs"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn original_library_publication_rechecks_closed_source_after_gate_wait() {
    let backend = Arc::new(test_backend());
    let (_library_dir, library_uri) =
        seed_autoload_library(&backend, "original-library-closed-race").await;
    let caller_uri = Uri::from_str("file:///original-library-closed-caller.tcl").unwrap();
    register(&backend, &caller_uri, "Rbc_ActiveLegend ARG\n").await;
    let caller_document = backend.read_document(&caller_uri).await.unwrap().unwrap();
    let caller = backend
        .library_publication_source(&caller_uri, caller_document)
        .await
        .unwrap();
    let library_document = backend.read_document(&library_uri).await.unwrap().unwrap();
    let analysis = backend
        .analysis_for(
            &library_uri,
            library_document.text.clone(),
            library_document.dialect.clone(),
        )
        .await
        .unwrap();
    let library = backend
        .library_publication_source(&library_uri, library_document)
        .await
        .unwrap();
    assert!(library.open_revision.is_none());
    let epoch = backend.diag_inputs_epoch();
    let gate = backend.rehoming_gate.lock().await;
    let (started, waiting) = tokio::sync::oneshot::channel();
    let publishing = crate::rt::spawn({
        let backend = Arc::clone(&backend);
        async move {
            started.send(()).unwrap();
            backend
                .publish_original_library_analyses(epoch, &caller, &[(library, analysis)])
                .await
        }
    });
    waiting.await.unwrap();
    std::fs::write(
        library_uri.to_file_path().unwrap(),
        "proc current_closed_target {arg} {}\n",
    )
    .unwrap();
    drop(gate);
    assert!(
        !crate::rt::timeout(std::time::Duration::from_secs(5), publishing)
            .await
            .unwrap()
            .unwrap()
    );
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::Rbc_ActiveLegend")
    );
}

/// Autoload rename leg: a rename triggered from the workspace
/// call site rewrites the library declaration and the library-internal
/// call site, so the whole family stays consistent.
#[tokio::test]
async fn autoload_merge_makes_rename_reach_the_library_m8() {
    let backend = test_backend();
    let (_library_dir, lib_uri) = seed_autoload_library(&backend, "autoload-rename").await;
    let app = Uri::from_str("file:///app.tcl").unwrap();
    let app_src = "Rbc_ActiveLegend .g\n";
    register(&backend, &app, app_src).await;
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(app_src, "tcl8.6").clone()
    };
    let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
        std::collections::HashMap::new();
    backend
        .add_cross_document_rename_edits(
            &app,
            app_src,
            &analysis,
            Position::new(0, 3),
            "Rbc_Shiny",
            &mut changes,
        )
        .await
        .unwrap();
    let lib_edits = changes.get(&lib_uri).cloned().unwrap_or_default();
    assert_eq!(
        lib_edits.len(),
        2,
        "library declaration + internal call: {lib_edits:?}"
    );
}

/// FP guard: when the workspace itself defines the command, the autoload
/// tier must not fire — the workspace definition wins and no library file
/// is merged into the index.
#[tokio::test]
async fn autoload_merge_abstains_when_the_workspace_defines_the_command() {
    let backend = test_backend();
    let (_library_dir, lib_uri) = seed_autoload_library(&backend, "autoload-abstain").await;
    let def = Uri::from_str("file:///def.tcl").unwrap();
    register(&backend, &def, "proc Rbc_ActiveLegend {graph} {}\n").await;
    let app = Uri::from_str("file:///app.tcl").unwrap();
    let app_src = "Rbc_ActiveLegend .g\n";
    register(&backend, &app, app_src).await;
    let analysis = {
        let mut a = Analyser::new();
        a.analyse(app_src, "tcl8.6").clone()
    };
    let refs = backend
        .cross_document_references(&app, app_src, &analysis, Position::new(0, 3), true)
        .await
        .unwrap();
    assert!(
        refs.iter().all(|l| l.uri != lib_uri),
        "the same-named library must stay unmerged: {refs:?}"
    );
    assert!(
        backend.autoloaded_library_uris.lock().await.is_empty(),
        "no library merge may be recorded"
    );
}

/// A package-database rebuild drops previously-merged library files from
/// the index (they may no longer be on the resolved `auto_path`); the next
/// query re-merges on demand.
#[tokio::test]
async fn autoload_merge_is_dropped_when_the_package_database_rebuilds() {
    let backend = test_backend();
    let (_library_dir, _) = seed_autoload_library(&backend, "autoload-rebuild").await;
    let resolved = backend
        .ensure_autoload_indexed("Rbc_ActiveLegend", "::")
        .await
        .unwrap();
    assert_eq!(resolved.as_deref(), Some("::Rbc_ActiveLegend"));
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::Rbc_ActiveLegend")
    );
    // Rebuild with no library paths: the merged entries must be dropped.
    backend.scan_workspace_folders().await.unwrap();
    assert!(
        !backend
            .workspace_index
            .read()
            .await
            .workspace_command_exists("::Rbc_ActiveLegend"),
        "stale library definitions must not survive a package-database rebuild"
    );
    assert!(backend.autoloaded_library_uris.lock().await.is_empty());
}
