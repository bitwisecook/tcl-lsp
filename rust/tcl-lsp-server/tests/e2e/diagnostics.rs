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

//! Native port of `tests/lsp_e2e/test_diagnostics_e2e.py`.
//!
//! Push diagnostics, end-to-end against the packaged server. The server
//! advertises no pull provider, so these assert on the `publishDiagnostics` the
//! server pushes after analysis, keyed by version.

use crate::common::{Lsp, unique_uri};

use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

/// The set of `code` strings carried by `diags` (mirrors Python `_codes`).
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

/// Whether any diagnostic carries `code`.
fn has_code(diags: &[Value], code: &str) -> bool {
    codes(diags).contains(code)
}

/// The set of start lines for diagnostics carrying `code` (mirrors `_on_line`).
fn on_line(diags: &[Value], code: &str) -> BTreeSet<i64> {
    diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some(code) && d.get("range").is_some())
        .filter_map(|d| {
            d.get("range")
                .and_then(|r| r.get("start"))
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
        })
        .collect()
}

/// A diagnostic's `code`, normalised to a `String` (stringifying non-strings).
fn code_str(d: &Value) -> Option<String> {
    match d.get("code") {
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

/// A diagnostic's `message` text (empty if absent).
fn message(d: &Value) -> &str {
    d.get("message").and_then(Value::as_str).unwrap_or("")
}

/// Diagnostics from `diags` carrying `code`.
fn with_code(diags: &[Value], code: &str) -> Vec<Value> {
    diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some(code))
        .cloned()
        .collect()
}

// -- TestPushDiagnostics -------------------------------------------------

#[test]
fn unbraced_expr_is_w100() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "if $a {puts x}\n"), "W100"));
}

#[test]
fn catch_without_result_is_w302() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "catch {error e}\n"), "W302"));
}

#[test]
fn arity_error_is_e002_with_error_severity() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set\n");
    let e002: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E002"))
        .collect();
    assert!(!e002.is_empty());
    assert_eq!(e002[0].get("severity").and_then(Value::as_i64), Some(1)); // Error
}

// -- E004 malformed `if` — end-to-end. Each message/range is
// cross-checked against tclsh 8.6 and Tcl 9.0.4's `Tcl_IfObjCmd` source
// in the unit-level truth table (`tcl-registry`'s
// `commands::tcl::if_::tests`, `tcl-compiler`'s `analyser::state::tests`
// `tp_*` / `fp_*` / `tn_*` cases); this layer only asserts the
// diagnostic survives the full LSP round trip (server → JSON-RPC →
// `publishDiagnostics`) with the right code, message, and — critically
// — a *tight* range, not the whole statement.

/// A diagnostic's `range` as `((start_line, start_char), (end_line, end_char))`.
fn diag_range(d: &Value) -> ((i64, i64), (i64, i64)) {
    let get = |path: &[&str]| -> i64 {
        let mut v = d.get("range").expect("diagnostic has a range");
        for p in path {
            v = v.get(p).unwrap_or(&Value::Null);
        }
        v.as_i64().unwrap_or(-1)
    };
    (
        (get(&["start", "line"]), get(&["start", "character"])),
        (get(&["end", "line"]), get(&["end", "character"])),
    )
}

#[test]
fn e004_bare_if_names_the_invoked_command_and_anchors_on_it() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if\n");
    let e004 = with_code(&diags, "E004");
    assert_eq!(e004.len(), 1, "got {diags:?}");
    assert_eq!(message(&e004[0]), "No expression after \"if\" argument");
    assert_eq!(diag_range(&e004[0]), ((0, 0), (0, 2)));
}

#[test]
fn e004_condition_without_body_anchors_on_the_condition_word() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1}\n");
    let e004 = with_code(&diags, "E004");
    assert_eq!(e004.len(), 1, "got {diags:?}");
    assert_eq!(message(&e004[0]), "No script following \"1\" argument");
    assert_eq!(diag_range(&e004[0]), ((0, 3), (0, 6)));
}

#[test]
fn e004_extra_words_anchors_only_the_extra_word_not_the_whole_statement() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1} {a} {b} {c}\n");
    let e004 = with_code(&diags, "E004");
    assert_eq!(e004.len(), 1, "got {diags:?}");
    assert_eq!(
        message(&e004[0]),
        "Extra words after \"else\" clause in \"if\" command"
    );
    // Just "{c}" (columns 15..18) — not the whole `if ... {c}` statement.
    assert_eq!(diag_range(&e004[0]), ((0, 15), (0, 18)));
}

#[test]
fn e004_leading_else_bareword_condition_is_not_flagged() {
    // `if else {a}` — "else" is a well-formed (if ill-typed) condition,
    // not a malformed `if`; see the FP fix in
    // `tcl-compiler`'s `analyser::state::tests::fp_leading_else_is_not_malformed`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if else {a}\n");
    assert!(with_code(&diags, "E004").is_empty(), "got {diags:?}");
}

#[test]
fn e004_qualified_double_colon_if_is_checked() {
    // `::if` names the same global command as `if` — the E004 dispatch
    // is generic on the resolved spec's hook, not on the literal
    // `cmd_name == "if"` text, so registry `::`-stripping picks this up
    // for free.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "::if {1} {a} {b} {c}\n");
    assert_eq!(with_code(&diags, "E004").len(), 1, "got {diags:?}");
}

#[test]
fn e004_well_formed_elseif_chain_has_no_e004() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "if {$a} {\n  puts a\n} elseif {$b} {\n  puts b\n} else {\n  puts c\n}\n",
    );
    assert!(with_code(&diags, "E004").is_empty(), "got {diags:?}");
}

#[test]
fn e004_no_duplicate_e002_for_the_same_malformed_if() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1}\n");
    assert_eq!(with_code(&diags, "E004").len(), 1, "got {diags:?}");
    assert!(
        with_code(&diags, "E002").is_empty(),
        "E004 must not carry a redundant generic E002 alongside it: {diags:?}"
    );
}

#[test]
fn clean_file_has_no_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(
        lsp.open_ready(&uri, "set x [clock seconds]\nputs $x\n")
            .is_empty()
    );
}

#[test]
fn renamed_away_command_is_w128() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc a {} {return 1}\na\nrename a b\na\n");
    assert!(has_code(&diags, "W128"));
}

#[test]
fn unbraced_expr_inside_catch_body_is_w100() {
    // The analyser must recurse into `catch { ... }` bodies, so the unbraced
    // `expr` inside is a W100 just as at the top level (catch-body-walk fix).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "catch { set y [expr $a + $b] } msg\n");
    assert!(has_code(&diags, "W100"));
    assert_eq!(on_line(&diags, "W100"), BTreeSet::from([0]));
}

#[test]
fn unbraced_expr_inside_tcltest_body_is_w100() {
    // `tcltest::test` evaluates its body as Tcl, so diagnostics inside it are
    // real (tcltest body-role resolver fix).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "package require tcltest\ntcltest::test t {d} { set y [expr $a + $b] } {}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W100"));
}

// -- TestSubcommandOptionArity -------------------------------------------
// End-to-end arity checks for per-subcommand option flags (issue #581).
// `file link -symbolic linkName target` is valid Tcl — the optional
// `-linktype` flag precedes the two positionals — but the packaged server used
// to emit "Too many arguments for 'file link'" because the subcommand's declared
// options were never skipped before the positional count.

#[test]
fn file_link_symbolic_has_no_arity_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "file link -symbolic $dst $src\n");
    assert!(!has_code(&diags, "E003"));
}

#[test]
fn file_link_hard_has_no_arity_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "file link -hard $dst $src\n");
    assert!(!has_code(&diags, "E003"));
}

#[test]
fn file_link_too_many_positionals_is_e003() {
    // No option flag: three positionals genuinely exceed the max of 2, so the
    // arity error must still fire (the fix skips options, not real arguments).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "file link $a $b $c\n");
    let e003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E003"))
        .collect();
    assert!(!e003.is_empty());
    assert_eq!(e003[0].get("severity").and_then(Value::as_i64), Some(1)); // Error
}

#[test]
fn string_match_nocase_has_no_arity_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "string match -nocase $pat $str\n");
    assert!(!has_code(&diags, "E003"));
}

// -- E003 tight range + registry arity-data corrections ------------------
// The too-many-args squiggle covers only the surplus words, and several
// command arities are corrected to match C Tcl 9.

#[test]
fn e003_too_many_args_highlights_only_the_surplus_words() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "lreverse {a b c} extra1 extra2\n");
    let e003 = with_code(&diags, "E003");
    assert_eq!(e003.len(), 1, "got {diags:?}");
    // Tight range: only `extra1 extra2` (cols 17..30), not the whole command
    // from column 0.
    assert_eq!(diag_range(&e003[0]), ((0, 17), (0, 30)), "got {diags:?}");
}

#[test]
fn lmap_even_arg_count_is_e005() {
    // `lmap` shares `foreach`'s odd/even grammar; an even count is wrong.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "lmap a b c d\n");
    assert!(has_code(&diags, "E005"), "got {diags:?}");
    // A valid odd count is silent.
    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(&uri2, "lmap x {1 2} {incr x}\n");
    assert!(!has_code(&diags2, "E005"), "got {diags2:?}");
}

#[test]
fn global_and_variable_zero_args_have_no_arity_error() {
    let mut lsp = Lsp::tcl();
    for src in ["global\n", "variable\n"] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(!has_code(&diags, "E002"), "{src:?} → {diags:?}");
    }
}

#[test]
fn while_true_with_throw_or_tailcall_is_not_infinite() {
    // W241 must not fire when the body leaves the loop via `throw`/`tailcall`.
    let mut lsp = Lsp::tcl();
    for src in ["while 1 {throw MYERR boom}\n", "while 1 {tailcall next}\n"] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(!has_code(&diags, "W241"), "{src:?} → {diags:?}");
    }
    // Control: a body with no exit is still flagged.
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "while 1 {incr n}\n");
    assert!(has_code(&diags, "W241"), "got {diags:?}");
}

// -- TestW004DialectInvalidOption -----------------------------------------
// End-to-end coverage for W004 (option not available in the active
// dialect): the abbreviated-subcommand fix, and the shadow-suppression
// fix for a same-file proc that redefines a builtin.

#[test]
fn lsearch_stride_on_tcl86_is_w004() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.6\nlsearch -stride 2 {a b} x\n");
    assert!(has_code(&diags, "W004"));
}

#[test]
fn chan_configure_inputmode_on_tcl86_is_w004() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.6\nchan configure $chan -inputmode raw\n",
    );
    assert!(has_code(&diags, "W004"));
}

#[test]
fn abbreviated_chan_conf_inputmode_on_tcl86_is_still_w004() {
    // `conf` uniquely abbreviates `configure` — real Tcl ensemble dispatch
    // accepts it, so the option check must resolve it too rather than
    // silently skipping the whole option scan.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.6\nchan conf $chan -inputmode raw\n",
    );
    assert!(has_code(&diags, "W004"));
}

#[test]
fn lsearch_stride_on_tcl90_has_no_w004() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.0\nlsearch -stride 2 {a b} x\n");
    assert!(!has_code(&diags, "W004"));
}

#[test]
fn user_proc_shadowing_lsearch_suppresses_w004() {
    // `lsearch` really dispatches to the user's own proc here, so the
    // builtin's dialect-restricted `-stride` no longer applies.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "# tcl-dialect: tcl8.6\nproc lsearch {l args} { return $l }\nlsearch -stride 2 {a b}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W004"),
        "a user proc shadowing lsearch should suppress W004: {diags:?}"
    );
}

// -- TestE001MissingDispatchWord ------------------------------------------
// End-to-end coverage for E001 ("missing subcommand" / TclOO "missing
// method"): tight command-head-only highlighting, the `history`
// bare-call carve-out (issue: bare `history` defaults to `history info`
// per history(n), so it must not be flagged), and the TclOO object-
// dispatch generalisation (`$obj` with no method word at all).

#[test]
fn bare_string_is_e001_with_tight_command_head_span() {
    // The diagnostic must highlight only the command word itself — there is
    // no subcommand to include, so the span should not creep onto the
    // trailing newline or beyond the four characters of `string`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "string\n");
    let e001: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E001"))
        .collect();
    assert_eq!(e001.len(), 1, "expected exactly one E001: {diags:?}");
    let range = &e001[0]["range"];
    assert_eq!(range["start"]["line"], 0);
    assert_eq!(range["start"]["character"], 0);
    assert_eq!(range["end"]["line"], 0);
    assert_eq!(
        range["end"]["character"], 6,
        "span must cover only 'string'"
    );
    assert_eq!(
        e001[0].get("severity").and_then(Value::as_i64),
        Some(1) // Error
    );
}

#[test]
fn bare_history_has_no_e001() {
    // FP regression (history(n)): `history` alone is a well-defined call
    // (equivalent to `history info`), not a missing-subcommand error, even
    // though `history` is a `WithSubcommands` registry command like `string`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "history\n");
    assert!(
        !has_code(&diags, "E001"),
        "bare `history` must not be E001: {diags:?}"
    );
}

#[test]
fn history_with_subcommand_is_still_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "history clear\n");
    assert!(!has_code(&diags, "E001"));
    assert!(!has_code(&diags, "W001"));
}

#[test]
fn history_unknown_subcommand_is_still_w001_not_e001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "history bogus\n");
    assert!(has_code(&diags, "W001"));
    assert!(!has_code(&diags, "E001"));
}

// -- W002 (disabled-in-dialect command), end to end -----------------------

/// The `(character, character)` span of the first diagnostic carrying `code`
/// on `line`, or `None` if there isn't one.
fn range_on_line(diags: &[Value], code: &str, line: i64) -> Option<(i64, i64)> {
    diags.iter().find_map(|d| {
        if code_str(d).as_deref() != Some(code) {
            return None;
        }
        let range = d.get("range")?;
        let start = range.get("start")?;
        if start.get("line")?.as_i64()? != line {
            return None;
        }
        let end = range.get("end")?;
        Some((
            start.get("character")?.as_i64()?,
            end.get("character")?.as_i64()?,
        ))
    })
}

#[test]
fn disabled_command_is_w002_with_a_tight_span() {
    // `dict` is Tcl 8.5+; under `tcl8.4` the call is disabled, and the
    // squiggle must cover exactly the 4-character `dict` token — not the
    // whole line — so the editor highlights only the offending name.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\ndict create a 1\n");
    let w002: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W002"))
        .collect();
    assert_eq!(w002.len(), 1, "expected exactly one W002: {diags:?}");
    assert_eq!(
        message(w002[0]),
        "'dict' is disabled in the active dialect profile \
         (available in: tcl8.5, tcl8.6, tcl9.0, tcl9.1)"
    );
    assert_eq!(
        range_on_line(&diags, "W002", 1),
        Some((0, 4)),
        "the span must cover exactly 'dict' on line 1: {diags:?}"
    );
}

#[test]
fn disabled_command_enabled_in_active_dialect_is_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.0\ndict create a 1\n");
    assert!(
        !has_code(&diags, "W002"),
        "dict is native in 9.0: {diags:?}"
    );
}

#[test]
fn disabled_subcommand_is_w002_with_a_command_plus_subcommand_span() {
    // `package files` is Tcl 9.0+; the squiggle covers `package files`
    // (command head through the subcommand word), not the whole call.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "package files mypackage\n");
    assert!(
        !has_code(&diags, "W001"),
        "must not report as unknown: {diags:?}"
    );
    assert_eq!(
        range_on_line(&diags, "W002", 0),
        Some((0, 13)),
        "the span must cover 'package files' (13 chars): {diags:?}"
    );
}

#[test]
fn disabled_command_shadowed_by_namespace_scoped_proc_is_clean() {
    // A namespace-scoped `proc dict` shadows the disabled builtin for
    // unqualified calls resolved inside that namespace — Tcl's real
    // current-namespace-then-global resolution rule, not merely "was `dict`
    // defined anywhere in the file".
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\nnamespace eval ::ns {\n    proc dict {args} { return $args }\n    dict foo bar\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W002"),
        "a namespace-scoped shadowing proc must suppress W002: {diags:?}"
    );
}

#[test]
fn disabled_command_shadowed_by_forward_declared_proc_body_is_clean() {
    // The shadowing proc is declared *after* the call site textually, but
    // the call only runs (inside another proc's body) once the whole file
    // has loaded — so the later definition is already in effect.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\nproc use_dict {} {\n    dict create a 1\n}\nproc dict {args} { return $args }\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W002"),
        "a forward-declared proc-body shadow must suppress W002: {diags:?}"
    );
}

#[test]
fn disabled_command_top_level_call_before_shadowing_proc_still_fires() {
    // The mirror image: a *top-level* call before its shadowing proc's
    // definition still reaches the disabled builtin at load time.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\ndict create a 1\nproc dict {args} { return $args }\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "W002"),
        "a top-level call before its shadowing proc must still fire W002: {diags:?}"
    );
}

#[test]
fn disabled_command_shadowed_by_interp_alias_is_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\ninterp alias {} dict {} list\ndict create a 1\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W002"),
        "an interp alias establishing the name must suppress W002: {diags:?}"
    );
}

#[test]
fn disabled_command_shadowed_by_rename_is_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# tcl-dialect: tcl8.4\nproc myimpl {args} { return $args }\nrename myimpl dict\ndict create a 1\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W002"),
        "a rename establishing the name must suppress W002: {diags:?}"
    );
}

#[test]
fn disabled_subcommand_form_shadowed_by_ensemble_head_proc_is_clean() {
    // A user `proc package` overrides the *whole* ensemble command — the
    // call never reaches the registry `package files` subcommand check.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc package {args} { return $args }\npackage files mypackage\n",
    );
    assert!(
        !has_code(&diags, "W002"),
        "a shadowed ensemble head must suppress the subcommand-form W002: {diags:?}"
    );
}

#[test]
fn w001_span_covers_only_the_subcommand_word() {
    // The squiggle must sit tightly on the offending word alone — not the
    // command name too. `string bogus $x`: "string " is 7 characters, so
    // "bogus" spans [7, 12).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "string bogus $x\n");
    let w001: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W001"))
        .collect();
    assert_eq!(w001.len(), 1, "expected exactly one W001: {diags:?}");
    let range = &w001[0]["range"];
    assert_eq!(range["start"]["line"], 0);
    assert_eq!(
        range["start"]["character"], 7,
        "span must start at 'bogus', not 'string'"
    );
    assert_eq!(range["end"]["line"], 0);
    assert_eq!(
        range["end"]["character"], 12,
        "span must cover only 'bogus'"
    );
}

#[test]
fn proc_shadowing_ensemble_command_suppresses_w001() {
    // FP regression (FP-STY-17): a same-file `proc string {...}` replaces
    // the builtin `string` ensemble at the call site, end to end.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc string {op args} { return $op }\nstring reverse hello\n",
    );
    assert!(
        !has_code(&diags, "W001"),
        "proc-shadowed ensemble call must not fire W001: {diags:?}"
    );
}

#[test]
fn unshadowed_ensemble_command_still_fires_w001_alongside_a_shadowed_one() {
    // TP control paired with the FP above: shadowing `string` must not
    // blind the server to a genuine unknown subcommand on a different
    // ensemble in the same file.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc string {op args} { return $op }\ninfo bogus\n");
    assert!(
        has_code(&diags, "W001"),
        "unshadowed ensemble must still fire W001: {diags:?}"
    );
}

#[test]
fn bare_tcloo_object_dispatch_is_e001() {
    // `set o [Dog new]; $o` — TclOO's per-object dispatcher requires a
    // method word before it attempts any method lookup.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Dog { method bark {} { return woof } }\nset o [Dog new]\n$o\n";
    let diags = lsp.open_ready(&uri, src);
    let e001: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E001"))
        .collect();
    assert_eq!(e001.len(), 1, "expected exactly one E001: {diags:?}");
    assert_eq!(message(e001[0]), "'o' requires a method");
    assert_eq!(on_line(&diags, "E001"), BTreeSet::from([2]));
}

#[test]
fn tcloo_object_dispatch_with_method_is_not_e001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Dog { method bark {} { return woof } }\nset o [Dog new]\n$o bark\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "E001"));
}

#[test]
fn bare_snit_instance_dispatch_is_not_e001() {
    // snit's generated dispatcher proc is a different mechanism the
    // analyser does not model precisely enough to assume it shares
    // TclOO's unconditional "wrong # args" behaviour on a bare call.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "snit::type Dog { method bark {} { return woof } }\nDog t\n$t\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "E001"));
}

// -- TestT101OutputSinkSpan ------------------------------------------------
// T101 (tainted data into `puts`) end-to-end: the diagnostic must highlight
// only the tainted argument word, not the whole `puts $x` statement.

#[test]
fn puts_tainted_data_has_tight_argument_span() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x [gets stdin]\nputs $x\n";
    let diags = lsp.open_ready(&uri, src);
    let t101: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("T101"))
        .collect();
    assert_eq!(t101.len(), 1, "expected exactly one T101: {diags:?}");
    let range = &t101[0]["range"];
    // Line 1 (0-based) is `puts $x`; `$x` starts at character 5, ends at 7 —
    // not the whole 7-character statement starting at character 0.
    assert_eq!(range["start"]["line"], 1);
    assert_eq!(
        range["start"]["character"], 5,
        "span must start at `$x`, not the `puts` command word: {diags:?}"
    );
    assert_eq!(range["end"]["line"], 1);
    assert_eq!(
        range["end"]["character"], 7,
        "span must cover only `$x`: {diags:?}"
    );
}

// -- TestSameFileCallArity ------------------------------------------------
// End-to-end arity checks generalised beyond the builtin registry to
// same-file proc / `interp alias` / `rename` calls. Previously, calling a
// same-file proc with the wrong number of arguments produced no diagnostic
// at all — an out-of-range variable reference inside the proc body would
// correctly fire W210, but the call site itself was silently accepted.

#[test]
fn same_file_proc_call_too_many_args_is_e003() {
    // The reported repro: a 7-parameter proc called with 8 arguments.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "\
proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return \"$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7\"
}
demonstrate one two three four five six seven eight
";
    let diags = lsp.open_ready(&uri, src);
    let e003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E003"))
        .collect();
    assert!(
        !e003.is_empty(),
        "expected E003 for the 8-arg call to a 7-param proc; got {diags:?}"
    );
    assert!(message(e003[0]).contains("demonstrate"));
    assert_eq!(e003[0].get("severity").and_then(Value::as_i64), Some(1)); // Error
}

#[test]
fn same_file_proc_call_with_correct_arity_has_no_arity_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "\
proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return \"$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7\"
}
demonstrate one two three four five six seven
";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "E002"));
    assert!(!has_code(&diags, "E003"));
}

#[test]
fn same_file_renamed_proc_call_inherits_original_arity() {
    // `rename` is a pure name move — the renamed name must still be
    // checked against the proc's own (unchanged) arity.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc target {a b c} {}\nrename target target_orig\ntarget_orig 1 2\n",
    );
    assert!(has_code(&diags, "E002"));
}

#[test]
fn same_file_interp_alias_call_arity_is_shifted_by_prepended_args() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc target {a b c} {}\ninterp alias {} shortcut {} target 100\nshortcut 2\n",
    );
    assert!(has_code(&diags, "E002"));
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(
        &uri2,
        "proc target {a b c} {}\ninterp alias {} shortcut {} target 100\nshortcut 2 3\n",
    );
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn same_file_tcloo_forward_via_my_arity_is_shifted_by_prepended_args() {
    // `forward NAME my TARGET ?ARG…?` is the documented TclOO idiom for
    // forwarding to a sibling method — confirmed against tclsh 9.0.4 that
    // a bare method name (`forward NAME TARGET`) is never a valid forward
    // target, but routing through `my` resolves via the receiver's own
    // method-resolution order, arity-shifted by any arguments after it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "\
oo::class create Widget {
    method base {a b c} { return \"$a$b$c\" }
    forward fwd my base fixedarg
}
set w1 [Widget new]
$w1 fwd 1 2 3
";
    let diags = lsp.open_ready(&uri, src);
    let e003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E003"))
        .collect();
    assert!(
        !e003.is_empty(),
        "expected E003 for a 3-arg call to a forward shifted down to 2 args; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(
        &uri2,
        "oo::class create Widget {\n    method base {a b c} { return \"$a$b$c\" }\n    forward fwd my base fixedarg\n}\nset w1 [Widget new]\n$w1 fwd 1 2\n",
    );
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn same_file_tcloo_constructor_call_arity_is_checked() {
    // `ClassName new ?args?` / `ClassName create name ?args?` is checked
    // against the class's own (or nearest inherited) `constructor` — a
    // gap this review closed; previously neither form drew any arity
    // diagnostic at all.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Widget { constructor {a b} { } }\nWidget new 1\n",
    );
    assert!(
        has_code(&diags, "E002"),
        "expected E002 for a 1-arg call to a 2-arg constructor; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_create = lsp.open_ready(
        &uri2,
        "oo::class create Widget { constructor {a b} { } }\nWidget create fido 1 2 3\n",
    );
    assert!(
        has_code(&diags_create, "E003"),
        "expected E003 for create's mandatory name plus 3 extra args against a 2-arg constructor; got {diags_create:?}"
    );
    let uri3 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(
        &uri3,
        "oo::class create Widget { constructor {a b} { } }\nWidget create fido 1 2\n",
    );
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn same_file_tcloo_constructor_inherited_through_superclass_is_checked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Base { constructor {a b} { } }\noo::class create Sub { superclass Base }\nSub new 1\n",
    );
    assert!(
        has_code(&diags, "E002"),
        "a subclass with no constructor of its own must inherit the superclass's; got {diags:?}"
    );
}

#[test]
fn same_file_tcloo_no_explicit_constructor_is_never_checked() {
    // TclOO's default (inherited from `oo::object`) constructor accepts
    // and ignores any number of arguments.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Widget { method bar {} { } }\nWidget new 1 2 3 4 5\n",
    );
    assert!(!has_code(&diags, "E002"));
    assert!(!has_code(&diags, "E003"));
}

#[test]
fn same_file_tcloo_next_call_arity_is_checked() {
    // `next` re-invokes the current method's next-in-MRO implementation —
    // a gap this review closed; previously `next` drew no arity
    // diagnostic at all regardless of the resolved superclass method's
    // own signature.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Base { method speak {a b} { return \"$a$b\" } }\n\
         oo::class create Derived { superclass Base\n method speak {a b} { next 1 } }\n\
         [Derived new] speak x y\n",
    );
    assert!(
        has_code(&diags, "E002"),
        "expected E002 for a 1-arg `next` against a 2-arg superclass method; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(
        &uri2,
        "oo::class create Base { method speak {a b} { return \"$a$b\" } }\n\
         oo::class create Derived { superclass Base\n method speak {a b} { next 1 2 } }\n\
         [Derived new] speak x y\n",
    );
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn same_file_tcloo_nextto_call_arity_checks_named_target() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Root { method speak {a} { return $a } }\n\
         oo::class create Mid { superclass Root\n method speak {a b} { return \"$a$b\" } }\n\
         oo::class create Derived { superclass Mid\n method speak {a b} { nextto Root 1 2 } }\n\
         [Derived new] speak x y\n",
    );
    assert!(
        has_code(&diags, "E003"),
        "expected E003 for a 2-arg `nextto Root` against Root's 1-arg speak; got {diags:?}"
    );
}

#[test]
fn same_file_apply_lambda_call_arity_is_checked() {
    // A direct `apply {{params} body} ?args?` call — another gap this
    // review closed.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "apply {{a b} {return [expr {$a+$b}]}} 1\n");
    assert!(
        has_code(&diags, "E002"),
        "expected E002 for a 1-arg call to a 2-param lambda; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(&uri2, "apply {{a b} {return [expr {$a+$b}]}} 1 2\n");
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn dict_create_odd_key_value_tail_is_checked() {
    // `dict create ?key value ...?` needs an even tail — a gap this
    // review closed; previously an odd (unpaired) tail drew no
    // diagnostic at all.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "dict create a\n");
    assert!(
        has_code(&diags, "E005"),
        "expected E005 for an odd dict create tail; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(&uri2, "dict create a b\n");
    assert!(!has_code(&diags_ok, "E005"));
}

#[test]
fn foreach_unpaired_varlist_is_checked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "foreach x $l y {puts $x}\n");
    assert!(
        has_code(&diags, "E005"),
        "expected E005 for an unpaired foreach var-list; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(&uri2, "foreach x $l {puts $x}\n");
    assert!(!has_code(&diags_ok, "E005"));
}

#[test]
fn switch_flat_unpaired_pattern_is_checked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "switch $s a b c\n");
    assert!(
        has_code(&diags, "E005"),
        "expected E005 for an unpaired switch pattern; got {diags:?}"
    );
    // Both valid shapes stay silent: the flat paired form and the
    // single-braced-body shorthand.
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(&uri2, "switch $s a b c d\n");
    assert!(!has_code(&diags_ok, "E005"));
    let uri3 = unique_uri("tcl");
    let diags_braced = lsp.open_ready(&uri3, "switch $s {a b c d e f}\n");
    assert!(!has_code(&diags_braced, "E005"));
}

// -- TestDiagnosticCanaries ----------------------------------------------
// One canary per analysis family, locked to the server's published output.

// -- W210: read of a possibly-unset variable -----------------------------

#[test]
fn w210_read_before_set_on_path_merge() {
    // `y` is defined only inside the `if` arm; the read at the merge point may
    // see it unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {x} {\n    if {$x} {\n        set y 1\n    }\n    return $y\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
    // The diagnostic anchors on the read site (`return $y`), not the def.
    assert!(on_line(&diags, "W210").contains(&4));
}

#[test]
fn w210_use_after_unset() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set a 1\nunset a\nputs $a\n");
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W210").contains(&2));
}

#[test]
fn w210_silent_when_set_on_all_paths() {
    // Defined on both branches → no read-before-set (must-stay-silent).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {x} {\n    if {$x} {\n        set y 1\n    } else {\n        set y 2\n    }\n    return $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- W307: a variable used in command position ---------------------------

#[test]
fn w307_known_literal_non_command_fires() {
    // `cmd` resolves to the literal `foo` which is not a command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {} {\n    set cmd foo\n    $cmd bar\n}\n");
    assert!(has_code(&diags, "W307"), "{:?}", codes(&diags));
}

#[test]
fn w307_silent_for_opaque_dispatch_target() {
    // `$self` is an opaque parameter (a method-dispatch idiom); with no known
    // non-command literal value, W307 must not fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {self} {\n    $self configure -x 1\n}\n");
    assert!(!has_code(&diags, "W307"), "{:?}", codes(&diags));
}

// -- W220: dead store -----------------------------------------------------

#[test]
fn w220_dead_store_fires() {
    // `x` is set, then overwritten before any read → the first store is dead.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc p {} {\n    set x 1\n    set x 2\n    return $x\n}\n",
    );
    assert!(has_code(&diags, "W220"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W220").contains(&1));
}

#[test]
fn w220_silent_when_value_is_read_between() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set x 1\n    puts $x\n    set x 2\n    return $x\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W220"));
}

#[test]
fn w220_silent_for_command_name_read_nested_in_dict_for_if() {
    // Issue #833: `$x` is read as the command name of `$x a $key`, nested inside
    // `if {$value}` inside `dict for`. Before dict-for bodies were lowered into
    // real CFG blocks, that read was invisible and `set x set` false-fired W220.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc demo {} {\n    set x set\n    set d [dict create a true b false c true]\n    dict for {key value} $d {\n        if {$value} {\n            $x a $key\n        }\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W220"), "{:?}", codes(&diags));
    assert!(!has_code(&diags, "W211"), "{:?}", codes(&diags));
}

#[test]
fn w220_dead_store_inside_dict_for_body_fires() {
    // Precision gained by #833's fix: a dead store *inside* the (now lowered)
    // dict-for body is a real W220 — it was invisible while the body was opaque.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc demo {d} {\n    dict for {k v} $d {\n        set tmp 1\n        set tmp 2\n        puts $tmp\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W220"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W220").contains(&2));
}

#[test]
fn uplevel_issue_837_body_is_silent_through_server() {
    // Issue #837: the exact reproducer must produce no diagnostics through the
    // packaged server — the recursed `uplevel` body is clean Tcl.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc forgetXyce {} {\n    uplevel 1 {foreach nameSpc [namespace children ::Foo] {\n        namespace forget ${nameSpc}::*\n    }}\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        diags.is_empty(),
        "issue #837 repro must be silent: {:?}",
        codes(&diags)
    );
}

#[test]
fn uplevel_unbraced_substituted_body_fires_w105_through_server() {
    // Now that `uplevel`'s body is registry-known, an unbraced substituted body
    // fires W105 end-to-end, exactly as `eval` does — guiding the user to brace it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    uplevel 1 \"puts $x\"\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W105"), "{:?}", codes(&diags));
}

// -- Clean code stays clean (cross-family negative control) --------------

#[test]
fn clean_dataflow_has_no_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {x} {\n    set d [dict create a 1]\n    if {[dict exists $d a]} {\n        return [dict get $d a]\n    }\n    return $x\n}\n";
    assert!(lsp.open_ready(&uri, src).is_empty());
}

// -- TestIndirectArrayIdiom ----------------------------------------------
// FP-STY-12: `${var}(idx)` in a varname position is the indirect-array-element
// idiom (`var` holds the array name), not a broken `$var(idx)` — so neither
// W216 nor W212 fire through the server pipeline. A value-position `${arr}(x)`
// still fires W216.

#[test]
fn set_indirect_array_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set token ::http::1\nset ${token}(status) eof\n");
    assert!(!has_code(&diags, "W216"), "{:?}", codes(&diags));
    assert!(!has_code(&diags, "W212"), "{:?}", codes(&diags));
}

#[test]
fn info_exists_indirect_array_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "info exists ${token}(-pipeline)\n");
    assert!(!has_code(&diags, "W216"));
    assert!(!has_code(&diags, "W212"));
}

#[test]
fn unset_and_vwait_indirect_array_silent() {
    for src in ["unset ${tok}(socketcoro)\n", "vwait ${token}(status)\n"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        assert!(!has_code(&lsp.open_ready(&uri, src), "W216"), "{src:?}");
    }
}

#[test]
fn value_position_still_fires_w216() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "puts ${arr}(x)\n");
    assert!(has_code(&diags, "W216"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W216").contains(&0));
}

#[test]
fn bare_dollar_name_still_fires_w212() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "set $x v\n"), "W212"));
}

// -- TestOverridableLibraryProcs -----------------------------------------
// FP-STY-13: redefining an overridable Tcl *library* proc (`unknown`,
// `history`, `auto_*` …) is not shadowing a C built-in — no W113. Redefining a
// genuine built-in (`set`/`clock`) still fires.

#[test]
fn unknown_override_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(
        &lsp.open_ready(&uri, "proc unknown args { return }\n"),
        "W113"
    ));
}

#[test]
fn library_procs_silent() {
    for name in ["history", "auto_execok", "tcl_findLibrary", "pkg_mkIndex"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("proc {name} {{args}} {{ return }}\n");
        assert!(!has_code(&lsp.open_ready(&uri, &src), "W113"), "{name}");
    }
}

#[test]
fn c_builtin_override_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(
        &lsp.open_ready(&uri, "proc set {a b} { return }\n"),
        "W113"
    ));
}

#[test]
fn non_bytecompiled_c_command_still_fires() {
    // clock/after/socket/glob are C commands (not byte-compiled) but must still
    // fire — the library-proc exemption must not over-reach.
    for name in ["clock", "after", "socket", "glob"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("proc {name} {{a}} {{ return }}\n");
        assert!(has_code(&lsp.open_ready(&uri, &src), "W113"), "{name}");
    }
}

// -- TestSingleVarBodyW105 -----------------------------------------------
// FP-STY-14: a body argument that is a single bare variable substitution
// (`eval $cmd`, `$state(-command)`, `after 0 $coroName`) is a script-valued
// reference, not an inline block — no W105 through the server pipeline. A
// quoted/composite interpolated body still fires.

#[test]
fn eval_single_var_body_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(&lsp.open_ready(&uri, "eval $cmd\n"), "W105"));
}

#[test]
fn callback_dispatch_body_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval :: $state(-command) $token\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W105"));
}

#[test]
fn after_and_dynamic_proc_silent() {
    for src in ["after 0 $coroName\n", "proc $fakeName $arglist $body\n"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        assert!(!has_code(&lsp.open_ready(&uri, src), "W105"), "{src:?}");
    }
}

#[test]
fn quoted_interpolated_body_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "eval \"do $script\"\n");
    assert!(has_code(&diags, "W105"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W105").contains(&0));
}

#[test]
fn composite_body_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "eval $cmd$args\n"), "W105"));
}

// -- TestDollarBeforeCloseQuoteW306 --------------------------------------
// FP-STY-15: a `$` immediately before a closing `"` (the regex end-anchor
// `"^foo$"` / `"\n$"`) is literal — the lexer must not merge the quoted word
// with the next, so no E002/E205 and no spurious W306. A live `$bar` in a quoted
// pattern still fires W306.

#[test]
fn regsub_end_anchor_no_errors() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "regsub \"\\n$\" $msg \"\" out\n");
    let cs = codes(&diags);
    assert!(!cs.contains("E002"), "{cs:?}");
    assert!(!cs.contains("E205"), "{cs:?}");
    assert!(!cs.contains("W306"), "{cs:?}");
}

#[test]
fn string_match_end_anchor_no_arity() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(
        &lsp.open_ready(&uri, "string match \"abc$\" $x\n"),
        "E002"
    ));
}

#[test]
fn regex_end_anchor_no_w306() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(
        &lsp.open_ready(&uri, "regexp -- \"^foo$\" $text\n"),
        "W306"
    ));
}

#[test]
fn live_var_in_quoted_pattern_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(
        &lsp.open_ready(&uri, "regexp -- \"^foo$bar\" $text\n"),
        "W306"
    ));
}

// -- TestControlFlowRBSFamilyE2E -----------------------------------------
// W210 read-before-set, control-flow modelling family (PR #634).

// -- tailcall ends straight-line flow (FP-RBS-13) ------------------------

#[test]
fn tailcall_terminated_branch_silent() {
    // `tailcall g` returns from the proc, so `return $result` is reached only
    // via the else branch where `result` is set.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {cond} {\n    if {$cond} {\n        tailcall g\n    } else {\n        set result 1\n    }\n    return $result\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn non_terminating_branch_still_fires() {
    // Replace the tailcall with a completing command → `result` is maybe-unset
    // on the then-path, so W210 must fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {cond} {\n    if {$cond} {\n        puts hi\n    } else {\n        set result 1\n    }\n    return $result\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
}

// -- non-empty-literal foreach runs its body (FP-RBS-17) -----------------

#[test]
fn foreach_non_empty_literal_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {1 2 3} { set y $x }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn foreach_empty_literal_still_fires() {
    // An empty literal never runs the body → `y` is unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {} { set y $x }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- for whose condition is true on entry runs its body (FP-RBS-18) ------

#[test]
fn for_true_on_entry_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    for {set i 0} {$i < 3} {incr i} { set y $i }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn for_false_on_entry_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    for {set i 5} {$i < 3} {incr i} { set y $i }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- while 1 only exits via break, where the var is set (FP-RBS-16) ------

#[test]
fn while1_break_set_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    while 1 { set y 1; break }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn normal_while_body_defined_silent() {
    // FP-RBS-19 (#756): a non-constant `while` may run zero times, but its body
    // unconditionally sets `y`, so a read after the loop is defined whenever the
    // loop ran. Matching C Tcl (which errors only when the condition is false on
    // entry at runtime), we assume a may-run loop runs — no W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {n} {\n    while {$n > 0} { set y 1; incr n -1 }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn foreach_accumulator_after_loop_silent() {
    // FP-RBS-19 (#756), the reporter's exact pattern: a `lappend` accumulator
    // built inside a dynamic multi-group `foreach`, read after the loop. The
    // body defines the accumulators on every iteration, so the after-loop reads
    // are not read-before-set.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set data [getDataDict]\n\
        foreach time [dict get $data time] vout [dict get $data osc_out] {\n\
        \x20   lappend timeVout [list $time $vout]\n\
        }\n\
        puts $timeVout\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn foreach_empty_literal_after_loop_still_fires() {
    // TP control: a provably-empty literal list runs zero times, so tclsh always
    // errors — the after-loop read must keep firing.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {} { lappend acc $x }\n    puts $acc\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- opaque switch whose every arm exits is a terminator (FP-RBS-15) -----

#[test]
fn all_arms_return_makes_trailing_read_unreachable() {
    // Every arm returns, so `puts $y` is unreachable dead code → no W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {x} {\n    switch -glob -- $x { a* { return 1 } default { return 2 } }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn no_default_switch_falls_through_fires() {
    // Without a `default` an unmatched subject falls through to the read, so the
    // switch is not a terminator and `y` is maybe-unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {x} {\n    switch -glob -- $x { a* { return 1 } b* { return 2 } }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- opaque switch must-define excludes non-completing arms (FP-RBS-14) --

#[test]
fn returning_arm_excluded_from_must_define() {
    // The `a*` arm returns before reaching `puts $y`; the only path that does
    // (default) sets `y` → definitely defined, no W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {x} {\n    switch -glob -- $x { a* { return 0 } default { set y 2 } }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn break_arm_escaping_loop_still_fires() {
    // Codex regression on #634: `break`/`continue` are loop-jumps, not
    // proc-exits, so a break arm does NOT define the other arm's var on the path
    // that escapes the loop — `y` is maybe-unset, W210 must fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {a} { switch -glob -- $x { a* { break } default { set y 1 } } }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- TestWhenBodyDialectGatingE2E ----------------------------------------
// `when` is an iRules-only builtin (PR #640). Under plain Tcl it is an unknown
// would-be user command whose braced argument is opaque *data*, not a handler
// script — so its body must not be analysed.

#[test]
fn when_body_not_analysed_under_plain_tcl() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n");
    // `when` itself is unknown under Tcl, but the opaque body must not be
    // recursed into: no W123 naming the body command, no W210 on its var.
    let body_w123: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W123") && message(d).contains("boguscmd"))
        .collect();
    assert!(body_w123.is_empty(), "{diags:?}");
    assert!(!has_code(&diags, "W210"), "{:?}", codes(&diags));
}

// -- TestConstantStringConditionFoldE2E ----------------------------------
// I230 (always-true/false condition; alternate branch unreachable) now folds
// `==`/`!=` on string operands, matching Tcl's polymorphic compare
// (`expr {"foo" == "foo"}` -> 1) — previously only the `eq`/`ne` spelling folded
// (PR #640).

#[test]
fn double_equals_string_condition_folds() {
    for op in ["==", "eq"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}\n");
        assert!(has_code(&lsp.open_ready(&uri, &src), "I230"), "{op}");
    }
}

#[test]
fn bang_equals_string_condition_folds() {
    for op in ["!=", "ne"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}\n");
        assert!(has_code(&lsp.open_ready(&uri, &src), "I230"), "{op}");
    }
}

// -- TestDiagnosticsTrackEdits -------------------------------------------

#[test]
fn fixing_the_source_clears_the_diagnostic() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if $a {puts x}\n");
    assert!(has_code(&diags, "W100"));
    // Wrap the expression in braces via an incremental edit: `$a` -> `{$a}`.
    lsp.change_document(
        &uri,
        2,
        serde_json::json!([
            {
                "range": {
                    "start": { "line": 0, "character": 3 },
                    "end": { "line": 0, "character": 3 },
                },
                "text": "{",
            },
            {
                "range": {
                    "start": { "line": 0, "character": 6 },
                    "end": { "line": 0, "character": 6 },
                },
                "text": "}",
            },
        ]),
    );
    let cleared = lsp.await_diagnostics_version(&uri, Some(2), Duration::from_secs(30));
    assert!(!has_code(&cleared, "W100"));
}

#[test]
fn introducing_an_error_publishes_it() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(lsp.open_ready(&uri, "puts hello\n").is_empty());
    // Replace the whole line with an arity error.
    lsp.replace_document(&uri, 2, "set\n");
    let diags = lsp.await_diagnostics_version(&uri, Some(2), Duration::from_secs(30));
    assert!(has_code(&diags, "E002"));
}

// -- TestSoundnessRegressionsE2E -----------------------------------------
// End-to-end coverage for three latent W210 soundness bugs surfaced while
// reviewing the Rust port and fixed in the analyser. Ground truth is real
// tclsh 9.0.3.

// -- Omitted-arg call-site constants are poisoned (interproc) ------------

#[test]
fn omitted_default_arg_does_not_hide_read_before_set() {
    // `p` (slot 0 omitted → default `x == 0`) leaves `y` unset, so `puts $y` is
    // a real read-before-set; the literal `1` passed by `p 1` must not be bound
    // as a constant for `x`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {{x 0}} {\n    if {$x} {\n        set y 5\n    }\n    puts $y\n}\np\np 1\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W210").contains(&4));
}

#[test]
fn uniform_literal_arg_still_binds_silent() {
    // Every caller passes `1` at slot 0 → the constant binding holds, the
    // `if {$x}` body is provably taken, `y` is always set: stay silent.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc q {{x 0}} {\n    if {$x} {\n        set y 5\n    }\n    puts $y\n}\nq 1\nq 1\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- regexp `-expanded` is not unconditionally literal-safe --------------

#[test]
fn regexp_expanded_whitespace_pattern_silent() {
    // `-expanded` ignores unescaped whitespace, so `{a b}` matches the substring
    // `ab` and writes `v`; reading `v` must not fire W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc g {input} {\n    regexp -expanded {a b} $input v\n    puts $v\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn regexp_expanded_clean_literal_still_fires() {
    // A whitespace/`#`-free literal stays provably literal: `{x}` never matches
    // `X`, never writes `w` → reading `w` is read-before-set.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc g {} {\n    regexp -expanded {x} X w\n    puts $w\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- try body throw keeps its handler exception edge ---------------------

#[test]
fn try_body_throw_keeps_handler_defs_silent() {
    // `x` is set before the only throw, so the handler always sees it (tclsh
    // prints `1`): stay silent.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    try {\n        set x 1\n        error boom\n    } on error {} {\n        puts $x\n    }\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn try_body_earlier_conditional_throw_fires() {
    // The handler is reachable from every throw point; `x` is unset on the
    // earlier `if {$c} {error a}` path, so the read is maybe-unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {c} {\n    try {\n        if {$c} { error a }\n        set x 1\n        error b\n    } on error {} {\n        puts $x\n    }\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- issue #777: `CLASS create NAME` binds a command NAME -----------------

#[test]
fn w307_silent_for_create_named_objects_iterated() {
    // Exact repro of issue #777: object commands are bound by `C create c1`,
    // then iterated via `foreach elem [list c1 l1 …]` and dispatched through
    // `$elem`.  The created names are known commands, so W307 must not fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "\
C create c1 1 out 0 -c 1e-9
L create l1 1 out 0 -l 10e-6
C create c2 2 n002 0 -c 1e-9
foreach elem [list c1 l1 c2] {
    $elem actOnParam -set 1
}
";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W307"),
        "dispatch over created object names must not fire W307: {:?}",
        codes(&diags),
    );
}

#[test]
fn w123_silent_for_create_named_object_literal_dispatch() {
    // A known class's `create NAME` binds `NAME`; a literal `NAME method` call
    // is a real command, not an unknown (W123).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create C { method configure args {} }\nC create c1\nc1 configure -x 2\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !on_line(&diags, "W123").contains(&2),
        "`c1 configure` on line 2 must not be an unknown command: {:?}",
        codes(&diags),
    );
}

// ---------------------------------------------------------------------------
// Issue #806 — report::defstyle scoped command environment.
// ---------------------------------------------------------------------------

#[test]
fn defstyle_body_scoped_commands_no_w123() {
    // The report configuration methods (top/data/columns/…) are scoped
    // commands inside the style script — no unknown-command diagnostic.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "::report::defstyle simpletable {} {\n\
               \x20   top set [split \"x\"]\n\
               \x20   data set [split \"y\"]\n\
               \x20   bottom enable\n\
               \x20   topdatasep enable\n\
               \x20   columns\n\
               }\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "W123"),
        "no W123 in a valid style body: {diags:?}"
    );
}

#[test]
fn defstyle_body_typo_still_w123() {
    // A genuine typo inside the body is still an unknown command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "::report::defstyle st {} {\n    toop set x\n}\n");
    assert!(has_code(&diags, "W123"), "typo `toop` flagged: {diags:?}");
    assert!(
        diags.iter().any(|d| message(d).contains("toop")),
        "message names the typo: {diags:?}",
    );
}

#[test]
fn defstyle_bad_operation_is_w001() {
    // `top bogus` — unknown ensemble operation of a scoped command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "::report::defstyle st {} {\n    top bogus\n}\n");
    assert!(
        has_code(&diags, "W001"),
        "unknown scoped op → W001: {diags:?}"
    );
}

#[test]
fn report_object_methods_have_no_unknown_command() {
    // `report::report` binds `r`; `r <method>` resolves via the object class.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require report\n::report::report r 3\nr data set x\nr printmatrix m\n",
    );
    assert!(
        !has_code(&diags, "W123"),
        "report object methods resolve: {diags:?}"
    );
}

// -- Issue #832: command defined in an auto_path library ------------------
// A command a `tclIndex` on the configured `libraryPaths` auto-loads (the
// BLT/Rbc idiom: `Rbc_ZoomStack` / `Rbc_ActiveLegend`) must not be flagged
// "Unknown command" (W123), end-to-end against the packaged server, with
// `xcDiagnostics` left at its default (off).  The package database resolves the
// command exactly as go-to-definition does; the diagnostic must agree.

/// Per-call counter so repeat runs don't collide on the temp dir.
static RBC_LIB_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[test]
fn autoload_library_command_not_unknown_issue_832() {
    use std::sync::atomic::Ordering;

    // A library dir with a `tclIndex` that auto-loads two global procs by bare
    // name — no `package require` needed, the exact shape from the issue.
    let libdir = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-rbc-{}-{}",
        std::process::id(),
        RBC_LIB_N.fetch_add(1, Ordering::Relaxed)
    ));
    let pkgdir = libdir.join("rbc");
    std::fs::create_dir_all(&pkgdir).expect("mk rbc lib dir");
    std::fs::write(
        pkgdir.join("graph.tcl"),
        "proc Rbc_ActiveLegend {graph} {}\nproc Rbc_ZoomStack {graph args} {}\n",
    )
    .expect("write graph.tcl");
    std::fs::write(
        pkgdir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
         set auto_index(Rbc_ZoomStack) [list source [file join $dir graph.tcl]]\n",
    )
    .expect("write tclIndex");

    // Point the server's auto_path (`libraryPaths`) at the library's parent dir;
    // `scan_path` descends into the `rbc/` subdir (C-Tcl auto_path rule) at
    // startup, so its `tclIndex` enters the package database.
    let mut lsp = Lsp::with_config(serde_json::json!({
        "features": { "linkedEditingRange": true },
        "libraryPaths": [ libdir.to_string_lossy() ],
    }));

    let uri = unique_uri("tcl");
    let src = "Rbc_ActiveLegend .g\nRbc_ZoomStack .g\n";
    lsp.open_document(&uri, src);

    // The package database is (re)built asynchronously at startup; poll the
    // deterministic pull path until it is live (W123 clears) or the deadline.
    let mut diags = lsp.pull_diagnostics(&uri);
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while has_code(&diags, "W123") && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }
    assert!(
        !has_code(&diags, "W123"),
        "auto_path library commands must not be W123 (#832), got: {:?}",
        codes(&diags),
    );

    // Control: a typo the `tclIndex` does not declare still fires W123 — the
    // database is loaded and the check is data-driven, not disabled wholesale.
    let typo = unique_uri("tcl");
    let tdiags = lsp.open_ready(&typo, "Rbc_ActveLegend .g\n");
    assert!(
        has_code(&tdiags, "W123"),
        "a command no index declares must still be W123, got: {:?}",
        codes(&tdiags),
    );

    let _ = std::fs::remove_dir_all(&libdir);
}

// -- #844 progressive (two-tier) diagnostics -----------------------------

/// Build a large Tcl document (~`n` procs, ~10×`n` lines) whose deep
/// diagnostics pass reliably overruns `DIAGNOSTICS_FAST_TIER_BUDGET`, plus one
/// proc whose unbraced `expr` yields a fast-tier **W100** (an analyser code) and
/// a deep-tier-only **O111** (the optimiser hint paired with W100) — so the two
/// progressive publishes are distinguishable regardless of package-database
/// state.
fn big_tcl_with_split_markers(n: usize) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(n * 200);
    s.push_str("namespace eval ::bench {\n    variable counter 0\n}\n\n");
    for i in 0..n {
        let _ = write!(
            s,
            "proc ::bench::step{i} {{a b}} {{\n\
             \x20   set v{i} [expr {{$a + $b}}]\n\
             \x20   if {{$v{i} > 10}} {{\n\
             \x20       set v{i} [expr {{$v{i} + 1}}]\n\
             \x20   }}\n\
             \x20   return $v{i}\n\
             }}\n\n"
        );
    }
    // The distinguishing pair: an unbraced `expr` → analyser W100 (fast tier),
    // whose paired optimiser hint O111 is produced only by the deep pass.
    s.push_str("proc ::bench::w100_probe {x} {\n    return [expr $x + 1]\n}\n");
    s
}

/// #844 acceptance criterion (b): on a large / cold file the client sees the
/// workspace-independent **fast tier** first (analyser syntax / structural /
/// style diagnostics) and then the **deep tier** (adding compiler / optimiser
/// diagnostics), which is a strict superset and replaces it for the same
/// version.  The unbraced `expr` gives a fast-tier `W100`; its paired optimiser
/// hint `O111` is produced only by the deep pass, so the two publishes are
/// distinguishable regardless of package-database state.
///
/// A small / warm file settles inside `DIAGNOSTICS_FAST_TIER_BUDGET` and skips
/// the fast tier entirely (a single publish — the debounce-skip guarded by the
/// existing `diagnostics_delivery_smoke` single-publish tests); this test
/// deliberately uses a large file so the deep pass overruns the budget and the
/// fast tier fires.
#[test]
fn large_file_publishes_fast_tier_before_deep_tier() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let big = big_tcl_with_split_markers(600);

    let since = lsp.notification_cursor();
    lsp.open_document_lang(&uri, &big, "tcl", 1);
    // The `[timing] deep diagnostics` log fires only from the deep publish, so it
    // is the reliable "deep pass finished" barrier; both publishes are buffered
    // by the time it arrives.
    lsp.await_log(
        &["deep diagnostics", uri.as_str()],
        Duration::from_secs(45),
        since,
    );

    let pubs: Vec<Vec<Value>> = lsp
        .notifications()
        .into_iter()
        .skip(since)
        .filter(|n| {
            n.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics")
                && n.get("params")
                    .and_then(|p| p.get("uri"))
                    .and_then(Value::as_str)
                    == Some(uri.as_str())
        })
        .map(|n| {
            n.get("params")
                .and_then(|p| p.get("diagnostics"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .collect();

    let all_codes: Vec<BTreeSet<String>> = pubs.iter().map(|p| codes(p)).collect();
    let Some(deep_idx) = pubs.iter().position(|p| codes(p).contains("O111")) else {
        panic!(
            "no deep-tier publish (carrying the optimiser O111) arrived; publishes: {all_codes:?}"
        );
    };
    if deep_idx == 0 {
        // Coalesced into a single publish on an unexpectedly fast host (the deep
        // pass landed inside the 40 ms budget). The two sibling
        // progressive/convergence tests carry the same escape hatch; the fast→deep
        // split is not observable this run, but completeness still is.
        let only = codes(&pubs[0]);
        assert!(
            only.contains("W100") && only.contains("O111"),
            "a coalesced single publish must still carry the complete set: {only:?}",
        );
        return;
    }
    assert!(
        deep_idx >= 1,
        "the fast tier must be published before the deep tier on a large file, \
         but the first publish already carried the deep-only O111: {all_codes:?}",
    );

    let fast = codes(&pubs[deep_idx - 1]);
    let deep = codes(&pubs[deep_idx]);
    assert!(
        fast.contains("W100"),
        "the fast tier must carry the workspace-independent analyser W100: {fast:?}",
    );
    assert!(
        !fast.contains("O111") && !fast.contains("W120") && !fast.contains("W123"),
        "the fast tier must exclude deep-only optimiser and workspace-refined \
         codes: {fast:?}",
    );
    assert!(
        deep.contains("W100") && deep.contains("O111"),
        "the deep tier must carry both the analyser W100 and the optimiser O111: {deep:?}",
    );
    assert!(
        fast.is_subset(&deep),
        "the deep tier must be a strict superset of the fast tier (no fast-tier \
         diagnostic is ever removed by the deep pass): fast={fast:?} deep={deep:?}",
    );
}

// -- E100 / E102 stray-closer diagnostics --------------------------------
//
// A bare `]` / `}` has no special meaning to Tcl outside `[...]` / `{...}`,
// so these are "probably a typo" heuristics, not hard parse errors. The
// range must be tight around the offending character — end-to-end coverage
// for issue-class bugs found in review: the highlighted range excluding the
// stray character itself, a fix-less diagnostic spanning the whole command
// instead of just the character, and a bad repair corrupting an unrelated
// "Unknown command" diagnostic elsewhere in the file.

fn range_of(diags: &[Value], code: &str) -> (i64, i64, i64, i64) {
    let d = diags
        .iter()
        .find(|d| code_str(d).as_deref() == Some(code))
        .unwrap_or_else(|| panic!("no {code} in {diags:?}"));
    let r = &d["range"];
    (
        r["start"]["line"].as_i64().unwrap(),
        r["start"]["character"].as_i64().unwrap(),
        r["end"]["line"].as_i64().unwrap(),
        r["end"]["character"].as_i64().unwrap(),
    )
}

#[test]
fn e100_range_is_tight_around_bracket_without_a_fix() {
    // `set x blah]` — no known command / arity overflow, so no fix is
    // available; the highlighted range must be just the `]` (char 10..11),
    // not the whole command from `set`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x blah]\n");
    assert_eq!(range_of(&diags, "E100"), (0, 10, 0, 11));
}

#[test]
fn e100_range_includes_the_bracket_with_a_fix() {
    // `puts string]` — `string` is a known command right before the `]`;
    // the range must still run through (and include) the `]` itself.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "puts string]\n");
    let (l0, c0, l1, c1) = range_of(&diags, "E100");
    assert_eq!((l0, l1), (0, 0));
    assert_eq!(c1, 12, "range end must include the ']' at char 11");
    assert!(c0 < c1);
}

#[test]
fn e102_range_is_tight_around_embedded_brace() {
    // A `}` embedded in a bareword (not the whole token) must still be
    // flagged, with a range covering only the `}` character.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x foo}bar\n");
    assert_eq!(range_of(&diags, "E102"), (0, 9, 0, 10));
}

#[test]
fn e100_repair_does_not_corrupt_unrelated_command_name() {
    // Regression: a stray `]` after a call to an already-declared user
    // proc used to get "repaired" into a virtual command-substitution
    // token with a byte-offset bug, corrupting the recorded invocation
    // and firing a phantom "Unknown command" (W123) on a garbled
    // substring of the proc name — with no E100 fix to explain it either.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc myHelper {a b} {return $a}\nset y myHelper arg1 arg2]\n",
    );
    assert!(has_code(&diags, "E100"));
    assert!(
        !has_code(&diags, "W123"),
        "no phantom unknown-command diagnostic expected: {:?}",
        diags
            .iter()
            .map(|d| (code_str(d), message(d).to_string()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn e100_escaped_bracket_under_arity_overflow_is_silent() {
    // A genuinely escaped trailing `]` (`\]`) combined with an arity
    // overflow on the enclosing command must not fire E100 at all, and
    // must not trigger a repair that corrupts anything downstream.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set y bar baz\\]\n");
    assert!(!has_code(&diags, "E100"), "{:?}", codes(&diags));
    assert!(!has_code(&diags, "W123"), "{:?}", codes(&diags));
}

#[test]
fn e101_recovery_does_not_swallow_a_call_to_a_known_proc() {
    // Regression: only registry builtins were excluded from looking
    // like an orphaned switch case, so a genuine call to an
    // already-declared user proc with a single braced argument right
    // after the case list — `renderReport { prose text }` — was
    // swallowed as an extra case, corrupting the switch's argv and
    // running the braced prose through command analysis as if it were
    // Tcl (a phantom "Unknown command" on ordinary text).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc renderReport {body} {\n    puts $body\n}\n\nproc foo {x} {\n    switch $x\n    a {\n        return 1\n    }\n    renderReport {\n        Some unrelated braced-argument call, not a switch case.\n    }\n}\n",
    );
    assert!(has_code(&diags, "E101"), "{:?}", codes(&diags));
    assert!(
        !has_code(&diags, "W123"),
        "the renderReport call must not be parsed as switch-case body text: {:?}",
        diags
            .iter()
            .map(|d| (code_str(d), message(d).to_string()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn e103_abstains_when_missing_brace_swallows_more_than_one_statement() {
    // Regression: the "stolen close brace" heuristic used to fire on
    // whichever `}` was LAST in the swallowed text, even when that
    // text spanned more than one top-level statement (here a sibling
    // `proc` swallowed along with the `if` that actually stole the
    // brace). Applying that fix parsed clean but silently nested the
    // sibling proc inside the unclosed one instead of closing it
    // where the missing brace belongs — a structural corruption, not
    // just an imprecise diagnostic. Pure brace-counting can't safely
    // pick a location once more than one statement is swallowed, so
    // this must fall back to the generic (fix-less) E200 instead of
    // guessing wrong.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc foo {} {\n    if {1} {\n        puts hi\n    }\nproc bar {} {\n    return 1\n}\n",
    );
    assert!(!has_code(&diags, "E103"), "{:?}", codes(&diags));
    assert!(has_code(&diags, "E200"), "{:?}", codes(&diags));
}

#[test]
fn shimmer_incr_on_string_is_s100_info_with_tight_range() {
    // The range must cover only the `$x` argument — not the whole `incr x`
    // call and not the `[lindex $x 0]` substitution around it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x hello\nlindex $x 0\n");
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert_eq!(s100.len(), 1, "expected exactly one S100: {diags:?}");
    let range = &s100[0]["range"];
    assert_eq!(range["start"]["line"], 1);
    assert_eq!(range["start"]["character"], 7, "must start at '$x'");
    assert_eq!(range["end"]["line"], 1);
    assert_eq!(range["end"]["character"], 9, "must end after '$x'");
    assert_eq!(
        s100[0].get("severity").and_then(Value::as_i64),
        Some(3), // Information
        "S100 is informational: {:?}",
        s100[0]
    );
}

#[test]
fn clean_list_used_with_lindex_has_no_s100() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [list 1 2 3]\nlindex $x 0\n");
    assert!(!has_code(&diags, "S100"), "unexpected S100: {diags:?}");
}

#[test]
fn shimmer_fires_inside_tcloo_method_body() {
    // TclOO method bodies previously got zero shimmer coverage (the
    // compiler-checks aggregator only walked the top level and procedures).
    // Exactly one, not two: `tcl-lsp-db::proc_taint_solve` (the live server's
    // memoised path) has its own top-up loop for methods/body units,
    // independent of `compiler_checks.rs`'s direct path — a regression where
    // both the top-up loop *and* the main loop covered methods/body units
    // would double-emit every shimmer diagnostic inside one.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src =
        "oo::class create C {\n    method m {} {\n        set x hello\n        incr x\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert_eq!(
        s100.len(),
        1,
        "expected exactly one S100 inside a TclOO method body: {diags:?}"
    );
}

#[test]
fn shimmer_fires_inside_namespace_eval_body() {
    // Same double-count guard as `shimmer_fires_inside_tcloo_method_body`,
    // for the other synthetic body-unit kind.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ns {\n    set x hello\n    incr x\n}\n";
    let diags = lsp.open_ready(&uri, src);
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert_eq!(
        s100.len(),
        1,
        "expected exactly one S100 inside a namespace eval body: {diags:?}"
    );
}

#[test]
fn no_shimmer_for_tcloo_instance_variable_linked_via_my_variable() {
    // (FP guard) `my variable count` links an object-instance variable whose
    // true intrep depends on another method's last write, not the nominal
    // return type of `my`. Before the registry fix (`oo_my.rs`'s `variable`
    // subcommand), this spuriously claimed a String->Int shimmer.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Counter {\n    method bump {} {\n        my variable count\n        incr count\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !diags.iter().any(|d| {
            matches!(code_str(d).as_deref(), Some("S100" | "S101"))
                && d.get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .contains("'count'")
        }),
        "'my variable'-linked instance var must not spuriously shimmer: {diags:?}"
    );
}

/// (Regression guard) `my` dispatches to an arbitrary `TclOO` method name — the
/// registry only recognises `variable` (for the scope-alias trait) and must
/// keep `allow_unknown_subcommands` set, or every other `my <method>` call
/// (the overwhelmingly common form) would falsely draw W001.
#[test]
fn my_arbitrary_method_call_does_not_draw_w001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Counter {\n    method bump {} {\n        my touch\n    }\n    method touch {} {}\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W001"), "unexpected W001: {diags:?}");
}

#[test]
fn shimmer_fires_through_interp_alias() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "interp alias {} myindex {} ::lindex\nset x hello\nmyindex $x 0\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "S100"),
        "expected S100 through an interp alias to a shimmering builtin: {diags:?}"
    );
}

#[test]
fn shimmer_noqa_suppresses_s100() {
    // `# noqa: S100` on the line before the shimmering command must
    // suppress it through the live publishDiagnostics pipeline.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x hello\n# noqa: S100\nlindex $x 0\n");
    assert!(
        !has_code(&diags, "S100"),
        "S100 must be suppressed by a preceding '# noqa: S100': {diags:?}"
    );
}

#[test]
fn shimmer_noqa_for_unrelated_code_does_not_suppress_s100() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x hello\n# noqa: W100\nlindex $x 0\n");
    assert!(
        has_code(&diags, "S100"),
        "S100 must still fire when the preceding noqa names an unrelated code: {diags:?}"
    );
}

// -- W003 (dialect-gated expr operators) ---------------------------------

#[test]
fn w003_tight_span_covers_only_the_operator() {
    // The diagnostic must highlight just the 2-byte `lt`, not the whole
    // `{$a lt $b}` condition or the enclosing `if`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\nif {$a lt $b} { puts hi }\n");
    let w003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W003"))
        .collect();
    assert_eq!(w003.len(), 1, "{diags:?}");
    let range = &w003[0]["range"];
    assert_eq!(range["start"]["line"], 1);
    assert_eq!(range["start"]["character"], 7);
    assert_eq!(range["end"]["line"], 1);
    assert_eq!(range["end"]["character"], 9, "span must cover only 'lt'");
}

#[test]
fn w003_repeated_operators_get_distinct_ranges_over_the_wire() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl8.4\nif {$a in $b && $c in $d} { puts hi }\n",
    );
    let w003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W003"))
        .collect();
    assert_eq!(w003.len(), 2, "{diags:?}");
    assert_ne!(w003[0]["range"], w003[1]["range"]);
}

#[test]
fn w003_message_cites_the_relevant_tip() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.4\nexpr {2 in {1 2 3}}\n");
    let w003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W003"))
        .collect();
    assert_eq!(w003.len(), 1, "{diags:?}");
    assert!(message(w003[0]).contains("TIP 201"), "{}", message(w003[0]));
}

#[test]
fn w003_eda_vendor_dialect_does_not_over_fire_on_in() {
    // Regression: `xilinx-eda-tcl` is documented as running on top of a
    // real Tcl 8.5 core, so TIP 201's `in` must not be flagged there.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: xilinx-eda-tcl\nexpr {2 in {1 2 3}}\n");
    assert!(!has_code(&diags, "W003"), "{diags:?}");
}

#[test]
fn w003_f5_tmsh_flags_string_relational_operators() {
    // Regression: `f5-tmsh` used to have no `DialectSet` bit at all, so
    // W003 silently never fired for it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: f5-tmsh\nif {$a lt $b} { puts hi }\n");
    assert!(has_code(&diags, "W003"), "{diags:?}");
}
