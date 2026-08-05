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

//! Issue #1214 — server-constructed and client-sent URIs share one canonical
//! form, and a client that sends an unencoded URI is accepted rather than
//! rejected.

use crate::common::Lsp;

use serde_json::{Value, json};

/// A workspace root whose name contains a space, so the folder URI a
/// `JetBrains`- / Neovim-style client sends carries a raw ` ` and is not a valid
/// URI at all.
fn spaced_workspace(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tcl lsp e2e {label} {}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&root).expect("mk workspace root");
    root
}

/// §B: an unencoded folder URI used to fail `Uri`'s deserialisation, which
/// failed the whole `initialize` request — the session never started.  The
/// transport boundary now repairs it, so the handshake succeeds and the server
/// reports the folder in its *encoded* form.
#[test]
fn an_unencoded_workspace_folder_uri_is_accepted_and_normalised() {
    let root = spaced_workspace("unencoded");
    let raw_uri = format!("file://{}", root.to_string_lossy());
    assert!(
        raw_uri.contains(' '),
        "the fixture must exercise a raw space: {raw_uri}"
    );

    let mut lsp = Lsp::spawn(json!({}));
    let result = lsp.initialize_at(&raw_uri);
    assert!(
        result.get("capabilities").is_some(),
        "initialize must succeed against an unencoded folder URI: {result}"
    );

    // The server holds the folder in the repaired, encoded form — the same
    // spelling it constructs for files it scans under that folder.
    let cfg = lsp.effective_config("");
    let folders: Vec<&str> = cfg["known_folder_uris"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    assert!(
        folders.iter().any(|f| f.contains("%20")),
        "the folder URI must be normalised to its encoded form: {folders:?}"
    );
    assert!(
        !folders.iter().any(|f| f.contains(' ')),
        "no raw space may survive into the server's folder list: {folders:?}"
    );

    std::fs::remove_dir_all(&root).ok();
}

/// §1: a file the *server* discovers on disk and the same file the *client*
/// opens must be one document, not two.  The two sides now spell it the same
/// way, so the workspace index has a single entry and find-references reports a
/// single definition site.
#[test]
fn a_scanned_file_and_the_same_open_file_are_one_document() {
    use std::time::{Duration, Instant};

    let root = spaced_workspace("one-doc");
    let lib = root.join("lib.tcl");
    let caller = root.join("caller.tcl");
    std::fs::write(&lib, "proc shared_marker {} { return 1 }\n").expect("write lib");
    std::fs::write(&caller, "shared_marker\n").expect("write caller");

    // The client sends unencoded URIs throughout — the shape that used to make
    // the scan's spelling and the editor's spelling disagree.  `didOpen` is
    // driven by hand rather than through `open_ready`, because the server
    // answers on the *normalised* URI (the only valid spelling of the two), so
    // there is no `publishDiagnostics` for the raw one to wait on.
    let raw_root = format!("file://{}", root.to_string_lossy());
    let raw_lib = format!("file://{}", lib.to_string_lossy());
    let encoded_lib = raw_lib.replace(' ', "%20");
    let mut lsp = Lsp::spawn(json!({}));
    lsp.initialize_at(&raw_root);
    lsp.notify(
        "textDocument/didOpen",
        json!({ "textDocument": {
            "uri": raw_lib,
            "languageId": "tcl",
            "version": 1,
            "text": "proc shared_marker {} { return 1 }\n",
        }}),
    );

    // Exactly one workspace symbol for the proc: a second would mean the
    // scanned copy and the open buffer were indexed as two documents.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let hits: Vec<Value> = lsp
            .workspace_symbols("shared_marker")
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|s| s.get("name").and_then(Value::as_str) == Some("shared_marker"))
            .collect();
        if hits.len() == 1 {
            let uri = hits[0]["location"]["uri"].as_str().unwrap_or_default();
            assert!(
                uri.contains("%20") && !uri.contains(' '),
                "the reported URI must be the canonical encoded form: {uri}"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the scanned file and the open buffer must be one document; got {hits:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }

    // And the cross-document call site in the unopened sibling resolves — which
    // it can only do if the scan indexed it under a URI the resolver agrees
    // with.
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let uris: Vec<String> = lsp
            .references(&encoded_lib, 0, 6, true)
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|r| r["uri"].as_str().map(str::to_owned))
            .collect();
        if uris.iter().any(|u| u.ends_with("caller.tcl")) {
            assert!(
                uris.iter().all(|u| !u.contains(' ')),
                "every reported URI must be in the canonical encoded form: {uris:?}"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the unopened caller must be found through the shared canonical form: {uris:?}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }

    std::fs::remove_dir_all(&root).ok();
}
