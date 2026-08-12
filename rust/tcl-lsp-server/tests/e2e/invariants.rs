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

//! Universal invariants + adversarial robustness, end-to-end against the server.
//! These assert properties that must hold for *every* provider on *every*
//! document: well-formed ranges, disjoint `WorkspaceEdit`s, and that hostile
//! inputs never wedge/crash the server or emit a malformed span.

use std::time::Duration;

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};

/// A corpus spanning clean code, every multibyte/EOL hazard, and the recovery
/// paths. Reused by the range-invariant and robustness batteries.
fn corpus() -> Vec<(&'static str, String)> {
    let mut large = String::new();
    for i in 0..400 {
        use std::fmt::Write;
        let _ = writeln!(large, "proc p{i} {{a b}} {{ return [expr {{$a + $b}}] }}");
    }
    let mut v: Vec<(&'static str, String)> = vec![
        (
            "clean",
            "proc greet {name} {\n    puts \"Hello $name\"\n}\ngreet World\n".to_owned(),
        ),
        (
            "vars",
            "set x 1\nset y $x\nputs [expr {$x + $y}]\n".to_owned(),
        ),
        (
            "multibyte",
            "set s \"café résumé naïve\"\nputs $s\nproc f {s} { return $s }\nf $s\n".to_owned(),
        ),
        (
            "emoji",
            "set e \"😀 🚀 🐫 done\"\nputs $e\nset n 1\n".to_owned(),
        ),
        ("unicode_ident", "set 日本語 1\nputs ${日本語}\n".to_owned()),
        (
            "crlf",
            "proc p {} {\r\n    set x 1\r\n}\r\np\r\n".to_owned(),
        ),
        ("bom", "\u{feff}puts hello\nset x 1\n".to_owned()),
        (
            "unterminated_bracket",
            "set x [foo bar\nproc recovered {} {}\nputs hi\n".to_owned(),
        ),
        (
            "unterminated_brace",
            "proc p {} {\n  set y [foo\n}\nset z 1\n".to_owned(),
        ),
        ("unterminated_quote", "set s \"open\nputs done\n".to_owned()),
        (
            "deep_nesting",
            "set x [a [b [c [d [e [f [g\nputs tail\n".to_owned(),
        ),
        ("empty", String::new()),
        ("blank_lines", "\n\n\n".to_owned()),
        ("only_comment", "# just a comment\n".to_owned()),
        ("large", large),
    ];
    v.sort_by(|a, b| a.0.cmp(b.0));
    v
}

/// Run a broad request battery against `uri`; return `(name, result)` pairs.
///
/// Every request must return (not raise / not time out). Positions are chosen to
/// land on real tokens in the smaller corpus entries and are harmless elsewhere.
fn exercise_all_providers(lsp: &mut Lsp, uri: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("documentSymbol", lsp.document_symbols(uri)),
        ("foldingRange", lsp.folding_range(uri)),
        ("semanticTokens", lsp.semantic_tokens(uri)),
        ("hover@0,1", lsp.hover(uri, 0, 1)),
        ("definition@0,1", lsp.definition(uri, 0, 1)),
        ("references@0,5", lsp.references(uri, 0, 5, true)),
        ("highlight@0,5", lsp.document_highlight(uri, 0, 5)),
        ("completion@0,1", lsp.completion(uri, 0, 1)),
        ("signatureHelp@1,8", lsp.signature_help(uri, 1, 8)),
        (
            "selectionRange@0,1",
            lsp.selection_range(uri, json!([{ "line": 0, "character": 1 }])),
        ),
        (
            "codeAction@0,0-1,0",
            lsp.code_actions(
                uri,
                json!({
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 1, "character": 0 },
                }),
                json!([]),
            ),
        ),
        ("documentLink", lsp.document_links(uri)),
        ("codeLens", lsp.code_lens(uri)),
    ]
}

// -- TestRangeInvariants -------------------------------------------------

/// For one corpus entry: every Range from every provider is well-formed, and
/// (implicitly) the server neither hangs nor errors on the input — the request
/// helpers panic on a JSON-RPC error or time out, so reaching the end *is* the
/// "survives and responds" assertion. Each entry is its own `#[test]` so the
/// 15 (formerly two serial corpus loops) run in parallel.
fn provider_ranges_well_formed_for(idx: usize) {
    let mut lsp = Lsp::tcl();
    let entries = corpus();
    let (name, source) = &entries[idx];
    let uri = unique_uri("tcl");
    // `large` is 400 procs — the one corpus entry that is not the "tens of
    // lines" document `open_ready`'s 30 s backstop is sized for. A full
    // debug-build analysis of it, on a machine already running the rest of
    // the suite beside it, is legitimately tens of seconds; the harness
    // documents this case and asks such call sites to supply their own
    // backstop rather than the whole suite inheriting a worst-case number
    // (`common/mod.rs`, `open_ready_timeout`). Load scaling still applies on
    // top, but it keys off capacity, not off a sibling CI job saturating the
    // machine — which is what turned this into a red build rather than a slow
    // one. Sized by the document so a future large entry is covered too.
    if source.lines().count() > 100 {
        lsp.open_ready_timeout(&uri, source, Duration::from_secs(180));
    } else {
        lsp.open_ready(&uri, source);
    }
    let results = exercise_all_providers(&mut lsp, &uri);
    let mut violations: Vec<String> = Vec::new();
    for (req, res) in &results {
        violations.extend(range_violations(
            res,
            source,
            &format!("{name}/{req}"),
            false,
        ));
    }
    assert!(
        violations.is_empty(),
        "malformed ranges for corpus[{idx}] ({name}):\n{}",
        violations.join("\n")
    );
}

macro_rules! corpus_range_tests {
    ($($test:ident = $idx:literal;)+) => {
        /// Guard: the explicit per-entry list must cover the whole corpus, so a
        /// new corpus entry can't silently escape the invariant sweep.
        #[test]
        fn corpus_index_list_is_complete() {
            let covered = [$($idx),+];
            assert_eq!(
                covered.len(), corpus().len(),
                "corpus size changed — add/remove a corpus_range_tests entry"
            );
        }
        $( #[test] fn $test() { provider_ranges_well_formed_for($idx); } )+
    };
}

corpus_range_tests! {
    ranges_entry_00 = 0;  ranges_entry_01 = 1;  ranges_entry_02 = 2;
    ranges_entry_03 = 3;  ranges_entry_04 = 4;  ranges_entry_05 = 5;
    ranges_entry_06 = 6;  ranges_entry_07 = 7;  ranges_entry_08 = 8;
    ranges_entry_09 = 9;  ranges_entry_10 = 10; ranges_entry_11 = 11;
    ranges_entry_12 = 12; ranges_entry_13 = 13; ranges_entry_14 = 14;
}

// -- TestWorkspaceEditInvariants -----------------------------------------

#[test]
fn test_rename_edits_do_not_overlap() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc greet {name} { puts $name }\ngreet a\ngreet b\ngreet c\n";
    lsp.open_ready(&uri, src);
    let edit = lsp.rename(&uri, 0, 6, "welcome");
    let violations = workspace_edit_violations(&edit, "rename");
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn test_multisite_variable_rename_edits_are_disjoint() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "proc p {} {\n    set count 0\n    incr count\n    incr count\n    return $count\n}\n";
    lsp.open_ready(&uri, src);
    let edit = lsp.rename(&uri, 1, 8, "counter");
    let violations = workspace_edit_violations(&edit, "var-rename");
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn test_code_action_edits_do_not_overlap() {
    // W100 (unbraced expr) offers a safe wrap fix; its edits must be disjoint.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "if $a { puts yes }\nif $b { puts no }\n");
    let actions = lsp.code_actions(
        &uri,
        json!({
            "start": { "line": 0, "character": 0 },
            "end": { "line": 2, "character": 0 },
        }),
        json!([]),
    );
    let actions = actions.as_array().cloned().unwrap_or_default();
    for action in &actions {
        let edit = action.get("edit").cloned().unwrap_or(Value::Null);
        assert!(workspace_edit_violations(&edit, "code-action").is_empty());
    }
}

// -- TestRobustnessAdversarial -------------------------------------------

// The former `test_server_survives_and_responds` (a second serial loop over the
// same corpus with the same range-well-formedness check) is now covered
// per-entry by `corpus_range_tests!` above — each entry's `#[test]` proves both
// "ranges well-formed" and "server survives/responds" for that input.

#[test]
fn test_server_still_responsive_after_adversarial_burst() {
    // After hammering hostile documents, a normal request on a fresh doc must
    // still answer correctly — proves no document poisoned global state.
    let mut lsp = Lsp::tcl();
    let by_name: std::collections::HashMap<&str, String> = corpus().into_iter().collect();
    for name in ["deep_nesting", "unterminated_quote", "emoji", "large"] {
        let u = unique_uri("tcl");
        let src = by_name[name].clone();
        lsp.open_ready(&u, &src);
        let _ = exercise_all_providers(&mut lsp, &u);
    }
    let sane = unique_uri("tcl");
    lsp.open_ready(&sane, "puts hello\n");
    assert!(hover_text(&lsp.hover(&sane, 0, 2)).contains("puts"));
}
