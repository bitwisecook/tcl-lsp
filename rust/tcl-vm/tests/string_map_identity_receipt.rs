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
fn empty_map_retains_subject_identity_and_worker_guard() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    let emitted = Rc::new(RefCell::new(Vec::new()));
    vm.set_compiler(Box::new(RecordingCompiler {
        inner: BytecodeCompileService::for_profile(profile),
        emitted: Rc::clone(&emitted),
    }));
    let handle = vm
        .try_compile_function("string map {{} x} $subject")
        .expect("VM-owned handle");
    let modules = emitted.borrow();
    let module = modules
        .iter()
        .find(|m| m.source.bytes() == b"string map {{} x} $subject")
        .expect("exact source module");
    assert!(module.top_level.instructions.iter().any(|i| {
        i.native_compiler_selection.as_ref().is_some_and(|site| match &site.prerequisite {
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(required) => required.selected_worker.as_ref().and_then(|w| w.compiler.as_ref()).is_some_and(|c| c.registry_identity.trim_start_matches("::") == "tcl::string::map"),
            _ => false,
        })
    }), "selected worker receipt");
    assert!(
        !module.top_level.instructions.iter().any(|i| matches!(
            i.op,
            tcl_bytecode::Op::STR_MAP
                | tcl_bytecode::Op::INVOKE_STK1
                | tcl_bytecode::Op::INVOKE_STK4
        )),
        "identity instruction must not dispatch or map"
    );
    drop(modules);
    for subject in [
        tcl_vm::Value::empty(),
        tcl_vm::Value::string("hello"),
        tcl_vm::Value::int(42),
        tcl_vm::Value::byte_array(Rc::<[u8]>::from([0, 255])),
    ] {
        let identity = subject.native_object_identity();
        vm.set_var("subject", subject).expect("set subject");
        let result = vm.try_invoke_function(&handle).expect("identity execution");
        assert_eq!(result.code, Code::Ok);
        assert_eq!(
            result.result.native_object_identity(),
            identity,
            "original object identity"
        );
    }
    assert_eq!(eval(&mut vm, "rename ::tcl::string::map ::tcl::string::oldmap; proc ::tcl::string::map args {return OVERRIDE}").0, Code::Ok);
    let result = vm
        .try_invoke_function(&handle)
        .expect("stale handle recovery");
    assert_eq!(result.code, Code::Ok);
    let projection = vm
        .capture_original_operand_projection()
        .expect("projection");
    assert_eq!(
        vm.original_operand_unicode(&projection, &result.result)
            .expect("result"),
        "OVERRIDE"
    );
    drop(result);
    vm.check_original_operand_projection(&projection)
        .expect("current projection");
    vm.finish_original_operand_projection(projection)
        .expect("settled projection");
}

#[test]
fn empty_map_keeps_selected_worker_during_subject_evaluation() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    let emitted = Rc::new(RefCell::new(Vec::new()));
    vm.set_compiler(Box::new(RecordingCompiler {
        inner: BytecodeCompileService::for_profile(profile),
        emitted: Rc::clone(&emitted),
    }));
    assert_eq!(eval(&mut vm, "proc mutate {} {rename ::tcl::string::map ::tcl::string::oldmap; proc ::tcl::string::map args {return OVERRIDE}; return payload}").0, Code::Ok);
    let handle = vm
        .try_compile_function("string map {{} x} [mutate]")
        .expect("VM-owned handle");
    let modules = emitted.borrow();
    let module = modules
        .iter()
        .find(|m| m.source.bytes() == b"string map {{} x} [mutate]")
        .expect("exact source module");
    assert!(module.top_level.instructions.iter().any(|i| i.native_compiler_selection.as_ref().is_some_and(|site| match &site.prerequisite {
        tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(required) => required.selected_worker.as_ref().and_then(|w| w.compiler.as_ref()).is_some_and(|c| c.registry_identity.trim_start_matches("::") == "tcl::string::map"),
        _ => false,
    })), "selected worker receipt");
    assert!(
        !module
            .top_level
            .instructions
            .iter()
            .any(|i| matches!(i.op, tcl_bytecode::Op::STR_MAP)),
        "identity instruction must not map"
    );
    drop(modules);
    let result = vm
        .try_invoke_function(&handle)
        .expect("selected invocation");
    assert_eq!(result.code, Code::Ok);
    let projection = vm
        .capture_original_operand_projection()
        .expect("projection");
    assert_eq!(
        vm.original_operand_unicode(&projection, &result.result)
            .expect("result"),
        "payload"
    );
    drop(result);
    vm.check_original_operand_projection(&projection)
        .expect("current projection");
    vm.finish_original_operand_projection(projection)
        .expect("settled projection");
    assert_eq!(
        eval(&mut vm, "string map {{} x} payload"),
        (Code::Ok, "OVERRIDE".to_string())
    );
}

#[test]
fn empty_map_private_identity_and_subject_error() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    let emitted = Rc::new(RefCell::new(Vec::new()));
    vm.set_compiler(Box::new(RecordingCompiler {
        inner: BytecodeCompileService::for_profile(profile),
        emitted: Rc::clone(&emitted),
    }));
    let handle = vm
        .try_compile_function("::tcl::string::map {{} x} $subject")
        .expect("VM-owned private handle");
    let modules = emitted.borrow();
    let module = modules
        .iter()
        .find(|m| m.source.bytes() == b"::tcl::string::map {{} x} $subject")
        .expect("exact private source");
    assert!(module.top_level.instructions.iter().any(|i| i.native_compiler_selection.as_ref().is_some_and(|site| match &site.prerequisite {
        tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Command(required) => required.compiler.registry_identity.trim_start_matches("::") == "tcl::string::map",
        _ => false,
    })), "selected private worker receipt");
    assert!(!module.top_level.instructions.iter().any(|i| matches!(
        i.op,
        tcl_bytecode::Op::STR_MAP | tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
    )));
    drop(modules);
    let subject = tcl_vm::Value::int(42);
    let identity = subject.native_object_identity();
    vm.set_var("subject", subject).expect("set subject");
    let result = vm.try_invoke_function(&handle).expect("private identity");
    assert_eq!(result.code, Code::Ok);
    assert_eq!(result.result.native_object_identity(), identity);
    drop(result);
    assert_eq!(
        eval(&mut vm, "proc fail_subject {} {error SUBJECT_ERROR}").0,
        Code::Ok
    );
    assert_eq!(
        eval(&mut vm, "string map {{} x} [fail_subject]"),
        (Code::Error, "SUBJECT_ERROR".to_string())
    );
}

#[test]
fn empty_map_retained_handle_observes_worker_execution_trace() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    let emitted = Rc::new(RefCell::new(Vec::new()));
    vm.set_compiler(Box::new(RecordingCompiler {
        inner: BytecodeCompileService::for_profile(profile),
        emitted: Rc::clone(&emitted),
    }));
    assert_eq!(
        eval(
            &mut vm,
            "set subject abc; set events {}; proc trace_enter {cmd op} {lappend ::events enter}"
        )
        .0,
        Code::Ok
    );
    let handle = vm
        .try_compile_function("string map {{} x} $subject")
        .expect("VM-owned handle");
    let modules = emitted.borrow();
    let module = modules
        .iter()
        .find(|m| m.source.bytes() == b"string map {{} x} $subject")
        .expect("exact source module");
    assert!(module.top_level.instructions.iter().any(|i| i.native_compiler_selection.as_ref().is_some_and(|site| match &site.prerequisite {
        tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(required) => required.selected_worker.as_ref().and_then(|w| w.compiler.as_ref()).is_some_and(|c| c.registry_identity.trim_start_matches("::") == "tcl::string::map"),
        _ => false,
    })), "selected worker receipt");
    assert!(!module.top_level.instructions.iter().any(|i| matches!(
        i.op,
        tcl_bytecode::Op::STR_MAP | tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
    )));
    drop(modules);
    assert_eq!(
        eval(
            &mut vm,
            "trace add execution ::tcl::string::map enter trace_enter"
        )
        .0,
        Code::Ok
    );
    let result = vm
        .try_invoke_function(&handle)
        .expect("traced retained handle");
    assert_eq!(result.code, Code::Ok);
    let projection = vm
        .capture_original_operand_projection()
        .expect("projection");
    assert_eq!(
        vm.original_operand_unicode(&projection, &result.result)
            .expect("result"),
        "abc"
    );
    drop(result);
    vm.check_original_operand_projection(&projection)
        .expect("current projection");
    vm.finish_original_operand_projection(projection)
        .expect("settled projection");
    assert_eq!(eval(&mut vm, "set events"), (Code::Ok, "enter".to_string()));
}
