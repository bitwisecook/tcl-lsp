//! Integration: the real Tcl 9 `tcltest.tcl` library loads end-to-end.
//!
//! This is the M4 milestone — sourcing the genuine
//! `tmp/tcl9.0.3/library/tcltest/tcltest.tcl` exercises namespaces, namespace
//! variables (scalar + array), `package`, variable traces, `file`/`pwd`/`cd`,
//! `regexp`, `subst`, dynamic proc bodies, and `upvar` to namespace variables.
//! The test is skipped when the Tcl source tree isn't present.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct Svc(CommandRegistry);
impl CompileService for Svc {
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, &self.0);
        let cfg = build_cfg(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.0))
    }
}

#[derive(Clone)]
struct Cap(Rc<RefCell<Vec<u8>>>);
impl Write for Cap {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn tcltest_library_loads() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tmp/tcl9.0.3/library/tcltest/tcltest.tcl"
    );
    if !std::path::Path::new(path).exists() {
        eprintln!("skipping: {path} not present");
        return;
    }

    // Run on a worker thread with a watchdog so a regression can't hang CI.
    let handle = std::thread::spawn(move || {
        let registry = CommandRegistry::build_default();
        let src = format!("source {path}\nputs LOADED-OK\n");
        let ir = lower_to_ir(&src, &registry);
        let cfg = build_cfg(&ir, false);
        let asm = codegen_module(&cfg, &ir, &registry);
        let buf = Rc::new(RefCell::new(Vec::new()));
        let mut vm = Vm::with_output(Box::new(Cap(Rc::clone(&buf))));
        vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
        let c = vm.run_module(&asm);
        let out = String::from_utf8_lossy(&buf.borrow()).into_owned();
        (c.code.is_ok(), c.result.to_str().to_string(), out)
    });

    // Poll for completion up to a generous bound.
    for _ in 0..200 {
        if handle.is_finished() {
            let (ok, result, out) = handle.join().expect("worker panicked");
            assert!(ok, "tcltest.tcl failed to load: {result}");
            assert!(
                out.contains("LOADED-OK"),
                "tcltest.tcl did not reach end of load; output: {out:?}"
            );
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("tcltest.tcl load did not finish within 20s (possible infinite loop)");
}

/// Real `::tcltest::test` cases run end-to-end: passing tests pass, a wrong
/// expected result is reported as a failure, and `cleanupTests` prints the
/// summary. Exercises namespaces, the option getters (read-trace cascade with
/// per-variable trace suppression), `uplevel`, `{*}` expansion, channels, and
/// the variable-index array access that `tcltest::test` relies on.
#[test]
fn tcltest_runs_test_cases() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tmp/tcl9.0.3/library/tcltest/tcltest.tcl"
    );
    if !std::path::Path::new(path).exists() {
        eprintln!("skipping: {path} not present");
        return;
    }

    let handle = std::thread::spawn(move || {
        let registry = CommandRegistry::build_default();
        let src = format!(
            "source {path}\n\
             namespace import ::tcltest::*\n\
             test pass-1.1 {{arith}} {{ expr {{1+1}} }} 2\n\
             test pass-1.2 {{strlen}} {{ string length hello }} 5\n\
             test fail-1.1 {{wrong}} {{ expr {{2+2}} }} 5\n\
             cleanupTests\n\
             puts ALLDONE\n"
        );
        let ir = lower_to_ir(&src, &registry);
        let cfg = build_cfg(&ir, false);
        let asm = codegen_module(&cfg, &ir, &registry);
        let buf = Rc::new(RefCell::new(Vec::new()));
        let mut vm = Vm::with_output(Box::new(Cap(Rc::clone(&buf))));
        vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
        let c = vm.run_module(&asm);
        let out = String::from_utf8_lossy(&buf.borrow()).into_owned();
        (c.code.is_ok(), c.result.to_str().to_string(), out)
    });

    for _ in 0..300 {
        if handle.is_finished() {
            let (ok, result, out) = handle.join().expect("worker panicked");
            assert!(ok, "test run errored: {result}\noutput: {out}");
            assert!(out.contains("ALLDONE"), "run did not finish; output: {out}");
            assert!(
                out.contains("fail-1.1") && out.contains("FAILED"),
                "expected fail-1.1 to be reported as a failure; output: {out}"
            );
            // The summary line records two passes and one failure.
            assert!(
                out.contains("Total\t3\tPassed\t2\tSkipped\t0\tFailed\t1"),
                "unexpected summary line; output: {out}"
            );
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("tcltest test run did not finish within 30s (possible infinite loop)");
}
