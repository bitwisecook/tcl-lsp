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

//! Document highlight, end-to-end against the packaged server. Highlight kinds
//! come back as raw LSP integers: 1=Text, 2=Read, 3=Write.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

const TEXT: i64 = 1;
const READ: i64 = 2;
const WRITE: i64 = 3;

/// The document-highlight result at a position, as a list (never null).
fn highlights(lsp: &mut Lsp, uri: &str, line: u32, char: u32) -> Vec<Value> {
    match lsp.document_highlight(uri, line, char) {
        Value::Array(a) => a,
        _ => Vec::new(),
    }
}

/// A highlight's `kind`, defaulting to `TEXT` when absent.
fn kind_or_text(h: &Value) -> i64 {
    h.get("kind").and_then(Value::as_i64).unwrap_or(TEXT)
}

// -- TestDocumentHighlightProc -------------------------------------------

#[test]
fn test_highlights_proc_definition_and_calls() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name} { puts \"Hello $name\" }\ngreet World\ngreet Everyone\n";
    lsp.open_ready(&uri, src);
    let hls = highlights(&mut lsp, &uri, 0, 6);
    assert!(hls.len() >= 2, "{hls:?}");
    assert!(hls.iter().all(|h| kind_or_text(h) == TEXT), "{hls:?}");
}

#[test]
fn test_highlights_include_declaration() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} { return }\ngreet\ngreet\n");
    let result = Value::Array(highlights(&mut lsp, &uri, 0, 6));
    let lines = start_lines(&result);
    for expected in [0, 1, 2] {
        assert!(
            lines.contains(&expected),
            "missing line {expected} in {lines:?}"
        );
    }
}

#[test]
fn test_cursor_on_whitespace_returns_empty() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} { return }\n");
    assert!(highlights(&mut lsp, &uri, 0, 0).is_empty());
}

#[test]
fn test_no_duplicate_ranges() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} { return }\ngreet\n");
    let hls = highlights(&mut lsp, &uri, 0, 6);
    let keys: Vec<(i64, i64, i64, i64)> = hls
        .iter()
        .map(|h| {
            let r = &h["range"];
            (
                r["start"]["line"].as_i64().unwrap_or(0),
                r["start"]["character"].as_i64().unwrap_or(0),
                r["end"]["line"].as_i64().unwrap_or(0),
                r["end"]["character"].as_i64().unwrap_or(0),
            )
        })
        .collect();
    let unique: std::collections::BTreeSet<_> = keys.iter().copied().collect();
    assert_eq!(keys.len(), unique.len(), "{keys:?}");
}

// -- TestDocumentHighlightVariable ---------------------------------------

#[test]
fn test_highlights_variable_uses() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc sum {} {\n    set x 1\n    set y [expr {$x + 2}]\n    return $x\n}\n";
    lsp.open_ready(&uri, src);
    let hls = highlights(&mut lsp, &uri, 3, 12);
    assert!(!hls.is_empty(), "{hls:?}");
    assert!(
        hls.iter()
            .all(|h| h["range"]["start"]["line"].as_i64().unwrap_or(0) >= 1),
        "{hls:?}"
    );
}

#[test]
fn test_write_kind_on_definition_read_kind_on_use() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set x 1\n    return $x\n}\n";
    lsp.open_ready(&uri, src);
    let kinds: std::collections::BTreeSet<i64> = highlights(&mut lsp, &uri, 2, 12)
        .iter()
        .filter_map(|h| h.get("kind").and_then(Value::as_i64))
        .collect();
    assert!(kinds.contains(&WRITE), "{kinds:?}");
    assert!(kinds.contains(&READ), "{kinds:?}");
}

#[test]
fn test_scope_isolation_between_global_and_local() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x 1\nproc uses_x {} {\n    set x 2\n    puts $x\n}\nputs $x\n";
    lsp.open_ready(&uri, src);
    let result = Value::Array(highlights(&mut lsp, &uri, 3, 10));
    let lines = start_lines(&result);
    assert!(!lines.contains(&0), "{lines:?}");
    assert!(!lines.contains(&5), "{lines:?}");
    assert!(lines.contains(&2) && lines.contains(&3), "{lines:?}");
}
