// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::cell::RefCell;
use std::rc::Rc;
use tcl_bytecode::{ModuleAsm, Op, Operand};
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_runtime_api::{
    CompileError, NativeCompilationEntry, ScriptCommandPlan, ScriptCompileTargetBytes, SourceImage,
};
use tcl_vm::CompileService;

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

#[test]
fn clock_read_compilation_executes_and_dynamic_options_dispatch() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    for (source, selector) in [
        ("clock clicks -mic", Some(1)),
        ("clock clicks -mil", Some(2)),
        ("clock seconds", Some(3)),
        ("clock clicks", None),
        ("set option -microseconds; clock clicks $option", None),
        ("clock clicks -mi", None),
    ] {
        let service = BytecodeCompileService::for_profile(profile);
        let aot = service
            .compile_for_profile(source, profile)
            .expect("compile without entry");
        assert!(
            aot.top_level
                .instructions
                .iter()
                .all(|i| i.op != Op::CLOCK_READ),
            "profile alone: {source}"
        );
        let emitted = Rc::new(RefCell::new(Vec::new()));
        let mut vm =
            tcl_vm::test_support::native_vm_for_profile(profile, Box::new(std::io::sink()));
        vm.set_compiler(Box::new(RecordingCompiler {
            inner: service,
            emitted: Rc::clone(&emitted),
        }));
        let result = vm.eval_source(source).unwrap_or_else(|error| {
            panic!(
                "entry-aware compilation {source}: {error:?}; emitted: {:#?}",
                emitted.borrow()
            )
        });
        if source == "clock clicks -mi" {
            assert!(!result.code.is_ok(), "{source}");
            assert!(
                result.result.to_str().contains("ambiguous"),
                "{source}: {:?}",
                result.result
            );
        } else {
            assert!(result.code.is_ok(), "{source}: {:?}", result.result);
            assert!(
                result
                    .result
                    .to_str()
                    .parse::<i64>()
                    .expect("integer clock value")
                    > 0,
                "{source}"
            );
        }
        let modules = emitted.borrow();
        assert!(
            !modules.is_empty(),
            "VM did not invoke the entry-aware compiler: {source}"
        );
        let reads: Vec<_> = modules
            .iter()
            .flat_map(|m| &m.top_level.instructions)
            .filter(|i| i.op == Op::CLOCK_READ)
            .collect();
        if let Some(selector) = selector {
            assert_eq!(reads.len(), 1, "{source}: {modules:#?}");
            assert_eq!(reads[0].operands, [Operand::Imm(selector)], "{source}");
        } else {
            assert!(reads.is_empty(), "{source}");
        }
    }
}
