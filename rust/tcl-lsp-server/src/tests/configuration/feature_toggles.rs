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

//! Feature toggles, their folder overlay and the providers they gate.

use super::*;

#[test]
fn feature_toggles_default_on_and_merge() {
    let mut toggles = FeatureToggles::default();
    // Absent keys default to enabled.
    assert!(toggles.is_enabled("hover"));
    assert!(toggles.is_enabled("completion"));
    // Applying a partial `features` object sets only the named keys.
    let features = serde_json::json!({"hover": false, "folding": false});
    toggles.apply(features.as_object().unwrap());
    assert!(!toggles.is_enabled("hover"));
    assert!(!toggles.is_enabled("folding"));
    // Unnamed keys keep their default-on value.
    assert!(toggles.is_enabled("completion"));
    // Re-enabling merges (does not reset the rest).
    toggles.apply(serde_json::json!({"hover": true}).as_object().unwrap());
    assert!(toggles.is_enabled("hover"));
    assert!(!toggles.is_enabled("folding"));
}

#[test]
fn inlay_keys_default_off() {
    // The opt-in inlay families default off, so `getEffectiveConfig`
    // reports them disabled until an editor sets them (matching the
    // handler's default-off gate).
    let toggles = FeatureToggles::default();
    assert!(!toggles.is_enabled("inlayTypeHints"));
    assert!(!toggles.is_enabled("inlayParameterHints"));
    let map = toggles.resolved_map();
    assert_eq!(
        map.get("inlayTypeHints")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert_eq!(
        map.get("inlayParameterHints")
            .and_then(serde_json::Value::as_bool),
        Some(false),
    );
}

#[test]
fn legacy_inlay_hints_key_maps_to_type_family() {
    // The retired `inlayHints` key maps to `inlayTypeHints`: an existing
    // explicit opt-in keeps showing the
    // inferred-variable-type hints after the rename, while the verbose
    // parameter-name hints stay off.
    let mut toggles = FeatureToggles::default();
    toggles.apply(serde_json::json!({"inlayHints": true}).as_object().unwrap());
    assert!(toggles.is_enabled("inlayTypeHints"));
    // It does not touch the parameter-name family.
    assert!(!toggles.is_enabled("inlayParameterHints"));
}

#[test]
fn explicit_new_inlay_key_wins_over_legacy_alias() {
    // When a config carries both, the explicit new `inlayTypeHints` key
    // takes precedence over the `inlayHints` alias.
    let mut toggles = FeatureToggles::default();
    toggles.apply(
        serde_json::json!({"inlayHints": true, "inlayTypeHints": false})
            .as_object()
            .unwrap(),
    );
    assert!(!toggles.is_enabled("inlayTypeHints"));
}

#[test]
fn feature_toggles_resolved_map_covers_known_keys() {
    let mut toggles = FeatureToggles::default();
    toggles.apply(serde_json::json!({"hover": false}).as_object().unwrap());
    let map = toggles.resolved_map();
    // Every advertised key is present and boolean.  Default-on keys
    // report `true` (except the one we disabled); the opt-in
    // `DEFAULT_OFF` inlay keys report `false`.
    for key in FeatureToggles::KEYS {
        let expected = *key != "hover" && !FeatureToggles::DEFAULT_OFF.contains(key);
        assert_eq!(
            map.get(*key).and_then(serde_json::Value::as_bool),
            Some(expected),
            "feature {key}"
        );
    }
    // The toggleable keys the e2e contract checks are all reported.
    for key in [
        "hover",
        "completion",
        "documentSymbols",
        "definition",
        "references",
        "signatureHelp",
        "folding",
        "selectionRange",
        "documentLinks",
    ] {
        assert!(map.contains_key(key), "missing reported feature {key}");
    }
}

/// The folder-over-global overlay `getEffectiveConfig` and
/// every provider gate share.  A folder that mentions one feature must
/// override exactly that one and leave the rest of the global set alone —
/// the mistake in the other direction (a folder resetting unmentioned keys
/// to their defaults) would silently re-enable a globally-disabled feature
/// for every document under that folder.
#[test]
fn folder_toggles_overlay_only_the_keys_the_folder_sets() {
    let mut global = FeatureToggles::default();
    global.apply(
        serde_json::json!({ "hover": false, "completion": false })
            .as_object()
            .unwrap(),
    );
    let mut folder = FeatureToggles::default();
    folder.apply(
        serde_json::json!({ "hover": true, "selectionRange": false })
            .as_object()
            .unwrap(),
    );

    let merged = global.overlaid_with(&folder);
    assert!(merged.is_enabled("hover"), "folder must win over global");
    assert!(
        !merged.is_enabled("completion"),
        "a key the folder does not mention keeps the global value"
    );
    assert!(
        !merged.is_enabled("selectionRange"),
        "a key only the folder sets must take effect"
    );
    assert!(
        merged.is_enabled("definition"),
        "a key neither sets keeps its default"
    );
    // Default-off keys stay off through the overlay unless something sets
    // them, so an opt-in feature cannot be switched on by mere merging.
    assert!(!merged.is_enabled("inlayTypeHints"));
    assert!(!merged.is_enabled_default_off("willSaveWaitUntil"));

    // An empty folder override is the identity, which is what makes the
    // no-folder-config case indistinguishable from the pre-overlay
    // behaviour.
    let unchanged = global.overlaid_with(&FeatureToggles::default());
    assert_eq!(unchanged.resolved_map(), global.resolved_map());
}

/// The shipped `tclLsp.xcDiagnostics.enabled` setting is a dedicated
/// config *section*, not a `features.*` key — `parse_folder_config` must
/// still map it onto the `xcDiagnostics` feature toggle so the advertised
/// `VS Code` opt-in actually reaches `xc_diagnostics_enabled`.
#[test]
fn folder_config_maps_xc_diagnostics_section_to_toggle() {
    let cfg = serde_json::json!({ "xcDiagnostics": { "enabled": true } });
    let fc = parse_folder_config(&cfg).expect("folder config");
    assert_eq!(
        fc.feature_toggles.set.get("xcDiagnostics").copied(),
        Some(true),
        "tclLsp.xcDiagnostics.enabled must set the xcDiagnostics toggle",
    );
    assert!(fc.feature_toggles.is_enabled_default_off("xcDiagnostics"));

    // Absent section leaves the (default-off) toggle unset.
    let empty = parse_folder_config(&serde_json::json!({})).expect("folder config");
    assert_eq!(empty.feature_toggles.set.get("xcDiagnostics"), None);
    assert!(
        !empty
            .feature_toggles
            .is_enabled_default_off("xcDiagnostics")
    );
}

/// Format-on-save honours a folder-scoped opt-in: a document under a folder
/// that turns on `willSaveWaitUntil` formats, while documents outside it
/// stay inert under the (off) global default.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn will_save_format_respects_per_folder_opt_in() {
    let backend = test_backend();
    let folder = Uri::from_str("file:///proj-a").unwrap();
    let inside = Uri::from_str("file:///proj-a/file.tcl").unwrap();
    let outside = Uri::from_str("file:///other/file.tcl").unwrap();
    *backend.workspace_folders.lock().await = vec![folder.clone()];
    register(&backend, &inside, "set    x     1\n").await;
    register(&backend, &outside, "set    x     1\n").await;

    // Global toggle stays off; only proj-a opts in.
    let feature_toggles = {
        let mut t = FeatureToggles::default();
        t.set.insert("willSaveWaitUntil".to_owned(), true);
        t
    };
    let fc = FolderConfig {
        feature_toggles,
        ..FolderConfig::default()
    };
    backend
        .apply_folder_configs(vec![(folder.clone(), fc)])
        .await;

    let params = |uri: &Uri| WillSaveTextDocumentParams {
        text_document: tower_lsp_server::ls_types::TextDocumentIdentifier { uri: uri.clone() },
        reason: tower_lsp_server::ls_types::TextDocumentSaveReason::MANUAL,
    };
    assert!(
        backend
            .will_save_wait_until(params(&inside))
            .await
            .unwrap()
            .is_some_and(|edits| !edits.is_empty()),
        "a folder-scoped opt-in must format on save",
    );
    assert!(
        backend
            .will_save_wait_until(params(&outside))
            .await
            .unwrap()
            .is_none(),
        "outside the opted-in folder, save stays inert under the global default",
    );
}

/// `willSaveWaitUntil` is opt-in: with the feature toggle unset it returns
/// no edits even for mis-formatted source; once enabled it formats.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn will_save_format_is_opt_in() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///will-save.tcl").unwrap();
    register(&backend, &uri, "set    x     1\n").await;
    let params = WillSaveTextDocumentParams {
        text_document: tower_lsp_server::ls_types::TextDocumentIdentifier { uri: uri.clone() },
        reason: tower_lsp_server::ls_types::TextDocumentSaveReason::MANUAL,
    };

    let disabled = backend.will_save_wait_until(params.clone()).await.unwrap();
    assert!(
        disabled.is_none(),
        "format-on-save must be inert until the feature toggle is set",
    );

    backend
        .feature_toggles
        .lock()
        .await
        .set
        .insert("willSaveWaitUntil".to_owned(), true);
    let enabled = backend.will_save_wait_until(params).await.unwrap();
    assert!(
        enabled.is_some_and(|edits| !edits.is_empty()),
        "with the toggle on, mis-formatted source must yield edits",
    );
}

/// Build a backend with one document registered, then disable the named
/// `tclLsp.features.*` toggle.  Returns the backend plus the document
/// identifier and a cursor-position params for the request handlers.
async fn backend_with_feature_disabled(
    feature: &str,
) -> (Backend, TextDocumentIdentifier, TextDocumentPositionParams) {
    let backend = test_backend();
    let uri = Uri::from_str("file:///g.tcl").unwrap();
    register(&backend, &uri, "proc helper {} {}\nhelper\n").await;
    backend
        .feature_toggles
        .lock()
        .await
        .apply(serde_json::json!({ feature: false }).as_object().unwrap());
    let td = TextDocumentIdentifier { uri };
    let pos_params = TextDocumentPositionParams {
        text_document: td.clone(),
        position: Position::new(0, 5),
    };
    (backend, td, pos_params)
}

#[tokio::test]
async fn disabled_semantic_tokens_toggle_yields_none() {
    let (backend, td, _) = backend_with_feature_disabled("semanticTokens").await;
    assert!(
        backend
            .semantic_tokens_full(SemanticTokensParams {
                text_document: td,
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("ok")
            .is_none(),
        "semanticTokens disabled must yield None",
    );
}

/// Negative: with the `semanticTokens` toggle left at its default (enabled),
/// the same handler returns a token set — proving the gate, not the handler,
/// is what suppresses the result above.
#[tokio::test]
async fn enabled_semantic_tokens_toggle_yields_some() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///g.tcl").unwrap();
    register(&backend, &uri, "proc helper {} {}\nhelper\n").await;
    assert!(
        backend
            .semantic_tokens_full(SemanticTokensParams {
                text_document: TextDocumentIdentifier { uri },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("ok")
            .is_some(),
        "semanticTokens enabled must yield a token set",
    );
}

#[tokio::test]
async fn disabled_code_actions_toggle_yields_none() {
    let (backend, td, _) = backend_with_feature_disabled("codeActions").await;
    assert!(
        backend
            .code_action(CodeActionParams {
                text_document: td,
                range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                context: tower_lsp_server::ls_types::CodeActionContext::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("ok")
            .is_none(),
        "codeActions disabled must yield None",
    );
}

#[tokio::test]
async fn disabled_rename_toggle_yields_none() {
    let (backend, _, pos_params) = backend_with_feature_disabled("rename").await;
    assert!(
        backend
            .rename(RenameParams {
                text_document_position: pos_params,
                new_name: "do_it".to_owned(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("ok")
            .is_none(),
        "rename disabled must yield None",
    );
}

#[tokio::test]
async fn disabled_document_highlight_toggle_yields_none() {
    let (backend, _, pos_params) = backend_with_feature_disabled("documentHighlight").await;
    assert!(
        backend
            .document_highlight(DocumentHighlightParams {
                text_document_position_params: pos_params,
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("ok")
            .is_none(),
        "documentHighlight disabled must yield None",
    );
}

#[tokio::test]
async fn disabled_code_lens_toggle_yields_none() {
    let (backend, td, _) = backend_with_feature_disabled("codeLens").await;
    assert!(
        backend
            .code_lens(CodeLensParams {
                text_document: td,
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("ok")
            .is_none(),
        "codeLens disabled must yield None",
    );
}

#[tokio::test]
async fn disabled_linked_editing_range_toggle_yields_none() {
    let (backend, _, pos_params) = backend_with_feature_disabled("linkedEditingRange").await;
    assert!(
        backend
            .linked_editing_range(LinkedEditingRangeParams {
                text_document_position_params: pos_params,
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("ok")
            .is_none(),
        "linkedEditingRange disabled must yield None",
    );
}

#[tokio::test]
async fn disabled_call_hierarchy_toggle_yields_none() {
    let (backend, _, pos_params) = backend_with_feature_disabled("callHierarchy").await;
    assert!(
        backend
            .prepare_call_hierarchy(CallHierarchyPrepareParams {
                text_document_position_params: pos_params,
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .await
            .expect("ok")
            .is_none(),
        "callHierarchy disabled must yield None",
    );
}

#[tokio::test]
async fn disabled_workspace_symbols_toggle_yields_none() {
    let (backend, _, _) = backend_with_feature_disabled("workspaceSymbols").await;
    assert!(
        backend
            .symbol(WorkspaceSymbolParams {
                query: "helper".to_owned(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("ok")
            .is_none(),
        "workspaceSymbols disabled must yield None",
    );
}
