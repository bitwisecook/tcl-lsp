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

//! End-to-end: compile real Tcl to bytecode (via `tcl-compiler`, dev-dep only)
//! then run it through `tcl-vm`, asserting result + captured `puts` output.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_codegen::codegen::codegen_module;
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, Commands, CompileService, Traces, Value, Vm};

mod arithmetic;
mod control_flow;
mod inlined_catch;
mod introspection;
mod lists_and_dicts;
mod procedure_bodies;
mod strings_and_binary;
mod traces_and_errors;
mod word_substitution;

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

/// The suite's release: the default VM's Tcl 9.0.
const RELEASE: tcl_dialect::TclVersion = tcl_dialect::TclVersion::V9_0;

/// Compile `src` for the release it runs on, and a VM of that release.
fn compiled_for_release(src: &str, out: Box<dyn Write>) -> (tcl_bytecode::ModuleAsm, tcl_vm::Vm) {
    // A unit compiled without a Tcl version selects no native plan.
    let profile = tcl_registry::model::ingress::resolve_environment(RELEASE.dialect_name())
        .analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let asm = tcl_vm::CompileService::compile_for_profile(&service, src, profile)
        .expect("test script compiles for its selected profile");
    let mut vm = tcl_vm::test_support::native_vm_for_release(RELEASE, out);
    vm.set_compiler(Box::new(service));
    (asm, vm)
}

/// Compile and run `src`; return `(ok, result-string, captured-stdout)`.
fn run(src: &str) -> (bool, String, String) {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let (asm, mut vm) = compiled_for_release(src, Box::new(Capture(Rc::clone(&buf))));
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

/// Compile through the shipping `TclVM` lowering path. Most historical tests in
/// this file predate that split and intentionally retain `run`; deferred
/// control-flow regressions must exercise the runtime-call shapes.
fn run_bytecode(src: &str) -> (bool, String, String) {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let (asm, mut vm) = compiled_for_release(src, Box::new(Capture(Rc::clone(&buf))));
    let completion = vm.run_module(&asm);
    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

/// `Commands::dispatch` runs a compiled proc to completion — the nested
/// `is_proc` activation path (`invoke_command` pushes the call-frame, runs the
/// body, and absorbs `return` into an ok completion), and surfaces the proc's
/// arg-count usage error when binding fails.
#[test]
fn commands_dispatch_runs_proc() {
    let registry = CommandRegistry::build_default();
    let asm = {
        let src = "proc add {a b} { return [expr {$a + $b}] }";
        let ir = lower_to_ir(src, &registry);
        let cfg = build_cfg_codegen(&ir, false);
        codegen_module(&cfg, &ir, &registry).expect("original source namespace")
    };

    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    // Running the module registers `add`.
    assert!(vm.run_module(&asm).code.is_ok());

    // Dispatch the proc by name with name-stripped argv.
    let c = vm
        .dispatch("add", &[Value::string("3"), Value::string("4")])
        .unwrap();
    assert_eq!(c.code, Code::Ok);
    assert_eq!(&*c.result.to_str(), "7");

    // Wrong arg count surfaces the proc usage error (the `enter_proc` Err path).
    let c = vm.dispatch("add", &[Value::string("3")]).unwrap();
    assert_eq!(c.code, Code::Error);
    assert_eq!(&*c.result.to_str(), "wrong # args: should be \"add a b\"");
}

/// A `PUSH1 idx` that pushes literal `idx` verbatim (no word substitution) — the
/// form the codegen uses for braced/constant words. Needed so a regexp pattern
/// like `^[0-9]+$` is pushed as data, not run through `subst_word`.
fn pushv(idx: i32) -> tcl_bytecode::Instruction {
    use tcl_bytecode::{Instruction, Op, Operand};
    let mut i = Instruction::new(Op::PUSH1, vec![Operand::Imm(idx)]);
    i.push_verbatim = true;
    i
}

/// Run a hand-assembled top-level instruction stream (with its literal pool),
/// returning the result string. `DONE` yields the value on top of the stack, so
/// the caller ends the stream with the opcode under test. This drives the VM's
/// opcode dispatch directly — unlike [`run`], it does not go through the codegen,
/// so it exercises an opcode even when no codegen path emits it yet.
fn run_asm(literals: &[&str], instrs: Vec<tcl_bytecode::Instruction>) -> String {
    use tcl_bytecode::{FunctionAsm, LiteralTable, LocalVarTable, ModuleAsm};
    let mut lits = LiteralTable::new();
    for l in literals {
        lits.intern(l);
    }
    let mut instrs = instrs;
    let labels =
        tcl_bytecode::layout::resolve_layout(&mut instrs, &std::collections::HashMap::new());
    let top = FunctionAsm {
        name: "::top".to_string(),
        literals: lits,
        lvt: LocalVarTable::new(&[]),
        instructions: instrs,
        labels,
        loop_targets: std::collections::HashMap::new(),
        body_base_line: 0,
        native_compilation_failure: None,
        native_compilation_preflight: tcl_runtime_api::NativeCompilationPreflight::NotRequired,
        native_math_table_prerequisite: None,
        native_compiler_prerequisites: Vec::new(),
        required_compiled_local_layout: None,
        proc_body_src: None,
        error_regions: Vec::new(),
        splice_ranges: Vec::new(),
        plain_command_dispatch: false,
        command_bindings: Vec::new(),
        procedure_bindings: Vec::new(),
        site_claims: Vec::new(),
    };
    let module = ModuleAsm {
        profile: tcl_dialect::DialectProfile::plain_tcl(),
        source: tcl_lexer::SourceImage::default(),
        source_namespace: tcl_runtime_api::ByteNamespacePath::root(),
        plain_command_dispatch: false,
        top_level: top,
        top_level_body: FunctionAsm::default(),
        procedures: std::collections::HashMap::new(),
        procedure_provenance: std::collections::HashMap::new(),
        manifest: None,
    };
    let mut vm = tcl_vm::test_support::native_vm();
    vm.run_module(&module).result.to_str().to_string()
}
