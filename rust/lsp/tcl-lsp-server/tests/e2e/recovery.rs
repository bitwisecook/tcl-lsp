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

//! Native port of `tests/lsp_e2e/test_recovery_e2e.py`.
//!
//! Error-recovery correctness contract — end-to-end against the packaged server.
//!
//! This suite is deliberately **implementation-agnostic**: it asserts the
//! *observable* recovery behaviour over the LSP protocol, never a Python internal
//! or a Python-specific diagnostic code. It is the contract the recovery engine
//! must satisfy in *any* implementation — Python before the port, Rust now — so
//! the same assertions run unchanged against the native server to prove the port
//! is correct.
//!
//! The stable contracts (each a property any correct recovery must have):
//!
//!   C1  an unterminated `[` / `"` / `{` is flagged with an error diagnostic;
//!   C2  recovery is non-fatal and bounded — the rest of the document is still
//!       analysed (a proc after the break appears in document symbols) instead of
//!       being swallowed to EOF;
//!   C3  the recovered token stream is well-formed (5-int groups, in-document
//!       positions) and re-lexes commands after the break;
//!   C4  the published diagnostic set never contains exact duplicates;
//!   C5  edits that introduce a break raise the error and edits that fix it clear
//!       it, version by version;
//!   C6  well-formed code produces no recovery error.
//!
//! Brittle, implementation-specific facts (the exact `E200` vs `E201` code, the
//! precise split offset) are intentionally NOT asserted here — they belong to the
//! language-internal unit/differential/fuzz suites.


use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::time::Duration;

/// Parser/recovery diagnostic family (unclosed/extra-delimiter). A conforming
/// implementation may pick any code in this family or word the message its own
/// way; the contract is only that *some* error in this space is reported.
const RECOVERY_CODES: &[&str] = &["E200", "E201", "E202", "E203", "E204", "E205", "E206"];
const RECOVERY_WORDS: &[&str] = &[
    "missing",
    "close",
    "unterminated",
    "unclosed",
    "unbalanced",
    "extra characters",
];

/// A diagnostic's `code`, normalised to a `String` (stringifying non-strings),
/// mirroring Python's `str(d.get("code"))`.
fn code_str(d: &Value) -> String {
    match d.get("code") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "None".to_owned(),
    }
}

/// The set of `code` strings carried by `diags` (mirrors Python `_codes`).
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags.iter().map(code_str).collect()
}

/// A diagnostic's `message` text (empty if absent).
fn message(d: &Value) -> String {
    d.get("message")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

/// Whether `d` is a recovery error: a code in the family, or an error-severity
/// diagnostic whose message uses recovery vocabulary (mirrors `_is_recovery_error`).
fn is_recovery_error(d: &Value) -> bool {
    let code = code_str(d);
    if RECOVERY_CODES.contains(&code.as_str()) {
        return true;
    }
    let msg = message(d).to_lowercase();
    let sev = d.get("severity").and_then(Value::as_i64);
    sev == Some(1) && RECOVERY_WORDS.iter().any(|w| msg.contains(w))
}

/// Whether any diagnostic is a recovery error (mirrors `_has_recovery_error`).
fn has_recovery_error(diags: &[Value]) -> bool {
    diags.iter().any(is_recovery_error)
}

/// The identity tuple of a diagnostic (code, start/end line/char, message),
/// rendered to a stable string — mirrors Python `_diag_identity`, used to detect
/// exact duplicates. (Serialised to a `String` because `serde_json::Value` is
/// not `Ord`/`Hash`.)
fn diag_identity(d: &Value) -> String {
    let rng = d.get("range").cloned().unwrap_or(Value::Null);
    let start = rng.get("start").cloned().unwrap_or(Value::Null);
    let end = rng.get("end").cloned().unwrap_or(Value::Null);
    format!(
        "{}|{}|{}|{}|{}|{}",
        code_str(d),
        start.get("line").cloned().unwrap_or(Value::Null),
        start.get("character").cloned().unwrap_or(Value::Null),
        end.get("line").cloned().unwrap_or(Value::Null),
        end.get("character").cloned().unwrap_or(Value::Null),
        d.get("message").cloned().unwrap_or(Value::Null),
    )
}

/// The `tokenTypes` legend from the server's `initialize` result (mirrors
/// `_legend`).
fn legend(lsp: &Lsp) -> Vec<String> {
    lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|v| v.as_str().unwrap_or("").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// A semantic token with its resolved type name (mirrors `_typed`): decode the
/// delta encoding, then map each token's numeric type through the legend.
struct TypedToken {
    line: i64,
    length: i64,
    ttype: String,
}

fn typed(lsp: &mut Lsp, uri: &str) -> Vec<TypedToken> {
    let leg = legend(lsp);
    let raw = lsp.semantic_tokens(uri);
    decode_semantic_tokens(&raw)
        .into_iter()
        .map(|tok| TypedToken {
            line: tok.line,
            length: tok.length,
            ttype: leg
                .get(tok.ttype as usize)
                .cloned()
                .unwrap_or_default(),
        })
        .collect()
}

/// The symbol names (depth-first) of a document-symbol result (mirrors
/// `_symbol_names`).
fn symbol_name_list(syms: &Value) -> Vec<String> {
    flatten_symbols(syms)
        .iter()
        .map(|s| {
            s.get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned()
        })
        .collect()
}

// --------------------------------------------------------------------------- //
// C1 — an unterminated delimiter is flagged
// --------------------------------------------------------------------------- //

#[test]
fn c1_unterminated_bracket() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo bar\nputs hi\n");
    assert!(
        has_recovery_error(&diags),
        "unterminated [ should be flagged; got {:?}",
        codes(&diags)
    );
}

#[test]
fn c1_unterminated_brace() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {} {\n  set y [foo\n}\nset\n");
    assert!(
        has_recovery_error(&diags),
        "unterminated delimiter expected; got {:?}",
        codes(&diags)
    );
}

#[test]
fn c1_well_formed_is_not_flagged() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo bar]\nputs hi\n");
    assert!(
        !has_recovery_error(&diags),
        "well-formed flagged a recovery error: {:?}",
        codes(&diags)
    );
}

// --------------------------------------------------------------------------- //
// C2 — recovery is non-fatal: the tail is still analysed
// --------------------------------------------------------------------------- //

#[test]
fn c2_proc_after_unterminated_bracket_is_a_symbol() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo\nproc recovered_after_bracket {} {}\n");
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_bracket"),
        "tail proc not recovered; symbols={names:?}"
    );
}

#[test]
fn c2_proc_after_unterminated_brace_body_is_a_symbol() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "namespace eval n {\nproc recovered_in_ns {} {}\n");
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_in_ns"),
        "tail proc not recovered; symbols={names:?}"
    );
}

#[test]
fn c2_proc_after_multiple_breaks_is_a_symbol() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set a [foo\nset b 2\nset c [bar\nproc recovered_after_two {} {}\n";
    lsp.open_ready(&uri, src);
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_two"),
        "tail proc not recovered; symbols={names:?}"
    );
}

#[test]
fn c2_bare_set_after_break_still_arity_errors() {
    // A bare `set` after a single break must be analysed as a command (and so
    // raise its arity error) — proof the tail is parsed as code, not text.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo bar\nset\n");
    assert!(
        codes(&diags).contains("E002"),
        "tail `set` should arity-error; got {:?}",
        codes(&diags)
    );
}

#[test]
fn c2_bare_set_after_unterminated_expr_brace_still_analysed() {
    // An unterminated braced *expression* (`if {…`) must also recover so the
    // following command is analysed — a bare `set` still arity-errors.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {$x > 5\nset\n");
    assert!(
        codes(&diags).contains("E002"),
        "tail `set` after `if {{` should arity-error; got {:?}",
        codes(&diags)
    );
}

// --------------------------------------------------------------------------- //
// C3 — recovered token stream is well-formed
// --------------------------------------------------------------------------- //

#[test]
fn c3_command_after_break_is_tokenised() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo bar\nputs hi\n");
    let toks = typed(&mut lsp, &uri);
    assert!(
        toks.iter().any(|t| t.line == 1 && t.ttype == "function"),
        "expected `puts` recovered as a function on line 1; got {:?}",
        toks.iter().map(|t| (t.line, &t.ttype)).collect::<Vec<_>>()
    );
}

#[test]
fn c3_nested_break_recovers_following_lines() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc p {} {\n  set x [foo\n}\nputs done\n");
    let toks = typed(&mut lsp, &uri);
    assert!(
        toks.iter().any(|t| t.line == 3 && t.ttype == "function"),
        "{:?}",
        toks.iter().map(|t| (t.line, &t.ttype)).collect::<Vec<_>>()
    );
}

#[test]
fn c3_tokens_well_formed_for_pathological_input() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n  set x [foo \"bar\n  if {1} {\n    puts [baz\n}\nputs end\n";
    lsp.open_ready(&uri, src);
    let raw = lsp.semantic_tokens(&uri);
    let data = raw.get("data").and_then(Value::as_array);
    assert!(
        data.map(|d| d.len() % 5 == 0).unwrap_or(false),
        "token data must be 5-int groups"
    );
    let n_lines = (src.matches('\n').count() + 1) as i64;
    for t in typed(&mut lsp, &uri) {
        assert!(
            (0..n_lines).contains(&t.line),
            "token line out of range: line={}",
            t.line
        );
        assert!(t.length >= 0);
    }
}

#[test]
fn c3_deeply_nested_unterminated_does_not_hang() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [a [b [c [d [e\nputs tail\n");
    // returned promptly, no hang/crash — `diags` is always a Vec here.
    let _: &Vec<Value> = &diags;
}

#[test]
fn c3_crlf_document_recovers() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo\r\nproc recovered_crlf {} {}\r\n");
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_crlf"),
        "CRLF tail not recovered; symbols={names:?}"
    );
}

// --------------------------------------------------------------------------- //
// C4 — no duplicate diagnostics
// --------------------------------------------------------------------------- //

#[test]
fn c4_no_exact_duplicate_published() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x \"\nif {1} {\n  puts [foo\n}\nset\n");
    let mut seen: BTreeSet<_> = BTreeSet::new();
    for d in &diags {
        let ident = diag_identity(d);
        assert!(seen.insert(ident.clone()), "duplicate diagnostic published: {ident:?}");
    }
}

// --------------------------------------------------------------------------- //
// Inert close-bracket veto (issue #560) — recovery must not fabricate a fix
// that leaves the command incomplete.
// --------------------------------------------------------------------------- //

// The `]` command/comment-break heuristics must veto an inert offset. For
//   set x [foo {bar
//   puts baz}
// the `puts` word sits *inside* the balanced brace word `{bar … baz}`, so
// inserting `]` after `bar` yields `set x [foo {bar]…}` which C Tcl reports as
// incomplete — an objectively wrong fix. Before #560 the bracket path lacked the
// inert-offset veto the brace path already had. Observably, the unterminated `[`
// is still flagged, the tail is still analysed as code, nothing is published
// twice, and the analysis doesn't hang. The mid-word look-alike (a literal `"`
// mid-word) still recovers normally.

#[test]
fn inert_brace_word_break_still_flags_and_recovers() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo {bar\nproc recovered_after_brace_word {} {}\n");
    assert!(
        has_recovery_error(&diags),
        "unterminated [ should flag; got {:?}",
        codes(&diags)
    );
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_brace_word"),
        "tail not recovered; symbols={names:?}"
    );
}

#[test]
fn inert_brace_word_break_no_duplicate_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo {bar\nputs baz}\nset\n");
    let mut seen: BTreeSet<_> = BTreeSet::new();
    for d in &diags {
        let ident = diag_identity(d);
        assert!(seen.insert(ident.clone()), "duplicate diagnostic published: {ident:?}");
    }
    // The trailing bare `set` is still parsed as a command → arity errors,
    // proving the tail is analysed as code rather than swallowed.
    assert!(
        codes(&diags).contains("E002"),
        "tail `set` should arity-error; got {:?}",
        codes(&diags)
    );
}

#[test]
fn inert_midword_delimiter_still_recovers() {
    // `foo abc"` — the `"` is mid-word, an ordinary literal, so the following
    // line is a genuine command-break that must still recover (the veto must not
    // over-fire and strand an unfixable error).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo abc\"\nproc recovered_after_midword {} {}\n");
    let names = symbol_name_list(&lsp.document_symbols(&uri));
    assert!(
        names.iter().any(|n| n == "recovered_after_midword"),
        "tail not recovered; symbols={names:?}"
    );
}

#[test]
fn inert_brace_word_break_tokens_well_formed() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo {bar\nputs baz}\nputs end\n");
    let raw = lsp.semantic_tokens(&uri);
    let data = raw.get("data").and_then(Value::as_array);
    assert!(
        data.map(|d| d.len() % 5 == 0).unwrap_or(false),
        "token data must be 5-int groups"
    );
}

// --------------------------------------------------------------------------- //
// C5 — edits toggle recovery
// --------------------------------------------------------------------------- //

fn del_close_bracket() -> Value {
    json!([
        {
            "range": {"start": {"line": 0, "character": 14}, "end": {"line": 0, "character": 15}},
            "text": "",
        }
    ])
}

fn insert_close_bracket() -> Value {
    json!([
        {
            "range": {"start": {"line": 0, "character": 14}, "end": {"line": 0, "character": 14}},
            "text": "]",
        }
    ])
}

#[test]
fn c5_break_then_fix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [foo bar]\nset\n");
    assert!(!has_recovery_error(&diags));
    lsp.change_document(&uri, 2, del_close_bracket());
    let diags = lsp.await_diagnostics_version(&uri, Some(2), Duration::from_secs(30));
    assert!(
        has_recovery_error(&diags),
        "break should flag; got {:?}",
        codes(&diags)
    );
    lsp.change_document(&uri, 3, insert_close_bracket());
    let diags = lsp.await_diagnostics_version(&uri, Some(3), Duration::from_secs(30));
    assert!(
        !has_recovery_error(&diags),
        "fix should clear; got {:?}",
        codes(&diags)
    );
}

#[test]
fn c5_rapid_toggling_stays_consistent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x [foo bar]\nset\n");
    let mut version = 1i64;
    for _ in 0..3 {
        version += 1;
        lsp.change_document(&uri, version, del_close_bracket());
        let diags = lsp.await_diagnostics_version(&uri, Some(version), Duration::from_secs(30));
        assert!(
            has_recovery_error(&diags),
            "v{version} broken; got {:?}",
            codes(&diags)
        );
        version += 1;
        lsp.change_document(&uri, version, insert_close_bracket());
        let diags = lsp.await_diagnostics_version(&uri, Some(version), Duration::from_secs(30));
        assert!(
            !has_recovery_error(&diags),
            "v{version} clean; got {:?}",
            codes(&diags)
        );
    }
}
