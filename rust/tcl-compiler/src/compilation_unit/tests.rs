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

use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

fn fixture_method<'a>(unit: &'a CompilationUnit, label: &str) -> Option<&'a FunctionUnit> {
    let (id, _) = unit.ir_module.methods.unique_display(label)?;
    unit.methods.get(&id)
}

use super::*;

#[test]
fn authored_fixture_lookup_keeps_actual_body_owner_without_publication() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for("proc ::p {} {set x 1}", &registry, false);
    assert!(unit.ir_module.procedures.is_empty());
    let function = unit.function("::p").expect("original analysis body");
    let id = function.body_id.expect("actual issued body");
    assert!(std::ptr::eq(
        function,
        unit.function_for_body(Some(id))
            .expect("same stamped store")
    ));
    let procedure = unit.original_procedure_for_body(id).expect("original card");
    assert!(procedure.publication.is_none());
    assert!(procedure.body.native_compilation_admission.is_none());
    assert!(
        unit.ir_module
            .original_native_declaration_body_units
            .is_empty()
    );
    assert!(unit.function("::missing").is_none());
}

#[test]
fn authored_fixture_lookup_refuses_duplicate_reporting_labels() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(
        "proc ::p {} {return FIRST}; proc ::p {} {return SECOND}",
        &registry,
        false,
    );
    assert_eq!(
        unit.ir_module
            .original_authored_declaration_body_units
            .len(),
        2
    );
    assert!(unit.function("::p").is_none());
}

fn assert_standalone_currency_withdrawals(
    source: &str,
    options: UnitBuildOptions<'_>,
    input: &crate::analyser::ResolvedAnalysisInput,
) {
    let mut changed_config = options.config;
    changed_config.leading_bom = tcl_lexer::LeadingBom::Skip;
    assert!(
        CompilationUnit::build_standalone_with_analysis_input(
            source,
            UnitBuildOptions {
                config: changed_config,
                ..options
            },
            input
        )
        .is_none()
    );
    assert!(
        CompilationUnit::build_standalone_with_analysis_input(
            source,
            UnitBuildOptions {
                dialect: tcl_dialect::DialectProfile::find("tcl8.6"),
                ..options
            },
            input
        )
        .is_none()
    );
    let foreign_store = CommandRegistry::build_default();
    assert!(
        CompilationUnit::build_standalone_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: &foreign_store,
                ..options
            },
            input
        )
        .is_none()
    );
}

#[test]
fn standalone_source_input_keeps_authoring_purpose_and_exact_metadata_currency() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut store = CommandRegistry::build_default();
    store.insert(tcl_registry::CommandSpec {
        name: "standaloneRetainedStoreMarker",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let store = Arc::new(store);
    let context = tcl_registry::model::ingress::resolve_environment("tcl9.0")
        .default_context_registry()
        .with_command_store(Arc::clone(&store));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::new(context),
        tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Content,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        },
    );
    let source = "set value [expr {1 + 2}]";
    let options = UnitBuildOptions {
        registry: &store,
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    };
    let standalone = CompilationUnit::build_standalone_with_analysis_input(source, options, &input)
        .expect("explicit Standalone source request with actual complete input");
    assert_eq!(
        standalone.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    assert_eq!(
        standalone.ir_module.registry_snapshot.as_ref(),
        Some(&store.snapshot())
    );
    assert_eq!(standalone.ir_module.lexer_config, input.lexer_config());
    assert!(standalone.ir_module.source_entry.native_entry.is_none());
    assert!(!standalone.ir_module.source_entry.unknown_entry);
    assert!(
        standalone
            .ir_module
            .source_entry
            .invocation_dialect
            .is_some()
    );
    assert!(
        standalone
            .ir_module
            .source_entry
            .logical_source_input
            .is_none()
    );
    // The same metadata does not promote an ordinary supplied-document
    // request to Standalone; with no entry it starts a fresh interpreter.
    let supplied = CompilationUnit::build_with_analysis_input(source, options, None, &input);
    assert_eq!(
        supplied.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    assert!(supplied.ir_module.source_entry.native_entry.is_none());
    assert!(!supplied.ir_module.source_entry.unknown_entry);
    assert!(
        !supplied
            .ir_module
            .source_entry
            .metadata_context
            .is_standalone()
    );
    assert_standalone_currency_withdrawals(source, options, &input);
}

#[test]
fn function_consumers_keep_the_actual_generation_and_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = Arc::new(CommandRegistry::build_default());
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let actual = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(&registry)),
    );
    let config = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let unit = CompilationUnit::build_with_context_registry(
        "proc p {x} {set y $x; return $y}; namespace eval n {set z 1}",
        UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        Arc::clone(&actual),
    );
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    assert!(Arc::ptr_eq(&input.context_registry(), &actual));
    assert!(!unit.procedures.is_empty());
    let body_id = fixture_id(&unit.ir_module.procedures, "::p");
    assert!(std::ptr::eq(
        unit.function_for_body(Some(body_id)).unwrap(),
        std::ptr::from_ref(&unit.procedures[&body_id])
    ));
    assert!(std::ptr::eq(
        unit.function_for_body(None).unwrap(),
        std::ptr::from_ref(&unit.top_level)
    ));
    let mut wrong_stamp = unit.clone();
    wrong_stamp.procedures.get_mut(&body_id).unwrap().body_id = None;
    assert!(wrong_stamp.function_for_body(Some(body_id)).is_none());
    wrong_stamp.top_level.body_id = Some(body_id);
    assert!(wrong_stamp.function_for_body(None).is_none());
    let foreign_unit = CompilationUnit::build_with_context_registry(
        &unit.source,
        UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        Arc::clone(&actual),
    );
    let foreign_id = fixture_id(&foreign_unit.ir_module.procedures, "::p");
    let mut foreign_owner = unit.clone();
    foreign_owner.ir_module.procedure_ids = foreign_unit.ir_module.procedure_ids.clone();
    for (&id, function) in unit.methods.iter().chain(unit.body_units.iter()) {
        assert!(std::ptr::eq(
            unit.function_for_body(Some(id))
                .expect("actual retained body unit"),
            function
        ));
        assert!(foreign_owner.function_for_body(Some(id)).is_none());
    }
    assert!(foreign_owner.function_for_body(Some(body_id)).is_none());
    assert!(unit.function_for_body(Some(foreign_id)).is_none());
    for function in std::iter::once(&unit.top_level)
        .chain(unit.procedures.values())
        .chain(unit.methods.values())
        .chain(unit.body_units.values())
    {
        assert_eq!(function.source_metadata_input(), Some(input));
        assert_eq!(function.source_lexer_config(), config);
        let selected = function.invocation_metadata_context(&registry).unwrap();
        assert!(std::ptr::eq(selected.context(), actual.context()));
        let foreign = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").unit_profile(),
        );
        assert!(function.invocation_metadata_context(&foreign).is_none());
        let mut missing = function.clone();
        missing.source_metadata_input = None;
        assert!(missing.invocation_metadata_context(&registry).is_none());
    }
}

fn assert_original_taint_owner_withdrawals(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    analysis: &InterproceduralAnalysis,
    body: crate::ir::ProcedureId,
    profile: &'static tcl_dialect::DialectProfile,
    source: &str,
) {
    let mut missing = unit.clone();
    missing.ir_module.retained_source_bindings = None;
    assert!(
        missing
            .original_interproc_taints_for_body(Some(body), registry, analysis)
            .is_none()
    );
    let mut missing = unit.clone();
    missing.ir_module.source_metadata_input = None;
    assert!(
        missing
            .original_interproc_taints_for_body(Some(body), registry, analysis)
            .is_none()
    );
    let mut missing = unit.clone();
    missing.top_level.source_metadata_input = None;
    assert!(
        missing
            .original_interproc_taints_for_body(None, registry, analysis)
            .is_none()
    );
    let mut changed = unit.clone();
    changed.ir_module.source = tcl_lexer::SourceImage::document(&format!("{source}; puts CHANGED"));
    assert!(
        changed
            .original_interproc_taints_for_body(Some(body), registry, analysis)
            .is_none()
    );
    let mut foreign_registry = registry.project_for_profile(profile);
    let mut extra = registry.get("puts").unwrap().clone();
    extra.name = "taint_source_owner_control";
    foreign_registry.insert(extra);
    assert!(
        unit.original_interproc_taints_for_body(Some(body), &foreign_registry, analysis)
            .is_none()
    );
    let mut missing_analysis = analysis.clone();
    missing_analysis.global_instance_class_owner = None;
    assert!(
        unit.original_interproc_taints_for_body(Some(body), registry, &missing_analysis)
            .is_none()
    );
}

#[test]
fn original_module_taint_replay_preserves_receivers_and_refuses_foreign_owners() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tk").default_context_registry();
    let registry = context.commands();
    let profile = tcl_registry::model::ingress::resolve_environment("tk").unit_profile();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let source = "entry .e; proc danger {} {set value [.e get]; exec $value}";
    let build = |source| {
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        )
    };
    let unit = build(source);
    let body = *unit
        .ir_module
        .original_source_procedure_bodies()
        .next()
        .unwrap()
        .0;
    let direct = unit.clone().with_interprocedural(registry, Some(profile));
    let analysis = direct.interproc.as_ref().unwrap();
    assert!(analysis.global_instance_classes[&body].contains_key(".e"));
    let expected = direct.function_for_body(Some(body)).unwrap();
    assert!(expected.taints.values().any(|taint| taint.is_tainted()));
    assert_eq!(
        unit.original_interproc_taints_for_body(Some(body), registry, analysis),
        Some(expected.taints.as_ref().clone()),
    );
    let mut observed = 0;
    let memoised = unit.clone().with_interprocedural_memoized(
        registry,
        Some(profile),
        &mut |actual_body, _, _| {
            assert_eq!(actual_body, body);
            observed += 1;
            None
        },
    );
    assert_eq!(observed, 1);
    assert_eq!(memoised.top_level.taints, direct.top_level.taints);
    assert_eq!(
        memoised.function_for_body(Some(body)).unwrap().taints,
        expected.taints
    );

    let other = build("entry .other; proc danger {} {set value [.other get]; exec $value}");
    let foreign_body = *other
        .ir_module
        .original_source_procedure_bodies()
        .next()
        .unwrap()
        .0;
    assert!(
        unit.original_interproc_taints_for_body(Some(foreign_body), registry, analysis)
            .is_none()
    );
    let other = other.with_interprocedural(registry, Some(profile));
    assert!(
        unit.original_interproc_taints_for_body(
            Some(body),
            registry,
            other.interproc.as_ref().unwrap()
        )
        .is_none()
    );
    assert_original_taint_owner_withdrawals(&unit, registry, analysis, body, profile, source);
}

#[test]
fn module_realm_refusal_withholds_caller_seeds_and_retires_interprocedural_results() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let source = "proc retained {x} {list $x}; retained VALUE";
    let options = UnitBuildOptions {
        registry,
        defer_top_level: false,
        config,
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    };
    let (module, cfg, _, prepared) =
        lower_and_build_cfg(source, options, (None, None), None, None, None, None);
    assert!(crate::realm::document_realm_bindings_for_module(&module, registry).is_some());
    let (actual, _, _) = resolve_unit_scope(&module, &cfg, Some(&prepared), options);
    assert_eq!(
        actual.get("::retained").unwrap().uniform_literal_at(0),
        Some("VALUE")
    );
    let mut missing = module.clone();
    missing.retained_source_bindings = None;
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("proc other {} {};");
    let mut missing_snapshot = module;
    missing_snapshot.registry_snapshot = None;
    for refused in [missing, changed, missing_snapshot] {
        let (unknown, _, _) = resolve_unit_scope(&refused, &cfg, Some(&prepared), options);
        let caller = unknown.get("::retained").unwrap();
        assert!(caller.opaque_caller);
        assert_eq!(caller.uniform_literal_at(0), None);
    }
    let completed = CompilationUnit::build_for_profile(source, registry, false, profile)
        .with_interprocedural(registry, Some(profile));
    assert!(completed.interproc.is_some());
    let mut missing = completed.clone();
    missing.ir_module.retained_source_bindings = None;
    let ordinary = missing
        .clone()
        .with_interprocedural(registry, Some(profile));
    assert!(ordinary.interproc.is_none());
    let mut callbacks = 0;
    let mut callback = |_: crate::ir::ProcedureId, _: &str, _: &InterproceduralAnalysis| {
        callbacks += 1;
        None
    };
    let memoised = missing.with_interprocedural_memoized(registry, Some(profile), &mut callback);
    assert!(memoised.interproc.is_none());
    assert_eq!(callbacks, 0);
}

fn semantic_bundle_has_source_metadata(unit: &crate::compilation_unit::FunctionUnit) -> bool {
    unit.semantic_facts
        .conditional_source_metadata()
        .invocations()
        .next()
        .is_some()
}

#[test]
fn semantic_bundle_requires_retained_availability_and_source_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // These are source metadata and availability assertions, not Native entry facts.
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut puts = registry.get("puts").unwrap().clone();
    puts.name = "metadata_current";
    puts.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(puts);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let registry = current.commands();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&current),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let unit = CompilationUnit::build_with_analysis_input(
        "metadata_current VALUE",
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let selected = &unit.top_level;
    assert_eq!(
        selected.semantic_facts.metadata_context(),
        Some(current.context())
    );
    assert!(semantic_bundle_has_source_metadata(selected));
    let input = selected.source_metadata_input().unwrap();
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(registry)),
    );
    assert!(Arc::ptr_eq(older.commands(), registry));
    let mut unavailable = selected.clone();
    unavailable.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        older,
        input.lexer_config(),
    ));
    let unavailable = unavailable.with_retained_semantic_analysis(
        registry,
        Some(&unit.ir_module.top_level),
        crate::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
    );
    assert!(!semantic_bundle_has_source_metadata(&unavailable));
    let mut foreign = selected.clone();
    foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    ));
    let mut missing = selected.clone();
    missing.source_metadata_input = None;
    let mut different_grammar = selected.clone();
    different_grammar.source_config.strict_quoting =
        !different_grammar.source_config.strict_quoting;
    for negative in vec![foreign, missing, different_grammar].into_boxed_slice() {
        let refused = negative.with_retained_semantic_analysis(
            registry,
            Some(&unit.ir_module.top_level),
            crate::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
        );
        assert!(matches!(
            refused.semantic_facts.executable(),
            crate::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
        ));
    }
}

#[test]
fn retained_deep_sidecars_keep_availability_and_refuse_missing_function_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = baseline.commands().project_for_profile(profile);
    let mut puts = registry.get("puts").unwrap().clone();
    puts.name = "metadata_current";
    puts.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(puts);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let registry = current.commands();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&current),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let unit = CompilationUnit::build_with_analysis_input(
        "metadata_current VALUE",
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let selected = unit.clone().with_retained_deep_semantic_analysis(registry);
    assert!(semantic_bundle_has_source_metadata(&selected.top_level));
    assert!(
        selected
            .top_level
            .semantic_facts
            .executable()
            .world_state_ssa()
            .is_some()
    );
    let input = unit.top_level.source_metadata_input().unwrap();
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(registry)),
    );
    let mut unavailable = unit.clone();
    unavailable.top_level.source_metadata_input =
        Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            older,
            input.lexer_config(),
        ));
    unavailable.ir_module.source_metadata_input =
        unavailable.top_level.source_metadata_input.clone();
    let unavailable = unavailable.with_retained_deep_semantic_analysis(registry);
    assert!(!semantic_bundle_has_source_metadata(&unavailable.top_level));
    let mut missing = unit.clone();
    missing.top_level.source_metadata_input = None;
    let mut foreign = unit.clone();
    foreign.top_level.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    ));
    let mut stale = unit.clone();
    stale.top_level.source_config.strict_quoting = !stale.top_level.source_config.strict_quoting;
    for negative in vec![missing, foreign, stale].into_boxed_slice() {
        let refused = negative.with_retained_deep_semantic_analysis(registry);
        assert!(matches!(
            refused.top_level.semantic_facts.executable(),
            crate::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
        ));
    }
}

#[test]
fn retained_deep_sidecars_require_the_module_source_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Full source sidecars borrow Module scripts; no Native entry is established.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let mut unit = retained_memory_fixture(&current);
    let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
    let mut missing = unit.clone();
    missing.ir_module.source_metadata_input = None;
    let mut stale = unit.clone();
    stale.ir_module.lexer_config.strict_quoting = !stale.ir_module.lexer_config.strict_quoting;
    let mut different_owner = unit.clone();
    different_owner.ir_module.source_metadata_input =
        Some(crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            Arc::new(current.with_command_store(Arc::clone(registry))),
            input.lexer_config(),
        ));
    assert!(semantic_bundle_has_source_metadata(
        &unit
            .clone()
            .with_retained_deep_semantic_analysis(registry)
            .top_level
    ));
    for refused in vec![missing, stale, different_owner].into_boxed_slice() {
        let refused = refused.with_retained_deep_semantic_analysis(registry);
        assert!(matches!(
            refused.top_level.semantic_facts.executable(),
            crate::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
        ));
    }
    // FU's explicit API owns its own supplied script independently of a Module.
    unit.ir_module.source_metadata_input = None;
    let standalone = unit.top_level.clone().with_retained_semantic_analysis(
        registry,
        Some(&unit.ir_module.top_level),
        crate::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
    );
    assert!(semantic_bundle_has_source_metadata(&standalone));
}

fn retained_memory_fixture(context: &Arc<tcl_registry::model::ContextRegistry>) -> CompilationUnit {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    CompilationUnit::build_with_context_registry(
        "dict create key value",
        UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        Arc::clone(context),
    )
}

#[test]
fn retained_memory_facts_require_matching_module_and_function_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // This checks metadata admission and opaque memory obligations, not Native entry.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let unit = retained_memory_fixture(&current);
    let original = unit.top_level.source_metadata_input().unwrap();
    let annotated = unit.clone().with_retained_memory_ssa(registry);
    assert!(
        !annotated
            .top_level
            .memory_ssa
            .unwrap()
            .has_wildcard_aliasing
    );
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(registry)),
    );
    let mut unavailable = unit.clone();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        Arc::clone(&older),
        original.lexer_config(),
    );
    unavailable.ir_module.source_metadata_input = Some(input.clone());
    unavailable.top_level.source_metadata_input = Some(input);
    assert!(
        unavailable
            .top_level
            .invocation_metadata_context_for_module(registry, &unavailable.ir_module)
            .is_some()
    );
    let mut missing_module = unit.clone();
    missing_module.ir_module.source_metadata_input = None;
    let mut missing_function = unit.clone();
    missing_function.top_level.source_metadata_input = None;
    let mut stale_module = unit.clone();
    stale_module.ir_module.lexer_config.strict_quoting =
        !stale_module.ir_module.lexer_config.strict_quoting;
    let mut stale_function = unit.clone();
    stale_function.top_level.source_config.strict_quoting =
        !stale_function.top_level.source_config.strict_quoting;
    let mut different_owner = unit.clone();
    different_owner.ir_module.source_metadata_input =
        Some(crate::analyser::ResolvedAnalysisInput::new(
            original.analyser_profile(),
            original.unit_profile(),
            older,
            original.lexer_config(),
        ));
    let mut foreign = unit.clone();
    foreign.top_level.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        original.lexer_config(),
    ));
    let mut different_profile = unit.clone();
    different_profile.top_level.source_metadata_input =
        Some(crate::analyser::ResolvedAnalysisInput::new(
            tcl_dialect::DialectProfile::find("tcl8.4").unwrap(),
            original.unit_profile(),
            Arc::clone(&current),
            original.lexer_config(),
        ));
    for refused in vec![
        missing_module,
        missing_function,
        stale_module,
        stale_function,
        different_owner,
        foreign,
        different_profile,
    ]
    .into_boxed_slice()
    {
        assert!(
            refused
                .top_level
                .invocation_metadata_context_for_module(registry, &refused.ir_module)
                .is_none()
        );
        let refused = refused.with_retained_memory_ssa(registry);
        assert!(refused.top_level.memory_ssa.unwrap().has_wildcard_aliasing);
    }
    let unavailable = unavailable.with_retained_memory_ssa(registry);
    assert!(
        unavailable
            .top_level
            .memory_ssa
            .unwrap()
            .has_wildcard_aliasing
    );
}

fn retained_memo_fixture_unit(
    req: &LatticeRequest<'_>,
    registry: &CommandRegistry,
) -> FunctionUnit {
    let prepared = PreparedCfgContext::from_source_input(
        (
            req.upvar_procs.clone(),
            req.proc_params.clone(),
            req.global_write_procs.clone(),
            req.command_bindings.clone(),
            req.procedure_summary_owners.clone(),
        ),
        registry,
        req.source_metadata_input,
    );
    let cfg = crate::cfg_builder::build_cfg_function_with_prepared_context(
        req.qname,
        req.body,
        true,
        registry,
        req.plain_command_dispatch,
        &prepared,
        req.lexer_config,
    );
    let known_classes = req.known_classes.iter().cloned().collect();
    let traced_variables = req.traced_variables.iter().cloned().collect();
    let parameters = decode_param_constants(req.param_constants);
    FunctionUnit::build_for_lattice(
        req.qname,
        cfg,
        req.params,
        FunctionLatticeInputs {
            dialect: UnitDialect {
                registry,
                config: req.lexer_config,
                source_metadata_input: req.source_metadata_input,
            },
            param_constants: parameters.as_ref(),
            known_classes: &known_classes,
            trace_facts: ModuleTraceFacts {
                traced_variables: &traced_variables,
                has_dynamic_variable_trace: req.has_dynamic_variable_trace,
                deferred_writes: &req.analysis_context.deferred_writes,
            },
            analysis_context: req.analysis_context,
            command_trust: req.command_trust,
            event_body: req.irules_event_body.map(|event| (event, req.body)),
        },
    )
    .with_retained_semantic_analysis(
        registry,
        Some(req.body),
        crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
    )
}

#[test]
fn memoised_source_facts_retain_nested_bom_and_lexer_overlays() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig {
        escapes: tcl_dialect::EscapeSyntax::Tcl84,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let mut requests = 0;
    let mut memo = |req: &LatticeRequest<'_>| {
        requests += 1;
        let input = req.source_metadata_input.unwrap();
        assert!(Arc::ptr_eq(&input.context_registry(), &current));
        assert_eq!(input.lexer_config(), config.nested().normalized());
        assert_eq!(input.lexer_config(), req.lexer_config);
        assert_eq!(input.unit_profile().cache_key(), profile.cache_key());
        assert_eq!(
            req.original_metadata_context(registry),
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                input
            )),
        );
        let wrong_config = LatticeRequest {
            lexer_config: tcl_lexer::LexerConfig {
                strict_quoting: !req.lexer_config.strict_quoting,
                ..req.lexer_config
            },
            ..*req
        };
        assert!(matches!(
            wrong_config.original_metadata_context(registry),
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable
        ));
        let wrong_profile = LatticeRequest {
            dialect: Some(tcl_dialect::DialectProfile::find("tcl9.1").unwrap()),
            ..*req
        };
        assert!(matches!(
            wrong_profile.original_metadata_context(registry),
            crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable
        ));
        let selected = retained_memo_fixture_unit(req, registry);
        assert!(semantic_bundle_has_source_metadata(&selected));
        let mut stale = selected.clone();
        stale.source_config.escapes = tcl_dialect::EscapeSyntax::Tcl86;
        assert!(stale.invocation_metadata_context(registry).is_none());
        let stale = stale.with_retained_semantic_analysis(
            registry,
            Some(req.body),
            crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
        );
        assert!(matches!(
            stale.semantic_facts.executable(),
            crate::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
        ));
        selected
    };
    let unit = CompilationUnit::build_with_entry(
        r"proc p {} {::puts \U00000041}",
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&mut memo),
        None,
        None,
        Some(Arc::clone(&current)),
    );
    assert_eq!(requests, 1);
    assert!(semantic_bundle_has_source_metadata(
        unit.procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("authored fixture body")
    ));
    assert!(
        unit.procedures
            .get(&fixture_id(&unit.procedures, "::p"))
            .expect("authored fixture body")
            .invocation_metadata_context_for_module(registry, &unit.ir_module)
            .is_some()
    );
    let deep = unit.with_retained_deep_semantic_analysis(registry);
    assert!(semantic_bundle_has_source_metadata(
        deep.procedures
            .get(&fixture_id(&deep.procedures, "::p"))
            .expect("authored fixture body")
    ));
}

#[test]
fn supplied_source_input_retains_axes_and_independent_logical_entry() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source declarations and aliases are conditional authored facts, not Native entry.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&current),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let options = UnitBuildOptions {
        registry: current.commands(),
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(input.unit_profile()),
        external_call_sites: None,
        declared_commands: None,
    };
    let unit = CompilationUnit::build_with_analysis_input(
        "proc aliaser {} {global ::n::shared; set shared 1}",
        options,
        None,
        &input,
    );
    assert_eq!(unit.ir_module.source_metadata_input.as_ref(), Some(&input));
    assert_eq!(
        unit.ir_module.source_entry.logical_source_input.as_ref(),
        Some(&input)
    );
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    assert!(fixture_id_if_present(&unit.procedures, "::aliaser").is_some());
    let explicit = crate::command_binding::SourceAnalysisEntry::default();
    let unknown = CompilationUnit::build_with_analysis_input(
        "proc aliaser {} {global ::n::shared; set shared 1}",
        options,
        Some(&explicit),
        &input,
    );
    assert!(
        unknown
            .ir_module
            .source_entry
            .logical_source_input
            .is_none()
    );
    assert!(unknown.ir_module.source_entry.native_entry.is_none());
    assert_eq!(
        unknown.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    let native_profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let native = crate::analyser::ResolvedAnalysisInput::new(
        native_profile,
        native_profile,
        Arc::clone(&current),
        input.lexer_config(),
    );
    let native_unit = CompilationUnit::build_with_analysis_input(
        "puts VALUE",
        UnitBuildOptions {
            dialect: Some(native_profile),
            ..options
        },
        None,
        &native,
    );
    assert!(
        native_unit
            .ir_module
            .source_entry
            .logical_source_input
            .is_none()
    );
    assert!(native_unit.ir_module.source_entry.native_entry.is_none());
    // No supplied entry: a fresh interpreter, not unknown history.
    assert!(!native_unit.ir_module.source_entry.unknown_entry);
    assert_eq!(
        native_unit.ir_module.source_metadata_input.as_ref(),
        Some(&native)
    );
}

#[test]
fn default_unit_source_profile_keeps_explicit_standalone_authoring_purpose() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let options = UnitBuildOptions {
        registry: context.commands(),
        config,
        dialect: Some(profile),
        defer_top_level: false,
        external_call_sites: None,
        declared_commands: None,
    };
    let source = "proc local {value} {set result $value; return $result}";
    // A dialect-bearing default build is a whole document in a fresh
    // interpreter of that dialect (user decision 2026-10-19T14:20Z):
    // `tclsh8.6` defines `::local`, so the unit publishes it.
    let fresh = CompilationUnit::build_with_options(source, options);
    assert!(fresh.ir_module.source_metadata_input.is_some());
    assert!(fresh.function("::local").is_some());
    // Explicit Standalone authoring remains the dialect-blind build.
    let options = UnitBuildOptions {
        dialect: None,
        ..options
    };
    let unit = CompilationUnit::build_with_options(source, options);
    let module = &unit.ir_module;
    assert!(module.source_entry.metadata_context.is_standalone());
    assert!(module.source_metadata_input.is_none());
    assert!(module.source_entry.native_entry.is_none());
    assert_eq!(module.original_authored_declaration_body_units.len(), 1);
    assert!(module.original_native_declaration_body_units.is_empty());
    let (id, card) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let procedure = module.body_units.get(id).unwrap();
    assert!(card.matches_procedure(module, procedure));
    assert!(procedure.publication.is_none());
    assert!(procedure.body.executed_source.is_none());
    assert!(procedure.body.native_compilation_admission.is_none());
    let header = module
        .top_level
        .retained_source_tokens_for_statement(&module.top_level.statements[0])
        .unwrap();
    assert!(
        crate::registry_invocation::admitted_native_compiler_invocation(
            context.commands(),
            None,
            header
        )
        .is_none()
    );
}

#[test]
fn default_source_purpose_does_not_replace_supplied_input_or_unknown_driver_entry() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let options = UnitBuildOptions {
        registry: context.commands(),
        config,
        dialect: Some(profile),
        defer_top_level: false,
        external_call_sites: None,
        declared_commands: None,
    };
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let source = "proc original {value} {return $value}";
    let supplied = CompilationUnit::build_with_analysis_input(source, options, None, &input);
    assert_eq!(
        supplied.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    assert!(
        !supplied
            .ir_module
            .source_entry
            .metadata_context
            .is_standalone()
    );
    assert!(
        supplied
            .ir_module
            .original_authored_declaration_body_units
            .is_empty()
    );
    // No supplied entry: a fresh tclsh defines `::original` (user decision
    // 2026-10-19T14:20Z); an unentered card needs explicit unknown history.
    assert!(supplied.function("::original").is_some());
    let unknown_supplied = CompilationUnit::build_with_analysis_input(
        source,
        options,
        Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
        &input,
    );
    assert_eq!(
        unknown_supplied
            .ir_module
            .original_native_declaration_body_units
            .len(),
        1
    );
    let entry = crate::command_binding::SourceAnalysisEntry {
        unknown_entry: true,
        ..Default::default()
    };
    let unknown = CompilationUnit::build_with_source_entry(source, options, &entry);
    assert!(unknown.ir_module.source_entry.unknown_entry);
    assert!(
        unknown
            .ir_module
            .original_authored_declaration_body_units
            .is_empty()
    );
}

#[test]
fn supplied_source_producer_refuses_changed_profile_and_lexer_metadata() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual input is retained for reporting; unrelated producer options grant no facts.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let options = UnitBuildOptions {
        registry: context.commands(),
        defer_top_level: false,
        config: input.lexer_config(),
        dialect: Some(profile),
        external_call_sites: None,
        declared_commands: None,
    };
    let selected = CompilationUnit::build_with_analysis_input("puts VALUE", options, None, &input);
    assert!(
        selected
            .top_level
            .invocation_metadata_context_for_module(context.commands(), &selected.ir_module,)
            .is_some()
    );
    let mut wrong_config = options;
    wrong_config.config.strict_quoting = !wrong_config.config.strict_quoting;
    let wrong_profile = UnitBuildOptions {
        dialect: Some(tcl_dialect::DialectProfile::find("tcl8.6").unwrap()),
        ..options
    };
    for options in [wrong_config, wrong_profile] {
        let refused =
            CompilationUnit::build_with_analysis_input("puts VALUE", options, None, &input);
        assert_eq!(
            refused.ir_module.source_metadata_input.as_ref(),
            Some(&input)
        );
        assert!(refused.ir_module.source_entry.unknown_entry);
        assert!(
            refused
                .ir_module
                .source_entry
                .logical_source_input
                .is_none()
        );
        assert!(refused.top_level.source_metadata_input().is_none());
        assert!(
            refused
                .top_level
                .invocation_metadata_context_for_module(context.commands(), &refused.ir_module,)
                .is_none()
        );
        assert!(matches!(
            refused.top_level.semantic_facts.executable(),
            crate::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
        ));
    }
    let mut stale_module = selected.ir_module.clone();
    stale_module.dialect_profile = wrong_profile.dialect;
    assert!(
        selected
            .top_level
            .invocation_metadata_context_for_module(context.commands(), &stale_module,)
            .is_none()
    );
}

#[test]
fn supplied_hosted_source_entry_retains_advice_without_activation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    for environment in ["f5-irules", "f5-iapps"] {
        let environment = tcl_registry::model::ingress::resolve_environment(environment);
        let profile = environment.unit_profile();
        let context = environment.default_context_registry();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let unit = CompilationUnit::build_with_analysis_input(
            "set source_value VALUE",
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let entry = &unit.ir_module.source_entry;
        assert_eq!(entry.vendor_source_input.as_ref(), Some(&input));
        assert!(entry.native_entry.is_none());
        assert!(entry.logical_source_input.is_none());
        assert!(entry.hosted_execution_context.is_none());
        // Documented correction (user decision 2026-10-21T00:10Z,
        // "realm-aware command world"): a hosted document supplied without
        // an entry is a fresh rule load where the dialect admits the
        // authored F5 naming simulation; elsewhere history stays unknown.
        if let Some(fresh) =
            crate::command_binding::SourceAnalysisEntry::for_fresh_hosted_source(&input)
                .filter(|fresh| !fresh.unknown_entry)
        {
            assert!(!entry.unknown_entry);
            assert_eq!(
                entry.invocation_realm,
                tcl_dialect::model::InvocationRealm::RuleLoader
            );
            assert_eq!(entry.execution_name_policy, fresh.execution_name_policy);
        } else {
            assert!(entry.unknown_entry);
            assert!(entry.execution_name_policy.is_none());
        }
    }
}

fn test_context() -> SemanticContext {
    SemanticContext::for_environment("tcl8.6")
}

fn registry() -> CommandRegistry {
    // These fixtures assert concrete C Tcl behaviour, including 9.1 grammar.
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    )
}

#[test]
fn semantic_value_projection_uses_actual_jim_entry_over_c_catalogue() {
    let registry = registry();
    // The actual interpreter entry selects Jim independently of the C catalogue.
    // This software fixture observes no external provider execution.
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let context = Arc::new(
        tcl_registry::model::ingress::static_context_for("jim")
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let (_runtime_owner, captured) =
        crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        metadata_context:
            crate::registry_invocation::OwnedInvocationMetadataContext::for_source_input(Some(
                &input,
            )),
        native_entry: Some(Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            ..Default::default()
        },
        ..Default::default()
    };
    let source =
        "set x 003; set y [expr {$x}]; set z [expr {9223372036854775807 + 1}]; expr {$z + 1}";
    let compiled = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: &registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    );
    assert!(compiled.ir_module.source_entry.native_entry.is_some());
    assert_eq!(
        compiled.ir_module.source_metadata_input.as_ref(),
        Some(&input)
    );
    let unit = &compiled.top_level;
    let facts = unit
        .semantic_values()
        .expect("retained actual engine inputs");
    assert!(facts.contents_iter().any(|(_, value)| {
        *value
            == crate::analyses::LatticeValue::Const(crate::analyses::ConstValue::String(
                "003".to_owned(),
            ))
    }));
    assert!(facts.expression_iter().any(|(_, analysis)| {
        analysis.evaluation.value == crate::tcl_expr_eval::TclValue::Int(i64::MIN)
    }));
    assert!(facts.expression_iter().any(|(_, analysis)| {
        analysis.evaluation.value == crate::tcl_expr_eval::TclValue::Int(i64::MIN + 1)
    }));
    assert_eq!(
        unit.semantic_value_projection,
        Arc::new(unit.semantic_value_projection.uncached())
    );
    assert!(std::ptr::eq(facts, unit.semantic_values().unwrap()));
}

#[test]
fn prepare_cfg_context_retains_same_tailed_body_parameters() {
    let reg = registry();
    let src = "namespace eval ::a { proc x {p1} { set q $p1 } }
namespace eval ::b { proc x {p2 extra} { set q $p2 } }
";
    let m = crate::lowering::lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::default());
    let (_, proc_params, _, _, _) = crate::cfg_builder::prepare_cfg_context(&m);
    let a = fixture_id(&m.procedures, "::a::x");
    let b = fixture_id(&m.procedures, "::b::x");
    assert_ne!(a, b);
    assert_eq!(proc_params.len(), 2);
    assert_eq!(proc_params.get(&a), Some(&vec!["p1".to_string()]));
    assert_eq!(
        proc_params.get(&b),
        Some(&vec!["p2".to_string(), "extra".to_string()])
    );
}

#[test]
fn build_for_empty_source() {
    let cu = CompilationUnit::build_for("", &registry(), false);
    assert_eq!(cu.source, "");
    assert_eq!(cu.top_level.name, "::top");
    assert!(cu.procedures.is_empty());
}

#[test]
fn prepared_binding_mutation_projection_matches_compatibility_scan() {
    let reg = registry();
    let cu = CompilationUnit::build_for(
        "rename string saved-string\nrename saved-string string\n",
        &reg,
        false,
    );
    assert_eq!(
        cu.command_mutations,
        crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg),
    );
}

/// Every method unit carries exactly one instance-state carrier
/// (`method_facts`), and its content matches the typed IR — the
/// invariant that keeps the existence fold (I230/O100/O101), the
/// W210/W211/W220 `known_bound` set, and the optimiser's method-constants
/// escaping set reading the same struct.
#[test]
fn method_units_carry_the_single_method_facts_carrier() {
    let src = "oo::class create C {\n\
                       variable state\n\
                       method m {arg} { ::return $arg }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = fixture_method(&cu, "::C::m").expect("method unit built");
    let facts = fu
        .method_facts
        .as_deref()
        .expect("a method unit must carry method_facts");
    assert_eq!(facts.params, vec!["arg".to_string()]);
    assert!(facts.instance_vars.contains("state"));
    let known = facts.known_bound_at_entry();
    assert!(known.contains("arg") && known.contains("state"));
    // The IR the carrier was built from agrees (same construction site).
    let ir = cu
        .ir_module
        .methods
        .unique_display("::C::m")
        .map(|(_, method)| method)
        .expect("method IR");
    assert_eq!(facts.instance_vars, ir.instance_vars);
    assert_eq!(facts.params, ir.params);
}

/// TN: bodies with no object state — the top level and plain
/// procedures — carry no `method_facts`, so no consumer can mistake a
/// proc frame for a method frame.
#[test]
fn non_method_units_carry_no_method_facts() {
    let src = "proc p {a} { return $a }\nset x 1\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    assert!(cu.top_level.method_facts.is_none());
    let fu = fixture_id_if_present(&cu.procedures, "::p")
        .and_then(|id| cu.procedures.get(&id))
        .expect("proc unit built");
    assert!(fu.method_facts.is_none());
}

#[test]
fn build_for_single_statement() {
    let cu = CompilationUnit::build_for("set x 1", &registry(), false);
    assert!(!cu.top_level.cfg.blocks.is_empty());
    // SCCP should mark entry executable for a non-empty CFG.
    assert!(
        cu.top_level
            .sccp
            .executable_blocks
            .contains(&cu.top_level.ssa.entry)
    );
}

/// A reached callee write withdraws the old value at later reads without
/// poisoning the value that existed before the call.
fn assert_callee_global_write_is_positioned(name: &str) {
    use crate::analyses::{ConstValue, LatticeValue};
    let src = format!(
        "set {name} 4\nset before ${name}\nproc helper {{}} {{global {name}; set {name} 17}}\nhelper\nset after ${name}\nif {{${name} == 4}} {{set stale BAD}}\n"
    );
    let cu = CompilationUnit::build_for(&src, &registry(), false);
    let before = cu
        .top_level
        .sccp
        .values
        .iter()
        .filter(|((symbol, _), _)| cu.top_level.ssa.var_name(*symbol) == "before")
        .collect::<Vec<_>>();
    assert!(
        before.iter().any(
            |(_, value)| matches!(value, LatticeValue::Const(ConstValue::Int(4)))
                || matches!(value, LatticeValue::Const(ConstValue::String(value)) if value == "4")
        ),
        "prior value must remain represented: {before:?}"
    );
    let after = cu
        .top_level
        .sccp
        .values
        .iter()
        .filter(|((symbol, _), _)| cu.top_level.ssa.var_name(*symbol) == "after")
        .collect::<Vec<_>>();
    assert!(!after.is_empty(), "post-call read must remain represented");
    assert!(
        after.iter().all(
            |(_, value)| !matches!(value, LatticeValue::Const(ConstValue::Int(4)))
                && !matches!(value, LatticeValue::Const(ConstValue::String(value)) if value == "4")
        ),
        "post-call read must not reuse the stale value: {after:?}"
    );
    assert!(
        cu.top_level
            .sccp
            .constant_branches
            .iter()
            .all(|selected| !selected.value),
        "stale condition cannot select its true body"
    );
}

#[test]
fn top_level_var_touched_by_callee_global_is_overdefined() {
    assert_callee_global_write_is_positioned("g");
}

#[test]
fn top_level_dual_ported_var_touched_by_callee_global_is_overdefined() {
    assert_callee_global_write_is_positioned("tcl_precision");
}

/// Control: a top-level name *no* procedure ever `global`-declares must
/// still fold to a genuine `Const` — the whole-module scan must not
/// over-widen every top-level variable (that would be a precision
/// regression, not a soundness fix).
#[test]
fn top_level_var_untouched_by_any_callee_still_folds() {
    let reg = registry();
    let src = "set safe_const 42\nproc other {} { puts unrelated }\nother\n";
    let cu = CompilationUnit::build_for(src, &reg, false);
    let sym = cu
        .top_level
        .ssa
        .var_symbol("safe_const")
        .expect("top-level `safe_const` should be interned");
    let has_const = cu
        .top_level
        .sccp
        .values
        .iter()
        .any(|((s, _), lv)| *s == sym && matches!(lv, crate::analyses::LatticeValue::Const(_)));
    assert!(has_const, "expected `safe_const` to still fold to a Const");
}

/// Existence is a deep-tier fact: a procedure lattice requested
/// at the fast tier runs no rung, so every read of it answers
/// `Unavailable(Fast)` — neither bound nor unbound — and W210 and W213
/// stay silent where the deep build reports both; a unit over the
/// complexity ceiling answers `Unavailable(ComplexityGuarded)`.
#[test]
fn existence_is_unavailable_at_the_fast_tier() {
    use tcl_core_types::DiagCode;
    let reg = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let src = "proc p {} {set x 1; unset x; unset x; puts $x}\n";
    let lifecycle = |cu: CompilationUnit| -> Vec<DiagCode> {
        let mut analyser = crate::analyser::Analyser::new();
        analyser.set_cu_override(Arc::new(cu));
        analyser
            .analyse(src, "tcl8.6")
            .diagnostics
            .into_iter()
            .map(|d| d.code)
            .filter(|code| matches!(code, DiagCode::W210 | DiagCode::W213))
            .collect()
    };
    let deep = CompilationUnit::build_for_dialect(src, reg, false, "tcl8.6");
    let deep_p = deep.function("::p").expect("::p built");
    assert_eq!(deep_p.tier, AnalysisTier::Deep);
    let x = deep_p.ssa.var_symbol("x").expect("x interned");
    let entry = deep_p.cfg.entry;
    let cfg = deep_p.cfg.clone();
    assert!(matches!(
        deep_p.existence(x, ExistencePoint::Exit(entry)),
        FactView::Domain(DomainFact::Existence(_))
    ));

    let key = crate::value_transfer::AnalysisContextKey::detached().at_tier(AnalysisTier::Fast);
    let fast_p = FunctionUnit::build_with_param_constants_and_classes_under(
        "::p",
        cfg.clone(),
        &[],
        UnitDialect {
            registry: reg,
            config: tcl_lexer::LexerConfig::default(),
            source_metadata_input: None,
        },
        None,
        &HashSet::new(),
        ModuleAnalysisFacts {
            trace: ModuleTraceFacts::none(),
            analysis_context: &key,
            command_trust: &crate::command_binding::ModuleCommandMutations::default(),
        },
    );
    assert_eq!(fast_p.tier, AnalysisTier::Fast);
    assert!(fast_p.sccp.existence.is_empty());
    for point in [
        ExistencePoint::Exit(entry),
        ExistencePoint::Before(entry, 0),
        ExistencePoint::Version(1),
    ] {
        assert_eq!(
            fast_p.existence(x, point),
            FactView::Top(DeclineReason::Unavailable(AnalysisTier::Fast))
        );
    }
    let mut fast = deep.clone();
    let p_id = fixture_id(&deep.ir_module.procedures, "::p");
    fast.procedures.insert(p_id, fast_p);

    let found = lifecycle(deep);
    assert!(
        found.contains(&DiagCode::W213) && found.contains(&DiagCode::W210),
        "{found:?}"
    );
    assert_eq!(lifecycle(fast), Vec::<DiagCode>::new());

    let guarded = FunctionUnit::trivial_guarded("::p", cfg);
    assert_eq!(
        guarded.existence(x, ExistencePoint::Exit(entry)),
        FactView::Top(DeclineReason::Unavailable(AnalysisTier::ComplexityGuarded))
    );
}

#[test]
fn complexity_guard_flags_byte_huge_proc() {
    // A normal proc is analysed (not guarded), with real SSA + SCCP.
    let normal = CompilationUnit::build_for("proc small {} { set x 1 }", &registry(), false);
    let small = normal.function("::small").expect("::small built");
    assert!(!small.complexity_guarded);
    assert!(!small.ssa.blocks.is_empty());

    // A block-light but byte-huge body (one statement with a ~270 KB
    // literal) trips the body-byte half of the complexity guard: the unit
    // is flagged and carries a trivial SSA, so the O(blocks·vars) walk and
    // the dataflow passes never run on it.
    let big_literal = "A".repeat(270_000);
    let src = format!("proc big {{}} {{ set x \"{big_literal}\" }}");
    let cu = CompilationUnit::build_for(&src, &registry(), false);
    let big = cu.function("::big").expect("::big built");
    assert!(big.complexity_guarded, "byte-huge body must be guarded");
    assert!(big.ssa.blocks.is_empty(), "guarded unit has trivial SSA");
    assert!(big.sccp.values.is_empty());
    // Guarded functions are excluded from the analysable-function view the
    // per-proc diagnostic and optimiser passes iterate.
    assert!(
        cu.analysable_functions().all(|fu| fu.name != "::big"),
        "guarded ::big must be skipped by analysable_functions"
    );
}

#[test]
fn build_for_captures_procedures() {
    let cu = CompilationUnit::build_for("proc greet {name} {puts $name}", &registry(), false);
    assert!(!cu.procedures.is_empty());
    assert!(cu.function("::greet").is_some());
    assert!(cu.function("::top").is_some());
    // No OO methods in a plain proc source.
    assert!(cu.methods.is_empty());
}

#[test]
fn colliding_method_reports_keep_independent_analysis_units() {
    let source = "namespace eval A {}\noo::class create A::B {method c {} {return LEFT}}\noo::class create A {method B::c {} {return RIGHT}; method <constructor> {} {return NAMED}; constructor {} {set local ACTUAL}; export B::c <constructor>}\n";
    let unit = CompilationUnit::build_for(source, &registry(), false);
    assert_eq!(unit.ir_module.methods.len(), 4);
    assert_eq!(unit.methods.len(), 4);
    for (id, method) in &unit.ir_module.methods {
        let function = unit
            .methods
            .get(id)
            .expect("every retained method has its own analysis unit");
        assert_eq!(function.body_id, Some(*id));
        assert_eq!(function.name, method.reporting_label());
    }
}

#[test]
fn build_for_lowers_oo_methods_to_function_units() {
    // TclOO method bodies get their own
    // FunctionUnit (CFG → SSA → analysis) in `cu.methods`, kept
    // separate from `procedures` so the optimiser's O126 gate can
    // iterate them. `procedures` stays free of method qnames.
    let src = "oo::class create Counter {\n\
                   \x20   variable n\n\
                   \x20   method bump {} { ::incr n }\n\
                   \x20   method get {} { ::return $n }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let mut keys: Vec<&String> = cu.methods.values().map(|method| &method.name).collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            &"::Counter::bump".to_string(),
            &"::Counter::get".to_string()
        ],
        "method units: {keys:?}",
    );
    // Each method unit carries a real analysis pipeline (its CFG
    // has the entry block at minimum).
    let get = fixture_method(&cu, "::Counter::get").expect("unique retained fixture method");
    assert_eq!(get.name, "::Counter::get");
    assert!(!get.cfg.blocks.is_empty());
    // Methods are NOT mixed into the per-proc map.
    assert!(
        !cu.procedures
            .values()
            .any(|procedure| procedure.name == "::Counter::get")
    );
}

#[test]
fn runtime_selected_method_commands_guard_deep_cfg_analysis() {
    let cu = CompilationUnit::build_for(
        "interp alias {} e {} expr\n\
             oo::class create C {\n\
                 method relative {} {set x 1}\n\
                 method absolute {} {::set x 1}\n\
                 method absolute_dispatch {} {::set x [my absolute]}\n\
                 method absolute_expr {p} {::return [::expr {$p + 1}]}\n\
                 method absolute_info {p} {::if {[::info exists p]} {::puts yes}}\n\
                 method absolute_regex {s} {::set re {a+}; ::regexp $re $s}\n\
                 method nested_alias {p} {::set x [e {$p + 1}]}\n\
             }",
        &registry(),
        false,
    );
    assert!(
        fixture_method(&cu, "::C::relative")
            .expect("unique retained fixture method")
            .complexity_guarded,
        "a receiver-local command may shadow the lexical builtin"
    );
    assert!(
        !fixture_method(&cu, "::C::absolute")
            .expect("unique retained fixture method")
            .complexity_guarded,
        "an absolute-only method is independent of the receiver namespace"
    );
    assert!(
        fixture_method(&cu, "::C::absolute_dispatch")
            .expect("unique retained fixture method")
            .complexity_guarded,
        "the receiver namespace can shadow TclOO's injected dispatch commands"
    );
    for name in [
        "::C::absolute_expr",
        "::C::absolute_info",
        "::C::absolute_regex",
    ] {
        assert!(
            !fixture_method(&cu, name)
                .expect("unique retained fixture method")
                .complexity_guarded,
            "absolute-only method {name} must retain deep analysis: {:#?}",
            fixture_method(&cu, name).expect("unique retained fixture method")
        );
    }
    assert!(
        fixture_method(&cu, "::C::nested_alias")
            .expect("unique retained fixture method")
            .complexity_guarded,
        "a relative nested binding consumed by typed lowering remains runtime-selected"
    );
}

#[test]
fn switch_subject_counts_as_param_use() {
    // The `switch -- $col` subject lowers to an
    // `ExprNode::Raw` branch condition.  The expr var-scan now
    // recovers `$col`, so the parameter's def-use chain has a
    // live (terminator) use instead of looking dead — the precise
    // path behind W214 (unused parameter).  `col` is referenced
    // *only* as the switch subject here, so a live chain proves
    // the Raw scan reached the def-use builder.
    let cu = CompilationUnit::build_for(
        "proc p {col} { switch -- $col { a {set y 1} } }",
        &registry(),
        false,
    );
    let fu = cu.function("::p").expect("proc ::p should exist");
    let col_live = fu.def_use.chains.iter().any(|(key, chain)| {
        fu.def_use
            .symbol(&fu.ssa, key.0)
            .is_some_and(|symbol| fu.ssa.var_name(symbol) == "col")
            && !chain.is_dead()
    });
    assert!(
        col_live,
        "switch subject `$col` should register a live use; chains: {:?}",
        fu.def_use.chains.keys().collect::<Vec<_>>(),
    );
}

#[test]
fn switch_glob_arm_body_read_counts_as_param_use() {
    // A `switch -glob`/`-regexp` arm body is a real analysed CFG region,
    // so a parameter referenced *only* inside a glob-arm body has a live
    // def-use chain — the precise path behind W214.
    // Multi-arg arm form (the single-braced-body form is a separate,
    // pre-existing lowering gap affecting every mode equally).
    for (mode, pat) in [("-glob", "a*"), ("-regexp", "a.*")] {
        let src = format!("proc p {{val}} {{ switch {mode} -- $col {pat} {{puts $val}} }}");
        let cu = CompilationUnit::build_for(&src, &registry(), false);
        let fu = cu.function("::p").expect("proc ::p should exist");
        let val_live = fu.def_use.chains.iter().any(|(key, chain)| {
            fu.def_use
                .symbol(&fu.ssa, key.0)
                .is_some_and(|symbol| fu.ssa.var_name(symbol) == "val")
                && !chain.is_dead()
        });
        assert!(
            val_live,
            "{mode} arm-body read `$val` should register a live use; chains: {:?}",
            fu.def_use.chains.keys().collect::<Vec<_>>(),
        );
    }
}

#[test]
fn return_type_infers_int_literal() {
    let cu = CompilationUnit::build_for("proc f {} { return 1 }", &registry(), false);
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(fu.return_type, TypeLattice::of(tcl_registry::TclType::Int));
}

#[test]
fn return_type_infers_string_literal() {
    let cu = CompilationUnit::build_for("proc f {} { return \"hi\" }", &registry(), false);
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(
        fu.return_type,
        TypeLattice::of(tcl_registry::TclType::String)
    );
}

#[test]
fn return_type_follows_local_var() {
    let cu = CompilationUnit::build_for("proc f {} { set x 1; return $x }", &registry(), false);
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(fu.return_type, TypeLattice::of(tcl_registry::TclType::Int));
}

#[test]
fn return_type_joins_branches_to_common_int() {
    let cu = CompilationUnit::build_for(
        "proc f {a} { if {$a} { return 1 } else { return 2 } }",
        &registry(),
        false,
    );
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(fu.return_type, TypeLattice::of(tcl_registry::TclType::Int));
}

#[test]
fn return_type_partial_return_widens_via_fallthrough() {
    // `if {$a} { return 1 }` with no else: the false path falls off
    // the end of the body (Tcl returns the last command's result),
    // so the joined return type must not be a confident `Int` — the
    // fall-through exit widens it to Overdefined.
    let cu = CompilationUnit::build_for("proc f {a} { if {$a} { return 1 } }", &registry(), false);
    let fu = cu.function("::f").expect("proc ::f");
    assert_eq!(
        fu.return_type,
        TypeLattice::overdefined(),
        "partial-return proc must widen, got {:?}",
        fu.return_type,
    );
}

#[test]
fn with_memory_ssa_populates_optional() {
    let cu = CompilationUnit::build_for("set x 1", &registry(), false)
        .with_memory_ssa(&registry(), Some(test_context()));
    assert!(cu.top_level.memory_ssa.is_some());
}

#[test]
fn semantic_bundle_keeps_dialect_and_defers_unneeded_world_state() {
    let linear = CompilationUnit::build_for_profile(
        "puts hello",
        &registry(),
        false,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let facts = &linear.top_level.semantic_facts;
    assert_eq!(
        facts.metadata_context(),
        linear
            .top_level
            .source_metadata_input()
            .map(|input| { input.borrowed_context_registry().context() }),
    );
    let executable = facts.executable();
    assert_eq!(executable.invocations().count(), 1);
    assert!(executable.world_state_ssa().is_none());
    assert!(matches!(
        executable,
        crate::semantic_analysis::ExecutableAnalysisAvailability::WorldStateNotRequired { .. }
    ));
    assert_eq!(executable.completion_inputs().count(), 1);
    assert_eq!(executable.effect_inputs().count(), 1);

    let structured = CompilationUnit::build_for_profile(
        "if {1} { puts hello }",
        &registry(),
        false,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let structured = structured.top_level.semantic_facts.executable();
    assert!(structured.function().is_some());
    // `if` is projected into executable edges, so it is a structured
    // region rather than an opaque compatibility barrier.
    assert_eq!(structured.opaque_regions().count(), 0);
    assert_eq!(structured.structured_regions().count(), 1);
    assert!(structured.world_state_ssa().is_none());

    let ir = crate::lowering::lower_to_ir("puts hello", &registry());
    let explicit = crate::semantic_analysis::SemanticAnalysisBundle::build(
        &registry(),
        Some(test_context()),
        &ir.top_level,
        crate::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
    );
    assert!(explicit.executable().world_state_ssa().is_some());

    let legacy = CompilationUnit::build_standalone_for("puts hello", &registry(), false);
    assert!(matches!(
        legacy.top_level.semantic_facts.executable(),
        crate::semantic_analysis::ExecutableAnalysisAvailability::ContextUnavailable
    ));
}

#[test]
fn deep_semantic_analysis_materialises_every_retained_body_sidecar() {
    let deep = CompilationUnit::build_for_profile(
        "proc p {} { puts hello }\nnamespace eval ::n { puts body }\n",
        &registry(),
        false,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .with_deep_semantic_analysis(&registry(), Some(test_context()));
    // The top-level source contains a structural `namespace eval` whose
    // completion switch is not yet representable by the common world-SSA
    // planner. Deep inspection must still retain executable facts and the
    // typed `WorldStateDeclined` outcome rather than requiring a graph.
    assert!(
        deep.top_level
            .semantic_facts
            .executable()
            .function()
            .is_some()
    );
    assert!(
        fixture_id_if_present(&deep.procedures, "::p")
            .and_then(|id| deep.procedures.get(&id))
            .unwrap()
            .semantic_facts
            .executable()
            .function()
            .is_some()
    );
    assert!(
        deep.body_units
            .values()
            .all(|unit| unit.semantic_facts.executable().function().is_some())
    );
}

#[test]
fn functions_iterator_yields_top_plus_procs() {
    let cu = CompilationUnit::build_for(
        "proc foo {} {return 1}\nproc bar {} {return 2}",
        &registry(),
        false,
    );
    let count = cu.functions().count();
    assert_eq!(count, cu.procedures.len() + 1);
}

/// Interprocedural call-site literal seeding (TP/FP/TN/FN suite for
/// [`collect_call_site_constants`] / [`params_constants_from_native_call_sites`]).
mod call_site_param_constants {
    use super::*;
    use crate::analyses::LatticeValue;

    /// FN regression: a call from inside a `TclOO` method body to an
    /// ordinary user proc is a real caller with a differing argument,
    /// but methods are built in a *separate* pass that runs after the
    /// call-site scan. Without
    /// `build_extra_call_site_scan_contexts` the scan sees only the one
    /// external `helper a` call and wrongly seeds `mode` as the constant
    /// `"a"`.
    #[test]
    fn call_site_inside_tcloo_method_body_is_not_missed() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                oo::class create Widget {
                    method go {} { helper b }
                }
                helper a
                [Widget new] go
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(f, "mode"),
            "helper is called with both \"a\" (external) and \"b\" (from the method body); must not fold: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// Imported calls retain the original implementation allocation and
    /// contribute their differing arguments to the shared caller evidence.
    #[test]
    fn call_site_via_namespace_import_alias_is_not_missed() {
        let reg = registry();
        let src = "
                namespace eval ::lib {
                    namespace export helper
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                }
                namespace eval ::app {
                    namespace import ::lib::helper
                    proc go {} { helper b }
                }
                ::lib::helper a
                ::app::go
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::lib::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(f, "mode"),
            "helper is called with both \"a\" (direct) and \"b\" (via the ::app import); must not fold: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// A real wildcard import requires the source namespace to export
    /// its procedure. Publishing that command retains an open caller set.
    const WILDCARD_IMPORT_SRC: &str = "
            namespace eval ::lib {
                namespace export helper
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                }
            }
            namespace eval ::app {
                namespace import ::lib::*
                proc go {} { helper prod }
            }
            ::lib::helper prod
            ::app::go
        ";

    #[test]
    fn wildcard_namespace_import_preserves_actual_identity_and_open_callers() {
        let reg = registry();
        let cu = CompilationUnit::build_for(WILDCARD_IMPORT_SRC, &reg, false);
        let bindings = crate::command_binding::SourceCommandBindings::analyse(
            WILDCARD_IMPORT_SRC,
            tcl_lexer::LexerConfig::default(),
            &reg,
        );
        let imported = bindings.invocation_at(
            "helper",
            "::app",
            u32::try_from(WILDCARD_IMPORT_SRC.find("helper prod").unwrap()).unwrap(),
        );
        let direct = bindings.invocation_at(
            "::lib::helper",
            "::",
            u32::try_from(WILDCARD_IMPORT_SRC.find("::lib::helper prod").unwrap()).unwrap(),
        );
        let imported_target = imported
            .proved_handler_target()
            .expect("reached imported proc");
        let direct_target = direct.proved_handler_target().expect("reached direct proc");
        assert_eq!(imported_target.command, "::lib::helper");
        assert_eq!(imported_target.identity, direct_target.identity);
        assert_eq!(
            imported_target.implementation_allocation,
            direct_target.implementation_allocation,
        );
        assert_eq!(imported.evaluated_written_argument_value(0), Some("prod"));
        assert_eq!(direct.evaluated_written_argument_value(0), Some("prod"));
        let function = fixture_id_if_present(&cu.procedures, "::lib::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(!knows_condition_mentioning(function, "mode"));
        assert_eq!(
            function.sccp.constant_branches,
            [] as [crate::sccp::ConstantBranch; 0]
        );
    }

    #[test]
    fn wildcard_import_without_export_does_not_manufacture_a_caller() {
        let reg = registry();
        let source = WILDCARD_IMPORT_SRC.replace("namespace export helper", "");
        let bindings = crate::command_binding::SourceCommandBindings::analyse(
            &source,
            tcl_lexer::LexerConfig::default(),
            &reg,
        );
        let imported = bindings.invocation_at(
            "helper",
            "::app",
            u32::try_from(source.find("helper prod").unwrap()).unwrap(),
        );
        assert!(imported.proved_handler_target().is_none());
        assert!(
            imported
                .targets
                .iter()
                .all(|target| target.command != "::lib::helper")
        );
    }

    /// Wildcard publication retains an open caller set even when a host
    /// supplies a closed view of the files in its current workspace.
    /// An export publishes `::lib::helper` for *any* other unit to import
    /// and call with a different literal, including one in a different
    /// checkout that no project enumeration can reach.
    #[test]
    fn exported_namespace_declines_the_seed_even_with_a_workspace_view() {
        let reg = registry();
        let exported = WILDCARD_IMPORT_SRC.replace(
            "namespace eval ::lib {",
            "namespace eval ::lib {\n                namespace export *",
        );
        for cu in vec![
            CompilationUnit::build_for(&exported, &reg, false),
            build_closed_world(&exported, &reg),
        ]
        .into_boxed_slice()
        {
            let f = fixture_id_if_present(&cu.procedures, "::lib::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(f, "mode"),
                "`namespace export *` publishes helper beyond any enumerable project: {:?}",
                f.sccp.constant_branches,
            );
        }
    }

    /// A proc reached only through a
    /// `namespace ensemble` `-map` redirection is a real caller the
    /// call-site scan cannot resolve — it has no model of ensemble
    /// dispatch. tclsh8.6/9.0-confirmed: with `namespace ensemble
    /// create -command myens -map {go helper}`, `myens go dev` prints
    /// `helper mode=dev`, so `mode` genuinely varies and must not fold.
    ///
    /// Nothing pinned that today. What makes it safe is *indirect*: the
    /// registry marks `namespace ensemble` with `Traits::EXPORTS_COMMAND`,
    /// which `params_constants_from_call_sites` treats as a boundary
    /// publishing the file's commands to callers it cannot enumerate, so
    /// the whole module declines seeding. Sound but blunt — narrowing
    /// that trait, or resolving *some* ensemble maps, would silently
    /// reopen the false-fold shape here. This test guards the mechanism,
    /// whatever replaces it: the fold must stay off unless a real
    /// ensemble-map resolution exists.
    #[test]
    fn ensemble_map_redirected_caller_does_not_fold_issue_979() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                }
                helper prod
                helper prod
                namespace ensemble create -command myens -map {go helper}
                myens go dev
            ";
        for cu in vec![
            CompilationUnit::build_for(src, &reg, false),
            build_closed_world(src, &reg),
        ]
        .into_boxed_slice()
        {
            let f = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(f, "mode"),
                "`myens go dev` reaches helper with \"dev\" (tclsh8.6/9.0-confirmed), \
                     so mode is not caller-invariant: {:?}",
                f.sccp.constant_branches,
            );
        }
    }

    /// FP guard: a `TclOO` method body resolves
    /// bare commands against the GLOBAL namespace, never the class's own
    /// namespace — tclsh8.6-confirmed live: `[::foo::Widget new] go`
    /// (method body `helper b`) calls `::helper`, never
    /// `::foo::Widget::helper`, even though the latter exists and is
    /// exactly what naively deriving the caller's namespace from the
    /// method's own qualified name (`::foo::Widget::go` → `::foo::Widget`)
    /// would try first. Before forcing method bodies to resolve against
    /// global, this misattributed the method's call to
    /// `::foo::Widget::helper` (folding a condition on a call that never
    /// actually happens) while simultaneously losing it as evidence for
    /// the real target `::helper` (which then wrongly folded on its one
    /// remaining, external-only literal).
    #[test]
    fn method_body_bare_call_resolves_against_global_not_class_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    oo::class create Widget {
                        method go {} { helper b }
                    }
                }
                namespace eval ::foo::Widget {
                    proc helper {mode} {
                        if {$mode eq \"WRONG\"} { set r 1 } else { set r 2 }
                    }
                }
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                helper a
                [::foo::Widget new] go
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let global_helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::helper analysed");
        let ns_helper = fixture_id_if_present(&cu.procedures, "::foo::Widget::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::Widget::helper analysed");
        assert!(
            !knows_condition_mentioning(global_helper, "mode"),
            "::helper is called with both \"a\" and \"b\" (the method body, once \
                 correctly resolved to global): {:?}",
            global_helper.sccp.constant_branches,
        );
        assert!(
            !knows_condition_mentioning(ns_helper, "mode"),
            "::foo::Widget::helper is never actually called by real Tcl semantics, \
                 so no call-site evidence should ever reach it: {:?}",
            ns_helper.sccp.constant_branches,
        );
    }

    /// FN regression: `uplevel #0
    /// { … }`'s body resolves bare commands against the GLOBAL
    /// namespace — tclsh8.6/9.0-confirmed live: `uplevel #0 { helper b
    /// }` inside `::foo::runIt` prints `GLOBAL helper mode=b`, never the
    /// `::foo::helper` sitting in the enclosing namespace.
    /// `Statement::UpFrame`'s body survives CFG construction as a block
    /// *statement*, which `scan_cfg_callers` (walking only
    /// `Call`/`Barrier`) skips entirely, so this real, differing call
    /// site would vanish from `::helper`'s evidence, leaving its one
    /// remaining caller's `"a"` to fold the condition.
    ///
    /// `build_extra_call_site_scan_contexts` builds a bare CFG for
    /// every *absolute* shift-`0` `UpFrame` body, resolved as `"::top"`
    /// — mirroring how a `TclOO` method body is forced global.
    ///
    /// `uplevel N` for any relative (non-`#0`) level stays out of scope:
    /// the target frame's namespace depends on the live call stack,
    /// which single-file static analysis cannot decide — a documented,
    /// permanent approximation.
    #[test]
    fn uplevel_zero_body_resolves_against_global_not_enclosing_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"WRONG\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        uplevel #0 { helper b }
                    }
                }
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let global_helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::helper analysed");
        let ns_helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            !knows_condition_mentioning(global_helper, "mode"),
            "::helper is called with both \"a\" and \"b\" (uplevel #0, once correctly \
                 resolved to global): {:?}",
            global_helper.sccp.constant_branches,
        );
        assert!(
            !knows_condition_mentioning(ns_helper, "mode"),
            "::foo::helper is never actually called by real Tcl semantics: {:?}",
            ns_helper.sccp.constant_branches,
        );
    }

    /// MISCOMPILE regression: `uplevel 0 { … }` is the *relative*
    /// current-frame form and must NOT be treated as the absolute global
    /// form. Encoding `#0` and `0` as the same `frame_shift == 0` resolves
    /// this body against `"::top"`, losing `::foo::helper`'s only varying
    /// call site — folding a branch that real Tcl reaches both ways.
    ///
    /// On tclsh8.6 and tclsh9.0: inside
    /// `::foo::runIt`, `uplevel #0 { helper b }` prints `GLOBAL helper`
    /// while `uplevel 0 { helper c }` prints `FOO helper`.
    #[test]
    fn uplevel_bare_zero_body_resolves_against_the_enclosing_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        uplevel 0 { helper b }
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "`uplevel 0` runs in the current frame, so ::foo::helper sees both \
                 \"a\" (direct) and \"b\" (the uplevel body): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// Approximation pin: a *relative* `uplevel N { … }`
    /// keeps resolving against the enclosing unit's own namespace. The
    /// target frame's namespace depends on the live call stack, so this
    /// is a deliberate, permanent approximation — but it must stay an
    /// approximation that still *counts* the call site, not one that
    /// drops it. `::foo::helper` here sees both `"a"` and `"b"`.
    #[test]
    fn uplevel_relative_body_keeps_the_enclosing_units_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        uplevel 1 { helper b }
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "::foo::helper sees both \"a\" (direct) and \"b\" (the relative uplevel \
                 body, approximated to the enclosing namespace): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// FN guard for the `Traits::DEFINES_PROCEDURE` body-recursion skip
    /// a definition body is not re-walked from the
    /// definition site, so its call sites must still arrive through the
    /// defined procedure's *own* CFG. A conditionally-defined proc is
    /// the shape most at risk — its `proc` call is not a plain top-level
    /// statement.
    #[test]
    fn conditionally_defined_proc_body_call_sites_are_still_counted() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                if {[info exists ::env(X)]} {
                    proc runIt {} { helper b }
                }
                helper a
                runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "::helper is called with both \"a\" and \"b\" (the conditionally-defined \
                 runIt's body): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// FP guard: `namespace eval ::other { … }`
    /// runs its body in `::other`, never the enclosing proc's own
    /// namespace — tclsh8.6-confirmed live: a bare call inside such a
    /// block, nested arbitrarily deep inside an unrelated proc, still
    /// resolves against `::other`. Before threading the block's real
    /// target namespace into its body-unit qname
    /// (`register_body_unit`/`lower_namespace_eval`), every
    /// `namespace eval` body unit's qname reduced to the *global*
    /// namespace regardless of its actual target, so this call would
    /// have resolved (or misattributed) against global instead of
    /// `::other`.
    #[test]
    fn namespace_eval_body_nested_in_a_proc_resolves_against_its_own_namespace() {
        let reg = registry();
        let src = "
                namespace eval ::other {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                }
                proc runIt {} {
                    namespace eval ::other { helper b }
                }
                ::other::helper a
                runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::other::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::other::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "::other::helper is called with both \"a\" (direct) and \"b\" (via the \
                 namespace eval block nested inside runIt): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// Namespace-resolution regression: `apply {params body
    /// ns}`'s third element names the namespace the body runs in, so a
    /// bare command word inside it resolves against *that* namespace.
    /// `lower_apply` computed the right `body_ns` and lowered the body
    /// against it, then registered the body unit under the bare `apply`
    /// marker — whose qname puts it in the global namespace. The call
    /// site was attributed to a `::helper` that does not exist and
    /// vanished from `::foo::helper`'s evidence, folding a branch real
    /// Tcl reaches both ways.
    ///
    /// On tclsh8.6 and tclsh9.0: inside `::foo::runIt`,
    /// `apply {{x} { helper $x } ::foo} b`
    /// returns 2 — the `else` arm of `::foo::helper` — while the direct
    /// `::foo::helper a` returns 1.
    #[test]
    fn apply_with_a_namespace_element_resolves_the_body_against_it() {
        let reg = registry();
        let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        apply {{x} { helper $x } ::foo} b
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "::foo::helper is called with both \"a\" (direct) and \"b\" (through the \
                 ::foo-pinned lambda): {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// The two-element form is unchanged: with no namespace element,
    /// `apply` runs the body in the *global* namespace, not the caller's
    /// (Tcl `apply` manual). So the enclosing namespace's `helper` is
    /// genuinely never called from the lambda and keeps its fold.
    #[test]
    fn apply_without_a_namespace_element_still_resolves_globally() {
        let reg = registry();
        let src = "
                proc ::helper {x} {return $x}
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        apply {{x} { helper $x }} b
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::foo::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::foo::helper analysed");
        assert!(
            knows_condition_mentioning(helper, "mode"),
            "an unpinned lambda body resolves globally, so ::foo::helper only ever \
                 sees the direct \"a\": {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// Test diagnostic branch knowledge without donating caller contents
    /// to an executable replacement of retained native object coercions.
    /// Build `src` under a **closed world**: an empty cross-file evidence
    /// set, which is the host asserting "I enumerated the project and no
    /// other file calls into this one" — the semantics for a file that is
    /// genuinely the whole program.
    fn build_closed_world(src: &str, reg: &CommandRegistry) -> CompilationUnit {
        let empty = crate::unit_scope::CallSiteEvidence::default();
        CompilationUnit::build_with_options(
            src,
            UnitBuildOptions {
                registry: reg,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::default(),
                dialect: None,
                external_call_sites: Some(&empty),
                declared_commands: None,
            },
        )
    }

    fn knows_condition_mentioning(fu: &FunctionUnit, needle: &str) -> bool {
        fu.diagnostic_value_facts()
            .constant_branches()
            .iter()
            .any(|branch| branch.condition.contains(needle))
    }

    #[test]
    fn caller_contents_do_not_license_erasing_retained_parameter_coercions() {
        let reg = registry();
        let source =
            "proc helper {n} {if {$n + 0 == 7} {set result yes} else {set result no}}; helper 007";
        let unit = CompilationUnit::build_for(source, &reg, false);
        let function = fixture_id_if_present(&unit.procedures, "::helper")
            .and_then(|id| unit.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            knows_condition_mentioning(function, "n"),
            "caller_scope={:?} names={:?} preparations={} branch_bindings={} semantic_expressions={}",
            unit.caller_scope,
            function.ssa.var_names(),
            function.cfg.expression_preparations.len(),
            function.cfg.condition_binding_sites.len(),
            function
                .semantic_values()
                .map_or(0, |facts| facts.expression_iter().count())
        );
        assert_eq!(
            function.sccp.constant_branches,
            [] as [crate::sccp::ConstantBranch; 0]
        );
        let symbol = function.ssa.var_symbol("n").expect("retained formal slot");
        assert_eq!(
            function.semantic_values().unwrap().contents((symbol, 0)),
            Some(&crate::analyses::LatticeValue::Const(
                crate::analyses::ConstValue::String("007".to_owned())
            ))
        );
    }

    /// FN: a proc declared inside a `namespace eval` block recurses into
    /// itself by its *bare* name. A resolver that only tries
    /// global-qualified spellings of the command word can never match the
    /// proc's namespaced qualified name — the recursive call (whose
    /// argument necessarily varies call to call) vanishes from the
    /// call-site scan, leaving only the one external caller's literal `0`
    /// visible. `params_constants_from_call_sites` would then seed `count`
    /// as the compile-time constant `0`, folding the always-alternating
    /// `$count & 1` parity check to a fixed `false` — exactly the
    /// reported false positive.
    #[test]
    fn namespaced_recursive_proc_parity_check_is_not_constant_folded() {
        let reg = registry();
        let src = "
                namespace eval ::graph {
                    proc dfs {count} {
                        if {$count & 1} {
                            set parity odd
                        } else {
                            set parity even
                        }
                        if {$count < 3} {
                            dfs [expr {$count + 1}]
                        }
                    }
                }
                ::graph::dfs 0
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let dfs = fixture_id_if_present(&cu.procedures, "::graph::dfs")
            .and_then(|id| cu.procedures.get(&id))
            .expect("dfs analysed");
        assert!(
            !knows_condition_mentioning(dfs, "count"),
            "recursive parity check on `count` must not fold to a constant: {:?}",
            dfs.sccp.constant_branches,
        );
    }

    /// TN control: the same shape at the *top level* (no namespace),
    /// where a bare recursive call resolves to the right un-namespaced
    /// qualified name on its own, must stay sound.
    #[test]
    fn top_level_recursive_proc_parity_check_is_not_constant_folded() {
        let reg = registry();
        let src = "
                proc count_up {n} {
                    if {$n & 1} { set p odd } else { set p even }
                    if {$n < 3} { count_up [expr {$n + 1}] }
                }
                count_up 0
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::count_up")
            .and_then(|id| cu.procedures.get(&id))
            .expect("count_up analysed");
        assert!(
            !knows_condition_mentioning(f, "n"),
            "recursive parity check on `n` must not fold to a constant: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// TP control: the interprocedural seed must still fire for a
    /// genuinely proc-call-invariant parameter — two callers (one via a
    /// same-namespace bare call, proving the namespace-aware resolver
    /// still positively resolves, not just negatively declines) passing
    /// the identical literal.
    #[test]
    fn two_same_namespace_callers_with_uniform_literal_still_folds() {
        let reg = registry();
        let src = "
                namespace eval ::a {
                    proc go {mode} {
                        if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                    }
                    proc caller1 {} { go prod }
                    proc caller2 {} { go prod }
                }
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let go = fixture_id_if_present(&cu.procedures, "::a::go")
            .and_then(|id| cu.procedures.get(&id))
            .expect("go analysed");
        assert!(
            knows_condition_mentioning(go, "mode"),
            "two same-namespace callers passing the identical literal should still fold: caller_scope={:?}, contents={:?}, preparations={}, condition_bindings={}",
            cu.caller_scope,
            go.semantic_values()
                .map(|facts| facts.contents_iter().collect::<Vec<_>>()),
            go.cfg.expression_preparations.len(),
            go.cfg.condition_binding_sites.len(),
        );
        assert!(
            go.sccp.constant_branches.is_empty(),
            "logical incoming contents do not prove erasure of native operand conversion",
        );
    }

    /// FP guard: two same-leaf-name procs in different namespaces must
    /// never be conflated by a bare same-namespace call — `::a::go`'s
    /// only caller passes `"same"`; `::b::go`'s only caller passes
    /// `"different"`. Each must fold to *its own* literal, not the
    /// other's — a namespace-blind resolver matches neither bare call to a
    /// real proc.
    #[test]
    fn sibling_namespace_procs_with_same_leaf_name_are_not_conflated() {
        let reg = registry();
        let src = "
                namespace eval ::a {
                    proc go {mode} {
                        if {$mode eq \"same\"} { set r 1 } else { set r 2 }
                    }
                    proc caller {} { go same }
                }
                namespace eval ::b {
                    proc go {mode} {
                        if {$mode eq \"same\"} { set r 1 } else { set r 2 }
                    }
                }
                ::b::go different
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let a_go = fixture_id_if_present(&cu.procedures, "::a::go")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::a::go analysed");
        let b_go = fixture_id_if_present(&cu.procedures, "::b::go")
            .and_then(|id| cu.procedures.get(&id))
            .expect("::b::go analysed");
        assert!(
            knows_condition_mentioning(a_go, "mode"),
            "::a::go's sole caller passes a uniform literal, should fold: {:?}",
            a_go.sccp.constant_branches,
        );
        assert!(
            b_go.sccp
                .constant_branches
                .iter()
                .any(|b| b.condition.contains("mode") && !b.value),
            "::b::go's sole caller passes \"different\", condition should fold false: {:?}",
            b_go.sccp.constant_branches,
        );
    }

    /// FP guard: a `rename` that could redirect a *different* command
    /// onto the callee's own name must disqualify the seed, even though
    /// every call site this scan can see (all made before the rename,
    /// textually) passes a uniform literal — `trusts_proc_binding` is
    /// flow-insensitive/whole-module by design (matching the identical
    /// gate already trusted for the optimiser's O103 proc-call fold), so
    /// it can't assume the calls happen before the rename takes effect.
    #[test]
    fn rename_onto_callee_name_disqualifies_call_site_seed() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                proc other {mode} {
                    if {$mode eq \"prod\"} { set y 3 } else { set y 4 }
                }
                helper prod
                helper prod
                rename other helper
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a callee whose name is later rebound must not fold on stale call-site evidence: {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// TN control and documented scope boundary: a dynamic (non-literal)
    /// call-site head elsewhere in the module is simply unresolvable —
    /// it contributes no evidence for or against any proc's params,
    /// exactly like any other call this scan can't attribute to a
    /// specific callee. `helper`'s own two uniform-literal callers still
    /// fold. Closing the residual "the dynamic call might secretly
    /// target `helper` too" gap would need a value-set fact for `$cmd`
    /// this pass runs too early in the pipeline to have (it produces
    /// the very SCCP seed such a fact would depend on) — a pre-existing
    /// limitation of this call-site scan, not a regression.
    #[test]
    fn dynamic_dispatch_elsewhere_does_not_disqualify_an_unrelated_seed() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
                set cmd helper
                $cmd prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            knows_condition_mentioning(helper, "mode"),
            "an unrelated dynamic call site must not disqualify helper's own uniform-literal seed: {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// FN regression: a call site embedded inside a `catch { … }` body is
    /// a real caller with a differing argument, but `catch`'s body is an
    /// `ArgRole::Body` argument of the *builtin* `catch` — never a user
    /// proc — so a flat, one-level call-site walk resolves `catch`,
    /// finds no matching proc, and moves on without ever noticing the
    /// `isEven 4` sitting inside it. Before recursing into `ArgRole::Body`
    /// arguments, this scan only ever saw the one external `isEven 3`
    /// call, so it wrongly seeded `n` as the constant `3`.
    #[test]
    fn call_site_inside_catch_body_is_not_missed() {
        let reg = registry();
        let src = "
                proc is_even {n} {
                    if {$n % 2 == 0} { return 1 } else { return 0 }
                }
                proc main {} {
                    is_even 3
                    catch { is_even 4 }
                }
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::is_even")
            .and_then(|id| cu.procedures.get(&id))
            .expect("is_even analysed");
        assert!(
            !knows_condition_mentioning(f, "n"),
            "is_even is called with both 3 and 4 (the latter inside `catch`); must not fold: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// FN regression, `uplevel` flavour: a literal `uplevel {…}` body is
    /// an `ArgRole::Body` argument of the builtin `uplevel`, exactly like
    /// `catch`'s.
    #[test]
    fn call_site_inside_uplevel_body_is_not_missed() {
        let reg = registry();
        let src = "
                proc is_even {n} {
                    if {$n % 2 == 0} { return 1 } else { return 0 }
                }
                proc main {} {
                    is_even 3
                    uplevel 1 { is_even 4 }
                }
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let f = fixture_id_if_present(&cu.procedures, "::is_even")
            .and_then(|id| cu.procedures.get(&id))
            .expect("is_even analysed");
        assert!(
            !knows_condition_mentioning(f, "n"),
            "is_even is called with both 3 and 4 (the latter inside `uplevel`); must not fold: {:?}",
            f.sccp.constant_branches,
        );
    }

    /// TN control: an unrelated call inside a `catch` body must not
    /// disqualify a *different*, genuinely call-site-invariant proc's
    /// seed — the recursive Body-arg scan must attribute evidence to the
    /// right callee, not blanket-disqualify everything the way the
    /// (reverted) dynamic-dispatch wildcard did.
    #[test]
    fn unrelated_catch_body_does_not_disqualify_a_different_proc() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                proc noisy {} {
                    catch { error expected_failure }
                }
                helper prod
                helper prod
                noisy
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            knows_condition_mentioning(helper, "mode"),
            "an unrelated catch body must not disqualify helper's own uniform-literal seed: {:?}",
            helper.sccp.constant_branches,
        );
    }

    #[test]
    fn caught_unresolved_command_keeps_its_callback_mutation_boundary() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                proc noisy {} {catch {unresolved_command}}
                helper prod
                noisy
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(!knows_condition_mentioning(helper, "mode"));
        assert_eq!(
            helper.sccp.constant_branches,
            [] as [crate::sccp::ConstantBranch; 0]
        );
    }

    /// FP guard: a `package provide` file may export procs another file
    /// calls with a different literal — this (single-file) compilation
    /// unit can never see that caller, so it must never seed from the
    /// call sites it happens to see locally.
    #[test]
    fn package_provide_file_disqualifies_every_seed() {
        let reg = registry();
        let src = "
                package provide mylib 1.0
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a package-providing file must not seed from locally-visible call sites: {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// TN control: a plain script with no `package provide` keeps
    /// folding — the guard above must be scoped to the specific
    /// evidence (`package provide` presence), not silently disable the
    /// whole mechanism.
    #[test]
    fn non_package_file_is_unaffected_by_package_provide_guard() {
        let reg = registry();
        let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            knows_condition_mentioning(helper, "mode"),
            "no package-provide in this file, seed should still fold: {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// FP guard: `package provide` merely *mentioned* in a comment must
    /// not disable the interprocedural seed — the guard checks the lowered
    /// IR for a real, resolved invocation, not a raw-text substring match
    /// over the whole file.
    #[test]
    fn package_provide_mentioned_only_in_a_comment_does_not_disqualify() {
        let reg = registry();
        let src = "
                # this file does not package provide anything itself
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            knows_condition_mentioning(helper, "mode"),
            "a comment merely mentioning the phrase must not disqualify: {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// FN guard: a real `package provide` invocation must still
    /// disqualify the seed even when it is spelled with unusual
    /// whitespace or fully namespace-qualified — cases a
    /// `source.contains("package provide")` substring check would miss.
    #[test]
    fn package_provide_with_unusual_spelling_still_disqualifies() {
        let reg = registry();
        let src = "
                ::package\tprovide mylib 1.0
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let helper = fixture_id_if_present(&cu.procedures, "::helper")
            .and_then(|id| cu.procedures.get(&id))
            .expect("helper analysed");
        assert!(
            !knows_condition_mentioning(helper, "mode"),
            "a real (if oddly-spelled) package provide must still disqualify: {:?}",
            helper.sccp.constant_branches,
        );
    }

    /// Two-file repro: `lib.tcl` has **no** `package provide`, so the
    /// provide guard never fires; its only two visible callers both pass
    /// `"prod"`, so seeding `mode` as that constant folds the condition —
    /// even though `main.tcl` `source`s it and calls `helper dev`.
    mod cross_file {
        use super::*;
        use crate::unit_scope::CallSiteEvidence;

        /// A plain library file, no `package provide`, whose in-file
        /// callers agree.
        const LIB: &str = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                }
                helper prod
                helper prod
            ";

        /// Sources the library and calls its proc with a literal
        /// `lib.tcl` never sees.
        const MAIN: &str = "
                source lib.tcl
                helper dev
            ";

        /// Cross-file evidence a host builds from `other`, resolved
        /// against the procedures `LIB` declares.
        fn library_entry() -> crate::command_binding::SourceAnalysisEntry {
            let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
            crate::command_binding::SourceAnalysisEntry {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                trusted_source_modules: vec![
                    crate::command_binding::TrustedSourceModuleLoader::new(
                        tcl_dialect::model::Family::Tcl,
                        "lib.tcl".into(),
                        None,
                        "workspace/library/revision-one".into(),
                        &Arc::from(LIB),
                    ),
                ],
                ..crate::command_binding::SourceAnalysisEntry::default()
            }
        }

        fn evidence_from(other: &str, reg: &CommandRegistry) -> CallSiteEvidence {
            let known: HashSet<String> = ["::helper".to_owned()].into_iter().collect();
            let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
            let entry = library_entry();
            // The driver actually loads the library before this caller file.
            // MAIN already performs that read at its authored source site.
            let loaded;
            let source = if other == MAIN || other.starts_with("source lib.tcl") {
                other
            } else {
                loaded = format!("source lib.tcl\n{other}");
                &loaded
            };
            let context = Arc::new(
                crate::environment_ingress::context_for_profile(profile)
                    .with_command_store(reg.snapshot().shared_registry()),
            );
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            );
            crate::unit_scope::scan_source_call_sites_with_source_input(
                source,
                None,
                &known,
                &[],
                &input,
                &entry,
            )
        }

        fn build_with_evidence(
            src: &str,
            reg: &CommandRegistry,
            evidence: &CallSiteEvidence,
        ) -> CompilationUnit {
            build_with_source_evidence(src, reg, Some(evidence))
        }

        fn build_with_source_evidence(
            src: &str,
            reg: &CommandRegistry,
            evidence: Option<&CallSiteEvidence>,
        ) -> CompilationUnit {
            let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
            let context = Arc::new(
                crate::environment_ingress::context_for_profile(profile)
                    .with_command_store(reg.snapshot().shared_registry()),
            );
            CompilationUnit::build_with_context_registry(
                src,
                UnitBuildOptions {
                    registry: reg,
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                    dialect: Some(profile),
                    external_call_sites: evidence,
                    declared_commands: None,
                },
                Some(&library_entry()),
                context,
            )
        }

        /// FP guard — the reported bug. With `main.tcl`'s `helper dev`
        /// merged in, `mode` has two distinct literals and must not fold.
        #[test]
        fn sourcing_file_with_a_differing_literal_retracts_the_fold() {
            let reg = registry();
            let evidence = evidence_from(MAIN, &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "main.tcl calls helper with \"dev\"; lib.tcl must not fold on \"prod\": {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// The evidence itself, before any lattice: the cross-file scan
        /// must attribute `helper dev` to `::helper` at position 0.
        #[test]
        fn cross_file_scan_attributes_the_call_to_the_library_proc() {
            let reg = registry();
            let evidence = evidence_from(MAIN, &reg);
            let helper = evidence.get("::helper").expect("helper call recorded");
            assert_eq!(helper.uniform_literal_at(0), Some("dev"));
        }

        /// TP control — the mechanism must still fold when the other file
        /// agrees. `main.tcl` passing `prod` too leaves one literal
        /// across the whole project, so the seed is sound and fires.
        #[test]
        fn sourcing_file_agreeing_on_the_literal_still_folds() {
            let reg = registry();
            let evidence = evidence_from("source lib.tcl\nhelper prod\n", &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                knows_condition_mentioning(helper, "mode"),
                "every caller in the project passes \"prod\"; the seed is sound: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// TN control — a workspace file that never mentions `helper`
        /// contributes no evidence and must not disturb the fold.
        #[test]
        fn unrelated_workspace_file_contributes_nothing() {
            let reg = registry();
            let known = HashSet::from(["::helper".to_owned()]);
            let evidence = crate::unit_scope::scan_source_call_sites(
                "proc other {x} { return $x }\nother 1\n",
                &reg,
                None,
                tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile"),
                &known,
                &[],
            );
            assert!(
                evidence.get("::helper").is_none(),
                "an unrelated file supplies no caller of ::helper: {evidence:?}",
            );
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                knows_condition_mentioning(helper, "mode"),
                "an unrelated file must not retract a sound fold: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FN guard — a cross-file caller reached through a *dynamic*
        /// argument is still a caller. `helper $mode` in `main.tcl`
        /// poisons position 0, so the fold is retracted even though no
        /// second literal was ever written.
        #[test]
        fn cross_file_dynamic_argument_poisons_the_position() {
            let reg = registry();
            let evidence = evidence_from("set m dev\nhelper $m\n", &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "a cross-file caller with a non-literal argument must retract the fold: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FN guard — a deferred script reads an unknown value when the
        /// callback runs. Its possible caller cannot inherit the immediate
        /// file's uniform argument. The body role and future frame come
        /// from the selected native descriptor, independently of spelling.
        #[test]
        fn cross_file_command_prefix_callback_poisons_every_position() {
            let reg = registry();
            let evidence = evidence_from("after 0 {helper $::runtime_mode}\n", &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "a deferred cross-file callback reads an unknown argument: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FN guard — a cross-file `rename` moves `helper`'s binding, so
        /// a call reaching it need not be one any scan attributed to it.
        /// `command_mutations` only sees the file being compiled, so
        /// without this the rebinding is invisible across files.
        #[test]
        fn cross_file_rename_poisons_the_callee() {
            let reg = registry();
            let evidence = evidence_from("rename helper legacy_helper\n", &reg);
            let cu = build_with_evidence(LIB, &reg, &evidence);
            let helper = fixture_id_if_present(&cu.procedures, "::helper")
                .and_then(|id| cu.procedures.get(&id))
                .expect("helper analysed");
            assert!(
                !knows_condition_mentioning(helper, "mode"),
                "another file renamed helper; its binding has moved: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// A `source`ing file is itself a boundary: `lib.tcl` on its own
        /// (no workspace view) has no boundary at all and still folds —
        /// the documented, accepted limit of a standalone single-file
        /// build — but the *sourcing* file's own procs do not, because
        /// `source` declares `LOADS_EXTERNAL_UNIT`.
        #[test]
        fn a_file_that_sources_another_declines_the_seed_on_its_own() {
            let reg = registry();
            let src = "
                    source lib.tcl
                    proc local {mode} {
                        if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                    }
                    local prod
                    local prod
                ";
            let cu = build_with_source_evidence(src, &reg, None);
            let f = fixture_id_if_present(&cu.procedures, "::local")
                .and_then(|id| cu.procedures.get(&id))
                .expect("local analysed");
            assert!(
                !knows_condition_mentioning(f, "mode"),
                "the sourced file's script can call ::local with anything: {:?}",
                f.sccp.constant_branches,
            );
            // …and with a workspace view that says otherwise, it folds.
            let empty = CallSiteEvidence::default();
            let closed = build_with_evidence(src, &reg, &empty);
            let f = fixture_id_if_present(&closed.procedures, "::local")
                .and_then(|id| closed.procedures.get(&id))
                .expect("local analysed");
            assert!(
                knows_condition_mentioning(f, "mode"),
                "with the project enumerated, the boundary is no longer a blind spot: {:?}",
                f.sccp.constant_branches,
            );
        }
    }

    /// FN regression pinned at the `LatticeValue` level (not just
    /// "no constant branch"): the seed for a genuinely-recursive
    /// parameter must never even reach `Const` for the parameter's
    /// version-0 lattice entry, confirming the fix operates at the SCCP
    /// seed itself (the deepest layer), not merely suppressing the
    /// downstream diagnostic.
    #[test]
    fn recursive_param_version_zero_is_never_const_in_lattice() {
        let reg = registry();
        let src = "
                namespace eval ::graph {
                    proc dfs {count} {
                        if {$count & 1} { set p odd } else { set p even }
                        if {$count < 3} { dfs [expr {$count + 1}] }
                    }
                }
                ::graph::dfs 0
            ";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let dfs = fixture_id_if_present(&cu.procedures, "::graph::dfs")
            .and_then(|id| cu.procedures.get(&id))
            .expect("dfs analysed");
        // Incoming activation alternatives do not manufacture one scalar
        // SSA symbol. If a physical template slot exists, its entry value
        // must still refuse a recursive caller's constant seed.
        let v0 = dfs
            .ssa
            .var_symbol("count")
            .and_then(|sym| dfs.sccp.values.get(&(sym, 0)));
        assert_eq!(
            dfs.sccp.constant_branches,
            [] as [crate::sccp::ConstantBranch; 0]
        );
        assert!(
            cu.caller_scope
                .call_sites
                .get("::graph::dfs")
                .is_some_and(|calls| calls
                    .slots
                    .get(&0)
                    .is_some_and(|slot| slot.unknown || slot.values.len() > 1))
        );
        assert!(
            !matches!(v0, Some(LatticeValue::Const(_))),
            "recursive param's version-0 lattice entry must not be Const, got {v0:?}",
        );
    }
}
