//! Drive a tcltest `.test` file through the VM (the bytecode executor) — the VM
//! analogue of `runtime/rust`'s `run_script --init`, for comparing tcltest
//! pass/fail/skip performance between the two runtimes.
//!
//! Usage: `TCL_LIBRARY=.../library cargo run -p tcl-vm --example run_test -- <file.test>`
//!
//! The VM has no `init.tcl` bootstrap path, so the real `tcltest.tcl` is sourced
//! directly (it `package provide`s itself, so the test file's `package require
//! tcltest` then resolves). Runs on a large-stack worker thread so deep recursion
//! is a catchable error rather than a native-stack overflow (matching the
//! `run_script` driver).

use std::io::Write;

use tcl_compiler::cfg_builder::build_cfg_codegen as build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

/// The `CompileService` the VM uses for runtime `eval` / command substitution:
/// the real Rust compiler pipeline (lower → CFG → bytecode).
struct Svc(CommandRegistry);
impl CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.0);
        let cfg = build_cfg(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.0))
    }
}

/// Pass the VM's `puts` output straight through to the process stdout.
struct Stdout;
impl Write for Stdout {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        std::io::stdout().write(b)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

const STACK_BYTES: usize = 512 * 1024 * 1024;

fn main() {
    let code = std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(run)
        .expect("spawn")
        .join()
        .expect("worker panicked");
    std::process::exit(code);
}

fn run() -> i32 {
    let Some(testfile) = std::env::args().nth(1) else {
        eprintln!("usage: run_test <file.test>");
        return 2;
    };
    let lib = std::env::var("TCL_LIBRARY").unwrap_or_default();
    let tcltest = format!("{lib}/tcltest/tcltest.tcl");

    // Source tcltest directly, then import its exported commands into the global
    // namespace. A real `.test` file's preamble does
    // `package require tcltest; namespace import -force ::tcltest::*`, but it
    // guards on `::tcltest` not already existing — which it does here — so the
    // driver performs the import the file would otherwise do.
    let registry = CommandRegistry::build_default();
    let src =
        format!("source {tcltest}\nnamespace import -force ::tcltest::*\nsource {testfile}\n");
    let ir = lower_to_ir(&src, &registry);
    let cfg = build_cfg(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let mut vm = Vm::with_output(Box::new(Stdout));
    vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
    // Install the autoloader so library procs (word.tcl, …) resolve on demand,
    // like `tclsh` after `init.tcl`.
    let init = vm.init_auto_load();
    if !init.code.is_ok() {
        eprintln!("auto-load bootstrap error: {}", init.result.to_str());
    }
    let c = vm.run_module(&asm);
    if c.code.is_ok() {
        0
    } else {
        eprintln!("VM error: {}", c.result.to_str());
        1
    }
}
