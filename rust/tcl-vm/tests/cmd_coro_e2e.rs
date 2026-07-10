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
//! A `yield` reached across a host re-entry that the VM runs on the *native*
//! Rust stack (`catch`/`eval`/`uplevel`/`apply`) still errors `cannot yield: C
//! stack busy`. C Tcl makes those commands NR-enabled, so a real tclsh yields
//! through them — this is the VM's remaining divergence and a documented
//! follow-up (making those constructs re-enter the explicit trampoline).

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
