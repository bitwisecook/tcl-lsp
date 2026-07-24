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

//! Native port of `tests/lsp_e2e/test_hover_e2e.py` (full parity, all 37 tests).
//!
//! Full-parity port of `tests/test_hover.py` (the in-process provider tests)
//! onto the live JSON-RPC surface: open a document, wait for analysis, ask
//! `textDocument/hover` at a position, assert on the rendered markdown. The
//! iRules-dialect cases open the document with the `tcl-irule` language id (and
//! an `.irule` URI) so the server resolves the F5 command pack — the wire
//! equivalent of the in-process `configure_signatures(dialect="f5-irules")`.
//!
//! The white-box test that `_infer_var_type` / `_infer_var_taint` agree on the
//! cached vs recomputed path has no JSON-RPC surface and stays in
//! `tests/test_hover.py`.

use crate::common::helpers::hover_text;
use crate::common::{Lsp, unique_uri};

/// Hover text at a position (`_hover` in the pytest suite).
fn hover(lsp: &mut Lsp, uri: &str, line: u32, ch: u32) -> String {
    hover_text(&lsp.hover(uri, line, ch))
}

// -- TestCommandHover ----------------------------------------------------

#[test]
fn builtin_command() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\n");
    let text = hover(&mut lsp, &uri, 0, 1);
    assert!(text.contains("set"), "hover: {text:?}");
    assert!(text.to_lowercase().contains("variable"), "hover: {text:?}");
}

#[test]
fn puts_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts hello\n");
    assert!(hover(&mut lsp, &uri, 0, 2).contains("puts"));
}

#[test]
fn unknown_command_no_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "mycommand arg\n");
    assert!(hover_text(&lsp.hover(&uri, 0, 4)).is_empty());
}

#[test]
fn subcommand_parent_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "string length hello\n");
    let text = hover(&mut lsp, &uri, 0, 2);
    assert!(
        text.to_lowercase().contains("subcommand") || text.contains("string"),
        "hover: {text:?}"
    );
}

#[test]
fn socket_hover_uses_registry_snippet() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "socket localhost 80\n");
    let text = hover(&mut lsp, &uri, 0, 1);
    assert!(
        text.contains("socket ?options? host port"),
        "hover: {text:?}"
    );
    assert!(
        text.to_lowercase().contains("tcp client or server socket"),
        "hover: {text:?}"
    );
}

#[test]
fn socket_server_option_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "socket -server accept 8080\n");
    let text = hover(&mut lsp, &uri, 0, 8);
    assert!(text.contains("-server"), "hover: {text:?}");
    assert!(text.to_lowercase().contains("callback"), "hover: {text:?}");
}

#[test]
fn socket_server_option_hover_ignores_semicolon_in_quoted_arg() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let source = "socket \"a;b\" -server accept 8080\n";
    lsp.open_ready(&uri, source);
    let option_pos = u32::try_from(source.find("-server").unwrap()).unwrap() + 2;
    let text = hover(&mut lsp, &uri, 0, option_pos);
    assert!(text.contains("-server"), "hover: {text:?}");
}

#[test]
fn curated_command_hover_does_not_mark_refinement_status() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "append x y\n");
    assert!(
        !hover(&mut lsp, &uri, 0, 1).to_lowercase().contains("note:"),
        "hover should not mark refinement status"
    );
}

// -- TestProcHover -------------------------------------------------------

#[test]
fn proc_signature() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {name} { puts \"Hello $name\" }\ngreet World\n",
    );
    let text = hover(&mut lsp, &uri, 1, 2);
    assert!(text.contains("greet"), "hover: {text:?}");
    assert!(text.contains("name"), "hover: {text:?}");
}

#[test]
fn proc_with_doc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# Says hello to someone\nproc greet {name} { puts \"Hello $name\" }\ngreet World\n";
    lsp.open_ready(&uri, src);
    assert!(hover(&mut lsp, &uri, 2, 2).contains("Says hello"));
}

#[test]
fn proc_multiline_doc() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# Greet a user.\n\
               # This proc says hello.\n\
               proc greet {name} { puts \"Hello $name\" }\n\
               greet World\n";
    lsp.open_ready(&uri, src);
    let text = hover(&mut lsp, &uri, 3, 2);
    assert!(text.contains("Greet a user."), "hover: {text:?}");
    assert!(text.contains("This proc says hello."), "hover: {text:?}");
}

#[test]
fn proc_body_docstring() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc update_pkg {filename} {\n\
               \x20   # ....................................................................\n\
               \x20   # Update build date and time in pkg\n\
               \x20   #\n\
               \x20   # @param filename - Filename to pkg file\n\
               \x20   # ....................................................................\n\
               \x20   puts $filename\n\
               }\n\
               update_pkg test.pkg\n";
    lsp.open_ready(&uri, src);
    let text = hover(&mut lsp, &uri, 8, 2);
    assert!(text.contains("Update build date"), "hover: {text:?}");
    assert!(text.contains("**filename**"), "hover: {text:?}");
}

#[test]
fn proc_no_comment_bleed() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc first {} {\n\
               \x20   # internal comment\n\
               \x20   puts hello\n\
               }\n\
               proc second {} { puts world }\n\
               second\n";
    lsp.open_ready(&uri, src);
    assert!(!hover(&mut lsp, &uri, 5, 2).contains("internal comment"));
}

#[test]
fn proc_doxygen_tags() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "# @brief Calculate the sum\n\
               # @param a - First number\n\
               # @param b - Second number\n\
               # @return The sum of a and b\n\
               proc add {a b} { expr {$a + $b} }\n\
               add 1 2\n";
    lsp.open_ready(&uri, src);
    let text = hover(&mut lsp, &uri, 5, 2);
    assert!(text.contains("Calculate the sum"), "hover: {text:?}");
    assert!(text.contains("**Parameters:**"), "hover: {text:?}");
    assert!(text.contains("**a**"), "hover: {text:?}");
    assert!(text.contains("**b**"), "hover: {text:?}");
    assert!(text.contains("**Returns:**"), "hover: {text:?}");
}

#[test]
fn proc_with_defaults() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greet {{name World}} { puts \"Hello $name\" }\ngreet\n",
    );
    assert!(hover(&mut lsp, &uri, 1, 2).contains("World"));
}

// -- TestNamespaceResolutionHover ------------------------------------------
// C Tcl resolves an unqualified command in the current namespace first, then
// the global namespace (`Tcl_FindCommand`, `tclNamesp.c`) — never a sibling
// namespace picked by proc-table iteration order.

#[test]
fn unqualified_call_hover_prefers_callers_namespace_proc() {
    // Two namespaces each define `helper`; hovering the unqualified call
    // inside ::b must surface ::b::helper (with its own param list).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval a {\n    proc helper {x} { return 1 }\n}\nnamespace eval b {\n    proc helper {y} { return 2 }\n    helper 5\n}\n";
    lsp.open_ready(&uri, src);
    let text = hover(&mut lsp, &uri, 5, 6);
    assert!(text.contains("proc ::b::helper {y}"), "hover: {text:?}");
}

#[test]
fn namespace_proc_set_shadows_builtin_only_inside_its_namespace() {
    // A namespace proc named like a builtin: inside ::ns the proc wins; at
    // global scope only the builtin resolves — a proc in an unrelated
    // namespace must not hijack builtin hover.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ns {\n    proc set {key value} { return $value }\n    set x 1\n}\nset y 2\n";
    lsp.open_ready(&uri, src);
    let inside = hover(&mut lsp, &uri, 2, 5);
    assert!(inside.contains("proc ::ns::set"), "inside ns: {inside:?}");
    let global = hover(&mut lsp, &uri, 4, 1);
    assert!(global.contains("built-in"), "global: {global:?}");
    assert!(!global.contains("::ns::set"), "global: {global:?}");
}

#[test]
fn global_call_hover_fallback_is_deterministic() {
    // No ::helper exists, so the lenient fallback fires — and must pick the
    // lexicographically smallest qualified name (::a::helper), identically
    // on every repeat (::z::helper is deliberately defined first).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval z {\n    proc helper {a} { return 26 }\n}\nnamespace eval a {\n    proc helper {b} { return 1 }\n}\nhelper 1\n";
    lsp.open_ready(&uri, src);
    let first = hover(&mut lsp, &uri, 6, 2);
    assert!(first.contains("proc ::a::helper"), "hover: {first:?}");
    for attempt in 0..3 {
        let repeat = hover(&mut lsp, &uri, 6, 2);
        assert_eq!(first, repeat, "attempt {attempt}: hover must be stable");
    }
}

// -- TestVariableHover ---------------------------------------------------

#[test]
fn var_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    let text = hover(&mut lsp, &uri, 1, 7);
    assert!(text.contains("Variable"), "hover: {text:?}");
    assert!(text.contains('x'), "hover: {text:?}");
}

#[test]
fn var_hover_shows_refs() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\nputs $x\n");
    assert!(
        hover(&mut lsp, &uri, 1, 7)
            .to_lowercase()
            .contains("reference")
    );
}

#[test]
fn namespace_var_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval myns {\n\
               \x20   variable nsVar 1\n\
               \x20   puts $nsVar\n\
               }\n";
    lsp.open_ready(&uri, src);
    let text = hover(&mut lsp, &uri, 2, 10);
    assert!(text.contains("Variable"), "hover: {text:?}");
    assert!(text.contains("nsVar"), "hover: {text:?}");
}

// -- TestLeanHover -------------------------------------------------------
// Hover shows synopsis + summary but omits snippet/examples.

#[test]
fn set_hover_omits_snippet() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set x 42\n");
    let text = hover(&mut lsp, &uri, 0, 1);
    assert!(text.contains("set varName"), "hover: {text:?}");
    assert!(text.contains("Read or write"), "hover: {text:?}");
    assert!(!text.contains("With one argument"), "hover: {text:?}");
}

#[test]
fn socket_hover_omits_snippet() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "socket localhost 80\n");
    let text = hover(&mut lsp, &uri, 0, 1);
    assert!(
        text.contains("socket ?options? host port"),
        "hover: {text:?}"
    );
    assert!(!text.contains("listening socket"), "hover: {text:?}");
}

#[test]
fn option_hover_omits_snippet() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "socket -myaddr 0.0.0.0 localhost 80\n");
    let text = hover(&mut lsp, &uri, 0, 8);
    assert!(text.contains("-myaddr"), "hover: {text:?}");
    assert!(!text.contains("multi-homed"), "hover: {text:?}");
}

// -- TestFormatStringHover -----------------------------------------------

#[test]
fn sprintf_format_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "format \"%s %d\" hello 123\n");
    let text = hover(&mut lsp, &uri, 0, 9);
    assert!(
        text.to_lowercase().contains("sprintf") || text.to_lowercase().contains("format string"),
        "hover: {text:?}"
    );
    assert!(text.contains("String"), "hover: {text:?}");
    assert!(text.to_lowercase().contains("integer"), "hover: {text:?}");
}

#[test]
fn sprintf_format_hover_flags() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "format \"%-10.5f\" 3.14\n");
    let text = hover(&mut lsp, &uri, 0, 9);
    assert!(
        text.to_lowercase().contains("float") || text.contains("Floating"),
        "hover: {text:?}"
    );
}

#[test]
fn clock_format_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "clock format $t -format \"%Y-%m-%d\"\n");
    let text = hover(&mut lsp, &uri, 0, 26);
    assert!(text.to_lowercase().contains("year"), "hover: {text:?}");
    assert!(text.to_lowercase().contains("month"), "hover: {text:?}");
}

#[test]
fn binary_format_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "binary format c2s $x $y\n");
    let text = hover(&mut lsp, &uri, 0, 14);
    assert!(text.contains("int8"), "hover: {text:?}");
    assert!(text.contains("int16"), "hover: {text:?}");
    assert!(text.contains("binary format"), "hover: {text:?}");
    assert!(text.contains("4 bytes"), "hover: {text:?}");
}

#[test]
fn regsub_subspec_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "regsub {pat} $str {\\1} result\n");
    let text = hover(&mut lsp, &uri, 0, 20);
    assert!(text.to_lowercase().contains("capture"), "hover: {text:?}");
}

#[test]
fn glob_pattern_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "string match {*.tcl} $f\n");
    let text = hover(&mut lsp, &uri, 0, 15);
    assert!(
        text.to_lowercase().contains("sequence") || text.contains("Matches"),
        "hover: {text:?}"
    );
}

#[test]
fn non_format_arg_no_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "puts \"hello\"\n");
    let result = lsp.hover(&uri, 0, 7);
    assert!(
        result.is_null() || !hover_text(&result).to_lowercase().contains("format"),
        "hover: {result:?}"
    );
}

#[test]
fn binary_scan_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "binary scan $data cu2 x y\n");
    let text = hover(&mut lsp, &uri, 0, 18);
    assert!(text.contains("uint8"), "hover: {text:?}");
    assert!(text.contains("binary scan"), "hover: {text:?}");
}

#[test]
fn binary_format_hover_with_absolute_seek() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "binary format c3S4i@20f 1 2 3 4 5 6 7 8 9\n");
    let text = hover(&mut lsp, &uri, 0, 15);
    assert!(text.contains("binary format"), "hover: {text:?}");
    assert!(text.contains("24 bytes"), "hover: {text:?}");
}

#[test]
fn regexp_pattern_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "regexp {^(\\d+)\\s+(\\w+)$} $line\n");
    let text = hover(&mut lsp, &uri, 0, 10);
    assert!(text.contains("Regex pattern"), "hover: {text:?}");
    assert!(text.to_lowercase().contains("anchor"), "hover: {text:?}");
    assert!(text.contains("Capture group"), "hover: {text:?}");
    assert!(text.contains("Digit"), "hover: {text:?}");
}

#[test]
fn regexp_pattern_hover_char_class() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "regexp {[a-z]+} $str\n");
    assert!(hover(&mut lsp, &uri, 0, 10).contains("Character class"));
}

#[test]
fn regexp_with_options_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "regexp -nocase -- {^hello} $str\n");
    let text = hover(&mut lsp, &uri, 0, 22);
    assert!(text.contains("Regex pattern"), "hover: {text:?}");
    assert!(text.to_lowercase().contains("anchor"), "hover: {text:?}");
}

#[test]
fn regsub_pattern_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "regsub {(\\w+)} $str {\\1} result\n");
    let text = hover(&mut lsp, &uri, 0, 10);
    assert!(text.contains("Regex pattern"), "hover: {text:?}");
    assert!(text.contains("Word character"), "hover: {text:?}");
}

#[test]
fn regexp_literal_no_metachar() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "regexp {hello} $str\n");
    let text = hover(&mut lsp, &uri, 0, 10);
    assert!(
        text.contains("Literal") || text.to_lowercase().contains("no metacharacters"),
        "hover: {text:?}"
    );
}

// -- TestAliasHover ------------------------------------------------------

#[test]
fn alias_hover_shows_target() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "interp alias {} = {} expr\n= {1 + 2}\n");
    let text = hover(&mut lsp, &uri, 1, 0);
    assert!(text.contains("Alias"), "hover: {text:?}");
    assert!(text.contains("expr"), "hover: {text:?}");
}

#[test]
fn alias_hover_with_prepended_args() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "interp alias {} myput {} puts stdout\nmyput hello\n");
    let text = hover(&mut lsp, &uri, 1, 0);
    assert!(text.contains("Alias"), "hover: {text:?}");
    assert!(text.contains("puts"), "hover: {text:?}");
    assert!(text.contains("stdout"), "hover: {text:?}");
}

#[test]
fn imported_command_resolves_to_qualified_spec() {
    // Peer of #776: a bare command imported into the global scope hovers as its
    // qualified spec — `test` after `namespace import ::tcltest::*`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace import ::tcltest::*\ntest t-1 {desc} -body { set x 1 } -result 1\n",
    );
    let text = hover(&mut lsp, &uri, 1, 2);
    assert!(
        text.contains("tcltest::test"),
        "bare imported `test` must hover as tcltest::test: {text:?}",
    );
}

// Issue #806 — hover on a scoped report::defstyle command.
#[test]
fn defstyle_scoped_command_hover() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "::report::defstyle st {} {\n    top set foo\n    columns\n}\n",
    );
    // `top` (line 1, char 5) surfaces the scoped-environment hover.
    let h = hover(&mut lsp, &uri, 1, 5);
    assert!(h.contains("report style"), "scoped env hover: {h}");
    // `columns` (line 2, char 7) surfaces its description.
    let c = hover(&mut lsp, &uri, 2, 7);
    assert!(c.contains("number of columns"), "columns hover: {c}");
}

/// idx 76 (differential-audit main audit wave, high severity, tomato
/// corpus): the finding's own headline hypothesis — the LSP guessing the
/// wrong class for the genuinely dynamic, `switch`-dispatched `[$obj
/// GetType]` call — is REFUTED (the LSP correctly abstains there). Tracing
/// it uncovered a distinct CONFIRMED gap on the exact same class: a
/// definite, single-target `my methodName` call had no hover at all,
/// unlike go-to-definition/find-references (already fixed by idx 52) or a
/// `link`-exposed bareword sibling call (idx 113) — reproduces in the real
/// corpus's own two-block `oo::class create` + separate `oo::define`
/// convention (all 9 of tomato's classes use it).
#[test]
fn my_dispatch_hover_resolves_when_class_extended_via_separate_oo_define() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "oo::class create geo::Plane {\n    constructor {args} {}\n}\noo::define geo::Plane {\n    method GetType {} { return Plane }\n    method WhichAmI {} { return [my GetType] }\n}\n",
    );
    // Line 5: `    method WhichAmI {} { return [my GetType] }` — cursor on
    // `GetType` inside `my GetType` (col 38).
    let h = hover(&mut lsp, &uri, 5, 38);
    assert!(h.contains("method"), "hover: {h}");
    assert!(h.contains("geo::Plane::GetType"), "hover: {h}");
}
