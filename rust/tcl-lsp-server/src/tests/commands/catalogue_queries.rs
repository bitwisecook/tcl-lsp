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

//! Registry and package queries.

use super::*;

#[test]
fn tcl_installation_picker_selection_refuses_unrepresentable_native_paths() {
    let unicode = core_tcl_install::TclInstallation {
        version: "9.0".to_owned(),
        tcl_library: PathBuf::from("/opt/tcl9.0/lib/tcl9.0"),
        auto_path: vec![PathBuf::from("/opt/tcl9.0/lib")],
    };
    assert_eq!(
        tcl_installation_selection_paths(&unicode),
        Ok((
            "/opt/tcl9.0/lib/tcl9.0".to_owned(),
            vec!["/opt/tcl9.0/lib".to_owned()],
        )),
    );

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;
        let opaque = PathBuf::from(std::ffi::OsString::from_vec(
            b"/opt/tcl9.0/lib/\xff".to_vec(),
        ));
        let replacement_sibling = PathBuf::from("/opt/tcl9.0/lib/\u{fffd}");
        assert_ne!(opaque, replacement_sibling);
        let discovered = core_tcl_install::TclInstallation {
            version: "9.0".to_owned(),
            tcl_library: PathBuf::from("/opt/tcl9.0/lib/tcl9.0"),
            auto_path: vec![opaque],
        };
        assert_eq!(
            tcl_installation_selection_paths(&discovered),
            Err(TclInstallationSelectionUnavailable::NonUnicodeAutoPath),
        );
    }
}

#[test]
fn list_irule_events_command_returns_sorted_known_events() {
    let out = Backend::list_irule_events_command();
    let names: Vec<&str> = out
        .get("events")
        .and_then(|v| v.as_array())
        .expect("events array")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert!(names.contains(&"HTTP_REQUEST"), "{names:?}");
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "events must be sorted");
}

#[tokio::test]
async fn describe_irule_event_command_reports_known_and_unknown() {
    let backend = test_backend();
    let known = backend
        .describe_irule_event_command(&[serde_json::json!("HTTP_REQUEST")])
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        known.get("known").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert!(
        known
            .get("validCommandCount")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            > 0,
        "a known event should report valid commands: {known:?}",
    );
    let unknown = backend
        .describe_irule_event_command(&[serde_json::json!("NOPE_EVENT")])
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        unknown.get("known").and_then(serde_json::Value::as_bool),
        Some(false),
    );
}

#[tokio::test]
async fn describe_irule_command_keeps_context_unavailable_descriptors_out_of_case_advice() {
    let backend = test_backend();
    let registry = backend.registry_for_dialect("f5-irules").await;
    let context = tcl_lsp_core::context_for_dialect("f5-irules");
    assert!(registry.get("oo::configurable").is_some());
    assert!(
        context
            .context()
            .resolve_spec(&registry, "oo::configurable")
            .is_none()
    );
    for name in ["oo::configurable", "OO::CONFIGURABLE"] {
        let response = backend
            .describe_irule_command_command(&[serde_json::json!(name)])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response["found"], false, "{name}: {response}");
    }
}

#[tokio::test]
async fn describe_irule_command_preserves_literal_space_and_empty_queries() {
    let backend = test_backend();
    for name in [
        "",
        " HTTP::header",
        "HTTP::header ",
        "\tHTTP::header",
        "http::HEADER ",
    ] {
        let response = backend
            .describe_irule_command_command(&[serde_json::json!(name)])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response["found"], false, "{name:?}: {response}");
        assert_eq!(response["command"], name);
    }
}

#[tokio::test]
async fn describe_irule_command_command_resolves_case_insensitively() {
    let backend = test_backend();
    let found = backend
        .describe_irule_command_command(&[serde_json::json!("HTTP::header")])
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        found.get("found").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    let canonical = found.get("command").and_then(serde_json::Value::as_str);
    // A differently-cased spelling resolves to the same canonical command.
    let ci = backend
        .describe_irule_command_command(&[serde_json::json!("http::HEADER")])
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        ci.get("found").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        ci.get("command").and_then(serde_json::Value::as_str),
        canonical
    );
    // An unknown command reports `found: false`.
    let missing = backend
        .describe_irule_command_command(&[serde_json::json!("NoSuchCmd")])
        .await
        .expect("ok")
        .expect("some");
    assert_eq!(
        missing.get("found").and_then(serde_json::Value::as_bool),
        Some(false)
    );
}

#[tokio::test]
async fn list_subcommands_command_returns_sorted_subcommands() {
    let backend = test_backend();
    let out = backend
        .list_subcommands_command(&[serde_json::json!("string")])
        .await
        .expect("some");
    assert_eq!(
        out.get("command").and_then(serde_json::Value::as_str),
        Some("string")
    );
    let subs: Vec<&str> = out
        .get("subcommands")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|s| s.get("name").and_then(serde_json::Value::as_str))
        .collect();
    assert!(subs.contains(&"length"), "string subcommands: {subs:?}");
    let mut sorted = subs.clone();
    sorted.sort_unstable();
    assert_eq!(subs, sorted, "subcommands must be sorted");
}

#[test]
fn suggest_packages_echoes_symbol_with_list() {
    let args = vec![serde_json::json!("json::write")];
    let out = Backend::suggest_packages_for_symbol_command(&args);
    assert_eq!(
        out.get("symbol").and_then(|v| v.as_str()),
        Some("json::write")
    );
    assert!(
        out.get("suggestions")
            .is_some_and(serde_json::Value::is_array)
    );
}

#[test]
fn list_known_packages_reports_a_list() {
    let out = Backend::list_known_packages_command();
    assert!(out.get("packages").is_some_and(serde_json::Value::is_array));
}
