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

//! Signature help, end-to-end against the packaged server.

use crate::common::{Lsp, unique_uri};

use serde_json::Value;

/// The `documentation` text of a signature (string or `MarkupContent`).
fn doc_text(sig: &Value) -> String {
    match sig.get("documentation") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Object(o)) => o
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        _ => String::new(),
    }
}

/// True for a null/absent signature-help result.
fn is_empty(result: &Value) -> bool {
    match result {
        Value::Object(o) => o
            .get("signatures")
            .and_then(Value::as_array)
            .is_none_or(std::vec::Vec::is_empty),
        _ => true,
    }
}

fn signatures(result: &Value) -> Vec<Value> {
    result
        .get("signatures")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn params(sig: &Value) -> Vec<Value> {
    sig.get("parameters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn label(sig: &Value) -> String {
    sig.get("label")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

// -- TestSignatureHelpUserProcs ------------------------------------------

#[test]
fn proc_first_arg() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name greeting} { puts \"$greeting $name\" }\ngreet World\n";
    lsp.open_ready(&uri, src);
    let result = lsp.signature_help(&uri, 1, 12);
    assert!(!is_empty(&result));
    let sigs = signatures(&result);
    assert_eq!(sigs.len(), 1);
    assert!(label(&sigs[0]).contains("greet"));
    assert_eq!(params(&sigs[0]).len(), 2);
}

#[test]
fn proc_active_param_tracking() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc add {a b} { expr {$a + $b} }\nadd 1 ");
    let result = lsp.signature_help(&uri, 1, 6);
    assert!(!is_empty(&result));
    assert_eq!(
        result.get("activeParameter").and_then(Value::as_i64),
        Some(1)
    );
}

#[test]
fn proc_with_defaults_marks_optional() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name {greeting Hello}} { puts \"$greeting $name\" }\ngreet World\n";
    lsp.open_ready(&uri, src);
    let result = lsp.signature_help(&uri, 1, 12);
    let ps = params(&signatures(&result)[0]);
    assert_eq!(ps.len(), 2);
    assert!(
        ps[1]
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("")
            .contains('?')
    );
}

// -- TestSignatureHelpBuiltins -------------------------------------------

#[test]
fn set_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x ");
    let result = lsp.signature_help(&uri, 0, 6);
    assert!(!is_empty(&result));
    assert!(label(&signatures(&result)[0]).contains("set"));
}

#[test]
fn cursor_on_command_name_returns_none() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set");
    assert!(is_empty(&lsp.signature_help(&uri, 0, 2)));
}

#[test]
fn unknown_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "nonexistent_command arg ");
    assert!(is_empty(&lsp.signature_help(&uri, 0, 24)));
}

#[test]
fn empty_file() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "");
    assert!(is_empty(&lsp.signature_help(&uri, 0, 0)));
}

// -- TestSignatureHelpDocumentation --------------------------------------

#[test]
fn set_has_documentation() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x ");
    let result = lsp.signature_help(&uri, 0, 6);
    assert!(doc_text(&signatures(&result)[0]).contains("With one argument"));
}

#[test]
fn socket_has_documentation() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "socket localhost ");
    let result = lsp.signature_help(&uri, 0, 17);
    let doc = doc_text(&signatures(&result)[0]);
    assert!(
        doc.contains("listening socket") || doc.to_lowercase().contains("server"),
        "doc: {doc:?}"
    );
}

#[test]
fn proc_still_has_documentation() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"Hello $name\" }\ngreet World ",
    );
    let result = lsp.signature_help(&uri, 1, 12);
    assert!(label(&signatures(&result)[0]).starts_with("greet"));
}

#[test]
fn proc_doc_comment_in_signature_help() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# Says hello\nproc greet {name} { puts \"Hello $name\" }\ngreet World ";
    lsp.open_ready(&uri, src);
    let result = lsp.signature_help(&uri, 2, 12);
    assert!(doc_text(&signatures(&result)[0]).contains("Says hello"));
}

// -- TestSignatureHelpSubcommands ----------------------------------------

#[test]
fn string_length() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "string length ");
    let result = lsp.signature_help(&uri, 0, 14);
    assert!(!is_empty(&result));
    assert!(
        signatures(&result)
            .iter()
            .any(|s| label(s) == "string length string")
    );
}

#[test]
fn string_index_subcommand() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "string index ");
    let result = lsp.signature_help(&uri, 0, 13);
    assert!(!is_empty(&result));
    assert!(
        signatures(&result)
            .iter()
            .any(|s| label(s) == "string index string charIndex")
    );
}

#[test]
fn string_unknown_subcommand_no_signature() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "string bogus ");
    assert!(is_empty(&lsp.signature_help(&uri, 0, 13)));
}
