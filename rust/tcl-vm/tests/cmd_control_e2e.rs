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

//! End-to-end coverage of the control-flow command implementations:
//! `try`/`throw` (`cmd_try.rs`), the `if`/`while`/`for`/`foreach`/`lmap`
//! runtime fallbacks (`cmd_control.rs`), and `switch`'s runtime path
//! (`cmd_switch.rs`).
//!
//! Each test compiles real Tcl via `tcl-compiler` and runs it through `tcl-vm`,
//! asserting the observable behaviour (result value, captured `puts` output, or
//! caught error message). Every expected value is the output of real `tclsh`
//! (8.6 and 9.0, which agree on all of these) — differential testing with tclsh
//! as the oracle. The `// tclsh:` comments cite that reference output.
//!
//! DRIVING THE RUNTIME PATH. The compiler inlines the static literal forms of
//! these constructs (e.g. `if {...} {...}`, `switch x {...}`, and — see the BUG
//! note on `try_literal_form_*` below — even some `try` forms compile to the
//! unimplemented `beginCatch` opcode). To exercise the *runtime command*
//! implementations in the three target files, every test invokes the command
//! through a computed name (`set c if; $c ...`), which defeats the inline fast
//! path and routes through the registered builtin. This is the same technique
//! the existing `run_script.rs`/`builtins_e2e.rs` suites use for fallback paths.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
// Compile the way the real VM runtime does (`tcl-vm-cli`): the bytecode lowering
// turns constructs the backend can't compile correctly — a literal `try` with
// handlers, nested complex `foreach`/`lmap` — into runtime-command barriers,
// where the analysis-oriented `lower_to_ir` would instead emit `beginCatch`
// exception ranges the VM does not implement. Using the analysis lowering here
// would test bytecode the VM never actually runs.
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

/// A `tcl-compiler`-backed compile service so the VM can resolve runtime
/// `eval` / `[command substitution]` (the injection seam — `tcl-vm` itself
/// never depends on the compiler).
struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        // Mirror the real VM compile services: a hard parse error becomes a
        // catchable runtime error with the exact C Tcl message.
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

/// A `Write` sink backed by a shared buffer the test can read afterwards.
#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run `src`; return `(ok, result-string, captured-stdout)`.
fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

// ===========================================================================
// try / throw  (cmd_try.rs)
//
// Driven through a computed command name `set t try; $t ...` so the construct
// routes through the registered `cmd_try`/`cmd_throw` builtins. (The literal
// `try { ... } on ... { ... }` form is mis-compiled — see `try_literal_form_*`.)
// ===========================================================================

/// `try body` with no handlers returns the body result; an unmatched error
/// propagates.
#[test]
fn try_body_only_passes_through() {
    // tclsh: `2`
    assert_eq!(run("set t try; $t { expr {1+1} }").1, "2");
    // No matching handler: the error propagates.
    // tclsh: error "boom"
    let (ok, msg, _) = run("set t try; $t { error boom }");
    assert!(!ok);
    assert_eq!(msg, "boom");
}

/// `on code varList script`: an `on ok` handler binds the body result and runs;
/// an `on error` handler binds `[resultVar optionsVar]` and the options dict
/// carries `-code`/`-errorcode`.
#[test]
fn try_on_ok_and_on_error() {
    // tclsh: `5`
    assert_eq!(run("set t try; $t { expr {2+3} } on ok r { set r }").1, "5");
    // on error binds result + options; -code is 1 (error).
    // tclsh: `oops 1`
    assert_eq!(
        run("set t try; $t { error oops } on error {m o} { list $m [dict get $o -code] }").1,
        "oops 1",
    );
}

/// `on` matches the numeric completion code: `on 7` matches a `return -code 7`
/// body; the symbolic word `break` (code 3) matches a `break` body.
#[test]
fn try_on_numeric_and_symbolic_codes() {
    // A custom completion code 7 raised by the body matches `on 7`.
    // tclsh: `hi`
    assert_eq!(
        run("set t try; $t { return -code 7 hi } on 7 r { set r }").1,
        "hi",
    );
    // Symbolic `break` (= code 3): a `break` body is caught by `on break`.
    // tclsh: `caught`
    assert_eq!(
        run("set t try; $t { break } on break {} { set r caught }").1,
        "caught",
    );
}

/// `trap pattern varList script`: matches an error whose `-errorcode` has the
/// given leading sublist. An empty pattern matches any error.
#[test]
fn try_trap_errorcode_prefix() {
    // A `throw {MY ERR} boom` is trapped by `trap MY` (leading-sublist match).
    // tclsh: `caught boom {MY ERR}`
    assert_eq!(
        run("set t try; $t { throw {MY ERR} boom } trap MY {m o} \
             { list caught $m [dict get $o -errorcode] }")
        .1,
        "caught boom {MY ERR}",
    );
    // A `return -code error -errorcode {A B}` body is trapped by `trap {A B}`.
    // tclsh: `fail`
    assert_eq!(
        run("set t try; $t { return -code error -errorcode {A B} fail } trap {A B} m { set m }").1,
        "fail",
    );
    // Empty trap pattern matches any error.
    // tclsh: `any`
    assert_eq!(
        run("set t try; $t { error whatever } trap {} {} { set r any }").1,
        "any",
    );
}

/// A `trap` whose pattern is not a prefix of the error's `-errorcode` does NOT
/// match, so the error propagates.
#[test]
fn try_trap_no_match_propagates() {
    // errorcode is {MY ERR}; `trap OTHER` does not match → propagate.
    // tclsh: error "boom" (rc 1)
    let (ok, msg, _) = run("set t try; $t { throw {MY ERR} boom } trap OTHER {} { set r x }");
    assert!(!ok);
    assert_eq!(msg, "boom");
}

/// `finally` always runs; an ok finally leaves the prior outcome, a throwing
/// finally overrides it.
#[test]
fn try_finally_runs_and_can_override() {
    // finally runs for its side effect; the on-error result is preserved.
    // tclsh stdout: "FIN\n", and the `try` result is "got".
    assert_eq!(
        run("set t try; puts [$t { error e } on error {} { set r got } finally { puts FIN }]").2,
        "FIN\ngot\n",
    );
    // A throwing finally overrides the handler's result/outcome.
    // tclsh: `finerr`
    let (ok, msg, _) =
        run("set t try; $t { error e } on error {} { set x ok } finally { error finerr }");
    assert!(!ok);
    assert_eq!(msg, "finerr");
}

/// `finally` runs even when the body succeeds and no handler matches.
#[test]
fn try_finally_runs_on_success() {
    // tclsh stdout: "FIN\n", result "5".
    assert_eq!(
        run("set t try; puts [$t { expr {2+3} } finally { puts FIN }]").2,
        "FIN\n5\n",
    );
}

/// A handler that itself throws supersedes the body's exception and chains it
/// via `-during` (TIP 329 exception chaining): the body's options ride along.
#[test]
fn try_handler_throws_chains_during() {
    let (ok, _msg, _) = run("set t try; $t { error body } on error {} { error handler }");
    assert!(!ok);
    // The propagated error's options carry a `-during` link to the body error,
    // whose own `-code` is 1.
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set t try\n",
            "catch {$t { error body } on error {} { error handler }} m o\n",
            "dict get [dict get $o -during] -code"
        ))
        .1,
        "1",
    );
}

/// An `on error -` (dash body) falls through to the next clause's body.
#[test]
fn try_dash_fallthrough() {
    // The first clause's `-` body falls through to the next `on` clause's body.
    // tclsh: `fell`
    assert_eq!(
        run("set t try; $t { error e } on error {} - on error {} { set r fell }").1,
        "fell",
    );
    // A multi-step `-` chain across `trap` clauses resolves to the next real body.
    // tclsh: `td`
    assert_eq!(
        run("set t try; $t { throw {A} x } trap A {} - trap {} {} { set r td }").1,
        "td",
    );
}

/// `try` grammar errors are raised before the body runs (validated by
/// `parse_clauses`).
#[test]
fn try_grammar_errors() {
    // tclsh: `bad handler type "badkw": must be finally, on, or trap`
    assert_eq!(
        run("set t try; catch {$t good badkw foo} e; set e").1,
        "bad handler type \"badkw\": must be finally, on, or trap",
    );
    // tclsh: `wrong # args to on clause: must be "... on code variableList script"`
    assert_eq!(
        run("set t try; catch {$t good on} e; set e").1,
        "wrong # args to on clause: must be \"... on code variableList script\"",
    );
    // tclsh: `wrong # args to trap clause: must be "... trap pattern variableList script"`
    assert_eq!(
        run("set t try; catch {$t good trap} e; set e").1,
        "wrong # args to trap clause: must be \"... trap pattern variableList script\"",
    );
    // tclsh: `wrong # args to finally clause: must be "... finally script"`
    assert_eq!(
        run("set t try; catch {$t good finally} e; set e").1,
        "wrong # args to finally clause: must be \"... finally script\"",
    );
    // tclsh: `finally clause must be last`
    assert_eq!(
        run("set t try; catch {$t good finally a b} e; set e").1,
        "finally clause must be last",
    );
    // tclsh: `bad completion code "notacode": ...`
    assert_eq!(
        run("set t try; catch {$t good on notacode {} {x}} e; set e").1,
        "bad completion code \"notacode\": must be ok, error, return, break, \
         continue, or an integer",
    );
    // tclsh: `last non-finally clause must not have a body of "-"`
    assert_eq!(
        run("set t try; catch {$t good on error {} -} e; set e").1,
        "last non-finally clause must not have a body of \"-\"",
    );
    // tclsh: `wrong # args: should be "try body ?handler ...? ?finally script?"`
    assert_eq!(
        run("set t try; catch {$t} e; set e").1,
        "wrong # args: should be \"try body ?handler ...? ?finally script?\"",
    );
}

/// A `trap` pattern that is not a well-formed list errors (the `as_list` Err arm
/// in `parse_clauses`). The VM reports a `must be a list` prefix error.
#[test]
fn try_trap_bad_pattern_list() {
    // An unbalanced brace makes the pattern an invalid list; the parse error is
    // caught. The VM's `parse_clauses` rejects it with a "must be a list" wording.
    let (ok, m, _) = run("set t try; catch {$t good trap \"a b \\{\" {} {x}} e; set e");
    assert!(ok, "the catch should complete: {m}");
    assert!(
        m.contains("must be a list"),
        "expected a 'must be a list' prefix error, got: {m}",
    );
}

/// A completion code outside the signed 32-bit range is not a valid `on` code
/// word (error-20.2): `code_word_to_int` rejects it.
#[test]
fn try_on_code_out_of_int_range() {
    // tclsh: `bad completion code "9999999999": ...`
    assert_eq!(
        run("set t try; catch {$t good on 9999999999 {} {x}} e; set e").1,
        "bad completion code \"9999999999\": must be ok, error, return, break, \
         continue, or an integer",
    );
}

/// `throw type message` raises an error with `-errorcode type`. The type must be
/// a non-empty list; `throw` arity is exactly 2.
#[test]
fn throw_command() {
    // tclsh: `boom {MY ERR}`
    assert_eq!(
        run("set th throw; catch {$th {MY ERR} boom} m o; list $m [dict get $o -errorcode]").1,
        "boom {MY ERR}",
    );
    // tclsh: `type must be non-empty list`
    let (ok, msg, _) = run("set th throw; $th {} msg");
    assert!(!ok);
    assert_eq!(msg, "type must be non-empty list");
    // tclsh: `wrong # args: should be "throw type message"`
    let (ok, msg, _) = run("set th throw; $th");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"throw type message\"");
}

/// The LITERAL `try { body } on error {vars} { handler }` form must route
/// through `cmd_try`. The analysis lowering emits the bytecode exception-range
/// opcode (`beginCatch`) for it, which the VM does not implement; the *bytecode*
/// lowering the real VM runtime uses (`lower_to_ir_for_bytecode`, mirrored by
/// this file's `run`) instead drops it to a runtime-command barrier, so the
/// handler runs. Previously this file compiled with the analysis lowering, so
/// the literal form skipped the handler and propagated the body's error — and a
/// literal `try { … } finally { … }` aborted with a hard
/// "opcode beginCatch4 not implemented in tcl-vm" VM error.
///
/// tclsh 8.6/9.0:  `try { error oops } on error {m o} { set m }`  ->  result "oops" (rc 0)
///
/// The dynamic form (`set t try; $t ...`) was always correct, confirming the
/// defect was the harness's lowering choice, not `cmd_try` itself.
#[test]
fn try_literal_form_on_error_handler() {
    // tclsh: `oops` (the handler runs and returns the body's message)
    let (ok, msg, _) = run("try { error oops } on error {m o} { set m }");
    assert!(
        ok,
        "literal try/on-error should run the handler, got error: {msg}",
    );
    assert_eq!(msg, "oops");
}

// ===========================================================================
// if  (cmd_control.rs, runtime fallback via computed command name)
// ===========================================================================

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

/// A condition expression error in the runtime `if` propagates.
#[test]
fn if_runtime_condition_error() {
    // tclsh: `expected boolean value but got "notbool"`
    let (ok, msg, _) = run("set c if; $c {notbool} {set r 1}");
    assert!(!ok);
    assert_eq!(msg, "expected boolean value but got \"notbool\"");
}

// ===========================================================================
// while  (cmd_control.rs)
// ===========================================================================

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

/// A condition expression error in the runtime `while` propagates.
#[test]
fn while_runtime_condition_error() {
    // tclsh: `expected boolean value but got "notbool"`
    let (ok, msg, _) = run("set c while; $c {notbool} {set x 1}");
    assert!(!ok);
    assert_eq!(msg, "expected boolean value but got \"notbool\"");
}

// ===========================================================================
// for  (cmd_control.rs)
// ===========================================================================

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
    // condition error propagates.
    // tclsh: `expected boolean value but got "x"`
    let (ok, msg, _) = run("set c for; $c {set i 0} {x} {incr i} {set z 1}");
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

// ===========================================================================
// foreach / lmap  (cmd_control.rs)
// ===========================================================================

/// The runtime `foreach` fallback: single var, multiple vars per group, and
/// multiple groups (short groups pad with empty).
#[test]
fn foreach_runtime_variants() {
    // single var
    // tclsh stdout: "1\n2\n3\n"
    assert_eq!(run("set c foreach; $c x {1 2 3} {puts $x}").2, "1\n2\n3\n");
    // two vars per group consume the list two at a time
    // tclsh stdout: "1-2\n3-4\n"
    assert_eq!(
        run("set c foreach; $c {a b} {1 2 3 4} {puts \"$a-$b\"}").2,
        "1-2\n3-4\n",
    );
    // two groups, second shorter -> empty pad in the last iteration
    // tclsh stdout: "1a\n2b\n3\n"
    assert_eq!(
        run("set c foreach; $c x {1 2 3} y {a b} {puts \"$x$y\"}").2,
        "1a\n2b\n3\n",
    );
}

/// A direct nested iterator routes the outer literal `foreach` through the
/// runtime command boundary. That gives the inner loop a fresh activation;
/// inlining both loops into one CFG used to leave only the final outer item.
#[test]
fn nested_foreach_preserves_every_outer_iteration() {
    // tclsh 8.6 / 9.0: `a:HTTP TCP|b:HTTP TCP|c:HTTP TCP`
    assert_eq!(
        run("set result {}; \
             foreach outer {a b c} { \
                 foreach {key value} {profiles {HTTP TCP} steps ignored} { \
                     if {$key eq \"profiles\"} { lappend result \"$outer:$value\" } \
                 } \
             }; \
             join $result {|}",)
        .1,
        "a:HTTP TCP|b:HTTP TCP|c:HTTP TCP",
    );
}

/// `foreach`/`lmap` arity: name-stripped argv must be an odd count >= 3.
#[test]
fn foreach_lmap_runtime_arity() {
    // tclsh: `wrong # args: should be "foreach varList list ?varList list ...? command"`
    assert_eq!(
        run("set c foreach; catch {$c} m; set m").1,
        "wrong # args: should be \"foreach varList list ?varList list ...? command\"",
    );
    // even arg count
    assert_eq!(
        run("set c foreach; catch {$c a b c d} m; set m").1,
        "wrong # args: should be \"foreach varList list ?varList list ...? command\"",
    );
    // tclsh: `wrong # args: should be "lmap varList list ?varList list ...? command"`
    assert_eq!(
        run("set c lmap; catch {$c x y} m; set m").1,
        "wrong # args: should be \"lmap varList list ?varList list ...? command\"",
    );
}

/// An empty var-list for a group errors with the command name.
#[test]
fn foreach_lmap_runtime_empty_varlist() {
    // tclsh: `foreach varlist is empty`
    assert_eq!(
        run("set c foreach; catch {$c {} {1 2} {body}} m; set m").1,
        "foreach varlist is empty",
    );
    // tclsh: `lmap varlist is empty`
    assert_eq!(
        run("set c lmap; catch {$c {} {1 2} {body}} m; set m").1,
        "lmap varlist is empty",
    );
}

/// `lmap` collects each non-`continue` body result; `continue` skips collection;
/// `break` ends the loop.
#[test]
fn lmap_runtime_collects() {
    // tclsh: `2 4 6`
    assert_eq!(run("set c lmap; $c x {1 2 3} {expr {$x*2}}").1, "2 4 6");
    // continue skips the element.
    // tclsh: `2 6`
    assert_eq!(
        run("set c lmap; $c x {1 2 3} {if {$x==2} continue; expr {$x*2}}").1,
        "2 6",
    );
    // break ends collection early.
    // tclsh: `2 4`
    assert_eq!(
        run("set c lmap; $c x {1 2 3 4 5} {if {$x>2} break; expr {$x*2}}").1,
        "2 4",
    );
}

/// `break` in the runtime `foreach` body.
#[test]
fn foreach_runtime_break() {
    // tclsh: `1 2`
    assert_eq!(
        run("set c foreach; set r {}; $c x {1 2 3 4} {if {$x==3} break; lappend r $x}; set r").1,
        "1 2",
    );
}

/// A `return` out of the runtime `foreach` body propagates.
#[test]
fn foreach_runtime_return_propagates() {
    // tclsh: `early`
    assert_eq!(
        run("proc p {} { set c foreach; $c x {1 2} { return early } }; p").1,
        "early",
    );
}

/// An error in the runtime `foreach`/`lmap` body adds the body frame.
#[test]
fn foreach_runtime_body_error_adds_frame() {
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set c foreach\n",
            "catch {$c x {1 2 3} {error boom}}\n",
            "string match {boom*(\"foreach\" body line 1)*} $::errorInfo"
        ))
        .1,
        "1",
    );
}

/// A malformed list as the value list errors (the `as_list` Err arm).
#[test]
fn foreach_runtime_bad_list_value() {
    // An unbalanced brace in the value list errors.
    // tclsh: `unmatched open brace in list`
    let (ok, msg, _) = run("set c foreach; $c x \"a b \\{\" {set z 1}");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

// ===========================================================================
// switch  (cmd_switch.rs, runtime path via -glob/-regexp/-nocase/dynamic)
// ===========================================================================

/// `-glob` matching with a `default` fall-through clause.
#[test]
fn switch_glob_with_default() {
    // tclsh: `glob-a`
    assert_eq!(
        run("switch -glob abc {a* {set r glob-a} b* {set r glob-b} default {set r none}}").1,
        "glob-a",
    );
    // no glob matches -> default runs.
    // tclsh: `none`
    assert_eq!(
        run("switch -glob zzz {a* {set r glob-a} b* {set r glob-b} default {set r none}}").1,
        "none",
    );
}

/// `-regexp` matching, including the `-` fall-through across pattern/body pairs
/// and a trailing `default`.
#[test]
fn switch_regexp_fallthrough_and_default() {
    // The first pattern's `-` body falls through to the next pattern's body.
    // tclsh: `matched`
    assert_eq!(
        run("switch -regexp -- abc123 {[a-z]+ - [0-9]+ {set r matched} default {set r none}}").1,
        "matched",
    );
}

/// `-nocase` exact matching, and combined with `-glob`.
#[test]
fn switch_nocase() {
    // tclsh: `yes`
    assert_eq!(
        run("switch -nocase ABC {abc {set r yes} default {set r no}}").1,
        "yes",
    );
    // -nocase combined with -glob.
    // tclsh: `hit`
    assert_eq!(
        run("switch -nocase -glob ABC {a*c {set r hit} default {set r no}}").1,
        "hit",
    );
}

/// `--` ends option processing so a value starting with `-` is treated as the
/// string, not an option.
#[test]
fn switch_double_dash() {
    // tclsh: `dashx`
    assert_eq!(
        run("switch -- -x {-x {set r dashx} default {set r d}}").1,
        "dashx",
    );
}

/// `-matchvar` captures the regexp match + submatches; `-indexvar` captures the
/// match index pairs (TIP #75). Both are written before the body runs.
#[test]
fn switch_matchvar_indexvar() {
    // -matchvar holds {whole sub1 sub2 ...}.
    // tclsh: `abcXYZ abc XYZ`
    assert_eq!(
        run("switch -regexp -matchvar mv -- abcXYZ {([a-z]+)([A-Z]+) {set mv}}").1,
        "abcXYZ abc XYZ",
    );
    // The matchvar is readable inside the body for indexing a submatch.
    // tclsh: `123`
    assert_eq!(
        run("switch -regexp -matchvar m -- foo123bar {([0-9]+) {lindex $m 1}}").1,
        "123",
    );
    // -indexvar holds the {start end} index pair of the whole match.
    // tclsh: `{2 3}`
    assert_eq!(
        run("switch -regexp -indexvar iv -- abcde {cd {set iv}}").1,
        "{2 3}",
    );
}

/// A multi-step `-` fall-through chain resolves to the next real body.
#[test]
fn switch_multi_dash_fallthrough() {
    // tclsh: `abc`
    assert_eq!(run("switch -glob a {a - b - c {set r abc}}").1, "abc");
}

/// The inline (non-brace-list) pattern/body form: pattern/body words given as
/// separate args rather than a single braced list.
#[test]
fn switch_inline_pairs_form() {
    // tclsh: `ia`
    assert_eq!(
        run("set s switch; $s -glob abc a* {set r ia} b* {set r ib}").1,
        "ia",
    );
}

/// No pattern matches and there is no `default`: `switch` returns empty.
#[test]
fn switch_no_match_returns_empty() {
    // tclsh: `<>`
    assert_eq!(
        run("set r [switch -glob nomatch {a* 1 b* 2}]; subst <$r>").1,
        "<>",
    );
}

/// `switch` options abbreviate like tclsh (`Tcl_GetIndexFromObj`, flags 0):
/// `-gl`/`-e`/`-noc` resolve to their options in both the direct form (the
/// compiler bails to the runtime for a non-exact option word) and the
/// runtime-dispatch form. Probed tclsh 8.6.14 (S4.2).
#[test]
fn switch_option_abbreviation() {
    // tclsh: `switch -gl -- abc {a* {concat ia}}` → `ia`.
    assert_eq!(run("switch -gl -- abc {a* {concat ia}}").1, "ia");
    // tclsh: `-e` → -exact, `-noc` → -nocase.
    assert_eq!(
        run("set s switch; $s -e -- b {a {concat A} b {concat B}}").1,
        "B",
    );
    assert_eq!(
        run("set s switch; $s -noc -- ABC {abc {concat nc}}").1,
        "nc"
    );
}

/// `switch` error paths: bad option, empty body list, odd pattern/body count
/// (both brace-list and inline), a trailing `-` with nothing to fall to, and the
/// misplaced-comment heuristic.
#[test]
fn switch_error_paths() {
    // bad option
    // tclsh: `bad option "-badopt": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --`
    assert_eq!(
        run("set s switch; catch {$s -badopt x {a 1}} m; set m").1,
        "bad option \"-badopt\": must be -exact, -glob, -indexvar, -matchvar, \
         -nocase, -regexp, or --",
    );
    // ambiguous option: a lone `-` prefixes every table entry.
    // tclsh: `ambiguous option "-": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --`
    assert_eq!(
        run("set s switch; catch {$s - x {a 1}} m; set m").1,
        "ambiguous option \"-\": must be -exact, -glob, -indexvar, -matchvar, \
         -nocase, -regexp, or --",
    );
    // empty body list
    // tclsh: `wrong # args: should be "switch ?-option ...? string {?pattern body ...? ?default body?}"`
    assert_eq!(
        run("set s switch; catch {$s -glob x {}} m; set m").1,
        "wrong # args: should be \"switch ?-option ...? string \
         {?pattern body ...? ?default body?}\"",
    );
    // odd pattern/body count in the brace-list form
    // tclsh: `extra switch pattern with no body`
    assert_eq!(
        run("set s switch; catch {$s -glob x {a 1 b}} m; set m").1,
        "extra switch pattern with no body",
    );
    // odd pattern/body count in the inline form
    assert_eq!(
        run("set s switch; catch {$s -glob x a 1 b} m; set m").1,
        "extra switch pattern with no body",
    );
    // trailing `-` body with nothing to fall through to
    // tclsh: `no body specified for pattern "c"`
    assert_eq!(
        run("set s switch; catch {$s -glob x {a 1 b 2 c -}} m; set m").1,
        "no body specified for pattern \"c\"",
    );
    // misplaced-comment heuristic: odd count whose first pattern begins with `#`.
    // tclsh: `extra switch pattern with no body, this may be due to a comment ...`
    assert_eq!(
        run("set s switch; catch {$s -glob x {# 1 2}} m; set m").1,
        "extra switch pattern with no body, this may be due to a comment \
         incorrectly placed outside of a switch body - see the \"switch\" \
         documentation",
    );
}

/// A `switch` body that is `return`/`break` propagates that code.
#[test]
fn switch_body_control_propagates() {
    // a return inside a switch body inside a proc returns from the proc.
    // tclsh: `fromswitch`
    assert_eq!(
        run("proc p {} { switch -glob abc {a* { return fromswitch }} }; p").1,
        "fromswitch",
    );
    // a break inside a switch body inside a loop breaks the loop.
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set c foreach; set n 0\n",
            "$c x {1 2 3} { switch -glob $x {1 {incr n} * {break}} }\n",
            "set n"
        ))
        .1,
        "1",
    );
}

// ===========================================================================
// try / throw — additional error-path coverage  (cmd_try.rs)
// ===========================================================================

/// A hard *parse* error in a `try` body (not just a runtime error) is mapped to
/// an error completion that a handler can catch — the `eval_body` `Err(e)` arm.
#[test]
fn try_body_parse_error_is_catchable() {
    // `set "x"yyy` is a hard parse error (text after a close-quote); the
    // `on error` handler runs.
    // tclsh: result "h" (the handler ran)
    assert_eq!(
        run("set t try; $t { set \"x\"yyy 1 } on error {} { set r h }").1,
        "h",
    );
}

/// A handler's variable bind that *fails* (here: binding into a constant, a Tcl
/// 9 feature the VM targets) becomes the handler outcome (skipping the body) and
/// chains the body's exception via `-during` (C's `handlerFailed`).
#[test]
fn try_handler_var_bind_failure_chains_during() {
    // Binding the result var into a constant `c` fails.
    // tclsh 9.0: `can't set "c": variable is a constant`
    let (ok, msg, _) = run("const c 1; set t try; $t { error body } on error {c} { set r ran }");
    assert!(!ok);
    assert_eq!(msg, "can't set \"c\": variable is a constant");
    // The bind failure chains the body's exception as `-during` (-code 1).
    // tclsh 9.0: `1`
    assert_eq!(
        run(concat!(
            "const c 1; set t try\n",
            "catch {$t { error body } on error {c} { set r ran }} m o\n",
            "dict get [dict get $o -during] -code"
        ))
        .1,
        "1",
    );
}

/// `add_during` REPLACES an existing `-during` rather than duplicating it: a
/// handler throws (chaining the body as `-during`), then a `finally` throws,
/// whose `add_during` over the same options must overwrite the link. The
/// surviving `-during` still resolves to the body's `-code` (1).
#[test]
fn try_add_during_replaces_existing() {
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set t try\n",
            "catch {$t { error body } on error {} { error handler } finally { error fin }} m o\n",
            "dict get [dict get $o -during] -code"
        ))
        .1,
        "1",
    );
}

/// `exit` inside a `try` body is not catchable (C's `Tcl_Exit`): the unwind
/// propagates *without* running any handler or the `finally` clause.
#[test]
fn try_exit_skips_handlers_and_finally() {
    // The finally must NOT run (no "FIN" on stdout) and an on-error handler must
    // not catch the exit. (tclsh actually terminates the process here with the
    // exit code; the VM models the unwind, but the observable — finally skipped —
    // matches.)
    let (ok, _msg, out) =
        run("set t try; $t { exit 3 } on error {} { set r h } finally { puts FIN }");
    assert!(!ok, "exit should propagate as a non-ok completion");
    assert_eq!(
        out, "",
        "neither the handler nor finally should run on exit"
    );
}

/// `throw` whose `type` is not a well-formed list errors (the `ty.as_list()` Err
/// arm in `cmd_throw`).
#[test]
fn throw_bad_type_list() {
    // An unbalanced brace makes the type an invalid list.
    // tclsh: `unmatched open brace in list`
    let (ok, msg, _) = run("set th throw; $th \"a \\{ b\" msg");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

// ===========================================================================
// cmd_control.rs / cmd_switch.rs — additional error-path coverage
// ===========================================================================

/// A `foreach`/`lmap` loop-variable write that fails propagates the set error
/// (the `set_var` Err arm in `each_loop`). Binding the loop var to a constant
/// (Tcl 9) fails.
#[test]
fn foreach_runtime_var_write_failure() {
    // tclsh 9.0: `can't set "c": variable is a constant`
    let (ok, msg, _) = run("const c 1; set f foreach; $f c {1 2 3} {set z 1}");
    assert!(!ok);
    assert_eq!(msg, "can't set \"c\": variable is a constant");
}

/// A malformed list as a `foreach` var-list errors (the `pair[0].as_list()` Err
/// arm in `each_loop`).
#[test]
fn foreach_runtime_bad_varlist_list() {
    // An unbalanced brace makes the var-list an invalid list.
    // tclsh: `unmatched open brace in list`
    let (ok, msg, _) = run("set f foreach; $f \"a \\{\" {1 2} {set z 1}");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// A malformed brace-list body errors (the `rest[0].as_list()` Err arm in
/// `cmd_switch`).
#[test]
fn switch_runtime_bad_body_list() {
    // An unbalanced brace in the body list.
    // tclsh: `unmatched open brace in list`
    let (ok, msg, _) = run("set s switch; $s -glob x \"a 1 \\{\"");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// A `-regexp` switch whose pattern does not compile errors (the
/// `core_switch::select` Err arm in `cmd_switch`). The VM targets the Tcl 9.0
/// wording (8.6 phrases it differently).
#[test]
fn switch_runtime_bad_regexp() {
    // tclsh 9.0: `cannot compile regular expression pattern: invalid quantifier operand`
    // (tclsh 8.6: `couldn't compile regular expression pattern: quantifier operand invalid`)
    let (ok, msg, _) = run("set s switch; $s -regexp -- abc {* {set z 1}}");
    assert!(!ok);
    assert_eq!(
        msg,
        "cannot compile regular expression pattern: invalid quantifier operand",
    );
}

/// A `-matchvar`/`-indexvar` write that fails propagates the set error (the
/// `set_var` Err arm in `cmd_switch`). Writing into a constant (Tcl 9) fails.
#[test]
fn switch_runtime_matchvar_write_failure() {
    // tclsh 9.0: `can't set "c": variable is a constant`
    let (ok, msg, _) =
        run("const c 1; set s switch; $s -regexp -matchvar c -- abc {a.. {set z 1}}");
    assert!(!ok);
    assert_eq!(msg, "can't set \"c\": variable is a constant");
}

/// A hard *parse* error in the chosen `switch` body is mapped to an error (the
/// `eval_source` Err arm in `cmd_switch`).
#[test]
fn switch_runtime_body_parse_error() {
    // tclsh: `extra characters after close-quote`
    let (ok, msg, _) = run("set s switch; $s -glob abc {a* {set \"q\"zzz 1}}");
    assert!(!ok);
    assert_eq!(msg, "extra characters after close-quote");
}

// ===========================================================================
/// `add_during` REPLACES (does not duplicate) a `-during` key that is already
/// present in the options dict it is given. Three nested `try`s: the outer body
/// errors; the outer handler runs a nested `try` whose own handler throws
/// (yielding an error that already carries `-during`); chaining that as the
/// outer outcome's `-during` must overwrite the inner link in place. The
/// surviving `-during` still resolves to the body's `-code` (1).
#[test]
fn try_add_during_replaces_in_nested_chain() {
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set t try\n",
            "catch {$t { error obody } on error {} \
                 { $t { error ibody } on error {} { error ihandler } }} m o\n",
            "dict get [dict get $o -during] -code"
        ))
        .1,
        "1",
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

// Former VM-vs-tclsh divergences: each now asserts the correct tclsh behaviour
// and passes, guarding the fix against regression.
// ===========================================================================

/// BUG (`Vm::set_var`, surfaced via `cmd_try`'s `bind_handler_vars`): binding a
/// `try` handler's result/options variable into an *array element whose base is
/// a scalar* (`x(y)` while `x` is a scalar) should fail with
/// `can't set "x(y)": variable isn't array` (C's `handlerFailed` → the bind
/// error becomes the handler outcome, skipping the body). The `set` *command*
/// already enforces this (a bare `set x(y) 5` errors correctly — verified), but
/// the internal `Vm::set_var` that `bind_handler_vars` calls does not, so the
/// bind silently succeeds and the handler body runs.
///
/// tclsh 8.6/9.0:  result is the error `can't set "x(y)": variable isn't array`.
///
/// Fixed: `bind_handler_vars` now writes through `Vm::var_set`, which resolves
/// `x(y)` to the array element and fails on a scalar base (guards regression).
#[test]
fn try_handler_var_bind_array_on_scalar_should_fail() {
    let (ok, msg, _) = run("set t try; set x 1; $t { error e } on error {x(y)} { set r ran }");
    assert!(
        !ok,
        "binding handler result into x(y) while x is scalar should fail, got ok with: {msg}",
    );
    assert_eq!(msg, "can't set \"x(y)\": variable isn't array");
}

// ===========================================================================
// Native-stack safety (issue #996) — the runtime `if`/`while`/`for` fallback
// (this file) recurses on the host stack via `Vm::eval_source` when driven
// through a computed command name (`set c if; $c ...`, defeating the
// compiled fast path — see this file's module doc comment). Confirmed
// empirically: before `NATIVE_EVAL_SOURCE_DEPTH_LIMIT`, this reliably
// overflowed the stack (SIGABRT) between depth 50 and 60 on a 2 MiB thread.
// ===========================================================================

/// Deep dynamic-dispatch `if` nesting returns a catchable error instead of
/// crashing the process. 100 is comfortably past both the measured 50-60
/// crash range and `NATIVE_EVAL_SOURCE_DEPTH_LIMIT` (16).
#[test]
fn deeply_nested_dynamic_if_errors_instead_of_crashing() {
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
    assert!(!ok, "expected a catchable error, got ok with: {result}");
    assert_eq!(result, "too many nested evaluations (infinite loop?)");
}

/// A shallow dynamic-dispatch `if` (well under the safety net) still runs
/// normally — the safety net must not fire on realistic nesting depths.
#[test]
fn shallow_dynamic_if_still_runs() {
    assert_eq!(run("set c if\n$c {1} {\n    set done 1\n}\n").1, "1");
}

/// `CONTROL_FALLBACK_DEPTH_LIMIT` is scoped to `cmd_control.rs`'s runtime
/// fallback specifically (see that constant's doc comment) — ordinary
/// nested `[…]` command substitution, unrelated to this file's fallback
/// commands, must not be affected by it. An earlier version of this fix
/// capped `Vm::eval_source` itself (the shared mechanism command
/// substitution also uses) and broke exactly this: 45 real nested
/// substitutions is far more than any realistic iRule needs, comfortably
/// past what the earlier, wrongly-scoped fix would have allowed, and
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
