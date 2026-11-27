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

//! The function lattice under a source input whose entry the VM supplied:
//! the native compilation entry a real `tcl_vm::Vm` hands its compiler keeps
//! the lattice's completion, and a missing, foreign or older input does not.

use std::sync::Arc;

use tcl_compiler::body_analysis_capsule::{BodyLatticeCommandBindings, BodyLatticeInputs};
use tcl_compiler::compilation_unit::FunctionUnit;
use tcl_lsp_db::{
    CfgContext, CompilerMemoSnapshot, FnLatticeEntry, FnLatticeKey, FnLatticeModuleFacts,
    TclDatabase, TclDb, ValueTransferContext, function_lattice,
};

fn supplied_cfg_lattice_key<'db>(
    db: &'db dyn TclDb,
    module: &tcl_compiler::ir::Module,
    input: Option<tcl_compiler::analyser::ResolvedAnalysisInput>,
) -> FnLatticeKey<'db> {
    let registry = module.resolved_registry();
    let (upvars, params, globals, bindings, owners) =
        tcl_compiler::cfg_builder::prepare_cfg_context_with_registry(module, registry);
    assert!(upvars.is_empty() && params.is_empty() && globals.is_empty());
    let context = CfgContext::new(
        db,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        BodyLatticeCommandBindings::from(bindings),
        owners,
    );
    FnLatticeKey::new(
        db,
        BodyLatticeInputs::from(module.top_level.clone()),
        "::subject".to_owned(),
        Vec::new(),
        context,
        module.lexer_config.normalized(),
        module.dialect.clone().unwrap(),
        CompilerMemoSnapshot {
            profile: module
                .dialect_profile
                .map(tcl_dialect::DialectProfile::cache_key),
            registry: registry.snapshot(),
        },
        Vec::new(),
        FnLatticeModuleFacts {
            known_classes: Vec::new(),
            traced_variables: Vec::new(),
            has_dynamic_variable_trace: false,
        },
        FnLatticeEntry {
            original_declaration_body: None,
            plain_command_dispatch: module.plain_command_dispatch,
            metadata_context:
                tcl_compiler::registry_invocation::OwnedInvocationMetadataContext::for_source_input(
                    input.as_ref(),
                ),
            source_metadata_input: input,
            irules_event_body: None,
            command_trust: tcl_compiler::command_binding::ModuleCommandMutations::default()
                .snapshot(),
        },
        ValueTransferContext::of(
            db,
            tcl_compiler::value_transfer::AnalysisContextKey::for_module(
                &tcl_compiler::command_binding::ModuleCommandMutations::default(),
                registry,
            ),
            0,
        ),
    )
}

fn assert_supplied_function_lattice_currency<'db>(
    db: &'db dyn TclDb,
    module: &tcl_compiler::ir::Module,
    generation: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    entry: &tcl_compiler::command_binding::SourceAnalysisEntry,
    unknown: FnLatticeKey<'db>,
) -> (Arc<FunctionUnit>, Option<tcl_compiler::cfg::Terminator>) {
    let input = module.source_metadata_input.as_ref().unwrap();
    assert!(Arc::ptr_eq(&input.context_registry(), generation));
    let selected = supplied_cfg_lattice_key(db, module, Some(input.clone()));
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(generation.commands())),
    );
    assert!(Arc::ptr_eq(older.commands(), generation.commands()));
    let unavailable = supplied_cfg_lattice_key(
        db,
        module,
        Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        )),
    );
    let foreign = supplied_cfg_lattice_key(
        db,
        module,
        Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::context_for_profile(
                tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
            ),
            input.lexer_config(),
        )),
    );
    let missing = supplied_cfg_lattice_key(db, module, None);
    let expected = Some(tcl_compiler::cfg::Terminator::Complete {
        route: tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
            tcl_registry::CompletionCode::Error,
        ),
        span: Some(module.top_level.statements[0].span()),
    });
    let current = function_lattice(db, selected);
    assert_eq!(current.cfg.blocks[&current.cfg.entry].terminator, expected);
    assert_eq!(current.cfg.blocks[&current.cfg.entry].statements.len(), 1);
    for negative in [unavailable, foreign, missing, unknown] {
        assert!(
            selected != negative,
            "complete source input owns memo identity"
        );
        let conservative = function_lattice(db, negative);
        let entry = &conservative.cfg.blocks[&conservative.cfg.entry];
        assert_ne!(entry.terminator, expected);
        assert_eq!(
            entry.statements.len(),
            2,
            "withheld completion keeps the following call"
        );
    }
    assert!(Arc::ptr_eq(&current, &function_lattice(db, selected)));
    assert!(
        entry
            .native_entry
            .as_ref()
            .unwrap()
            .compiler_pass_environment
            .as_ref()
            .unwrap()
            .is_current_for(entry.native_entry.as_ref().unwrap().interpreter)
    );
    (current, expected)
}

fn supplied_lattice_test_input(
    generation: &Arc<tcl_registry::model::assembly::ContextRegistry>,
    config: tcl_lexer::LexerConfig,
) -> tcl_compiler::analyser::ResolvedAnalysisInput {
    tcl_compiler::analyser::ResolvedAnalysisInput::new(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        Arc::clone(generation),
        config,
    )
}

#[test]
fn function_lattice_retains_supplied_availability_and_refuses_missing_or_foreign_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source CFG completion only; no Native execution or handler grant.
    struct Capture(
        std::rc::Rc<std::cell::RefCell<Option<tcl_runtime_api::NativeCompilationEntry>>>,
    );
    impl tcl_vm::CompileService for Capture {
        type Module = tcl_bytecode::ModuleAsm;

        fn compile(&self, _: &str) -> Result<Self::Module, tcl_vm::CompileError> {
            Err(tcl_vm::CompileError::Unsupported(
                "entry capture only".into(),
            ))
        }

        fn compile_script_with_entry(
            &self,
            _: tcl_runtime_api::ScriptCompileTarget<'_>,
            _: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<Self::Module, tcl_vm::CompileError> {
            *self.0.borrow_mut() = Some(entry.clone());
            self.compile("")
        }
    }
    let db = TclDatabase::default();
    let generation = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let supplied = supplied_lattice_test_input(&generation, config);
    let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(generation.commands(), config)
        .with_resolved_analysis_input(supplied.clone());
    // Without an entry the document runs in a fresh interpreter; unknown
    // history must be supplied explicitly.
    lowerer.set_source_analysis_options(
        tcl_compiler::command_binding::SourceAnalysisEntry {
            metadata_context:
                tcl_compiler::registry_invocation::OwnedInvocationMetadataContext::for_source_input(
                    Some(&supplied),
                ),
            unknown_entry: true,
            ..tcl_compiler::command_binding::SourceAnalysisEntry::default()
        }
        .options(),
    );
    let source = "throw ERROR payload; set reached 1";
    let unknown_module = lowerer.lower(source).clone();
    assert!(unknown_module.source_entry.unknown_entry);
    assert!(unknown_module.source_entry.native_entry.is_none());
    let input = unknown_module.source_metadata_input.as_ref().unwrap();
    let unknown = supplied_cfg_lattice_key(&db, &unknown_module, Some(input.clone()));

    let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut owner = tcl_vm::Vm::with_native_core(
        Box::new(std::io::sink()),
        std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    owner.set_compiler(Box::new(Capture(std::rc::Rc::clone(&captured))));
    assert!(owner.try_compile_function("set entry_probe 1").is_err());
    let native = captured
        .borrow_mut()
        .take()
        .expect("actual VM compiler entry");
    assert_eq!(
        native.execution_point,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").point()
    );
    assert!(
        native
            .compiler_pass_environment
            .as_ref()
            .unwrap()
            .is_current_for(native.interpreter)
    );
    let entry = tcl_compiler::command_binding::SourceAnalysisEntry {
        metadata_context:
            tcl_compiler::registry_invocation::OwnedInvocationMetadataContext::for_source_input(
                Some(input),
            ),
        invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
            loop_depth: 0,
            catch_depth: Some(0),
        },
        native_entry: Some(Arc::new(native)),
        ..tcl_compiler::command_binding::SourceAnalysisEntry::default()
    };
    let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(generation.commands(), config)
        .with_resolved_analysis_input(input.clone());
    lowerer.set_source_analysis_options(entry.options());
    let module = lowerer.lower(source).clone();
    assert!(!module.source_entry.unknown_entry);
    assert!(module.source_entry.native_entry.is_some());
    assert_eq!(module.source_metadata_input.as_ref(), Some(input));
    let _retained_currency =
        assert_supplied_function_lattice_currency(&db, &module, &generation, &entry, unknown);
    drop(owner);
}
