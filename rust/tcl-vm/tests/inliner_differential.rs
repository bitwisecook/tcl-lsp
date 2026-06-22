//! Execution-differential gate for the FE-OPT v3 (parameterised) inliner.
//!
//! The inliner is an optimiser: its output must be *observationally identical*
//! to the un-inlined program. IR-shape unit tests (in `tcl-compiler`) prove the
//! transform fires and α-renames correctly, but the repo standard for an
//! optimiser is an **execution** differential. This test compiles each program
//! twice — once straight, once with `inline_module` applied to the IR — runs
//! both on `tcl-vm`, and asserts the captured stdout (and ok/err status) match.
//! When `tclsh9.0` is present it is also used as an independent ground-truth
//! oracle (§0: C Tcl is the reference standard).
//!
//! Each corpus entry is chosen so the inliner actually fires (a parameterised
//! proc over a splice-safe `puts` body), so the differential is meaningful
//! rather than vacuously comparing two identical programs.

use std::cell::RefCell;
use std::io::Write;
use std::process::Command;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::inlining::inline_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
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

/// Compile + run `src` on `tcl-vm`, optionally inlining the IR first. Returns
/// `(ok, stdout)`.
fn run(src: &str, inline: bool) -> (bool, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let ir = if inline { inline_module(ir) } else { ir };
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let c = vm.run_module(&asm);
    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8");
    (c.code.is_ok(), out)
}

/// Whether `inline_module` actually rewrites `src` (the proc call disappears
/// from the lowered IR). Guards against a vacuous differential.
fn inlining_fires(src: &str) -> bool {
    let registry = CommandRegistry::build_default();
    let plain = format!("{:?}", lower_to_ir(src, &registry).top_level);
    let inlined = format!("{:?}", inline_module(lower_to_ir(src, &registry)).top_level);
    plain != inlined
}

/// Run `src` through `tclsh9.0` if available; `None` when the binary is absent.
fn tclsh(src: &str) -> Option<(bool, String)> {
    let mut child = Command::new("tclsh9.0")
        .arg("/dev/stdin")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take().unwrap().write_all(src.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    Some((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    ))
}

/// Assert inlined and un-inlined runs agree (and match `tclsh` when present),
/// and that the inliner actually fired.
fn assert_differential(src: &str) {
    assert!(
        inlining_fires(src),
        "inliner did not fire — vacuous case:\n{src}"
    );
    let (ok_plain, out_plain) = run(src, false);
    let (ok_inlined, out_inlined) = run(src, true);
    assert_eq!(
        ok_plain, ok_inlined,
        "status diverged after inlining:\n{src}"
    );
    assert_eq!(
        out_plain, out_inlined,
        "stdout diverged after inlining:\n{src}\nplain={out_plain:?} inlined={out_inlined:?}"
    );
    if let Some((ok_ref, out_ref)) = tclsh(src) {
        assert_eq!(ok_ref, ok_inlined, "status vs tclsh9.0 diverged:\n{src}");
        assert_eq!(
            out_ref, out_inlined,
            "stdout vs tclsh9.0 diverged:\n{src}\ntclsh={out_ref:?} inlined={out_inlined:?}"
        );
    }
}

#[test]
fn single_param_literal_arg() {
    assert_differential("proc ::p {x} { puts \"x=$x\" }\np hello\n");
}

#[test]
fn two_params_mixed_in_value() {
    assert_differential("proc ::p {a b} { puts \"$a-$b\" }\np foo bar\n");
}

#[test]
fn brace_substitution_form() {
    assert_differential("proc ::p {x} { puts \"<${x}>\" }\np mid\n");
}

#[test]
fn variable_argument_evaluated_in_caller() {
    // The arg `$y` must bind to the caller's `y` at the splice point.
    assert_differential("set y world\nproc ::p {x} { puts \"hi $x\" }\np $y\n");
}

#[test]
fn multi_statement_body() {
    assert_differential("proc ::p {x} { puts a\n puts $x\n puts b }\np z\n");
}

#[test]
fn repeated_calls_get_distinct_bindings() {
    // Three sites → three unique α-rename suffixes; output order preserved.
    assert_differential("proc ::p {x} { puts \">$x<\" }\np 1\np 2\np 3\n");
}

#[test]
fn inlined_inside_foreach_body() {
    assert_differential("proc ::log {m} { puts \"L:$m\" }\nforeach i {a b c} { log $i }\n");
}

#[test]
fn inlined_inside_if_body() {
    assert_differential("proc ::p {x} { puts $x }\nif {1} { p inside }\n");
}

#[test]
fn escaped_dollar_in_body_is_literal() {
    // The body's `\$x` is a literal `$x`, not a substitution — it must survive
    // α-renaming untouched (a naive rewrite would corrupt it).
    assert_differential("proc ::p {x} { puts \"lit \\$x got $x\" }\np val\n");
}

#[test]
fn param_name_substring_not_confused() {
    // `$x` and `$xy` are distinct parameters; α-renaming `x` must not corrupt
    // the `$xy` reference (name boundaries are respected, and each param gets
    // its own mangled name).
    assert_differential("proc ::p {x xy} { puts \"$x|$xy\" }\np A B\n");
}
