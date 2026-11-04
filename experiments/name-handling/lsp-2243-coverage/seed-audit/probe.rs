// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;
use std::sync::Arc;
use tcl_compiler::analyser::ResolvedAnalysisInput;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};

struct Capture(Rc<RefCell<Option<tcl_runtime_api::NativeCompilationEntry>>>);
impl tcl_runtime_api::CompileService for Capture {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, _: &str) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "entry capture only".into(),
        ))
    }
    fn compile_script_bytes_with_entry(
        &self,
        _: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        *self.0.borrow_mut() = Some(entry.clone());
        self.compile("")
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        _: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        _: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        _: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        *self.0.borrow_mut() = Some(entry.clone());
        self.compile("")
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        _: &'static tcl_dialect::DialectProfile,
        _: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        Ok(tcl_runtime_api::ScriptCommandPlan::complete(source.len()))
    }
    fn compile_script_with_entry(
        &self,
        _: tcl_runtime_api::ScriptCompileTarget<'_>,
        _: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        *self.0.borrow_mut() = Some(entry.clone());
        self.compile("")
    }
}

fn main() {
    let dialect = std::env::args().nth(1).expect("dialect");
    let mut source = String::new();
    std::io::stdin().read_to_string(&mut source).unwrap();
    let environment = tcl_registry::model::ingress::resolve_environment(&dialect);
    let profile = environment.unit_profile();
    let input = ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let registry = input.borrowed_context_registry().commands();
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
    .expect("actual VM native registration");
    owner.set_compiler(Box::new(Capture(Rc::clone(&captured))));
    assert!(owner.try_compile_function("set entry_probe 1").is_err());
    let native_entry = captured
        .borrow_mut()
        .take()
        .expect("actual VM compiler request");
    let mut entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
        registry,
        &input,
        input.lexer_config(),
        Some(profile),
    );
    entry.native_entry = Some(Arc::new(native_entry));
    let unit = CompilationUnit::build_with_analysis_input(
        &source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: input.lexer_config(),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    )
    .with_interprocedural(registry, Some(profile));
    println!(
        "UNIT native_entry={} procedures={} functions={} input_matches={}",
        unit.ir_module.source_entry.native_entry.is_some(),
        unit.procedures.len(),
        unit.all_body_function_units().count(),
        unit.ir_module.source_metadata_input.as_ref() == Some(&input)
    );
    let solved =
        tcl_compiler::taint_interproc::solve_interprocedural_taints(&unit, registry, Some(profile));
    for spelling in ["$argv", "$::argv", "$env(PATH)", "$argc"] {
        for (start, _) in source.match_indices(spelling) {
            let span = tcl_lexer::Span::new(
                u32::try_from(start).unwrap(),
                u32::try_from(start + spelling.len()).unwrap(),
            );
            match tcl_compiler::ssa::original_source_read_at_unit(&unit, span, registry) {
                Some(read) => {
                    let function = read.function();
                    let taints = solved.taints_for_function(&unit, function);
                    let native_root = function.ssa.cell_key(read.symbol()).is_some_and(|cell| {
                        matches!(cell.root(), tcl_compiler::var_resolve::VariableCellKey::Namespace { identity, .. }
                            if unit.ir_module.source_entry.native_entry.as_deref().is_some_and(|entry| identity.is_native_root_in_entry(entry)))
                    });
                    println!(
                        "READ {spelling} {span:?} symbol={:?} version={:?} native_root={native_root} cell={:?} entry={:?} represented={:?}",
                        read.symbol(),
                        read.version(),
                        function.ssa.cell_key(read.symbol()),
                        taints.get(&(read.symbol(), 0)),
                        read.version()
                            .and_then(|version| taints.get(&(read.symbol(), version))),
                    );
                }
                None => println!("READ {spelling} {span:?} UNAVAILABLE"),
            }
        }
    }
    for warning in tcl_compiler::taint::find_taint_warnings_for_cu(&unit, registry, Some(profile)) {
        println!(
            "WARNING {:?} {:?} {:?}",
            warning.code, warning.span, warning.variable
        );
    }
}
