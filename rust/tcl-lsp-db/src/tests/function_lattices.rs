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

//! The per-procedure lattice memo: sharing, key identity, supplied availability and reuse when bodies shift.

use super::*;

/// A memo *hit* must hand its per-procedure lattices over by
/// refcount, not deep-copy them.
///
/// The span-carrying halves of a `FunctionUnit` (`cfg` / `ssa` /
/// `sccp.constant_branches`) genuinely have to be copied — the memo stores
/// the unit at offset 0 and every consumer is rebased to the procedure's
/// real position — but `def_use` / `types` / `taints` / `rendered_props` are
/// span-free, so a rebuild that hits the memo must share them.
///
/// Two *distinct* `SourceFile`s carrying the same text give two distinct
/// `compilation_unit` keys, so both really build — while their per-procedure
/// lattice demands land on the same `function_lattice` memos.  Driving it
/// through the tracked query rather than calling `memoised_compilation_unit`
/// directly is deliberate: the per-body keys are only garbage-collectable
/// when interned inside a tracked query (see the crate docs' "The interned
/// garbage collector is load-bearing").
#[test]
fn memoised_lattices_are_shared_not_deep_copied_across_builds() {
    const SRC: &str = "proc alpha {a b} {\n    set s [expr {$a + $b}]\n    return $s\n}\n\
                           proc beta {x} {\n    return [alpha $x 1]\n}\n";
    let db = TclDatabase::default();
    let cfg_key = lexer_cfg_key(&db, "tcl8.6");
    let file_a = SourceFile::new(&db, SRC.to_owned(), "tcl8.6".to_owned(), None);
    let file_b = SourceFile::new(&db, SRC.to_owned(), "tcl8.6".to_owned(), None);
    let first = compilation_unit(&db, file_a, cfg_key, 0).expect("no overlay always builds");
    let second = compilation_unit(&db, file_b, cfg_key, 0).expect("no overlay always builds");
    for (qname, offset) in [("::alpha", 0), ("::beta", SRC.find("proc beta").unwrap())] {
        let first_id = original_test_body_id(&first, offset);
        let second_id = original_test_body_id(&second, offset);
        let a = first
            .function_for_body(Some(first_id))
            .expect("procedure in first build");
        let b = second
            .function_for_body(Some(second_id))
            .expect("procedure in second build");
        assert!(
            Arc::ptr_eq(&a.def_use, &b.def_use),
            "{qname}: def-use chains must be shared across a memo hit",
        );
        assert!(
            Arc::ptr_eq(&a.types, &b.types),
            "{qname}: the type lattice must be shared across a memo hit",
        );
        assert!(
            Arc::ptr_eq(&a.rendered_props, &b.rendered_props),
            "{qname}: rendered properties must be shared across a memo hit",
        );
        assert!(
            Arc::ptr_eq(&a.taints, &b.taints),
            "{qname}: the taint cascade result must be shared across a memo hit",
        );
    }
}

/// A per-procedure lattice is rebuilt through the public single-function
/// CFG entry point, so that entry point must receive the document's exact
/// registry surface. Tcl 8.4 has no builtin `throw`: a user procedure with
/// that name returns normally and the following statement stays reachable.
#[test]
fn tcl84_memoised_cfg_keeps_user_throw_call_fallthrough_reachable() {
    const SRC: &str = "proc throw {} {return ok}\n\
                           proc subject {} {throw; set after_throw 1}\n";
    let db = TclDatabase::default();
    let cfg_key = lexer_cfg_key(&db, "tcl8.4");
    let file = SourceFile::new(&db, SRC.to_owned(), "tcl8.4".to_owned(), None);
    let unit = compilation_unit(&db, file, cfg_key, 0).expect("no overlay always builds");
    let subject_id = original_test_body_id(&unit, SRC.find("proc subject").unwrap());
    let subject = unit
        .procedures
        .get(&subject_id)
        .expect("subject procedure lattice");

    assert!(
        subject.cfg.blocks.values().any(|block| {
            block.statements.iter().any(|stmt| {
                matches!(
                    stmt,
                    tcl_compiler::ir::Statement::AssignConst { name, .. }
                        if name == "after_throw"
                )
            })
        }),
        "Tcl 8.4 user proc `throw` must fall through to `after_throw`"
    );
}

#[test]
// Implementation contract: naming.variable.registry-event-frame-identity
// docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
// Implementation contract: naming.compiler.original-function-lattice-mutation-identity
// docs/design/analysis/name-resolution-proofs/compiler-original-function-lattice-mutation-identity.md
fn original_event_lattice_cache_retains_conditional_entry_and_reuses_same_source() {
    let source = "when HTTP_REQUEST {set local 1}";
    let db = TclDatabase::default();
    let config = lexer_cfg_key(&db, "f5-irules");
    let first_file = SourceFile::new(&db, source.to_owned(), "f5-irules".to_owned(), None);
    let second_file = SourceFile::new(&db, source.to_owned(), "f5-irules".to_owned(), None);
    let first = compilation_unit(&db, first_file, config, 0)
        .expect("zero overlay selects the retained stock F5 Registry");
    let second = compilation_unit(&db, second_file, config, 0)
        .expect("zero overlay selects the retained stock F5 Registry");
    let first_id = original_test_body_id(&first, 0);
    let second_id = original_test_body_id(&second, 0);
    let a = first.procedures.get(&first_id).unwrap();
    let b = second.procedures.get(&second_id).unwrap();
    assert!(
        Arc::ptr_eq(&a.def_use, &b.def_use),
        "the sealed event participates in the lattice memo"
    );
    for unit in [a, b] {
        let event = unit
            .irules_event_body
            .as_ref()
            .expect("actual event producer");
        assert_eq!(event.event(), "HTTP_REQUEST");
        assert!(event.matches_source(
            &tcl_lexer::SourceImage::document(source),
            first.ir_module.lexer_config
        ));
        let (&block, _) = unit
            .cfg
            .blocks
            .iter()
            .find(|(_, block)| !block.statements.is_empty())
            .unwrap();
        let point = unit
            .ssa
            .point_contexts
            .as_ref()
            .unwrap()
            .context_before(block, 0)
            .unwrap();
        assert_eq!(
            point.frame_kind,
            tcl_compiler::var_resolve::VariableFrameKind::Local
        );
        assert_eq!(
            point.hosted_execution_context,
            Some(tcl_registry::f5::BigIpExecutionContext::TmmIRule)
        );
        assert_eq!(
            point.execution, None,
            "memoisation cannot manufacture an executing worker"
        );
    }
}

fn original_event_test_context_snapshot(db: &dyn TclDb) -> (CfgContext<'_>, CompilerMemoSnapshot) {
    let context = CfgContext::new(
        db,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        BodyLatticeCommandBindings::from(ModuleCommandBindings::default()),
        OriginalProcedureSummaryOwners::default(),
    );
    let snapshot = CompilerMemoSnapshot {
        profile: Some(tcl_dialect::DialectProfile::irules().cache_key()),
        registry: db.registry("f5-irules").snapshot(),
    };
    (context, snapshot)
}

fn original_event_changed_mutations<'db>(
    db: &'db dyn TclDb,
    selected: FnLatticeKey<'db>,
) -> FnLatticeKey<'db> {
    FnLatticeKey::new(
        db,
        selected.body(db).clone(),
        selected.qname(db).clone(),
        selected.params(db).clone(),
        selected.context(db),
        selected.lexer_config(db),
        selected.dialect(db).clone(),
        selected.snapshot(db).clone(),
        selected.param_constants(db).clone(),
        selected.module_facts(db).clone(),
        FnLatticeEntry {
            original_declaration_body: None,
            plain_command_dispatch: selected.entry(db).plain_command_dispatch,
            metadata_context: selected.entry(db).metadata_context.clone(),
            source_metadata_input: selected.entry(db).source_metadata_input.clone(),
            irules_event_body: selected.entry(db).irules_event_body.clone(),
            command_trust: tcl_compiler::command_binding::ModuleCommandMutations::distrust_all()
                .snapshot(),
        },
        selected.analysis_context(db),
    )
}

type OriginalEventKeyBase<'db, 'a> = (
    &'db dyn TclDb,
    &'a CompilationUnit,
    CfgContext<'db>,
    &'a str,
);

type OriginalEventKeyRequest = (
    Script,
    tcl_lexer::LexerConfig,
    CompilerMemoSnapshot,
    Option<Arc<tcl_compiler::ir::SourceIrulesEventBody>>,
);

fn original_event_make_key<'db>(
    (db, unit, context, qname): OriginalEventKeyBase<'db, '_>,
    (body, config, snapshot, event): OriginalEventKeyRequest,
) -> FnLatticeKey<'db> {
    FnLatticeKey::new(
        db,
        BodyLatticeInputs::from(body),
        qname.to_owned(),
        Vec::new(),
        context,
        config,
        "f5-irules".to_owned(),
        snapshot,
        Vec::new(),
        FnLatticeModuleFacts {
            known_classes: Vec::new(),
            traced_variables: Vec::new(),
            has_dynamic_variable_trace: false,
        },
        FnLatticeEntry {
            original_declaration_body: None,
            plain_command_dispatch: false,
            metadata_context: unit.ir_module.source_entry.metadata_context.clone(),
            source_metadata_input: unit.ir_module.source_metadata_input.clone(),
            irules_event_body: event,
            command_trust: tcl_compiler::command_binding::ModuleCommandMutations::default()
                .snapshot(),
        },
        ValueTransferContext::of(
            db,
            tcl_compiler::value_transfer::AnalysisContextKey::for_module(
                &unit.command_mutations,
                unit.ir_module.resolved_registry(),
            ),
            0,
        ),
    )
}

#[test]
// Implementation contract: naming.variable.registry-event-frame-identity
// docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
fn original_event_lattice_key_rejects_foreign_source_body_config_and_registry() {
    let source = "when HTTP_REQUEST {set local 1}";
    let db = TclDatabase::default();
    let config_key = lexer_cfg_key(&db, "f5-irules");
    let file = SourceFile::new(&db, source.to_owned(), "f5-irules".to_owned(), None);
    let unit = compilation_unit(&db, file, config_key, 0)
        .expect("zero overlay selects the retained stock F5 Registry");
    let qname = "::when::HTTP_REQUEST";
    let body_id = original_test_body_id(&unit, 0);
    let event = unit.ir_module.irules_event_bodies.get(&body_id).unwrap();
    let procedure = unit.ir_module.procedures.get(&body_id).unwrap();
    assert_eq!(
        procedure.span.start(),
        0,
        "this actual declaration already owns the normalization origin"
    );
    let config = unit.ir_module.lexer_config.nested().normalized();
    let (context, snapshot) = original_event_test_context_snapshot(&db);
    let make_key = |body: Script, config, snapshot, event| {
        original_event_make_key(
            (&db, &unit, context, qname),
            (body, config, snapshot, event),
        )
    };
    let selected = make_key(
        procedure.body.clone(),
        config,
        snapshot.clone(),
        Some(Arc::clone(event)),
    );
    let plain = make_key(procedure.body.clone(), config, snapshot.clone(), None);
    assert!(
        selected != plain,
        "the same name and body cannot donate event entry"
    );
    // Implementation contract: naming.compiler.original-function-lattice-mutation-identity
    // docs/design/analysis/name-resolution-proofs/compiler-original-function-lattice-mutation-identity.md
    let changed_mutations = original_event_changed_mutations(&db, selected);
    assert!(
        selected != changed_mutations,
        "unchanged event/source/Registry/body cannot erase mutation obligations"
    );
    assert!(!function_lattice(&db, selected).complexity_guarded);
    assert!(function_lattice(&db, selected).irules_event_body.is_some());
    assert!(function_lattice(&db, plain).irules_event_body.is_none());
    let changed_file = SourceFile::new(
        &db,
        format!("{source}\n# changed source"),
        "f5-irules".to_owned(),
        None,
    );
    let changed_unit = compilation_unit(&db, changed_file, config_key, 0)
        .expect("zero overlay selects the retained stock F5 Registry");
    let changed_body_id = original_test_body_id(&changed_unit, 0);
    let foreign = make_key(
        procedure.body.clone(),
        config,
        snapshot.clone(),
        changed_unit
            .ir_module
            .irules_event_bodies
            .get(&changed_body_id)
            .cloned(),
    );
    assert!(
        selected != foreign,
        "complete original source participates in key equality"
    );
    assert!(function_lattice(&db, foreign).complexity_guarded);
    let wrong_body = make_key(
        Script::default(),
        config,
        snapshot.clone(),
        Some(Arc::clone(event)),
    );
    assert!(function_lattice(&db, wrong_body).complexity_guarded);
    let mut wrong_config = config;
    wrong_config.strict_quoting = !wrong_config.strict_quoting;
    let wrong_config_key = make_key(
        procedure.body.clone(),
        wrong_config,
        snapshot.clone(),
        Some(Arc::clone(event)),
    );
    assert!(function_lattice(&db, wrong_config_key).complexity_guarded);
    let wrong_registry = make_key(
        procedure.body.clone(),
        config,
        CompilerMemoSnapshot {
            profile: Some(tcl_lsp_core::profile_for_dialect("tcl8.6").cache_key()),
            registry: db.registry("tcl8.6").snapshot(),
        },
        Some(Arc::clone(event)),
    );
    assert!(function_lattice(&db, wrong_registry).complexity_guarded);
}

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
        type Module = <tcl_codegen::compile_service::BytecodeCompileService as tcl_vm::CompileService>::Module;

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
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
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

fn semantic_bundle_has_source_metadata(
    unit: &tcl_compiler::compilation_unit::FunctionUnit,
) -> bool {
    unit.semantic_facts
        .conditional_source_metadata()
        .invocations()
        .next()
        .is_some()
}

#[test]
fn function_lattice_semantic_bundle_uses_retained_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let db = TclDatabase::default();
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut puts = registry.get("puts").unwrap().clone();
    puts.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(puts);
    let registry = registry
        .project_for_profile(tcl_registry::model::resolve_environment("tcl8.6").unit_profile());
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    assert!(
        current
            .context()
            .resolve_spec(current.commands(), "puts")
            .is_some()
    );
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(current.commands(), config)
        .with_resolved_analysis_input(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            Arc::clone(&current),
            config,
        ));
    let module = lowerer.lower("puts VALUE").clone();
    let input = module.source_metadata_input.as_ref().unwrap();
    let selected = supplied_cfg_lattice_key(&db, &module, Some(input.clone()));
    assert!(semantic_bundle_has_source_metadata(&function_lattice(
        &db, selected
    )));
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    assert!(Arc::ptr_eq(older.commands(), current.commands()));
    assert!(
        older
            .context()
            .resolve_spec(older.commands(), "puts")
            .is_none()
    );
    let unavailable = supplied_cfg_lattice_key(
        &db,
        &module,
        Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        )),
    );
    assert!(selected != unavailable);
    assert!(!semantic_bundle_has_source_metadata(&function_lattice(
        &db,
        unavailable
    )));
    let foreign = supplied_cfg_lattice_key(
        &db,
        &module,
        Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            input.lexer_config(),
        )),
    );
    let missing = supplied_cfg_lattice_key(&db, &module, None);
    for negative in [foreign, missing] {
        assert!(selected != negative);
        assert!(matches!(
            function_lattice(&db, negative).semantic_facts.executable(),
            tcl_compiler::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
        ));
    }
}

#[test]
fn function_lattice_key_keeps_the_exact_normalized_lexer_config() {
    let db = TclDatabase::default();
    let context = CfgContext::new(
        &db,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        BodyLatticeCommandBindings::from(ModuleCommandBindings::default()),
        OriginalProcedureSummaryOwners::default(),
    );
    let make_key = |config: tcl_lexer::LexerConfig| {
        FnLatticeKey::new(
            &db,
            BodyLatticeInputs::from(Script::default()),
            "::subject".to_owned(),
            Vec::new(),
            context,
            config.normalized(),
            "tcl8.6".to_owned(),
            CompilerMemoSnapshot {
                profile: Some(tcl_lsp_core::profile_for_dialect("tcl8.6").cache_key()),
                registry: db.registry("tcl8.6").snapshot(),
            },
            Vec::new(),
            FnLatticeModuleFacts {
                known_classes: Vec::new(),
                traced_variables: Vec::new(),
                has_dynamic_variable_trace: false,
            },
            FnLatticeEntry {
                original_declaration_body: None,
                plain_command_dispatch: false,
                metadata_context:
                    tcl_compiler::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
                source_metadata_input: None,
                irules_event_body: None,
                command_trust: tcl_compiler::command_binding::ModuleCommandMutations::default()
                    .snapshot(),
            },
            ValueTransferContext::of(
                &db,
                tcl_compiler::value_transfer::AnalysisContextKey::detached(),
                0,
            ),
        )
    };

    let tcl_key = make_key(tcl_lexer::LexerConfig::for_dialect("tcl8.6"));
    let irules_key = make_key(tcl_lexer::LexerConfig::for_dialect("f5-irules"));
    let jim_config = tcl_lexer::LexerConfig::for_dialect("jim");
    let jim_key = make_key(jim_config);
    assert!(
        tcl_key != irules_key,
        "the iRules ghost-separator axis is part of memo identity"
    );
    assert!(
        tcl_key != jim_key,
        "Jim's vertical-tab word-separator axis is part of memo identity"
    );

    let shifted_jim_key = make_key(tcl_lexer::LexerConfig {
        base_offset: 91,
        base_line: 7,
        base_col: 13,
        ..jim_config
    });
    assert!(
        jim_key == shifted_jim_key,
        "position-only relocation must normalize out of memo identity"
    );

    let _cold_other_config = function_lattice(&db, tcl_key);
    let jim_cold = function_lattice(&db, jim_key);
    let jim_warm = function_lattice(&db, jim_key);
    assert!(
        Arc::ptr_eq(&jim_cold, &jim_warm),
        "the warm exact-config demand must reuse the cold lattice"
    );
}

/// Regression (whole-file-shift determinism): a pure prepend that shifts every
/// procedure must leave every `function_lattice` a cache hit — reliably, not by
/// HashMap-seed luck.  Before `prepare_cfg_context` was made deterministic, the
/// memo key flaked and this could re-execute *all* procedures.  Two procedures
/// share the short name `x` (the collision that exposed the nondeterminism).
#[test]
fn function_lattice_reused_on_whole_file_shift() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let l = Arc::clone(&log);
    let sink = move |ev: salsa::Event| {
        if let salsa::EventKind::WillExecute { database_key } = ev.kind {
            l.lock().unwrap().push(format!("{database_key:?}"));
        }
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let src = "namespace eval ::a { proc x {p} { set q $p; return $q } }\n\
                   namespace eval ::b { proc x {p} { set q $p; return $q } }\n\
                   proc top {} { set z 1 }\n";
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let file = SourceFile::new(&db, src.to_owned(), "tcl8.6".to_owned(), None);
    let _ = file_analysis_incremental(&db, file, cfg);
    log.lock().unwrap().clear();
    // Pure whole-file shift: prepend a blank line (every proc shifts, none
    // change).  All offset-0 lattice keys are unchanged -> all cache hits.
    file.set_text(&mut db).to(format!("\n{src}"));
    let _ = file_analysis_incremental(&db, file, cfg);
    let reexec = std::mem::take(&mut *log.lock().unwrap())
        .into_iter()
        .filter(|s| s.contains("function_lattice"))
        .count();
    assert_eq!(
        reexec, 0,
        "whole-file shift must reuse every proc lattice (deterministic key): {reexec} re-ran"
    );
}

/// A length-changing body edit to one procedure shifts the others but must
/// recompute exactly ONE `function_lattice` (the salsa-native per-procedure
/// lattice is offset-invariant: an unedited-but-shifted body interns to the
/// same key and is a cache hit, rebased to its new offset).
#[test]
fn function_lattice_reused_on_body_shift() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let file = SourceFile::new(
        &db,
        "proc a {} { set x 11111 }\nproc b {} { set y 22222 }\n".to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    let init = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        init.iter()
            .filter(|s| s.contains("function_lattice"))
            .count(),
        2,
        "initial: both procedures' lattices built: {init:?}"
    );

    // Edit a's body length — shifts b's offset; b's offset-0 body is
    // unchanged, so its `function_lattice` key is unchanged (cache hit).
    file.set_text(&mut db)
        .to("proc a {} { set x 9999999999 }\nproc b {} { set y 22222 }\n".to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    let after = std::mem::take(&mut *log.lock().unwrap());
    assert_eq!(
        after
            .iter()
            .filter(|s| s.contains("function_lattice"))
            .count(),
        1,
        "length-changing body edit -> exactly ONE lattice recomputes (offset-invariant): {after:?}"
    );
}

/// Constant parameter seeds participate in the lattice memo key.
/// An unrelated edit reuses the callee; changed caller input rebuilds it.
#[test]
fn function_lattice_memoises_param_constant_procs() {
    use salsa::Setter as _;
    let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let l = Arc::clone(&log);
        move |ev: salsa::Event| {
            if let salsa::EventKind::WillExecute { database_key } = ev.kind {
                l.lock().unwrap().push(format!("{database_key:?}"));
            }
        }
    };
    let lattice_execs = |log: &Arc<Mutex<Vec<String>>>| {
        std::mem::take(&mut *log.lock().unwrap())
            .into_iter()
            .filter(|s| s.contains("function_lattice"))
            .count()
    };
    let mut db = TclDatabase {
        storage: salsa::Storage::new(Some(Box::new(sink))),
    };
    let cfg = AnalyserConfig::new(
        &db,
        Vec::new(),
        NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    // `other` first so editing it shifts the two below; `target` takes a
    // param `caller` always passes the literal `42` for -> param_constants.
    let file = SourceFile::new(
        &db,
        "proc other {} { set z 1 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 42 }\n"
            .to_owned(),
        "tcl8.6".to_owned(),
        None,
    );
    let _ = file_analysis_incremental(&db, file, cfg);
    assert_eq!(
        lattice_execs(&log),
        3,
        "cold build: every proc (incl. the param-constant callee) memoised"
    );

    // Length-changing edit to `other` shifts `target`/`caller`; their
    // offset-0 bodies (and `target`'s param_constants) are unchanged, so
    // they are cache hits — exactly one lattice recomputes.
    file.set_text(&mut db)
        .to("proc other {} { set z 123456789 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 42 }\n"
            .to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    assert_eq!(
        lattice_execs(&log),
        1,
        "unrelated body edit -> param-constant callee reused (offset-invariant hit)"
    );

    // Change the caller's literal -> `target`'s param_constants change ->
    // its `FnLatticeKey` re-interns -> its lattice rebuilds.
    file.set_text(&mut db)
        .to("proc other {} { set z 123456789 }\n\
             proc target {x} { set y $x }\n\
             proc caller {} { target 99 }\n"
            .to_owned());
    let _ = file_analysis_incremental(&db, file, cfg);
    assert!(
        lattice_execs(&log) >= 1,
        "caller literal change rebuilds the param-constant callee's lattice"
    );
}
