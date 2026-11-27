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

use crate::interprocedural::test_body_identity::fixture_id;

#[test]
fn direct_formals_consume_native_activation_plan() {
    use tcl_dialect::ParameterGrammar;
    for grammar in [ParameterGrammar::Tcl, ParameterGrammar::Jim] {
        assert!(super::direct_formal_bindings("x y", 2, grammar).is_ok());
        assert_eq!(
            super::direct_formal_bindings("x x", 2, grammar),
            Err(super::DirectProcDecline::SharedFormalSlotUnsupported)
        );
        assert_eq!(
            super::direct_formal_bindings("x y", 1, grammar),
            Err(super::DirectProcDecline::ArityMismatch {
                expected: 2,
                actual: 1
            })
        );
        assert_eq!(
            super::direct_formal_bindings("x args", 2, grammar),
            Err(super::DirectProcDecline::VariadicUnsupported)
        );
    }
    assert!(super::direct_formal_bindings("args x", 2, ParameterGrammar::Tcl).is_ok());
    assert_eq!(
        super::direct_formal_bindings("args x", 2, ParameterGrammar::Jim),
        Err(super::DirectProcDecline::VariadicUnsupported)
    );
    assert_eq!(
        super::direct_formal_bindings("&x", 1, ParameterGrammar::Jim),
        Err(super::DirectProcDecline::ReferenceArgumentUnsupported)
    );
    assert!(super::direct_formal_bindings("&x", 1, ParameterGrammar::Tcl).is_ok());
}

use super::*;
use tcl_registry::{IntrinsicId, TclType};

fn plan(source: &str, config: SemanticOptimisationConfig) -> CommonAotProofPlan {
    plan_in_environment(source, config, CommonAotEnvironment::Hosted)
}

fn plan_in_environment(
    source: &str,
    config: SemanticOptimisationConfig,
    environment: CommonAotEnvironment,
) -> CommonAotProofPlan {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit(source, std::sync::Arc::clone(&context));
    CommonAotProofPlan::build_with_retained_metadata(&unit, context.commands(), config, environment)
}

fn native_unit(
    source: &str,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> crate::environment_ingress::RetainedNativeUnit {
    let result = native_unit_borrowed(source, &context);
    drop(context);
    result
}

fn native_unit_borrowed(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> crate::environment_ingress::RetainedNativeUnit {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let (owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_context_registry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(context),
    );
    crate::environment_ingress::RetainedNativeUnit::new(unit, owner)
}

fn logical_unit(
    source: &str,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> CompilationUnit {
    let result = logical_unit_borrowed(source, &context);
    drop(context);
    result
}

fn logical_unit_borrowed(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> CompilationUnit {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(context),
        config,
    );
    let entry = crate::command_binding::SourceAnalysisEntry::for_logical_source(&input)
        .expect("positive supplied Logical source input");
    CompilationUnit::build_with_analysis_input(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        &input,
    )
}

/// An explicit authored compiler/naming model, without an entered Native world.
fn authored_program_unit(
    source: &str,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> CompilationUnit {
    let result = authored_program_unit_borrowed(source, &context);
    drop(context);
    result
}

fn authored_program_unit_borrowed(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> CompilationUnit {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let entry = crate::command_binding::SourceAnalysisEntry {
        invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    assert_eq!(
        entry
            .options()
            .execution_name_policy()
            .unwrap()
            .native_recipe()
            .unwrap()
            .authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation,
    );
    CompilationUnit::build_with_context_registry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(context),
    )
}

#[test]
fn authored_program_storage_keeps_original_coverage_without_native_frame_or_body_grants() {
    // naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    // These positives describe original schemas and future storage templates.
    // Executable coverage retains its independent entry/provider requirement.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = authored_program_unit(
        "proc p {x y} {}; set d 2; set e 4; puts [p $d $e]",
        std::sync::Arc::clone(&context),
    );
    let module = &unit.ir_module;
    assert!(module.source_entry.native_entry.is_none());
    let source = OriginalAuthoredProgramSourcePlan::build(module, context.commands())
        .expect("current explicitly authored source owner");
    assert!(source.is_complete(), "source schemas: {source:#?}");
    assert_eq!(source.statements().len(), 4);
    let templates: Vec<_> = source.storage_templates().collect();
    assert_eq!(templates.len(), 2);
    for (name, value) in [(b"d".as_slice(), "2"), (b"e".as_slice(), "4")] {
        let template = templates
            .iter()
            .find(|template| template.name().as_bytes() == name)
            .expect("exact original scalar destination");
        assert_eq!(template.literal(), value);
        assert_eq!(
            template.contents_type().single_shape(),
            Some(&TypeShape::Int)
        );
    }
    let OriginalAuthoredSourceStatementKind::ProcedureDeclaration {
        procedure,
        arguments,
    } = source.statements()[0].kind()
    else {
        panic!("exact original declaration")
    };
    assert_eq!(procedure.qualified_name, "::p");
    assert_eq!(arguments.arguments().ordinal(b"x"), Some(0));
    assert_eq!(arguments.arguments().ordinal(b"y"), Some(1));
    assert!(matches!(source.statements()[3].kind(),
            OriginalAuthoredSourceStatementKind::Invocation { procedures }
                if procedures.as_slice() == [procedure.clone()]));
    assert!(
        crate::native_compilation_admission::script_requires_admission(
            &module
                .procedures
                .get(&fixture_id(&module.procedures, "::p"))
                .expect("authored fixture body")
                .body
        )
    );
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::SealedProgram,
    );
    assert_eq!(
        plan.closed_program_coverage(),
        &ClosedProgramCoverageDecision::Declined(ClosedProgramCoverageDecline::UncoveredStatement)
    );
    assert!(
        plan.direct_calls().any(|(_, decision)| matches!(
            decision,
            DirectProcDecision::Declined(DirectProcDecline::NativeCompilationAdmissionRequired)
        )),
        "the source template cannot admit this call: {plan:#?}"
    );
    assert!(
        plan.materialisable_slots()
            .all(|(_, decision)| !matches!(decision, MaterialisableSlotDecision::Selected(_))),
        "source templates allocate no program or borrowed Native cells: {plan:#?}"
    );
}

#[test]
fn authored_source_templates_withdraw_changed_module_and_point_owners() {
    // naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = authored_program_unit("set value 2", std::sync::Arc::clone(&context));
    let module = &unit.ir_module;
    let baseline = OriginalAuthoredProgramSourcePlan::build(module, context.commands()).unwrap();
    assert!(baseline.is_complete(), "{baseline:#?}");
    assert_eq!(baseline.storage_templates().count(), 1);
    let mut missing = module.clone();
    missing.source_metadata_input = None;
    let mut changed_source = module.clone();
    changed_source.source = tcl_lexer::SourceImage::document("set value OTHER");
    let mut changed_config = module.clone();
    changed_config.lexer_config.escapes = tcl_dialect::EscapeSyntax::Tcl84;
    let mut changed_entry = module.clone();
    changed_entry.source_entry.unknown_entry = true;
    let mut changed_namespace = module.clone();
    changed_namespace.top_level_namespace = "::different".into();
    for changed in [
        missing,
        changed_source,
        changed_config,
        changed_entry,
        changed_namespace,
    ] {
        assert_eq!(
            OriginalAuthoredProgramSourcePlan::build(&changed, context.commands()),
            Err(OriginalAuthoredProgramSourceDecline::SourceOwnerUnavailable)
        );
    }
    let mut omitted = module.clone();
    omitted.top_level.statements.clear();
    assert_eq!(
        OriginalAuthoredProgramSourcePlan::build(&omitted, context.commands()),
        Err(OriginalAuthoredProgramSourceDecline::RootSourceUnavailable)
    );
    let foreign = tcl_registry::CommandRegistry::build_default();
    assert_eq!(
        OriginalAuthoredProgramSourcePlan::build(module, &foreign),
        Err(OriginalAuthoredProgramSourceDecline::SourceOwnerUnavailable)
    );
    let mut missing_point = module.clone();
    missing_point.top_level.command_binding_sites = crate::ir::CommandBindingSites::default();
    for statement in &mut missing_point.top_level.statements {
        if let Statement::AssignValue { tokens, .. } | Statement::Call { tokens, .. } = statement {
            *tokens = None;
        }
    }
    let source =
        OriginalAuthoredProgramSourcePlan::build(&missing_point, context.commands()).unwrap();
    assert_eq!(source.residuals().len(), 1);
    assert_eq!(
        source.residuals()[0].1,
        OriginalAuthoredSourceResidual::StatementCarrierUnavailable
    );
    assert_eq!(source.storage_templates().count(), 0);
}

#[test]
fn authored_source_inventory_keeps_availability_replacement_and_native_entry_separate() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = authored_program_unit("throw ERROR payload", std::sync::Arc::clone(&current));
    let source =
        OriginalAuthoredProgramSourcePlan::build(&unit.ir_module, current.commands()).unwrap();
    assert!(source.is_complete(), "{source:#?}");
    let older =
        tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
    let unit = authored_program_unit("throw ERROR payload", std::sync::Arc::clone(&older));
    let source =
        OriginalAuthoredProgramSourcePlan::build(&unit.ir_module, older.commands()).unwrap();
    assert!(!source.is_complete());
    assert_eq!(
        source.residuals()[0].1,
        OriginalAuthoredSourceResidual::StatementSchemaUnavailable
    );
    for original in [
        "proc set args {}; set value 2",
        "rename set {}; set value 2",
        "set a(k) 2",
        "set ::qualified 2",
        "set value $outside",
    ] {
        let unit = authored_program_unit(original, std::sync::Arc::clone(&current));
        let source =
            OriginalAuthoredProgramSourcePlan::build(&unit.ir_module, current.commands()).unwrap();
        assert!(!source.is_complete(), "{original}: {source:#?}");
        assert_eq!(source.storage_templates().count(), 0, "{original}");
    }
    let entered = native_unit("proc p {x} {}; p VALUE", std::sync::Arc::clone(&current));
    assert_eq!(
        OriginalAuthoredProgramSourcePlan::build(&entered.ir_module, current.commands()),
        Err(OriginalAuthoredProgramSourceDecline::AuthoredPurposeUnavailable)
    );
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &entered,
        current.commands(),
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    assert!(
        plan.direct_calls().any(|(_, decision)| matches!(decision,
            DirectProcDecision::Selected(direct) if direct.callee.qualified_name == "::p")),
        "genuine Native empty-body call remains independent: {plan:#?}"
    );
}

#[test]
fn original_declared_argument_slots_do_not_borrow_the_parent_native_frame() {
    // Implementation contract: naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    use crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots;
    use tcl_syntax::naming::NativeCompiledVariableEnvironment;
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit("proc p {x y} {return $x}; p FIRST SECOND", context);
    let module = &unit.ir_module;
    let procedure = module
        .procedures
        .get(&fixture_id(&module.procedures, "::p"))
        .expect("authored fixture body");
    let entry = module.source_entry.native_entry.as_deref().unwrap();
    assert!(entry.compiled_local_layout.is_none());
    let receipt = OriginalDeclaredProcedureArgumentSlots::from_module(module, procedure)
        .expect("original declaration allocates its own argument convention");
    assert_eq!(receipt.arguments().ordinal(b"x"), Some(0));
    assert_eq!(receipt.arguments().ordinal(b"y"), Some(1));
    assert_eq!(receipt.declaration_span(), procedure.span);
    assert!(
        receipt
            .protocol()
            .supports_environment(NativeCompiledVariableEnvironment::DeclareProcedure)
    );
    assert_eq!(
        receipt.arguments().original_input(),
        receipt.topology().original_input()
    );
    let mut entered = module.clone();
    entered.source_entry.compilation_scope = tcl_runtime_api::SourceCompilationScope::EnteredSource;
    assert!(OriginalDeclaredProcedureArgumentSlots::from_module(&entered, procedure).is_none());
    let mut unavailable = module.clone();
    std::sync::Arc::make_mut(unavailable.source_entry.native_entry.as_mut().unwrap())
        .compiled_variable_protocol = None;
    assert!(OriginalDeclaredProcedureArgumentSlots::from_module(&unavailable, procedure).is_none());
    let mut missing = module.clone();
    missing.procedure_implementation_bodies = std::sync::Arc::from([]);
    assert!(OriginalDeclaredProcedureArgumentSlots::from_module(&missing, procedure).is_none());
    let mut stale = module.clone();
    stale.source = tcl_lexer::SourceImage::document("proc p {x y} {return OTHER}");
    assert!(OriginalDeclaredProcedureArgumentSlots::from_module(&stale, procedure).is_none());
    let mut changed = procedure.clone();
    changed.body.executed_source = None;
    assert!(OriginalDeclaredProcedureArgumentSlots::from_module(module, &changed).is_none());
}

#[test]
fn original_procedure_body_preflight_keeps_entry_and_missing_provider_independent() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Compiler traversal only: no body execution, numeric result, old frame
    // slot or direct-body specialisation is certified by this control.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    for (formals, body, arguments) in [
        ("x", "return $x", "VALUE"),
        ("x y", "return [expr {$x+$y}]", "1 2"),
        ("", "return [expr {1+2}]", ""),
        ("x", "set y $x; return $y", "VALUE"),
    ] {
        let source = format!("proc p {{{formals}}} {{{body}}}; p {arguments}");
        let unit = native_unit(&source, std::sync::Arc::clone(&context));
        let procedure = unit
            .ir_module
            .procedures
            .get(&fixture_id(&unit.ir_module.procedures, "::p"))
            .expect("authored fixture body");
        let original = procedure.body.executed_source.as_ref().unwrap();
        assert_eq!(original.text.bytes(), body.as_bytes());
        assert!(
            !crate::native_compilation_admission::script_requires_admission(&procedure.body),
            "{body}: {:?}",
            procedure.body.native_compilation_admission,
        );
        let unavailable = crate::command_binding::SourceCommandBindings::analyse_with_options(
            body,
            config,
            context.commands(),
            crate::command_binding::SourceAnalysisOptions {
                unknown_entry: true,
                invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        assert!(unavailable.native_compilation_source_at(0).is_some());
        assert!(
            unavailable.native_compilation_provider_required_at(0),
            "{body}"
        );
    }
}

#[test]
fn original_empty_procedure_direct_call_keeps_formals_without_body_specialisation() {
    // Implementation contract: naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit("proc p {x} {}; p VALUE", std::sync::Arc::clone(&context));
    let procedure = unit
        .ir_module
        .procedures
        .get(&fixture_id(&unit.ir_module.procedures, "::p"))
        .expect("authored fixture body");
    assert_eq!(procedure.body.statements, [] as [crate::ir::Statement; 0]);
    assert!(!crate::native_compilation_admission::script_requires_admission(&procedure.body));
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    let selected = plan
        .direct_calls()
        .find_map(|(_, decision)| match decision {
            DirectProcDecision::Selected(evidence) if evidence.callee.qualified_name == "::p" => {
                Some(evidence)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("original empty procedure direct call: {plan:#?}"));
    assert_eq!(selected.formals, ["x"]);
    assert_eq!(
        selected.original_formals.as_ref().unwrap().ordinal(b"x"),
        Some(0)
    );
    assert!(matches!(
        selected.body,
        DirectProcBodyDecision::Declined(DirectProcBodyDecline::UnsupportedBodyShape)
    ));
    assert!(!selected.frame_elidable);
}

#[test]
fn retained_aot_metadata_cannot_borrow_missing_or_foreign_function_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = tcl_registry::CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(
        "proc p {x} {return $x}; p VALUE",
        &registry,
        false,
        "tcl8.6",
    );
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        &registry,
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    assert_eq!(plan.direct_calls().count(), 1);
    assert!(plan.direct_calls().all(|(_, decision)| !matches!(
        decision,
        DirectProcDecision::Declined(DirectProcDecline::ContextUnavailable)
    )));
    let mut missing_module = unit.clone();
    missing_module.ir_module.source_metadata_input = None;
    let mut missing_caller = unit.clone();
    missing_caller.top_level.source_metadata_input = None;
    let mut missing_target = unit.clone();
    {
        let id = fixture_id(&missing_target.procedures, "::p");
        missing_target.procedures.get_mut(&id)
    }
    .unwrap()
    .source_metadata_input = None;
    let mut stale_grammar = unit.clone();
    stale_grammar.top_level.source_config.expand_syntax =
        !stale_grammar.top_level.source_config.expand_syntax;
    let input = unit.top_level.source_metadata_input().unwrap();
    let mut foreign = unit.clone();
    foreign.top_level.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    ));
    for refused in vec![
        missing_module,
        missing_caller,
        missing_target,
        stale_grammar,
        foreign,
    ]
    .into_boxed_slice()
    {
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &refused,
            &registry,
            enabled(),
            CommonAotEnvironment::Hosted,
        );
        assert_eq!(plan.direct_calls().count(), 1);
        assert!(plan.direct_calls().all(|(_, decision)| matches!(
            decision,
            DirectProcDecision::Declined(DirectProcDecline::ContextUnavailable)
        )));
    }
}

#[test]
fn original_source_queries_require_unanimous_statement_carriers() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = logical_unit("set result VALUE", std::sync::Arc::clone(&context));
    let (block, index) = unit
        .top_level
        .ssa
        .blocks
        .iter()
        .find_map(|(id, block)| {
            block
                .statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    matches!(statement.statement, Statement::AssignConst { .. })
                        .then_some((*id, index))
                })
        })
        .expect("original source store retains a structural statement");
    let id = DirectCallSiteId {
        module_id: unit.ir_module.procedure_ids.module_id(),
        body_id: None,
        function: "::top".into(),
        block,
        statement_index: u32::try_from(index).unwrap(),
        nested_argument: None,
    };
    let statement = &unit.top_level.ssa.blocks[&block].statements[index].statement;
    assert!(
        statement.tokens().is_none(),
        "typed store consumes its inline vector"
    );
    assert_eq!(
        direct_call_tokens(&unit, &id).unwrap().argv_texts,
        ["set", "result", "VALUE"],
    );
    let foreign = logical_unit("set result VALUE", context);
    assert_ne!(id.module_id, foreign.ir_module.procedure_ids.module_id());
    assert!(direct_call_tokens(&foreign, &id).is_none());
    assert!(direct_call_argument_read(&foreign, &id, 0).is_none());
    assert!(direct_call_argument_value(&foreign, &id, 0).is_none());
    let mut missing = unit.clone();
    missing.top_level.cfg.command_binding_sites.clear();
    assert!(direct_call_tokens(&missing, &id).is_none());
    let mut conflicting = unit.clone();
    let mut other = conflicting
        .top_level
        .cfg
        .command_binding_sites
        .iter()
        .find(|site| site.span == statement.span() && site.source_tokens.is_some())
        .unwrap()
        .clone();
    other.source_tokens.as_mut().unwrap().argv_texts[0] = "other".into();
    conflicting.top_level.cfg.command_binding_sites.push(other);
    assert!(direct_call_tokens(&conflicting, &id).is_none());
}

#[test]
fn structural_call_candidates_keep_original_nested_words_and_literal_boundaries() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = logical_unit(
        "proc p {x} {return $x}; proc q {x} {return [p $x]}; list {[p VALUE]}",
        context,
    );
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::q"))
        .expect("authored fixture body");
    let candidates = call_sites(&unit, "::q", function, function.source_lexer_config());
    let nested = candidates
        .iter()
        .find(|site| site.command == "p")
        .expect("structural return retains its original nested call");
    assert_eq!(nested.id.nested_argument, Some(0));
    assert_eq!(nested.args, ["$x"]);
    assert_eq!(nested.tokens.as_ref().unwrap().argv_texts, ["p", "$x"]);
    let top = call_sites(
        &unit,
        "::top",
        &unit.top_level,
        unit.top_level.source_lexer_config(),
    );
    assert!(
        top.iter().all(|site| site.id.nested_argument.is_none()),
        "procedure body words and braced list values are data at these invocations"
    );
    let mut missing = function.clone();
    missing.cfg.command_binding_sites.clear();
    for block in missing.cfg.blocks.values_mut() {
        for statement in &mut block.statements {
            if let Statement::Return { tokens, .. } = statement {
                *tokens = None;
            } else {
                assert!(statement.tokens().is_none());
            }
        }
    }
    assert!(
        call_sites(&unit, "::q", &missing, missing.source_lexer_config())
            .iter()
            .all(|site| site.command != "p")
    );
}

#[test]
fn original_direct_expression_body_requires_the_actual_nested_operation_receipts() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let registry = context.commands();
    let unit = logical_unit(
        "proc p {x y} {return [expr {$x+$y}]}",
        std::sync::Arc::clone(&context),
    );
    let function = unit
        .procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body");
    let procedure = unit
        .ir_module
        .procedures
        .get(&fixture_id(&unit.ir_module.procedures, "::p"))
        .expect("authored fixture body");
    let metadata = function.invocation_metadata_context_for_module(registry, &unit.ir_module);
    let operations = original_direct_expression_body_operations(
        registry,
        metadata,
        procedure,
        function.source_lexer_config(),
    );
    assert_eq!(
        operations,
        Some([
            SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
            SemanticOperationId::StructuredLowering(LoweringHookId::Return),
        ])
    );
    assert!(
        original_direct_expression_body_operations(
            registry,
            None,
            procedure,
            function.source_lexer_config(),
        )
        .is_none()
    );
    let mut missing_source = procedure.clone();
    missing_source.body.command_binding_sites = crate::ir::CommandBindingSites::default();
    for statement in &mut missing_source.body.statements {
        if let Statement::Return { tokens, .. } = statement {
            *tokens = None;
        }
    }
    assert!(
        original_direct_expression_body_operations(
            registry,
            metadata,
            &missing_source,
            function.source_lexer_config(),
        )
        .is_none()
    );
    for source in [
        "proc expr args {return 0}; proc p {x y} {return [expr {$x+$y}]}",
        "proc return args {}; proc p {x y} {return [expr {$x+$y}]}",
        "proc p {x y} {return [expr {$x+[unknown]}]}",
    ] {
        let changed = logical_unit(source, std::sync::Arc::clone(&context));
        let function = changed
            .procedures
            .get(&fixture_id(&changed.procedures, "::p"))
            .expect("authored fixture body");
        assert!(
            original_direct_expression_body_operations(
                registry,
                function.invocation_metadata_context_for_module(registry, &changed.ir_module),
                changed
                    .ir_module
                    .procedures
                    .get(&fixture_id(&changed.ir_module.procedures, "::p"))
                    .expect("authored fixture body"),
                function.source_lexer_config(),
            )
            .is_none(),
            "{source}"
        );
    }
}

fn enabled() -> SemanticOptimisationConfig {
    SemanticOptimisationConfig::new()
        .with_enabled(SemanticOptimisationPassId::DirectProc)
        .with_enabled(SemanticOptimisationPassId::MaterialisableSlot)
        .with_enabled(SemanticOptimisationPassId::FrameElision)
        .with_enabled(SemanticOptimisationPassId::NativeInteger)
        .with_enabled(SemanticOptimisationPassId::SemanticOperationSpecialisation)
}

const ADD: &str = "proc add {b c} { return [expr {$b+$c}] }\nset d 2\nset e 4\nputs [add $d $e]\n";

#[test]
fn direct_procedure_selection_preserves_native_preformal_admission() {
    use crate::native_compilation_admission::NativeCompilationAdmission;
    use std::sync::Arc;
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut unit =
        CompilationUnit::build_for_dialect("proc p {} {return 1}; p", &registry, false, "tcl9.0");
    {
        let id = fixture_id(&unit.ir_module.procedures, "::p");
        unit.ir_module.procedures.get_mut(&id)
    }
    .unwrap()
    .body
    .native_compilation_admission = Some(Arc::new(NativeCompilationAdmission {
        source: None,
        failure: None,
        provider_required: true,
    }));
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        &registry,
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    assert!(plan.direct_calls().any(|(_, decision)| matches!(
        decision,
        DirectProcDecision::Declined(DirectProcDecline::NativeCompilationAdmissionRequired)
    )));
}

#[test]
fn a_declined_caller_poisons_propagated_actual_types() {
    // `::p` is reached by a selected top-level call passing an integer and
    // by a declined call (exceptional control flow) passing a string.  The
    // propagated join covers the selected site alone, so without poisoning
    // it narrows the parameter to `Int` — while the caller-scope constant
    // for the very same slot is the declined site's `abc`.  A materialised
    // slot must not be selected from that contradiction.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit(
        "proc p {x} { return [expr {$x + 1}] }\nproc risky {} { try { p abc } on error {} {} }\nset d 7\nputs [p $d]\n",
        std::sync::Arc::clone(&context),
    );
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    assert!(plan.has_declined_direct_call_to(fixture_id(&unit.ir_module.procedures, "::p")));
    for (id, decision) in plan.materialisable_slots() {
        assert!(
            id.function != "::p" || matches!(decision, MaterialisableSlotDecision::Declined(_)),
            "an unselected caller must leave `::p`'s parameter unproved: {decision:?}"
        );
    }
}

#[test]
fn passes_are_off_by_default() {
    let plan = plan(ADD, SemanticOptimisationConfig::default());
    assert!(plan.direct_calls().any(|(_, decision)| matches!(
        decision,
        DirectProcDecision::Declined(DirectProcDecline::PassDisabled)
    )));
    assert!(plan.materialisable_slots().all(|(_, decision)| matches!(
        decision,
        MaterialisableSlotDecision::Declined(MaterialisableSlotDecline::PassDisabled)
    )));
}

fn original_formal_diagnostics(unit: &CompilationUnit, function: &str) -> String {
    let function = &unit
        .procedures
        .get(&fixture_id(&unit.procedures, function))
        .expect("authored fixture body");
    let mut rows = vec![format!("SSA cells: {:?}", function.ssa.cell_keys())];
    if let Some(points) = &function.ssa.point_contexts {
        for (&block, body) in &function.cfg.blocks {
            for index in (0..body.statements.len()).chain(std::iter::once(usize::MAX)) {
                if let Some(context) = points.context_before(block, index) {
                    rows.push(format!(
                        "{block:?}/{index}: activation={:?}, topology={}, observers={}, dynamic={}",
                        context.activation,
                        context.original_formal_topology.is_some(),
                        context.activation_observers_closed(),
                        context.dynamic_bindings,
                    ));
                }
                for access in points.source_reads_at(block, index) {
                    rows.push(format!(
                        "read {:?} at {:?}: residual={:?}, activations={:?}",
                        access.original_spelling,
                        access.source.span,
                        access.context_residual(),
                        access
                            .context_alternatives()
                            .iter()
                            .map(|context| (
                                &context.activation,
                                context.original_formal_topology.is_some(),
                                context.activation_observers_closed(),
                                context.dynamic_bindings,
                            ))
                            .collect::<Vec<_>>(),
                    ));
                }
            }
        }
    }
    rows.join("\n")
}

fn assert_declared_add_arguments(
    plan: &CommonAotProofPlan,
    unit: &CompilationUnit,
    direct: &DirectProcEvidence,
) {
    let selected: Vec<_> = plan
        .declared_arguments()
        .filter_map(|(identity, decision)| match decision {
            DeclaredArgumentDecision::Selected(evidence) if identity.procedure == direct.callee => {
                Some((identity, evidence))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        selected.len(),
        2,
        "{}\n{plan:#?}",
        original_formal_diagnostics(unit, "::add")
    );
    assert!(
        selected
            .iter()
            .all(|(_, slot)| slot.shape() == &TypeShape::Int)
    );
    assert!(selected.iter().all(|(_, slot)| {
        slot.storage() == VarStorage::MaterializableSlot
            && slot.recipe()
                == MaterialisationRecipe::RetainOriginalTclObject {
                    sharing: SharingState::Shared,
                }
    }));
    assert!(
        plan.materialisable_slots().all(|(identity, decision)| {
            identity.function != "::add"
                || !matches!(decision, MaterialisableSlotDecision::Selected(_))
        }),
        "declared arguments cannot fabricate a physical SSA value"
    );
    assert!(selected.iter().all(|(identity, slot)| {
        slot.reads().iter().all(|read| {
            plan.declared_argument_for_incoming_read(&direct.callee, read)
                .is_some_and(|(selected, _)| selected == *identity)
        })
    }));
}

#[test]
fn add_call_and_integer_formals_receive_common_proofs() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit(ADD, std::sync::Arc::clone(&context));
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    let direct = plan
        .direct_calls()
        .find_map(|(_, decision)| match decision {
            DirectProcDecision::Selected(evidence) if evidence.callee.qualified_name == "::add" => {
                Some(evidence)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("nested add call selected: {plan:#?}"));
    assert_eq!(direct.formals, ["b", "c"]);
    assert!(direct.frame_escape_private);
    assert!(direct.frame_elidable);
    assert!(matches!(direct.body, DirectProcBodyDecision::Selected(_)));
    assert!(
        direct
            .actual_types
            .iter()
            .all(|ty| ty.tcl_type() == Some(TclType::Int))
    );

    assert_declared_add_arguments(&plan, &unit, direct);

    assert_eq!(direct.actual_values.len(), 2);
    assert!(direct.actual_values.iter().all(|actual| matches!(
        actual,
        DirectActualValue::Ssa(SsaValueIdentity { function, .. }) if function == "::top"
    )));

    let outer = plan
        .semantic_calls()
        .find_map(|(site, decision)| match decision {
            SemanticCallDecision::Selected(evidence)
                if evidence.operation
                    == SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite) =>
            {
                Some((site, evidence))
            }
            _ => None,
        })
        .expect("outer channel-write call selected");
    assert_eq!(outer.0.nested_argument, None);
    assert!(matches!(
        outer.1.arguments.as_slice(),
        [SemanticCallArgument::NestedDirect {
            outer_argument: 0,
            call
        }] if call.nested_argument == Some(0)
    ));
}

#[test]
fn original_literal_actual_types_do_not_create_native_or_ssa_value_identity() {
    // naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit("proc p {x} {}; p 2", std::sync::Arc::clone(&context));
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    let selected = plan
        .direct_calls()
        .find_map(|(_, decision)| match decision {
            DirectProcDecision::Selected(evidence) if evidence.callee.qualified_name == "::p" => {
                Some(evidence)
            }
            _ => None,
        })
        .expect("independent actual calling convention selects the original empty procedure");
    assert_eq!(selected.actual_types[0].tcl_type(), Some(TclType::Int));
    assert_eq!(selected.actual_values, [DirectActualValue::Unproven]);
    assert!(
        !plan
            .materialisable_slots()
            .any(|(_, decision)| { matches!(decision, MaterialisableSlotDecision::Selected(_)) })
    );
    let function = &unit.top_level;
    let site = call_sites(&unit, "::top", function, function.source_lexer_config())
        .into_iter()
        .find(|site| site.command == "p")
        .unwrap();
    let mut missing = unit.ir_module.clone();
    missing.source_metadata_input = None;
    let (contents, identity) = actual_fact(function, &site, 0, context.commands(), &missing);
    assert!(matches!(contents.kind(), TypeKind::Unknown));
    assert_eq!(identity, DirectActualValue::Unproven);
    let foreign = tcl_registry::CommandRegistry::build_default();
    let (contents, identity) = actual_fact(function, &site, 0, &foreign, &unit.ir_module);
    assert!(matches!(contents.kind(), TypeKind::Unknown));
    assert_eq!(identity, DirectActualValue::Unproven);
}

#[test]
fn declared_arguments_keep_distinct_reached_cells_and_exact_source_receipts() {
    // naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit(
        "proc p {x y} {return [expr {$x + $y}]}; p 1 2; p 3 4",
        std::sync::Arc::clone(&context),
    );
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    let arguments: Vec<_> = plan
        .declared_arguments()
        .filter_map(|(identity, decision)| {
            let DeclaredArgumentDecision::Selected(evidence) = decision else {
                return None;
            };
            Some((identity, evidence))
        })
        .collect();
    assert_eq!(arguments.len(), 2, "{plan:#?}");
    assert_eq!(
        arguments
            .iter()
            .map(|(identity, _)| identity.ordinal)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert!(
        arguments
            .iter()
            .all(|(_, evidence)| evidence.reads().iter().any(|read| read.cells.len() >= 2))
    );
    assert!(
        plan.materialisable_slots()
            .all(|(identity, decision)| identity.function != "::p"
                || !matches!(decision, MaterialisableSlotDecision::Selected(_)))
    );
    let (identity, evidence) = arguments[0];
    let read = &evidence.reads()[0];
    assert!(
        plan.declared_argument_for_incoming_read(&identity.procedure, read)
            .is_some()
    );
    let mut wrong_source = read.clone();
    wrong_source.source.span =
        tcl_lexer::Span::new(read.source.span.start() + 1, read.source.span.end());
    let mut wrong_cell = read.clone();
    wrong_cell.cells.pop();
    let mut wrong_slot = read.clone();
    wrong_slot.slot = "y".into();
    for invalid in [wrong_source, wrong_cell, wrong_slot] {
        assert!(
            plan.declared_argument_for_incoming_read(&identity.procedure, &invalid)
                .is_none()
        );
    }
    let mut wrong_declaration = identity.procedure.clone();
    wrong_declaration.definition_start += 1;
    assert!(
        plan.declared_argument_for_incoming_read(&wrong_declaration, read)
            .is_none()
    );
}

#[test]
fn declared_arguments_refuse_missing_owners_changed_contents_and_future_callers() {
    // naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit(
        "proc p {x} {return [expr {$x + 0}]}; p 1",
        std::sync::Arc::clone(&context),
    );
    let make_plan = |unit: &CompilationUnit| {
        CommonAotProofPlan::build_with_retained_metadata(
            unit,
            context.commands(),
            enabled(),
            CommonAotEnvironment::Hosted,
        )
    };
    assert!(
        make_plan(&unit)
            .declared_arguments()
            .any(|(_, decision)| matches!(decision, DeclaredArgumentDecision::Selected(_)))
    );
    let mut missing = (*unit).clone();
    missing.ir_module.source_metadata_input = None;
    let mut changed = (*unit).clone();
    changed.ir_module.source =
        tcl_lexer::SourceImage::document("proc p {x} {return DIFFERENT}; p 1");
    for refused in [&missing, &changed] {
        assert!(
            make_plan(refused)
                .declared_arguments()
                .all(|(_, decision)| matches!(decision, DeclaredArgumentDecision::Declined(_)))
        );
    }
    for source in [
        "proc p {x} {set x 9; return [expr {$x + 0}]}; p 1",
        "proc p {x} {trace add variable x read {list}; return [expr {$x + 0}]}; p 1",
        "proc p {x} {return [expr {$x + 0}]}; proc future {y} {p $y}; p 1",
    ] {
        let unit = native_unit(source, std::sync::Arc::clone(&context));
        let plan = make_plan(&unit);
        assert!(
            plan.declared_arguments()
                .all(
                    |(identity, decision)| identity.procedure.qualified_name != "::p"
                        || matches!(decision, DeclaredArgumentDecision::Declined(_))
                ),
            "{source}: {plan:#?}"
        );
    }
}

#[test]
fn semantic_operation_pass_is_independently_default_off() {
    let config = enabled().with_enabled(SemanticOptimisationPassId::DirectProc);
    let mut without_semantic = config;
    without_semantic.disable(SemanticOptimisationPassId::SemanticOperationSpecialisation);
    let plan = plan(ADD, without_semantic);
    assert!(plan.semantic_calls().all(|(_, decision)| matches!(
        decision,
        SemanticCallDecision::Declined(SemanticCallDecline::PassDisabled)
    )));
}

#[test]
fn semantic_operation_rebinding_alias_trace_and_dynamic_mutation_decline() {
    for (source, expected) in [
        (
            "proc add {b c} {return [expr {$b+$c}]}\nrename puts q\nq [add 2 4]\n",
            SemanticCallDecline::ReboundOrAliased,
        ),
        (
            "proc add {b c} {return [expr {$b+$c}]}\nrename puts coreputs\ninterp alias {} puts {} coreputs\nputs [add 2 4]\n",
            SemanticCallDecline::BindingNotOriginal {
                kind: BindingKind::Alias,
            },
        ),
        (
            "proc cb args {}; proc add {b c} {return [expr {$b+$c}]}\ntrace add execution puts enter cb\nputs [add 2 4]\n",
            SemanticCallDecline::ExecutionTrace,
        ),
        (
            "proc mutate {name} {rename $name q}\nproc add {b c} {return [expr {$b+$c}]}\nputs [add 2 4]\n",
            SemanticCallDecline::DynamicCommandMutation,
        ),
    ] {
        let plan = plan(source, enabled());
        assert!(
            plan.semantic_calls().any(|(_, decision)| matches!(
                decision,
                SemanticCallDecision::Declined(reason) if reason == &expected
            )),
            "{source}: {plan:#?}"
        );
    }
}

#[test]
fn hosted_top_level_state_is_explicitly_observable() {
    let plan = plan(ADD, enabled());
    assert_eq!(plan.environment(), CommonAotEnvironment::Hosted);
    assert!(!plan.environment().permits_native_only());
    assert!(plan.materialisable_slots().any(|(identity, decision)| {
        identity.function == "::top"
            && matches!(
                decision,
                MaterialisableSlotDecision::Declined(
                    MaterialisableSlotDecline::HostedTopLevelObservable
                )
            )
    }));
    assert!(plan.materialisable_slots().all(|(_, decision)| {
        !matches!(
            decision,
            MaterialisableSlotDecision::Selected(evidence)
                if evidence.storage == VarStorage::NativeOnly
        )
    }));
    assert!(CommonAotEnvironment::SealedProgram.permits_native_only());
}

#[test]
fn retained_native_program_coverage_does_not_donate_root_frame_slots() {
    // naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit(ADD, std::sync::Arc::clone(&context));
    assert!(
        unit.ir_module
            .source_entry
            .native_entry
            .as_ref()
            .unwrap()
            .compiled_local_layout
            .is_none()
    );
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::SealedProgram,
    );
    let ClosedProgramCoverageDecision::Selected(coverage) = plan.closed_program_coverage() else {
        panic!("sealed program coverage missing: {plan:#?}");
    };
    assert_eq!(coverage.statements.len(), 4);
    assert!(matches!(
        coverage.statements.as_slice(),
        [
            ClosedProgramStatementEvidence::DirectProcedureDefinition { .. },
            ClosedProgramStatementEvidence::DirectActualConstant { .. },
            ClosedProgramStatementEvidence::DirectActualConstant { .. },
            ClosedProgramStatementEvidence::SemanticBoundary { .. }
        ]
    ));
    for variable in ["d", "e"] {
        let decisions: Vec<_> = plan
            .materialisable_slots()
            .filter(|(identity, _)| {
                identity.function == "::top"
                    && unit.top_level.ssa.var_name(identity.symbol) == variable
            })
            .map(|(_, decision)| decision)
            .collect();
        assert!(
            !decisions.is_empty(),
            "original root {variable} is represented"
        );
        assert!(
            decisions.iter().all(|decision| matches!(
                decision,
                MaterialisableSlotDecision::Declined(
                    MaterialisableSlotDecline::OriginalSlotUnavailable
                )
            )),
            "captured ScriptCode cannot borrow a procedure local: {decisions:#?}"
        );
    }

    let (direct_site, direct) = plan
        .direct_calls()
        .find_map(|(site, decision)| match decision {
            DirectProcDecision::Selected(evidence) if evidence.callee.qualified_name == "::add" => {
                Some((site, evidence))
            }
            _ => None,
        })
        .expect("sealed direct add selected");
    assert!(direct.frame_elidable);
    assert!(matches!(direct.body, DirectProcBodyDecision::Selected(_)));
    assert!(plan.semantic_calls().any(|(_, decision)| matches!(
        decision,
        SemanticCallDecision::Selected(evidence) if matches!(
            evidence.as_ref(),
            SemanticCallEvidence {
                operation: SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite),
                arguments,
                ..
            } if matches!(
                arguments.as_slice(),
                [SemanticCallArgument::NestedDirect { outer_argument: 0, call }]
                    if call == direct_site
            )
        )
    )));
}

#[test]
fn frame_elision_has_an_independent_default_off_gate() {
    let config =
        SemanticOptimisationConfig::new().with_enabled(SemanticOptimisationPassId::DirectProc);
    let plan = plan(ADD, config);
    assert!(plan.direct_calls().any(|(_, decision)| {
        matches!(
            decision,
            DirectProcDecision::Selected(evidence) if matches!(
                evidence.as_ref(),
                DirectProcEvidence {
                    body: DirectProcBodyDecision::Declined(
                        DirectProcBodyDecline::FrameElisionPassDisabled
                    ),
                    frame_elidable: false,
                    ..
                }
            )
        )
    }));
}

#[test]
fn internal_operation_rebinding_and_tracing_decline_body_specialisation() {
    for (source, expected_operation, transitive_mutation_is_dynamic) in [
        (
            "proc add {b c} {return [expr {$b+$c}]}\nrename expr saved_expr\nadd 2 4\n",
            SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
            true,
        ),
        (
            "proc cb args {}; proc add {b c} {return [expr {$b+$c}]}\ntrace add execution expr enter cb\nadd 2 4\n",
            SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
            false,
        ),
    ] {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let unit = native_unit(source, std::sync::Arc::clone(&context));
        assert_eq!(
            unit.command_mutations.has_dynamic_mutation(),
            transitive_mutation_is_dynamic,
            "the closed effect projection must retain unresolved/autoload opacity"
        );
        assert!(
            unit.caller_scope
                .proc_binding_trust
                .trusts_proc_binding("::add"),
            "body-dispatch opacity must not erase the separately proven proc identity"
        );
        let plan = CommonAotProofPlan::build_with_retained_metadata(
            &unit,
            registry,
            enabled(),
            CommonAotEnvironment::Hosted,
        );
        assert!(
            plan.direct_calls().any(|(_, decision)| match decision {
                DirectProcDecision::Selected(evidence) => matches!(
                    evidence.body,
                    DirectProcBodyDecision::Declined(
                        DirectProcBodyDecline::InternalDispatchUntrusted { operation }
                            | DirectProcBodyDecline::InternalExecutionTrace { operation }
                    ) if operation == expected_operation
                ),
                DirectProcDecision::Declined(_) => false,
            }),
            "{source}: {plan:#?}"
        );
    }
}

#[test]
fn known_computed_rename_preserves_callee_identity_and_rejects_its_internal_operation() {
    let proof = plan(
        "proc add {b c} {return [expr {$b+$c}]}; set command expr; rename $command saved_expr; add 2 4",
        enabled(),
    );
    assert!(
        proof.direct_calls().any(|(_, decision)| matches!(
            decision,
            DirectProcDecision::Selected(evidence) if matches!(
                evidence.as_ref(),
                DirectProcEvidence {
                    body: DirectProcBodyDecision::Declined(
                        DirectProcBodyDecline::InternalDispatchUntrusted { .. }
                    ),
                    ..
                }
            )
        )),
        "{proof:#?}"
    );
}

#[test]
fn dynamic_binding_transition_still_declines_direct_call_identity() {
    let plan = plan(
        "proc add {b c} {return [expr {$b+$c}]}\n\
             set command [lindex $argv 0]\nrename $command saved_expr\nadd 2 4\n",
        enabled(),
    );
    assert!(plan.direct_calls().any(|(_, decision)| matches!(
        decision,
        DirectProcDecision::Declined(DirectProcDecline::DynamicCommandMutation)
    )));
}

#[test]
fn dialect_tcloo_and_variable_trace_premises_are_retained() {
    let registry = tcl_registry::CommandRegistry::build_default();
    // An availability set spanning several dialects is not something a
    // production caller can produce, so it is not what this pins.
    // The executable-IR vocabulary is now a resolved
    // environment, which names exactly one context or none, so an
    // ambiguous premise is unrepresentable rather than declined. The
    // decline it pins is therefore the surviving one: a unit built with
    // no dialect carries no context.
    let unit = CompilationUnit::build_for(ADD, &registry, false);
    let ambiguous = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        &registry,
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    assert!(ambiguous.direct_calls().any(|(_, decision)| matches!(
        decision,
        DirectProcDecision::Declined(DirectProcDecline::ContextUnavailable)
    )));

    let oo = plan(
        "oo::class create C { method m {x} { return $x } }\n",
        enabled(),
    );
    assert!(
        oo.coverage_declines()
            .contains(&CommonAotCoverageDecline::TclOoMethods)
    );

    let traced = plan(
        "proc p {x} {return $x}\ntrace add variable x read cb\np 1\n",
        enabled(),
    );
    assert!(traced.materialisable_slots().any(|(_, decision)| matches!(
        decision,
        MaterialisableSlotDecision::Declined(MaterialisableSlotDecline::VariableTrace)
    )));
}

#[test]
fn unknown_actual_type_poisons_cross_call_formal_propagation() {
    // naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit(
        "proc add {b c} {return [expr {$b+$c}]}\nproc caller {x} {set d 2; add $d $x}\nset d 2\nset e 4\nadd $d $e\ncaller [eval {set original_input 4}]\n",
        std::sync::Arc::clone(&context),
    );
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    // The second caller is actually entered. Its original eval operand
    // is not a represented SSA actual, so the call's type remains unknown
    // independently of the runtime bytes produced by the original setter.
    // Exact source reads can be retained without a scalar SSA entry key.
    // Join the same selected-caller type owner used by materialisation;
    // absence of a physical formal SSA key cannot be replaced by fake v0.
    let mut propagated = HashMap::new();
    let mut selected_callers = 0;
    for (_, decision) in plan.direct_calls() {
        if let DirectProcDecision::Selected(evidence) = decision
            && evidence.callee.qualified_name == "::add"
        {
            selected_callers += 1;
            propagate_actual_types(&mut propagated, evidence.callee.id, decision);
        }
    }
    assert_eq!(
        selected_callers,
        2,
        "{}\n{}\n{plan:#?}",
        original_formal_diagnostics(&unit, "::caller"),
        original_formal_diagnostics(&unit, "::add"),
    );
    let actual = propagated
        .get(&(fixture_id(&unit.ir_module.procedures, "::add"), 1))
        .expect("both callers recorded");
    assert!(
        actual
            .as_ref()
            .is_none_or(|value| value.single_shape().is_none())
    );
    assert!(!plan.materialisable_slots().any(|(identity, decision)| {
        identity.function == "::add" && matches!(decision, MaterialisableSlotDecision::Selected(_))
    }));
}

#[test]
fn unentered_unknown_caller_keeps_a_refusal_without_fabricating_a_reached_call() {
    // naming.variable.aot-original-slot-purpose
    // docs/design/analysis/name-resolution-proofs/aot-original-slot-purpose.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit(
        "proc add {b c} {return [expr {$b+$c}]}\nproc caller {x} {set d 2; add $d $x}\nset d 2\nset e 4\nadd $d $e\n",
        std::sync::Arc::clone(&context),
    );
    let plan = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    assert_eq!(
        plan.direct_calls()
            .filter(|(_, decision)| matches!(decision,
                DirectProcDecision::Selected(evidence) if evidence.callee.qualified_name == "::add"
            ))
            .count(),
        1,
        "{plan:#?}"
    );
    assert!(
        plan.direct_calls()
            .any(|(site, decision)| site.function == "::caller"
                && matches!(
                    decision,
                    DirectProcDecision::Declined(DirectProcDecline::BindingNotProcedure {
                        kind: BindingKind::Unknown
                    })
                )),
        "future conditional lookup cannot grant reached procedure dispatch: {plan:#?}"
    );
    assert!(
        plan.declared_arguments()
            .any(
                |(identity, decision)| identity.procedure.qualified_name == "::add"
                    && identity.ordinal == 1
                    && matches!(
                        decision,
                        DeclaredArgumentDecision::Declined(
                            MaterialisableSlotDecline::TypeNotSingleton
                        )
                    )
            ),
        "unresolved source caller remains an independent poison premise: {plan:#?}"
    );
}

#[test]
fn direct_calls_retain_native_argv_expansion_at_the_generic_boundary() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let unit = native_unit(
        "proc p {x y} {return $x}; p {*}{2 4}",
        std::sync::Arc::clone(&context),
    );
    let proof = CommonAotProofPlan::build_with_retained_metadata(
        &unit,
        context.commands(),
        enabled(),
        CommonAotEnvironment::Hosted,
    );
    assert!(
        proof.direct_calls().any(|(_, decision)| matches!(
            decision,
            DirectProcDecision::Declined(DirectProcDecline::ExpandedArgumentsUnsupported)
        )),
        "{proof:#?}"
    );
    assert!(proof.has_declined_direct_call_to(fixture_id(&unit.ir_module.procedures, "::p")));
}

#[test]
fn defaults_variadics_rebinding_aliases_and_traces_decline() {
    for (source, expected) in [
        (
            "proc p {{x 1}} {return $x}\np\n",
            DirectProcDecline::DefaultArgumentUnsupported,
        ),
        (
            "proc p {args} {return $args}\np 1\n",
            DirectProcDecline::VariadicUnsupported,
        ),
        (
            "proc p {x} {return $x}\nrename p q\nq 1\n",
            DirectProcDecline::ReboundOrAliased,
        ),
        (
            "proc p {x} {return $x}\ninterp alias {} q {} p\nq 1\n",
            DirectProcDecline::ReboundOrAliased,
        ),
        (
            "proc p {x} {return $x}\ntrace add execution p enter cb\np 1\n",
            DirectProcDecline::ExecutionTrace,
        ),
    ] {
        let plan = plan(source, enabled());
        assert!(
            plan.direct_calls().any(|(_, decision)| {
                matches!(decision, DirectProcDecision::Declined(reason) if reason == &expected)
            }),
            "{source}: {plan:?}"
        );
    }
}

#[test]
fn exceptional_edges_decline_direct_and_slot_plans() {
    let plan = plan(
        "proc p {x} {try {return [expr {$x+1}]} on error e {return $e}}\np 1\n",
        enabled(),
    );
    assert!(plan.direct_calls().any(|(_, decision)| matches!(
        decision,
        DirectProcDecision::Declined(DirectProcDecline::ExceptionalControlFlow)
    )));
    assert!(plan.materialisable_slots().any(|(_, decision)| matches!(
        decision,
        MaterialisableSlotDecision::Declined(MaterialisableSlotDecline::ExceptionalControlFlow)
    )));
}

fn sealed_plan(source: &str) -> CommonAotProofPlan {
    plan_in_environment(source, enabled(), CommonAotEnvironment::SealedProgram)
}

#[test]
fn a_procedure_with_an_execution_trace_keeps_its_body_unspecialised() {
    // Tcl 9.0: the enter trace runs before `add`'s body, so neither the call
    // nor the body may be replaced by a direct native sequence.
    let traced = sealed_plan(&format!(
        "proc cb args {{}}\ntrace add execution add enter cb\n{ADD}"
    ));
    assert!(
        traced.direct_calls().all(|(_, decision)| !matches!(
            decision,
            DirectProcDecision::Selected(evidence)
                if evidence.callee.qualified_name == "::add"
                    && matches!(evidence.body, DirectProcBodyDecision::Selected(_))
        )),
        "{traced:#?}"
    );
    assert!(
        matches!(
            traced.closed_program_coverage(),
            ClosedProgramCoverageDecision::Declined(_)
        ),
        "{traced:#?}"
    );
}

#[test]
fn a_constant_store_to_a_variable_with_a_write_trace_is_not_covered() {
    // Tcl 9.0: `set d 2` runs the write trace, so the store is observable and
    // cannot be folded into the sealed program as a direct actual constant.
    let traced = sealed_plan(&format!(
        "proc cb args {{}}\ntrace add variable d write cb\n{ADD}"
    ));
    assert!(
        matches!(
            traced.closed_program_coverage(),
            ClosedProgramCoverageDecision::Declined(_)
        ),
        "{traced:#?}"
    );
}
