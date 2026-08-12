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

//! Hover, end-to-end against the packaged server: open a document, wait for
//! analysis, ask `textDocument/hover` at a position, assert on the rendered
//! markdown. The iRules-dialect cases open the document with the `tcl-irule`
//! language id (and an `.irule` URI) so the server resolves the F5 command
//! pack.

use crate::common::helpers::{hover_text, starts, symbol_names};
use crate::common::{Lsp, unique_uri};
use serde_json::json;

/// Hover text at a position.
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
        text.to_lowercase()
            .contains("tcp client connection or listening server socket"),
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
    assert!(
        text.to_lowercase().contains("command prefix"),
        "hover: {text:?}"
    );
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

// -- Issue #1337: declaration-site proc hover ---------------------------

/// Declaration hover must be independent of whether the proc is referenced
/// elsewhere in the document.  Exercise the live server rather than the core
/// provider so this also covers the configured feature gate and the server's
/// settled analysis path.
#[test]
fn proc_declaration_hover_is_independent_of_references_1337() {
    let cases = [
        (
            "undocumented and unreferenced",
            "proc greet {name} { return $name }\n",
            0,
            false,
        ),
        (
            "documented and unreferenced",
            "# Return the supplied name.\nproc greet {name} { return $name }\n",
            1,
            true,
        ),
        (
            "undocumented and referenced",
            "proc greet {name} { return $name }\ngreet Ada\n",
            0,
            false,
        ),
        (
            "documented and referenced",
            "# Return the supplied name.\nproc greet {name} { return $name }\ngreet Ada\n",
            1,
            true,
        ),
    ];

    for (case, source, declaration_line, documented) in cases {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        lsp.open_ready(&uri, source);

        // TP/FN: the declaration is a hover target even without any call.
        let text = hover(&mut lsp, &uri, declaration_line, 6);
        assert!(text.contains("proc ::greet {name}"), "{case}: {text:?}");
        assert_eq!(
            text.contains("Return the supplied name."),
            documented,
            "{case}: {text:?}"
        );

        // Navigation and outline must agree that the same declaration exists.
        assert_eq!(
            starts(&lsp.definition(&uri, declaration_line, 6)),
            [(i64::from(declaration_line), 5)].into(),
            "{case}: definition did not return the declaration",
        );
        assert!(
            symbol_names(&lsp.document_symbols(&uri)).contains("greet"),
            "{case}: document symbols did not include the declaration",
        );
    }
}

#[test]
fn proc_declaration_hover_respects_the_settled_hover_toggle_1337() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "proc greet {name} { return $name }\n");

    assert!(
        hover(&mut lsp, &uri, 0, 6).contains("proc ::greet {name}"),
        "enabled baseline"
    );

    // TN/FP: an explicitly disabled hover feature returns no hover, while the
    // unrelated definition and outline providers remain available.
    lsp.apply_configuration_settle(json!({ "features": { "hover": false } }), &uri, |cfg| {
        cfg["features"]["hover"] == false
    });
    assert!(hover_text(&lsp.hover(&uri, 0, 6)).is_empty());
    assert_eq!(starts(&lsp.definition(&uri, 0, 6)), [(0, 5)].into());
    assert!(symbol_names(&lsp.document_symbols(&uri)).contains("greet"));

    // A settled re-enable restores the declaration hover, proving that a null
    // at this location is a feature-toggle result, not a cached proc export.
    lsp.apply_configuration_settle(json!({ "features": { "hover": true } }), &uri, |cfg| {
        cfg["features"]["hover"] == true
    });
    assert!(
        hover(&mut lsp, &uri, 0, 6).contains("proc ::greet {name}"),
        "hover did not recover after the settled re-enable"
    );
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
    assert!(
        text.contains("Read the value of a variable"),
        "hover: {text:?}"
    );
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

// -- Issue #1018: cross-document and autoload hover ----------------------
//
// Go-to-definition, find-references, and the unknown-command diagnostic all
// resolve a command whose `proc` lives in a sibling file. Hover was the one
// provider that gave up at the file boundary and showed nothing — at the exact
// call site the other three resolved. These are the first cross-file cases in
// this suite.

/// The plain `source` control: `main.tcl` sources `lib.tcl`, which declares
/// `greetPerson`. Hovering the call in `main.tcl` must render the real
/// signature, exactly as hovering the declaration in `lib.tcl` does.
#[test]
fn hover_resolves_a_proc_defined_in_a_sourced_sibling_1018() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    let lib_name = lib.rsplit('/').next().expect("lib basename").to_owned();
    let main = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "proc greetPerson {name {greeting hello}} {\n    return \"$greeting, $name\"\n}\n",
    );
    lsp.open_ready(&main, &format!("source {lib_name}\ngreetPerson Alice\n"));

    let call = hover(&mut lsp, &main, 1, 4);
    assert!(
        call.contains("greetPerson"),
        "cross-file call hover must name the proc: {call:?}",
    );
    assert!(
        call.contains("name") && call.contains("greeting"),
        "…and render its real parameter list: {call:?}",
    );

    // The same symbol hovered at its own declaration renders the same body —
    // one renderer, one input, so the two views cannot drift apart.
    let decl = hover(&mut lsp, &lib, 0, 8);
    assert_eq!(call, decl, "call-site and declaration hover must agree");
}

/// TN: a command nothing in the workspace defines still hovers nothing. The
/// fallback resolves real symbols; it does not invent one for a typo.
#[test]
fn hover_stays_empty_for_a_command_no_document_defines_1018() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    let lib_name = lib.rsplit('/').next().expect("lib basename").to_owned();
    let main = unique_uri("tcl");
    lsp.open_ready(&lib, "proc greetPerson {name} { return $name }\n");
    lsp.open_ready(&main, &format!("source {lib_name}\ngreetPersonn Alice\n"));
    assert!(
        hover_text(&lsp.hover(&main, 1, 4)).is_empty(),
        "a command no document defines must hover nothing",
    );
}

/// FP guard: the fallback is gated on the cursor sitting on a *command head*.
/// An argument word that happens to match a sibling document's proc name must
/// not pop that proc's signature up.
#[test]
fn hover_does_not_resolve_an_argument_word_across_documents_1018() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    let lib_name = lib.rsplit('/').next().expect("lib basename").to_owned();
    let main = unique_uri("tcl");
    lsp.open_ready(&lib, "proc zqwidget {name} { return $name }\n");
    // `zqwidget` here is `puts`'s argument, not a command head.
    lsp.open_ready(&main, &format!("source {lib_name}\nputs zqwidget\n"));
    let text = hover_text(&lsp.hover(&main, 1, 7));
    assert!(
        !text.contains("proc ::zqwidget"),
        "an argument word must not resolve to a sibling proc: {text:?}",
    );
}

/// Per-call counter for the autoload-hover fixture dir.
static AUTOLOAD_HOVER_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The issue's own repro (tcllib's `modules/math/math.tcl` shape): a package
/// whose commands are declared in a sibling file reached through
/// `auto_path` / `tclIndex`, with no `source` edge to follow.
///
/// Oracle (`tclsh8.6` and `tclsh9.0`, identical): the script runs and prints
/// `frob 1 2 3 4` / `42`, so `::math::zzqfrobnicate` and `::math::wibblenum`
/// in `misc.tcl` are the real, unambiguous targets. Go-to-definition already
/// jumped there; hover showed nothing.
#[test]
fn hover_resolves_an_autoloaded_library_proc_1018() {
    use std::sync::atomic::Ordering;

    let libdir = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-hover-autoload-{}-{}",
        std::process::id(),
        AUTOLOAD_HOVER_N.fetch_add(1, Ordering::Relaxed)
    ));
    let pkgdir = libdir.join("mathlib");
    std::fs::create_dir_all(&pkgdir).expect("mk mathlib dir");
    std::fs::write(
        pkgdir.join("misc.tcl"),
        "proc ::math::zzqfrobnicate {val args} {\n    return \"frob $val $args\"\n}\n\
         proc ::math::wibblenum {n} {\n    return [expr {$n * 2}]\n}\n",
    )
    .expect("write misc.tcl");
    std::fs::write(
        pkgdir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
         set auto_index(::math::zzqfrobnicate) [list source [file join $dir misc.tcl]]\n\
         set auto_index(::math::wibblenum) [list source [file join $dir misc.tcl]]\n",
    )
    .expect("write tclIndex");

    let mut lsp = Lsp::with_config(serde_json::json!({
        "libraryPaths": [ libdir.to_string_lossy() ],
    }));
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "package require math\nmath::zzqfrobnicate 1 2 3 4\n");

    // The package database is built asynchronously at startup; poll until the
    // autoload tier can answer, then assert on what it rendered.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut text = hover(&mut lsp, &uri, 1, 8);
    while text.is_empty() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(100));
        text = hover(&mut lsp, &uri, 1, 8);
    }
    assert!(
        text.contains("math::zzqfrobnicate"),
        "autoload hover must name the resolved proc: {text:?}",
    );
    assert!(
        text.contains("val") && text.contains("args"),
        "…and render its real signature: {text:?}",
    );

    // TN on the same fixture: a name the `tclIndex` does not declare still
    // hovers nothing, so the tier resolves rather than blanket-suppressing.
    let typo = unique_uri("tcl");
    lsp.open_ready(&typo, "package require math\nmath::zzqfrobnicat 1\n");
    assert!(
        hover_text(&lsp.hover(&typo, 1, 8)).is_empty(),
        "a command no index declares must hover nothing",
    );

    let _ = std::fs::remove_dir_all(&libdir);
}

/// Same-file hover is unchanged — the fallback only runs when the
/// in-document provider has already declined.
#[test]
fn hover_same_file_behaviour_is_unchanged_1018() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc greetPerson {name} { return $name }\ngreetPerson Alice\n",
    );
    let text = hover(&mut lsp, &uri, 1, 4);
    assert!(text.contains("greetPerson"), "same-file hover: {text:?}");
}

// -- expr math functions (issue #974 defect 1) ---------------------------

/// A bare `sin(…)` inside `expr` renders the same registry documentation the
/// namespace-qualified `::tcl::mathfunc::sin` spelling already did — before
/// this it drew nothing at any column of the function word.
#[test]
fn bare_mathfunc_call_in_expr_hovers_974() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set a [expr {sin(1.0)}]\n");
    let text = hover(&mut lsp, &uri, 0, 14);
    assert!(text.contains("sin"), "hover: {text:?}");
    assert!(text.contains("math function"), "hover: {text:?}");
    assert!(
        text.contains("::tcl::mathfunc::sin"),
        "names its dispatch target: {text:?}"
    );
}

/// A user `proc ::tcl::mathfunc::NAME` override wins over the built-in — it is
/// a real command in the table C Tcl dispatches through.
#[test]
fn overridden_mathfunc_hovers_as_the_proc_974() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "namespace eval tcl::mathfunc {\n    proc li {list index args} { ::lindex $list $index }\n}\nproc caller {} {\n    if {li({0 1},1)} { return yes }\n}\n",
    );
    let text = hover(&mut lsp, &uri, 4, 9);
    assert!(
        text.contains("::tcl::mathfunc::li"),
        "the override: {text:?}"
    );
}

/// TN: the same word outside an expression draws no math-function hover.
#[test]
fn mathfunc_hover_does_not_fire_outside_expr_974() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, "set sin 1\nputs $sin\n");
    let text = hover(&mut lsp, &uri, 1, 7);
    assert!(text.to_lowercase().contains("variable"), "hover: {text:?}");
    assert!(!text.contains("math function"), "hover: {text:?}");
}

// -- inert `$var` text (issue #923 idx 24) -------------------------------

/// A `$var`-shaped substring inside a comment or a brace-quoted data word is
/// emitted verbatim by Tcl, so it must not hover as a variable.
#[test]
fn inert_dollar_ref_does_not_hover_923_idx24() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc holder {} {\n    set level 42\n    # so $level ...\n    set t {plain $level here}\n    return $t\n}\n",
    );
    assert!(
        hover_text(&lsp.hover(&uri, 2, 11)).is_empty(),
        "comment occurrence must not hover"
    );
    assert!(
        hover_text(&lsp.hover(&uri, 3, 19)).is_empty(),
        "data-brace occurrence must not hover"
    );
    // FP guard: a live read still hovers.
    let text = hover(&mut lsp, &uri, 4, 12);
    assert!(text.to_lowercase().contains("variable"), "hover: {text:?}");
}

// -- parameter-list data words (issue #923 idx 104) ----------------------

/// A parameter's default-value literal is data, not a command reference.
#[test]
fn parameter_default_literal_does_not_hover_923_idx104() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc destroy {w} { puts $w }\nproc rfg {grab focus {destroy destroy}} {\n    destroy $grab\n}\n",
    );
    assert!(
        hover_text(&lsp.hover(&uri, 1, 30)).is_empty(),
        "the default-value literal must not hover as a command"
    );
    // FP guard: the real call in the body still resolves.
    let text = hover(&mut lsp, &uri, 2, 5);
    assert!(text.contains("destroy"), "hover: {text:?}");
}

/// Issue #923 differential-audit findings idx 3 / idx 4 — tcllib's
/// `textutil::adjust` submodule. `package require textutil::adjust` followed
/// by `namespace import textutil::adjust::*` makes bare `adjust` / `indent`
/// callable, and they are the *three*-segment commands
/// `::textutil::adjust::adjust` / `::textutil::adjust::indent` — distinct
/// from the two-segment `textutil::adjust` / `textutil::indent` flattened
/// aliases the umbrella `textutil` package creates.
///
/// Oracle (tclsh 8.6.16 and 9.0.4, tcllib 2.0 on `auto_path`):
/// `namespace origin adjust` → `::textutil::adjust::adjust` and
/// `namespace origin indent` → `::textutil::adjust::indent`.
///
/// Coverage was split between registry-table unit tests (naming and gating)
/// and a generic `resolve_imported_command` e2e test using `tcltest`; nothing
/// tied the two together for this pair, so a regression in either layer's
/// interaction would have gone unseen. The idiom is real corpus code
/// (`argparse.tcl` buries both inside an `if`, as here).
#[test]
fn a_wildcard_imported_textutil_submodule_command_hovers_qualified_923_idx3_idx4() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        concat!(
            "proc buildHelp {args} {\n",
            "    if {[llength $args]} {\n",
            "        package require textutil::adjust\n",
            "        namespace import textutil::adjust::*\n",
            "        set wrapped [adjust \"a long help string\" -length 20]\n",
            "        set final [indent $wrapped \"  \"]\n",
            "        puts $final\n",
            "    }\n",
            "}\n",
        ),
    );
    // Line 4, `adjust` starts at column 21.
    let adjust = hover(&mut lsp, &uri, 4, 22);
    assert!(
        adjust.contains("textutil::adjust::adjust"),
        "bare imported `adjust` must hover as the three-segment submodule \
         command, not the umbrella alias: {adjust:?}",
    );
    // Line 5, `indent` starts at column 19.
    let indent = hover(&mut lsp, &uri, 5, 20);
    assert!(
        indent.contains("textutil::adjust::indent"),
        "bare imported `indent` must hover as `textutil::adjust::indent`: \
         {indent:?}",
    );
}

/// Issue #923 differential-audit finding idx 102, its *secondary* claim —
/// hover on the `{file}` **parameter declaration** token must report a
/// variable, never the built-in `file` command's documentation.
///
/// The primary claim (the `$file` read inside the `[list source [file join
/// …]]` body resolving at all) is pinned by
/// `references_reach_a_parameter_read_inside_a_list_built_namespace_body`;
/// nothing guarded this half, which is fixed only as a by-product of the
/// read being classified as a variable reference before any registry lookup
/// happens — precisely the kind of thing a later refactor reintroduces.
///
/// Oracle (tclsh 8.6.16 and 9.0.4): the parameter genuinely drives which file
/// is sourced, so it is a variable at both ends.
#[test]
fn hover_on_a_parameter_named_after_a_builtin_is_a_variable_923_idx102() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(
        &uri,
        "proc ::tk::SourceLibFile {file} {\n    namespace eval :: [list source [file join $::tk_library $file.tcl]]\n}\n",
    );
    // Line 0, the `file` of the parameter list `{file}` — column 25.
    let decl = hover(&mut lsp, &uri, 0, 26);
    assert!(
        decl.contains("Variable"),
        "the `{{file}}` parameter declaration must hover as a variable: {decl:?}",
    );
    assert!(
        !decl.contains("file dirname") && !decl.contains("file exists"),
        "it must not be misattributed to the builtin `file` command: {decl:?}",
    );
    // And the read inside the list-built body agrees.
    let read = hover(&mut lsp, &uri, 1, 62);
    assert!(
        read.contains("Variable"),
        "the `$file` read must hover as the same variable: {read:?}",
    );
}
