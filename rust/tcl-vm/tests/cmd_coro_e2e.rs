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

//! End-to-end coroutine tests for the bytecode VM.
//!
//! Every expectation is the output of the same script under a locally-built
//! **tclsh 9.0.4** (the truth oracle). Coroutines are implemented by capturing
//! the explicit activation stack (no OS threads); a `yield` crosses the compiled
//! stack (proc bodies, inline loops, nested proc calls) **and command
//! substitution** — the `set arg [yield $result]` resume-value idiom and
//! `cmd [yield]` argument position both stay on the explicit stack (a whole-word
//! `[…]` compiles to an inline `INVOKE`, not a runtime `subst_word` re-entry).
//!
//! `yield` also crosses `eval`, `uplevel 0`, `catch`, a straight-line `lmap`,
//! `subst`, `try` (body/handler/`finally`, each its own explicit-stack phase —
//! issue #1311), a value-consuming `lmap`/`foreach` runtime fallback (issue
//! #1311's `each_loop` activation), and a bare `apply` call (issue #1311 —
//! bound to a temporary proc run via `pending_eval`, mirroring how
//! `coroutine … apply {lambda}` already ran the lambda on the coroutine's own
//! stack). Each of these runs its body on the explicit stack (a transparent
//! script / catch / try-phase / each-loop activation, the inline collecting
//! loop, or the scanner-driven subst frame) rather than through a nested
//! `Vm::eval_source` drive on the native Rust stack. A `yield` reached across
//! a genuine host re-entry (`lsort -command`'s comparator, an `invoke_command`
//! re-entry from outside the trampoline) still errors `cannot yield: C stack
//! busy` — C Tcl refuses that one too (issue #1311's investigation confirmed
//! `lsort -command` is parity, not a gap).

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
// `lower_to_ir_for_bytecode`, not the analysis-oriented `lower_to_ir` (which
// e.g. emits `beginCatch`-shaped IR for `try` instead of the runtime-call
// shape `tcl-vm-cli` actually compiles) — this file's harness used the wrong
// one, which happened not to matter for eval/lmap/subst/catch's IR shape but
// gave a *different, wrong* compiled shape for `try`'s handler/`finally`
// phases (issue #1311 test-writing fallout: `yield_across_try_handler`
// silently ran the whole coroutine to completion in one resume instead of
// suspending in the handler, because this lowering does not go through
// `cmd_try` the way the real VM does). See `cmd_control_e2e.rs`'s identical
// import for the precedent.
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
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
// yield across `catch`
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

// ===========================================================================
// yield across `lmap`
// ===========================================================================

#[test]
fn yield_across_lmap_generator_collects() {
    // tclsh 9.0.4: a straight-line `lmap` body lowers to the inline collecting
    // loop, so `yield` crosses it on the explicit stack. `coroutine` consumes the
    // first yield (1); the resumes see 2, 3, and the loop then returns the mapped
    // list of the three resume values.
    assert_eq!(
        result(
            "proc g {} { lmap n {1 2 3} {yield $n} }; \
             coroutine c g; list [c A] [c B] [c C]"
        ),
        "2 3 {A B C}"
    );
}

#[test]
fn yield_across_multivar_lmap_generator() {
    // A two-variable `lmap` (2 groups over 4 elements = 2 iterations): the first
    // yield ("12") is consumed by creation, `[c P]` yields the second ("34"), then
    // the loop returns the collected resume values `{P Q}`.
    assert_eq!(
        result(
            "proc g {} { lmap {a b} {1 2 3 4} {yield $a$b} }; \
             coroutine c g; list [c P] [c Q]"
        ),
        "34 {P Q}"
    );
}

#[test]
fn yield_across_value_consumed_lmap_generator() {
    // issue #1311 — `set r [lmap x {1 2} { yield $x }]` reaches `lmap` through
    // generic command dispatch (a value-consuming position, not a bare
    // statement), which used to run through `Vm::eval_source`'s nested drive
    // and reject the `yield` with "cannot yield: C stack busy". tclsh 9.0.4:
    // creation consumes the first yield (1, whose resume value "A" becomes
    // the first iteration's collected element); `[c A]` yields the second
    // (2); `[c B]`'s resume value "B" becomes the second iteration's
    // collected element, and the proc then returns the collected list.
    assert_eq!(
        result(
            "proc g {} { set r [lmap x {1 2} { yield $x }]; return $r }; \
             coroutine c g; list [c A] [c B]"
        ),
        "2 {A B}"
    );
}

// ===========================================================================
// yield across `subst`
// ===========================================================================

#[test]
fn yield_across_subst_command_substitutions() {
    // tclsh 9.0.4: a `subst` whose template has yielding `[…]` brackets runs on a
    // scanner-driven subst frame, so `yield` crosses it. `coroutine` consumes the
    // first yield (1); `[c P]` resumes it (folding "P" into the output) and yields
    // the second bracket's 2; `[c Q]` resumes that and the scan finishes with the
    // fully-substituted template.
    assert_eq!(
        result(
            "proc g {} { subst {a=[yield 1]b=[yield 2]} }; \
             coroutine c g; list [c P] [c Q]"
        ),
        "2 a=Pb=Q"
    );
}

#[test]
fn subst_generator_of_bracket_values() {
    // Two adjacent yielding brackets: the resume values concatenate into the subst
    // result once the scan completes.
    assert_eq!(
        result(
            "proc g {} { subst {[yield a][yield b]} }; \
             coroutine c g; list [c X] [c Y]"
        ),
        "b XY"
    );
}

#[test]
fn catch_absorbs_subst_bracket_error_across_yield() {
    // A `subst` bracket yields, then (after resume) a later bracket errors; the
    // error propagates out of the subst and is caught by the enclosing `catch`,
    // proving the subst frame's state survived the suspend and its error unwinds
    // cleanly (status 1, message).
    assert_eq!(
        result(
            "proc g {} { set r [catch {subst {v=[yield 1][error boom]}} m]; yield \"$r/$m\" }; \
             coroutine c g; list [c A] [c B]"
        ),
        "1/boom B"
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
fn yield_across_bare_apply_call() {
    // issue #1311 — `apply {{} { yield a }}` called *from inside* a coroutine
    // body (not as the coroutine's own initial command, which
    // `coroutine_apply_generator` above already covers via a dedicated
    // internal-proc binding) used to run the lambda body through a
    // host-stack re-entry, where `yield` could not cross it. tclsh 9.0.4:
    // the bare `apply` call yields once ("a", consumed by creation); the
    // proc then continues past it and returns "done".
    assert_eq!(
        result(
            "proc g {} { apply {{} { yield a }}; return done }; \
             coroutine c g; c"
        ),
        "done"
    );
}

// ===========================================================================
// yield across `try` (issue #1311)
// ===========================================================================

#[test]
fn yield_across_try_body() {
    // tclsh 9.0.4: `try`'s body used to run through `Vm::eval_source`'s
    // nested drive (`cmd_try.rs`'s `eval_body`), rejecting a `yield` inside
    // it with "cannot yield: C stack busy". Creation consumes the first
    // yield ("a"); `[c]` resumes it, runs the second `yield b`, and returns
    // "b" (the resume value from creation, "a", is discarded — nothing reads
    // it — matching the repro in issue #1311).
    assert_eq!(
        result(
            "proc g {} { try { yield a; yield b } finally {}; return done }; \
             coroutine c g; c"
        ),
        "b"
    );
}

#[test]
fn yield_across_try_handler() {
    // A matched `on ok` handler's script also runs on the explicit stack:
    // the body yields once (consumed by creation), the resume value becomes
    // the body's result, the handler matches (`on ok`) and itself yields.
    assert_eq!(
        result(
            "proc g {} { try { yield a } on ok {v} { yield \"h:$v\" }; return done }; \
             coroutine c g; c X"
        ),
        "h:X"
    );
}

#[test]
fn yield_across_try_finally() {
    // `finally` runs as its own phase on the explicit stack too: the body
    // completes without yielding, `finally` itself yields (creation consumes
    // it — "mid"), and once resumed `finally`'s own `Ok` completion does not
    // override the `try`'s result — the body's original outcome ("done")
    // does.
    assert_eq!(
        result(
            "proc g {} { set r [try { set x done } finally { yield mid }]; return $r }; \
             list [coroutine c g] [c]"
        ),
        "mid done"
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
fn completed_hidden_coroutine_is_retired_from_hidden_state() {
    // Tcl 9.0.4: completing through invokehidden consumes the hidden token;
    // neither a second resume nor a later expose can resurrect it.
    assert_eq!(
        result(
            "proc g {} {yield a; return done}; coroutine c g; interp hide {} c held; \
             list [interp invokehidden {} held] [interp hidden {}] \
             [catch {interp invokehidden {} held}] [catch {interp expose {} held again}]"
        ),
        "done {} 1 0"
    );
}

#[test]
fn completed_hidden_coroutine_fires_its_delete_trace_once() {
    assert_eq!(
        result(
            "proc g {} {yield a; return done}; proc cb args {lappend ::events [lindex $args end]}; \
             coroutine c g; trace add command c delete cb; interp hide {} c held; \
             list [interp invokehidden {} held] $::events [interp hidden {}]"
        ),
        "done delete {}"
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
