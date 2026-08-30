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

//! End-to-end event-loop tests for the bytecode VM.
//!
//! Every expectation is the output of the same script under a locally-built
//! **tclsh 9.0.4** (the truth oracle). The loop (`after`/`vwait`/`update`) is
//! the scheduler half of the coroutine subsystem: `after 0 $coro` schedules a
//! resume and `vwait`/`update` drives it. Tests avoid real wall-clock waits
//! (they use `after 0`/`after idle` and short, order-only timers) so they are
//! deterministic and fast.

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

/// The captured stdout of a script expected to succeed.
fn out(src: &str) -> String {
    let (ok, res, out) = run(src);
    assert!(ok, "script errored: {res}");
    out
}

/// The trimmed result string of a script expected to succeed.
fn result(src: &str) -> String {
    let (ok, res, _) = run(src);
    assert!(ok, "script errored: {res}");
    res
}

// after / vwait / update

#[test]
fn after_zero_then_vwait() {
    // tclsh 9.0.4: `after 0` schedules a timer that `vwait` drives to completion.
    assert_eq!(
        out("after 0 {set ::done 1}; vwait ::done; puts \"done=$::done\""),
        "done=1\n"
    );
}

#[test]
fn after_idle_then_update() {
    // tclsh 9.0.4: an idle handler runs on the next `update`.
    assert_eq!(out("after idle {puts idle}; update"), "idle\n");
}

#[test]
fn after_zero_events_run_in_scheduling_order() {
    // tclsh 9.0.4: two `after 0` handlers fire in the order scheduled.
    assert_eq!(
        out("set ::r {}; after 0 {lappend ::r a}; after 0 {lappend ::r b}; update; puts $::r"),
        "a b\n"
    );
}

#[test]
fn timers_fire_in_deadline_order() {
    // tclsh 9.0.4: timers fire by deadline, not scheduling order. A final `after`
    // sets the stop variable so `vwait` returns once all three have run.
    assert_eq!(
        out(
            "set ::r {}; after 30 {lappend ::r c}; after 10 {lappend ::r a}; \
             after 20 {lappend ::r b}; after 40 {set ::stop 1}; vwait ::stop; puts $::r"
        ),
        "a b c\n"
    );
}

#[test]
fn after_returns_zero_based_id() {
    // tclsh 9.0.4: the first scheduled event is `after#0`.
    assert_eq!(result("after idle {foo}"), "after#0");
}

#[test]
fn after_cancel_by_id() {
    // tclsh 9.0.4: a cancelled timer never fires.
    assert_eq!(
        out("set id [after 1000 {puts nope}]; after cancel $id; update; puts done"),
        "done\n"
    );
}

#[test]
fn after_cancel_by_script() {
    // tclsh 9.0.4: `after cancel <script>` cancels the matching handler.
    assert_eq!(
        out("after idle {puts nope}; after cancel {puts nope}; update; puts done"),
        "done\n"
    );
}

#[test]
fn update_idletasks_runs_only_idle_events() {
    // tclsh 9.0.4: `update idletasks` runs idle handlers but not due timers.
    assert_eq!(
        out(
            "set ::r {}; after 0 {lappend ::r timer}; after idle {lappend ::r idle}; \
             update idletasks; puts $::r"
        ),
        "idle\n"
    );
}

#[test]
fn update_bad_option() {
    // tclsh 9.0.4: a non-`idletasks` single argument is a bad option, but a
    // second argument is a wrong-# args error.
    let (ok, msg, _) = run("catch {update bogus} e; set e");
    assert!(ok);
    assert_eq!(msg, "bad option \"bogus\": must be idletasks");
    let (ok, msg, _) = run("catch {update a b} e; set e");
    assert!(ok);
    assert_eq!(msg, "wrong # args: should be \"update ?idletasks?\"");
}

#[test]
fn after_bad_argument() {
    let (ok, msg, _) = run("catch {after banana {x}} e; set e");
    assert!(ok);
    assert_eq!(
        msg,
        "bad argument \"banana\": must be cancel, idle, info, or an integer"
    );
}

// The coroutine driver: after 0 $coro

#[test]
fn event_loop_drives_a_coroutine() {
    // tclsh 9.0.4: the idiomatic pattern — an `after`-scheduled resume steps a
    // coroutine, which reschedules itself until it finishes and trips `vwait`.
    assert_eq!(
        out(
            "proc feed {} { foreach x {a b c} { puts $x; yield }; set ::done 1 }; \
             coroutine c feed; \
             proc pump {} { if {[llength [info commands c]]} { c; after idle pump } }; \
             after idle pump; vwait ::done; puts finished"
        ),
        "a\nb\nc\nfinished\n"
    );
}

#[test]
fn background_error_goes_to_bgerror() {
    // tclsh 9.0.4: an error in an event handler is routed to the `bgerror` proc,
    // and the loop continues.
    assert_eq!(
        out("proc bgerror {msg} { puts \"bg:$msg\" }; after 0 {error oops}; update; puts survived"),
        "bg:oops\nsurvived\n"
    );
}
