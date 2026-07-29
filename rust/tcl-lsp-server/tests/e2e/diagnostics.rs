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

#[test]
fn w212_covers_registry_name_positions_end_to_end() {
    // `vwait $x` and `catch {…} $res` are name/value confusions the old
    // hardcoded list missed; the registry roles now supply them.
    let mut lsp = Lsp::tcl();
    for src in [
        "proc p {} { vwait $x }\n",
        "proc p {} { catch {error e} $res }\n",
    ] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(has_code(&diags, "W212"), "{src:?} → {diags:?}");
    }
}

#[test]
fn w126_anchors_at_the_channel_argument() {
    // `puts notachan hello` — the non-channel literal `notachan` (cols 5..13)
    // is the problem, not the whole command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "puts notachan hello\n");
    let w126 = with_code(&diags, "W126");
    assert_eq!(w126.len(), 1, "got {diags:?}");
    assert_eq!(diag_range(&w126[0]), ((0, 5), (0, 13)), "got {diags:?}");
}

#[test]
fn w101_anchors_at_the_substituted_argument() {
    // `eval "safeprefix" $x` — the hazard is `$x` (col 18..20), not the safe
    // literal prefix. The squiggle must point at the substituted word.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "eval \"safeprefix\" $x\n");
    let w101 = with_code(&diags, "W101");
    assert_eq!(w101.len(), 1, "got {diags:?}");
    assert_eq!(diag_range(&w101[0]), ((0, 18), (0, 20)), "got {diags:?}");
}

#[test]
fn w216_upvar_local_indirect_array_is_silent() {
    // `upvar 1 remote ${arr}(x)` — the local-name slot is the legitimate
    // indirect-array idiom, so W216 must not fire (the two name-position lists
    // had drifted; W216's omitted `upvar`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {arr} { upvar 1 remote ${arr}(x) }\n");
    assert!(!has_code(&diags, "W216"), "got {diags:?}");
    // Control: the same shape in a value position IS a broken read.
    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(&uri2, "proc p {arr} { puts ${arr}(x) }\n");
    assert!(has_code(&diags2, "W216"), "got {diags2:?}");
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
fn namespace_ensemble_configure_splice_onto_tk_suppresses_w001_end_to_end() {
    // FP regression, issue #923 idx 84: the real `tk/library/systray.tcl`
    // idiom splices `systray`/`sysnotify` onto the pre-existing,
    // registry-builtin `tk` ensemble via `namespace ensemble configure tk
    // -map [dict merge [namespace ensemble configure tk -map] {systray
    // ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]` — a
    // `CONFIGURE`, not `CREATE`. tclsh9.0/8.6 both confirm `tk systray
    // create`/`tk sysnotify ...` are correct, documented calls.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc ::tk::systray {args} {}\n\
         proc ::tk::sysnotify::sysnotify {a b} {}\n\
         namespace ensemble configure tk -map \
         [dict merge [namespace ensemble configure tk -map] \
         {systray ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]\n\
         tk systray create -image book\n\
         tk sysnotify Alert message\n",
    );
    assert!(
        !has_code(&diags, "W001"),
        "a statically-spliced ensemble subcommand must not fire W001: {diags:?}"
    );
}

#[test]
fn namespace_ensemble_configure_genuinely_unknown_tk_subcommand_still_fires_w001_end_to_end() {
    // TP control paired with the FP above: splicing `systray` onto `tk`
    // must not blind the server to an unrelated, genuinely unknown `tk`
    // subcommand in the same file.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "namespace ensemble configure tk -map \
         [dict merge [namespace ensemble configure tk -map] \
         {systray ::tk::systray}]\n\
         tk zzznotreal\n",
    );
    assert!(
        has_code(&diags, "W001"),
        "a genuinely unknown tk subcommand must still fire W001: {diags:?}"
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

// Issue #923 idx 122: a helper proc's `upvar` write, reached only through
// a `while`/`if` loop CONDITION rather than a bare statement, was invisible
// to W210 — only 4 hardcoded builtins (`catch`/`scan`/`gets`/`regexp`) were
// recognised there. Real tcllib repro: `modules/cmdline/cmdline.tcl`'s
// `getopt`/`getKnownOpt` chain, `while {[set err [getopt argv $opts opt
// arg]]} { ... }`. tclsh9.0/8.6-verified the condition's own command
// substitution (including the upvar write) completes before the guarded
// body ever runs.

#[test]
fn w210_silent_for_upvar_proc_call_in_while_condition_923_idx122() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc getopt {ovar} {\n    upvar 1 $ovar opt\n    set opt value\n    return 1\n}\nwhile {[getopt opt]} {\n    puts $opt\n    break\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_silent_for_upvar_proc_wrapped_inside_a_while_condition_923_idx122() {
    // The exact tcllib shape: the upvar-writing call sits one bracket
    // deeper than the condition's own outermost command (`set err
    // [getopt ...]`, not a bare `[getopt ...]`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc getopt {ovar} {\n    upvar 1 $ovar opt\n    set opt value\n    return 1\n}\nwhile {[set err [getopt opt]]} {\n    puts $opt\n    break\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_silent_for_upvar_proc_call_in_if_condition_923_idx122() {
    // The `if`-conditioned analogue — never "frozen" (only `for`/`while`
    // have a command-substitution-condition opaque-barrier path), so this
    // exercises the ordinary `lower_if` → `condition_out_vars` route
    // rather than the frozen-loop synthetic `<cond>` Call.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc getopt {ovar} {\n    upvar 1 $ovar opt\n    set opt value\n    return 1\n}\nif {[getopt opt]} {\n    puts $opt\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_still_fires_for_a_genuinely_dynamic_condition_call_923_idx122() {
    // TN — a condition call to a proc that does *not* upvar-write its
    // argument must still warn: the fix must not blanket-suppress every
    // `$var`-in-condition read, only ones a known upvar proc actually
    // populates.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc doesNothing {ovar} {\n    puts \"called with $ovar\"\n}\nwhile {[doesNothing opt]} {\n    puts $opt\n    break\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// Issue #923 idx 18 (tcllib), revisited after PR #1020 review: a wrapper
// proc that reaches an `upvar`+`uplevel` "custom control structure" proc
// through a *plain* call (not `uplevel`) does NOT propagate the effect to
// its own caller — tclsh9.0/8.6-verified (`can't read "myf": no such
// variable` when the caller reads the variable outside any `uplevel`'d
// script argument). An earlier version of this fix treated every such
// pass-through as transitive based on a misleading test (reading the
// variable *inside* the same script block that wrote it, which
// coincidentally lands in the same frame regardless of whether real
// propagation happened); these tests now pin the correct, tclsh-verified
// behaviour instead. The real tcllib idiom (`page::util::flow`) reaches its
// worker via `uplevel 1 [list ...]`, which genuinely does propagate one
// frame further (also tclsh9.0-verified) — soundly modelling that shape is
// tracked at https://github.com/bitwisecook/tcl-lsp/issues/1019, not
// attempted here.

#[test]
fn w210_still_fires_for_a_plain_call_wrapper_923_idx18() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc real_worker {fvar nvar script} {\n    upvar 1 $fvar f\n    upvar 1 $nvar n\n    set f 1\n    set n 2\n    uplevel 1 $script\n}\nproc wrapper {fvar nvar script} {\n    real_worker $fvar $nvar $script\n}\nwrapper myf myn {\n    puts \"f=$myf n=$myn\"\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_still_fires_across_a_two_hop_plain_call_wrapper_chain_923_idx18() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc real_worker {fvar nvar script} {\n    upvar 1 $fvar f\n    upvar 1 $nvar n\n    set f 1\n    set n 2\n    uplevel 1 $script\n}\nproc middle_wrapper {fvar nvar script} {\n    real_worker $fvar $nvar $script\n}\nproc outer_wrapper {fvar nvar script} {\n    middle_wrapper $fvar $nvar $script\n}\nouter_wrapper myf myn {\n    puts \"f=$myf n=$myn\"\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn w210_still_fires_when_wrapper_does_not_pass_its_own_params_through_923_idx18() {
    // TN — `unrelated_wrapper` calls the known upvar proc `real_worker`
    // with literal args ("x"/"y"), not its own parameters passed through
    // unchanged. tclsh9.0/8.6-verified this genuinely errors ("can't read
    // \"myf\": no such variable"), so it must still warn.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc real_worker {fvar nvar script} {\n    upvar 1 $fvar f\n    upvar 1 $nvar n\n    set f 1\n    set n 2\n    uplevel 1 $script\n}\nproc unrelated_wrapper {a b c} {\n    real_worker x y $c\n}\nunrelated_wrapper myf myn {\n    puts \"f=$myf n=$myn\"\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
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

// Issue #923 idx 125 (tcllib): a value word's embedded `{…}` run that
// survived, as ordinary literal content, from an originally double-quoted
// or bareword-concatenated source word must not hide the `$var`
// substitutions inside it from W220 — real tcllib repro:
// `modules/htmlparse/htmlparse.tcl`'s `eval "$cmd {$vroot} {} {}
// \{$html\}"`. tclsh9.0/8.6-verified `{$vroot}` here is an ordinary
// substitution, exactly like the bare `$cmd` beside it.

#[test]
fn w220_silent_for_a_var_wrapped_in_braces_inside_a_double_quoted_value_923_idx125() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc demo {} {\n    set a AA\n    set b BB\n    set c CC\n    set s \"prefix {$a} suffix $b $c\"\n    puts $s\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W220"), "{:?}", codes(&diags));
}

#[test]
fn w220_still_fires_for_a_genuinely_brace_quoted_value_923_idx125() {
    // TN — the whole value is brace-quoted (`{$a}` as ONE word, not
    // embedded inside a larger double-quoted string): Tcl performs zero
    // substitution on it at all (tclsh9.0/8.6-verified `set s {$a}` stores
    // the literal two characters `$a`), so `a` is genuinely never read and
    // must still warn.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc demo {} {\n    set a 1\n    set s {$a}\n    puts $s\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W220"), "{:?}", codes(&diags));
}

// Regression guards (found while fixing idx 125): `itcl::class` /
// `snit::widget` / `snit::type` / `snit::widgetadaptor` bodies were missing
// the registry's `body_kind: Structural` classification `oo::class`
// already carries — their body argument was scanned as ordinary value text
// instead of being excluded as a separate definition scope. This was
// invisible before idx 125's fix only because the class/method body's own
// nested braces were, by the same quote-context bug, mis-read as a single
// non-substituting brace-quoted word, which happened to swallow every
// `$this` / instance-variable reference along with it.

#[test]
fn w210_silent_for_this_and_instance_vars_in_an_itcl_method_body_923_idx125() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "itcl::class C {\n    variable handler\n    common registry\n    method run {} {\n        $this configure\n        $handler process\n        return $registry\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W210"), "{:?}", codes(&diags));
}

#[test]
fn w210_silent_for_self_and_instance_vars_in_a_snit_widget_method_body_923_idx125() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "snit::widget mywidget {\n    variable helper\n    component inner\n    method draw {} {\n        $self configure -bg white\n        $inner render\n        $helper compute\n        return $win\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W210"), "{:?}", codes(&diags));
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

#[test]
fn quoted_pure_var_pattern_no_w306() {
    // `"$pattern"` is byte-for-byte identical at runtime to the bare
    // `$pattern` parameterised-pattern idiom (the quotes group nothing) — the
    // canonical form, not a foot-gun. No W306.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(
        &lsp.open_ready(&uri, "regexp -- \"$pattern\" $text\n"),
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

// -- TestI230InterproceduralCallSiteSeedingE2E ---------------------------
// Issue #969: "Condition '$count & 1' is always false" fired on a genuinely
// alternating parity check. Root cause: the interprocedural param-constant
// seed (`params_constants_from_call_sites`) trusted a proc's parameter as a
// compile-time literal whenever every call site *it could resolve* passed
// the same value — but a namespaced proc's own bare-name recursive
// self-call resolved incorrectly (namespace-blind lookup), so that call
// site (with its necessarily-varying argument) silently vanished from the
// evidence, leaving only the one external caller's literal `0` and folding
// `$count & 1` to a fixed `false`. See `compilation_unit.rs`'s
// `collect_call_site_constants` / `params_constants_from_call_sites` for the
// full fix (also closes a second, closely-related gap: a call site
// embedded inside a `catch` / `uplevel` body's `ArgRole::Body` argument).

#[test]
fn namespaced_recursive_proc_parity_check_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ::graph {\n    proc dfs {count} {\n        if {$count & 1} {\n            set parity odd\n        } else {\n            set parity even\n        }\n        if {$count < 3} {\n            dfs [expr {$count + 1}]\n        }\n    }\n}\n::graph::dfs 0\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "recursive parity check on `count` must not fold: {:?}",
        codes(&diags)
    );
}

#[test]
fn call_site_hidden_inside_catch_body_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc is_even {n} {\n    if {$n % 2 == 0} { return 1 } else { return 0 }\n}\nproc main {} {\n    is_even 3\n    catch { is_even 4 }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "is_even is called with both 3 and 4 (the latter inside catch): {:?}",
        codes(&diags)
    );
}

/// TP control: the interprocedural seed must still fire I230 for a
/// genuinely proc-call-invariant parameter — two callers passing the
/// identical literal to a private helper.
#[test]
fn two_callers_with_uniform_literal_still_fires_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n}\nproc caller1 {} { helper prod }\nproc caller2 {} { helper prod }\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "I230"),
        "two callers passing the identical literal should still fold: {:?}",
        codes(&diags)
    );
}

// Issue #976: the interprocedural seed above enumerated only *literal*
// command words, so a call dispatched through a variable (`set cmd helper;
// $cmd dev`) was skipped entirely — it counted neither for nor against any
// proc's parameters, even when it demonstrably reached one the scan had
// already seeded from its literal call sites. The scan now resolves a
// dispatch by value (`unit_scope.rs`): an enumerable one becomes an
// ordinary call site for each name it can hold, and an unenumerable one
// withdraws every seed in the module.

#[test]
fn dynamic_dispatch_with_a_differing_literal_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n}\nhelper prod\nhelper prod\nset cmd helper\n$cmd dev\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "`$cmd dev` also reaches helper, with a differing literal: {:?}",
        codes(&diags)
    );
}

/// TP control: the same dispatch passing the *same* literal every other
/// caller does must still fold — the fix resolves the dispatch rather than
/// blanket-disqualifying the module.
#[test]
fn dynamic_dispatch_agreeing_on_the_literal_still_fires_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n}\nhelper prod\nhelper prod\nset cmd helper\n$cmd prod\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "I230"),
        "every caller — dispatched or not — passes \"prod\": {:?}",
        codes(&diags)
    );
}

/// TP control: a dispatch that provably names a *different* proc must leave
/// an unrelated proc's fold alone.
#[test]
fn dynamic_dispatch_to_a_different_proc_still_fires_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n}\nproc other {a} { return $a }\nhelper prod\nhelper prod\nset cmd other\n$cmd dev\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "I230"),
        "the dispatch provably names `other`, never `helper`: {:?}",
        codes(&diags)
    );
}

#[test]
fn unenumerable_dynamic_dispatch_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n}\nhelper prod\nhelper prod\nset cmd [gets stdin]\n$cmd dev\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "a dispatch whose target cannot be enumerated may reach helper: {:?}",
        codes(&diags)
    );
}

#[test]
fn command_prefix_callback_target_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc cmp {a b} {\n    if {$a eq \"x\"} { return -1 } else { return 1 }\n}\ncmp x x\ncmp x x\nlsort -command cmp {p q}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "`lsort -command cmp` invokes cmp with runtime-supplied arguments: {:?}",
        codes(&diags)
    );
}

// Issue #977: PR #970's `package provide` guard did not cover the more common
// shape — a plain library file with NO `package provide`, `source`d by another
// file that calls its procs with a different literal. `lib.tcl` analysed alone
// sees only its own two `helper prod` callers, seeds `mode` as `"prod"`, and
// folds. The fix threads the project's call sites into the compilation unit
// (`tcl_lsp_db::project_call_site_evidence` → `SourceFile::external_call_sites`
// → `tcl_compiler::unit_scope`), so `main.tcl`'s `helper dev` retracts it.

/// The library file from the issue: no `package provide`, two agreeing
/// in-file callers.
const CROSS_FILE_LIB: &str = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n}\nhelper prod\nhelper prod\n";

#[test]
fn caller_in_a_sourcing_file_with_a_differing_literal_clears_i230() {
    let mut lsp = Lsp::tcl();
    let main_uri = unique_uri("tcl");
    let lib_uri = unique_uri("tcl");
    // Open the caller first so it is already in the project when the library
    // is analysed — the same ordering a workspace scan produces.
    lsp.open_ready(&main_uri, "source lib.tcl\nhelper dev\n");
    lsp.open_document(&lib_uri, CROSS_FILE_LIB);
    // The cross-file retraction lands on a *later* publish than the library's
    // own first result: the project's call-site evidence is refreshed after
    // publishing, not in front of it (putting it in front delayed the
    // semantic-token enrichment tier on a large document). Wait for the
    // converged state rather than the first publish.
    let diags = lsp.await_diagnostics_settled(&lib_uri, Duration::from_secs(20), |d| {
        !d.is_empty() && !has_code(d, "I230")
    });
    assert!(
        !has_code(&diags, "I230"),
        "helper is called with \"dev\" from the sourcing file; must not fold: {:?}",
        codes(&diags)
    );
}

/// TP control: with every project caller agreeing on the literal the seed is
/// sound and I230 still fires — the fix must not disable the mechanism.
#[test]
fn caller_in_another_file_agreeing_on_the_literal_still_fires_i230() {
    let mut lsp = Lsp::tcl();
    let main_uri = unique_uri("tcl");
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(&main_uri, "source lib.tcl\nhelper prod\n");
    let diags = lsp.open_ready(&lib_uri, CROSS_FILE_LIB);
    assert!(
        has_code(&diags, "I230"),
        "every caller in the project passes \"prod\": {:?}",
        codes(&diags)
    );
}

/// TN control: a project file that never calls into the library contributes
/// no evidence and must not disturb a sound fold.
#[test]
fn unrelated_project_file_does_not_clear_i230() {
    let mut lsp = Lsp::tcl();
    let other_uri = unique_uri("tcl");
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(&other_uri, "proc other {x} { return $x }\nother 1\n");
    let diags = lsp.open_ready(&lib_uri, CROSS_FILE_LIB);
    assert!(
        has_code(&diags, "I230"),
        "an unrelated file must not retract a sound fold: {:?}",
        codes(&diags)
    );
}

/// The other direction: the *sourcing* file's own proc is called back from
/// the file it sources. `main.tcl`'s two visible callers agree on `"prod"`,
/// but `lib.tcl` — whose script runs in the same interpreter — calls
/// `local dev`, so the fold must be retracted there too.
#[test]
fn callback_from_a_sourced_file_clears_i230_in_the_sourcing_file() {
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    let main_uri = unique_uri("tcl");
    lsp.open_ready(&lib_uri, "local dev\n");
    let src = "source lib.tcl\nproc local {mode} {\n    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n}\nlocal prod\nlocal prod\n";
    lsp.open_document(&main_uri, src);
    let diags = lsp.await_diagnostics_settled(&main_uri, Duration::from_secs(20), |d| {
        !d.is_empty() && !has_code(d, "I230")
    });
    assert!(
        !has_code(&diags, "I230"),
        "the sourced script calls ::local with \"dev\": {:?}",
        codes(&diags)
    );
}

/// A workspace pools every file into one project, so an unrelated file that
/// happens to reuse a common proc name must not drag its call sites into
/// this one's evidence.  Regression for the VS Code suite, where ~200
/// fixtures share a workspace folder and a second global `helper` (zero
/// arity) silently killed issue #969's TP control.
#[test]
fn an_unrelated_file_reusing_a_proc_name_does_not_clear_i230() {
    let mut lsp = Lsp::tcl();
    let unrelated_uri = unique_uri("tcl");
    let ctl_uri = unique_uri("tcl");
    lsp.open_ready(
        &unrelated_uri,
        "proc helper {} {}\nproc caller {} { helper }\n",
    );
    let diags = lsp.open_ready(&ctl_uri, CROSS_FILE_LIB);
    assert!(
        has_code(&diags, "I230"),
        "the unrelated file's own `helper` binds to its own definition: {:?}",
        codes(&diags)
    );
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

/// Per-call counter for the autoload go-to-definition fixture dir.
static AUTOLOAD_DEF_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[test]
fn autoload_library_command_go_to_definition_m8() {
    use std::sync::atomic::Ordering;

    // Same fixture as #832: a library dir whose `tclIndex` auto-loads a global
    // proc `Rbc_ActiveLegend`, defined on line 0 of `graph.tcl`.  The command
    // is defined nowhere in the open workspace, so go-to-definition must fall
    // through to the autoload tier (M8) and jump into the library file.
    let libdir = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-autoload-def-{}-{}",
        std::process::id(),
        AUTOLOAD_DEF_N.fetch_add(1, Ordering::Relaxed)
    ));
    let pkgdir = libdir.join("rbc");
    std::fs::create_dir_all(&pkgdir).expect("mk lib dir");
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

    let mut lsp = Lsp::with_config(serde_json::json!({
        "libraryPaths": [ libdir.to_string_lossy() ],
    }));

    let uri = unique_uri("tcl");
    let src = "Rbc_ActiveLegend .g\n";
    lsp.open_document(&uri, src);

    // The package database loads asynchronously at startup; poll (via the W123
    // clearing on the pull path) until it is live, then go-to-definition.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    let mut diags = lsp.pull_diagnostics(&uri);
    while has_code(&diags, "W123") && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }

    // Cursor on the `Rbc_ActiveLegend` call head (line 0, col 2).
    let locs = crate::common::helpers::locations(&lsp.definition(&uri, 0, 2));
    assert!(
        !locs.is_empty(),
        "autoload go-to-definition must resolve the library proc (M8); diags {:?}",
        codes(&diags),
    );
    assert!(
        locs[0].uri.ends_with("graph.tcl"),
        "must jump into the library file graph.tcl, got {}",
        locs[0].uri,
    );
    let line = locs[0]
        .range
        .get("start")
        .and_then(|s| s.get("line"))
        .and_then(Value::as_i64);
    assert_eq!(
        line,
        Some(0),
        "Rbc_ActiveLegend is declared on line 0 of graph.tcl",
    );

    let _ = std::fs::remove_dir_all(&libdir);
}

/// Per-call counter for the autoload references/rename fixture dir.
static AUTOLOAD_REFS_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// M8's second half: once the autoload tier resolves a library command, the
/// defining library file is merged into the workspace index, so
/// **find-references** reaches the library declaration and the library's own
/// call sites, and **rename** rewrites them alongside the workspace call.
#[test]
fn autoload_library_command_references_and_rename_m8() {
    use std::sync::atomic::Ordering;

    let libdir = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-autoload-refs-{}-{}",
        std::process::id(),
        AUTOLOAD_REFS_N.fetch_add(1, Ordering::Relaxed)
    ));
    let pkgdir = libdir.join("rbc");
    std::fs::create_dir_all(&pkgdir).expect("mk lib dir");
    // `Rbc_Wire` calls `Rbc_ActiveLegend` inside the library, so the library
    // contributes an internal call site of its own (line 1).
    std::fs::write(
        pkgdir.join("graph.tcl"),
        "proc Rbc_ActiveLegend {graph} {}\nproc Rbc_Wire {} { Rbc_ActiveLegend .g }\n",
    )
    .expect("write graph.tcl");
    std::fs::write(
        pkgdir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
         set auto_index(Rbc_Wire) [list source [file join $dir graph.tcl]]\n",
    )
    .expect("write tclIndex");

    let mut lsp = Lsp::with_config(serde_json::json!({
        "libraryPaths": [ libdir.to_string_lossy() ],
    }));

    let uri = unique_uri("tcl");
    let src = "Rbc_ActiveLegend .g\n";
    lsp.open_document(&uri, src);

    // The package database loads asynchronously at startup; poll (via the W123
    // clearing on the pull path) until it is live.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    let mut diags = lsp.pull_diagnostics(&uri);
    while has_code(&diags, "W123") && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        diags = lsp.pull_diagnostics(&uri);
    }

    // References from the workspace call site (cursor on the head, incl. the
    // declaration): the library's declaration (line 0) and its internal call
    // site (line 1) must both surface.
    let refs = crate::common::helpers::locations(&lsp.references(&uri, 0, 2, true));
    let lib_lines: Vec<i64> = refs
        .iter()
        .filter(|l| l.uri.ends_with("graph.tcl"))
        .filter_map(|l| {
            l.range
                .get("start")
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
        })
        .collect();
    assert!(
        lib_lines.contains(&0),
        "library declaration must be a reference target, got {refs:?}",
    );
    assert!(
        lib_lines.contains(&1),
        "library-internal call site must be a reference, got {refs:?}",
    );

    // Rename from the same cursor rewrites the workspace call *and* both
    // library sites, so the family stays consistent.
    let edits = crate::common::helpers::rename_edits(&lsp.rename(&uri, 0, 2, "Rbc_Shiny"));
    let lib_edit_count = edits
        .iter()
        .filter(|(u, _)| u.ends_with("graph.tcl"))
        .map(|(_, es)| es.len())
        .next()
        .unwrap_or(0);
    assert_eq!(
        lib_edit_count, 2,
        "library declaration + internal call must be rewritten, got {edits:?}",
    );
    assert!(
        edits.iter().any(|(u, _)| *u == uri),
        "the workspace call site is rewritten too: {edits:?}",
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
fn shimmer_committed_value_with_lindex_is_s100_info_with_tight_range() {
    // A *committed* Dict (from `[dict create]`) read as a list genuinely
    // shimmers — a pure string literal would promote for free (issue #940). The
    // range must cover only the `$x` argument, not the whole `lindex $x 0` call.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [dict create a 1 b 2]\nlindex $x 0\n");
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
fn issue_940_braced_list_literal_in_foreach_has_no_s100() {
    // Issue #940: a braced list literal is a pure string that `foreach` parses
    // into a list once, for free — no shimmer. Covers the reporter's exact
    // snippet plus the `{}` empty-list case named in the issue title.
    let mut lsp = Lsp::tcl();
    for src in [
        "set fontSizes {10.0 12.0 16.0 24.0}\nforeach size $fontSizes { puts $size }\n",
        "set empty {}\nforeach x $empty { puts $x }\n",
        "set a 1\nforeach b $a { puts $b }\n",
        "set d {a 1 b 2}\ndict for {k v} $d { puts \"$k=$v\" }\n",
    ] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(
            !has_code(&diags, "S100") && !has_code(&diags, "S101"),
            "issue #940: pure list literal must not shimmer for {src:?}: {diags:?}"
        );
    }
}

#[test]
fn issue_940_committed_container_in_foreach_still_fires_s100() {
    // TP control: the fix must not blanket-silence genuine shimmers — a
    // committed dict read as a list still fires.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set d [dict create a 1 b 2]\nforeach x $d { puts $x }\n",
    );
    assert!(
        has_code(&diags, "S100"),
        "committed dict in foreach must still shimmer: {diags:?}"
    );
}

#[test]
fn element_tracking_committed_lindex_retrieval_is_silent() {
    // P3 (type-tracking.md): container elements are shared objects, so a
    // committed numeric element retrieved by `lindex` is genuinely numeric —
    // arithmetic on it is not a shimmer.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set l [list [expr {2**20}] other]\nset first [lindex $l 0]\nset n [expr {$first + 1}]\nputs $n\n",
    );
    assert!(
        !has_code(&diags, "S100") && !has_code(&diags, "S101"),
        "a committed numeric element is numeric — no shimmer: {diags:?}"
    );
}

#[test]
fn union_lattice_three_way_merge_reports_every_type() {
    // P2: a three-way differently-typed merge stays a tracked union
    // (previously OVERDEFINED and silent); the phi message names every
    // member.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc f {c} {\n  if {$c == 1} { set x [list 1] } elseif {$c == 2} { set x [dict create a 1] } else { set x [expr {1+1}] }\n  return $x\n}\n",
    );
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert!(
        s100.iter().any(|d| {
            let m = d["message"].as_str().unwrap_or("");
            m.contains("merges") && m.contains("list") && m.contains("dict")
        }),
        "the 3-way merge message must name the merging types: {s100:?}"
    );
}

#[test]
fn bignum_values_raise_no_false_diagnostics() {
    // P4: beyond-wide integers classify on the tower (`Bignum`, coarse Int)
    // and fold exactly — a bignum literal chain must publish no shimmer /
    // type diagnostics. (The exact fold value is pinned at the compiler
    // tier: `fp_sh_23_bignum_folds_are_exact` and the tcl_expr_eval oracle
    // corpus.)
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set big [expr {2**64}]\nset next [expr {$big + 1}]\nincr next\nputs $next\n",
    );
    assert!(
        !has_code(&diags, "S100") && !has_code(&diags, "S101"),
        "exact bignum arithmetic is not a shimmer: {diags:?}"
    );
}

#[test]
fn commit_dataflow_second_conversion_fires_s100_with_committed_from_type() {
    // First-use commit: `expr` commits the numeric intrep on a pure literal;
    // the later `lindex` genuinely re-represents it (oracle: rep int → list).
    // The message names the *committed* from-type, not the def-time shape.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set v 5\nexpr {$v + 1}\nlindex $v 0\n");
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert!(
        s100.iter().any(|d| {
            d["message"]
                .as_str()
                .is_some_and(|m| m.contains("numeric intrep") && m.contains("expects list"))
        }),
        "expected a numeric→list second-conversion S100: {diags:?}"
    );
}

#[test]
fn commit_dataflow_loop_rethunk_fires_s101_each_target() {
    // A pure literal read as two distinct intreps inside a loop re-thunks
    // per iteration (oracle: list ↔ dict every pass) — both reads are S101.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set l {a 1 b 2}\nforeach i {1 2 3} {\n    llength $l\n    dict size $l\n}\n",
    );
    let s101: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S101"))
        .collect();
    assert!(
        s101.len() >= 2,
        "expected two per-iteration re-thunk S101s: {diags:?}"
    );
}

#[test]
fn commit_dataflow_path_dependent_merge_message() {
    // Two branch arms commit two different intreps; a post-merge use matching
    // neither pays on both paths — the message names the possibilities.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc f {c} {\n  set a {1 2}\n  if {$c} { llength $a } else { dict size $a }\n  string length $a\n}\n",
    );
    assert!(
        diags.iter().any(|d| {
            code_str(d).as_deref() == Some("S100")
                && d["message"]
                    .as_str()
                    .is_some_and(|m| m.contains("path-dependent"))
        }),
        "expected a path-dependent merge S100: {diags:?}"
    );
}

#[test]
fn w300_w103_silent_when_var_provably_literal() {
    // A `$var` proven to hold a compile-time literal path/filename is a known
    // constant — no more dangerous than writing it inline — so W300/W103 are
    // suppressed. A bare parameter stays flagged (the TP control).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let silent = lsp.open_ready(
        &uri,
        "set p \"./lib.tcl\"\nsource $p\nset f \"data.txt\"\nopen $f\n",
    );
    assert!(!has_code(&silent, "W300"), "W300 FP: {silent:?}");
    assert!(!has_code(&silent, "W103"), "W103 FP: {silent:?}");

    let uri2 = unique_uri("tcl");
    let fires = lsp.open_ready(&uri2, "proc f {p} { source $p }\nproc g {f} { open $f }\n");
    assert!(has_code(&fires, "W300"), "W300 TP (param): {fires:?}");
    assert!(has_code(&fires, "W103"), "W103 TP (param): {fires:?}");
}

#[test]
fn w213_unset_narrows_to_variable_and_carries_fix() {
    // `unset xs` on a possibly-undefined var → W213 squiggled on `xs` (col
    // 20..22 of the proc body), carrying the `-nocomplain` insert fix.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc foo {} { unset xs }\n");
    let w213: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W213"))
        .collect();
    assert_eq!(w213.len(), 1, "expected one W213: {diags:?}");
    assert_eq!(
        diag_range(w213[0]),
        ((0, 20), (0, 22)),
        "W213 must span only `xs`"
    );
}

#[test]
fn w214_unused_param_anchors_on_the_param_name() {
    // The squiggle must cover only the offending parameter's *name* (`unused`),
    // not the whole `proc` definition, so it aligns with go-to-definition/rename.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc f {unused x} { puts $x }\n");
    let w214: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W214"))
        .collect();
    assert_eq!(w214.len(), 1, "expected exactly one W214: {diags:?}");
    assert_eq!(
        diag_range(w214[0]),
        ((0, 8), (0, 14)),
        "W214 must span only `unused`"
    );
}

#[test]
fn w214_two_unused_params_get_separate_tight_ranges() {
    // Two unused params must yield two squiggles, each on its own name — not two
    // diagnostics stacked on the whole proc definition.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc g {aa bb} { return 0 }\n");
    let ranges: Vec<((i64, i64), (i64, i64))> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W214"))
        .map(diag_range)
        .collect();
    assert!(ranges.contains(&((0, 8), (0, 10))), "aa range: {ranges:?}");
    assert!(ranges.contains(&((0, 11), (0, 13))), "bb range: {ranges:?}");
}

#[test]
fn ordering_compare_on_strings_has_no_s100() {
    // `$s < "banana"` compares two strings — Tcl stays on the string path and
    // never coerces `$s` to a number, so its intrep is untouched (verified on
    // tclsh 8.6/9.0). No S100 (was a false positive before the ordering ops
    // were gated the same way as `==`/`!=`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set s [string trim hello]\nexpr {$s < \"banana\"}\n");
    assert!(!has_code(&diags, "S100"), "unexpected S100: {diags:?}");
    assert!(!has_code(&diags, "S101"), "unexpected S101: {diags:?}");
}

#[test]
fn ordering_compare_string_vs_numeric_literal_stays_silent() {
    // FP guard: a numeric literal on one side does NOT force the other
    // operand onto the numeric path — C Tcl probes each operand's OWN value
    // (`GetNumberFromObj` per operand), and "hello" cannot parse, so the
    // comparison string-compares and `s` keeps its `string` intrep
    // (tclsh-verified; see FP-SH-20's addendum). The old "TP control"
    // asserting S100 here locked in a refuted claim.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set s [string trim hello]\nexpr {$s <= 5}\n");
    assert!(!has_code(&diags, "S100"), "unexpected S100: {diags:?}");
    assert!(!has_code(&diags, "S101"), "unexpected S101: {diags:?}");
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
    let src = "interp alias {} myindex {} ::lindex\nset x [dict create a 1 b 2]\nmyindex $x 0\n";
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
    let diags = lsp.open_ready(
        &uri,
        "set x [dict create a 1 b 2]\n# noqa: S100\nlindex $x 0\n",
    );
    assert!(
        !has_code(&diags, "S100"),
        "S100 must be suppressed by a preceding '# noqa: S100': {diags:?}"
    );
}

#[test]
fn shimmer_noqa_for_unrelated_code_does_not_suppress_s100() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set x [dict create a 1 b 2]\n# noqa: W100\nlindex $x 0\n",
    );
    assert!(
        has_code(&diags, "S100"),
        "S100 must still fire when the preceding noqa names an unrelated code: {diags:?}"
    );
}

// -- S103 (shared-value copy-on-write) ------------------------------------

#[test]
fn s103_fires_on_shared_copy_mutation_as_hint_with_tight_range() {
    // The tight pattern: `set b $a`, `lappend b y`, `a` read afterwards —
    // tclsh-verified (8.6.14): b holds the same object as a until the
    // lappend, which duplicates the whole list before appending. The
    // range covers the mutating command; the severity is Hint (4).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set a [lrepeat 1000 x]\nset b $a\nlappend b y\nputs [llength $a]\n",
    );
    let s103: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S103"))
        .collect();
    assert_eq!(s103.len(), 1, "expected exactly one S103: {diags:?}");
    let range = &s103[0]["range"];
    assert_eq!(range["start"]["line"], 2);
    assert_eq!(range["start"]["character"], 0);
    assert_eq!(range["end"]["line"], 2);
    assert_eq!(range["end"]["character"], 11, "must cover `lappend b y`");
    assert_eq!(
        s103[0].get("severity").and_then(Value::as_i64),
        Some(4), // Hint
        "S103 is a hint: {:?}",
        s103[0]
    );
}

#[test]
fn s103_silent_for_param_mutation_and_explicit_copy_and_dead_source() {
    let mut lsp = Lsp::tcl();
    // FP guard: mutating a parameter directly is idiomatic (the caller's
    // copy is the normal case) — silent.
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {l} {\n    lappend l x\n    return $l\n}\n");
    assert!(!has_code(&diags, "S103"), "param mutation FP: {diags:?}");

    // FP guard: an explicit copy is not a pure-var-ref pair — silent.
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set a [list 1 2 3]\nset b [lrange $a 0 end]\nlappend b y\nputs [llength $a]\n",
    );
    assert!(!has_code(&diags, "S103"), "explicit copy FP: {diags:?}");

    // FP guard: the source is dead past the mutation — silent (the sharing
    // is unwanted; deliberate under-approximation).
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set a [list 1 2 3]\nset b $a\nlappend b y\nputs [llength $b]\n",
    );
    assert!(!has_code(&diags, "S103"), "dead source FP: {diags:?}");

    // FP guard: array elements conflate onto one SSA symbol — silent.
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set arr(src) [list 1 2 3]\nset b $arr(src)\nlappend b y\nputs [llength $arr(src)]\n",
    );
    assert!(!has_code(&diags, "S103"), "array element FP: {diags:?}");
}

#[test]
fn s103_noqa_suppresses_and_shimmer_toggle_disables() {
    // `# noqa: S103` on the preceding line suppresses it through the live
    // pipeline (the shimmer-family suppression path).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set a [list 1 2 3]\nset b $a\n# noqa: S103\nlappend b y\nputs [llength $a]\n",
    );
    assert!(
        !has_code(&diags, "S103"),
        "S103 must be suppressed by a preceding '# noqa: S103': {diags:?}"
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

/// Per-call counter for the workspace-folder consumer-rename fixture dir.
static WS_FOLDER_RENAME_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The VS Code fixture shape for M8's consumer rename, driven end-to-end
/// through a REAL workspace folder (no `libraryPaths` config): the library
/// lives in a `rbclib/` subdirectory of the folder (with a `tclIndex`), the
/// consumer at the folder root calls `Rbc_ActiveLegend` bare, and a rename at
/// the consumer's call site must rewrite the library declaration.  Mirrors
/// `editors/vscode/src/test/renameSymbol.test.ts`'s M8 test so the
/// environment-specific path (workspace-scan indexing, not the
/// `libraryPaths` autoload tier) is pinned in the fast harness too.
#[test]
fn consumer_rename_resolves_through_a_real_workspace_folder_m8() {
    use std::sync::atomic::Ordering;

    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-ws-rename-{}-{}",
        std::process::id(),
        WS_FOLDER_RENAME_N.fetch_add(1, Ordering::Relaxed)
    ));
    let libdir = root.join("rbclib");
    std::fs::create_dir_all(&libdir).expect("mk lib dir");
    std::fs::write(
        libdir.join("graph.tcl"),
        "# Auto-loaded library procs (BLT/Rbc idiom) — reachable by bare name via the\n\
         # sibling tclIndex, with no `package require` in the caller.\n\
         proc Rbc_ActiveLegend {graph} {}\n\
         proc Rbc_ZoomStack {graph args} {}\n",
    )
    .expect("write graph.tcl");
    std::fs::write(
        libdir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
         set auto_index(Rbc_ZoomStack) [list source [file join $dir graph.tcl]]\n",
    )
    .expect("write tclIndex");
    let consumer_path = root.join("autoloadLibrary.tcl");
    std::fs::write(&consumer_path, "Rbc_ActiveLegend .g\nRbc_ZoomStack .g\n")
        .expect("write consumer");

    let mut lsp = Lsp::tcl();
    // Attach the real folder (the fake `initialize` root indexes nothing) and
    // let the rescan index rbclib/graph.tcl from disk.
    let folder_uri = format!("file://{}", root.to_string_lossy());
    lsp.notify(
        "workspace/didChangeWorkspaceFolders",
        serde_json::json!({
            "event": {
                "added": [{ "uri": folder_uri, "name": "fixture" }],
                "removed": []
            }
        }),
    );

    let uri = format!("file://{}", consumer_path.to_string_lossy());
    lsp.open_document(&uri, "Rbc_ActiveLegend .g\nRbc_ZoomStack .g\n");

    // The folder scan runs asynchronously; poll the rename until the library
    // edit appears (or the deadline), exactly like the editor test.
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    let mut lib_lines: Vec<i64> = Vec::new();
    while std::time::Instant::now() < deadline && lib_lines.is_empty() {
        let resp = lsp.request(
            "textDocument/rename",
            serde_json::json!({
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 3 },
                "newName": "Rbc_ShinyLegend"
            }),
        );
        if let Some(changes) = resp.get("changes").and_then(|c| c.as_object()) {
            for (target, edits) in changes {
                if target.ends_with("graph.tcl") {
                    lib_lines = edits
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|e| e.pointer("/range/start/line").and_then(Value::as_i64))
                        .collect();
                }
            }
        }
        if lib_lines.is_empty() {
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    assert!(
        lib_lines.contains(&2),
        "the library declaration (graph.tcl line 2) must be rewritten: {lib_lines:?}"
    );

    // VS Code gates every rename behind `textDocument/prepareRename` — the
    // consumer document has no local declaration, so prepare must take the
    // workspace fall-through and accept with the call-site word's own range
    // (a `null` here made the editor abort with "The element can't be
    // renamed." before ever sending the rename).
    let prep = lsp.request(
        "textDocument/prepareRename",
        serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": 0, "character": 3 }
        }),
    );
    assert_eq!(
        prep.pointer("/placeholder").and_then(Value::as_str),
        Some("Rbc_ActiveLegend"),
        "prepare accepts the consumer call site: {prep:?}"
    );
    assert_eq!(
        prep.pointer("/range/start/character")
            .and_then(Value::as_i64),
        Some(0),
        "the highlighted range is the call-site word: {prep:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// -- Dialect-profile availability (dialect-profile-model.md, Milestone 2):
// vendor dialects resolve their embedded Tcl core end-to-end — server →
// JSON-RPC → publishDiagnostics — via the composed (version|vendor) profile
// masks; the version ladder and the subtractive iRules view still hold.

#[test]
fn iapps_embedded_85_core_is_clean_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("iapp");
    let diags = lsp.open_ready_lang(
        &uri,
        "dict set cfg pool p1\nlassign {a b} x y\napply {{v} {return $v}} 1\n",
        "tcl-iapp",
    );
    assert!(
        !has_code(&diags, "W123") && !has_code(&diags, "W002"),
        "iApps (Tcl 8.5.13 host) must resolve 8.5 core cleanly: {:?}",
        codes(&diags)
    );
}

#[test]
fn iapps_86_core_still_draws_w123_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("iapp");
    let diags = lsp.open_ready_lang(&uri, "lmap x {1 2} {set x}\n", "tcl-iapp");
    assert!(
        has_code(&diags, "W123"),
        "lmap is 8.6 — unavailable on the iApps 8.5 base: {:?}",
        codes(&diags)
    );
}

#[test]
fn expect_embedded_86_core_is_clean_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("exp");
    let diags = lsp.open_ready_lang(
        &uri,
        "spawn ssh host\ndict set cfg k v\ncoroutine pump ::apply {{} {}}\n",
        "tcl-expect",
    );
    assert!(
        !has_code(&diags, "W123") && !has_code(&diags, "W002"),
        "expect (Tcl 8.6 base) must resolve 8.6 core + expect surface cleanly: {:?}",
        codes(&diags)
    );
}

#[test]
fn irules_banned_commands_still_flag_end_to_end() {
    // The subtractive iRules profile is unchanged by the composed-mask fix:
    // banned 8.4 core still draws W002 in a `when` handler.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n  exec /bin/true\n}\n",
        "tcl-irule",
    );
    assert!(
        has_code(&diags, "W002"),
        "exec is banned in iRules: {:?}",
        codes(&diags)
    );
}

#[test]
fn argument_dsl_rung_gates_format_and_string_is_end_to_end() {
    // Milestone 7 (§6): the argument mini-languages validate against the
    // dialect's effective Tcl version. iRules embed Tcl 8.4.6, so the
    // 8.6-introduced `format %b` and the 9.0-introduced `string is dict`
    // both flag; the same source is clean on a 9.0 host.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n  set x [format %b 5]\n  set y [string is dict {a 1}]\n}\n",
        "tcl-irule",
    );
    assert!(
        has_code(&diags, "W138") && has_code(&diags, "W137"),
        "8.4-embedded iRules must flag %b (8.6+) and the dict class (9.0+): {:?}",
        codes(&diags)
    );
    let uri2 = unique_uri("tcl90");
    let diags2 = lsp.open_ready_lang(
        &uri2,
        "set x [format %b 5]\nset y [string is dict {a 1}]\n",
        "tcl9.0",
    );
    assert!(
        !has_code(&diags2, "W138") && !has_code(&diags2, "W137"),
        "a 9.0 host accepts both: {:?}",
        codes(&diags2)
    );
}

#[test]
fn tmsh_first_class_gates_both_directions_end_to_end() {
    // Milestone 6 (D8): a tmsh document analyses under TCL85|TMSH — the
    // tmsh:: surface and the 8.5 core are clean, while 8.6-core draws the
    // §7.2 reverse-regression diagnostic.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tmsh");
    let diags = lsp.open_ready_lang(
        &uri,
        "tmsh::create ltm pool p1\ndict set cfg k v\n",
        "tcl-tmsh",
    );
    assert!(
        !has_code(&diags, "W123") && !has_code(&diags, "W002"),
        "tmsh surface + 8.5 core must be clean: {:?}",
        codes(&diags)
    );
    let uri2 = unique_uri("tmsh");
    let diags2 = lsp.open_ready_lang(&uri2, "lmap x {1 2} {set x}\n", "tcl-tmsh");
    assert!(
        has_code(&diags2, "W123") || has_code(&diags2, "W002"),
        "lmap is 8.6 — unavailable on the tmsh 8.5 base: {:?}",
        codes(&diags2)
    );
}

#[test]
fn bpf_first_class_admits_90_core_end_to_end() {
    // Milestone 6 (D7): bpf = TCL90|BPF — 9.0-era core (lmap, dict) is
    // real, and the bpf surface resolves.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("bpf");
    let diags = lsp.open_ready_lang(&uri, "dict set cfg k v\nlmap x {1 2} {set x}\n", "tcl-bpf");
    assert!(
        !has_code(&diags, "W123") && !has_code(&diags, "W002"),
        "9.0 core is real under bpf: {:?}",
        codes(&diags)
    );
}

#[test]
fn irules_subcommands_named_like_banned_commands_are_clean_end_to_end() {
    // Milestone 5 retag FP-fix: `DNS::header cd` (the DNS Checking-Disabled
    // flag) and `IP::stats in` (inbound stats) are real iRules subcommands
    // whose names collide with the banned `cd` command and the `in`
    // operator spelling. The old bulk name-keyed tagging hid them and drew
    // spurious availability/subcommand diagnostics; exclusion is keyed on
    // the resolved spec now, never on a bare name.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when DNS_REQUEST {\n  DNS::header cd\n}\nwhen CLIENT_ACCEPTED {\n  IP::stats in\n}\n",
        "tcl-irule",
    );
    assert!(
        !has_code(&diags, "W001") && !has_code(&diags, "W002") && !has_code(&diags, "W123"),
        "real iRules subcommands must not be hidden by name collisions: {:?}",
        codes(&diags)
    );
}

// -- Issue #968: W123 false-positived on every built-in `expr` math
// function (`sin(...)`, `max(...)`, ...) — `expr_function_call_records_a_
// mathfunc_invocation` (commands.rs) already resolved the call to
// `::tcl::mathfunc::<name>`, but the W123 pass never recognised that
// qualified name unless a same-named user proc happened to shadow it.

#[test]
fn issue_968_builtin_expr_math_functions_are_clean_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [expr {sin(1.0) + max(1, 2, 3)}]\nputs $x\n");
    assert!(
        !has_code(&diags, "W123"),
        "issue #968: built-in expr math functions must not draw W123: {:?}",
        codes(&diags)
    );
}

#[test]
fn issue_968_nested_expr_math_functions_are_clean_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [expr {sqrt(abs($y))}]\n");
    assert!(
        !has_code(&diags, "W123"),
        "nested built-in expr math functions must not draw W123: {:?}",
        codes(&diags)
    );
}

#[test]
fn issue_968_unknown_expr_function_still_w123_end_to_end() {
    // Control: the fix must not swallow a genuinely unknown function name.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [expr {frobnicate(1)}]\n");
    assert!(
        has_code(&diags, "W123"),
        "an unknown expr function must still draw W123: {:?}",
        codes(&diags)
    );
    assert!(
        diags.iter().any(|d| message(d).contains("frobnicate")),
        "message names the unknown function: {diags:?}",
    );
}

#[test]
fn issue_968_version_gated_math_function_dual_fires_end_to_end() {
    // `min`/`max` are TIP 232 (Tcl 8.5+) — under an 8.4 document the name
    // is a real function elsewhere but disabled here, so both the
    // dialect-availability diagnostic (W002) and the unresolved-command
    // diagnostic (W123) fire, matching the established dual-fire precedent
    // for a dialect-disabled registry builtin (e.g. `lmap` under an 8.5
    // host, exercised by `iapps_86_core_still_draws_w123_end_to_end` above).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl84");
    let diags = lsp.open_ready_lang(&uri, "set x [expr {min(1, 2)}]\n", "tcl8.4");
    assert!(
        has_code(&diags, "W002") && has_code(&diags, "W123"),
        "min() under tcl8.4 must draw both W002 and W123: {:?}",
        codes(&diags)
    );

    let uri2 = unique_uri("tcl86");
    let diags2 = lsp.open_ready_lang(&uri2, "set x [expr {min(1, 2)}]\n", "tcl8.6");
    assert!(
        !has_code(&diags2, "W002") && !has_code(&diags2, "W123"),
        "min() under tcl8.6 must be clean: {:?}",
        codes(&diags2)
    );
}

#[test]
fn issue_968_expr_function_after_rename_away_still_w123_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "rename ::tcl::mathfunc::sin {}\nset x [expr {sin(1.0)}]\n",
    );
    assert!(
        has_code(&diags, "W123"),
        "a call through a renamed-away mathfunc must be unresolved: {:?}",
        codes(&diags)
    );
}

#[test]
fn issue_968_user_defined_mathfunc_override_still_resolves_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc ::tcl::mathfunc::myfunc {x} { return $x }\nset r [expr {myfunc(1)}]\n",
    );
    assert!(
        !has_code(&diags, "W123"),
        "a user-defined mathfunc override must resolve end-to-end: {:?}",
        codes(&diags)
    );
}

/// idx 77 (differential-audit main audit wave, high severity, tomato
/// corpus): `Vector3d.tcl`'s `method * {type}` reads `$other`, a variable
/// belonging to a *sibling* method (`DotProduct {other}`), never bound in
/// `*`'s own scope — tclsh8.6/9.0.4 both crash with `can't read "other": no
/// such variable` the instant `*` runs on an object operand. The entire
/// CFG/SSA dataflow diagnostic family (W210 read-before-set and its
/// siblings) previously never ran on any `TclOO`/snit method body at all —
/// a systemic false-negative gap, not a one-off miss — because the
/// per-function diagnostic loop only ever iterated `cu.procedures`, never
/// `cu.methods`. The identical unbound-read shape inside a plain `proc`
/// already fired W210.
#[test]
fn method_body_unbound_sibling_parameter_read_flags_w210_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Vector3d {\n    variable _x\n    constructor {x} { set _x $x }\n    method DotProduct {other} { return [expr {$_x * $other}] }\n    method Buggy {type} { return [my DotProduct $other] }\n}\n",
    );
    assert!(
        has_code(&diags, "W210"),
        "the unbound `$other` read inside Buggy must flag: {:?}",
        codes(&diags)
    );
}

/// FP guard (issue #923 idx 77): the fix must not flood false W210s on
/// ordinary instance-variable reads or a method's own parameters —
/// `TclOO` auto-binds class-level `variable` declarations in every
/// method's scope with no visible `variable` statement in the body itself.
#[test]
fn method_body_instance_variable_and_own_parameter_reads_stay_clean_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create P {\n    variable _x\n    constructor {x} { set _x $x }\n    method DotProduct {other} { return [expr {$_x * $other}] }\n}\n",
    );
    assert!(
        !has_code(&diags, "W210"),
        "legitimate instance-variable / own-parameter reads must not flag: {:?}",
        codes(&diags)
    );
}

/// idx 90 (differential-audit main audit wave, high severity): `tcl::OptProc`
/// (the `opt` package's automatic-option-parsing proc definer) had no
/// `AnalyserHookId` at all, so `all_procs` kept the stub's `{}`-arity
/// `ProcDef` — every real call falsely drew "wrong number of arguments"
/// (E003). tclsh9.0/8.6-verified: the runtime always installs
/// `::proc $name args {...}`, so any call arity is legitimate.
#[test]
fn opt_proc_real_call_draws_no_false_arity_diagnostic_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require opt\n::tcl::OptProc greet {child -use -display} { return $child }\ngreet a b c\n",
    );
    assert!(
        !has_code(&diags, "E003") && !has_code(&diags, "W123"),
        "a real tcl::OptProc call must never draw a false arity or unknown-command diagnostic: {:?}",
        codes(&diags)
    );
}

/// Issue #988: a direct call into a private `::tcl::` implementation
/// namespace is flagged W143 with a concrete public-command suggestion.
#[test]
fn direct_private_tcl_namespace_call_is_w143_with_suggestion() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "::tcl::dict::create a 1\n");
    let hits = with_code(&diags, "W143");
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one W143: {:?}",
        codes(&diags)
    );
    assert!(
        message(&hits[0]).contains("dict create"),
        "expected the concrete 'dict create' suggestion in the message, got: {:?}",
        message(&hits[0])
    );
}

/// The ordinary public ensemble call is never flagged, nor is a user's own
/// namespace nested under `tcl::`, nor Tcl's own public `tcl::`-rooted
/// namespaces (`tcl::mathop`/`tcl::mathfunc`).
#[test]
fn public_and_user_owned_tcl_namespaces_are_not_w143() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "dict create a 1\n\
         namespace eval ::tcl::mycustom { proc foo {} {} }\n\
         ::tcl::mycustom::foo\n\
         ::tcl::mathop::+ 1 2\n\
         tcl::mathop::* 2 3\n\
         ::tcl::mathfunc::sin 0\n\
         tcl::mathfunc::max 1 2\n",
    );
    assert!(
        !has_code(&diags, "W143"),
        "no public/user-owned namespace call should draw W143: {:?}",
        codes(&diags)
    );
}

/// tcllib publishes public packages *inside* `::tcl::chan` (the whole
/// `virtchannel_base` module), so W143's namespace claim there is
/// tail-restricted to real `chan` ensemble subcommands — a
/// `tcl::chan::memchan` call must never be flagged, and must never draw
/// W143 alongside the W120 that asks for its `package require`.
#[test]
fn public_tcllib_channel_packages_are_not_w143() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require tcl::chan::memchan\n\
         set c [tcl::chan::memchan]\n\
         set f [tcl::chan::fifo]\n",
    );
    assert!(
        !has_code(&diags, "W143"),
        "public tcllib virtchannel packages must not draw W143: {:?}",
        codes(&diags)
    );
}

/// A file that defines the command itself owns that name — a user's own
/// `proc ::tcl::dict::mine` is not a call into Tcl's internals.
#[test]
fn own_proc_in_a_private_namespace_is_not_w143() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "namespace eval ::tcl::dict { proc mine {d} { return $d } }\n\
         ::tcl::dict::mine {a 1}\n",
    );
    assert!(
        !has_code(&diags, "W143"),
        "a self-defined private-namespace command must not draw W143: {:?}",
        codes(&diags)
    );
}
