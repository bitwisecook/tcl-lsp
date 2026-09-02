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

//! Oracle-pinned regression coverage for the r4-parser-gaps lane
//! (#1576, #1586, #1577). Every expected message/errorCode below was taken
//! verbatim from `tclsh9.0` (9.0.4) and/or `tclsh8.6` (8.6.16); a reader can
//! paste the sheet into a real `tclsh` and re-derive the expectation without
//! this harness.

use tcl_runtime::interp::{Code, Interp};

/// Evaluate `sheet` and return `(code, result, errorCode)`. `errorCode` is
/// read from `::errorCode`, which the outermost `eval_str` publishes on an
/// uncaught error — matching how `catch ... opts; dict get $opts -errorcode`
/// observes it in real Tcl.
fn run(sheet: &str) -> (Code, String, String) {
    let mut interp = Interp::new();
    let code = interp.eval_str(sheet.as_bytes());
    let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    let error_code = if code == Code::Error {
        interp.eval_str(b"set ::errorCode");
        String::from_utf8_lossy(&interp.result_bytes()).into_owned()
    } else {
        String::new()
    };
    (code, result, error_code)
}

// ---------------------------------------------------------------------------
// #1576 — an unterminated `{` word must raise `missing close-brace`, not
// tokenize best-effort. Oracle: tclsh 8.6.16/9.0.4 both raise `missing
// close-brace` with `-errorcode NONE` for every repro in the issue (`list`,
// `set`, `string length`).
// ---------------------------------------------------------------------------

#[test]
fn unterminated_brace_word_raises_missing_close_brace() {
    let (code, result, error_code) = run(r#"eval "list a \{ b""#);
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace");
    assert_eq!(error_code, "NONE");
}

#[test]
fn unterminated_brace_word_raises_for_set_too() {
    let (code, result, _) = run(r#"eval "set x \{ b""#);
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace");
}

#[test]
fn unterminated_brace_word_raises_for_string_length_too() {
    let (code, result, _) = run(r#"eval "string length \{ b""#);
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace");
}

/// A properly closed braced word — including one glued to a second braced
/// word (`{*}{b c}`, no unterminated construct) — is unaffected by the
/// #1576 fix.
#[test]
fn well_formed_braced_words_still_parse() {
    let (code, result, _) = run("list a {hello world} c");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "a {hello world} c");

    let (code, result, _) = run("list a {*}{b c} d");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "a b c d");
}

// ---------------------------------------------------------------------------
// #1586 — an unterminated `${` inside a script word (or a script word nested
// in a command substitution) must raise `missing close-brace for variable
// name`, the same message `subst` already raises for the identical
// construct — not the lenient lexer recovery's `${a{` == name `a{` reading
// (which then fails downstream as `can't read "a{"`). Oracle: tclsh
// 8.6.16/9.0.4 both raise `missing close-brace for variable name` with
// `-errorcode NONE`.
// ---------------------------------------------------------------------------

#[test]
fn unterminated_braced_var_in_a_script_word_raises_missing_close_brace_for_var() {
    // eval "set x \"${a{\""
    let (code, result, error_code) = run("eval \"set x \\\"\\$\\{a\\{\\\"\"");
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace for variable name");
    assert_eq!(error_code, "NONE");
}

/// The same gap, nested inside a command substitution reached through
/// `subst` — `subst $t` where `t` is `[set y ${a{b]` — matching the second
/// row of the issue's table (the C parser reports the same error at every
/// nesting depth, not `can't read "a{b"`).
#[test]
fn unterminated_braced_var_nested_in_a_command_subst_raises_the_same_error() {
    // `t`'s value is the 14-byte string `[set y ${a{b]` — built through
    // `format` rather than a literal `{...}` word, since its raw text has
    // two unmatched `{` and would itself be an unterminated *outer* brace
    // word (this file's own #1576 sibling gap, not what this test targets).
    let (code, result, _) = run(r#"set t [format {[set y $%sa%sb]} "{" "{"]
subst $t"#);
    assert_eq!(code, Code::Error);
    assert_eq!(result, "missing close-brace for variable name");
}

/// A well-formed `${name}` (including one immediately followed by more
/// substitution) is unaffected by the #1586 fix.
#[test]
fn well_formed_braced_var_still_substitutes() {
    let (code, result, _) = run("set a hi\nsubst {x${a}y}");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "xhiy");
}
