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

//! Package definitions remain isolated by each caller's selected release.

use crate::common::Lsp;
use crate::common::helpers::locations;

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
const EXACT_ONE: &str = "package require -exact P 1.0\n";
const EXACT_TWO: &str = "package require -exact P 2.0\n";
const LATEST: &str = "package require P\n";

fn provider_text(version: &str) -> String {
    format!(
        "namespace eval ::P {{\n    proc run {{}} {{ return {version} }}\n    proc café {{}} {{ return {version} }}\n}}\npackage provide P {version}\n"
    )
}

fn caller_text(requirement: &str) -> String {
    format!("{requirement}puts [::P::run]\nputs [::P::café]\n")
}

fn path_uri(path: &Path) -> String {
    tcl_lsp_server::canonical_file_uri(path)
        .expect("fixture path has a representable file URI")
        .to_string()
}

fn write_provider(root: &Path, version: &str, body: &str) -> PathBuf {
    let directory = root.join(version);
    std::fs::create_dir_all(&directory).expect("create package directory");
    std::fs::write(
        directory.join("pkgIndex.tcl"),
        format!("package ifneeded P {version} [list source [file join $dir package.tcl]]\n"),
    )
    .expect("write package index");
    std::fs::write(directory.join("package.tcl"), body).expect("write package provider");
    directory
}

fn write_hidden_provider(root: &Path) {
    let directory = root.join("hidden");
    std::fs::create_dir_all(&directory).expect("create hidden package directory");
    std::fs::write(
        directory.join("pkgIndex.tcl"),
        "if {0} {package ifneeded P 99.0 [list source [file join $dir hidden.tcl]]}\n",
    )
    .expect("write hidden package index");
    std::fs::write(
        directory.join("hidden.tcl"),
        "namespace eval ::P {\n    proc run {} { return HIDDEN }\n    proc café {} { return HIDDEN }\n}\npackage provide P 99.0\n",
    )
    .expect("write hidden package provider");
}

fn diagnostics_at_completed_update(
    lsp: &Lsp,
    uri: &str,
    version: i64,
    cursor: usize,
) -> Vec<Value> {
    let uri_marker = format!("(uri={uri},");
    lsp.await_log(&["workspace_state.update", &uri_marker], TIMEOUT, cursor);
    let notifications = lsp.notifications();
    let update_index = notifications
        .iter()
        .enumerate()
        .skip(cursor)
        .find_map(|(index, note)| {
            let message = note
                .get("method")
                .filter(|method| method.as_str() == Some("window/logMessage"))
                .and_then(|_| note.get("params"))
                .and_then(|params| params.get("message"))
                .and_then(Value::as_str)?;
            (message.contains("workspace_state.update") && message.contains(&uri_marker))
                .then_some(index)
        })
        .expect("awaited per-URI workspace update is present");

    notifications
        .iter()
        .take(update_index)
        .enumerate()
        .skip(cursor)
        .rev()
        .find_map(|(_, note)| {
            let params = note
                .get("method")
                .filter(|method| method.as_str() == Some("textDocument/publishDiagnostics"))
                .and_then(|_| note.get("params"))?;
            if params.get("uri").and_then(Value::as_str) != Some(uri) {
                return None;
            }
            assert_eq!(params.get("version").and_then(Value::as_i64), Some(version));
            Some(
                params
                    .get("diagnostics")
                    .and_then(Value::as_array)
                    .expect("completed diagnostic publish has a findings array")
                    .clone(),
            )
        })
        .expect("completed update has a preceding exact-URI diagnostic publish")
}

fn open_ready(lsp: &mut Lsp, uri: &str, text: &str) -> Vec<Value> {
    let cursor = lsp.notification_cursor();
    lsp.open_document(uri, text);
    diagnostics_at_completed_update(lsp, uri, 1, cursor)
}

fn query_definition(lsp: &mut Lsp, uri: &str, line: u32) -> Value {
    // In both `puts [::P::run]` and `puts [::P::café]`, the command name starts
    // at UTF-16 column 11.
    lsp.definition(uri, line, 11)
}

fn assert_definition(
    lsp: &mut Lsp,
    caller_uri: &str,
    line: u32,
    provider_uri: &str,
    expected_line: u32,
    expected_character: u32,
    expected_length: u32,
) {
    let result = query_definition(lsp, caller_uri, line);
    let found = locations(&result);
    assert_eq!(found.len(), 1, "expected one package definition: {result}");
    assert_eq!(found[0].uri, provider_uri);
    assert_eq!(
        found[0].range,
        json!({
            "start": { "line": expected_line, "character": expected_character },
            "end": { "line": expected_line, "character": expected_character + expected_length },
        }),
        "definition range identifies the selected provider declaration"
    );
}

fn assert_references_include_provider_and_call(
    lsp: &mut Lsp,
    caller_uri: &str,
    provider_uri: &str,
) {
    for (line, name_end) in [(1, 14), (2, 15)] {
        let references = lsp.references(caller_uri, line, 11, true);
        let found = locations(&references);
        assert!(
            found.iter().any(|location| {
                location.uri == provider_uri
                    && location.range["start"] == json!({ "line": line - 1, "character": 9 })
            }),
            "references include the selected provider declaration: {references}"
        );
        assert!(
            found.iter().any(|location| {
                location.uri == caller_uri
                    && location.range["start"] == json!({ "line": line, "character": 11 })
                    && location.range["end"] == json!({ "line": line, "character": name_end })
            }),
            "references include the caller's exact command use: {references}"
        );
    }
}

fn make_fixture(
    label: &str,
) -> (
    PathBuf,
    PathBuf,
    PathBuf,
    PathBuf,
    PathBuf,
    PathBuf,
    PathBuf,
) {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-naming-packages-{}-{label}-{}",
        std::process::id(),
        line!()
    ));
    let workspace = root.join("workspace");
    let libraries_a = root.join("libraries-a");
    let libraries_b = root.join("libraries-b");
    std::fs::create_dir_all(&workspace).expect("create caller workspace");
    let p1 = write_provider(&libraries_a, "1.0", &provider_text("1.0"));
    let p2_a = write_provider(&libraries_a, "2.0", &provider_text("2.0"));
    write_hidden_provider(&libraries_a);
    let p2_b = write_provider(&libraries_b, "2.0", &provider_text("2.0"));
    (
        root,
        workspace,
        libraries_a,
        libraries_b,
        p1.join("package.tcl"),
        p2_a.join("package.tcl"),
        p2_b.join("package.tcl"),
    )
}

fn write_callers(workspace: &Path) -> [(String, String); 3] {
    [
        ("exact1.tcl", caller_text(EXACT_ONE)),
        ("exact2.tcl", caller_text(EXACT_TWO)),
        ("latest.tcl", caller_text(LATEST)),
    ]
    .map(|(name, text)| {
        let path = workspace.join(name);
        std::fs::write(&path, &text).expect("write package caller");
        (path_uri(&path), text)
    })
}

fn assert_package_queries(lsp: &mut Lsp, caller_uri: &str, provider_uri: &str) {
    // Package declarations are one-based only in prose; protocol ranges are
    // zero-based. `run` is ASCII length 3 and `café` is UTF-16 length 4.
    assert_definition(lsp, caller_uri, 1, provider_uri, 1, 9, 3);
    assert_definition(lsp, caller_uri, 2, provider_uri, 2, 9, 4);
    assert_references_include_provider_and_call(lsp, caller_uri, provider_uri);
}

fn run_cold_order(label: &str, order: [usize; 3]) {
    let (root, workspace, libraries_a, _libraries_b, p1, p2_a, _p2_b) = make_fixture(label);
    let callers = write_callers(&workspace);
    let libraries_a = libraries_a.to_str().expect("library path is UTF-8");
    let mut lsp = Lsp::with_config_at_root(json!({ "libraryPaths": [libraries_a] }), &workspace);
    // This marker is logged after package resolver publication and workspace
    // scan readiness, so the first naming requests see the configured paths.
    lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
    for (uri, text) in &callers {
        open_ready(&mut lsp, uri, text);
    }

    for index in order {
        let provider = if index == 0 { &p1 } else { &p2_a };
        assert_package_queries(&mut lsp, &callers[index].0, &path_uri(provider));
    }
    // The unconstrained latest caller picks P2.0, while the false-guarded P99
    // package index entry must never become a candidate.
    assert_package_queries(&mut lsp, &callers[2].0, &path_uri(&p2_a));

    drop(lsp);
    std::fs::remove_dir_all(root).ok();
}

fn has_code(diagnostics: &[Value], code: &str) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.get("code").and_then(Value::as_str) == Some(code))
}

#[test]
fn package_release_queries_are_order_independent_and_retire_old_library_paths() {
    // Each order gets a cold server; each entrypoint is queried in the stated
    // sequence so a previous caller cannot prime the next caller's release.
    run_cold_order("a-b-a", [0, 1, 0]);
    run_cold_order("b-a-b", [1, 0, 1]);

    let (root, workspace, libraries_a, libraries_b, p1, _p2_a, p2_b) =
        make_fixture("library-change");
    let callers = write_callers(&workspace);
    let library_a_string = libraries_a.to_str().expect("library path is UTF-8");
    let mut lsp =
        Lsp::with_config_at_root(json!({ "libraryPaths": [library_a_string] }), &workspace);
    lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
    for (uri, text) in &callers {
        open_ready(&mut lsp, uri, text);
    }
    assert_package_queries(&mut lsp, &callers[0].0, &path_uri(&p1));

    // Settled effective config plus the subsequent scan marker is independent
    // of naming results. The scan marker is emitted only after the new package
    // resolver and workspace index have been published.
    let cursor = lsp.notification_cursor();
    let library_b_string = libraries_b.to_str().expect("library path is UTF-8");
    lsp.apply_configuration_settle(
        json!({ "libraryPaths": [library_b_string] }),
        &callers[0].0,
        |effective| effective.get("library_paths") == Some(&json!([library_b_string])),
    );
    lsp.await_log(&["workspace_folders_scan"], TIMEOUT, cursor);
    // The scan notification carries no configuration-generation identifier;
    // effective_config above proves the requested path list, and this later
    // marker proves resolver/index publication completed for the new scan.

    // The unchanged exact-1.0 requirement is now unavailable. Positive exact
    // 2.0 and latest controls prove that the new library generation is usable.
    let retired = query_definition(&mut lsp, &callers[0].0, 1);
    assert!(
        locations(&retired).is_empty(),
        "exact P 1.0 must not resolve to a stale P1 or another release after switching to B: {retired}"
    );
    assert_package_queries(&mut lsp, &callers[1].0, &path_uri(&p2_b));
    assert_package_queries(&mut lsp, &callers[2].0, &path_uri(&p2_b));

    // Change the exact requirement in the open document. This is a distinct
    // source revision, so its first naming response follows its completed deep
    // diagnostic update, without a query used to repair readiness.
    let exact_two = caller_text(EXACT_TWO);
    let cursor = lsp.notification_cursor();
    lsp.replace_document(&callers[0].0, 2, &exact_two);
    let diagnostics = diagnostics_at_completed_update(&lsp, &callers[0].0, 2, cursor);
    assert!(
        !has_code(&diagnostics, "W123"),
        "the new exact 2.0 requirement resolves against library B: {diagnostics:?}"
    );
    assert_package_queries(&mut lsp, &callers[0].0, &path_uri(&p2_b));

    drop(lsp);
    std::fs::remove_dir_all(root).ok();
}
