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

//! The effective configuration and its INI export.

use super::*;

#[tokio::test]
async fn render_config_ini_emits_known_sections() {
    let backend = test_backend();
    let ini = backend.render_config_ini().await;
    for section in ["[features]", "[optimiser]", "[style]"] {
        assert!(ini.contains(section), "missing {section} in:\n{ini}");
    }
}

#[test]
fn non_ascii_mode_str_renders_null_for_default() {
    assert!(non_ascii_mode_str(NonAsciiMode::Default).is_null());
    assert_eq!(non_ascii_mode_str(NonAsciiMode::Off), "off");
    assert_eq!(non_ascii_mode_str(NonAsciiMode::Strict), "strict");
    assert_eq!(non_ascii_mode_str(NonAsciiMode::Common), "common");
    assert_eq!(non_ascii_mode_str(NonAsciiMode::Confusables), "confusables");
}

#[test]
fn config_ini_path_honours_xdg() {
    // `temp_env` sets + restores `XDG_CONFIG_HOME` and serialises against
    // other env-touching tests via its own global lock, so this crate stays
    // unsafe-free (`set_var` is `unsafe` on edition 2024).
    temp_env::with_var("XDG_CONFIG_HOME", Some("/tmp/xdg-probe"), || {
        assert_eq!(
            config_ini_path(),
            std::path::PathBuf::from("/tmp/xdg-probe/tcl-lsp/config.ini")
        );
    });
}

/// `getEffectiveConfig` labels come from the resolved environment, so `tk`,
/// `jim` and the lenient `tcl` carry them too; only a name that names no
/// environment reports `null`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn effective_config_labels_come_from_the_resolved_environment() {
    let backend = test_backend();
    let mut seen = Vec::new();
    for (dialect, display, short, kind, provenance) in [
        ("jim", "Jim Tcl", "Jim", "language", "built-in"),
        ("tk", "Tk", "Tk", "packages", "built-in"),
        ("tcl", "Tcl", "Tcl", "language", "built-in"),
        (
            "vivado",
            "Xilinx Vivado",
            "Vivado",
            "packages",
            "bundled-pack",
        ),
    ] {
        *backend.default_dialect.lock().await = dialect.to_owned();
        let config = backend
            .get_effective_config_command(&[])
            .await
            .expect("effective config")
            .expect("config payload");
        assert_eq!(config["dialect"], dialect);
        assert_eq!(config["dialect_display_name"], display, "{dialect}");
        assert_eq!(config["dialect_short_name"], short, "{dialect}");
        assert_eq!(config["dialect_kind"], kind, "{dialect}");
        assert_eq!(config["dialect_provenance"], provenance, "{dialect}");
        let description = config["dialect_description"]
            .as_str()
            .expect("a description");
        assert!(description.contains(display), "{dialect}: {description}");
        seen.push(config["dialect_id"].as_str().map(str::to_owned));
    }
    assert_eq!(
        seen,
        [
            Some("jim".to_owned()),
            Some("tk".to_owned()),
            Some("tcl".to_owned()),
            Some("xilinx-eda-tcl".to_owned())
        ]
    );

    *backend.default_dialect.lock().await = "klingon".to_owned();
    let unknown = backend
        .get_effective_config_command(&[])
        .await
        .expect("effective config")
        .expect("config payload");
    for field in [
        "dialect_id",
        "dialect_display_name",
        "dialect_short_name",
        "dialect_kind",
        "dialect_description",
        "dialect_provenance",
    ] {
        assert!(unknown[field].is_null(), "{field}: {unknown}");
    }
}

/// `getEffectiveConfig` and the INI export list the codes the policy
/// turns off, catalogued codes only: an uncatalogued spelling decides
/// nothing, and the default-off seed is listed until a layer turns the
/// code on.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_effective_skip_lists_catalogued_codes_only() {
    let backend = test_backend();
    backend
        .apply_global_config(&serde_json::json!({
            "diagnostics": { "W9999": false, "W210": false }
        }))
        .await
        .unwrap();
    let ini = backend.render_config_ini().await;
    let listed: Vec<&str> = ini
        .split("[diagnostics]\n")
        .nth(1)
        .expect("a [diagnostics] section")
        .lines()
        .take_while(|line| !line.is_empty())
        .collect();
    assert_eq!(listed, ["W210 = false", "W242 = false"], "{ini}");
    let effective = backend
        .get_effective_config_command(&[serde_json::json!("file:///skip.tcl")])
        .await
        .expect("effective config")
        .expect("config payload");
    assert_eq!(
        effective["disabled_diagnostics"],
        serde_json::json!(["W210", "W242"]),
        "{effective}"
    );
}

/// The legacy `features.inlayHints` alias must survive the *whole*
/// config-apply → effective-config wiring, not just the
/// `FeatureToggles::apply` unit. This mirrors the `lsp-e2e`
/// `test_legacy_inlay_hints_alias_enables_type_only` flow without a live
/// editor: apply `{"features": {"inlayHints": true}}` through
/// `apply_global_config` (the same call `pull_and_apply_config` makes after a
/// `didChangeConfiguration` re-pull), then read it back through
/// `get_effective_config_command` (the `getEffectiveConfig` handler). The
/// alias must resolve to `inlayTypeHints: true` while `inlayParameterHints`
/// stays off.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_global_config_inlay_alias_reaches_effective_config() {
    let backend = test_backend();
    // Default-off before any config: both inlay families report disabled.
    let before = backend
        .get_effective_config_command(&[serde_json::json!("file:///inlay.tcl")])
        .await
        .expect("effective config")
        .expect("config payload");
    assert_eq!(
        before["features"]["inlayTypeHints"],
        serde_json::Value::Bool(false),
        "type hints default off before any config: {before}",
    );

    backend
        .apply_global_config(&serde_json::json!({ "features": { "inlayHints": true } }))
        .await
        .unwrap();

    let after = backend
        .get_effective_config_command(&[serde_json::json!("file:///inlay.tcl")])
        .await
        .expect("effective config")
        .expect("config payload");
    assert_eq!(
        after["features"]["inlayTypeHints"],
        serde_json::Value::Bool(true),
        "the `inlayHints` alias must enable `inlayTypeHints` end-to-end: {after}",
    );
    assert_eq!(
        after["features"]["inlayParameterHints"],
        serde_json::Value::Bool(false),
        "the alias enables type hints only; parameter hints stay off: {after}",
    );
    // And the gate the inlay handler reads agrees with the reported config.
    let uri = Uri::from_str("file:///inlay.tcl").unwrap();
    assert!(
        backend.inlay_family_enabled(&uri, "inlayTypeHints").await,
        "the inlay-type gate must see the alias-enabled toggle",
    );
    assert!(
        !backend
            .inlay_family_enabled(&uri, "inlayParameterHints")
            .await,
        "parameter hints must stay gated off",
    );
}
