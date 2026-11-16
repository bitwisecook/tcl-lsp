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

//! Dialect listing and the session and document overrides.

use super::*;

/// `tcl-lsp.listDialects` reports every selectable environment, in
/// selectable order, with the presentation fields an editor needs to build
/// a picker without its own name table.
#[test]
fn list_dialects_command_reports_every_selectable_environment() {
    let value = Backend::list_dialects_command();
    let entries = value.as_array().expect("an array of dialects");
    let selectable = tcl_registry::model::selectable_environments();
    assert_eq!(entries.len(), selectable.len());
    for (entry, environment) in entries.iter().zip(&selectable) {
        let id = environment.id.as_str();
        assert_eq!(entry["name"], id);
        assert_eq!(entry["display_name"], environment.display_name.as_ref());
        assert_eq!(entry["short_name"], environment.short_name.as_ref());
        assert_eq!(entry["kind"], environment.kind.word(), "{id}");
        assert_eq!(entry["description"], environment.description(), "{id}");
        let aliases: Vec<&str> = entry["aliases"]
            .as_array()
            .expect("aliases is an array")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        let expected: Vec<&str> = environment.aliases.iter().map(AsRef::as_ref).collect();
        assert_eq!(aliases, expected, "{id}");
        match environment.editor_identity {
            Some(identity) => assert_eq!(entry["editor_language_id"], identity.as_str()),
            None => assert!(entry["editor_language_id"].is_null(), "{id}"),
        }
        let extensions = entry["extensions"]
            .as_array()
            .expect("extensions is an array");
        let claimed = &environment.server_detection.file_extensions;
        assert_eq!(extensions.len(), claimed.len(), "{id}");
        for (reported, claim) in extensions.iter().zip(claimed) {
            assert_eq!(reported["extension"], claim.extension.as_ref());
            assert_eq!(reported["display_name"], claim.display_name.as_ref());
        }
    }

    let names: Vec<&str> = entries
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    for offered in ["jim", "tk", "xilinx-eda-tcl", "tcl8.6"] {
        assert!(names.contains(&offered), "`{offered}` is selectable");
    }
    assert!(!names.contains(&"tcl"), "the lenient sink is not a choice");
    let kinds: Vec<&str> = entries
        .iter()
        .filter_map(|entry| entry["kind"].as_str())
        .collect();
    let languages = kinds.iter().take_while(|kind| **kind == "language").count();
    assert!(
        kinds[languages..].iter().all(|kind| *kind == "packages"),
        "languages come first, then tool shells: {kinds:?}",
    );

    let find = |name: &str| {
        entries
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("the list carries {name}"))
    };
    let irules = find("f5-irules");
    assert_eq!(irules["editor_language_id"], "tcl-irule");
    assert!(
        irules["extensions"]
            .as_array()
            .is_some_and(|exts| !exts.is_empty()),
        "iRules owns file extensions",
    );
    assert_eq!(find("jim")["editor_language_id"], "tcl-jim");
    assert_eq!(find("jim")["kind"], "language");
    assert_eq!(find("tk")["kind"], "packages");
    assert!(
        find("xilinx-eda-tcl")["aliases"]
            .as_array()
            .is_some_and(|aliases| aliases.iter().any(|alias| alias == "vivado")),
        "the Vivado shell answers to `vivado`",
    );
}

/// A deliberate session override outranks the configured default and
/// survives a configuration pull.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_dialect_override_survives_a_config_pull() {
    let backend = test_backend();
    backend
        .set_session_dialect_override_command(&[serde_json::json!("f5-irules")])
        .await
        .expect("override command");
    assert_eq!(backend.session_dialect().await, "f5-irules");

    // A pulled/pushed `tclLsp.dialect` rewrites the *configured* default …
    backend
        .apply_global_config(&serde_json::json!({ "dialect": "tcl9.0" }))
        .await
        .unwrap();
    assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
    // … and must leave the override in force.
    assert_eq!(
        backend.session_dialect().await,
        "f5-irules",
        "a config pull must not revert a deliberate session override",
    );

    // Clearing hands the session back to the configured value.
    backend
        .set_session_dialect_override_command(&[])
        .await
        .expect("clear command");
    assert_eq!(backend.session_dialect().await, "tcl9.0");
}

/// An unknown dialect is rejected rather than installed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_dialect_override_rejects_an_unknown_dialect() {
    let backend = test_backend();
    let reply = backend
        .set_session_dialect_override_command(&[serde_json::json!("tcl99")])
        .await
        .expect("override command")
        .expect("a reply");
    assert_eq!(reply["success"], serde_json::json!(false), "{reply}");
    assert!(backend.session_dialect_override.lock().await.is_none());
}

/// The override sits in the session-fallback tier, so it reaches a document
/// whose dialect is not pinned by anything stronger — and nothing else.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_dialect_override_only_reaches_the_fallback_tier() {
    let backend = test_backend();
    backend
        .set_session_dialect_override_command(&[serde_json::json!("f5-irules")])
        .await
        .expect("override command");
    let plain = Uri::from_str("file:///plain.tcl").unwrap();
    assert_eq!(
        backend.dialect_for_open(&plain, "", "set x 1\n").await,
        "f5-irules",
        "an unpinned document falls back to the override",
    );
    // An explicit language id still wins, exactly as it does over the
    // configured default.
    assert_eq!(
        backend.dialect_for_open(&plain, "tcl90", "set x 1\n").await,
        "tcl9.0",
        "an explicit language id must still outrank the session tier",
    );
}

/// A per-document override is the strongest tier — stronger
/// than an explicit language id and than the in-source directive — and it
/// reaches only the document it names.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_document_dialect_override_outranks_every_inference() {
    let backend = test_backend();
    let sample = Uri::from_str("file:///studio/test.tcl").unwrap();
    let sibling = Uri::from_str("file:///studio/other.tcl").unwrap();
    backend
        .set_document_dialect_override_command(&[
            serde_json::json!(sample.as_str()),
            serde_json::json!("f5-irules"),
        ])
        .await
        .expect("override command");

    assert_eq!(
        backend.dialect_for_open(&sample, "tcl", "set x 1\n").await,
        "f5-irules",
        "the named document takes the override",
    );
    assert_eq!(
        backend
            .dialect_for_open(&sample, "tcl90", "set x 1\n")
            .await,
        "f5-irules",
        "an explicit language id does not outrank a named document",
    );
    assert_eq!(
        backend
            .dialect_for_open(&sample, "tcl", "# tcl-dialect: tcl8.4\n")
            .await,
        "f5-irules",
        "an in-source directive does not outrank a named document",
    );
    assert_eq!(
        backend.dialect_for_open(&sibling, "tcl", "set x 1\n").await,
        "tcl8.6",
        "no other buffer is re-tagged — the whole point of the tier",
    );
}

/// Clearing releases the document back to ordinary resolution.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_document_dialect_override_is_released_by_a_null_argument() {
    let backend = test_backend();
    let sample = Uri::from_str("file:///studio/test.tcl").unwrap();
    backend
        .set_document_dialect_override_command(&[
            serde_json::json!(sample.as_str()),
            serde_json::json!("f5-irules"),
        ])
        .await
        .expect("override command");
    assert_eq!(
        backend.dialect_for_open(&sample, "tcl", "set x 1\n").await,
        "f5-irules",
    );
    backend
        .set_document_dialect_override_command(&[serde_json::json!(sample.as_str())])
        .await
        .expect("clear command");
    assert_eq!(
        backend.dialect_for_open(&sample, "tcl", "set x 1\n").await,
        "tcl8.6",
        "a cleared document resolves normally again",
    );
    assert!(backend.document_dialect_overrides.lock().await.is_empty());
}

/// A missing URI and an unknown dialect are both reported rather than
/// installed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_document_dialect_override_rejects_bad_arguments() {
    let backend = test_backend();
    let missing_uri = backend
        .set_document_dialect_override_command(&[])
        .await
        .expect("command")
        .expect("a reply");
    assert_eq!(missing_uri["success"], serde_json::json!(false));

    let unknown = backend
        .set_document_dialect_override_command(&[
            serde_json::json!("file:///studio/test.tcl"),
            serde_json::json!("tcl99"),
        ])
        .await
        .expect("command")
        .expect("a reply");
    assert_eq!(unknown["success"], serde_json::json!(false), "{unknown}");
    assert!(backend.document_dialect_overrides.lock().await.is_empty());
}
