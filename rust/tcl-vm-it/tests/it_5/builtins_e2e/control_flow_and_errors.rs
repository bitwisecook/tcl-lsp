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

//! `foreach`, `switch`, `expr`, `catch` and `uplevel`, and the error stack and `errorInfo` a caught error reports.

use super::*;

#[test]
fn foreach_single() {
    out_eq("foreach x {a b c} { puts $x }\n", "a\nb\nc\n");
}

#[test]
fn foreach_multi_var() {
    out_eq("foreach {k v} {a 1 b 2} { puts \"$k=$v\" }\n", "a=1\nb=2\n");
}

#[test]
fn foreach_two_lists() {
    out_eq("foreach x {1 2} y {3 4} { puts \"$x$y\" }\n", "13\n24\n");
}

#[test]
fn foreach_uneven_pads_empty() {
    out_eq("foreach {a b} {1 2 3} { puts \"$a-$b\" }\n", "1-2\n3-\n");
}

#[test]
fn foreach_accumulate() {
    out_eq(
        "set s 0\nforeach n {1 2 3 4} { incr s $n }\nputs $s\n",
        "10\n",
    );
}

#[test]
fn foreach_empty_list() {
    out_eq("foreach x {} { puts no }\nputs done\n", "done\n");
}

#[test]
fn unset_command() {
    out_eq("set x 1\nunset x\nputs [info exists x]\n", "0\n");
    out_eq("unset -nocomplain nope\nputs ok\n", "ok\n");
    out_eq("set a(x) 1\nunset a(x)\nputs [info exists a(x)]\n", "0\n");
}

#[test]
fn expr_math_functions() {
    out_eq("puts [expr {abs(-5)}]\n", "5\n");
    out_eq("puts [expr {max(3,7,2)}]\n", "7\n");
    out_eq("puts [expr {min(3,7,2)}]\n", "2\n");
    out_eq("puts [expr {int(3.9)}]\n", "3\n");
    out_eq("puts [expr {sqrt(9.0)}]\n", "3.0\n");
    out_eq("puts [expr {pow(2,10)}]\n", "1024.0\n");
}

#[test]
fn switch_shared_core() {
    // Option parsing + pattern selection route through the shared core. Exact
    // switches are codegen-inlined (the `JUMP_TABLE` path), so the core is
    // exercised via `-glob`/`-regexp` and the error/option cases. Pinned vs tclsh.
    out_eq(
        "puts [switch -glob ab { a {expr 1} ab {expr 2} default {expr 9} }]\n",
        "2\n",
    );
    out_eq("puts [switch -nocase -glob ABC { ab* {expr 7} }]\n", "7\n");
    // `-` fall-through.
    out_eq(
        "puts [switch -glob x { a - b {expr 3} x {expr 4} }]\n",
        "4\n",
    );
    // `-regexp` matches through the engine.
    out_eq("puts [switch -regexp aXb { {a(.)b} {expr 11} }]\n", "11\n");
    // TIP #75 -matchvar/-indexvar.
    out_eq(
        "switch -regexp -matchvar mv -indexvar iv aXb { {a(.)b} {} }\n\
         puts \"$mv|$iv\"\n",
        "aXb X|{0 2} {1 1}\n",
    );
    // `default` is only a wildcard as the *last* pattern; elsewhere it is a
    // literal pattern (so value "q" matches the `q` arm, returning 6).
    out_eq(
        "puts [switch -glob q { default {expr 5} q {expr 6} }]\n",
        "6\n",
    );
    // No match → empty string.
    out_eq("puts <[switch -glob zz { a {expr 1} }]>\n", "<>\n");
    // (Option-error cases route through the codegen's inline switch, not the
    // runtime `cmd_switch`, so they are pinned via the tree-walking runtime,
    // which always calls the shared core — see `runtime/rust` switch tests.)
}

#[test]
fn expr_ternary_string_branches() {
    // `tryCvtToNumeric` must pass non-numeric results through, not error.
    out_eq("puts [expr {1 ? \"a\" : \"b\"}]\n", "a\n");
    out_eq("puts [expr {0 ? \"a\" : \"b\"}]\n", "b\n");
    out_eq(
        "set x 5\nputs [expr {$x > 3 ? \"big\" : \"small\"}]\n",
        "big\n",
    );
    out_eq("puts [expr {1 ? 2 : 3}]\n", "2\n");
}

#[test]
fn expr_in_operator() {
    out_eq("puts [expr {3 in {1 2 3}}]\n", "1\n");
    out_eq("puts [expr {9 in {1 2 3}}]\n", "0\n");
    out_eq("puts [expr {5 ni {1 2 3}}]\n", "1\n");
}

#[test]
fn switch_glob_and_default() {
    out_eq(
        "switch -glob aa { a* {puts hit} default {puts no} }\n",
        "hit\n",
    );
    out_eq("switch zz { a {puts a} default {puts def} }\n", "def\n");
    out_eq("switch b { a {puts A} b {puts B} }\n", "B\n");
}

#[test]
fn catch_body_suppressed_until_eval() {
    out_eq(
        "set rc [catch { error boom } msg]\nputs \"$rc $msg\"\n",
        "1 boom\n",
    );
}

#[test]
fn uplevel_command() {
    out_eq(
        "proc setit {} { uplevel 1 { set x 42 } }\nproc caller {} { setit; return $x }\nputs [caller]\n",
        "42\n",
    );
    out_eq(
        "proc run {script} { uplevel 1 $script }\nset y 0\nrun {set y 9}\nputs $y\n",
        "9\n",
    );
    out_eq(
        "set g 0\nproc p {} { uplevel #0 { set g 7 } }\np\nputs $g\n",
        "7\n",
    );
}

/// TIP 348 metadata is accumulated once at the shared command-unwind seam and
/// exposed identically by catch and `info errorstack`. A compiled `error`
/// is C's `returnImm` (tclsh 8.6.18 and 9.0.4).
#[test]
fn catch_reports_structured_error_stack() {
    out_eq(
        "catch {error boom} m o\n\
         puts [list $m [dict get $o -errorstack] [info errorstack]]\n\
         proc f {x} {g $x$x}\n\
         proc g {x} {error G:$x}\n\
         catch {f 12} m o\n\
         puts [list $m [dict get $o -errorstack] [info errorstack]]\n",
        "boom {INNER {returnImm boom {}}} {INNER {returnImm boom {}}}\n\
         G:1212 {INNER {returnImm G:1212 {}} CALL {g 1212} CALL {f 12}} {INNER {returnImm G:1212 {}} CALL {g 1212} CALL {f 12}}\n",
    );
}

/// A specialised catch body still logs the failing command, not the enclosing
/// catch instruction. A body the compiled `catch` evaluates as a word logs the
/// `catch` command after it too, as the error leaves the script (tclsh 8.5.19
/// through 9.1.0).
#[test]
fn inline_and_word_catch_bodies_report_the_c_errorinfo() {
    out_eq(
        "proc inline {} {catch {error boom} m o; dict get $o -errorinfo}\n\
         proc generic {} {set body {error boom}; catch $body m o; dict get $o -errorinfo}\n\
         puts [list [inline] [generic]]\n",
        "{boom\n    while executing\n\"error boom\"} {boom\n    while executing\n\"error boom\"\n    invoked from within\n\"catch $body m o\"}\n",
    );
}

/// Proc-like adapters carry the exact invocation words into the one procedure
/// boundary that emits TIP 348 `CALL` entries.
#[test]
fn lambda_and_method_error_stacks_preserve_invocation_identity() {
    out_eq(
        "catch {apply {{} {error APPLY}}} m o\n\
         puts [lindex [dict get $o -errorstack] end]\n\
         oo::class create C {method fail {arg} {error METHOD}}\n\
         C create named\n\
         catch {named fail value} m o\n\
         puts [lindex [dict get $o -errorstack] end]\n\
         catch {::named fail rooted} m o\n\
         puts [lindex [dict get $o -errorstack] end]\n",
        "apply {{} {error APPLY}}\nnamed fail value\n::named fail rooted\n",
    );
}

/// An explicit errorInfo starts a real line-one error episode even though the
/// command itself is already logged.
#[test]
fn explicit_return_errorinfo_reports_line_one() {
    out_eq(
        "catch {return -level 0 -code error -errorinfo EXPLICIT boom} m o\n\
         puts [list [dict get $o -errorline] [dict get $o -errorinfo]]\n",
        "1 EXPLICIT\n",
    );
}

/// Tcl retains the prior TIP 348 stack when an explicit `-errorinfo` suppresses
/// logging a new inner command during a lazily reset error episode.
#[test]
fn explicit_return_errorinfo_retains_the_prior_error_stack() {
    out_eq(
        "catch {error first} m o\n\
         set prior [dict get $o -errorstack]\n\
         catch {return -level 0 -code error -errorinfo I second} m o\n\
         puts [list [expr {$prior ne {}}] \
                    [expr {[dict get $o -errorstack] eq $prior}] \
                    [expr {[info errorstack] eq $prior}]]\n",
        "1 1 1\n",
    );
}

#[test]
fn error_stack_call_preserves_qualified_invocation() {
    out_eq(
        "namespace eval n {proc p {x} {error E:$x}}\n\
         catch {::n::p z} m o\n\
         puts [dict get $o -errorstack]\n",
        "INNER {returnImm E:z {}} CALL {::n::p z}\n",
    );
}

/// Trace callback failures retain the callback's stack; a control completion
/// converted to an error begins a distinct trace episode.
#[test]
fn variable_trace_errors_retain_error_stack() {
    out_eq(
        "proc boom {n1 n2 op} {error TRACE}\n\
         trace add variable x write boom\n\
         catch {set x 1} m o\n\
         puts [list $m [dict get $o -errorstack]]\n\
         trace remove variable x write boom\n\
         unset -nocomplain x\n\
         proc storeStk {n1 n2 op} {return -code break}\n\
         trace add variable x write storeStk\n\
         catch {set x 1} m o\n\
         puts [list $m [dict get $o -errorstack]]\n",
        "{can't set \"x\": TRACE} {INNER {returnImm TRACE {}} CALL {boom x {} write}}\n\
         {can't set \"x\": } {INNER storeStk}\n",
    );
}

/// A valid explicit stack survives both immediate and pending return options.
#[test]
fn explicit_return_error_stack_is_preserved() {
    out_eq(
        "catch {return -level 0 -code error -errorstack {INNER custom} explicit} m o\n\
         puts [list $m [dict get $o -errorstack] [info errorstack]]\n\
         catch {return -code error -errorstack {CALL a CALL b} pending} m o\n\
         puts [list $m [dict get $o -code] [dict get $o -level] \
                    [dict get $o -errorstack] [info errorstack]]\n",
        "explicit {INNER custom} {INNER custom}\n\
         pending 1 1 {CALL a CALL b} {CALL a CALL b}\n",
    );
}

#[test]
fn explicit_return_error_stack_obeys_proc_boundaries() {
    out_eq(
        "proc direct {} {return -code error -errorstack {} boom}\n\
         catch {direct} m o\n\
         puts [dict get $o -errorstack]\n\
         proc inner {} {return -code error -errorstack {INNER custom} boom}\n\
         proc outer {} {inner}\n\
         catch {outer} m o\n\
         puts [dict get $o -errorstack]\n\
         proc metadata {} {return -code error -errorcode CUSTOM -errorinfo EXPLICIT boom}\n\
         catch {metadata} m o\n\
         puts [list [dict get $o -errorcode] \
                    [string match {EXPLICIT*} [dict get $o -errorinfo]]]\n",
        "\nINNER custom CALL outer\nCUSTOM 1\n",
    );
}

/// `try` consumes the same rich completion snapshot as catch.
#[test]
fn try_handler_receives_error_metadata() {
    out_eq(
        "set t try\n\
         puts [$t {error boom} on error {m o} {list [dict exists $o -errorinfo] [dict get $o -errorstack]}]\n",
        "1 {INNER {returnImm boom {}}}\n",
    );
}
