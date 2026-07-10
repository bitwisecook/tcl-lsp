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

//! End-to-end coroutine tests for the bytecode VM (`RUST_ISSUE_008`).
//!
//! Every expectation is the output of the same script under a locally-built
//! **tclsh 9.0.4** (the truth oracle). Coroutines are implemented by capturing
//! the explicit activation stack (no OS threads); a `yield` crosses the compiled
//! stack (proc bodies, inline loops, nested proc calls) **and command
//! substitution** — the `set arg [yield $result]` resume-value idiom and
//! `cmd [yield]` argument position both stay on the explicit stack (a whole-word
//! `[…]` compiles to an inline `INVOKE`, not a runtime `subst_word` re-entry).
//!
//! `yield` also crosses `eval`, `uplevel 0`, and `catch` — each runs its body on
//! the explicit stack (a transparent script / catch activation). A `yield`
//! reached across a host re-entry the VM still runs on the *native* Rust stack
//! (`subst`, `apply` in an arbitrary position, `lsort -command`) errors `cannot
//! yield: C stack busy`. C Tcl makes those NR-enabled, so a real tclsh yields
//! through them — the remaining divergence and a documented follow-up.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

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

/// The trimmed result string of a script expected to succeed.
fn result(src: &str) -> String {
    let (ok, res, _) = run(src);
    assert!(ok, "script errored: {res}");
    res
}

// ===========================================================================
// Generators / yield round-trip
// ===========================================================================

#[test]
fn proc_generator() {
    // tclsh 9.0.4: `coroutine` consumes the first yield ("a"); each `[c]` then
    // yields the next -> "b" "c", then the body ends -> "".
    assert_eq!(
        result("proc g {} { foreach x {a b c} { yield $x } }; coroutine c g; list [c] [c] [c]"),
        "b c {}"
    );
}

#[test]
fn yield_in_while_loop_with_state() {
    // tclsh 9.0.4: a stateful generator over an inline `while` loop. `coroutine`
    // consumes the first yield (5), so the three resumes give 6, 7, 8.
    assert_eq!(
        result(
            "proc counter {start} { set i $start; while 1 { yield $i; incr i } }; \
             coroutine c counter 5; list [c] [c] [c]"
        ),
        "6 7 8"
    );
}

#[test]
fn yield_across_nested_proc_call() {
    // tclsh 9.0.4: `yield` inside a proc the coroutine body called stays on the
    // explicit stack -> works. `coroutine` consumes the first yield ("deep").
    assert_eq!(
        result(
            "proc inner {} { yield deep } ; proc g {} { inner; yield back }; \
             coroutine c g; list [c] [c]"
        ),
        "back {}"
    );
}

#[test]
fn yield_resume_value_via_command_substitution() {
    // tclsh 9.0.4: the canonical resume-value idiom `set x [yield]`. `coroutine`
    // runs the body to the first `yield` (returning ""); each resume delivers its
    // argument as that `yield`'s result, which the body echoes on the next yield.
    // A whole-word `[yield]` compiles to an inline INVOKE, so the yield stays on
    // the explicit stack (a runtime `subst_word` re-entry could not cross it).
    assert_eq!(
        result(
            "proc g {} { while 1 { set x [yield]; puts \"got=$x\" } }; \
             coroutine c g; c one; c two"
        ),
        ""
    );
    assert_eq!(
        run("proc g {} { while 1 { set x [yield]; puts \"got=$x\" } }; \
             coroutine c g; c one; c two")
        .2,
        "got=one\ngot=two\n"
    );
}

#[test]
fn accumulator_generator_yield_expression() {
    // tclsh 9.0.4: `set n [yield $sum]` both yields a value and receives the
    // resume value in one command substitution — the generator's core pattern.
    assert_eq!(
        result(
            "proc acc {} { set sum 0; while 1 { set n [yield $sum]; incr sum $n } }; \
             coroutine a acc; list [a 5] [a 10] [a 3]"
        ),
        "5 15 18"
    );
}

// ===========================================================================
// yield across `catch` (RUST_ISSUE_008 piece 2)
// ===========================================================================

#[test]
fn yield_inside_catch_body_round_trips() {
    // tclsh 9.0.4: a `yield` inside a `catch {…}` body runs on the explicit
    // stack, so the coroutine suspends *through* the catch and, on resume, the
    // resume value becomes the catch body's result (status 0).
    assert_eq!(
        result(
            "proc g {} { set r [catch {yield inner} m]; yield \"done:$r:$m\" }; \
             coroutine c g; list [c] [c foo]"
        ),
        "done:0: foo"
    );
}

#[test]
fn yield_in_catch_inside_a_generator_loop() {
    // A generator whose loop body wraps each `yield` in `catch`: the three
    // resumes see 2, 3, then the body returns.
    assert_eq!(
        result(
            "proc g {} { foreach n {1 2 3} { catch {yield $n} }; return finished }; \
             coroutine c g; list [c] [c] [c]"
        ),
        "2 3 finished"
    );
}

#[test]
fn catch_captures_a_post_resume_error_across_yield() {
    // tclsh 9.0.4: the body yields inside the catch, then (after resume) raises an
    // error — which the *same* catch captures (status 1, message), proving the
    // catch context survives the suspend/resume.
    assert_eq!(
        result(
            "proc g {} { set c [catch { set x [yield a]; error \"boom-$x\" } m]; yield \"$c/$m\" }; \
             coroutine c g; list [c] [c Z]"
        ),
        "1/boom- Z"
    );
}

#[test]
fn yield_in_command_argument_position() {
    // tclsh 9.0.4: `[yield a]` as a command argument stays on the explicit stack.
    // `coroutine` consumes the first yield ("a"); the resume value ("X") is what
    // `set` stores and the body returns.
    assert_eq!(
        result(
            "proc g {} { set r [string cat [yield a]]; return $r }; \
             coroutine c g; c X"
        ),
        "X"
    );
}

// ===========================================================================
// coroutine … apply {lambda}
// ===========================================================================

#[test]
fn coroutine_apply_generator() {
    // tclsh 9.0.4: `coroutine c apply {lambda}` — the canonical anonymous
    // generator. The lambda body runs on the coroutine's explicit stack, so
    // `yield` in its loop works. Creation consumes the first yield ("hi"); the
    // four resumes give 1, 2, 3, then "" as the body ends.
    assert_eq!(
        run(
            "coroutine c apply {{} { yield hi; foreach x {1 2 3} { yield $x } }}; \
             puts [c]; puts [c]; puts [c]; puts [c]"
        )
        .2,
        "1\n2\n3\n\n"
    );
}

#[test]
fn coroutine_apply_with_parameter() {
    // tclsh 9.0.4: the lambda's formal parameter is bound from the extra
    // `coroutine` arguments. Creation consumes the first yield (10).
    assert_eq!(
        result(
            "coroutine c apply {{start} { set i $start; while 1 { yield $i; incr i } }} 10; \
             list [c] [c] [c]"
        ),
        "11 12 13"
    );
}

#[test]
fn coroutine_apply_lambda_proc_is_cleaned_up() {
    // tclsh 9.0.4: deleting the coroutine drops it; the VM additionally tears
    // down the internal proc the lambda was bound to (no leak). Observing "1"
    // here (the `catch` on the now-gone `c`) matches C's post-delete behaviour.
    assert_eq!(
        result("coroutine c apply {{} { yield a; yield b }}; rename c {}; catch {c}"),
        "1"
    );
}

#[test]
fn independent_coroutines_interleave() {
    // tclsh 9.0.4: two coroutines keep separate state.
    assert_eq!(
        result(
            "proc g {n} { while 1 { yield $n; incr n } }; \
             coroutine a g 10; coroutine b g 20; string cat [a] [b] [a] [b]"
        ),
        "11211222"
    );
}

// ===========================================================================
// Introspection: coroprobe / coroinject / corotype, and multi-arg yieldto
// ===========================================================================

#[test]
fn coroprobe_reads_suspended_context() {
    // tclsh 9.0.4: `coroprobe` evaluates a command in the *suspended* coroutine's
    // own frame without resuming it — here reading its `foreach` loop variable.
    assert_eq!(
        result(
            "coroutine c apply {{} { foreach i {1 2} yield }}; \
             list [coroprobe c set i] [c] [coroprobe c set i]"
        ),
        "1 {} 2"
    );
}

#[test]
fn coroinject_runs_at_next_resume() {
    // tclsh 9.0.4: an injected command runs in the coroutine's context at its next
    // resume, receiving the suspend kind and resume value.
    assert_eq!(
        result(
            "set ::log {}; coroutine c apply {{} { foreach i {1 2} { yield $i } }}; \
             coroinject c apply {{o v} {lappend ::log $o $v; return $v}}; \
             c X; c Y; set ::log"
        ),
        "yield X"
    );
}

#[test]
fn corotype_reports_suspend_kind() {
    // tclsh 9.0.4: `::tcl::unsupported::corotype` reports how a coroutine is
    // parked; a bare `yield` reads "yield".
    assert_eq!(
        result(
            "coroutine c apply {{} { yield; yield 1 }}; c; \
             ::tcl::unsupported::corotype c"
        ),
        "yield"
    );
    let (ok, msg, _) = run("catch {::tcl::unsupported::corotype nope} e; set e");
    assert!(ok);
    assert_eq!(msg, "can only get coroutine type of a coroutine");
}

#[test]
fn yieldto_delivers_all_resume_args_as_a_list() {
    // tclsh 9.0.4: a `yieldto`-suspended coroutine accepts any number of resume
    // arguments, delivered to the `yieldto` as a list (a `yield` takes at most
    // one).
    assert_eq!(
        result(
            "proc g {} { set a 1; while 1 { set a [yieldto return -level 0 $a] } }; \
             coroutine c g; c; c a b c"
        ),
        "a b c"
    );
}

// ===========================================================================
// info coroutine
// ===========================================================================

#[test]
fn info_coroutine_inside_and_outside() {
    // tclsh 9.0.4: the running coroutine's fully-qualified name inside (the first
    // yield, which `coroutine` returns); "" at top level.
    assert_eq!(
        result("proc g {} { yield [info coroutine] }; coroutine c g"),
        "::c"
    );
    assert_eq!(result("info coroutine"), "");
}

// ===========================================================================
// Errors / guards
// ===========================================================================

#[test]
fn yield_outside_coroutine_errors() {
    // tclsh 9.0.4.
    let (ok, msg, _) = run("catch {yield} e; set e");
    assert!(ok);
    assert_eq!(msg, "yield can only be called in a coroutine");
}

#[test]
fn yieldto_outside_coroutine_errors() {
    let (ok, msg, _) = run("catch {yieldto set x 1} e; set e");
    assert!(ok);
    assert_eq!(msg, "yieldto can only be called in a coroutine");
}

#[test]
fn resuming_a_running_coroutine_errors() {
    // tclsh 9.0.4: a coroutine that resumes itself -> already running (caught and
    // yielded back as the first-yield value).
    assert_eq!(
        result("proc g {} { catch c e; yield $e }; coroutine c g"),
        "coroutine \"c\" is already running"
    );
}

// ===========================================================================
// Teardown: completion, delete, rename
// ===========================================================================

#[test]
fn completed_coroutine_command_is_removed() {
    // tclsh 9.0.4: once the body returns, the command is gone.
    assert_eq!(
        result("proc g {} { yield a; return done }; coroutine c g; c; catch {c}"),
        "1"
    );
}

#[test]
fn deleting_a_suspended_coroutine() {
    // tclsh 9.0.4: `rename $coro {}` drops a suspended coroutine.
    assert_eq!(
        result("proc g {} { yield; yield }; coroutine c g; rename c {}; catch {c}"),
        "1"
    );
}

#[test]
fn renaming_a_coroutine_keeps_it_working() {
    // tclsh 9.0.4: a renamed coroutine keeps its continuation under the new name
    // (created -> yield 1; `c` -> yield 2; rename; `d` -> return 3).
    assert_eq!(
        result("proc g {} { yield 1; yield 2; return 3 }; coroutine c g; c; rename c d; d"),
        "3"
    );
}

#[test]
fn body_returning_a_custom_code_propagates_it() {
    // tclsh 9.0.4 (coroutine-2.4): a coroutine whose body finishes with a
    // non-standard `return -code 100` surfaces code 100 to the resumer, so
    // `catch` reports 100 (not 0). Wrapped in `apply` for a proc-level context.
    assert_eq!(
        result(
            "apply {{} { coroutine foo ::apply [list {} {yield;yield 1; return -code 100 ouch!}]; \
             list [foo] [catch foo msg] $msg [catch foo msg] $msg }}"
        ),
        "1 100 ouch! 1 {invalid command name \"foo\"}"
    );
}

#[test]
fn info_coroutine_is_empty_after_self_deletion() {
    // tclsh 9.0.4 (coroutine-3.5): once a coroutine deletes its own command,
    // `[info coroutine]` reports empty even though its driver is still running.
    assert_eq!(
        result(
            "proc a {} {info coroutine}; proc b {} {rename [info coroutine] {}; a}; \
             coroutine foo b"
        ),
        ""
    );
}

#[test]
fn completing_coroutine_fires_local_unset_traces() {
    // tclsh 9.0.4 (coroutine-4.1): the coroutine's frame is destroyed when its
    // body finishes, so the traced local `v` gets its unset trace (after the two
    // writes across the two resumes).
    assert_eq!(
        result(
            "proc foo {} { set v 1; trace add variable v {write unset} bar; \
             yield; set v 2; yield; set v 3 }; \
             proc bar args {lappend ::res $args}; coroutine a foo; \
             apply {{} { list [a] [a] $::res }}"
        ),
        "{} 3 {{v {} write} {v {} write} {v {} unset}}"
    );
}

#[test]
fn deleting_a_suspended_coroutine_fires_local_unset_traces() {
    // tclsh 9.0.4 (coroutine-4.3 tail): deleting a *suspended* coroutine unsets
    // its parked locals, firing their unset traces even though the frame never
    // returns normally.
    assert_eq!(
        result(
            "proc foo {} { set v 1; trace add variable v {write unset} bar; \
             yield; set v 2; yield; set v 3 }; \
             proc bar args {lappend ::res $args}; \
             apply {{} { coroutine a foo; a; rename a {}; set ::res }}"
        ),
        "{v {} write} {v {} unset}"
    );
}

#[test]
fn initial_command_resolves_in_the_creation_namespace() {
    // tclsh 9.0.4 (coroutine-4.4): the coroutine's initial command is resolved
    // in the namespace where `coroutine` was called, so the namespace-local `a`
    // (not the global one) runs.
    assert_eq!(
        result(
            "proc a {} {return global}; namespace eval b {proc a {} {return local}}; \
             namespace eval b {coroutine foo a}"
        ),
        "local"
    );
}

#[test]
fn yield_across_eval() {
    // tclsh 9.0.4 (coroutine-1.9/1.10): a `yield` reached through `eval` (the
    // list form, run on the explicit stack) crosses the boundary — creation
    // consumes 0, then each resume yields the next.
    assert_eq!(
        result(
            "proc gen {} {set i 0; while {$i<3} {eval yield [expr {$i*10}]; incr i}}; \
             coroutine c gen; list [c] [c] [c]"
        ),
        "10 20 {}"
    );
}

#[test]
fn yield_across_uplevel_0() {
    // tclsh 9.0.4 (coroutine-1.7/1.8/1.12): a `yield` reached through `uplevel 0`
    // (same frame, run on the explicit stack) crosses the boundary.
    assert_eq!(
        result(
            "proc gen {} {set i 0; while {$i<3} {uplevel 0 yield [expr {$i*10}]; incr i}}; \
             coroutine c gen; list [c] [c] [c]"
        ),
        "10 20 {}"
    );
}
