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

// ===========================================================================
// thread::mutex / cond / rwmutex — synchronisation primitives
// ===========================================================================
//
// Worker bodies are passed as *braced* scripts (no send-time interpolation) and
// read handles back from `tsv`, so the whole body runs in the worker. Each test
// joins every worker (`thread::send`'s sync barrier + `thread::release`) before
// reading a shared total, so the results are deterministic.

#[test]
fn mutex_create_lock_unlock_roundtrip() {
    // Handles are minted; lock/unlock/destroy succeed and return empty.
    assert_eq!(
        result(
            "set m [thread::mutex create]; \
             thread::mutex lock $m; thread::mutex unlock $m; \
             thread::mutex destroy $m; set m"
        ),
        "mutex0"
    );
}

#[test]
fn recursive_mutex_relocks_on_same_thread() {
    // A -recursive mutex can be re-locked by its owner without deadlocking.
    assert_eq!(
        result(
            "set m [thread::mutex create -recursive]; \
             thread::mutex lock $m; thread::mutex lock $m; \
             thread::mutex unlock $m; thread::mutex unlock $m; \
             thread::mutex destroy $m; set ok done"
        ),
        "done"
    );
}

#[test]
fn mutex_serialises_a_non_atomic_read_modify_write() {
    // Four workers each do 250 *non-atomic* tsv increments under one mutex; the
    // total is exact only because the mutex provides real mutual exclusion.
    let src = "
        tsv::set c n 0
        tsv::set c m [thread::mutex create]
        set ws {}
        for {set i 0} {$i < 4} {incr i} {
            set w [thread::create]
            lappend ws $w
            thread::send -async $w {
                set m [tsv::get c m]
                for {set j 0} {$j < 250} {incr j} {
                    thread::mutex lock $m
                    tsv::set c n [expr {[tsv::get c n] + 1}]
                    thread::mutex unlock $m
                }
            }
        }
        foreach w $ws { thread::send $w {expr 1}; thread::release $w }
        thread::mutex destroy [tsv::get c m]
        tsv::get c n
    ";
    assert_eq!(result(src), "1000");
}

#[test]
fn cond_wait_wakes_on_notify_with_a_predicate_loop() {
    // The canonical wait-in-a-predicate-loop idiom: a worker waits on the cond
    // (releasing the mutex) until a shared flag is set; the short timeout makes
    // it robust against a notify that races the wait.
    let src = "
        tsv::set f m [thread::mutex create]
        tsv::set f c [thread::cond create]
        tsv::set f go 0
        tsv::set f done 0
        set w [thread::create]
        thread::send -async $w {
            set m [tsv::get f m]
            set c [tsv::get f c]
            thread::mutex lock $m
            while {[tsv::get f go] == 0} { thread::cond wait $c $m 100 }
            thread::mutex unlock $m
            tsv::set f done 1
        }
        tsv::set f go 1
        thread::cond notify [tsv::get f c]
        thread::send $w {expr 1}
        thread::release $w
        thread::mutex destroy [tsv::get f m]
        thread::cond destroy [tsv::get f c]
        tsv::get f done
    ";
    assert_eq!(result(src), "1");
}

#[test]
fn cond_wait_times_out_without_a_notify() {
    // A bounded wait with no notify returns (does not hang), leaving the mutex
    // relocked so the following unlock succeeds.
    assert_eq!(
        result(
            "set m [thread::mutex create]; set c [thread::cond create]; \
             thread::mutex lock $m; thread::cond wait $c $m 20; thread::mutex unlock $m; \
             set ok woke"
        ),
        "woke"
    );
}

#[test]
fn rwmutex_write_lock_guards_a_non_atomic_increment() {
    // A write lock is exclusive: the same non-atomic-increment total holds.
    let src = "
        tsv::set c n 0
        tsv::set c rw [thread::rwmutex create]
        set ws {}
        for {set i 0} {$i < 4} {incr i} {
            set w [thread::create]
            lappend ws $w
            thread::send -async $w {
                set rw [tsv::get c rw]
                for {set j 0} {$j < 200} {incr j} {
                    thread::rwmutex wlock $rw
                    tsv::set c n [expr {[tsv::get c n] + 1}]
                    thread::rwmutex unlock $rw
                }
            }
        }
        foreach w $ws { thread::send $w {expr 1}; thread::release $w }
        thread::rwmutex destroy [tsv::get c rw]
        tsv::get c n
    ";
    assert_eq!(result(src), "800");
}

#[test]
fn mutex_missing_handle_errors() {
    assert_eq!(
        result("catch {thread::mutex lock nope} e; set e"),
        "mutex \"nope\" does not exist"
    );
}

// ===========================================================================
// tpool::* — worker pools
// ===========================================================================

#[test]
fn tpool_posts_and_collects_results() {
    // Two jobs on a pool; `tpool::get` blocks until each is done.
    assert_eq!(
        result(
            "set p [tpool::create -maxworkers 2]; \
             set j1 [tpool::post $p {expr {2 * 21}}]; \
             set j2 [tpool::post $p {string length hello}]; \
             set r1 [tpool::get $p $j1]; set r2 [tpool::get $p $j2]; \
             tpool::release $p; list $r1 $r2"
        ),
        "42 5"
    );
}

#[test]
fn tpool_worker_state_survives_across_jobs_via_initcmd() {
    // `-initcmd` runs once per worker; with a single worker, a variable it sets
    // is visible to every job that worker runs.
    assert_eq!(
        result(
            "set p [tpool::create -maxworkers 1 -initcmd {set base 100}]; \
             set j [tpool::post $p {incr base 7}]; \
             set r [tpool::get $p $j]; tpool::release $p; set r"
        ),
        "107"
    );
}

#[test]
fn tpool_wait_reports_done_and_pending() {
    // `tpool::wait` blocks until the job finishes, returns it as done, and stores
    // the (now empty) pending list in the var.
    assert_eq!(
        result(
            "set p [tpool::create -maxworkers 2]; \
             set j [tpool::post $p {expr {3 + 4}}]; \
             set done [tpool::wait $p [list $j] pending]; \
             set r [tpool::get $p $j]; tpool::release $p; \
             list [expr {[lindex $done 0] == $j}] $pending $r"
        ),
        "1 {} 7"
    );
}

#[test]
fn tpool_job_error_propagates_through_get() {
    assert_eq!(
        result(
            "set p [tpool::create -maxworkers 1]; \
             set j [tpool::post $p {error boom}]; \
             catch {tpool::get $p $j} e; tpool::release $p; set e"
        ),
        "boom"
    );
}

#[test]
fn tpool_detached_job_runs_without_a_result_id() {
    // A detached post returns empty and still runs (observable via tsv).
    assert_eq!(
        result(
            "set p [tpool::create -maxworkers 1]; \
             tsv::set r done 0; \
             set id [tpool::post -detached $p {tsv::set r done 1}]; \
             while {[tsv::get r done] == 0} {}; \
             tpool::release $p; list [string length $id] [tsv::get r done]"
        ),
        "0 1"
    );
}

#[test]
fn tpool_names_lists_live_pools() {
    assert_eq!(
        result("set p [tpool::create]; set n [tpool::names]; tpool::release $p; set n"),
        "tpool0"
    );
}
