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

//! Loop jump commands and their argument checks, `time`, and `tailcall` with its deferred-completion matrices.

use super::*;

/// A loop whose body redefines `break`/`continue` runs through the runtime
/// builtin (which dispatches them) instead of the inline JUMP fast-path, so the
/// redefinition is honoured; the inline fast-path would ignore the
/// redefinition and loop forever. Matches tclsh's proc-7.3 (Bug 729692).
#[test]
fn loop_body_redefining_continue_is_honoured() {
    let (ok, result, _) = run(concat!(
        "set i 0\n",
        "while {1} {\n",
        "  if {[incr i] > 3} { proc continue {} {return -code break} }\n",
        "  continue\n",
        "}\n",
        "set i",
    ));
    assert!(ok, "loop should terminate (no hang): {result}");
    assert_eq!(result, "4");
}

/// `while` / `for` with the wrong argument count raise `wrong # args` rather
/// than lowering a truncated loop and running an argument as a command.
/// Regression for while-old-4.3 / for-old-1.7.
#[test]
fn while_for_reject_wrong_arg_count() {
    let (ok, result, _) = run("catch {while 1 2 3} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "wrong # args: should be \"while test command\"");

    let (ok, result, _) = run("catch {for 1 2 3 4 5} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "wrong # args: should be \"for start test next command\""
    );
}

/// `break` / `continue` with any argument raise `wrong # args` — even inside a
/// loop, where the bare forms compile to a jump. The compiler only takes the
/// jump fast path when there are no arguments, so `break foo` reaches the
/// builtin. Regression for for-2.1 / for-3.1. The bare forms still control the
/// loop.
#[test]
fn break_continue_reject_arguments() {
    let (ok, result, _) =
        run("set r {}\nforeach x {1 2 3} {catch {break foo} m; lappend r $m; break}\nset r");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "{wrong # args: should be \"break\"}");

    let (ok, result, _) = run("catch {continue foo} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "wrong # args: should be \"continue\"");

    // The bare forms still break/continue the loop.
    let (ok, result, _) =
        run("set r {}\nforeach x {1 2 3 4} {if {$x==3} break; lappend r $x}\nset r");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1 2");
}

/// `tailcall` — the proc finishes and the named command runs in its place
/// (in the caller's activation). A tail-recursive loop must not grow the
/// activation stack or hit the recursion limit (the whole point of `tailcall`).
#[test]
fn tailcall_basic_and_deep() {
    // Tail call replaces the proc's continuation.
    assert_eq!(run("proc f {} { tailcall set g 9 }; f; puts $g").2, "9\n");
    assert_eq!(
        run("proc a {} { return A }; proc b {} { tailcall a }; puts [b]").2,
        "A\n"
    );
    // Tail-recursive factorial.
    assert_eq!(
        run("proc fact {n acc} { if {$n<=1} { return $acc }; tailcall fact [expr {$n-1}] [expr {$n*$acc}] }; puts [fact 5 1]").2,
        "120\n"
    );
    // Deep tail recursion: far past the recursion limit — must not overflow.
    assert_eq!(
        run("proc cd {n} { if {$n<=0} { return done }; tailcall cd [expr {$n-1}] }; puts [cd 200000]").2,
        "done\n"
    );
    // No-args tailcall terminates the proc with an empty result.
    assert_eq!(
        run("proc p {} { tailcall; puts X }; p; puts done").2,
        "done\n"
    );
    assert_eq!(run("proc p {} { tailcall }; puts [p]Y").2, "Y\n");
    // Dynamic command word (generic-invoke fallback).
    assert_eq!(
        run("proc p {} { set c set; tailcall $c z 5 }; p; puts $z").2,
        "5\n"
    );
    // Outside a proc it errors (matching tclsh 9.0).
    let (ok, msg, _) = run("tailcall foo");
    assert!(!ok);
    assert_eq!(
        msg,
        "tailcall can only be called from a proc, lambda or method"
    );
}

fn assert_tailcall_rows(rows: &[(&str, &str, bool, &str)]) {
    for &(name, source, want_ok, want_result) in rows {
        let (got_ok, got_result, _) = run_bytecode(source);
        assert_eq!(got_ok, want_ok, "{name}: {got_result}");
        assert_eq!(got_result, want_result, "{name}");
    }
}

/// A nested proc's tailcall target may itself defer a script activation. These
/// Tcl 9.0.4 rows cover successful, error, and `return` body completions rather
/// than accepting the native command's empty placeholder result.
#[test]
fn tailcall_deferred_script_completion_matrix() {
    assert_tailcall_rows(&[
        (
            "eval ok",
            "proc outer {} {inner}\nproc inner {} {tailcall eval {set ::mark E; string cat eval-ok}}\nlist [outer] $::mark",
            true,
            "eval-ok E",
        ),
        (
            "eval error",
            "proc outer {} {inner}\nproc inner {} {tailcall eval {error eval-error}}\nouter",
            false,
            "eval-error",
        ),
        (
            "eval return",
            "proc outer {} {inner; error after}\nproc inner {} {tailcall eval {return eval-return}}\nouter",
            true,
            "eval-return",
        ),
        (
            "uplevel ok",
            "proc outer {} {set local before; list [inner] $local}\nproc inner {} {tailcall uplevel 0 {set local uplevel-ok}}\nouter",
            true,
            "uplevel-ok uplevel-ok",
        ),
        (
            "uplevel error",
            "proc outer {} {inner}\nproc inner {} {tailcall uplevel 0 {error uplevel-error}}\nouter",
            false,
            "uplevel-error",
        ),
        (
            "uplevel return",
            "proc outer {} {inner; error after}\nproc inner {} {tailcall uplevel 0 {return uplevel-return}}\nouter",
            true,
            "uplevel-return",
        ),
        (
            "apply ok and cleanup",
            "proc outer {} {inner}\nproc inner {} {tailcall apply {{} {return apply-ok}}}\nlist [outer] [info commands tcl::apply::lambda*]",
            true,
            "apply-ok {}",
        ),
        (
            "apply error",
            "proc outer {} {inner}\nproc inner {} {tailcall apply {{} {error apply-error}}}\nouter",
            false,
            "apply-error",
        ),
        (
            "apply return",
            "proc outer {} {inner}\nproc inner {} {tailcall apply {{} {return apply-return}}}\nouter",
            true,
            "apply-return",
        ),
    ]);
}

/// Control-command activations absorb or propagate their bodies exactly as Tcl
/// 9.0.4 does after a nested tailcall installs them.
#[test]
fn tailcall_deferred_control_completion_matrix() {
    assert_tailcall_rows(&[
        (
            "catch ok",
            "proc outer {} {list [inner] $msg [dict get $opts -code]}\nproc inner {} {tailcall catch {string cat catch-ok} msg opts}\nouter",
            true,
            "0 catch-ok 0",
        ),
        (
            "catch error",
            "proc outer {} {list [inner] $msg [dict get $opts -code]}\nproc inner {} {tailcall catch {error catch-error} msg opts}\nouter",
            true,
            "1 catch-error 1",
        ),
        (
            "catch return",
            "proc outer {} {list [inner] $msg [dict get $opts -code]}\nproc inner {} {tailcall catch {return catch-return} msg opts}\nouter",
            true,
            "2 catch-return 0",
        ),
        (
            "try ok",
            "proc outer {} {inner}\nproc inner {} {tailcall try {string cat try-ok}}\nouter",
            true,
            "try-ok",
        ),
        (
            "try error handler",
            "proc outer {} {inner}\nproc inner {} {tailcall try {error try-error} on error e {list caught $e}}\nouter",
            true,
            "caught try-error",
        ),
        (
            "try return",
            "proc outer {} {inner; error after}\nproc inner {} {tailcall try {return try-return}}\nouter",
            true,
            "try-return",
        ),
        (
            "subst ok",
            "set template {subst-[set ::mark ok]}\nproc outer {} {inner}\nproc inner {} {tailcall subst $::template}\nouter",
            true,
            "subst-ok",
        ),
        (
            "subst error",
            "set template {before-[error subst-error]-after}\nproc outer {} {inner}\nproc inner {} {tailcall subst $::template}\nouter",
            false,
            "subst-error",
        ),
        (
            "subst return",
            "set template {before-[return -level 0 subst-return]-after}\nproc outer {} {list [inner] after}\nproc inner {} {tailcall subst $::template}\nouter",
            true,
            "before-subst-return-after after",
        ),
    ]);
}

/// The runtime-fallback foreach/lmap drivers are activation-bearing ticks too;
/// their body completion rules remain exact after a nested tailcall.
#[test]
fn tailcall_deferred_each_loop_completion_matrix() {
    assert_tailcall_rows(&[
        (
            "foreach ok",
            "proc outer {} {set ::seen {}; list [inner] $::seen}\nproc inner {} {tailcall foreach x {1 2} {lappend ::seen $x}}\nouter",
            true,
            "{} {1 2}",
        ),
        (
            "foreach error",
            "proc outer {} {inner}\nproc inner {} {tailcall foreach x {1 2} {error foreach-error}}\nouter",
            false,
            "foreach-error",
        ),
        (
            "foreach return",
            "proc outer {} {inner; error after}\nproc inner {} {tailcall foreach x {1 2} {return foreach-return}}\nouter",
            true,
            "foreach-return",
        ),
        (
            "lmap ok",
            "proc outer {} {inner}\nproc inner {} {tailcall lmap x {1 2} {string cat V $x}}\nouter",
            true,
            "V1 V2",
        ),
        (
            "lmap error",
            "proc outer {} {inner}\nproc inner {} {tailcall lmap x {1 2} {error lmap-error}}\nouter",
            false,
            "lmap-error",
        ),
        (
            "lmap return",
            "proc outer {} {inner; error after}\nproc inner {} {tailcall lmap x {1 2} {return lmap-return}}\nouter",
            true,
            "lmap-return",
        ),
    ]);
}

/// Every deferred target can freeze and resume on the coroutine's explicit
/// activation stack. Values are pinned to Tcl 9.0.4, including catch's
/// absorption of the post-resume `return` and subst's insertion of the resume
/// argument.
#[test]
fn tailcall_deferred_yield_matrix() {
    assert_tailcall_rows(&[
        (
            "eval yield",
            "proc outer {} {inner}\nproc inner {} {tailcall eval {set ::sent eval-yield; yield $::sent; return eval-done}}\ncoroutine c outer\nlist $::sent [c GO] [llength [info commands c]]",
            true,
            "eval-yield eval-done 0",
        ),
        (
            "uplevel yield",
            "proc outer {} {inner}\nproc inner {} {tailcall uplevel 0 {set ::sent uplevel-yield; yield $::sent; return uplevel-done}}\ncoroutine c outer\nlist $::sent [c GO] [llength [info commands c]]",
            true,
            "uplevel-yield uplevel-done 0",
        ),
        (
            "apply yield",
            "proc outer {} {inner}\nproc inner {} {tailcall apply {{} {set ::sent apply-yield; yield $::sent; return apply-done}}}\ncoroutine c outer\nlist $::sent [c GO] [llength [info commands c]] [info commands tcl::apply::lambda*]",
            true,
            "apply-yield apply-done 0 {}",
        ),
        (
            "catch yield",
            "proc outer {} {list [inner] $msg [dict get $opts -code]}\nproc inner {} {tailcall catch {set ::sent catch-yield; yield $::sent; return catch-done} msg opts}\ncoroutine c outer\nlist $::sent [c GO] [llength [info commands c]]",
            true,
            "catch-yield {2 catch-done 0} 0",
        ),
        (
            "try yield",
            "proc outer {} {inner}\nproc inner {} {tailcall try {set ::sent try-yield; yield $::sent; return try-done}}\ncoroutine c outer\nlist $::sent [c GO] [llength [info commands c]]",
            true,
            "try-yield try-done 0",
        ),
        (
            "subst yield",
            "set template {before-[set ::sent subst-yield; yield $::sent]-after}\nproc outer {} {inner}\nproc inner {} {tailcall subst $::template}\ncoroutine c outer\nlist $::sent [c subst-resume] [llength [info commands c]]",
            true,
            "subst-yield before-subst-resume-after 0",
        ),
        (
            "foreach yield",
            "proc outer {} {set ::seen {}; inner; set ::seen}\nproc inner {} {tailcall foreach x {1 2} {lappend ::seen $x; set ::sent F$x; yield $::sent}}\ncoroutine c outer\nlist $::sent [c one] [c two] [llength [info commands c]]",
            true,
            "F1 F2 {1 2} 0",
        ),
        (
            "lmap yield",
            "proc outer {} {inner}\nproc inner {} {tailcall lmap x {1 2} {set ::sent L$x; yield $::sent; string cat V $x}}\ncoroutine c outer\nlist $::sent [c one] [c two] [llength [info commands c]]",
            true,
            "L1 L2 {V1 V2} 0",
        ),
    ]);
}

/// A tailcalling proc leaves before its replacement enters, and a failing
/// leave callback prevents that target from running. Both orderings are pinned
/// to Tcl 9.0.4.
#[test]
fn tailcall_exec_trace_timing_and_ownership() {
    let (_, result, _) = run_bytecode(concat!(
        "set events {}\n",
        "proc note {tag args} {lappend ::events $tag:[lindex $args end]:[lindex $args 1]:[lindex $args 2]}\n",
        "proc outer {} {inner}\n",
        "proc inner {} {tailcall eval {string cat target-result}}\n",
        "trace add execution inner {enter leave} [list note I]\n",
        "trace add execution eval {enter leave} [list note E]\n",
        "list [outer] $events",
    ));
    assert_eq!(
        result,
        "target-result {I:enter:enter: I:leave:0: E:enter:enter: E:leave:0:target-result}"
    );

    let (_, result, _) = run_bytecode(concat!(
        "set events {}\n",
        "proc fail args {lappend ::events Ileave; error LEAVEFAIL}\n",
        "proc entered args {lappend ::events target-enter}\n",
        "proc outer {} {inner}\n",
        "proc inner {} {tailcall eval {set ::ran yes}}\n",
        "trace add execution inner leave fail\n",
        "trace add execution eval enter entered\n",
        "list [catch {outer} msg] $msg $events [info exists ::ran]",
    ));
    assert_eq!(result, "1 LEAVEFAIL Ileave 0");
}

/// Tcl's own tailcall-13.2 regression: the eval target defers a second
/// tailcall, which replaces the next enclosing lambda rather than returning a
/// placeholder and continuing either discarded body.
#[test]
fn tailcall_13_2_indirect_tailcall_command() {
    let (ok, result, _) = run_bytecode(concat!(
        "catch {apply {{} {",
        "apply {{} {tailcall eval tailcall subst ok; subst b}}; ",
        "subst c}}} msg opts\n",
        "list $msg [dict get $opts -code]",
    ));
    assert!(ok, "tailcall-13.2 harness: {result}");
    assert_eq!(result, "ok 0");
}

/// `time command ?count?` — runs the body in the current frame `count` times and
/// reports `N microseconds per iteration` as a 4-element list (C Tcl
/// `Tcl_TimeObjCmd`). Timing is non-deterministic, so the assertions cover the
/// shape, the current-frame side effects, error propagation, and the count<=0
/// edge (which is the one fixed value, 0).
#[test]
fn time_command() {
    // Body runs in the caller's frame.
    assert_eq!(run("time {set x 99}; puts $x").2, "99\n");
    assert_eq!(run("set n 0; time {incr n} 4; puts $n").2, "4\n");
    // count <= 0 runs nothing and reports 0 (an integer).
    assert_eq!(
        run("puts [time {set x 1} 0]").2,
        "0 microseconds per iteration\n"
    );
    // Result is the 4-element list shape.
    assert_eq!(
        run("puts [lrange [time {set x 1}] 1 3]").2,
        "microseconds per iteration\n"
    );
    // A body error propagates.
    let (ok, msg, _) = run("time {error boom}");
    assert!(!ok);
    assert_eq!(msg, "boom");
    // Arity error.
    let (ok, msg, _) = run("time");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"time command ?count?\"");
}
