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

//! Substitution of words: backslash escapes, array keys, command substitution inside words and assignments, and `subst`.

use super::*;

/// `concat` trims surrounding whitespace per element, but a trailing space
/// escaped by an odd run of backslashes (`\ `) is part of the element and kept
/// — C's `Tcl_ConcatObj` (lreplace.test ledit-1.25, concat-4.2).
#[test]
fn concat_keeps_backslash_escaped_trailing_space() {
    // Braced `{a\ }` is the literal `a`, `\`, ` `; the trailing space is escaped
    // by the backslash, so concat keeps it and adds its own join space.
    let (ok, result, _) = run("concat {a\\ } b");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a\\  b");

    let (ok, result, _) = run("concat {  a b  } {  c  }");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a b c"); // plain whitespace is trimmed
}

/// `TclParseBackslash` recognises only raw `\<LF>` as a continuation. A source
/// channel may normalise CRLF first, but bytes passed directly to the compiler
/// and VM retain raw CR/CRLF exactly like `Tcl_EvalObjEx`.
#[test]
fn raw_backslash_cr_is_data_not_a_line_continuation() {
    for (source, expected) in [
        ("set x \"a\\\rb\"; set x", "a\rb"),
        ("set x \"a\\\r\nb\"; set x", "a\r\nb"),
        ("set x {a\\\rb}; set x", "a\\\rb"),
        ("set x \"a\\\nb\"; set x", "a b"),
    ] {
        let (ok, result, _) = run(source);
        assert!(ok, "script should complete: {source:?}: {result:?}");
        assert_eq!(result, expected, "{source:?}");
    }

    // The related concat edge is already owned by tcl-cmd-core's
    // backslash-aware trim: the escaped trailing space remains data.
    let (ok, result, _) = run("concat \"\\\\ \"");
    assert!(ok, "concat should complete: {result:?}");
    assert_eq!(result, "\\ ");
}

/// A codegen hook (`concat`, `llength`, …) collapses a non-braced literal
/// argument's backslash escapes exactly like the generic per-word path — a
/// braced word stays verbatim. Regression for concat-1.4 / llength-2.3, where
/// the hook pushed the raw word and so saw `a\{` instead of `a{`.
#[test]
fn list_hooks_collapse_nonbraced_backslashes() {
    // Non-braced `a\{` collapses to `a{`; the braced `{b \{c d}` stays literal.
    let (ok, result, _) = run("concat a\\{ {b \\{c d} \\{d");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a{ b \\{c d {d");

    // llength of a quoted literal whose `\{` collapses to a bare `{` errors as
    // an unmatched open brace, rather than counting it as an escaped element.
    let (ok, result, _) = run("set rc [catch {llength \"a b c \\{\"} m]\nlist $rc $m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1 {unmatched open brace in list}");
}

/// An inline-compiled command substitution interpolates a leading-`$` word that
/// is more than a bare variable (`$i.x`, `$a$b`): the word is decomposed into
/// its `$var`/literal parts, not pushed raw. Regression for for-2.5 — `set m
/// [concat a "$i.x"]` had yielded the literal `$i.x`.
#[test]
fn inline_cmd_subst_interpolates_dollar_prefixed_word() {
    let (ok, result, _) = run("set i 5\nset m [concat a \"$i.x\"]\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a 5.x");

    let (ok, result, _) = run("set a 1\nset b 2\nset m [concat [list $a$b]]\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "12");
}

/// A brace-string array-element target keeps its key LITERAL: `set {a($x)} 5`
/// stores the element whose key is the literal string `$x` (Tcl braces suppress
/// substitution), so reading it back with the same braced key returns the value
/// — it does NOT substitute `$x`. Regression for set-1.25 (the unbraced
/// `set a($x) 5` still substitutes, covered by the array tests).
#[test]
fn braced_array_key_is_literal() {
    let (ok, result, _) = run(concat!(
        "catch {unset a}\n",
        "set {a($x)} 5\n",
        "set {a($x)}",
    ));
    assert!(
        ok,
        "braced key must not substitute $x (would error `can't read x`): {result}"
    );
    assert_eq!(result, "5");
}

/// A non-braced array-element key carrying backslash escapes is an ordinary
/// Tcl word: its escapes are decoded (`\w` → `w`, `\ ` → space) before the
/// element lookup, matching C Tcl. Regression for set-1.26.
#[test]
fn unbraced_array_key_decodes_backslashes() {
    let (ok, result, _) = run(concat!(
        "catch {unset be}\n",
        "set be(\\w\\w) 1\n",
        "set be(a\\ a) 1\n",
        "lsort [array names be]",
    ));
    assert!(ok, "should complete: {result}");
    assert_eq!(result, "{a a} ww");
}

/// An array-element read nested inside a command substitution that is itself
/// part of an array-key template substitutes its own key. Regression for
/// set-1.26's `set be([set be(a:$a)][set b\e($a)]) 1`.
#[test]
fn nested_array_read_in_key_template_substitutes() {
    let (ok, result, _) = run(concat!(
        "set a x\n",
        "set be(a:x) 5\n",
        "set be(x) 1\n",
        "set be([set be(a:$a)][set b\\e($a)]) 1\n",
        "lsort [array names be]",
    ));
    assert!(ok, "nested key read must substitute $a: {result}");
    assert_eq!(result, "51 a:x x");
}

/// A `\<newline>` line continuation inside an inline command substitution
/// assigned with `set` is a word separator: the inner command keeps all its
/// arguments. Regression for the inline-cmd-subst tokeniser dropping an arg
/// across a continuation (spurious `wrong # args`), which crashed tcltest's
/// `SubstArguments` and every test file using the `{-body … -result …}` dict
/// form (info / lrepeat / lseq).
#[test]
fn multiline_cmd_subst_in_assignment_keeps_all_args() {
    let (ok, result, _) = run("proc f {x} {set w [string range $x \\\n\t1 3]; return $w}\nf abcde");
    assert!(ok, "multi-line cmd subst must keep all args: {result}");
    assert_eq!(result, "bcd");
}

/// A hard parse error inside a compiled body (`set "i"xxx` — non-whitespace
/// after a close-quote) is deferred to a catchable runtime error carrying the
/// exact C Tcl message. Regression for set-1.3 / set-3.3.
#[test]
fn extra_chars_after_close_quote_is_catchable() {
    let (ok, result, _) = run(concat!(
        "set i 10\n",
        "catch {set \"i\"xxx} msg\n",
        "set msg",
    ));
    assert!(ok, "the catch must contain the parse error: {result}");
    assert_eq!(result, "extra characters after close-quote");
}

/// `subst` decodes one backslash escape at a time and handles the multi-byte
/// forms: a `\` before a multi-byte UTF-8 character (a fixed two-byte slice
/// would split the char boundary and panic), `\xHH` hex, and the
/// `\<newline><whitespace>` line continuation.
#[test]
fn subst_backslash_escapes_handle_multibyte_and_hex() {
    assert_eq!(run("subst {\\é}").1, "é");
    assert_eq!(run("subst {\\x41}").1, "A");
    // Tcl 9 caps `\x` at two hex digits: the rest is literal text.
    assert_eq!(run("subst {\\x41BC}").1, "ABC");
    // `\` then raw LF then leading whitespace collapses to a single space.
    // A whole module is a document, read as `tclsh file.tcl` reads it: the
    // channel's auto translation turns CRLF into LF first (tclsh 9.0.4 prints
    // `a b` for the CRLF form sourced from a file).
    assert_eq!(run("subst \"a\\\n   b\"").1, "a b");
    assert_eq!(run("subst \"a\\\r\n   b\"").1, "a b");
}

/// `set x [cmd …]` inline command-substitution assign:
/// the value is compiled inline (specialised opcode where available, else a
/// generic invoke) rather than pushed as raw `[…]` text for the runtime
/// `subst_word` fallback. Results verified against tclsh 9.0. The
/// `array names … $pat*` case pins the composite-arg fix (`$pat*` must be
/// substituted, not pushed literally) that unblocked real `tcltest.tcl`.
#[test]
fn set_inline_cmd_subst() {
    assert_eq!(run("set x [string length hello]; puts $x").2, "5\n");
    assert_eq!(
        run("proc f {s} { set n [string toupper $s]; return $n }; puts [f abc]").2,
        "ABC\n"
    );
    assert_eq!(run("set x [lindex {a b c} 1]; puts $x").2, "b\n");
    assert_eq!(run("set x [llength {a b c d}]; puts $x").2, "4\n");
    assert_eq!(run("set x [expr {2+3}]; puts $x").2, "5\n");
    assert_eq!(run("set d [dict get {a 1 b 2} b]; puts $d").2, "2\n");
    assert_eq!(
        run("proc h {} { set c [catch {error boom} m]; return $c:$m }; puts [h]").2,
        "1:boom\n"
    );
    // Composite arg (`$pat*`) must substitute then append the glob, not push
    // the literal `$pat*` — the bug that broke tcltest's MatchingOption.
    assert_eq!(
        run("array set A {xa 1 xb 2 yc 3}\nset m [lsort [array names A x*]]; puts $m").2,
        "xa xb\n"
    );
    assert_eq!(
        run("array set A {xa 1 xb 2 yc 3}\nset p x\nset m [lsort [array names A $p*]]; puts $m").2,
        "xa xb\n"
    );
    // Excluded forms still fold / behave as before.
    assert_eq!(run("set l [list a b c]; puts $l").2, "a b c\n");
    assert_eq!(
        run("set d [dict create k1 v1 k2 v2]; puts [dict get $d k2]").2,
        "v2\n"
    );
}

/// `{*}` expansion inside a command substitution in value position
/// (`set x [cmd a {*}$args b]`) compiles to `expandStart … expandStkTop N;
/// invokeExpanded` (result on the stack), matching tclsh 9.0.
/// Statement-position `{*}` already lowered correctly.
#[test]
fn set_inline_cmd_subst_expand() {
    assert_eq!(
        run("proc f {args} {set x [concat a {*}$args b]; return $x}; puts [f 1 2 3]").2,
        "a 1 2 3 b\n"
    );
    assert_eq!(
        run("set a {1 2}; set x [concat {*}$a 3]; puts $x").2,
        "1 2 3\n"
    );
    assert_eq!(
        run("proc h {args} {set x [string cat {*}$args]; return $x}; puts [h a b c]").2,
        "abc\n"
    );
    // `{*}{a b c}` expands a braced list verbatim.
    assert_eq!(
        run("set x [concat p {*}{a b c} q]; puts $x").2,
        "p a b c q\n"
    );
}

/// Regression tests for inline `set x [cmd]` command-substitution edge cases.
#[test]
fn inline_cmd_subst_review_fixes() {
    // `string is` generic fallback must keep the `is` subcommand — dropping it
    // would yield `string list …`.
    assert_eq!(run("set x [string is list {a b c}]; puts $x").2, "1\n");
    assert_eq!(run("set x [string is wideinteger 99]; puts $x").2, "1\n");
    // `-strict` char-class: STR_CLASS can't honour it (empty is a member), so it
    // defers to the command — empty string is a non-member under -strict.
    assert_eq!(
        run("proc p {} { string is alpha -strict \"\" }; puts [p]").2,
        "0\n"
    );
    assert_eq!(
        run("proc p {} { string is alpha -strict abc }; puts [p]").2,
        "1\n"
    );
    // `regexp` option words (`--`) are consumed; no stale stack value survives.
    assert_eq!(
        run("proc p {} { set r [regexp -- a a]; return $r }; puts [p]X").2,
        "1X\n"
    );
    // A multi-command substitution that also contains `{*}` runs as two commands
    // (the separator fallback precedes the expand path).
    assert_eq!(
        run("set x [set y 1; concat a {*}{b c}]; puts $x").2,
        "a b c\n"
    );
    // A registry-foldable command on either side of a separator remains two
    // commands. The substitution result is the final command's result.
    assert_eq!(run("set x [string cat a; string cat b]; puts $x").2, "b\n");
    // The expr fusion path uses the same single-command owner. `expr` has no
    // argument here and errors before the following command can run.
    assert_eq!(
        run("set code [catch {set x [expr; {1+2}]} msg]; puts $code").2,
        "1\n"
    );
    // A folded command's result is a finished Tcl value. Argument and
    // assignment paths must not run a second round of command/variable
    // substitution or strip braces from that result.
    assert_eq!(
        run("puts [string cat {[missing]}]; puts [string cat {{x}}]; set existing WRONG; puts [string cat {${existing}}]").2,
        "[missing]\n{x}\n${existing}\n"
    );
    assert_eq!(
        run("set x [string cat {[missing]}]; puts $x; set y [string cat {{x}}]; puts $y").2,
        "[missing]\n{x}\n"
    );
}
