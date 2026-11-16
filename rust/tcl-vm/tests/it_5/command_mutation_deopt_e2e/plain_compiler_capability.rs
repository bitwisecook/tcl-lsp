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

//! Recovery needs a plain-dispatch compiler: a missing, script-only or dishonest one makes the VM fail closed.

use super::*;

struct DishonestPlainCompilerSvc(BytecodeCompileService);

struct ScriptOnlyCompilerSvc(BytecodeCompileService);

impl CompileService for DishonestPlainCompilerSvc {
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
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let mut module = self.0.compile_script_bytes_for_profile(target, profile)?;
        module.top_level.command_bindings.clear();
        for procedure in module.procedures.values_mut() {
            procedure.command_bindings.clear();
        }
        Ok(module)
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let mut module = self
            .0
            .compile_script_bytes_with_entry(target, profile, entry)?;
        module.top_level.command_bindings.clear();
        for procedure in module.procedures.values_mut() {
            procedure.command_bindings.clear();
        }
        Ok(module)
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let mut module = self.0.compile_procedure_bytes_for_profile(
            target,
            profile,
            tcl_runtime_api::ProcedureDispatch::Optimised,
        )?;
        if dispatch == tcl_runtime_api::ProcedureDispatch::Plain {
            module.top_level.command_bindings.clear();
        }
        Ok(module)
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        let mut module = self.0.compile_procedure_bytes_with_entry(
            target,
            profile,
            entry,
            tcl_runtime_api::ProcedureDispatch::Optimised,
        )?;
        if dispatch == tcl_runtime_api::ProcedureDispatch::Plain {
            module.top_level.command_bindings.clear();
        }
        Ok(module)
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

    fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
        // Deliberately violate the capability contract: the VM must validate
        // the returned artifact's explicit mode instead of trusting this
        // method's name or inferring safety from its dependency list.
        let mut module = self.0.compile(src)?;
        module.top_level.command_bindings.clear();
        for procedure in module.procedures.values_mut() {
            procedure.command_bindings.clear();
        }
        Ok(module)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let mut module =
            self.0
                .compile_procedure_for_profile(target, profile, ProcedureDispatch::Optimised)?;
        if dispatch == ProcedureDispatch::Plain {
            module.top_level.command_bindings.clear();
        }
        Ok(module)
    }
}

impl CompileService for ScriptOnlyCompilerSvc {
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
        _target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "procedure compilation unavailable".into(),
        ))
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        _target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
        _dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "procedure compilation unavailable".into(),
        ))
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

    fn compile_script_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        self.0.compile_script_with_entry(target, profile, entry)
    }

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.0.compile(src)
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.0.compile_for_profile(src, profile)
    }

    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.0.compile_traced_for_profile(src, profile)
    }
}

/// A command replacement recompiles against the live commands and needs no
/// plain dispatch; a step trace still does, so a service without that
/// capability fails closed when a step-traced procedure is entered.
#[test]
fn step_traced_procedure_fails_closed_without_plain_dispatch_capability() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc p {} {expr {1+2}}; proc step_callback args {}; \
         trace add execution p enterstep step_callback",
    );
    vm.set_compiler(Box::new(OptimisedOnlyCompilerSvc(
        BytecodeCompileService::default(),
    )));

    let Err(tcl_vm::TclError::Host(tcl_vm::TclHostFailure::Execution(refusal))) =
        vm.eval_source("p")
    else {
        panic!("a missing plain-dispatch capability is a host refusal");
    };
    assert_eq!(
        refusal.to_string(),
        "CompileService does not support plain command dispatch"
    );
}

#[test]
fn native_replacement_fails_closed_when_no_plain_compiler_exists() {
    struct CountingOverride(Rc<Cell<usize>>);
    impl NativeCommand for CountingOverride {
        fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
            self.0.set(self.0.get() + 1);
            Completion::new(Code::Ok, Value::string("NATIVE_OVERRIDE"), Value::empty())
        }
    }
    let calls = Rc::new(Cell::new(0));
    let module = BytecodeCompileService::default()
        .compile("expr {1+2}")
        .expect("module compiles");
    let mut vm = tcl_vm::test_support::native_vm();
    vm.register_native_command("expr", Rc::new(CountingOverride(Rc::clone(&calls))));

    // Missing native compilation capability is a host refusal, outside Tcl completion.
    let error = vm
        .try_run_module(&module)
        .expect_err("stale module must be refused");
    let tcl_runtime_api::NativeExecutionError::CompileServiceRefusal(refusal) = error else {
        panic!("expected missing compiler refusal: {error:?}");
    };
    assert_eq!(
        refusal.reason,
        "eval / command substitution requires a byte CompileService"
    );
    assert_eq!(
        refusal.scope,
        tcl_runtime_api::NativeCompilationAdmissionScope::Script
    );
    assert_eq!(refusal.source.bytes(), b"expr {1+2}");
    assert_eq!(calls.get(), 0, "no replacement command ran before refusal");
}

#[test]
fn profileless_function_fails_closed_after_binding_replacement() {
    let module = BytecodeCompileService::default()
        .compile("expr {1+2}")
        .expect("module compiles");
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return FUNCTION_OVERRIDE}",
    );

    let completion = vm.run_function(&module.top_level);
    assert_eq!(completion.code, Code::Error, "{completion:?}");
    assert!(
        completion
            .result
            .to_str()
            .contains("has no source for plain dispatch"),
        "{}",
        completion.result.to_str()
    );
}

/// A step-traced procedure needs plain dispatch, so a service that answers
/// that request with optimised bytecode is rejected.
#[test]
fn procedure_recovery_rejects_a_dishonest_plain_compiler() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc p {} {expr {1+2}}; proc step_callback args {}; \
         trace add execution p enterstep step_callback",
    );
    vm.set_compiler(Box::new(DishonestPlainCompilerSvc(
        BytecodeCompileService::default(),
    )));

    let completion = vm.eval_source("p").expect("caller compiles");
    assert_eq!(completion.code, Code::Error, "{completion:?}");
    assert!(
        completion
            .result
            .to_str()
            .contains("plain-dispatch capability returned optimised bytecode"),
        "{}",
        completion.result.to_str()
    );
}

#[test]
fn procedure_consumers_preserve_a_missing_typed_compiler_capability() {
    const CAUSE: &str = "CompileService does not support procedure-body compilation";
    // A missing procedure capability is never bypassed: it surfaces as the
    // service's own diagnostic or as a host refusal, outside guest code.
    let refused = |result: Result<Completion<Value>, tcl_vm::TclError>| match result {
        Ok(completion) => {
            completion.code == Code::Error && completion.result.to_str().as_ref() == CAUSE
        }
        Err(tcl_vm::TclError::Host(tcl_vm::TclHostFailure::Execution(
            tcl_runtime_api::NativeExecutionError::CompileServiceRefusal(_),
        ))) => true,
        Err(_) => false,
    };

    let mut host_vm = tcl_vm::test_support::native_vm();
    host_vm.set_compiler(Box::new(ScriptOnlyCompilerSvc(
        BytecodeCompileService::default(),
    )));
    // As in C, a body compiles at its first call, not at its definition.
    host_vm
        .define_procedure("host_p", &[], "return HOST")
        .expect("a definition compiles nothing");
    assert!(refused(host_vm.eval_source("host_p")), "host procedure");

    for source in [
        "set body {return PROC}; proc p {} $body; p",
        "set lambda [list {} {return APPLY}]; apply $lambda",
        "oo::class create C {method m {} {return METHOD}}; set o [C new]; $o m",
    ] {
        let mut vm = tcl_vm::test_support::native_vm();
        vm.set_compiler(Box::new(ScriptOnlyCompilerSvc(
            BytecodeCompileService::default(),
        )));
        assert!(refused(vm.eval_source(source)), "{source}");
    }
}
