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

//! End-to-end coverage of the control-flow command implementations:
//! `try`/`throw` (`cmd_try.rs`), the `if`/`while`/`for`/`foreach`/`lmap`
//! runtime fallbacks (`cmd_control.rs`), and `switch`'s runtime path
//! (`cmd_switch.rs`).
//!
//! Each test compiles real Tcl via `tcl-compiler` and runs it through `tcl-vm`,
//! asserting the observable behaviour (result value, captured `puts` output, or
//! caught error message). Every expected value is the output of real `tclsh`
//! (8.6 and 9.0, which agree on all of these) — differential testing with tclsh
//! as the oracle. The `// tclsh:` comments cite that reference output.
//!
//! DRIVING THE RUNTIME PATH. The compiler inlines the static literal forms of
//! these constructs (e.g. `if {...} {...}`, `switch x {...}`, and — see the BUG
//! note on `try_literal_form_*` below — even some `try` forms compile to the
//! unimplemented `beginCatch` opcode). To exercise the *runtime command*
//! implementations in the three target files, every test invokes the command
//! through a computed name (`set c if; $c ...`), which defeats the inline fast
//! path and routes through the registered builtin. This is the same technique
//! the existing `run_script.rs`/`builtins_e2e.rs` suites use for fallback paths.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_codegen::codegen::codegen_module;
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_compiler::cfg_builder::build_cfg_codegen;
// Compile the way the real VM runtime does (`tcl-vm-cli`): the bytecode lowering
// turns constructs the backend can't compile correctly — a literal `try` with
// handlers, nested complex `foreach`/`lmap` — into runtime-command barriers,
// where the analysis-oriented `lower_to_ir` would instead emit `beginCatch`
// exception ranges the VM does not implement. Using the analysis lowering here
// would test bytecode the VM never actually runs.
use tcl_compiler::lowering::lower_to_ir_for_bytecode as lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::{CompileError, CompileService, ScriptCommandPlan};

mod catch_bodies;
mod deferred_parse_errors;
mod foreach_lmap;
mod if_while_for;
mod inline_compiled_forms;
mod switch_command;
mod try_throw;

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
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = tcl_vm::test_support::native_vm_with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let completion = vm.try_eval_source(src).expect("the VM's own compile entry");

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

/// Runtime compiler which deliberately emits ordinary command dispatch for
/// every dynamic script. This isolates the generic/plain `catch` and `try`
/// path without relying on execution-trace side effects to select it.
struct PlainRuntimeCompiler(BytecodeCompileService);

impl CompileService for PlainRuntimeCompiler {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0
            .compile_plain_script_bytes_for_profile(target, profile)
    }
    fn compile_substitution_with_entry(
        &self,
        target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0
            .compile_substitution_with_entry(target, profile, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0
            .compile_plain_script_bytes_for_profile(target, profile)
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        _dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0.compile_procedure_bytes_for_profile(
            target,
            profile,
            tcl_runtime_api::ProcedureDispatch::Plain,
        )
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        _dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0.compile_procedure_bytes_with_entry(
            target,
            profile,
            entry,
            tcl_runtime_api::ProcedureDispatch::Plain,
        )
    }
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        self.0
            .script_command_plan_bytes_for_profile(source, profile)
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        self.0
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.0.compile_traced(src)
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.0.compile_traced_for_profile(src, profile)
    }

    fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.0.compile_traced(src)
    }

    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.0.compile_traced_for_profile(src, profile)
    }

    fn script_command_plan_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<ScriptCommandPlan, CompileError> {
        self.0.script_command_plan_for_profile(src, profile)
    }
}

fn run_with_plain_runtime_compiler(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry).expect("original source namespace");

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = tcl_vm::test_support::native_vm_with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(PlainRuntimeCompiler(
        BytecodeCompileService::default(),
    )));
    let completion = vm.run_module(&asm);
    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}
