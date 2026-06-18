//! End-to-end: compile real Tcl to bytecode (via `tcl-compiler`, dev-dep only)
//! then run it through `tcl-vm`, asserting result + captured `puts` output.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

/// A `tcl-compiler`-backed compile service so the VM can resolve runtime
/// `eval` / `[command substitution]` (the injection seam — `tcl-vm` itself
/// never depends on the compiler).
struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
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

#[test]
fn set_expr_puts() {
    let (ok, _result, out) = run("set x 5\nputs [expr {$x * 2}]\n");
    assert!(ok);
    assert_eq!(out, "10\n");
}

#[test]
fn expr_precedence() {
    let (ok, result, _out) = run("expr {3 + 4 * 2}");
    assert!(ok);
    assert_eq!(result, "11");
}

#[test]
fn floored_integer_division() {
    let (ok, result, _out) = run("expr {-7 / 2}");
    assert!(ok);
    assert_eq!(result, "-4");
}

#[test]
fn incr_and_var_substitution() {
    let (ok, _result, out) = run("set n 0\nincr n\nincr n 5\nputs $n\n");
    assert!(ok);
    assert_eq!(out, "6\n");
}

#[test]
fn string_and_numeric_compare() {
    let (ok, result, _out) = run("expr {9 < 10}");
    assert!(ok);
    assert_eq!(result, "1");
}
