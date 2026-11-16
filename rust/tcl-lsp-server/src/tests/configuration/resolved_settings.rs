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

//! The values one document resolves.

use super::*;

#[tokio::test]
async fn resolved_formatting_applies_through_global_config() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///f.tcl").unwrap();
    backend
        .apply_global_config(&serde_json::json!({
            "formatting": { "indentSize": 8, "maxLineLength": 70 }
        }))
        .await
        .unwrap();
    let formatting = backend.resolved_formatting(&uri).await;
    assert_eq!(formatting["indentSize"], serde_json::json!(8));
    // With insertSpaces and no explicit tabSize override (tab_size: 0 → the
    // config's indentSize is used since LSP clamps 0 away).
    let opts = tower_lsp_server::ls_types::FormattingOptions {
        tab_size: 0,
        insert_spaces: true,
        ..Default::default()
    };
    let cfg = formatter_config_from(&formatting, &opts, tcl_lsp_core::profile_for_dialect("tcl"));
    assert_eq!(cfg.max_line_length, 70);
    // tab_size 0 → unwrap_or(cfg.indent_size=8).max(1) = 8.
    assert_eq!(cfg.indent_size, 8);
}

#[tokio::test]
async fn resolved_docstring_style_defaults_to_none() {
    // No `docstringStyle` configured at all — falls back to the
    // documented default, matching `FormatterConfig::default()` and
    // every editor catalogue's declared default.
    let backend = test_backend();
    let uri = Uri::from_str("file:///f.tcl").unwrap();
    assert_eq!(
        backend.resolved_docstring_style(&uri).await,
        core_formatting::DocstringStyle::None
    );
}

#[tokio::test]
async fn resolved_docstring_style_reads_the_global_config() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///f.tcl").unwrap();
    backend
        .apply_global_config(&serde_json::json!({
            "formatting": { "docstringStyle": "body" }
        }))
        .await
        .unwrap();
    assert_eq!(
        backend.resolved_docstring_style(&uri).await,
        core_formatting::DocstringStyle::Body
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolved_edit_line_ending_defaults_to_the_documents_own() {
    let backend = test_backend();
    let uri = Uri::from_str("file:///eol.tcl").unwrap();
    // No `lineEnding` configured → `auto` → follow the file.
    assert_eq!(
        backend.resolved_edit_line_ending(&uri, "a\r\nb\r\n").await,
        "\r\n"
    );
    assert_eq!(
        backend.resolved_edit_line_ending(&uri, "a\rb\r").await,
        "\r"
    );
    assert_eq!(
        backend.resolved_edit_line_ending(&uri, "a\nb\n").await,
        "\n"
    );
    assert_eq!(
        backend.resolved_edit_line_ending(&uri, "no breaks").await,
        "\n"
    );
    // An explicit setting still wins.
    backend
        .apply_global_config(&serde_json::json!({
            "formatting": { "lineEnding": "lf" }
        }))
        .await
        .unwrap();
    assert_eq!(
        backend.resolved_edit_line_ending(&uri, "a\r\nb\r\n").await,
        "\n"
    );
}

/// Every `"scope": "resource"` analyser input resolves per folder, not
/// just `packages.provides`. `bigipVersion` and `targets` are prone to the
/// same two holes: no folder override at all, and a folder handle
/// constructed with `None` / `Vec::new()` so *any* folder override of an
/// unrelated knob drops the global value.

#[tokio::test]
pub(super) async fn resource_scoped_analyser_inputs_resolve_per_folder() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({
            "bigipVersion": "17.1.0",
            "targets": { "tcl": "8.5-9.0" },
            "packages": { "provides": { "globalExt": ["Tk"] } },
        }))
        .await
        .unwrap();

    let root = Uri::from_str("file:///w/").unwrap();
    let other = Uri::from_str("file:///other/").unwrap();
    backend
        .apply_folder_configs(vec![
            (
                root.clone(),
                parse_folder_config(&serde_json::json!({
                    "bigipVersion": "16.1.0",
                    "targets": { "tcl": "8.6-8.6" },
                }))
                .expect("folder config parses"),
            ),
            (
                other.clone(),
                // Overrides an unrelated knob only: must still inherit
                // every global resource-scoped input.
                parse_folder_config(&serde_json::json!({ "extraCommands": ["zzz"] }))
                    .expect("folder config parses"),
            ),
        ])
        .await;

    let in_root = backend
        .resource_analyser_inputs(Some(&Uri::from_str("file:///w/a.tcl").unwrap()))
        .await;
    assert_eq!(in_root.bigip_version.as_deref(), Some("16.1.0"));
    // An explicit empty string at folder scope *clears* the pin rather
    // than inheriting the global one — the three-state distinction
    // `BigipVersionSetting` exists for.
    assert_eq!(
        parse_bigip_version(&serde_json::json!({ "bigipVersion": "" })),
        BigipVersionSetting::Present(None)
    );
    assert_eq!(
        parse_bigip_version(&serde_json::json!({})),
        BigipVersionSetting::Absent
    );
    assert_eq!(
        in_root.declared_targets,
        vec![("tcl".to_owned(), "8.6-8.6".to_owned())]
    );
    assert_eq!(
        in_root.package_provides,
        vec![("globalExt".to_owned(), vec!["Tk".to_owned()])],
        "a field the folder does not override still inherits"
    );

    let in_other = backend
        .resource_analyser_inputs(Some(&Uri::from_str("file:///other/a.tcl").unwrap()))
        .await;
    assert_eq!(in_other.bigip_version.as_deref(), Some("17.1.0"));
    assert_eq!(
        in_other.declared_targets,
        vec![("tcl".to_owned(), "8.5-9.0".to_owned())],
        "a folder overriding something else keeps every global input"
    );

    let nowhere = backend
        .resource_analyser_inputs(Some(&Uri::from_str("file:///nowhere/a.tcl").unwrap()))
        .await;
    assert_eq!(nowhere.bigip_version.as_deref(), Some("17.1.0"));
    assert_eq!(
        nowhere.declared_targets,
        vec![("tcl".to_owned(), "8.5-9.0".to_owned())]
    );

    // No document in hand answers with the globals.
    let global = backend.resource_analyser_inputs(None).await;
    assert_eq!(global.bigip_version.as_deref(), Some("17.1.0"));
}

/// `tclLsp.packages.provides` is
/// `"scope": "resource"`, so a folder override must win for documents
/// under it — and, just as importantly, a folder that overrides some
/// *other* analyser input must not silently lose the global edges (the
/// folder handle must not be constructed with an empty provides list).
#[tokio::test]
async fn declared_package_provides_resolve_per_folder() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({
            "packages": { "provides": { "globalExt": ["Tk"] } },
        }))
        .await
        .unwrap();

    let root = Uri::from_str("file:///w/").unwrap();
    let other = Uri::from_str("file:///other/").unwrap();
    backend
        .apply_folder_configs(vec![
            (
                root.clone(),
                parse_folder_config(&serde_json::json!({
                    "packages": { "provides": { "rootExt": ["Tk"] } },
                }))
                .expect("folder config parses"),
            ),
            (
                other.clone(),
                // Overrides an unrelated knob only: must still inherit the
                // global edges rather than get an empty map.
                parse_folder_config(&serde_json::json!({ "extraCommands": ["zzz"] }))
                    .expect("folder config parses"),
            ),
        ])
        .await;

    let in_root = Uri::from_str("file:///w/a.tcl").unwrap();
    assert_eq!(
        backend
            .resource_analyser_inputs(Some(&in_root))
            .await
            .package_provides,
        vec![("rootExt".to_owned(), vec!["Tk".to_owned()])],
        "a folder override wins for documents under it"
    );

    let in_other = Uri::from_str("file:///other/a.tcl").unwrap();
    assert_eq!(
        backend
            .resource_analyser_inputs(Some(&in_other))
            .await
            .package_provides,
        vec![("globalExt".to_owned(), vec!["Tk".to_owned()])],
        "a folder overriding something else still inherits the global edges"
    );

    let elsewhere = Uri::from_str("file:///nowhere/a.tcl").unwrap();
    assert_eq!(
        backend
            .resource_analyser_inputs(Some(&elsewhere))
            .await
            .package_provides,
        vec![("globalExt".to_owned(), vec!["Tk".to_owned()])],
        "no folder match falls back to the global map"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resolved_analysis_settings_falls_back_to_global_defaults() {
    // With no folder config registered, every knob resolves from the
    // backend's global state, and the session's layers decide the
    // analyser's skip and the optimiser: the per-code disable lands in the
    // skip, and the master switch and the per-code override in the policy.
    let backend = test_backend();
    *backend.non_ascii_mode.lock().await = NonAsciiMode::Strict;
    backend
        .apply_global_config(&serde_json::json!({
            "diagnostics": { "W211": false },
            "optimiser": { "enabled": false, "O100": false }
        }))
        .await
        .unwrap();
    let uri = Uri::from_str("file:///settings.tcl").unwrap();
    let (disabled, non_ascii) = backend.resolved_analysis_settings(&uri).await;
    assert!(
        disabled.contains("W211"),
        "global disabled set should apply"
    );
    assert_eq!(non_ascii, NonAsciiMode::Strict);
    let policy = backend.resolved_policy_layers(&uri).await.builder().build();
    assert!(
        !policy.optimiser.enabled,
        "global optimiser switch should apply"
    );
    assert!(
        policy.optimiser.disabled.contains(&DiagCode::O100),
        "a force-disable per-code override should land in the policy",
    );
}

/// The three `FolderGenericPatterns` states each resolve as documented, and
/// a doc outside every folder falls back to the global value. Pins the
/// 3-state replacement for the former `Option<Option<Vec<String>>>`.
#[tokio::test]
async fn resolved_generic_variable_patterns_three_state() {
    let backend = test_backend();
    let folder_replace = Uri::from_str("file:///replace").unwrap();
    let folder_builtin = Uri::from_str("file:///builtin").unwrap();
    let folder_inherit = Uri::from_str("file:///inherit").unwrap();
    *backend.workspace_folders.lock().await = vec![
        folder_replace.clone(),
        folder_builtin.clone(),
        folder_inherit.clone(),
    ];
    // Global supplies a list, so an inheriting folder (and a doc outside any
    // folder) must observe it — distinguishing `Inherit` from
    // `BuiltinDefaults` (which forces the built-in set, i.e. `None`).
    *backend.generic_variable_patterns.lock().await = Some(vec!["^global_".to_owned()]);

    backend
        .apply_folder_configs(vec![
            (
                folder_replace.clone(),
                FolderConfig {
                    generic_variable_patterns: FolderGenericPatterns::Replace(vec![
                        "^repl_".to_owned(),
                    ]),
                    ..FolderConfig::default()
                },
            ),
            (
                folder_builtin.clone(),
                FolderConfig {
                    generic_variable_patterns: FolderGenericPatterns::BuiltinDefaults,
                    ..FolderConfig::default()
                },
            ),
            (
                folder_inherit.clone(),
                FolderConfig {
                    // `Inherit` is the default; set another field so the
                    // folder still parses as a real (non-empty) override.
                    style_line_length: Some(120),
                    ..FolderConfig::default()
                },
            ),
        ])
        .await;

    // Positive: a folder that replaces the patterns gets exactly its list.
    let in_replace = Uri::from_str("file:///replace/a.tcl").unwrap();
    assert_eq!(
        backend
            .resolved_generic_variable_patterns(&in_replace)
            .await,
        Some(vec!["^repl_".to_owned()]),
        "Replace yields the folder's own list",
    );
    // Negative (vs Inherit): BuiltinDefaults forces the analyser built-ins
    // (`None`) even though the global is set.
    let in_builtin = Uri::from_str("file:///builtin/a.tcl").unwrap();
    assert_eq!(
        backend
            .resolved_generic_variable_patterns(&in_builtin)
            .await,
        None,
        "BuiltinDefaults forces the built-in set, ignoring the global list",
    );
    // Inherit folder observes the global list.
    let in_inherit = Uri::from_str("file:///inherit/a.tcl").unwrap();
    assert_eq!(
        backend
            .resolved_generic_variable_patterns(&in_inherit)
            .await,
        Some(vec!["^global_".to_owned()]),
        "Inherit falls back to the global patterns",
    );
    // A doc under no folder also inherits the global.
    let outside = Uri::from_str("file:///elsewhere/a.tcl").unwrap();
    assert_eq!(
        backend.resolved_generic_variable_patterns(&outside).await,
        Some(vec!["^global_".to_owned()]),
        "a doc outside every folder inherits the global patterns",
    );
}
