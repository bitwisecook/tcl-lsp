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
fn private_ascii_failindex_handle_uses_generic_dispatch_and_observes_rebinding() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    let emitted = Rc::new(RefCell::new(Vec::new()));
    vm.set_compiler(Box::new(RecordingCompiler {
        inner: BytecodeCompileService::for_profile(profile),
        emitted: Rc::clone(&emitted),
    }));
    assert_eq!(eval(&mut vm, "set s abcé; set idx -1").0, Code::Ok);
    let source = "::tcl::string::is ascii -failindex idx $s";
    let before = emitted.borrow().len();
    let handle = vm.try_compile_function(source).expect("VM-owned handle");
    assert_eq!(emitted.borrow().len(), before + 1);
    let recorded = emitted.borrow();
    let module = &recorded[before];
    assert_eq!(module.source.bytes(), source.as_bytes());
    let instruction = module
        .top_level
        .instructions
        .iter()
        .find(|i| {
            matches!(
                i.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            )
        })
        .expect("ordinary invocation");
    assert_eq!(instruction.operands, [tcl_bytecode::Operand::Imm(5)]);
    assert!(
        module
            .top_level
            .literals
            .entries()
            .iter()
            .any(|literal| literal.byte_payload() == Some(b"::tcl::string::is".as_slice()))
    );
    assert!(
        module
            .top_level
            .instructions
            .iter()
            .all(|i| i.op != tcl_bytecode::Op::STR_CLASS)
    );
    drop(recorded);
    let completion = vm.try_invoke_function(&handle).expect("initial handle");
    assert_eq!(completion.code, Code::Ok);
    let projection = vm
        .capture_original_operand_projection()
        .expect("result projection");
    let result = vm
        .original_operand_unicode(&projection, &completion.result)
        .expect("result text");
    drop(completion);
    vm.check_original_operand_projection(&projection)
        .expect("current projection");
    vm.finish_original_operand_projection(projection)
        .expect("settled projection");
    assert_eq!(result, "0");
    assert_eq!(eval(&mut vm, "set idx"), (Code::Ok, "3".to_owned()));
    assert_eq!(eval(&mut vm, "rename ::tcl::string::is ::tcl::string::oldis; proc ::tcl::string::is args {return OVERRIDE}").0, Code::Ok);
    let completion = vm
        .try_invoke_function(&handle)
        .expect("retained handle after replacement");
    assert_eq!(completion.code, Code::Ok);
    let projection = vm
        .capture_original_operand_projection()
        .expect("result projection");
    let result = vm
        .original_operand_unicode(&projection, &completion.result)
        .expect("result text");
    drop(completion);
    vm.check_original_operand_projection(&projection)
        .expect("current projection");
    vm.finish_original_operand_projection(projection)
        .expect("settled projection");
    assert_eq!(result, "OVERRIDE");
}

#[test]
fn private_ascii_failindex_keeps_operand_effects_errors_and_rebinding() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    assert_eq!(eval(&mut vm, "set seen {}; proc idx {} {global seen; lappend seen index; return idx}; proc val {} {global seen; lappend seen value; return abcé}").0, Code::Ok);
    assert_eq!(
        eval(
            &mut vm,
            "list [::tcl::string::is ascii -failindex [idx] [val]] $idx $seen"
        ),
        (Code::Ok, "0 3 {index value}".to_owned())
    );
    assert_eq!(
        eval(
            &mut vm,
            "set seen {}; list [catch {::tcl::string::is ascii -failindex [error boom] [val]} result] $result $seen"
        ),
        (Code::Ok, "1 boom {}".to_owned())
    );
    assert_eq!(eval(&mut vm, "set seen {}; rename idx oldidx; proc idx {} {global seen; lappend seen index; rename ::tcl::string::is ::tcl::string::oldis; proc ::tcl::string::is args {return OVERRIDE}; return idx}").0, Code::Ok);
    assert_eq!(
        eval(
            &mut vm,
            "list [::tcl::string::is ascii -failindex [idx] [val]] $seen"
        ),
        (Code::Ok, "OVERRIDE {index value}".to_owned())
    );
}

#[test]
fn private_ascii_strict_failindex_rejects_empty_and_sets_index() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    assert_eq!(
        eval(
            &mut vm,
            "set idx -1; list [::tcl::string::is ascii -strict -failindex idx {}] $idx"
        ),
        (Code::Ok, "0 0".to_owned())
    );
}
