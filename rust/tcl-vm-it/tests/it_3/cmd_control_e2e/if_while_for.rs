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

//! The `if`, `while` and `for` runtime fallbacks (`cmd_control.rs`), loop jumps across compiled and runtime loops, and deeply nested dynamic controls.

use super::*;

#[test]
fn runtime_loop_break_after_an_inner_compiled_loop_exits_the_outer_loop() {
    let source = r"
interp alias {} scan_loop {} while
set pos 0
set len 2
scan_loop {$pos < $len} {
    while {$pos < $len} {incr pos}
    if {$pos >= $len} {break}
}
set pos
";
    let (ok, result, _) = run(source);
    assert!(ok, "{result}");
    assert_eq!(result, "2");
}

#[test]
fn runtime_loop_continue_after_an_inner_compiled_loop_resumes_the_outer_loop() {
    let source = r"
interp alias {} scan_loop {} while
set n 0
scan_loop {$n < 2} {
    set inner 0
    while {$inner < 1} {incr inner}
    incr n
    continue
    error UNREACHABLE
}
set n
";
    let (ok, result, _) = run(source);
    assert!(ok, "{result}");
    assert_eq!(result, "2");
}

#[test]
fn replaced_loop_jump_commands_keep_their_normal_continuation() {
    for command in ["break", "continue"] {
        let source = format!(
            "proc {command} {{}} {{return NORMAL}}; set i 0; set count 0; \
             while {{$i < 2}} {{incr i; {command}; incr count}}; list $i $count"
        );
        let (ok, result, _) = run(&source);
        assert!(ok, "{command}: {result}");
        assert_eq!(result, "2 2", "{command}");
    }
}

#[test]
fn replacing_a_loop_jump_during_argument_evaluation_keeps_the_continuation() {
    for command in ["break", "continue"] {
        let source = format!(
            "proc install {{}} {{rename {command} original_jump; \
             proc {command} args {{return NORMAL}}; return ARG}}; \
             set i 0; set count 0; while {{$i < 1}} \
             {{incr i; {command} [install]; incr count}}; list $i $count"
        );
        let (ok, result, _) = run(&source);
        assert!(ok, "{command}: {result}");
        assert_eq!(result, "1 1", "{command}");
    }
}

#[test]
fn native_option_loop_breaks_through_an_aliased_if() {
    let script = r#"
rename while native_while
interp alias {} while {} native_while
rename if native_if
interp alias {} if {} native_if
proc parse_options {args} {
    set idx 0
    set n [llength $args]
    while {$idx < $n} {
        set tok [lindex $args $idx]
        if {$tok eq "--"} {incr idx; break}
        if {[string index $tok 0] eq "-"} {incr idx} else {break}
    }
    return [lindex $args $idx]
}
list [parse_options api.example.com equals hosts] [parse_options -nocase -- api.example.com equals hosts]
"#;
    assert_eq!(
        run(script),
        (
            true,
            "api.example.com api.example.com".into(),
            String::new()
        )
    );
}

// if  (cmd_control.rs, runtime fallback via computed command name)

/// The runtime `if` fallback: then/elseif/else chains, the optional `then`/
/// `else` keywords, and the implicit (bare) else body.
#[test]
fn if_runtime_then_elseif_else() {
    // tclsh: `yes`
    assert_eq!(
        run("set c if; $c {1} {set r yes} else {set r no}; set r").1,
        "yes",
    );
    // tclsh: `2`
    assert_eq!(
        run("set c if; $c {0} then {set r 1} elseif {1} then {set r 2} else {set r 3}; set r").1,
        "2",
    );
    // Implicit else body (no `else` keyword).
    // tclsh: `imp`
    assert_eq!(run("set c if; $c 0 {set r t} {set r imp}; set r").1, "imp");
    // A false `if` with no else returns empty.
    // tclsh: `<>`
    assert_eq!(run("set c if; set r [$c 0 {set r t}]; subst <$r>").1, "<>");
}

/// Later conditions in an `if` chain are NOT evaluated once a branch matches
/// (their side effects are skipped) — `then_body` records the first true branch
/// but the chain keeps scanning without re-evaluating conditions.
#[test]
fn if_runtime_later_conditions_not_evaluated() {
    // The elseif condition has a side effect (`set hit 1`) that must not run.
    // tclsh: `0`
    assert_eq!(
        run("set c if; set hit 0; $c {1} {set r A} elseif {[set hit 1]} {set r B}; set hit").1,
        "0",
    );
}

/// The runtime `if` grammar errors: no expression, no script after a token, and
/// extra words after the else clause.
#[test]
fn if_runtime_grammar_errors() {
    // tclsh: `wrong # args: no expression after "if" argument`
    assert_eq!(
        run("set c if; catch {$c} m; set m").1,
        "wrong # args: no expression after \"if\" argument",
    );
    // tclsh: `wrong # args: no script following "1" argument`
    assert_eq!(
        run("set c if; catch {$c 1} m; set m").1,
        "wrong # args: no script following \"1\" argument",
    );
    // tclsh: `wrong # args: no script following "then" argument`
    assert_eq!(
        run("set c if; catch {$c 1 then} m; set m").1,
        "wrong # args: no script following \"then\" argument",
    );
    // tclsh: `wrong # args: no script following "else" argument`
    assert_eq!(
        run("set c if; catch {$c 1 then x else} m; set m").1,
        "wrong # args: no script following \"else\" argument",
    );
    // tclsh: `wrong # args: extra words after "else" clause in "if" command`
    assert_eq!(
        run("set c if; catch {$c 1 then x extra words here} m; set m").1,
        "wrong # args: extra words after \"else\" clause in \"if\" command",
    );
    // A no-expression error after an `elseif` reports the `elseif` clause.
    // tclsh: `wrong # args: no expression after "elseif" argument`
    assert_eq!(
        run("set c if; catch {$c 0 {x} elseif} m; set m").1,
        "wrong # args: no expression after \"elseif\" argument",
    );
}

/// A condition expression error in the runtime `if` propagates — a *syntax*
/// error from the parse, and a boolean-coercion error from a parsable condition.
#[test]
fn if_runtime_condition_error() {
    // `Tcl_IfObjCmd` runs the condition through `Tcl_ExprBooleanObj`, so it is
    // parsed as an expression first: an unquoted word is a bareword, never a
    // string to coerce (`tclCompExpr.c:766`; parseExpr-21.3).
    // tclsh: `invalid bareword "notbool"` …
    let (ok, msg, _) = run("set c if; $c {notbool} {set r 1}");
    assert!(!ok);
    assert_eq!(
        msg,
        "invalid bareword \"notbool\"\nin expression \"notbool\";\n\
         should be \"$notbool\" or \"{notbool}\" or \"notbool(...)\" or ..."
    );
    // Quoted, it *is* a string operand, and the boolean coercion is what fails
    // (expr-old-28.12: `if {"abc"} {}`).
    // tclsh: `expected boolean value but got "notbool"`
    let (ok, msg, _) = run("set c if; $c {\"notbool\"} {set r 1}");
    assert!(!ok);
    assert_eq!(msg, "expected boolean value but got \"notbool\"");
}

// while  (cmd_control.rs)

/// The runtime `while` fallback loops while the condition holds, then returns
/// empty.
#[test]
fn while_runtime_loops() {
    // tclsh: `3`
    assert_eq!(
        run("set c while; set i 0; $c {$i<3} {incr i}; set i").1,
        "3"
    );
}

/// `while` arity is exactly 2.
#[test]
fn while_runtime_arity() {
    // tclsh: `wrong # args: should be "while test command"`
    assert_eq!(
        run("set c while; catch {$c 1 2 3} m; set m").1,
        "wrong # args: should be \"while test command\"",
    );
}

/// `break`/`continue` inside the runtime `while` body control the loop.
#[test]
fn while_runtime_break_continue() {
    // continue skips the rest; break exits. Collect the even values <= 4.
    // tclsh: `0 2 4`
    assert_eq!(
        run(concat!(
            "set c while; set i -1; set r {}\n",
            "$c {[incr i]<10} { if {$i%2} continue; if {$i>4} break; lappend r $i }\n",
            "set r"
        ))
        .1,
        "0 2 4",
    );
}

/// A `return` out of the runtime `while` body propagates (the `_ => return c`
/// arm).
#[test]
fn while_runtime_return_propagates() {
    // tclsh: a proc whose while returns -> "early"
    assert_eq!(
        run("proc p {} { set c while; $c {1} { return early } }; p").1,
        "early",
    );
}

/// An error in the runtime `while` body adds the `("while" body line N)` frame
/// to `errorInfo` (the `append_body_frame` path).
#[test]
fn while_runtime_error_adds_body_frame() {
    // tclsh: errorInfo contains the `("while" body line 1)` frame.
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set c while; set i 0\n",
            "catch {$c {$i<3} {incr i; error boom}}\n",
            "string match {boom*while executing*\"error boom\"*(\"while\" body line 1)*} \
             $::errorInfo"
        ))
        .1,
        "1",
    );
}

/// A condition expression error in the runtime `while` propagates — the
/// condition is parsed as an expression, so an unquoted word is a bareword
/// (`tclCompExpr.c:766`), and only a quoted one reaches the boolean coercion.
#[test]
fn while_runtime_condition_error() {
    // tclsh: `invalid bareword "notbool"` …
    let (ok, msg, _) = run("set c while; $c {notbool} {set x 1}");
    assert!(!ok);
    assert_eq!(
        msg,
        "invalid bareword \"notbool\"\nin expression \"notbool\";\n\
         should be \"$notbool\" or \"{notbool}\" or \"notbool(...)\" or ..."
    );
    // tclsh: `expected boolean value but got "notbool"`
    let (ok, msg, _) = run("set c while; $c {\"notbool\"} {set x 1}");
    assert!(!ok);
    assert_eq!(msg, "expected boolean value but got \"notbool\"");
}

// for  (cmd_control.rs)

/// The runtime `for` fallback runs init once, then loops test/body/next.
#[test]
fn for_runtime_basic() {
    // tclsh stdout: "0\n1\n2\n"
    assert_eq!(
        run("set c for; $c {set i 0} {$i<3} {incr i} {puts $i}").2,
        "0\n1\n2\n",
    );
}

/// `for` arity is exactly 4.
#[test]
fn for_runtime_arity() {
    // tclsh: `wrong # args: should be "for start test next command"`
    assert_eq!(
        run("set c for; catch {$c 1 2 3} m; set m").1,
        "wrong # args: should be \"for start test next command\"",
    );
}

/// `continue` in the runtime `for` body still runs the `next` clause.
#[test]
fn for_runtime_continue_runs_next() {
    // tclsh: `0 1 3 4`  (i==2 is skipped from the result, but incr still runs)
    assert_eq!(
        run(concat!(
            "set c for; set r {}\n",
            "$c {set i 0} {$i<5} {incr i} {if {$i==2} {continue}; lappend r $i}\n",
            "set r"
        ))
        .1,
        "0 1 3 4",
    );
}

/// `break` in the runtime `for` body ends the loop.
#[test]
fn for_runtime_break_in_body() {
    // tclsh: `0 1 2`
    assert_eq!(
        run(concat!(
            "set c for; set r {}\n",
            "$c {set i 0} {1} {incr i} {if {$i>2} break; lappend r $i}\n",
            "set r"
        ))
        .1,
        "0 1 2",
    );
}

/// A `break` in the runtime `for` *next* clause ends the loop cleanly (C's
/// `Tcl_ForObjCmd` maps `TCL_BREAK` from the step to `TCL_OK`).
#[test]
fn for_runtime_break_in_next() {
    // The body runs for i=0,1,2; the next clause breaks when i==2.
    // tclsh: `0 1 2`
    assert_eq!(
        run(concat!(
            "set c for; set r {}\n",
            "$c {set i 0} {$i<5} {if {$i==2} break; incr i} {lappend r $i}\n",
            "set r"
        ))
        .1,
        "0 1 2",
    );
}

/// An error in the runtime `for` body adds the `("for" body line N)` frame.
#[test]
fn for_runtime_body_error_adds_frame() {
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set c for\n",
            "catch {$c {set i 0} {$i<3} {incr i} {error boom}}\n",
            "string match {boom*(\"for\" body line 1)*} $::errorInfo"
        ))
        .1,
        "1",
    );
}

/// An error in the runtime `for` init / next / condition clause propagates.
#[test]
fn for_runtime_clause_errors() {
    // init error propagates immediately.
    // tclsh: `initerr`
    let (ok, msg, _) = run("set c for; $c {error initerr} {1} {x} {y}");
    assert!(!ok);
    assert_eq!(msg, "initerr");
    // next-clause error propagates.
    // tclsh: `nexterr`
    let (ok, msg, _) = run("set c for; $c {set i 0} {$i<3} {error nexterr} {incr i}");
    assert!(!ok);
    assert_eq!(msg, "nexterr");
    // condition error propagates. The clause is an expression, so a bare `x` is
    // C's bareword syntax error (`tclCompExpr.c:766`), not a string to coerce.
    // tclsh: `invalid bareword "x"` …
    let (ok, msg, _) = run("set c for; $c {set i 0} {x} {incr i} {set z 1}");
    assert!(!ok);
    assert_eq!(
        msg,
        "invalid bareword \"x\"\nin expression \"x\";\n\
         should be \"$x\" or \"{x}\" or \"x(...)\" or ..."
    );
    // A parsable condition that is not a boolean still reports the coercion.
    // tclsh: `expected boolean value but got "x"`
    let (ok, msg, _) = run("set c for; $c {set i 0} {\"x\"} {incr i} {set z 1}");
    assert!(!ok);
    assert_eq!(msg, "expected boolean value but got \"x\"");
}

/// A `return` out of the runtime `for` body propagates.
#[test]
fn for_runtime_return_propagates() {
    // tclsh: `early`
    assert_eq!(
        run("proc p {} { set c for; $c {set i 0} {1} {incr i} { return early } }; p").1,
        "early",
    );
}

/// Dynamic controls retain their continuations on the VM activation stack.
/// All five C Tcl references and Jim 0.84 complete this fixed nesting with 1.
#[test]
fn deeply_nested_dynamic_if_uses_retained_vm_continuations() {
    const DEPTH: usize = 100;
    let mut src = "set c if\n".to_owned();
    for _ in 0..DEPTH {
        src.push_str("$c {1} {\n");
    }
    src.push_str("set done 1\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    let (ok, result, _) = run(&src);
    assert!(ok, "{result}");
    assert_eq!(result, "1");
}

#[test]
fn dynamic_controls_preserve_the_configured_procedure_recursion_limit() {
    let source = r"
interp recursionlimit {} 40
proc walk {n} {
    set c if
    $c {$n > 0} {walk [expr {$n - 1}]} else {return done}
}
catch {walk 100} message
set message
";
    // Real C Tcl 8.4–9.1 all reject this invocation with the same message.
    let (ok, result, _) = run(source);
    assert!(ok, "{result}");
    assert_eq!(result, "too many nested evaluations (infinite loop?)");
}

/// A shallow dynamic-dispatch `if` follows the same continuation path.
#[test]
fn shallow_dynamic_if_still_runs() {
    assert_eq!(run("set c if\n$c {1} {\n    set done 1\n}\n").1, "1");
}

/// `CONTROL_FALLBACK_DEPTH_LIMIT` is scoped to `cmd_control.rs`'s runtime
/// fallback specifically (see that constant's doc comment) — ordinary
/// nested `[…]` command substitution, unrelated to this file's fallback
/// commands, must not be affected by it. Capping `Vm::eval_source` itself
/// (the shared mechanism command substitution also uses) would break exactly
/// this: 45 real nested substitutions is far more than any realistic iRule
/// needs, comfortably past what a low, uniform cap would allow, and
/// nowhere near where pure substitution recursion actually becomes
/// dangerous (empirically safe to at least depth 1000 on a 2 MiB thread).
#[test]
fn deeply_nested_command_substitution_is_unaffected() {
    const DEPTH: usize = 45;
    let mut src = String::new();
    for _ in 0..DEPTH {
        src.push_str("[subst {");
    }
    src.push('1');
    for _ in 0..DEPTH {
        src.push_str("}]");
    }
    assert_eq!(run(&format!("set x {src}\n")).1, "1");
}
