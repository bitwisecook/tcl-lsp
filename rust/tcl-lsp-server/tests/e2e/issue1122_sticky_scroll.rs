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

//! End-to-end coverage for issue #1122 — VS Code sticky scroll goes dead
//! while the extension is enabled.
//!
//! The extension sets `stickyScroll.defaultModel: foldingProviderModel` for
//! Tcl languages, which makes VS Code build the sticky model from the syntax
//! folding provider and fall back to indentation only when that provider
//! yields **nothing**.  In `stickyScrollModelProvider.ts` the folding
//! candidate's `isModelValid` is `model !== null`, so an *empty* folding
//! result is a valid, terminal sticky model: the chain stops there and the
//! sticky bar stays blank for the rest of the session.  A `null` result
//! falls through to the indentation model instead.
//!
//! The server therefore must never answer `textDocument/foldingRange` with
//! an empty array — not when the feature toggle is off, and not when the
//! document simply has nothing to fold.
//!
//! The second half pins the range bounds VS Code applies on top: since
//! 1.105 a sticky candidate is rejected when its 1-based `foldEnd + 1`
//! exceeds `lineCount`, and a rejected *top-level* candidate prunes its
//! whole subtree.

use serde_json::{Value, json};

use crate::common::{Lsp, unique_uri};

/// A versioned Tcl module in the shape of the one in the issue #1122 report
/// — a top-level `oo::class create` with a superclass, instance variables, a
/// constructor, and methods, whose closing brace is the document's last line.
/// The reported module is proprietary, so every identifier here is invented;
/// only the structure is carried over.
const MODULE_TM: &str = concat!(
    "package require Tcl 8.6\n",
    "package provide widget 2.0\n",
    "\n",
    "oo::class create Widget {\n",
    "    superclass WidgetBase\n",
    "    variable label\n",
    "    variable count\n",
    "\n",
    "    constructor {text} {\n",
    "        set label $text\n",
    "        set count 0\n",
    "    }\n",
    "\n",
    "    method render {} {\n",
    "        incr count\n",
    "        return \"widget $label\"\n",
    "    }\n",
    "\n",
    "    method reset {} {\n",
    "        set count 0\n",
    "    }\n",
    "}\n",
);

/// `cfg.features.folding` is exactly `false`.
fn folding_disabled(cfg: &Value) -> bool {
    cfg.get("features").and_then(|f| f.get("folding")) == Some(&Value::Bool(false))
}

/// The number of lines the client holds for `text`.
///
/// A trailing newline opens a final, empty line, exactly as VS Code's
/// `TextModel` counts.  Splitting on `\n` is right for both LF and CRLF
/// content — a CRLF document has the same line count as its LF twin.
fn line_count(text: &str) -> i64 {
    i64::try_from(text.split('\n').count()).expect("line count fits i64")
}

/// Assert every folding range for the already-open `uri` lands inside a
/// document of `text`.
fn assert_folds_in_bounds(lsp: &mut Lsp, uri: &str, text: &str, label: &str) {
    let result = lsp.folding_range(uri);
    let ranges = result
        .as_array()
        .unwrap_or_else(|| panic!("{label}: expected folding ranges, got {result}"));
    assert!(
        !ranges.is_empty(),
        "{label}: expected folding ranges, got {result}"
    );
    let lines = line_count(text);
    for range in ranges {
        let start = range["startLine"].as_i64().expect("startLine");
        let end = range["endLine"].as_i64().expect("endLine");
        assert!(
            end < lines,
            "{label}: fold {start}..{end} ends past the last line \
             (line_count {lines}): {result}",
        );
        assert!(
            start < end,
            "{label}: degenerate fold {start}..{end}: {result}"
        );
    }
}

/// Assert every document symbol for the already-open `uri` ends inside a
/// document of `text`.
fn assert_symbol_ranges_in_bounds(lsp: &mut Lsp, uri: &str, text: &str, label: &str) {
    fn walk(symbol: &Value, last_line: i64, label: &str, all: &Value) {
        for key in ["range", "selectionRange"] {
            let end = symbol[key]["end"]["line"]
                .as_i64()
                .unwrap_or_else(|| panic!("{label}: {key}.end.line missing on {symbol}"));
            assert!(
                end <= last_line,
                "{label}: symbol {} {key} ends on line {end} but the last line \
                 is {last_line}: {all}",
                symbol["name"],
            );
        }
        if let Some(children) = symbol["children"].as_array() {
            for child in children {
                walk(child, last_line, label, all);
            }
        }
    }

    let symbols = lsp.document_symbols(uri);
    let top = symbols
        .as_array()
        .unwrap_or_else(|| panic!("{label}: expected document symbols, got {symbols}"));
    assert!(
        !top.is_empty(),
        "{label}: expected document symbols, got {symbols}"
    );
    let last_line = line_count(text) - 1;
    for symbol in top {
        walk(symbol, last_line, label, &symbols);
    }
}

#[test]
fn folding_returns_null_not_empty_when_the_feature_is_disabled() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc demo {} {\n    set x 1\n    set y 2\n}\n");
    assert!(
        lsp.folding_range(&uri)
            .as_array()
            .is_some_and(|a| !a.is_empty()),
        "baseline: the document must fold with the feature on",
    );

    lsp.apply_configuration_settle(
        json!({ "features": { "folding": false } }),
        &uri,
        folding_disabled,
    );

    assert_eq!(
        lsp.folding_range(&uri),
        Value::Null,
        "a disabled folding feature must answer `null`; an empty array is \
         accepted by VS Code as a valid, terminal sticky-scroll model and \
         blanks the sticky bar permanently (issue #1122)",
    );
}

#[test]
fn folding_returns_null_not_empty_for_a_document_with_nothing_to_fold() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 1\nset y 2\nputs $x\n");
    assert_eq!(
        lsp.folding_range(&uri),
        Value::Null,
        "a document with no foldable region must answer `null`, so sticky \
         scroll falls through to its indentation model (issue #1122)",
    );
}

#[test]
fn tcloo_module_folds_stay_inside_the_document() {
    // LF and CRLF (the report is from Windows), each with and without a
    // final newline, so the closing `}` is the last line in half the cases.
    for (label, text) in module_variants() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tm");
        lsp.open_ready(&uri, &text);
        assert_folds_in_bounds(&mut lsp, &uri, &text, &label);
    }
}

/// The four on-disk shapes the reported module can take.
fn module_variants() -> Vec<(String, String)> {
    let crlf = MODULE_TM.replace('\n', "\r\n");
    vec![
        ("lf, trailing newline".to_owned(), MODULE_TM.to_owned()),
        (
            "lf, closing brace at EOF".to_owned(),
            MODULE_TM.trim_end_matches('\n').to_owned(),
        ),
        ("crlf, trailing newline".to_owned(), crlf.clone()),
        (
            "crlf, closing brace at EOF".to_owned(),
            crlf.trim_end_matches(['\r', '\n']).to_owned(),
        ),
    ]
}

/// The sticky-scroll bounds contract, stated in the 0-based terms the wire
/// format uses.
///
/// VS Code converts a folding range to 1-based (`start + 1`, `end + 1`) and
/// then rejects the sticky candidate when `foldEnd + 1 > lineCount` — i.e.
/// 0-based `end_line + 2 > line_count`.  A *top-level* range that ends on
/// the document's last content line trips that unavoidably, and that is
/// VS Code's own bug: shortening a top-level fold to dodge it would break
/// the folding UI for every client that behaves. So this test does **not**
/// require `end_line + 2 <= line_count` everywhere.  What it does require is
/// that we never make the situation worse — no range may exceed the last
/// line — and that the nested member folds, the ones sticky scroll shows
/// while scrolling through a method body, clear the filter outright.
#[test]
fn tcloo_module_member_folds_survive_the_sticky_scroll_filter() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tm");
    lsp.open_ready(&uri, MODULE_TM);
    let result = lsp.folding_range(&uri);
    let ranges = result.as_array().expect("folding ranges").clone();
    let lines = line_count(MODULE_TM);

    let members: Vec<&Value> = ranges
        .iter()
        .filter(|r| r["startLine"].as_i64().unwrap_or(0) > 0)
        .collect();
    assert!(
        !members.is_empty(),
        "expected constructor/method folds nested inside the class: {result}",
    );
    for range in members {
        let start = range["startLine"].as_i64().expect("startLine");
        let end = range["endLine"].as_i64().expect("endLine");
        assert!(
            end + 2 <= lines,
            "member fold {start}..{end} would be rejected by VS Code's sticky \
             filter (line_count {lines}): {result}",
        );
    }
}

/// Outline-model sticky scroll builds `StickyRange(selectionRange.start,
/// range.end)` and drops any symbol failing `TextModel.isValidRange` — while
/// breadcrumbs, which read the same symbols without that check, keep working.
/// That asymmetry is what masked #1122 for the dotted-language-id dialects
/// and non-VS Code editors, so the bound is pinned end-to-end too.
#[test]
fn document_symbol_ranges_stay_inside_the_document() {
    for (label, text) in module_variants() {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tm");
        lsp.open_ready(&uri, &text);
        assert_symbol_ranges_in_bounds(&mut lsp, &uri, &text, &label);
    }
}

/// A script whose top level is control flow, not definitions — the case the
/// outline model has nothing to say about and the folding model must carry.
///
/// The `switch` is the shape from issue #1216: its arms are a single
/// clause-list argument, so before that fix the only fold covering a line
/// inside an arm was the whole `switch` block and the sticky bar pinned the
/// `switch` header rather than the arm's own pattern line.
const CONTROL_FLOW_SCRIPT: &str = concat!(
    "foreach item $items {\n",              // 0
    "    if {[string match a* $item]} {\n", // 1
    "        puts first\n",                 // 2
    "        puts second\n",                // 3
    "    }\n",                              // 4
    "    switch -glob -- $item {\n",        // 5
    "        \"admin*\" {\n",               // 6
    "            puts denied\n",            // 7
    "            puts logged\n",            // 8
    "            puts done\n",              // 9
    "        }\n",                          // 10
    "        default {\n",                  // 11
    "            puts allowed\n",           // 12
    "            puts logged\n",            // 13
    "            puts done\n",              // 14
    "        }\n",                          // 15
    "    }\n",                              // 16
    "}\n",                                  // 17
);

/// The chain of folds VS Code's sticky widget would stack for a viewport whose
/// top line is 0-based `line`, outermost first, after applying its 1.105
/// candidate filter (`endLineNumber > startLineNumber + 1`, in 1-based Monaco
/// lines) — the same replication the extension's own sticky-scroll suite uses.
fn sticky_chain(ranges: &[Value], line: i64, lines: i64) -> Vec<(i64, i64)> {
    let mut covering: Vec<(i64, i64)> = ranges
        .iter()
        .filter_map(|r| Some((r["startLine"].as_i64()?, r["endLine"].as_i64()?)))
        // collectSyntaxRanges converts to 1-based and drops out-of-bounds …
        .map(|(s, e)| (s + 1, e + 1))
        .filter(|&(s, e)| e > s && e <= lines)
        // … then the candidate filter drops anything spanning no more than its
        // own header line plus one.
        .filter(|&(s, e)| e > s + 1)
        .filter(|&(s, e)| line + 1 >= s && line < e)
        .collect();
    covering.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    covering
}

#[test]
fn a_control_flow_script_pins_the_innermost_block_it_is_scrolled_into() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, CONTROL_FLOW_SCRIPT);
    let result = lsp.folding_range(&uri);
    let ranges = result.as_array().expect("folding ranges").clone();
    let lines = line_count(CONTROL_FLOW_SCRIPT);

    // Inside the `admin*` arm: the chain must reach the arm itself, not stop
    // at the enclosing `switch`.
    let chain = sticky_chain(&ranges, 8, lines);
    assert_eq!(
        chain.first().copied(),
        Some((1, 17)),
        "the outer foreach must be the outermost pinned block: {result}"
    );
    assert_eq!(
        chain.last().copied(),
        Some((7, 10)),
        "the `admin*` arm must be the innermost pinned block: {result}"
    );

    // Inside the `default` arm: same, for the other arm.
    let chain = sticky_chain(&ranges, 13, lines);
    assert_eq!(
        chain.last().copied(),
        Some((12, 15)),
        "the `default` arm must be the innermost pinned block: {result}"
    );

    // Inside the `if` body: the chain reaches the `if`, and the `switch` — a
    // sibling, not an ancestor — is not in it.
    let chain = sticky_chain(&ranges, 3, lines);
    assert_eq!(
        chain.last().copied(),
        Some((2, 4)),
        "the `if` body must be the innermost pinned block: {result}"
    );
    assert!(
        !chain.iter().any(|&(s, _)| s == 6),
        "a sibling switch must not be pinned: {chain:?}"
    );
}
