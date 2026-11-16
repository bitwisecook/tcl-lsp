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

//! Bodies that do not parse: the complete prefix runs before the fatal tail, and the error is raised on entry and names the malformed command.

use super::*;

#[test]
fn literal_catch_reports_incomplete_body_before_substitution_side_effects() {
    // tclsh 9.0.4: `1 0 {missing "}`
    assert_eq!(
        run(
            "proc p {} {set side 0; set code [catch {set x [incr side] \"} r]; list $code $side $r}; p"
        )
        .1,
        "1 0 {missing \"}",
    );
}

#[test]
fn literal_catch_executes_complete_prefix_before_later_parse_error() {
    // Tcl evaluates a script command at a time. A malformed later command is
    // caught only after every complete prefix command has already run.
    // tclsh 9.0.4: `1 1 {missing "}`
    assert_eq!(
        run(
            "proc p {} {set side 0; set code [catch {incr side; set x \"} r]; list $code $side $r}; p"
        )
        .1,
        "1 1 {missing \"}",
    );
}

#[test]
fn nested_try_executes_complete_phase_prefix_before_later_parse_error() {
    // The specialised try body reaches its handler only after the complete
    // prefix ran; the specialised handler likewise runs its complete prefix
    // before its malformed tail escapes to the enclosing catch.
    // tclsh 9.0.4: `0 1 {BODY:missing "}` / `1 1 {missing "}`
    assert_eq!(
        run(
            "proc p {} {set side 0; set code [catch {try {incr side; set x \"} on error m {set m BODY:$m}} result]; list $code $side $result}; p"
        )
        .1,
        "0 1 {BODY:missing \"}",
    );
    assert_eq!(
        run(
            "proc p {} {set side 0; set code [catch {try {error BOOM} on error m {incr side; set x \"}} result]; list $code $side $result}; p"
        )
        .1,
        "1 1 {missing \"}",
    );
}

/// A computed top-level `catch` reaches the ordinary builtin rather than the
/// procedure-only inline emitter. Its runtime compiler must preserve Tcl's
/// command-at-a-time parse boundary too.
#[test]
fn generic_top_level_catch_executes_prefix_before_fatal_tail() {
    // tclsh 9.0.4: `1 1 {missing "}`
    assert_eq!(
        run("set side 0; set c catch; set code [$c {incr side; set x \"} msg]; list $code $side $msg").1,
        "1 1 {missing \"}",
    );
}

/// The forced plain runtime compiler keeps every body command on ordinary
/// dispatch. The catch result must not depend on registry-specialised lowering.
#[test]
fn plain_dispatch_catch_executes_prefix_before_fatal_tail() {
    // tclsh 9.0.4: `1 1 {missing "}`
    assert_eq!(
        run_with_plain_runtime_compiler(
            "set side 0; set c catch; \
             set code [$c {incr side; set x \"} msg]; \
             list $code $side $msg"
        )
        .1,
        "1 1 {missing \"}",
    );
}

/// A computed top-level `try` uses the generic state machine. Its error
/// handler observes the side effect from the complete prefix and the exact Tcl
/// 9.0.4 parse message from the fatal tail.
#[test]
fn generic_top_level_try_executes_prefix_before_fatal_tail() {
    // tclsh 9.0.4: `{{missing "} 1} 1`
    assert_eq!(
        run("set side 0; set t try; \
             set result [$t {incr side; set x \"} on error m {list $m $side}]; \
             list $result $side")
        .1,
        "{{missing \"} 1} 1",
    );
}

/// Plain-dispatch runtime compilation and ordinary `try` dispatch consume the
/// same compiler-owned command-prefix plan as the optimised generic path.
#[test]
fn plain_dispatch_try_executes_prefix_before_fatal_tail() {
    // tclsh 9.0.4: `{{missing "} 1} 1`
    assert_eq!(
        run_with_plain_runtime_compiler(
            "set side 0; set t try; \
             set result [$t {incr side; set x \"} on error m {list $m $side}]; \
             list $result $side"
        )
        .1,
        "{{missing \"} 1} 1",
    );
}

/// Eval executes completed commands before a later fatal parse boundary.
#[test]
fn eval_executes_prefix_before_fatal_tail() {
    // tclsh 8.6.18 / 9.0.4: `1 1 {missing "}`
    assert_eq!(
        run(
            "set ::side 0; set code [catch {eval {incr ::side; set x \"}} msg]; \
             list $code $::side $msg"
        )
        .1,
        "1 1 {missing \"}",
    );
    // The same body reached through a variable, which never had the inlining
    // escape but did lose the prefix.
    // tclsh 8.6.18 / 9.0.4: `1 1 {missing "}`
    assert_eq!(
        run("set ::side 0; set b {incr ::side; set x \"}; \
             set code [catch {eval $b} msg]; list $code $::side $msg")
        .1,
        "1 1 {missing \"}",
    );
    // The shape that produced a *silent wrong answer* rather than a loud one:
    // lowered leniently, `set y "a"b` quietly meant `set y ab`, so the `eval`
    // returned `ab` with status 0 where C raises.
    // tclsh 8.6.18 / 9.0.4: `1 {extra characters after close-quote}`
    assert_eq!(
        run("set code [catch {eval {set y \"a\"b}} msg]; list $code $msg").1,
        "1 {extra characters after close-quote}",
    );
    // Positive control: a literal body that parses still runs and still
    // reports success, so neither assertion above can pass merely because
    // `eval` started failing or stopped running its body.
    // tclsh 8.6.18 / 9.0.4: `0 1 ok`
    assert_eq!(
        run(
            "set ::side 0; set code [catch {eval {incr ::side; set x ok}} msg]; \
             list $code $::side $msg"
        )
        .1,
        "0 1 ok",
    );
}

/// A procedure body is compiled when the procedure is **called**, so a body
/// that does not parse neither refuses the definition nor runs with the
/// lenient lowering's invented meaning (#1829).
///
/// The single asserted value pins all three facts at once: the procedure is
/// defined (`info procs` sees it), its clean prefix runs on entry (`::side`
/// reaches 1), and the parse error is raised after that prefix rather than at
/// `proc` time.
#[test]
fn a_procedure_body_that_does_not_parse_raises_on_entry_not_at_definition() {
    // tclsh 8.6.18 / 9.0.4: `p 1 1 {missing "}`
    assert_eq!(
        run("set ::side 0; proc p {} {incr ::side; set x \"}; \
             set d [info procs p]; set code [catch {p} msg]; \
             list $d $code $::side $msg")
        .1,
        "p 1 1 {missing \"}",
    );
    // A malformed body that is never called is not an error at all: the
    // definition stands and the script runs on. This is the half that used to
    // fail at `proc` time for a body reached through a variable.
    // tclsh 8.6.18 / 9.0.4: `q survived`
    assert_eq!(
        run("proc q {} {set x \"}; list [info procs q] survived").1,
        "q survived",
    );
    // The silent-wrong-answer shape: admitted from the enclosing module's
    // lenient lowering, `set y "a"b` quietly meant `set y ab`, so calling `p`
    // returned `ab` with status 0 where C raises.
    // tclsh 8.6.18 / 9.0.4: `1 {extra characters after close-quote}`
    assert_eq!(
        run(
            "proc p {} {set y \"a\"b; return $y}; set code [catch {p} msg]; \
             list $code $msg"
        )
        .1,
        "1 {extra characters after close-quote}",
    );
    // Positive control: a body that parses still runs on call and returns its
    // value, so the two assertions above cannot pass because procedure bodies
    // stopped executing.
    // tclsh 8.6.18 / 9.0.4: `0 1 ok`
    assert_eq!(
        run("set ::side 0; proc r {} {incr ::side; set x ok}; \
             set code [catch {r} msg]; list $code $::side $msg")
        .1,
        "0 1 ok",
    );
}

/// The deferred parse error becomes an error *before* the activation's
/// completion processing, not after it.
///
/// Raising it late would let the error-context frames, the procedure
/// boundary and leave traces all run against the prefix's `ok` completion,
/// so `-errorinfo` would carry no trace and a leave trace would observe
/// code 0 while the caller received code 1.
#[test]
fn a_deferred_parse_error_is_raised_before_the_activation_is_torn_down() {
    // tclsh 8.6.18 / 9.0.4: `1` — the trace is seeded with the message and
    // carries the `eval` body frame.
    assert_eq!(
        run("catch {eval {set a 1; set x \"}} m o; \
             list [string match {missing \"*} [dict get $o -errorinfo]] \
                  [string match {*(\"eval\" body line 1)*invoked from within*} \
                               [dict get $o -errorinfo]]")
        .1,
        "1 1",
    );
    // A leave trace on a procedure whose body has a malformed tail observes
    // the error, not the prefix's success.
    // tclsh 8.6.18 / 9.0.4: `1|missing "`
    assert_eq!(
        run("proc p {} {set a 1; set x \"}; set ::seen {}; \
             trace add execution p leave \
                 {apply {{c code r op} {set ::seen \"$code|$r\"}}}; \
             catch {p} m; set ::seen")
        .1,
        "1|missing \"",
    );
    // Positive control: an ordinary runtime error in the same position
    // already behaved this way, so the assertions above cannot pass merely
    // because leave traces or `-errorinfo` stopped working.
    // tclsh 8.6.18 / 9.0.4: `1|BOOM`
    assert_eq!(
        run("proc q {} {set a 1; error BOOM}; set ::seen2 {}; \
             trace add execution q leave \
                 {apply {{c code r op} {set ::seen2 \"$code|$r\"}}}; \
             catch {q} m; set ::seen2")
        .1,
        "1|BOOM",
    );
}

/// A hard *parse* error in a runtime control-flow body is mapped to an error
/// (the `eval_body` `Err(e)` arm in `cmd_control.rs`), uniformly across
/// `if`/`while`/`for`/`foreach`.
#[test]
fn control_runtime_body_parse_errors() {
    // `set "q"zzz` is a hard parse error (text after a close-quote).
    // tclsh: `extra characters after close-quote`
    let m = "extra characters after close-quote";
    assert_eq!(
        run("set c while; set i 0; $c {[incr i]<2} {set \"q\"zzz 1}").1,
        m,
    );
    assert_eq!(
        run("set c for; $c {set i 0} {$i<2} {incr i} {set \"q\"zzz 1}").1,
        m,
    );
    assert_eq!(run("set c foreach; $c x {1} {set \"q\"zzz 1}").1, m);
    assert_eq!(run("set c if; $c 1 {set \"q\"zzz 1}").1, m);
}

/// A hard *parse* error in a runtime control-flow condition is mapped to an
/// error (the `cond_bool` → `eval_expr` `Err(e)` arm in `cmd_control.rs`). The
/// VM surfaces the bare parse message (its `cond_bool` forwards `e.message`); C
/// Tcl additionally appends an `in expression "..."` context line, so we assert
/// the leading message only.
#[test]
fn control_runtime_condition_parse_error() {
    // tclsh leading line: `extra characters after close-quote`
    let (ok, msg, _) = run("set c while; $c {[set \"q\"zzz 1]} {set z 1}");
    assert!(!ok);
    assert!(
        msg.starts_with("extra characters after close-quote"),
        "got: {msg}",
    );
}

/// A deferred parse error names the command that failed to parse, the way an
/// ordinary runtime error in the same position does.
///
/// C compiles the failure through `Tcl_LogCommandInfo`, so `-errorinfo`
/// carries a `while executing` / `"<command>"` pair. `tclvm` raised the bare
/// message (#2172).
#[test]
fn a_deferred_parse_error_names_the_malformed_command() {
    // tclsh 8.6.18 / 9.0.4, identical.
    assert_eq!(
        run("set c catch; $c {set a 1; set x \"} m o; dict get $o -errorinfo").1,
        "missing \"\n    while executing\n\"set x \"\"",
    );
}

/// The quoted text stops at the character that opened the unterminated
/// construct, not at the end of the source.
///
/// C slices `source[commandStart ..= parsePtr->term]`, and for an unterminated
/// quote `ParseQuotedString` sets `term` to the opening `"` itself — so the
/// `abc\ndef` after it is not quoted. Reading the whole remainder instead (the
/// obvious implementation, and what #2172 first proposed) is wrong for every
/// row here but the first.
#[test]
fn the_named_command_stops_at_the_unterminated_delimiter() {
    // All five verified against tclsh 8.6.18 and 9.0.4, which agree.
    //
    // The body is passed braced where it is brace-balanced and quoted where it
    // is not — an unbalanced `{` cannot survive a braced wrapper, which is the
    // same reason C never sees that command as a word either.
    let cases = [
        // Construct opens last and nothing follows: whole remainder is right.
        (
            "set c catch; $c {set a 1; set x \"} m o; dict get $o -errorinfo",
            "missing \"",
            "set x \"",
        ),
        // Text after the opening quote is *not* quoted.
        (
            "set c catch; $c \"set a 1\\nset x \\\"abc\\ndef\" m o; dict get $o -errorinfo",
            "missing \"",
            "set x \"",
        ),
        // Same for a bracket, whose body would otherwise be included.
        (
            "set c catch; $c {set a 1; set x [foo bar} m o; dict get $o -errorinfo",
            "missing close-bracket",
            "set x [",
        ),
        // And a brace.
        (
            "set c catch; $c \"set a 1\\nset x {abc def\" m o; dict get $o -errorinfo",
            "missing close-brace",
            "set x {",
        ),
        // Reported in place rather than at an opener: the term is the
        // offending byte, and the frame stops there.
        (
            "set c catch; $c {set a 1; set x {a}b tail} m o; dict get $o -errorinfo",
            "extra characters after close-brace",
            "set x {a}b",
        ),
        // An earlier *complete* quoted word survives verbatim: the cut is at
        // the delimiter that failed, not the first one seen.
        (
            "set c catch; $c {set a 1; set x \"a\" y \"} m o; dict get $o -errorinfo",
            "missing \"",
            "set x \"a\" y \"",
        ),
    ];
    for (script, message, named) in cases {
        assert_eq!(
            run(script).1,
            format!("{message}\n    while executing\n\"{named}\""),
            "script: {script:?}",
        );
    }
}

/// A failure inside a `[…]` logs the inner construct C's parser stopped at:
/// tclsh 9.0.4 quotes `set x [list "`, the command up to the open quote.
#[test]
fn an_inner_construct_failure_logs_the_command_up_to_it() {
    assert_eq!(
        run("set c catch; $c {set a 1; set x [list \"oops]} m o; dict get $o -errorinfo").1,
        "missing \"\n    while executing\n\"set x [list \"\"",
    );
}
