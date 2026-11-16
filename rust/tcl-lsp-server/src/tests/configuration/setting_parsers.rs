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

//! Payload shapes and individual setting readers.

use super::*;

#[test]
fn collapse_inlay_alias_resolves_per_layer() {
    // Alias-only: `inlayHints` becomes `inlayTypeHints`.
    let mut alias_only = serde_json::json!({ "features": { "inlayHints": true } });
    collapse_inlay_alias(&mut alias_only);
    assert_eq!(
        alias_only["features"]["inlayTypeHints"],
        serde_json::json!(true)
    );
    assert!(alias_only["features"].get("inlayHints").is_none());

    // Both present in one layer: the explicit `inlayTypeHints` wins.
    let mut both = serde_json::json!({
        "features": { "inlayHints": true, "inlayTypeHints": false }
    });
    collapse_inlay_alias(&mut both);
    assert_eq!(both["features"]["inlayTypeHints"], serde_json::json!(false));
    assert!(both["features"].get("inlayHints").is_none());

    // Neither: untouched (negative case).
    let mut neither = serde_json::json!({ "features": { "codeActions": true } });
    collapse_inlay_alias(&mut neither);
    assert!(neither["features"].get("inlayTypeHints").is_none());
}

#[test]
fn inlay_alias_in_editor_layer_beats_global_inlay_type_hints() {
    // The global `config.ini` layer (e.g. written by
    // `exportConfig`) carries an explicit `inlayTypeHints: false`, while the
    // higher-precedence editor layer sets only the legacy `inlayHints`
    // alias. After per-layer collapse the editor must win → type hints on.
    let mut global = serde_json::json!({
        "features": { "inlayTypeHints": false, "inlayParameterHints": false, "codeActions": true }
    });
    let mut editor = serde_json::json!({ "features": { "inlayHints": true } });
    collapse_inlay_alias(&mut global);
    collapse_inlay_alias(&mut editor);
    let merged = config_ini::merge_settings(&global, &editor);
    assert_eq!(
        merged["features"]["inlayTypeHints"],
        serde_json::json!(true),
        "editor inlayHints alias must beat global inlayTypeHints:false",
    );
    // Parameter hints (only in the global layer) stay off; no stray alias key.
    assert_eq!(
        merged["features"]["inlayParameterHints"],
        serde_json::json!(false)
    );
    assert!(merged["features"].get("inlayHints").is_none());
}

#[test]
fn normalize_config_payload_handles_nested_flat_and_unwrapped() {
    // Nested `tclLsp` object.
    let nested = serde_json::json!({ "tclLsp": { "optimiser": { "O109": false } } });
    assert_eq!(
        normalize_config_payload(&nested),
        serde_json::json!({ "optimiser": { "O109": false } }),
    );
    // Flat dotted keys (with and without the `tclLsp.` prefix).
    let flat = serde_json::json!({
        "tclLsp.optimiser.O109": false,
        "tclLsp.dialect": "tcl8.6",
    });
    assert_eq!(
        normalize_config_payload(&flat),
        serde_json::json!({ "optimiser": { "O109": false }, "dialect": "tcl8.6" }),
    );
    // Unwrapped top-level sections (e.g. JetBrains).
    let unwrapped = serde_json::json!({ "optimiser": { "enabled": false }, "dialect": "tcl9.0" });
    assert_eq!(
        normalize_config_payload(&unwrapped),
        serde_json::json!({ "optimiser": { "enabled": false }, "dialect": "tcl9.0" }),
    );
    // Composed: nested + flat merge key-by-key.
    let composed = serde_json::json!({
        "tclLsp": { "optimiser": { "O109": false } },
        "tclLsp.optimiser.O110": false,
    });
    assert_eq!(
        normalize_config_payload(&composed),
        serde_json::json!({ "optimiser": { "O109": false, "O110": false } }),
    );
}

#[test]
fn normalize_config_payload_scalar_object_collision_does_not_panic() {
    // A flat scalar key that collides with a deeper dotted
    // key in the same payload must not panic the server; the nested
    // structure wins over the conflicting scalar.
    let collide = serde_json::json!({
        "tclLsp.optimiser": true,
        "tclLsp.optimiser.enabled": false,
    });
    assert_eq!(
        normalize_config_payload(&collide),
        serde_json::json!({ "optimiser": { "enabled": false } }),
    );
    // The reverse spelling (scalar under a nested prefix) also survives.
    let collide2 = serde_json::json!({
        "tclLsp": { "style": "x" },
        "tclLsp.style.nonAscii": "escape",
    });
    assert_eq!(
        normalize_config_payload(&collide2),
        serde_json::json!({ "style": { "nonAscii": "escape" } }),
    );
}

#[test]
fn normalize_config_payload_survives_mixed_shape_collision() {
    // A client that supplies both a scalar prefix and a dotted child at the
    // same segment (`tclLsp.features` = false *and*
    // `tclLsp.features.semanticTokens` = true) must not panic; the deeper
    // key replaces the colliding scalar rather than crashing the server.
    // (`serde_json`'s default `Map` orders keys, so `tclLsp.features` is
    // folded before its dotted child regardless of literal order.)
    let payload = serde_json::json!({
        "tclLsp.features": false,
        "tclLsp.features.semanticTokens": true,
    });
    assert_eq!(
        normalize_config_payload(&payload),
        serde_json::json!({ "features": { "semanticTokens": true } }),
    );
    // When the prefix is already an object, the dotted child merges into it
    // (no collision, both keys kept) — the non-panicking path we must not
    // regress.
    let obj_prefix = serde_json::json!({
        "tclLsp.features": { "hover": true },
        "tclLsp.features.semanticTokens": true,
    });
    assert_eq!(
        normalize_config_payload(&obj_prefix),
        serde_json::json!({ "features": { "hover": true, "semanticTokens": true } }),
    );
}

#[test]
fn parse_non_ascii_mode_maps_settings() {
    assert_eq!(parse_non_ascii_mode("off"), NonAsciiMode::Off);
    assert_eq!(parse_non_ascii_mode("strict"), NonAsciiMode::Strict);
    assert_eq!(
        parse_non_ascii_mode("confusables"),
        NonAsciiMode::Confusables
    );
    assert_eq!(parse_non_ascii_mode("common"), NonAsciiMode::Common);
    assert_eq!(parse_non_ascii_mode("bogus"), NonAsciiMode::Default);
}

#[test]
fn settings_non_ascii_mode_nested_and_flat() {
    let nested = serde_json::json!({"tclLsp": {"style": {"nonAscii": "common"}}});
    assert_eq!(settings_non_ascii_mode(&nested), Some(NonAsciiMode::Common));
    let flat = serde_json::json!({"tclLsp.style.nonAscii": "off"});
    assert_eq!(settings_non_ascii_mode(&flat), Some(NonAsciiMode::Off));
    let none = serde_json::json!({"tclLsp": {"dialect": "tcl9.0"}});
    assert_eq!(settings_non_ascii_mode(&none), None);
}

#[test]
fn environment_kind_setting_is_read_from_every_payload_shape() {
    let off = Some(false);
    // Nested under `tclLsp`, unwrapped, and flat-dotted.
    for payload in [
        serde_json::json!({ "tclLsp": { "notifications": { "environmentKind": false } } }),
        serde_json::json!({ "notifications": { "environmentKind": false } }),
        serde_json::json!({ "tclLsp.notifications.environmentKind": false }),
    ] {
        assert_eq!(
            settings_environment_kind_enabled(&payload),
            off,
            "{payload}"
        );
    }
    assert_eq!(
        settings_environment_kind_enabled(
            &serde_json::json!({ "tclLsp": { "notifications": { "environmentKind": true } } })
        ),
        Some(true)
    );
    // Anything else says nothing, so the current value stands.
    for payload in [
        serde_json::json!({}),
        serde_json::json!({ "tclLsp": { "notifications": {} } }),
        serde_json::json!({ "notifications": { "highlightingHealth": false } }),
        serde_json::json!({ "notifications": { "environmentKind": "no" } }),
        serde_json::Value::Null,
    ] {
        assert_eq!(
            settings_environment_kind_enabled(&payload),
            None,
            "{payload}"
        );
    }
}

/// The editor's Workspace Trust state reaches pack discovery from the two
/// places a client states it — `initializationOptions.workspaceTrust` and a
/// top-level `workspaceTrust` push — and from nowhere a workspace's own
/// settings file can write. The negative: `tclLsp.workspaceTrust` inside
/// a pushed `tclLsp` section (what VS Code's configuration sync sends, from
/// every settings layer) changes nothing. A client that says nothing is
/// trusted.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_trust_comes_from_the_client_and_never_from_a_setting() {
    use tcl_dialect::model::WorkspaceTrust;
    let owned = test_backend();
    let backend = &owned;
    let trust = move || async move { backend.spec_pack_discovery().await.workspace_trust };
    assert_eq!(trust().await, WorkspaceTrust::Trusted, "absent is trusted");

    backend
        .apply_initialization_options(&InitializeParams {
            initialization_options: Some(serde_json::json!({
                "workspaceTrust": "untrusted",
            })),
            ..InitializeParams::default()
        })
        .await;
    assert_eq!(trust().await, WorkspaceTrust::Untrusted);

    // Each push ends in the coalesced configuration reload, which fails
    // fast against the detached socket; the timeout is a backstop.
    let push = move |settings: serde_json::Value| {
        crate::rt::timeout(
            std::time::Duration::from_secs(60),
            backend.did_change_configuration(DidChangeConfigurationParams { settings }),
        )
    };
    push(serde_json::json!({ "tclLsp": { "workspaceTrust": "trusted" } }))
        .await
        .expect("did_change_configuration should not hang");
    push(serde_json::json!({ "tclLsp.workspaceTrust": "trusted" }))
        .await
        .expect("did_change_configuration should not hang");
    assert_eq!(
        trust().await,
        WorkspaceTrust::Untrusted,
        "a synchronised setting cannot grant trust"
    );

    push(serde_json::json!({ "workspaceTrust": "trusted" }))
        .await
        .expect("did_change_configuration should not hang");
    assert_eq!(trust().await, WorkspaceTrust::Trusted, "the client's grant");
    push(serde_json::json!({ "workspaceTrust": "sometimes" }))
        .await
        .expect("did_change_configuration should not hang");
    assert_eq!(
        trust().await,
        WorkspaceTrust::Trusted,
        "an unreadable value is no statement"
    );
}
