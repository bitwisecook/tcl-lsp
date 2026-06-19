//! Minimal VM script evaluator (no tcltest, no traces) — for isolating VM bugs.
//! Usage: `echo '<script>' | cargo run -p tcl-vm --example eval`

use std::io::Read;

use tcl_compiler::cfg_builder::build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct Svc(CommandRegistry);
impl CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, &self.0);
        let cfg = build_cfg(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.0))
    }
}

fn main() {
    let mut src = String::new();
    std::io::stdin().read_to_string(&mut src).expect("stdin");
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(&src, &registry);
    let cfg = build_cfg(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    if std::env::var("DIS").is_ok() {
        eprintln!(
            "=== ::top ===\n{}",
            tcl_compiler::codegen::format::format_function_asm(&asm.top_level)
        );
        for (name, f) in &asm.procedures {
            eprintln!(
                "=== proc {name} ===\n{}",
                tcl_compiler::codegen::format::format_function_asm(f)
            );
        }
    }
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
    let c = vm.run_module(&asm);
    if c.code.is_ok() {
        println!("OK: {}", c.result.to_str());
    } else {
        println!("ERR: {}", c.result.to_str());
    }
}
