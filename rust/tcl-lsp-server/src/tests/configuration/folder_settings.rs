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

//! Per-folder configuration: parse, isolation and analyser handles.

use super::*;

#[test]
fn folder_config_parses_entry_points() {
    let cfg = serde_json::json!({
        "entryPoints": ["main.tcl", "src/app.tcl"],
    });
    let fc = parse_folder_config(&cfg).unwrap();
    assert_eq!(
        fc.entry_points,
        Some(vec!["main.tcl".to_owned(), "src/app.tcl".to_owned()])
    );
}

#[test]
fn folder_config_parses_the_iruleslx_plugin_mapping() {
    let fc = parse_folder_config(&serde_json::json!({
        "iruleslx": {
            "plugins": { "prod": "workspaces/ws_alpha", "other": "/abs/ws" },
            "rules": { "prod": ["irules/http", "irules/tcp"] },
        }
    }))
    .expect("folder config");
    // Sorted by plugin name: a JSON object has no guaranteed order, and an
    // unstable candidate list would make an ambiguity report unstable too.
    assert_eq!(
        fc.iruleslx,
        vec![
            IlxPluginSpec {
                plugin: "other".to_owned(),
                workspace: "/abs/ws".to_owned(),
                extra_rule_dirs: Vec::new(),
            },
            IlxPluginSpec {
                plugin: "prod".to_owned(),
                workspace: "workspaces/ws_alpha".to_owned(),
                extra_rule_dirs: vec!["irules/http".to_owned(), "irules/tcp".to_owned()],
            },
        ]
    );
}

#[test]
fn an_iruleslx_rules_entry_without_a_plugin_entry_is_dropped() {
    // Extra caller directories are only meaningful once the plugin's own
    // workspace is known. Keeping the half-declaration would widen the
    // search for a plugin that still resolves by the directory-name
    // convention — an association the user never made.
    let fc = parse_folder_config(&serde_json::json!({
        "iruleslx": { "rules": { "prod": ["irules"] } }
    }))
    .expect("folder config");
    assert_eq!(fc.iruleslx, [] as [IlxPluginSpec; 0]);
    // …and an empty workspace path is not a declaration either.
    let blank = parse_folder_config(&serde_json::json!({
        "iruleslx": { "plugins": { "prod": "  " } }
    }))
    .expect("folder config");
    assert_eq!(blank.iruleslx, [] as [IlxPluginSpec; 0]);
}

#[test]
fn iruleslx_paths_resolve_against_the_folder_root() {
    let specs = vec![IlxPluginSpec {
        plugin: "prod".to_owned(),
        workspace: "../shared/ws".to_owned(),
        extra_rule_dirs: vec!["irules".to_owned(), "/abs/elsewhere".to_owned()],
    }];
    let roots = resolve_ilx_specs(&specs, Path::new("/repo/app"));
    assert_eq!(roots.len(), 1);
    // Lexically normalised, so this compares equal to the same directory
    // reached from a document's own ancestors.
    assert_eq!(roots[0].workspace, Path::new("/repo/shared/ws"));
    assert_eq!(
        roots[0].extra_rule_dirs,
        vec![
            PathBuf::from("/repo/app/irules"),
            PathBuf::from("/abs/elsewhere"),
        ]
    );
}

/// A configured folder whose resolved analyser inputs equal the session's
/// shares the session's `AnalyserConfig` handle, and so its memo: every
/// configured folder carries a skip of its own, and a handle per folder
/// would analyse each of its documents twice per revision.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_folder_matching_the_session_shares_its_analyser_handle() {
    let backend = test_backend();
    let folder = Uri::from_str("file:///proj-shared").unwrap();
    *backend.workspace_folders.lock().await = vec![folder.clone()];
    let session_skip = backend.disabled_diagnostics.lock().await.clone();
    backend
        .apply_folder_configs(vec![(
            folder.clone(),
            FolderConfig {
                disabled_diagnostics: Some(session_skip.clone()),
                ..FolderConfig::default()
            },
        )])
        .await;
    assert!(
        backend.folder_db_configs.lock().await.is_empty(),
        "a folder resolving to the session's inputs shares its handle",
    );

    let mut own = session_skip;
    own.insert("W100".to_owned());
    backend
        .apply_folder_configs(vec![(
            folder.clone(),
            FolderConfig {
                disabled_diagnostics: Some(own),
                ..FolderConfig::default()
            },
        )])
        .await;
    let handles = backend.folder_db_configs.lock().await;
    assert_eq!(handles.len(), 1, "a differing skip needs its own handle");
    assert_eq!(handles[0].0, folder);
}

/// A configured folder's documents resolve their policy from the folder's
/// own three layers; a folder with none of its own resolves under the
/// session's.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_configured_folder_resolves_its_own_three_layers() {
    let backend = test_backend();
    let own = Uri::from_str("file:///proj-own").unwrap();
    let bare = Uri::from_str("file:///proj-bare").unwrap();
    *backend.folder_configs.lock().await = vec![
        (
            own.clone(),
            FolderConfig {
                policy_layers: Some(PolicyLayers {
                    project: serde_json::json!({ "diagnostics": { "W112": false } }),
                    ..PolicyLayers::default()
                }),
                ..FolderConfig::default()
            },
        ),
        (
            bare.clone(),
            FolderConfig {
                policy_layers: None,
                ..FolderConfig::default()
            },
        ),
    ];
    let under_own = Uri::from_str("file:///proj-own/a.tcl").unwrap();
    let under_bare = Uri::from_str("file:///proj-bare/b.tcl").unwrap();
    assert_eq!(
        backend
            .resolved_policy_layers(&under_own)
            .await
            .builder()
            .build()
            .code_reason(DiagCode::W112),
        Some(core_policy::Reason::Disabled(
            core_policy::PolicyLayer::Project
        )),
    );
    assert_eq!(
        backend
            .resolved_policy_layers(&under_bare)
            .await
            .builder()
            .build()
            .code_reason(DiagCode::W112),
        None,
        "a folder with no layers of its own resolves under the session's",
    );
}

/// A secondary root whose own three layers carry no
/// policy section does not inherit the primary root's `.tcl-lsp.ini`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_secondary_root_does_not_inherit_the_primary_project_file() {
    let backend = test_backend();
    backend.policy_layers.lock().await.project =
        serde_json::json!({ "diagnostics": { "W112": false } });
    let secondary = Uri::from_str("file:///proj-secondary").unwrap();
    *backend.folder_configs.lock().await = vec![(
        secondary,
        FolderConfig {
            line_length: Some(100),
            policy_layers: Some(PolicyLayers::default()),
            ..FolderConfig::default()
        },
    )];
    let under_secondary = Uri::from_str("file:///proj-secondary/c.tcl").unwrap();
    let outside = Uri::from_str("file:///elsewhere/d.tcl").unwrap();
    assert_eq!(
        backend
            .resolved_policy_layers(&under_secondary)
            .await
            .builder()
            .build()
            .code_reason(DiagCode::W112),
        None,
        "the secondary root resolves under its own layers only",
    );
    assert_eq!(
        backend
            .resolved_policy_layers(&outside)
            .await
            .builder()
            .build()
            .code_reason(DiagCode::W112),
        Some(core_policy::Reason::Disabled(
            core_policy::PolicyLayer::Project
        )),
    );
}

/// A folder whose override set empties retires its `AnalyserConfig`
/// handle (payload cleared) and revives it when the override comes back —
/// `.tcl-lsp.ini` churn must not allocate a config input per save.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn folder_db_config_handle_is_retired_and_revived() {
    let backend = test_backend();
    let folder = Uri::from_str("file:///proj-a").unwrap();
    *backend.workspace_folders.lock().await = vec![folder.clone()];
    let overriding = || FolderConfig {
        disabled_diagnostics: Some(std::iter::once("W100".to_owned()).collect()),
        ..FolderConfig::default()
    };

    backend
        .apply_folder_configs(vec![(folder.clone(), overriding())])
        .await;
    let first = backend
        .folder_db_configs
        .lock()
        .await
        .iter()
        .find(|(u, _)| u == &folder)
        .map(|(_, handle)| *handle)
        .expect("an overriding folder gets its own AnalyserConfig handle");

    // The next re-pull drops the override, so the folder needs no handle.
    backend
        .apply_folder_configs(vec![(folder.clone(), FolderConfig::default())])
        .await;
    assert!(
        backend.folder_db_configs.lock().await.is_empty(),
        "a folder with no overrides must not keep a live handle",
    );
    {
        let db = backend.db.lock().await;
        let tombstones = backend.folder_db_config_tombstones.lock().await;
        let retired = *tombstones
            .get(&folder)
            .expect("the handle must be retired, not dropped");
        assert!(retired == first, "the retired handle is the original input");
        assert!(
            retired.disabled_diagnostics(&*db).is_empty(),
            "a retired config must release its code list",
        );
    }

    // Re-enabling the override revives the retired handle.
    backend
        .apply_folder_configs(vec![(folder.clone(), overriding())])
        .await;
    let second = backend
        .folder_db_configs
        .lock()
        .await
        .iter()
        .find(|(u, _)| u == &folder)
        .map(|(_, handle)| *handle)
        .expect("the re-enabled folder has a handle again");
    assert!(
        first == second,
        "re-enabling an override must revive the retired config input",
    );
    assert!(
        backend.folder_db_config_tombstones.lock().await.is_empty(),
        "a revived config must leave the tombstone map",
    );
}

/// `apply` is the single place a resource-scoped input reaches an
/// `Analyser`, so every construction site carries all of them. Pins that
/// the bundle is applied whole rather than field by field.
#[test]
fn resource_analyser_inputs_apply_every_field() {
    let inputs = ResourceAnalyserInputs {
        package_provides: vec![("ext".to_owned(), vec!["Tk".to_owned()])],
        bigip_version: Some("17.1.0".to_owned()),
        declared_targets: vec![("tcl".to_owned(), "8.6-9.0".to_owned())],
        script_encoding: tcl_registry::value_transfer::ScriptEncoding::parse("iso8859-1")
            .expect("a script encoding"),
    };
    let analyser = inputs.clone().apply(Analyser::new());
    assert_eq!(analyser.script_encoding, inputs.script_encoding);
    // `package_provides` and `declared_targets` are public on the analyser
    // and asserted directly. `bigip_version` is private, so the field that
    // feeds it is pinned instead by
    // `resource_scoped_analyser_inputs_resolve_per_folder`, which asserts
    // the resolved bundle carries it; `apply` hands over all three
    // together or none.
    assert_eq!(
        analyser.package_provides,
        vec![("ext".to_owned(), vec!["Tk".to_owned()])]
    );
    assert_eq!(
        analyser.declared_targets,
        vec![("tcl".to_owned(), "8.6-9.0".to_owned())]
    );
}

/// A per-folder editor config overrides the process-global settings for
/// documents under that folder, while documents under other folders keep
/// the global values (longest-prefix resolution).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn per_folder_config_overrides_global_settings() {
    let backend = test_backend();
    let folder_a = Uri::from_str("file:///proj-a").unwrap();
    let folder_b = Uri::from_str("file:///proj-b").unwrap();
    let inside = Uri::from_str("file:///proj-a/sub/file.tcl").unwrap();
    let outside = Uri::from_str("file:///proj-b/file.tcl").unwrap();
    *backend.workspace_folders.lock().await = vec![folder_a.clone(), folder_b.clone()];

    // proj-a turns hover off and disables W100; the global state keeps both
    // on / empty.
    let feature_toggles = {
        let mut t = FeatureToggles::default();
        t.set.insert("hover".to_owned(), false);
        t
    };
    let fc = FolderConfig {
        feature_toggles,
        disabled_diagnostics: Some(std::iter::once("W100".to_owned()).collect()),
        ..FolderConfig::default()
    };
    backend
        .apply_folder_configs(vec![(folder_a.clone(), fc)])
        .await;

    // Feature gating resolves per folder.
    assert!(
        !backend.feature_enabled("hover", &inside).await,
        "hover must be off under proj-a",
    );
    assert!(
        backend.feature_enabled("hover", &outside).await,
        "hover must stay on under proj-b (global default)",
    );

    // Disabled-diagnostics resolve per folder.
    let (disabled_in, ..) = backend.resolved_analysis_settings(&inside).await;
    assert!(disabled_in.contains("W100"), "proj-a disables W100");
    let (disabled_out, ..) = backend.resolved_analysis_settings(&outside).await;
    assert!(
        !disabled_out.contains("W100"),
        "proj-b inherits the empty global disabled set",
    );

    // A folder that overrides the disabled set gets its own salsa config
    // handle, so the suppression also reaches the cached analysis.
    assert!(
        backend
            .folder_db_configs
            .lock()
            .await
            .iter()
            .any(|(u, _)| u == &folder_a),
        "proj-a must have a per-folder AnalyserConfig handle",
    );
}

/// The folder-scoped `tclLsp.dialect` must survive the parse.  Without a
/// `FolderConfig` dialect field to land in, the scoped
/// `workspace/configuration` reply's value is silently dropped and every
/// folder uses the session default.
#[test]
fn parse_folder_config_reads_and_validates_the_dialect() {
    let fc =
        parse_folder_config(&serde_json::json!({ "dialect": "f5-irules" })).expect("folder config");
    assert_eq!(fc.dialect.as_deref(), Some("f5-irules"));
    // A config-only profile (not a `SpecSurface`-parseable Tcl dialect) is
    // accepted, matching what `initializationOptions.folderDialects` takes.
    let bigip =
        parse_folder_config(&serde_json::json!({ "dialect": "f5-bigip" })).expect("folder cfg");
    assert_eq!(bigip.dialect.as_deref(), Some("f5-bigip"));
    // An unknown name is dropped rather than pinning the folder to a
    // dialect no provider can resolve.
    let bogus =
        parse_folder_config(&serde_json::json!({ "dialect": "klingon" })).expect("folder cfg");
    assert_eq!(bogus.dialect, None);
    // An absent key inherits the session default.
    let absent =
        parse_folder_config(&serde_json::json!({ "extraCommands": [] })).expect("folder cfg");
    assert_eq!(absent.dialect, None);
}

#[test]
fn parse_folder_config_reads_extra_library_and_generic_patterns() {
    let cfg = serde_json::json!({
        "extraCommands": ["mylib::send", "mylib::recv"],
        "signatureHelp": { "disabledCommands": [" ::::set ", "incr", "set"] },
        "libraryPaths": ["/proj/lib"],
        "diagnostics": { "genericVariablePatterns": ["^proj_"] },
    });
    let fc = parse_folder_config(&cfg).expect("folder config");
    assert_eq!(
        fc.extra_commands.as_deref(),
        Some(["mylib::send".to_owned(), "mylib::recv".to_owned()].as_slice()),
    );
    assert_eq!(
        fc.library_paths.as_deref(),
        Some(["/proj/lib".to_owned()].as_slice())
    );
    assert_eq!(
        fc.signature_help_disabled_commands.as_deref(),
        Some(["::incr".to_owned(), "::set".to_owned()].as_slice()),
        "command names use the shared Tcl qualified-name canonicalisation",
    );
    assert_eq!(
        fc.generic_variable_patterns,
        FolderGenericPatterns::Replace(vec!["^proj_".to_owned()]),
    );
}

/// An explicit `genericVariablePatterns: null` selects the analyser's
/// built-in defaults (`BuiltinDefaults`), distinct from an absent key
/// (`Inherit`) and from a present array (`Replace`). Pins the third state.
#[test]
fn parse_folder_config_generic_patterns_null_is_builtin_defaults() {
    let with_null = serde_json::json!({
        "diagnostics": { "genericVariablePatterns": null },
    });
    let fc = parse_folder_config(&with_null).expect("folder config");
    assert_eq!(
        fc.generic_variable_patterns,
        FolderGenericPatterns::BuiltinDefaults,
        "null requests the built-in defaults",
    );

    // Negative: an absent key leaves the default `Inherit` (no override).
    let absent = serde_json::json!({ "extraCommands": ["x"] });
    let fc_absent = parse_folder_config(&absent).expect("folder config");
    assert_eq!(
        fc_absent.generic_variable_patterns,
        FolderGenericPatterns::Inherit,
        "an absent key inherits the global value",
    );
}

#[tokio::test]
async fn per_folder_extra_commands_and_patterns_are_isolated() {
    let backend = test_backend();
    let folder_a = Uri::from_str("file:///proj-a").unwrap();
    let folder_b = Uri::from_str("file:///proj-b").unwrap();
    let inside = Uri::from_str("file:///proj-a/sub/file.tcl").unwrap();
    let outside = Uri::from_str("file:///proj-b/file.tcl").unwrap();
    *backend.workspace_folders.lock().await = vec![folder_a.clone(), folder_b.clone()];

    // Global sets one extra command; proj-a overrides with its own.
    *backend.extra_commands.lock().await = vec!["globalcmd".to_owned()];
    *backend.signature_help_disabled_commands.lock().await = vec!["::set".to_owned()];
    let fc = FolderConfig {
        extra_commands: Some(vec!["projacmd".to_owned()]),
        signature_help_disabled_commands: Some(vec!["::incr".to_owned()]),
        generic_variable_patterns: FolderGenericPatterns::Replace(vec!["^proja_".to_owned()]),
        ..FolderConfig::default()
    };
    backend
        .apply_folder_configs(vec![(folder_a.clone(), fc)])
        .await;

    // extraCommands resolve per folder.
    assert_eq!(
        backend.resolved_extra_commands(&inside).await,
        vec!["projacmd".to_owned()],
        "proj-a uses its own extraCommands",
    );
    assert_eq!(
        backend.resolved_extra_commands(&outside).await,
        vec!["globalcmd".to_owned()],
        "proj-b inherits the global extraCommands",
    );
    assert_eq!(
        backend
            .resolved_signature_help_disabled_commands(&inside)
            .await,
        vec!["::incr".to_owned()],
        "proj-a replaces the global signature-help exclusion list",
    );
    assert_eq!(
        backend
            .resolved_signature_help_disabled_commands(&outside)
            .await,
        vec!["::set".to_owned()],
        "proj-b inherits the global signature-help exclusion list",
    );
    // genericVariablePatterns resolve per folder.
    assert_eq!(
        backend.resolved_generic_variable_patterns(&inside).await,
        Some(vec!["^proja_".to_owned()]),
    );
    assert_eq!(
        backend.resolved_generic_variable_patterns(&outside).await,
        None,
        "proj-b inherits the global (default) patterns",
    );
    // proj-a gets a salsa handle (it overrides analyser-config inputs).
    assert!(
        backend
            .folder_db_configs
            .lock()
            .await
            .iter()
            .any(|(u, _)| u == &folder_a),
        "proj-a must have a per-folder AnalyserConfig handle for extraCommands",
    );
}
