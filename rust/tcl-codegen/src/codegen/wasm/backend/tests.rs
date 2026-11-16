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

use tcl_compiler::interprocedural::test_body_identity::fixture_id_if_present;

use super::*;

#[test]
fn condition_argv_and_native_emitters_share_actual_abi_import_indices() {
    let mut module = WasmModule::new();
    let general = add_general_imports(&mut module, true);
    let argv = general.aot.unwrap().argv;
    assert_eq!(general.condition.frame_alloc, argv.frame_alloc);
    assert_eq!(general.condition.frame_free, argv.frame_free);
    assert_eq!(general.condition.string_owned, argv.string_owned);
    assert_eq!(general.condition.object_release, argv.object_release);
    let _ = native_emit::add_native_imports(&mut module, &mut add_codegen_import);
    let identities = module
        .imports
        .iter()
        .map(|import| (&import.module, &import.name, import.type_idx))
        .collect::<HashSet<_>>();
    assert_eq!(identities.len(), module.imports.len());
    assert_eq!(
        add_codegen_import(&mut module, CodegenAbiImportId::CallFrameAlloc),
        general.condition.frame_alloc
    );
    assert_eq!(
        add_codegen_import(&mut module, CodegenAbiImportId::CallFrameFree),
        argv.frame_free
    );
    let alloc = usize::try_from(argv.frame_alloc).unwrap();
    assert_eq!(
        module.imports[alloc].name,
        CodegenAbiImportId::CallFrameAlloc.descriptor().name
    );
}

/// Plan the top-level statements of `source` under the full analysed tier.

#[test]
fn redefined_original_procedure_versions_have_distinct_physical_exports() {
    for source in [
        include_str!("../../../../../../samples/wasm/t3-procs/35_native_dispatch_matrix.tcl"),
        include_str!("../../../../../../samples/wasm/t7-dynamic/71_exec_traces_rename.tcl"),
    ] {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let unit = native_source_unit(
            std::sync::Arc::clone(&context),
            source,
            tcl_lexer::LexerConfig::for_dialect("tcl9.0"),
            tcl_dialect::DialectProfile::find("tcl9.0").unwrap(),
        );
        for options in [
                WasmCompileOptions::standalone(true),
                WasmCompileOptions::standalone(true).with_semantic_optimisation(
                    tcl_compiler::semantic_optimisation::SemanticOptimisationPassId::LegacyAnalysisSpecialisation),
                WasmCompileOptions::standalone(true).native_tier(),
            ] {
                let mut module = super::super::pipeline::compile_wasm(
                    &unit, context.commands(), options,
                ).module;
                let exports = module.functions.iter().filter(|function| function.exported)
                    .map(|function| function.name.as_str()).collect::<Vec<_>>();
                assert_eq!(exports.len(), exports.iter().copied().collect::<HashSet<_>>().len());
                let versions = module.source_function_ids.values().filter(|owner|
                    matches!(owner, tcl_compiler::native_lowering::NativeFunctionId::Procedure(_))).count();
                assert!(versions >= 2, "actual retained original procedure versions");
                assert!(exports.contains(&"::top"));
                assert!(exports.iter().any(|name| name.starts_with("__tcl_wasm_body_")),
                    "actual redefinition requires physical body hygiene");
                assert_ne!(module.to_bytes(), [] as [u8; 0]);
                assert_ne!(module.to_wat(), "");
            }
    }
}

fn top_level_facts(source: &str) -> FunctionFacts {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let unit = retained_source_unit(
        std::sync::Arc::clone(&context),
        source,
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    );
    function_facts(
        &unit.top_level,
        &unit.ir_module,
        context.commands(),
        &unit.command_mutations,
        true,
    )
}

fn retained_source_unit(
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> crate::test_support::RetainedNativeUnit {
    native_source_unit(
        context,
        source,
        config,
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    )
}

fn native_source_unit(
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
    source: &str,
    config: tcl_lexer::LexerConfig,
    profile: &'static tcl_dialect::DialectProfile,
) -> crate::test_support::RetainedNativeUnit {
    let result = native_source_unit_borrowed(&context, source, config, profile);
    drop(context);
    result
}

fn native_source_unit_borrowed(
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
    source: &str,
    config: tcl_lexer::LexerConfig,
    profile: &'static tcl_dialect::DialectProfile,
) -> crate::test_support::RetainedNativeUnit {
    let (owner, captured) = crate::test_support::captured_native_entry_with_owner(profile);
    let entry = tcl_compiler::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_compiler::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_context_registry(
        source,
        tcl_compiler::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(context),
    );
    crate::test_support::RetainedNativeUnit::new(unit, owner)
}

fn assignment_facts(
    function: &FunctionUnit,
    unit: &CompilationUnit,
    registry: &CommandRegistry,
) -> FunctionFacts {
    function_facts(
        function,
        &unit.ir_module,
        registry,
        &unit.command_mutations,
        true,
    )
}

fn original_assignment_diagnostics(
    function: &FunctionUnit,
    unit: &CompilationUnit,
    registry: &CommandRegistry,
) -> String {
    let metadata = function.invocation_metadata_context_for_module(registry, &unit.ir_module);
    let initial = procedure_shadow_obligations(&unit.ir_module, registry, true)
        .expect("top level has no prior procedure shadow obligations");
    let bindings = analyse_command_binding(&function.cfg, registry, &initial);
    let mut rows = vec![format!("metadata available: {}", metadata.is_some())];
    for (&block, body) in &function.cfg.blocks {
        for (index, statement) in body.statements.iter().enumerate() {
            if !matches!(
                statement,
                Statement::AssignConst { .. } | Statement::AssignValue { .. }
            ) {
                continue;
            }
            let tokens = tcl_compiler::ir::CommandBindingSites::unanimous_statement_source_tokens(
                &function.cfg.command_binding_sites,
                statement,
            );
            rows.push(format!(
                "{block:?}/{index} {statement:?} original carrier: {}",
                tokens.is_some()
            ));
            if let Some(tokens) = tokens {
                let target = tokens.source_binding.as_ref().and_then(|binding| {
                    binding
                        .proved_execution_target()
                        .map(|target| (&target.command, target.kind, target.registry_backed))
                });
                let handler = tokens.source_binding.as_ref().and_then(|binding| {
                    binding
                        .proved_handler_target()
                        .map(|target| (&target.command, target.kind, target.registry_backed))
                });
                let literal =
                    original_assignment_literal(function, &unit.ir_module, registry, statement);
                let resolution = resolve_command_tokens_with_metadata_context(
                    registry, metadata, tokens,
                )
                .map(|resolved| match resolved {
                    RegistryInvocationResolution::Resolved(facts) => {
                        format!("{:?}: {}", facts.operation, facts.canonical_command)
                    }
                    other @ RegistryInvocationResolution::Unresolved(_) => {
                        format!("{other:?}")
                    }
                });
                rows.push(format!(
                        "words={:?}, aligned={}, target={target:?}, handler={handler:?}, literal={literal:?}, resolution={resolution:?}, flow={:?}, effective={:?}",
                        tokens.argv_texts,
                        tokens.words_align_with_argv_text(),
                        bindings.binding_at(block, index, tokens.argv_texts.first().map_or("", String::as_str)),
                        tcl_compiler::registry_invocation::effective_command_words(tokens),
                    ));
            }
        }
    }
    rows.join("\n")
}

#[test]
fn original_publication_names_ignore_reports_and_keep_duplicate_issued_ids() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let unit = retained_source_unit(
        std::sync::Arc::clone(&context),
        "proc append {} {return 1}\nproc append {} {return 2}\n",
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    );
    let original = OriginalProcedureNames::capture(&unit.ir_module, registry)
        .expect("genuine current primary source publication inventory");
    let expected = original.redefined_ids();
    assert_eq!(expected.len(), 2);
    let shadows = original.shadow_obligations().unwrap();
    assert_eq!(shadows.len(), 2);
    assert!(
        shadows
            .iter()
            .all(|(name, binding)| name == "::append" && binding.kind == BindingKind::Unknown)
    );
    let mut reports = unit.ir_module.clone();
    for procedure in reports.procedures.values_mut() {
        procedure.qualified_name = "::report::unrelated".to_owned();
    }
    reports.redefined_procedures.clear();
    let current = OriginalProcedureNames::capture(&reports, registry).unwrap();
    assert_eq!(current.redefined_ids(), expected);
    assert_eq!(current.shadow_obligations().unwrap(), shadows);

    let id = *expected.iter().next().unwrap();
    reports.procedures.get_mut(&id).unwrap().publication = None;
    assert!(OriginalProcedureNames::capture(&reports, registry).is_none());
    let mut wrong_kind = unit.ir_module.clone();
    wrong_kind.procedures.get_mut(&id).unwrap().body_kind =
        tcl_compiler::ir::ProcedureBodyKind::OriginalDeclaration;
    assert!(OriginalProcedureNames::capture(&wrong_kind, registry).is_none());
    let mut missing = unit.ir_module.clone();
    missing.retained_source_bindings = None;
    assert!(OriginalProcedureNames::capture(&missing, registry).is_none());
    assert!(procedure_shadow_obligations(&missing, registry, false).is_none());
}

#[test]
fn original_native_body_compilation_keeps_publication_and_direct_authority_separate() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for_dialect(
        "proc p {x} {return $x}\nset answer [p 42]\n",
        &registry,
        false,
        "tcl9.0",
    );
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    assert!(unit.ir_module.procedures.is_empty());
    let plan = procedure_plan(&unit.ir_module, Some((&unit, &registry)), 0);
    assert_eq!(plan.procs.len(), 1);
    assert!(plan.direct.is_empty());
    let (id, procedure) = plan.procs[0];
    assert_eq!(
        procedure.body_kind,
        tcl_compiler::ir::ProcedureBodyKind::OriginalDeclaration
    );
    assert!(procedure.publication.is_none());
    assert_eq!(unit.function_for_body(Some(id)).unwrap().body_id, Some(id));
    let site = procedure.declaration_site.as_ref().unwrap();
    let command_span =
        tcl_lexer::Span::new(0, u32::try_from("proc p {x} {return $x}".len()).unwrap());
    assert_ne!(procedure.span, command_span);
    assert_eq!(
        unit.ir_module
            .native_source_procedure_bodies_for_compilation_at_declaration(site, command_span)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        vec![id]
    );
    assert!(
        unit.ir_module
            .native_source_procedure_bodies_for_compilation_at_declaration(site, procedure.span)
            .next()
            .is_none()
    );
    let mut missing = unit.ir_module.clone();
    missing.original_native_declaration_body_units.clear();
    assert_eq!(
        procedure_plan(&missing, None, 0).procs,
        [] as [(tcl_runtime_api::ProcedureId, &tcl_compiler::ir::Procedure); 0]
    );
    let mut stale = unit.ir_module.clone();
    stale.source = tcl_lexer::SourceImage::document("changed source");
    assert_eq!(
        procedure_plan(&stale, None, 0).procs,
        [] as [(tcl_runtime_api::ProcedureId, &tcl_compiler::ir::Procedure); 0]
    );
}

#[test]
fn original_assignment_literals_keep_decoding_separate_from_execution_authority() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig {
        escapes: tcl_dialect::EscapeSyntax::Tcl84,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let unit = CompilationUnit::build_with_analysis_input(
        r"set result \U00000041",
        tcl_compiler::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let statement = unit.top_level.cfg.blocks.values().flat_map(|block| &block.statements)
            .find(|statement| matches!(statement, Statement::AssignValue { name, .. } if name == "result"))
            .expect("upstream bare literal retains AssignValue");
    assert_eq!(
        original_assignment_literal(
            &unit.top_level,
            &unit.ir_module,
            context.commands(),
            statement
        )
        .as_deref(),
        Some("U00000041")
    );
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    assert!(
        assignment_facts(&unit.top_level, &unit, context.commands())
            .direct_assignments
            .is_empty()
    );
    let mut changed = statement.clone();
    let Statement::AssignValue { value, .. } = &mut changed else {
        unreachable!()
    };
    *value = "OTHER".to_owned();
    assert!(
        original_assignment_literal(
            &unit.top_level,
            &unit.ir_module,
            context.commands(),
            &changed
        )
        .is_none()
    );
}

#[test]
fn assignment_plans_retain_decoded_original_names_separately_from_spelling() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    for (source, expected) in [
        (r"set a\x62 VALUE", "ab"),
        ("set {$name} VALUE", "$name"),
        ("set {} VALUE", ""),
    ] {
        let unit = retained_source_unit(
            std::sync::Arc::clone(&context),
            source,
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        );
        let statement = unit
            .top_level
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .find(|statement| {
                matches!(
                    statement,
                    Statement::AssignConst { .. } | Statement::AssignValue { .. }
                )
            })
            .expect("original lowered assignment");
        let (Statement::AssignConst { name, .. } | Statement::AssignValue { name, .. }) = statement
        else {
            unreachable!()
        };
        assert_eq!(
            original_assignment_name(
                &unit.top_level,
                &unit.ir_module,
                context.commands(),
                statement,
                name
            )
            .as_deref(),
            Some(expected)
        );
        let planned = assignment_facts(&unit.top_level, &unit, context.commands());
        assert_eq!(
            planned.direct_assignments.get(&span_key(statement.span())),
            Some(&(expected.to_owned(), "VALUE".to_owned()))
        );
        let mut missing = unit.ir_module.clone();
        missing.retained_source_bindings = None;
        assert!(
            original_assignment_name(
                &unit.top_level,
                &missing,
                context.commands(),
                statement,
                name
            )
            .is_none()
        );
        let mut stale = statement.clone();
        match &mut stale {
            Statement::AssignConst { name, .. } | Statement::AssignValue { name, .. } => {
                *name = "OTHER".to_owned();
            }
            _ => unreachable!(),
        }
        assert!(
            original_assignment_name(
                &unit.top_level,
                &unit.ir_module,
                context.commands(),
                &stale,
                "OTHER"
            )
            .is_none()
        );
    }
}

#[test]
fn structural_assignments_require_actual_availability_and_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Availability selection retains a separate actual command-entry owner;
    // it does not assert successful native compilation or fixture execution.
    use std::sync::Arc;
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut setter = registry.get("set").unwrap().clone();
    setter.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(setter);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let registry = current.commands();
    let unit = retained_source_unit(
        Arc::clone(&current),
        "set result VALUE",
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    );
    let selected = &unit.top_level;
    assert_eq!(
        assignment_facts(selected, &unit, registry)
            .direct_assignments
            .len(),
        1,
        "{}",
        original_assignment_diagnostics(selected, &unit, registry),
    );
    let input = selected.source_metadata_input().unwrap();
    let replace_input = |context| {
        let mut changed = selected.clone();
        changed.source_metadata_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            context,
            input.lexer_config(),
        ));
        changed
    };
    let older = replace_input(Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(registry)),
    ));
    assert!(Arc::ptr_eq(
        older
            .source_metadata_input()
            .unwrap()
            .context_registry()
            .commands(),
        registry
    ));
    let mut older_module = unit.ir_module.clone();
    older_module.source_metadata_input = older.source_metadata_input.clone();
    assert!(
        function_facts(
            &older,
            &older_module,
            registry,
            &unit.command_mutations,
            true,
        )
        .direct_assignments
        .is_empty()
    );
    let foreign = replace_input(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
    );
    let mut missing = selected.clone();
    missing.source_metadata_input = None;
    let mut different_grammar = selected.clone();
    different_grammar.source_config.strict_quoting =
        !different_grammar.source_config.strict_quoting;
    let mut missing_module = unit.ir_module.clone();
    missing_module.source_metadata_input = None;
    assert!(
        function_facts(
            selected,
            &missing_module,
            registry,
            &unit.command_mutations,
            true,
        )
        .direct_assignments
        .is_empty()
    );
    for refused in vec![older, foreign, missing, different_grammar].into_boxed_slice() {
        assert!(
            assignment_facts(&refused, &unit, registry)
                .direct_assignments
                .is_empty()
        );
    }
}

#[test]
fn structural_assignments_need_unanimous_original_source_carriers() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let unit = retained_source_unit(
        std::sync::Arc::clone(&context),
        "set result VALUE",
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    );
    let registry = context.commands();
    let selected = &unit.top_level;
    assert_eq!(
        assignment_facts(selected, &unit, registry)
            .direct_assignments
            .len(),
        1,
        "{}",
        original_assignment_diagnostics(selected, &unit, registry),
    );
    let mut missing = selected.clone();
    missing.cfg.command_binding_sites.clear();
    // AssignValue retains its own original carrier beside the consumed
    // command sites. This fixture withdraws both real sources of evidence.
    for block in missing.cfg.blocks.values_mut() {
        for statement in &mut block.statements {
            if let Statement::AssignValue { tokens, .. } = statement {
                *tokens = None;
            }
        }
    }
    assert!(
        assignment_facts(&missing, &unit, registry)
            .direct_assignments
            .is_empty()
    );
    let mut conflicting = selected.clone();
    let mut additional = conflicting
        .cfg
        .command_binding_sites
        .iter()
        .find(|site| site.source_tokens.is_some())
        .unwrap()
        .clone();
    let tokens = additional.source_tokens.as_mut().unwrap();
    tokens.argv_texts[0] = "different".into();
    conflicting.cfg.command_binding_sites.push(additional);
    assert!(
        assignment_facts(&conflicting, &unit, registry)
            .direct_assignments
            .is_empty()
    );
    let mut unqueried = selected.clone();
    for site in &mut unqueried.cfg.command_binding_sites {
        if let Some(tokens) = &mut site.source_tokens {
            tokens.source_binding = None;
        }
    }
    assert!(
        assignment_facts(&unqueried, &unit, registry)
            .direct_assignments
            .is_empty()
    );
}

#[test]
fn assignment_source_metadata_retains_nested_lexer_overlays() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let config = tcl_lexer::LexerConfig {
        escapes: tcl_dialect::EscapeSyntax::Tcl84,
        ..tcl_lexer::LexerConfig::for_dialect("tcl8.6")
    };
    // The original Tcl84 escape overlay is the measured Tcl8.5 lexer
    // grammar. Its actual command entry stays separate from C8.6 availability.
    let unit = native_source_unit(
        std::sync::Arc::clone(&context),
        r"proc p {} {set escaped \U00000041}; p",
        config,
        tcl_dialect::DialectProfile::find("tcl8.5").unwrap(),
    );
    let function = fixture_id_if_present(&unit.procedures, "::p")
        .and_then(|id| unit.procedures.get(&id))
        .unwrap();
    assert_eq!(function.source_lexer_config(), config.nested().normalized());
    assert_eq!(
        function.source_metadata_input().unwrap().lexer_config(),
        config.nested().normalized()
    );
    let facts = function_facts(
        function,
        &unit.ir_module,
        context.commands(),
        &unit.command_mutations,
        false,
    );
    assert_eq!(facts.direct_assignments.len(), 1);
    let mut stale = function.clone();
    stale.source_config.escapes = tcl_dialect::EscapeSyntax::Tcl86;
    assert!(
        stale
            .invocation_metadata_context(context.commands())
            .is_none()
    );
    let facts = function_facts(
        &stale,
        &unit.ir_module,
        context.commands(),
        &unit.command_mutations,
        false,
    );
    assert!(facts.direct_assignments.is_empty());
}

fn only_decline(facts: &FunctionFacts) -> WasmLeafInvokeDecline {
    let mut declines = facts.leaf_declines.values().copied().collect::<Vec<_>>();
    assert_eq!(declines.len(), 1, "expected exactly one declined statement");
    declines.pop().expect("one decline")
}

/// The `proc` statement inside `unit`, with the tokens the front end kept.
fn proc_statement_tokens(unit: &FunctionUnit) -> Vec<(Span, &tcl_compiler::ir::CommandTokens)> {
    let mut found = Vec::new();
    for block in unit.cfg.reverse_postorder() {
        let Some(cfg_block) = unit.cfg.blocks.get(&block) else {
            continue;
        };
        for statement in &cfg_block.statements {
            if let Statement::Call {
                span,
                command,
                tokens: Some(tokens),
                ..
            } = statement
                && command == "proc"
            {
                found.push((*span, tokens));
            }
        }
    }
    found
}

#[test]
fn host_script_admission_boxes_the_complete_original_byte_image() {
    use std::sync::Arc;
    use tcl_compiler::command_binding::{ExecutedScriptSource, SourceOriginId};
    let original = tcl_lexer::SourceImage::native(b"set raw \xff\0tail".as_slice());
    let origin = Arc::new(SourceOriginId::authored_image(original.clone()));
    let chunk = ExecutedScriptSource::contiguous_image(origin, original.clone(), 0).unwrap();
    let (module, _) = codegen_runtime_chunk(Some(&chunk), 64, false, false);
    assert_eq!(module.data_segments.len(), 1);
    assert_eq!(module.data_segments[0].offset, 64);
    assert_eq!(module.data_segments[0].data, original.bytes());
}

#[test]
fn a_substituted_definition_body_is_left_to_the_runtimes_own_proc() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(
        "proc make {} { set body {return hello} ; proc p {x} $body }\n",
        &registry,
        false,
    );
    let procedure = fixture_id_if_present(&unit.ir_module.procedures, "::p")
        .and_then(|id| unit.ir_module.procedures.get(&id))
        .expect("the inner definition survives as a procedure");
    assert_eq!(
        procedure.body_source.as_deref(),
        Some("${body}"),
        "the fixture must record the written word rather than the compiled \
             body, else this proves nothing",
    );

    let enclosing = fixture_id_if_present(&unit.procedures, "::make")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the enclosing body");
    let definition = proc_statement_tokens(enclosing)
        .into_iter()
        .find(|(span, _)| *span == procedure.span)
        .expect("the inner `proc` statement kept its tokens");
    let facts = function_facts(
        enclosing,
        &unit.ir_module,
        &registry,
        &unit.command_mutations,
        false,
    );
    let plan = facts
        .leaf_invocations
        .get(&span_key(definition.0))
        .expect("the original variable word has a generic argv plan");
    let [_, _, _, body] = plan.root.words.as_slice() else {
        panic!(
            "the original four-word vector survives: {:?}",
            plan.root.words
        );
    };
    assert!(matches!(body, WasmWordPlan::Scalar { name, .. } if name == "body"));
    assert!(!facts.leaf_declines.contains_key(&span_key(definition.0)));
}

#[test]
fn a_written_out_definition_body_keeps_original_generic_argv() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let unit = retained_source_unit(
        std::sync::Arc::clone(&context),
        "proc make {} { proc p {x} {return $x} }; make\n",
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    );
    let procedure = fixture_id_if_present(&unit.ir_module.procedures, "::p")
        .and_then(|id| unit.ir_module.procedures.get(&id))
        .expect("procedure");
    let enclosing = fixture_id_if_present(&unit.procedures, "::make")
        .and_then(|id| unit.procedures.get(&id))
        .expect("the enclosing body");
    let definition = proc_statement_tokens(enclosing)
        .into_iter()
        .find(|(span, _)| *span == procedure.span)
        .expect("the original definition keeps its own span");
    let facts = function_facts(
        enclosing,
        &unit.ir_module,
        registry,
        &unit.command_mutations,
        false,
    );
    let plan = facts
        .leaf_invocations
        .get(&span_key(definition.0))
        .expect("the original literal definition retains its generic argv");
    let [head, name, parameters, body] = plan.root.words.as_slice() else {
        panic!("one complete original vector: {:?}", plan.root.words);
    };
    assert!(matches!(head, WasmWordPlan::Literal { text, .. } if text == "proc"));
    assert!(matches!(name, WasmWordPlan::Literal { text, .. } if text == "p"));
    assert!(matches!(parameters, WasmWordPlan::Literal { text, .. } if text == "x"));
    assert!(matches!(body, WasmWordPlan::Literal { text, .. } if text == "return $x"));
    assert!(!facts.leaf_declines.contains_key(&span_key(definition.0)));
}

#[test]
fn a_top_level_written_out_definition_retains_original_name_operand() {
    let facts = top_level_facts("proc :audit {x} {return $x}\n");
    assert_eq!(facts.leaf_invocations.len(), 1);
    let plan = facts
        .leaf_invocations
        .values()
        .next()
        .expect("one original argv plan");
    let [_, name, _, body] = plan.root.words.as_slice() else {
        panic!("one complete original vector: {:?}", plan.root.words);
    };
    assert!(matches!(name, WasmWordPlan::Literal { text, .. } if text == ":audit"));
    assert!(matches!(body, WasmWordPlan::Literal { text, .. } if text == "return $x"));
    assert!(facts.leaf_declines.is_empty());
}

/// A leaf command whose words all compile selects a plan and records no
/// decline for that statement.
#[test]
fn provable_words_select_a_prebuilt_argv_plan() {
    let facts = top_level_facts("string length $value\n");
    assert_eq!(facts.leaf_invocations.len(), 1);
    assert!(facts.leaf_declines.is_empty(), "{:?}", facts.leaf_declines);
}

/// A word the emitter cannot prove records the precise typed reason, so the
/// remaining source-span evaluation stays observable rather than anonymous.
#[test]
fn unprovable_words_record_their_typed_decline() {
    for (source, expected) in [
        (
            "string length {*}$args\n",
            WasmLeafInvokeDecline::ArgumentExpansion,
        ),
        (
            "string length a\\tb\n",
            WasmLeafInvokeDecline::BackslashSubstitution,
        ),
        (
            "string length $a($i)\n",
            WasmLeafInvokeDecline::DynamicVariableName,
        ),
    ] {
        let facts = top_level_facts(source);
        assert!(
            facts.leaf_invocations.is_empty(),
            "{source} should not select a plan"
        );
        assert_eq!(only_decline(&facts), expected, "{source}");
    }
}

#[test]
fn constructed_procedure_calls_require_the_original_native_trace_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Real source compilation and original entry; no Native trace execution.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let plain = retained_source_unit(
        std::sync::Arc::clone(&context),
        "proc p {x} {return $x}; p VALUE",
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    );
    let current = function_facts(
        &plain.top_level,
        &plain.ir_module,
        context.commands(),
        &plain.command_mutations,
        true,
    );
    assert!(current.leaf_invocations.values().any(|plan| {
        matches!(plan.root.words.first(), Some(WasmWordPlan::Literal { text, .. }) if text == "p")
    }));
    let tokens = plain
        .ir_module
        .top_level
        .statements
        .iter()
        .rev()
        .find_map(tcl_compiler::ir::Statement::tokens)
        .unwrap();
    let target = tokens
        .source_binding
        .as_ref()
        .unwrap()
        .proved_execution_target()
        .unwrap();
    assert!(target.identity.as_ref().unwrap().runtime.is_none());
    assert!(
        !tokens
            .source_binding
            .as_ref()
            .unwrap()
            .unobserved_native_dispatch()
    );
    let traced = retained_source_unit(
        std::sync::Arc::clone(&context),
        "proc cb args {}; proc p {x} {return $x}; proc other args {}; trace add execution other enter cb; p VALUE",
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    );
    assert!(traced.ir_module.traced_commands.contains("other"));
    let refused = function_facts(
        &traced.top_level,
        &traced.ir_module,
        context.commands(),
        &traced.command_mutations,
        true,
    );
    assert!(refused.leaf_invocations.values().any(|plan| {
        matches!(plan.root.words.first(), Some(WasmWordPlan::Literal { text, .. }) if text == "p")
    }));
    let original = traced
        .ir_module
        .top_level
        .statements
        .iter()
        .rev()
        .find_map(tcl_compiler::ir::Statement::tokens)
        .expect("actual final source call");
    assert!(
        !original
            .source_binding
            .as_ref()
            .expect("original source binding")
            .unobserved_native_dispatch()
    );
}
