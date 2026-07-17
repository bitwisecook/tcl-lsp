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

//! WASM **value**-differential arm: run a generated program's compiled WASM
//! (its *control flow*) with leaf commands and conditions evaluated by an
//! embedded, persistent `tcl-vm`, and compare the captured output against
//! running the *same* program directly on `tcl-vm`.
//!
//! Both sides evaluate the actual Tcl commands with `tcl-vm`, so command
//! semantics are held constant; the **only** difference is whether control flow
//! (`if`/`while`/`for`/`break`/`continue`/`return`, branch conditions) comes
//! from the WASM emitter's structured codegen or from `tcl-vm`'s normal
//! execution. A divergence therefore isolates a **WASM control-flow
//! miscompile** — a real value differential for the part the eval-fallback
//! emitter actually compiles, with no `tclsh` needed and no confounding from
//! `tcl-vm` command bugs (those are caught by the main `tclvm`-vs-`tclsh` arm).
//!
//! The embedded host satisfies the eval-fallback ABI: `tcl_obj_new_string`
//! interns a command/condition string read from the module's linear memory,
//! `tcl_eval_code` runs it via `Vm::eval_source` (side effects — `puts` — flow to
//! the captured sink) and returns the command's **completion code**, and
//! `tcl_expr_bool` evaluates a condition via `Vm::eval_expr`. State (variables,
//! procs) persists across calls in the one `Vm`. Returning the real code lets the
//! emitted control flow honour an `error`/`return`/`break`/`continue` a leaf
//! command completes with — the same code the direct `tcl-vm` run acts on — so
//! this arm now covers abrupt-completion propagation (`RUST_ISSUE_010`), not just
//! branch/iteration shape.
//!
//! This is the in-process upgrade of the runnability arm (`wasm.rs`): it embeds
//! `wasmtime` rather than shelling out, so it can back the host with a live
//! interpreter.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::codegen::wasm::wasm_codegen_module;
use tcl_compiler::lowering::{lower_to_ir, lower_to_ir_traced};
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, CompileError, CompileService, Vm};
use wasmtime::{Caller, Config, Engine, Linker, Memory, MemoryType, Module, Store, Trap};

/// Instruction budget for a single WASM-driven run. Generated programs have
/// literal-bounded loops nested to a few levels, so a legitimate run consumes
/// well under this; exhausting it means the WASM control flow failed to
/// terminate (a real divergence, since the direct `tcl-vm` run does terminate).
const FUEL_BUDGET: u64 = 200_000_000;

/// How a value-differential check resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffVerdict {
    /// WASM-driven output matched direct `tcl-vm` execution (status + stdout).
    Match,
    /// Output or error-status diverged — a WASM control-flow miscompile.
    Divergence {
        /// Output captured while the WASM module drove control flow.
        wasm: String,
        /// Output from running the program directly on `tcl-vm`.
        direct: String,
    },
    /// The module could not be compiled / instantiated / invoked (codegen or
    /// embedder error) — recorded separately from a value divergence.
    Unrunnable(String),
    /// The WASM-driven run exhausted its instruction budget — non-termination
    /// in the compiled control flow (the direct `tcl-vm` run does terminate).
    WasmHang,
}

/// The `Write` sink that captures a `Vm`'s `puts` output.
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

/// The `CompileService` the embedded `Vm` uses for `eval_source`.
struct Svc {
    registry: CommandRegistry,
}
impl CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
    fn compile_traced(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir_traced(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

/// The wasmtime store's host data: the live interpreter, the string-handle
/// table, the imported memory, and an error latch.
struct HostState {
    vm: Vm,
    handles: Vec<String>,
    memory: Option<Memory>,
    errored: bool,
}

/// Build a fresh `Vm` wired with a compiler and an output capture.
fn fresh_vm() -> (Vm, Rc<RefCell<Vec<u8>>>) {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(Svc {
        registry: CommandRegistry::build_default(),
    }));
    (vm, buf)
}

/// Run `src` directly on `tcl-vm`; return `(errored, stdout)`.
fn run_direct(src: &str) -> (bool, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let (mut vm, buf) = fresh_vm();
    let comp = vm.run_module(&asm);
    let out = String::from_utf8_lossy(&buf.borrow()).into_owned();
    (comp.code == Code::Error, out)
}

/// Outcome of a WASM-driven run.
struct WasmRun {
    errored: bool,
    out: String,
    hung: bool,
}

/// Drive `src`'s compiled WASM control flow with an embedded `tcl-vm` host;
/// return the run outcome or an `Unrunnable` reason. Execution is fuel-bounded
/// so a non-terminating compiled loop traps (`hung`) rather than wedging.
fn run_wasm(engine: &Engine, src: &str) -> Result<WasmRun, String> {
    // Compile the program to the eval-fallback WASM module (catch a codegen
    // panic rather than abort the campaign).
    let wasm_bytes = {
        let src = src.to_owned();
        std::panic::catch_unwind(move || {
            let registry = CommandRegistry::build_default();
            let module = lower_to_ir(&src, &registry);
            wasm_codegen_module(&module, &src).to_bytes()
        })
        .map_err(|_| "wasm codegen panicked".to_owned())?
    };
    let module = Module::new(engine, &wasm_bytes).map_err(|e| format!("module: {e}"))?;

    let (vm, buf) = fresh_vm();
    let mut store = Store::new(
        engine,
        HostState {
            vm,
            handles: Vec::new(),
            memory: None,
            errored: false,
        },
    );
    // A generous 32-page (2 MiB) memory covers the data segments whether they
    // sit low or at the runtime's reserved base; min >= the module's import.
    store
        .set_fuel(FUEL_BUDGET)
        .map_err(|e| format!("fuel: {e}"))?;
    let memory =
        Memory::new(&mut store, MemoryType::new(32, None)).map_err(|e| format!("mem: {e}"))?;
    store.data_mut().memory = Some(memory);

    let mut linker = Linker::new(engine);
    linker
        .define(&store, "tcl", "memory", memory)
        .map_err(|e| format!("link mem: {e}"))?;
    linker
        .func_wrap("tcl", "tcl_obj_new_string", host_obj_new_string)
        .and_then(|l| l.func_wrap("tcl", "tcl_eval_code", host_eval_code))
        .and_then(|l| l.func_wrap("tcl", "tcl_expr_bool", host_expr_bool))
        .map_err(|e| format!("link funcs: {e}"))?;

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("instantiate: {e}"))?;
    let top = instance
        .get_typed_func::<(), ()>(&mut store, "::top")
        .map_err(|e| format!("no ::top: {e}"))?;
    let (trapped, hung) = match top.call(&mut store, ()) {
        Ok(()) => (false, false),
        Err(e) => {
            let hung = e.downcast_ref::<Trap>() == Some(&Trap::OutOfFuel);
            (!hung, hung)
        }
    };

    let errored = trapped || store.data().errored;
    let out = String::from_utf8_lossy(&buf.borrow()).into_owned();
    Ok(WasmRun { errored, out, hung })
}

/// `tcl_obj_new_string(ptr, len)` — intern a string read from linear memory.
fn host_obj_new_string(caller: Caller<'_, HostState>, ptr: i32, len: i32) -> i32 {
    let mut caller = caller;
    let Some(mem) = caller.data().memory else {
        return -1;
    };
    let (ptr, len) = (
        usize::try_from(ptr).unwrap_or(0),
        usize::try_from(len).unwrap_or(0),
    );
    let s = {
        let data = mem.data(&caller);
        data.get(ptr..ptr.saturating_add(len))
            .map_or_else(String::new, |b| String::from_utf8_lossy(b).into_owned())
    };
    let st = caller.data_mut();
    let h = st.handles.len();
    st.handles.push(s);
    i32::try_from(h).unwrap_or(-1)
}

/// Look up a handle's interned string (empty for an out-of-range handle).
fn handle_str(st: &HostState, h: i32) -> String {
    usize::try_from(h)
        .ok()
        .and_then(|i| st.handles.get(i))
        .cloned()
        .unwrap_or_default()
}

/// `tcl_eval_code(handle) -> i32` — run the command in the persistent interpreter
/// and return its completion code (`0` ok … `4` continue, or a `return -code N`),
/// so the emitted control flow can honour an abrupt completion. An ordinary error
/// latches `errored` (the run mismatches on error status); a propagating
/// `break`/`continue`/`return` escaping a command substitution is not an error.
fn host_eval_code(caller: Caller<'_, HostState>, h: i32) -> i32 {
    let mut caller = caller;
    let cmd = handle_str(caller.data(), h);
    let st = caller.data_mut();
    match st.vm.eval_source(&cmd) {
        Ok(comp) => {
            if comp.code == Code::Error {
                st.errored = true;
            }
            i32::try_from(comp.code.as_int()).unwrap_or(1)
        }
        Err(e) => match e.code {
            // A non-`Error` code carried out of a substitution propagates as-is.
            Some(c) if c != Code::Error => i32::try_from(c.as_int()).unwrap_or(1),
            _ => {
                st.errored = true;
                1
            }
        },
    }
}

/// `tcl_expr_bool(handle)` — evaluate the condition; `1` true, `0` false.
fn host_expr_bool(caller: Caller<'_, HostState>, h: i32) -> i32 {
    let mut caller = caller;
    let expr = handle_str(caller.data(), h);
    let st = caller.data_mut();
    // A non-numeric / unparseable condition is an error, latched and reported
    // as false (the run will mismatch on error status, not silently continue).
    match st.vm.eval_expr(&expr).ok().and_then(|v| v.as_bool().ok()) {
        Some(true) => 1,
        Some(false) => 0,
        None => {
            st.errored = true;
            0
        }
    }
}

/// Value-differential one program: WASM-driven vs direct `tcl-vm`.
#[must_use]
pub fn check(engine: &Engine, src: &str) -> DiffVerdict {
    let wasm = match run_wasm(engine, src) {
        Ok(r) => r,
        Err(reason) => return DiffVerdict::Unrunnable(reason),
    };
    if wasm.hung {
        return DiffVerdict::WasmHang;
    }
    let (wasm_err, wasm_out) = (wasm.errored, wasm.out);
    let (direct_err, direct_out) = run_direct(src);
    // Compare stdout only when neither errored (a partial errored stdout isn't
    // meaningfully comparable); always compare error status.
    if wasm_err != direct_err || (!wasm_err && wasm_out != direct_out) {
        return DiffVerdict::Divergence {
            wasm: wasm_out,
            direct: direct_out,
        };
    }
    DiffVerdict::Match
}

/// A reusable engine for a campaign (compiling many modules), with fuel metering
/// enabled so a non-terminating compiled program traps instead of wedging.
#[must_use]
pub fn engine() -> Engine {
    let mut config = Config::new();
    config.consume_fuel(true);
    Engine::new(&config).expect("fuel-metering engine")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_line_matches() {
        let e = engine();
        assert_eq!(
            check(&e, "puts hello\nputs world\n"),
            DiffVerdict::Match,
            "straight-line output must agree"
        );
    }

    #[test]
    fn if_branch_matches() {
        let e = engine();
        // The WASM `if` must take the same branch tcl-vm does.
        assert_eq!(
            check(
                &e,
                "set x 5\nif {$x > 3} { puts big } else { puts small }\n"
            ),
            DiffVerdict::Match
        );
        assert_eq!(
            check(
                &e,
                "set x 1\nif {$x > 3} { puts big } else { puts small }\n"
            ),
            DiffVerdict::Match
        );
    }

    #[test]
    fn loop_iteration_matches() {
        let e = engine();
        assert_eq!(
            check(&e, "for {set i 0} {$i < 3} {incr i} { puts $i }\n"),
            DiffVerdict::Match,
            "WASM loop must iterate like tcl-vm"
        );
    }

    #[test]
    fn while_with_break_matches() {
        let e = engine();
        let src = "set i 0\nwhile {1} { incr i\n if {$i >= 2} { break }\n puts $i }\n";
        assert_eq!(check(&e, src), DiffVerdict::Match);
    }

    // --- completion-code propagation (RUST_ISSUE_010) ---------------------
    // Each of these terminates and matches *because* a leaf command's abrupt
    // completion code is honoured by the emitted control flow. Under the prior
    // "swallow the code" behaviour they would `WasmHang` (the `while {1}` never
    // exits) or `Divergence` (dead code runs) — so they lock in the fix.

    #[test]
    fn error_in_loop_unwinds_not_hangs() {
        let e = engine();
        // `error` inside `while {1}`: the compiled loop must stop and propagate,
        // not iterate forever. Both sides end errored → Match (a swallow hangs).
        let src = "set i 0\nwhile {1} { incr i\n if {$i == 3} { error boom }\n puts $i }\n";
        assert_eq!(check(&e, src), DiffVerdict::Match);
    }

    #[test]
    fn return_in_loop_stops_the_script() {
        let e = engine();
        // Top-level `return` from inside `while {1}` unwinds `::top`; the output
        // is compared (neither side errors), so a swallow would hang or diverge.
        let src = "set i 0\nwhile {1} { incr i\n if {$i == 3} { return }\n puts $i }\n";
        assert_eq!(check(&e, src), DiffVerdict::Match);
    }

    #[test]
    fn dynamic_break_exits_the_loop() {
        let e = engine();
        // A `break` reached through a *called command* (`eval break`, not a
        // literal `break`, so it flows through `tcl_eval_code` as a `break`
        // completion — and `eval` is not a break boundary) must still exit the
        // enclosing compiled loop.
        let src = "set i 0\nwhile {1} { incr i\n puts $i\n eval break }\n";
        assert_eq!(check(&e, src), DiffVerdict::Match);
    }

    #[test]
    fn dynamic_continue_skips_the_iteration() {
        let e = engine();
        // A `continue` completion from a called command (`eval continue`) re-enters
        // the loop step, so the guarded `puts` is skipped for that iteration only
        // — output must match direct execution (a swallow would run the skipped
        // `puts`, and `eval` is not a continue boundary so it genuinely propagates).
        let src = "for {set i 0} {$i < 4} {incr i} { if {$i == 1} { eval continue }\n puts $i }\n";
        assert_eq!(check(&e, src), DiffVerdict::Match);
    }
}
