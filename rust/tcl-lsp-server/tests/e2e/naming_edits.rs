// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Apply actual protocol edits with UTF-16 coordinates, then execute C Tcl.

use crate::common::Lsp;
use crate::common::helpers::{locations, rename_edits};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
const PROVIDER: &str =
    "namespace eval ::space {\n    proc saluté {value} {return \"ok:$value\"}\n}\n";
const CALLER: &str = "source [file join [file dirname [info script]] provider.tcl]\nset marker \"😀\"; puts [::space::saluté payload]\n";

fn file_uri(path: &Path) -> String {
    tcl_lsp_server::canonical_file_uri(path)
        .expect("representable edit fixture URI")
        .to_string()
}

fn fixture_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-naming-edit-{}-{label}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create edit fixture workspace");
    root
}

fn position(source: &str, needle: &str) -> (u32, u32) {
    let offset = source.find(needle).expect("fixture name occurrence");
    let index = tcl_lexer::LineIndex::new_lsp(source);
    let position = index.position_at_utf16(u32::try_from(offset).unwrap(), source);
    (position.line, position.character.get())
}

fn byte_offset(source: &str, position: &Value) -> usize {
    let line = u32::try_from(position["line"].as_u64().expect("line")).unwrap();
    let column = u32::try_from(position["character"].as_u64().expect("UTF16 column")).unwrap();
    let index = tcl_lexer::LineIndex::new_lsp(source);
    let offset = index.offset_at_utf16(line, tcl_lexer::Utf16Col::new(column), source);
    let roundtrip = index.position_at_utf16(offset, source);
    assert_eq!(roundtrip.line, line, "invalid edit line");
    assert_eq!(roundtrip.character.get(), column, "invalid UTF16 boundary");
    usize::try_from(offset).unwrap()
}

fn apply_workspace_edit(
    result: &Value,
    sources: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let edits = rename_edits(result);
    assert!(
        !edits.is_empty(),
        "a positive rename must carry complete edits: {result}"
    );
    let mut updated = sources.clone();
    for (uri, edits) in edits {
        let original = sources
            .get(&uri)
            .expect("edit targets a captured source URI");
        let mut edits = edits
            .iter()
            .map(|edit| {
                (
                    byte_offset(original, &edit["range"]["start"]),
                    byte_offset(original, &edit["range"]["end"]),
                    edit["newText"].as_str().expect("text edit replacement"),
                )
            })
            .collect::<Vec<_>>();
        edits.sort_by_key(|(start, end, _)| (*start, *end));
        assert!(edits.iter().all(|(start, end, _)| start < end));
        assert!(
            edits.windows(2).all(|pair| pair[0].1 <= pair[1].0),
            "overlapping or duplicated edits"
        );
        let changed = updated.get_mut(&uri).unwrap();
        for (start, end, replacement) in edits.into_iter().rev() {
            changed.replace_range(start..end, replacement);
        }
    }
    updated
}

fn oracle(path: &Path) -> Option<tcl_test_support::ScriptOutcome> {
    let interpreter = tcl_test_support::witness_tclsh(tcl_dialect::TclVersion::V8_6)?;
    Some(
        tcl_test_support::run_script_file(&interpreter.path, path)
            .expect("matching C8.6 execution"),
    )
}

fn assert_execution_equal(
    before: Option<&tcl_test_support::ScriptOutcome>,
    after: Option<&tcl_test_support::ScriptOutcome>,
    expected: &[u8],
) {
    assert_eq!(
        before.is_some(),
        after.is_some(),
        "oracle availability is stable"
    );
    if let Some((before, after)) = before.zip(after) {
        assert!(
            before.success() && after.success(),
            "before={before:?}, after={after:?}"
        );
        assert_eq!(before.stdout, expected);
        assert_eq!(after.stdout, before.stdout);
        assert_eq!(after.stderr, before.stderr);
    }
}

fn write_sources(root: &Path, sources: &BTreeMap<String, String>) {
    for name in ["provider.tcl", "caller.tcl"] {
        let path = root.join(name);
        std::fs::write(&path, &sources[&file_uri(&path)]).expect("apply complete workspace source");
    }
}

fn publish_and_check_applied_sources(
    lsp: &mut Lsp,
    open_uri: &str,
    provider_uri: &str,
    changed: &BTreeMap<String, String>,
) {
    lsp.replace_document(open_uri, 2, &changed[open_uri]);
    let watched = changed
        .keys()
        .filter(|uri| uri.as_str() != open_uri)
        .map(|uri| json!({"uri":uri,"type":2}))
        .collect::<Vec<_>>();
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes":watched}),
    );
    let _ = lsp.current_complete_diagnostics(open_uri, 2, &changed[open_uri], TIMEOUT);
    let (line, character) = position(&changed[open_uri], "welcomeé");
    let definitions = locations(&lsp.definition(open_uri, line, character));
    assert_eq!(definitions.len(), 1, "first applied-source definition");
    assert_eq!(definitions[0].uri, provider_uri);
    assert_eq!(
        definitions[0].range["start"],
        json!({"line":1,"character":9})
    );
    let references = locations(&lsp.references(open_uri, line, character, true));
    assert_eq!(references.len(), 2, "first applied-source references");
    for reference in references {
        let source = &changed[&reference.uri];
        let start = byte_offset(source, &reference.range["start"]);
        let end = byte_offset(source, &reference.range["end"]);
        assert!(source[start..end].contains("welcomeé"));
    }
}

#[test]
fn declaration_and_call_origin_edits_agree_with_references_and_execute_across_open_closed_files() {
    for declaration_origin in [false, true] {
        let root = fixture_root(if declaration_origin {
            "declaration"
        } else {
            "call"
        });
        let provider_path = root.join("provider.tcl");
        let caller_path = root.join("caller.tcl");
        let provider_uri = file_uri(&provider_path);
        let caller_uri = file_uri(&caller_path);
        let sources = BTreeMap::from([
            (provider_uri.clone(), PROVIDER.to_owned()),
            (caller_uri.clone(), CALLER.to_owned()),
        ]);
        write_sources(&root, &sources);
        let before = oracle(&caller_path);
        let mut lsp = Lsp::with_config_at_root(json!({"dialect":"tcl8.6"}), &root);
        lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
        let (uri, text) = if declaration_origin {
            (&provider_uri, PROVIDER)
        } else {
            (&caller_uri, CALLER)
        };
        // The other physical document remains closed throughout this leg.
        lsp.open_document(uri, text);
        let _ = lsp.current_complete_diagnostics(uri, 1, text, TIMEOUT);
        let (line, character) = position(text, "saluté");
        let definitions = locations(&lsp.definition(uri, line, character));
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].uri, provider_uri);
        assert_eq!(
            definitions[0].range["start"],
            json!({"line":1,"character":9})
        );
        let references = locations(&lsp.references(uri, line, character, true));
        assert_eq!(references.len(), 2, "exact declaration and call coverage");
        assert!(
            references
                .iter()
                .any(|reference| reference.uri == provider_uri)
        );
        assert!(
            references
                .iter()
                .any(|reference| reference.uri == caller_uri)
        );
        assert!(!lsp.prepare_rename(uri, line, character).is_null());
        let result = lsp.rename(uri, line, character, "welcomeé");
        let edits = rename_edits(&result);
        assert_eq!(
            edits.len(),
            2,
            "both physical files must be edited: {result}"
        );
        assert_eq!(edits.values().map(Vec::len).sum::<usize>(), 2);
        for (owner, edits) in &edits {
            for edit in edits {
                let source = &sources[owner];
                let start = byte_offset(source, &edit["range"]["start"]);
                let end = byte_offset(source, &edit["range"]["end"]);
                assert!(
                    references.iter().any(|reference| {
                        reference.uri == *owner
                            && byte_offset(source, &reference.range["start"]) <= start
                            && end <= byte_offset(source, &reference.range["end"])
                    }),
                    "every edit belongs to the same reported occurrence: {edit}"
                );
            }
        }
        if let Some(changes) = result["documentChanges"].as_array() {
            for change in changes {
                let version = &change["textDocument"]["version"];
                let owner = change["textDocument"]["uri"].as_str().unwrap();
                assert_eq!(version.as_i64(), (owner == uri.as_str()).then_some(1));
            }
        }
        let changed = apply_workspace_edit(&result, &sources);
        assert!(changed[&provider_uri].contains("proc welcomeé"));
        assert!(changed[&caller_uri].contains("::space::welcomeé payload"));
        assert!(!changed.values().any(|source| source.contains("saluté")));
        write_sources(&root, &changed);
        assert_execution_equal(
            before.as_ref(),
            oracle(&caller_path).as_ref(),
            b"ok:payload\n",
        );
        publish_and_check_applied_sources(&mut lsp, uri, &provider_uri, &changed);
        lsp.close_document(uri);
        drop(lsp);
        std::fs::remove_dir_all(root).expect("remove edit fixture sources");
    }
}

#[test]
fn class_and_method_collisions_are_atomic_typed_protocol_refusals() {
    for (label, source, needle, new_name, expected) in [
        (
            "class-collision",
            "oo::class create First {}\noo::class create Second {}\n",
            "First",
            "Second",
            "original-command-collision",
        ),
        (
            "method-collision",
            "oo::class create C {method first {} {return FIRST}; method second {} {return SECOND}}\n",
            "first",
            "second",
            "original-member-collision",
        ),
    ] {
        let mut lsp = Lsp::with_config(json!({"dialect":"tcl8.6"}));
        let uri = crate::common::unique_uri("tcl");
        lsp.open_document(&uri, source);
        let before = lsp.current_complete_diagnostics(&uri, 1, source, TIMEOUT);
        let (line, character) = position(source, needle);
        let response = lsp.request_response(
            "textDocument/rename",
            json!({"textDocument":{"uri":uri},"position":{"line":line,"character":character},"newName":new_name}),
            TIMEOUT,
        );
        assert_eq!(
            response["error"]["data"]["code"], expected,
            "{label}: {response}"
        );
        assert!(rename_edits(&response["result"]).is_empty(), "{response}");
        assert_eq!(
            lsp.current_complete_diagnostics(&uri, 1, source, TIMEOUT),
            before,
            "a refusal leaves the current source unchanged"
        );
    }
}

#[test]
fn import_and_alias_rename_is_complete_or_returns_an_atomic_typed_refusal() {
    // Review N04 permits a complete transactional plan or a refusal. A
    // declaration-only edit must never escape as a successful WorkspaceEdit.
    let source = concat!(
        "namespace eval ::space {\n",
        "    proc saluté {value} {return \"ok:$value\"}\n",
        "    namespace export saluté\n",
        "}\n",
        "namespace import ::space::saluté\n",
        "interp alias {} route {} ::space::saluté\n",
        "puts [saluté imported]\n",
        "puts [route aliased]\n",
    );
    let root = fixture_root("import-alias");
    let path = root.join("root.tcl");
    std::fs::write(&path, source).expect("write import/alias source");
    let uri = file_uri(&path);
    let before = oracle(&path);
    let mut lsp = Lsp::with_config_at_root(json!({"dialect":"tcl8.6"}), &root);
    lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
    lsp.open_document(&uri, source);
    let _ = lsp.current_complete_diagnostics(&uri, 1, source, TIMEOUT);
    let (line, character) = position(source, "saluté");
    let definitions = locations(&lsp.definition(&uri, line, character));
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].uri, uri);
    let response = lsp.request_response(
        "textDocument/rename",
        json!({"textDocument":{"uri":uri},"position":{"line":line,"character":character},"newName":"welcomeé"}),
        TIMEOUT,
    );
    if let Some(error) = response.get("error") {
        assert!(
            matches!(
                error["data"]["code"].as_str(),
                Some(
                    "unclosed-original-name-link-edits"
                        | "incomplete-original-command-coverage"
                        | "unknown-completed-command-world"
                )
            ),
            "unrelated errors cannot satisfy an atomic rename refusal: {response}"
        );
        assert!(response.get("result").is_none_or(Value::is_null));
    } else {
        let updated = apply_workspace_edit(
            &response["result"],
            &BTreeMap::from([(uri.clone(), source.to_owned())]),
        );
        let changed = &updated[&uri];
        assert!(
            !changed.contains("saluté"),
            "all declaration/import/export/alias spelling obligations: {changed}"
        );
        assert!(changed.contains("proc welcomeé"));
        assert!(
            changed.contains("puts [route aliased]"),
            "the alias's own name stays distinct"
        );
        std::fs::write(&path, changed).expect("apply complete import/alias rename");
    }
    assert_execution_equal(
        before.as_ref(),
        oracle(&path).as_ref(),
        b"ok:imported\nok:aliased\n",
    );
    lsp.close_document(&uri);
    drop(lsp);
    std::fs::remove_dir_all(root).expect("remove import/alias fixture sources");
}

#[test]
fn shadowed_builtin_identity_agrees_with_diagnostics_and_enabled_transforms() {
    let source = concat!(
        "proc llength {value} {return 77}\n",
        "proc expr {args} {return SHADOW}\n",
        "set n [expr 3+4]\n",
        "puts $n\n",
        "puts [llength [list a b c]]\n",
    );
    let root = fixture_root("shadowed-transforms");
    let path = root.join("root.tcl");
    std::fs::write(&path, source).expect("write shadowed-handler fixture");
    let uri = file_uri(&path);
    let before = oracle(&path);
    let mut lsp = Lsp::with_config_at_root(json!({"dialect":"tcl8.6"}), &root);
    lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
    lsp.open_document(&uri, source);
    let diagnostics = lsp.current_complete_diagnostics(&uri, 1, source, TIMEOUT);
    assert!(
        !diagnostics.iter().any(|finding| finding["code"] == "W100"),
        "the custom expr declaration cannot donate builtin expression diagnostics: {diagnostics:?}"
    );
    for (needle, declaration_line) in [("llength [list", 0), ("expr 3+4", 1)] {
        let (line, character) = position(source, needle);
        let targets = locations(&lsp.definition(&uri, line, character));
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].uri, uri);
        assert_eq!(targets[0].range["start"]["line"], declaration_line);
        assert_eq!(
            locations(&lsp.references(&uri, line, character, true)).len(),
            2
        );
    }
    for (command, arguments) in [
        ("tcl-lsp.fixAllSafeIssues", json!([uri])),
        ("tcl-lsp.optimiseDocument", json!([uri, "full"])),
        ("tcl-lsp.minifyDocument", json!([uri, false, false, false])),
    ] {
        let result = lsp.execute_command(command, arguments);
        let transformed = result["source"]
            .as_str()
            .expect("actual full transform source");
        if command == "tcl-lsp.fixAllSafeIssues" {
            assert!(
                !result["applied"]
                    .as_array()
                    .expect("applied fix inventory")
                    .iter()
                    .any(|fix| fix["code"] == "W100")
            );
        }
        let proposed = root.join("transformed.tcl");
        std::fs::write(&proposed, transformed).expect("apply actual transform output");
        assert_execution_equal(before.as_ref(), oracle(&proposed).as_ref(), b"SHADOW\n77\n");
    }
    lsp.close_document(&uri);
    drop(lsp);
    std::fs::remove_dir_all(root).expect("remove shadowed-handler fixture sources");
}
