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

//! Actual whole-program Native entry and its live issuing VM for WASM fixtures.

use std::cell::RefCell;
use std::rc::Rc;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_runtime_api::CompileService;

struct Capture {
    original: String,
    entry: Rc<RefCell<Option<tcl_runtime_api::NativeCompilationEntry>>>,
}
impl CompileService for Capture {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, _: &str) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "entry capture only".into(),
        ))
    }
    fn compile_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        assert_eq!(target.source.bytes(), self.original.as_bytes());
        assert_eq!(
            target.namespace.as_segments(),
            [] as [tcl_runtime_api::NameBytes; 0]
        );
        *self.entry.borrow_mut() = Some(entry.clone());
        self.compile("")
    }
    fn compile_script_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTarget<'_>,
        _: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        assert_eq!(target.source, self.original);
        assert_eq!(target.namespace, "");
        *self.entry.borrow_mut() = Some(entry.clone());
        self.compile("")
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }
}

pub(super) struct NativeWholeProgramFixture {
    pub(super) unit: CompilationUnit,
    pub(super) context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    _owner: tcl_vm::Vm,
}

pub(super) fn native_whole_program_fixture(program: &str) -> NativeWholeProgramFixture {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl9.0");
    let profile = environment.unit_profile();
    let context = environment.default_context_registry();
    let captured = Rc::new(RefCell::new(None));
    let mut owner = tcl_vm::Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("actual native core for whole-program WASM fixture");
    owner.set_compiler(Box::new(Capture {
        original: program.to_owned(),
        entry: Rc::clone(&captured),
    }));
    assert!(owner.try_compile_function(program).is_err());
    let native = captured
        .borrow_mut()
        .take()
        .expect("actual original-program native entry");
    // Same whole-program/Direct source contract as common_aot_plan::tests::native_unit;
    // environment_ingress::captured_native_entry_with_owner retains its actual VM too.
    // This AOT driver purpose is independent of the callback's bytecode request.
    let entry = tcl_compiler::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
            loop_depth: 0,
            catch_depth: Some(0),
        },
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_context_registry(
        program,
        tcl_compiler::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    NativeWholeProgramFixture {
        unit,
        context,
        _owner: owner,
    }
}
