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
fn dictionary_named_fallback_rechecks_retained_worker() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
    let emitted = Rc::new(RefCell::new(Vec::new()));
    vm.set_compiler(Box::new(RecordingCompiler {
        inner: BytecodeCompileService::for_profile(profile),
        emitted: Rc::clone(&emitted),
    }));
    assert_eq!(eval(&mut vm, "set d {a 2}").0, Code::Ok);
    let before = emitted.borrow().len();
    let handle = vm
        .try_compile_function("dict incr d a 5")
        .expect("VM-owned handle");
    assert_eq!(emitted.borrow().len(), before + 1);
    let recorded = emitted.borrow();
    let selected = recorded[before..].iter().filter(|m| m.source.bytes() == b"dict incr d a 5").any(|m| {
        let instructions = &m.top_level.instructions;
        instructions.iter().enumerate().any(|(index, instruction)| {
            let Some(site) = instruction.native_compiler_selection.as_ref() else { return false; };
            let selected_worker = match &site.prerequisite {
                tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(required) => required.selected_worker.as_ref().and_then(|w| w.compiler.as_ref()).is_some_and(|c| c.registry_identity.trim_start_matches("::") == "tcl::dict::incr"),
                _ => false,
            };
            let Some(&end) = m.top_level.labels.get(&site.end) else { return false; };
            selected_worker && instructions[index + 1..].iter().any(|i| {
                i.offset >= 0 && (i.offset as usize) < end
                    && matches!(i.op, tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4)
                    && i.operands == [tcl_bytecode::Operand::Imm(4)]
            })
        })
    });
    drop(recorded);
    assert!(
        selected,
        "handle must contain the selected named dictionary fallback and its worker receipt"
    );
    assert_eq!(eval(&mut vm, "rename ::tcl::dict::incr ::tcl::dict::oldincr; proc ::tcl::dict::incr args {return [list override {*}$args]}").0, Code::Ok);
    let completion = vm
        .try_invoke_function(&handle)
        .expect("retained handle recovery");
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
    assert_eq!(result, "override d a 5");
}
