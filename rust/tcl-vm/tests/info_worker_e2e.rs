// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::cell::RefCell;
use std::rc::Rc;
use tcl_bytecode::ModuleAsm;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_runtime_api::{
    CompileError, NativeCompilationEntry, ScriptCommandPlan, ScriptCompileTargetBytes, SourceImage,
};
use tcl_vm::CompileService;
use tcl_vm::{Code, Vm};

struct RecordingCompiler {
    inner: BytecodeCompileService,
    emitted: Rc<RefCell<Vec<ModuleAsm>>>,
}

impl CompileService for RecordingCompiler {
    type Module = ModuleAsm;

    fn compile(&self, source: &str) -> Result<ModuleAsm, CompileError> {
        self.inner.compile(source)
    }

    fn compile_script_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ModuleAsm, CompileError> {
        let module = self
            .inner
            .compile_script_with_entry(target, profile, entry)?;
        self.emitted.borrow_mut().push(module.clone());
        Ok(module)
    }

    fn script_command_plan_bytes_with_entry(
        &self,
        source: &SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ScriptCommandPlan, CompileError> {
        self.inner
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ModuleAsm, CompileError> {
        let module = self
            .inner
            .compile_script_bytes_with_entry(target, profile, entry)?;
        self.emitted.borrow_mut().push(module.clone());
        Ok(module)
    }

    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
    }

    fn compile_original_procedure_bytes_with_entry(
        &self,
        request: &tcl_runtime_api::OriginalProcedureCompileRequest,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_original_procedure_bytes_with_entry(request, profile, entry, dispatch)
    }

    fn compile_plain_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }
}

fn eval(vm: &mut Vm, source: &str) -> (Code, String) {
    let completion = vm.try_eval_source(source).expect("VM compilation entry");
    let original = vm
        .capture_original_operand_projection()
        .expect("result projection");
    let text = vm
        .original_operand_unicode(&original, &completion.result)
        .expect("result text");
    let code = completion.code;
    drop(completion);
    vm.check_original_operand_projection(&original)
        .expect("current result projection");
    vm.finish_original_operand_projection(original)
        .expect("settled result projection");
    (code, text)
}

#[test]
fn replacing_info_errorstack_worker_invalidates_a_cached_call() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    let emitted = Rc::new(RefCell::new(Vec::new()));
    vm.set_compiler(Box::new(RecordingCompiler {
        inner: BytecodeCompileService::for_profile(profile),
        emitted: Rc::clone(&emitted),
    }));
    let (code, _) = eval(&mut vm, "catch {error hi}");
    assert_eq!(code, Code::Ok);
    let before = emitted.borrow().len();
    let handle = vm
        .try_compile_function("info errorstack")
        .expect("VM-owned warm handle");
    assert_eq!(
        emitted.borrow().len(),
        before + 1,
        "handle construction must compile exactly once"
    );
    let native_selection = emitted.borrow()[before..].iter().filter(|module| module.source.bytes() == b"info errorstack").any(|module| {
        module.top_level.instructions.iter().filter_map(|instruction| instruction.native_compiler_selection.as_ref()).any(|site| {
            match &site.prerequisite {
                tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(required) => required.selected_worker.as_ref().and_then(|worker| worker.compiler.as_ref()).is_some_and(|compiler| compiler.registry_identity == "tcl::info::errorstack"),
                _ => false,
            }
        })
    });
    assert!(
        native_selection,
        "warm call must select the actual errorstack worker"
    );
    let (code, stack) = eval(&mut vm, "info errorstack");
    assert_eq!(code, Code::Ok);
    assert_eq!(stack, "INNER {returnImm hi {}}");
    let (code, _) = eval(
        &mut vm,
        "rename ::tcl::info::errorstack ::tcl::info::old_errorstack; proc ::tcl::info::errorstack {} {return OVERRIDE}",
    );
    assert_eq!(code, Code::Ok);
    let (code, result) = eval(&mut vm, "info errorstack");
    assert_eq!(code, Code::Ok);
    assert_eq!(result, "OVERRIDE");
    let completion = vm
        .try_invoke_function(&handle)
        .expect("the retained handle must recover safely");
    assert_eq!(completion.code, Code::Ok);
    let original = vm
        .capture_original_operand_projection()
        .expect("handle result projection");
    let result = vm
        .original_operand_unicode(&original, &completion.result)
        .expect("handle result text");
    drop(completion);
    vm.check_original_operand_projection(&original)
        .expect("handle projection current");
    vm.finish_original_operand_projection(original)
        .expect("handle projection settled");
    assert_eq!(result, "OVERRIDE");
}
