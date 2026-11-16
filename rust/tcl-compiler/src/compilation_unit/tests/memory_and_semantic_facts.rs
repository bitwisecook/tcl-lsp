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

//! Memory SSA and semantic sidecars: retained availability, source owners, memoised overlays and deep analysis.

use super::*;

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

fn test_context() -> SemanticContext {
    SemanticContext::for_environment("tcl8.6")
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
