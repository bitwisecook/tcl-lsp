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

//! Applying a configuration section to the session.

use super::*;

#[tokio::test]
async fn signature_help_disabled_commands_apply_and_clear() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({
            "signatureHelp": { "disabledCommands": [" ::::set ", "incr", "set"] }
        }))
        .await
        .unwrap();
    assert_eq!(
        *backend.signature_help_disabled_commands.lock().await,
        vec!["::incr".to_owned(), "::set".to_owned()],
    );

    // Removing the setting from every merged layer must clear a previously
    // applied list rather than latching stale suppression until restart.
    backend
        .apply_global_config(&serde_json::json!({}))
        .await
        .unwrap();
    assert!(
        backend
            .signature_help_disabled_commands
            .lock()
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn generic_variable_patterns_apply_through_global_config() {
    let backend = test_backend();
    // Default: unset (built-in IRULE4002 pattern set).
    assert!(backend.generic_variable_patterns.lock().await.is_none());
    // `tclLsp.diagnostics.genericVariablePatterns` replaces the default set.
    backend
        .apply_global_config(&serde_json::json!({
            "diagnostics": { "genericVariablePatterns": ["^myapp_"] }
        }))
        .await
        .unwrap();
    assert_eq!(
        *backend.generic_variable_patterns.lock().await,
        Some(vec!["^myapp_".to_owned()]),
    );
    // The salsa `AnalyserConfig` input mirrors the value.
    {
        let db = backend.db.lock().await;
        let cfg = *backend.db_config.lock().await;
        assert_eq!(
            cfg.generic_variable_patterns(&*db).as_deref(),
            Some(["^myapp_".to_owned()].as_slice()),
        );
    }
    // An explicit empty list disables the check (Some(empty), not None).
    backend
        .apply_global_config(&serde_json::json!({
            "diagnostics": { "genericVariablePatterns": [] }
        }))
        .await
        .unwrap();
    assert_eq!(
        *backend.generic_variable_patterns.lock().await,
        Some(Vec::new()),
    );
}

#[tokio::test]
async fn config_file_settings_apply_through_global_config() {
    // A `config.ini`-shaped file, read and applied through the same path as
    // editor settings, takes effect.
    let backend = test_backend();
    let ws = TmpWs::new("cfgfile");
    let path = ws.0.join("config.ini");
    std::fs::write(
        &path,
        "[global]\ndialect = tcl9.0\n[optimiser]\nenabled = false\n",
    )
    .unwrap();
    let global = read_ini_layer(&vfs::NativeStore, Some(path), config_ini::Layer::Global);
    backend.apply_global_config(&global).await.unwrap();
    assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
    assert!(!*backend.optimiser_enabled.lock().await);
}

#[tokio::test]
async fn config_precedence_project_over_editor_over_global() {
    // global config.ini < editor < project .tcl-lsp.ini.
    let backend = test_backend();
    let global =
        config_ini::settings_from_ini("[global]\ndialect = tcl8.5\n", config_ini::Layer::Global);
    let editor = serde_json::json!({ "dialect": "tcl8.6" });
    let project =
        config_ini::settings_from_ini("[project]\ndialect = tcl9.0\n", config_ini::Layer::Project);
    let merged =
        config_ini::merge_settings(&config_ini::merge_settings(&global, &editor), &project);
    backend.apply_global_config(&merged).await.unwrap();
    assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
}

/// The off-lock project read may finish after
/// a newer publication has retired one of its handles. Its stale result
/// must not restore evidence onto that tombstone.
async fn salsa_script_encoding(backend: &Backend) -> tcl_registry::value_transfer::ScriptEncoding {
    let config = *backend.db_config.lock().await;
    let db = backend.db.lock().await;
    config.script_encoding(&*db)
}

#[tokio::test]
async fn source_encoding_reaches_the_session_and_the_salsa_config_2129() {
    use tcl_registry::value_transfer::ScriptEncoding;
    let backend = test_backend();
    let declared = |backend: &Backend| backend.declared_script_encoding();
    assert_eq!(declared(&backend), ScriptEncoding::Auto);
    let latin1 = ScriptEncoding::parse("iso8859-1").expect("a script encoding");
    backend
        .apply_global_config(&serde_json::json!({ "sourceEncoding": "iso8859-1" }))
        .await
        .unwrap();
    assert_eq!(declared(&backend), latin1);
    assert_eq!(salsa_script_encoding(&backend).await, latin1);
    backend
        .apply_global_config(&serde_json::json!({ "sourceEncoding": "koi8-r" }))
        .await
        .unwrap();
    assert_eq!(declared(&backend), latin1, "an unknown name is ignored");
    backend
        .apply_global_config(&serde_json::json!({ "sourceEncoding": "auto" }))
        .await
        .unwrap();
    assert_eq!(declared(&backend), ScriptEncoding::Auto);
    assert_eq!(salsa_script_encoding(&backend).await, ScriptEncoding::Auto);
}

/// The test seam `apply_global_config` fills the session's editor layer,
/// and the session's analyser skip follows it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_global_config_populates_the_editor_layer() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({ "diagnostics": { "W112": false } }))
        .await
        .unwrap();
    let uri = Uri::from_str("file:///any/e.tcl").unwrap();
    assert_eq!(
        backend
            .resolved_policy_layers(&uri)
            .await
            .builder()
            .build()
            .code_reason(DiagCode::W112),
        Some(core_policy::Reason::Disabled(
            core_policy::PolicyLayer::Editor
        )),
    );
    assert!(
        backend.disabled_diagnostics.lock().await.contains("W112"),
        "the session skip holds the editor layer's disable",
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_global_config_applies_every_knob() {
    let backend = test_backend();
    let cfg = serde_json::json!({
        "features": { "hover": false },
        "xcDiagnostics": { "enabled": true },
        "optimiser": { "enabled": false, "profile": "full", "O100": false },
        "formatting": { "lineLength": 120 },
        "dialect": "tcl9.0",
        "style": { "nonAscii": "strict" },
        "diagnostics": { "W211": false },
        "packages": {
            "preferLatest": true,
            "provides": { "myExtension": ["Tk"], "single": "Img" },
        },
        "workspaceScan": { "maxFiles": 6000 },
    });
    backend.apply_global_config(&cfg).await.unwrap();
    assert!(!backend.feature_toggles.lock().await.is_enabled("hover"));
    assert!(
        backend
            .feature_toggles
            .lock()
            .await
            .is_enabled("xcDiagnostics"),
        "xcDiagnostics section flag should map onto the toggle",
    );
    assert!(!*backend.optimiser_enabled.lock().await);
    let policy = backend.policy_layers.lock().await.builder().build();
    assert!(
        policy.optimiser.disabled.contains(&DiagCode::O100),
        "optimiser.O100=false should record a force-disable override",
    );
    assert_eq!(*backend.line_length.lock().await, 120);
    assert_eq!(*backend.workspace_scan_max_files.lock().await, 6000);
    assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
    assert_eq!(*backend.non_ascii_mode.lock().await, NonAsciiMode::Strict);
    assert!(backend.disabled_diagnostics.lock().await.contains("W211"));
    // The interpreter's starting `package prefer`
    // mode is a setting, since neither `TCL_PKG_PREFER_LATEST` nor an
    // unstable 9.0+ build is visible in the source tree.
    assert_eq!(
        backend.default_package_prefer().await,
        tcl_lsp_core::package_resolver::PackagePrefer::Latest,
    );
    // TN control: a fresh backend starts at the interpreter default.
    assert_eq!(
        test_backend().default_package_prefer().await,
        tcl_lsp_core::package_resolver::PackagePrefer::Stable,
    );
    // Declared "this package also loads that one" edges,
    // in a stable order and accepting a bare string for a single name.
    assert_eq!(
        *backend.package_provides.lock().await,
        vec![
            ("myExtension".to_owned(), vec!["Tk".to_owned()]),
            ("single".to_owned(), vec!["Img".to_owned()]),
        ],
    );
}

/// Issue #2021: the workspace scan's file budget is a setting, and a
/// session that configures nothing keeps the built-in 2000 — the value the
/// XDG `config.ini` / editor / project layers all start from.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_scan_budget_defaults_to_the_built_in_cap() {
    let backend = test_backend();
    assert_eq!(
        *backend.workspace_scan_max_files.lock().await,
        WORKSPACE_SCAN_FILE_CAP,
    );
    assert_eq!(WORKSPACE_SCAN_FILE_CAP, 2000);
    // An empty config file layer — what an absent `config.ini` parses to —
    // carries no key, so the merged config leaves the default alone.
    let empty = config_ini::settings_from_ini("", config_ini::Layer::Global);
    backend.apply_global_config(&empty).await.unwrap();
    assert_eq!(*backend.workspace_scan_max_files.lock().await, 2000);
    let reported = backend
        .get_effective_config_command(&[serde_json::json!("file:///scan.tcl")])
        .await
        .expect("effective config")
        .expect("config payload");
    assert_eq!(
        reported["workspace_scan_max_files"],
        serde_json::json!(2000)
    );
}

/// The layered path a real session takes: the project `.tcl-lsp.ini` wins
/// over the editor's value, and the merged result is what the scan runs
/// under (issue #2021).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_scan_budget_takes_the_project_ini_over_the_editor() {
    let backend = test_backend();
    let global = config_ini::settings_from_ini(
        "[workspaceScan]\nmax_files = 100\n",
        config_ini::Layer::Global,
    );
    let editor = serde_json::json!({ "workspaceScan": { "maxFiles": 500 } });
    let project = config_ini::settings_from_ini(
        "[workspaceScan]\nmax_files = 9000\n",
        config_ini::Layer::Project,
    );
    let merged =
        config_ini::merge_settings(&config_ini::merge_settings(&global, &editor), &project);
    backend.apply_global_config(&merged).await.unwrap();
    assert_eq!(*backend.workspace_scan_max_files.lock().await, 9000);
    let reported = backend
        .get_effective_config_command(&[serde_json::json!("file:///scan.tcl")])
        .await
        .expect("effective config")
        .expect("config payload");
    assert_eq!(
        reported["workspace_scan_max_files"],
        serde_json::json!(9000)
    );
}

/// A zero or negative budget would mean "index nothing", which nobody
/// setting a file budget means; the editor schemas declare `minimum: 1`
/// and the server ignores anything below it rather than obeying it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_scan_budget_ignores_a_zero_or_junk_value() {
    let backend = test_backend();
    for cfg in [
        serde_json::json!({ "workspaceScan": { "maxFiles": 0 } }),
        serde_json::json!({ "workspaceScan": { "maxFiles": -5 } }),
        serde_json::json!({ "workspaceScan": { "maxFiles": "lots" } }),
        serde_json::json!({ "workspaceScan": {} }),
    ] {
        backend.apply_global_config(&cfg).await.unwrap();
        assert_eq!(
            *backend.workspace_scan_max_files.lock().await,
            WORKSPACE_SCAN_FILE_CAP,
            "{cfg} must leave the default budget in place",
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_global_config_ignores_non_object() {
    // A non-object config (e.g. a `null` from an editor with no settings)
    // is a no-op — the defaults survive.
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::Value::Null)
        .await
        .unwrap();
    assert_eq!(*backend.default_dialect.lock().await, "tcl8.6");
}
