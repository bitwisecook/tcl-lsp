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

//! Minimal VM script evaluator (no tcltest, no traces) — for isolating VM bugs.
//! Usage: `echo '<script>' | cargo run -p tcl-vm --example eval`

use std::io::Read;

use tcl_compiler::cfg_builder::build_cfg_codegen as build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_compiler::lowering::lower_to_ir_traced;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

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
    fn compile_traced(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir_traced(src, &self.0);
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
