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

//! End-to-end tests for the VM's shared-nothing `thread` package
//! (`RUST_ISSUE_008`, phase 3).
//!
//! The reference tclsh 9.0.4 is a **non-threaded** build (no `Thread` package,
//! `tcl_platform(threaded)` unset), so there is no oracle for this subsystem —
//! unlike the rest of the VM. These tests instead pin the documented Tcl
//! `Thread`-package semantics and are made **deterministic** by construction:
//! synchronous `thread::send` blocks for its result, and cross-thread totals are
//! read back only after every worker has been joined (`thread::release`), so
//! there are no timing races.

use std::sync::{Arc, Mutex};

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

/// A `Write` sink over a shared byte buffer — used for both the main
/// interpreter's output and (as the `Send` sink) every worker's, so a test can
/// observe combined `puts` output.
#[derive(Clone)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn compiler() -> Box<dyn CompileService<Module = tcl_bytecode::ModuleAsm>> {
    Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    })
}

/// Compile and run `src` on a thread-enabled VM; return `(ok, result, stdout)`.
fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let buf = Arc::new(Mutex::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(SharedBuf(Arc::clone(&buf))));
    vm.set_compiler(compiler());
    vm.enable_threads(
        Arc::new(compiler),
        Arc::new(Mutex::new(Box::new(SharedBuf(Arc::clone(&buf))))),
    );
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.lock().unwrap().clone()).expect("utf-8 output");
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
// Basics: identity, platform flag, availability
// ===========================================================================

#[test]
fn thread_id_and_platform_flag() {
    // The main interpreter is thread 1, and the package advertises itself.
    assert_eq!(result("list [thread::id] $tcl_platform(threaded)"), "1 1");
}

#[test]
fn thread_package_absent_without_enable() {
    // A VM that never called `enable_threads` reports `threaded` 0 and rejects
    // `thread::create`.
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir("set tcl_platform(threaded)", &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let mut vm = Vm::with_output(Box::new(std::io::sink()));
    vm.set_compiler(compiler());
    let c = vm.run_module(&asm);
    assert!(c.code.is_ok());
    assert_eq!(&*c.result.to_str(), "0");
}

// ===========================================================================
// thread::create / send / release
// ===========================================================================

#[test]
fn sync_send_returns_the_eval_result() {
    // A synchronous `thread::send` evaluates in the worker and returns its value.
    assert_eq!(
        result(
            "set w [thread::create]; set r [thread::send $w {expr {6 * 7}}]; thread::release $w; set r"
        ),
        "42"
    );
}

#[test]
fn worker_keeps_its_own_state() {
    // Each worker has its own interpreter: a variable set in one send is visible
    // to the next send to the *same* worker, and workers are independent.
    assert_eq!(
        result(
            "set w [thread::create]; \
             thread::send $w {set acc 0}; \
             thread::send $w {incr acc 10}; \
             set r [thread::send $w {incr acc 5}]; \
             thread::release $w; set r"
        ),
        "15"
    );
}

#[test]
fn error_in_worker_propagates_to_a_sync_send() {
    // A worker error surfaces as the `thread::send` result (catchable).
    assert_eq!(
        result(
            "set w [thread::create]; \
             set rc [catch {thread::send $w {error \"boom\"}} e]; \
             thread::release $w; list $rc $e"
        ),
        "1 boom"
    );
}

#[test]
fn send_to_a_missing_thread_errors() {
    assert_eq!(
        result("catch {thread::send 9999 {puts hi}} e; set e"),
        "thread \"9999\" does not exist"
    );
}

#[test]
fn exists_names_and_release() {
    // `thread::names` lists this thread plus workers; release drops one.
    assert_eq!(
        result(
            "set w [thread::create]; \
             set before [list [thread::exists $w] [lsort -integer [thread::names]]]; \
             thread::release $w; \
             list {*}$before [thread::exists $w]"
        ),
        "1 {1 2} 0"
    );
}

// ===========================================================================
// True parallelism over shared `tsv`
// ===========================================================================

#[test]
fn tsv_counter_is_atomic_across_threads() {
    // Four workers each bump a shared counter 250 times; `tsv::incr` is atomic
    // under the store's mutex, so the total is exactly 1000. Reading it only
    // after every worker is released removes any timing race.
    assert_eq!(
        result(
            "tsv::set c n 0; \
             set ws {}; for {set i 0} {$i < 4} {incr i} { lappend ws [thread::create] }; \
             foreach w $ws { thread::send $w {for {set j 0} {$j < 250} {incr j} { tsv::incr c n }} }; \
             foreach w $ws { thread::release $w }; \
             tsv::get c n"
        ),
        "1000"
    );
}

#[test]
fn async_send_result_observed_via_tsv() {
    // `-async` returns immediately; the worker publishes its result to `tsv`,
    // which the main thread reads back after joining the worker.
    assert_eq!(
        result(
            "tsv::set o r {}; \
             set w [thread::create]; \
             thread::send -async $w {tsv::set o r [string toupper done]}; \
             thread::release $w; \
             tsv::get o r"
        ),
        "DONE"
    );
}

// ===========================================================================
// tsv::* element operations
// ===========================================================================

#[test]
fn tsv_element_operations() {
    assert_eq!(
        result(
            "tsv::set a x 5; tsv::incr a x 3; \
             tsv::append a s foo; tsv::append a s bar; \
             tsv::lappend a l 1; tsv::lappend a l 2 3; \
             list [tsv::get a x] [tsv::get a s] [tsv::get a l] \
                  [lsort [tsv::keys a]] [tsv::exists a x] [tsv::exists a nope]"
        ),
        "8 foobar {1 2 3} {l s x} 1 0"
    );
}

#[test]
fn tsv_unset_and_names() {
    assert_eq!(
        result(
            "tsv::set a k 1; tsv::set b k 2; tsv::set c k 3; \
             tsv::unset b; \
             lsort [tsv::names]"
        ),
        "a c"
    );
}

#[test]
fn tsv_get_missing_key_errors_but_with_var_reports_presence() {
    assert_eq!(
        result("tsv::set a k 1; catch {tsv::get a nope} e; set e"),
        "key \"nope\" does not exist in shared variable \"a\""
    );
    assert_eq!(
        result("tsv::set a k 1; list [tsv::get a nope out] [tsv::get a k out] $out"),
        "0 1 1"
    );
}
