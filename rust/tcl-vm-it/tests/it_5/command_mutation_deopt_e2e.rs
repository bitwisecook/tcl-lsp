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

//! Runtime command-mutation invalidation.
//!
//! Tcl 9.0.4 is the oracle. Its command compile epoch makes a replacement of a
//! byte-compiled builtin visible across separately compiled proc/eval units;
//! the VM mirrors that by switching future units to ordinary dispatch.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use tcl_codegen::codegen::codegen_module;
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_codegen::inlining::inline_module;
use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::lowering::lower_to_ir_traced_with_dialect;
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_lexer::LexerConfig;
use tcl_vm::{
    Code, Commands, CompileError, CompileService, Completion, FunctionHandle, NativeCommand,
    ProcedureCompileTarget, ProcedureDispatch, ScriptCompileTarget, Value, Vm,
};

mod argument_substitution_mutation;
mod binding_revalidation;
mod compiler_swap;
mod inlined_procedures;
mod manifests;
mod plain_compiler_capability;
mod recompilation;
mod reusable_artifacts;
mod stale_replay;

fn sole_compiled_procedure(module: &tcl_bytecode::ModuleAsm) -> &tcl_bytecode::FunctionAsm {
    assert_eq!(
        module.procedures.len(),
        1,
        "fixture must compile one original procedure"
    );
    module
        .procedures
        .values()
        .next()
        .expect("one compiler-issued body")
}

struct OptimisedOnlyCompilerSvc(BytecodeCompileService);

struct CountingCompilerSvc {
    inner: BytecodeCompileService,
    fast_calls: Rc<Cell<usize>>,
    plain_calls: Rc<Cell<usize>>,
}

impl CompileService for OptimisedOnlyCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.0.compile_script_bytes_for_profile(target, profile)
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
            .compile_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "plain script compilation unavailable".into(),
        ))
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "plain script compilation unavailable".into(),
        ))
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        if dispatch == tcl_runtime_api::ProcedureDispatch::Plain {
            return Err(tcl_runtime_api::CompileError::Unsupported(
                "plain procedure compilation unavailable".into(),
            ));
        }
        self.0
            .compile_procedure_bytes_for_profile(target, profile, dispatch)
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        if dispatch == tcl_runtime_api::ProcedureDispatch::Plain {
            return Err(tcl_runtime_api::CompileError::Unsupported(
                "plain procedure compilation unavailable".into(),
            ));
        }
        self.0
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
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
        self.0.compile(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        if dispatch == ProcedureDispatch::Plain {
            return Err(CompileError::Unsupported(
                "plain procedure compilation unavailable".into(),
            ));
        }
        self.0
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

impl CompileService for CountingCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.fast_calls.set(self.fast_calls.get() + 1);
        self.inner.compile_script_bytes_for_profile(target, profile)
    }
    fn compile_substitution_with_entry(
        &self,
        target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.fast_calls.set(self.fast_calls.get() + 1);
        self.inner
            .compile_substitution_with_entry(target, profile, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.fast_calls.set(self.fast_calls.get() + 1);
        self.inner
            .compile_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.plain_calls.set(self.plain_calls.get() + 1);
        self.inner
            .compile_plain_script_bytes_for_profile(target, profile)
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.plain_calls.set(self.plain_calls.get() + 1);
        self.inner
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        match dispatch {
            tcl_runtime_api::ProcedureDispatch::Optimised => {
                self.fast_calls.set(self.fast_calls.get() + 1);
            }
            tcl_runtime_api::ProcedureDispatch::Plain => {
                self.plain_calls.set(self.plain_calls.get() + 1);
            }
        }
        self.inner
            .compile_procedure_bytes_for_profile(target, profile, dispatch)
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        match dispatch {
            tcl_runtime_api::ProcedureDispatch::Optimised => {
                self.fast_calls.set(self.fast_calls.get() + 1);
            }
            tcl_runtime_api::ProcedureDispatch::Plain => {
                self.plain_calls.set(self.plain_calls.get() + 1);
            }
        }
        self.inner
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
    }
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        self.inner
            .script_command_plan_bytes_for_profile(source, profile)
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        self.inner
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile_script_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        self.fast_calls.set(self.fast_calls.get() + 1);
        self.inner.compile_script_with_entry(target, profile, entry)
    }

    fn compile_procedure_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        match dispatch {
            ProcedureDispatch::Optimised => self.fast_calls.set(self.fast_calls.get() + 1),
            ProcedureDispatch::Plain => self.plain_calls.set(self.plain_calls.get() + 1),
        }
        self.inner
            .compile_procedure_with_entry(target, profile, entry, dispatch)
    }

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.fast_calls.set(self.fast_calls.get() + 1);
        self.inner.compile(src)
    }

    fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.plain_calls.set(self.plain_calls.get() + 1);
        self.inner.compile_traced(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        match dispatch {
            ProcedureDispatch::Optimised => self.fast_calls.set(self.fast_calls.get() + 1),
            ProcedureDispatch::Plain => self.plain_calls.set(self.plain_calls.get() + 1),
        }
        self.inner
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn vm() -> (Vm, Capture) {
    let output = Capture::default();
    let mut vm = tcl_vm::test_support::native_vm_with_output(Box::new(output.clone()));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    (vm, output)
}

fn eval_ok(vm: &mut Vm, source: &str) -> String {
    let completion = vm.eval_source(source).expect("source compiles");
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    completion.result.to_str().to_string()
}

fn invoke(vm: &mut Vm, handle: &FunctionHandle) -> String {
    let completion = vm.invoke_function(handle);
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    completion.result.to_str().to_string()
}
