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

//! Incremental lifecycle answers must agree with a fresh server at the same
//! source revision, for both open-buffer edits and closed watched providers.

use crate::common::helpers::locations;
use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
const SECOND_PROC: &str = "proc p {} {}\n";
const RENAME: &str = "rename p q\n";
const CONSUMERS: &str = "proc consumer {} { p }\nproc qconsumer {} { q }\n";

fn lifecycle_sources() -> (String, String) {
    let initial = "proc p {} {return initial}\n";
    let padding = format!("#{}\n", " ".repeat(SECOND_PROC.len() - 2));
    let old = format!("{initial}{SECOND_PROC}{RENAME}{padding}{CONSUMERS}");
    let new = format!("{initial}{padding}{RENAME}{SECOND_PROC}{CONSUMERS}");
    assert_eq!(old.find(RENAME), new.find(RENAME));
    assert_eq!(old.len(), new.len());
    (old, new)
}

fn position_of_name(text: &str, name: &str) -> (u32, u32) {
    let context = format!("{{ {name} }}");
    let offset = text.find(&context).expect("fixture query text exists") + 2;
    let prefix = &text[..offset];
    let line = prefix.bytes().filter(|&byte| byte == b'\n').count();
    let character = prefix
        .rsplit_once('\n')
        .map_or(prefix, |(_, tail)| tail)
        .encode_utf16()
        .count();
    (
        u32::try_from(line).expect("fixture line fits u32"),
        u32::try_from(character).expect("fixture column fits u32"),
    )
}

fn normalized_locations(result: &Value) -> Vec<(String, Value)> {
    let mut locations: Vec<_> = locations(result)
        .into_iter()
        .map(|location| (location.uri, location.range))
        .collect();
    locations.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.to_string().cmp(&right.1.to_string()))
    });
    locations
}

fn assert_new_p_targets(
    definition: &Value,
    references: &Value,
    provider_uri: &str,
    consumer_uri: &str,
    consumer_text: &str,
) {
    let definitions = normalized_locations(definition);
    assert_eq!(definitions.len(), 1, "p has one current definition");
    assert_eq!(definitions[0].0, provider_uri);
    assert_eq!(
        definitions[0].1["range"]["start"],
        json!({ "line": 3, "character": 5 }),
        "definition points to the post-rename p declaration"
    );

    let references = normalized_locations(references);
    let (consumer_line, consumer_character) = position_of_name(consumer_text, "p");
    assert!(references.iter().any(|(uri, range)| {
        uri == provider_uri && range["start"] == json!({ "line": 3, "character": 5 })
    }));
    assert!(references.iter().any(|(uri, range)| {
        uri == consumer_uri
            && range["start"] == json!({ "line": consumer_line, "character": consumer_character })
    }));
}

fn has_code(diagnostics: &[Value], code: &str) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.get("code").and_then(Value::as_str) == Some(code))
}

fn query_p(lsp: &mut Lsp, uri: &str, text: &str) -> (Value, Value) {
    let (line, character) = position_of_name(text, "p");
    (
        lsp.definition(uri, line, character),
        lsp.references(uri, line, character, true),
    )
}

fn assert_same_answers(
    warm_definition: &Value,
    warm_references: &Value,
    warm_diagnostics: &[Value],
    fresh_definition: &Value,
    fresh_references: &Value,
    fresh_diagnostics: &[Value],
) {
    assert_eq!(
        normalized_locations(warm_definition),
        normalized_locations(fresh_definition),
        "warm and fresh definitions must agree for the same source revision"
    );
    assert_eq!(
        normalized_locations(warm_references),
        normalized_locations(fresh_references),
        "warm and fresh references must agree for the same source revision"
    );
    assert_eq!(
        warm_diagnostics, fresh_diagnostics,
        "warm and fresh diagnostic payloads must agree for the same source revision"
    );
}

fn assert_old_revision_is_retired(lsp: &mut Lsp, uri: &str, text: &str, diagnostics: &[Value]) {
    let (definition, _references) = query_p(lsp, uri, text);
    assert!(
        normalized_locations(&definition).is_empty(),
        "the old revision retires p before the consumer body"
    );
    assert!(
        has_code(diagnostics, "W123"),
        "the old revision reports the unresolved p call: {diagnostics:?}"
    );

    let (line, character) = position_of_name(text, "q");
    let q = lsp.definition(uri, line, character);
    assert!(
        !normalized_locations(&q).is_empty(),
        "q is a live control target after rename"
    );
}

fn await_completed_revision_diagnostics(
    lsp: &Lsp,
    uri: &str,
    version: i64,
    cursor: usize,
) -> Vec<Value> {
    // The server may publish a provisional fast-tier result before this marker.
    // Select the first completed update after the caller's cursor, then pair it
    // with the nearest preceding exact-URI publish at the expected document
    // version; do not return the earlier provisional notification.
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
        .expect("awaited per-URI workspace update is present in notification snapshot");

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
        .expect("completed per-URI update has a preceding diagnostic publish")
}

fn open_and_await_completed_diagnostics(lsp: &mut Lsp, uri: &str, text: &str) -> Vec<Value> {
    let cursor = lsp.notification_cursor();
    lsp.open_document(uri, text);
    await_completed_revision_diagnostics(lsp, uri, 1, cursor)
}

fn await_workspace_scan(lsp: &Lsp) {
    lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
}

fn new_workspace_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-naming-lifecycle-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&root).expect("create lifecycle workspace");
    root
}

fn file_uri(path: &Path) -> String {
    tcl_lsp_server::canonical_file_uri(path)
        .expect("fixture path has a representable file URI")
        .to_string()
}

#[test]
fn warm_lifecycle_answers_match_fresh_after_open_and_closed_provider_changes() {
    let (old, new) = lifecycle_sources();

    // Both sources execute successfully: the p/q call sites are in procedure
    // bodies that the fixture never invokes, so old p may be retired.
    // Open-buffer leg: use the exact N12 before/after pair through didChange.
    let uri = unique_uri("tcl");
    let mut warm = Lsp::tcl();
    let old_diagnostics = open_and_await_completed_diagnostics(&mut warm, &uri, &old);
    assert_old_revision_is_retired(&mut warm, &uri, &old, &old_diagnostics);

    warm.clear_notifications();
    let cursor = warm.notification_cursor();
    warm.replace_document(&uri, 2, &new);
    let new_diagnostics = await_completed_revision_diagnostics(&warm, &uri, 2, cursor);
    let (new_definition, new_references) = query_p(&mut warm, &uri, &new);
    assert_new_p_targets(&new_definition, &new_references, &uri, &uri, &new);
    assert!(
        !has_code(&new_diagnostics, "W123"),
        "the new revision resolves p and clears its unknown-command diagnostic: {new_diagnostics:?}"
    );

    let mut fresh = Lsp::tcl();
    let fresh_diagnostics = open_and_await_completed_diagnostics(&mut fresh, &uri, &new);
    let (fresh_definition, fresh_references) = query_p(&mut fresh, &uri, &new);
    assert_same_answers(
        &new_definition,
        &new_references,
        &new_diagnostics,
        &fresh_definition,
        &fresh_references,
        &fresh_diagnostics,
    );

    // Closed-provider leg: only the watched file changes; the consumer stays
    // open at one text revision throughout.
    let root = new_workspace_root();
    let provider_path = root.join("provider.tcl");
    let consumer_path = root.join("consumer.tcl");
    let old_provider = old.strip_suffix(CONSUMERS).expect("old provider suffix");
    let new_provider = new.strip_suffix(CONSUMERS).expect("new provider suffix");
    assert_eq!(old_provider.find(RENAME), new_provider.find(RENAME));
    let consumer = format!("source provider.tcl\n{CONSUMERS}");
    std::fs::write(&provider_path, old_provider).expect("write old closed provider");
    std::fs::write(&consumer_path, &consumer).expect("write consumer source");
    let provider_uri = file_uri(&provider_path);
    let consumer_uri = file_uri(&consumer_path);

    let mut warm = Lsp::at_workspace_root(&root);
    await_workspace_scan(&warm);
    let old_diagnostics = open_and_await_completed_diagnostics(&mut warm, &consumer_uri, &consumer);
    assert_old_revision_is_retired(&mut warm, &consumer_uri, &consumer, &old_diagnostics);

    // Clear first, then take the cursor. The workspace update marker identifies
    // the consumer URI but carries no source revision. An in-flight stale
    // publish could still arrive after this boundary; the test consumes the
    // first exact-URI publish and asserts the expected changed diagnostics and
    // targets, so stale state fails instead of being hidden by a retry.
    warm.clear_notifications();
    let cursor = warm.notification_cursor();
    std::fs::write(&provider_path, new_provider).expect("write new closed provider");
    warm.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": provider_uri, "type": 2 }] }),
    );
    let new_diagnostics = await_completed_revision_diagnostics(&warm, &consumer_uri, 1, cursor);
    let (new_definition, new_references) = query_p(&mut warm, &consumer_uri, &consumer);
    assert_new_p_targets(
        &new_definition,
        &new_references,
        &provider_uri,
        &consumer_uri,
        &consumer,
    );
    assert!(
        !has_code(&new_diagnostics, "W123"),
        "the watched provider revision resolves p and clears W123: {new_diagnostics:?}"
    );

    let mut fresh = Lsp::at_workspace_root(&root);
    await_workspace_scan(&fresh);
    let fresh_diagnostics =
        open_and_await_completed_diagnostics(&mut fresh, &consumer_uri, &consumer);
    let (fresh_definition, fresh_references) = query_p(&mut fresh, &consumer_uri, &consumer);
    assert_same_answers(
        &new_definition,
        &new_references,
        &new_diagnostics,
        &fresh_definition,
        &fresh_references,
        &fresh_diagnostics,
    );

    drop(fresh);
    drop(warm);
    std::fs::remove_dir_all(&root).ok();
}
