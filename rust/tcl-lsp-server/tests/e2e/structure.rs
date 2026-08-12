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

//! Folding ranges, selection ranges, and workspace symbols, end-to-end.
//!
//! The folding and selection-range cases mirror the VS Code
//! `foldingRanges.test.ts` / `selectionRange.test.ts` scenarios, so a gap
//! between the extension host and the raw protocol is caught here too.

use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};
use std::collections::BTreeSet;

const FUNCTION: i64 = 12;

/// The `(startLine, endLine)` span set of a folding-range result.
fn spans(result: &Value) -> BTreeSet<(i64, i64)> {
    result
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|r| {
                    (
                        r.get("startLine").and_then(Value::as_i64).unwrap_or(0),
                        r.get("endLine").and_then(Value::as_i64).unwrap_or(0),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A folding-range result normalised to an iterable array (`null` → empty).
fn folds(result: &Value) -> Vec<Value> {
    result.as_array().cloned().unwrap_or_default()
}

// -- TestFoldingRange ----------------------------------------------------

#[test]
fn proc_body_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} {\n    set x 1\n    return $x\n}\n");
    let ranges = folds(&lsp.folding_range(&uri));
    assert!(ranges.iter().any(|r| {
        r.get("startLine").and_then(Value::as_i64) == Some(0)
            && r.get("endLine").and_then(Value::as_i64).unwrap_or(0) >= 2
    }));
}

#[test]
fn no_folds_in_flat_file() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\nset y 2\n");
    assert!(folds(&lsp.folding_range(&uri)).is_empty());
}

// -- TestLineContinuationFolding -----------------------------------------
// Backslash line-continuation folding — issue #541, end-to-end.
//
// A command stretched across physical lines by trailing `\` joins is a single
// logical command; the provider folds the run down to its opening line. The
// look-alikes that are *not* continuations (an escaped `\\` is a literal
// backslash) must stay unfolded. The feature was dropped by the folding rewrite
// and restored in #541 — this pins the editor-visible result.

#[test]
fn backslash_continued_command_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "MyProcCall $var1 \\\n           $var2 \\\n           $var3\n",
    );
    // The three physical lines are one logical command; they fold (0..2).
    let s = spans(&lsp.folding_range(&uri));
    assert!(s.contains(&(0, 2)), "{s:?}");
}

#[test]
fn two_line_continuation_folds() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x $a \\\n    $b\n");
    let s = spans(&lsp.folding_range(&uri));
    assert!(s.contains(&(0, 1)), "{s:?}");
}

#[test]
fn separate_runs_fold_independently() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "foo $a \\\n   $b\nputs sep\nbar $c \\\n   $d\n");
    let s = spans(&lsp.folding_range(&uri));
    assert!(BTreeSet::from([(0, 1), (3, 4)]).is_subset(&s), "{s:?}");
}

#[test]
fn escaped_backslash_is_not_a_continuation() {
    // `puts foo\\` ends in a *literal* backslash (escaped), so the two lines are
    // separate commands and must not fold.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts foo\\\\\nputs bar\n");
    let s = spans(&lsp.folding_range(&uri));
    assert!(!s.contains(&(0, 1)), "{s:?}");
}

// -- TestSelectionRange --------------------------------------------------

#[test]
fn widens_from_inner_to_outer() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} {\n    set x 1\n    return $x\n}\n");
    let result = lsp.selection_range(&uri, json!([{ "line": 2, "character": 11 }]));
    let arr = result.as_array().cloned().unwrap_or_default();
    assert!(!arr.is_empty());
    let mut node = arr[0].clone();
    let mut depth = 0;
    loop {
        depth += 1;
        match node.get("parent").cloned() {
            Some(parent) if !parent.is_null() => node = parent,
            _ => break,
        }
    }
    assert!(depth >= 2);
}

// -- TestWorkspaceSymbols ------------------------------------------------

#[test]
fn find_proc() {
    let mut lsp = Lsp::tcl();
    let a = unique_uri("tcl");
    let b = unique_uri("tcl");
    lsp.open_ready(&a, "proc greet_uniquely_aaa {} { return }\n");
    lsp.open_ready(&b, "proc farewell_uniquely_bbb {} { return }\n");
    let result = lsp.workspace_symbols("greet_uniquely_aaa");
    let matched: Vec<&Value> = result
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|s| s.get("name").and_then(Value::as_str) == Some("greet_uniquely_aaa"))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(matched.len(), 1);
    assert_eq!(
        matched[0].get("kind").and_then(Value::as_i64),
        Some(FUNCTION)
    );
}

#[test]
fn empty_query_returns_all() {
    // An empty query returns every indexed symbol, so assert our uniquely-named
    // procs are present rather than pinning the exact result set.
    let mut lsp = Lsp::tcl();
    let a = unique_uri("tcl");
    let b = unique_uri("tcl");
    lsp.open_ready(&a, "proc foo_empty_q_aaa {} { return }\n");
    lsp.open_ready(&b, "proc bar_empty_q_bbb {} { return }\n");
    let result = lsp.workspace_symbols("");
    let names: BTreeSet<String> = result
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.get("name").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        BTreeSet::from(["foo_empty_q_aaa".to_owned(), "bar_empty_q_bbb".to_owned()])
            .is_subset(&names)
    );
}

#[test]
fn partial_match() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc calculate_total_zzz {} { return }\n");
    let result = lsp.workspace_symbols("calc");
    let found = result.as_array().is_some_and(|arr| {
        arr.iter()
            .any(|s| s.get("name").and_then(Value::as_str) == Some("calculate_total_zzz"))
    });
    assert!(found);
}

#[test]
fn no_match() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {} { return }\n");
    let result = lsp.workspace_symbols("zzz_no_such_symbol_qqq");
    let all_ne = result.as_array().is_none_or(|arr| {
        arr.iter()
            .all(|s| s.get("name").and_then(Value::as_str) != Some("zzz_no_such_symbol_qqq"))
    });
    assert!(all_ne);
}
