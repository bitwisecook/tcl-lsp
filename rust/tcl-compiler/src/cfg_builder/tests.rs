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

use super::*;
fn summary_id(module: &Module, label: &str) -> crate::ir::ProcedureId {
    let matching: Vec<_> = module
        .original_source_procedure_bodies()
        .filter(|(_, procedure)| {
            procedure.qualified_name == label
                || procedure.name == label
                || tcl_syntax::naming::key_tail(&procedure.name) == label
        })
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "authored fixture summary {label:?} must identify one body"
    );
    matching[0]
}

use crate::ir::{ForeachIterator, IfClause, Script, SwitchArm, SwitchMode};
use tcl_lexer::Span;

fn native_registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
    )
}

fn build_test_cfg_function(name: &str, script: &Script, inline_loops: bool) -> Function {
    let registry = native_registry();
    CfgBuilder::new(inline_loops, &registry)
        .with_lexer_config(tcl_lexer::LexerConfig::for_profile(registry.profile()))
        .with_faithful_exceptions()
        .with_invocation_namespace(crate::ir::ExecutionNamespace::exact(command_namespace(
            name,
        )))
        .build_function(name, script)
}

/// The registry-derived commands which leave a procedure when they appear
/// as ordinary calls. Keep test data tied to traits rather than duplicating
/// command names alongside the production lookup.
fn proc_exit_host_commands() -> Vec<String> {
    let classes = CfgCommandClasses::for_dialect(Some("tcl9.0"));
    let mut commands: Vec<_> = classes
        .semantics
        .command_names_with_traits(Traits::TERMINATES_BLOCK)
        .filter(|name| {
            classes
                .semantics
                .command(name)
                .is_some_and(|facts| facts.lowering_hook() != Some(LoweringHookId::Return))
        })
        .chain(
            classes
                .semantics
                .command_names_with_traits(Traits::REPLACES_FRAME),
        )
        .map(str::to_owned)
        .collect();
    commands.sort_unstable();
    commands.dedup();
    commands
}

fn host_tokens(command: &str) -> CommandTokens {
    let registry = native_registry();
    let mut lowerer = crate::lowering::Lowerer::new(&registry);
    let module = lowerer.lower(command);
    module.top_level.statements[0]
        .tokens()
        .expect("real source invocation proof")
        .clone()
}

fn host_call(command: String) -> Statement {
    let tokens = host_tokens(&command);
    Statement::Call {
        span: Span::new(0, u32::try_from(command.len()).unwrap()),
        command,
        canonical_command: None,
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(tokens),
        foreach_groups: None,
    }
}

fn host_barrier(command: String) -> Statement {
    let tokens = host_tokens(&command);
    Statement::Barrier {
        span: Span::new(0, u32::try_from(command.len()).unwrap()),
        reason: "opaque host command".into(),
        command,
        canonical_command: None,
        args: Vec::new(),
        tokens: Some(tokens),
    }
}

fn build_analysis_cfg(name: &str, script: &Script) -> Function {
    CfgBuilder::new(true, tcl_registry::default_registry())
        .with_faithful_exceptions()
        .build_function(name, script)
}

fn proc_exit_barrier_switch(mode: SwitchMode) -> Script {
    let commands = proc_exit_host_commands();
    let arms = commands
        .iter()
        .enumerate()
        .map(|(i, command)| SwitchArm {
            pattern: i.to_string(),
            pattern_braced: true,
            pattern_span: Span::new(0, 1),
            // Exercise absolute spellings inside both flattened and opaque
            // switch paths; classification must normalise the `::` prefix.
            body: Some(Script::from_statements(vec![host_barrier(format!(
                "::{command}"
            ))])),
            body_span: Some(Span::new(0, 1)),
            fallthrough: false,
        })
        .collect();
    Script::from_statements(vec![Statement::Switch {
        subject_braced: false,
        raw_arg_braced: Vec::new(),
        raw_arg_quoted: Vec::new(),
        command: "switch".into(),
        span: Span::new(0, 1),
        subject: "$which".into(),
        subject_span: Span::new(0, 1),
        arms,
        default_body: Some(Script::from_statements(vec![host_barrier(format!(
            "::{}",
            commands[0]
        ))])),
        default_span: Some(Span::new(0, 1)),
        mode,
        nocase: false,
        raw_args: Vec::new(),
        patterns_braced: true,
    }])
}

/// Drift guard: the registry-derived classification sets must equal the
/// name lists pinned here, so a trait stamping change is a conscious
/// CFG-shape decision rather than a silent one.
#[test]
fn registry_derived_cfg_classes_match_previous_hardcodes() {
    fn sorted(classes: &CfgCommandClasses, traits: Traits) -> Vec<&str> {
        let mut v: Vec<&str> = classes
            .semantics
            .command_names_with_traits(traits)
            .collect();
        v.sort_unstable();
        v
    }
    let classes = CfgCommandClasses::for_dialect(Some("tcl9.0"));
    // `return` carries TERMINATES_BLOCK but lowers to
    // `Statement::Return`, so the name-keyed terminator set excludes it.
    let mut terminates = sorted(&classes, Traits::TERMINATES_BLOCK);
    terminates.retain(|name| {
        classes
            .semantics
            .command(name)
            .is_some_and(|facts| facts.lowering_hook() != Some(LoweringHookId::Return))
    });
    assert_eq!(terminates, ["error", "exit", "throw"]);
    assert_eq!(
        sorted(&classes, Traits::CATCHABLE_THROW),
        ["error", "throw"]
    );
    assert_eq!(sorted(&classes, Traits::REPLACES_FRAME), ["tailcall"]);
    assert_eq!(sorted(&classes, Traits::BREAKS_LOOP), ["break"]);
    assert_eq!(sorted(&classes, Traits::CONTINUES_LOOP), ["continue"]);

    let tcl84 = CfgCommandClasses::for_dialect(Some("tcl8.4"));
    assert!(
        !tcl84.is_block_terminating_command("throw")
            && !tcl84
                .semantics
                .command("throw")
                .is_some_and(|facts| facts.has_traits(Traits::CATCHABLE_THROW)),
        "Tcl 8.4 must select its visible spec set, not inherit later `throw` traits"
    );
}

#[test]
fn observed_completion_never_manufactures_a_procedure_return() {
    use tcl_registry::completion::CompletionCode;
    use tcl_registry::completion_route::InvocationCompletionRoute as Route;
    let registry = native_registry().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    for (source, expected) in [
        ("return -level 0 VALUE; set reached 1", None),
        ("return -level $level VALUE; set reached 1", None),
        (
            "return -code error -level 0 BOOM; set reached 1",
            Some(Route::Tcl(CompletionCode::Error)),
        ),
        ("exit 0; set reached 1", Some(Route::ProcessExit)),
    ] {
        let module = crate::lowering::lower_to_ir(source, &registry);
        let cfg = build_cfg_with_registry(&module, false, &registry);
        let entry = &cfg.top_level.blocks[&cfg.top_level.entry];
        if let Some(expected) = expected {
            assert!(
                matches!(entry.terminator, Some(Terminator::Complete {route, ..}) if route == expected),
                "{source}: {entry:?}"
            );
        } else {
            assert!(
                !matches!(
                    entry.terminator,
                    Some(Terminator::Complete { .. } | Terminator::Return { .. })
                ),
                "{source}: {entry:?}"
            );
            assert!(entry.statements.iter().any(|statement| matches!(statement,
                    Statement::AssignConst {name, ..} | Statement::AssignValue {name, ..} if name == "reached")), "{source}: {entry:?}");
        }
    }
}

#[test]
fn cfg_and_binding_analysis_retain_the_same_registry_generation() {
    let registry = native_registry();
    let module = crate::lowering::lower_to_ir("set value 1", &registry);
    let bindings = ModuleCommandBindings::analyse(&module, &registry);
    let classes = CfgCommandClasses::from_registry(&registry);

    assert!(Arc::ptr_eq(
        &registry.effective_semantics(),
        &module.resolved_registry().effective_semantics(),
    ));
    // The actual entry roster is narrower than catalogue assistance. Both
    // projections query the same frozen registry generation.
    assert!(bindings.effective_semantics().command("set").is_some());
    assert!(classes.semantics.command("set").is_some());
}

#[test]
fn prepared_cfg_source_metadata_preserves_availability_and_refuses_missing_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let context = Arc::new(generation.with_command_store(Arc::clone(generation.commands())));
    let registry = generation.commands();
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .with_dialect(registry.profile())
        .with_context_registry(Arc::clone(&context));
    let module = lowerer.lower("throw ERROR payload").clone();
    let statement = &module.top_level.statements[0];
    let tokens = statement.tokens().expect("original command words");
    let prepared = prepare_cfg_context_bundle(&module, registry);
    assert!(
        prepared
            .command_classes
            .logical_structured_invocation(tokens)
            .is_some()
    );
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(registry)),
    );
    let original = module
        .source_metadata_input
        .as_ref()
        .expect("retained metadata");
    let input = crate::analyser::ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        older,
        original.lexer_config(),
    );
    let unavailable =
        PreparedCfgContext::from_source_input(prepared.context.clone(), registry, Some(&input));
    assert!(
        unavailable
            .command_classes
            .logical_structured_invocation(tokens)
            .is_none()
    );
    assert!(
        unavailable
            .command_classes
            .completion_route(statement)
            .is_none()
    );
    let foreign = CfgCommandClasses::from_metadata(
        registry,
        Some(tcl_registry::model::ingress::context_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        )),
    );
    assert!(foreign.logical_structured_invocation(tokens).is_none());
    assert!(foreign.resolved_tokens(tokens).is_none());
    let absent = PreparedCfgContext::from_source_input(prepared.context.clone(), registry, None);
    assert!(
        absent
            .command_classes
            .logical_structured_invocation(tokens)
            .is_none()
    );
    assert!(absent.command_classes.resolved_tokens(tokens).is_none());
    assert!(
        absent
            .command_classes
            .exact_completion(statement, &|_| None)
            .is_none()
    );
    let mut missing = module.clone();
    missing.source_metadata_input = None;
    assert!(
        prepare_cfg_context_bundle(&missing, registry)
            .command_classes
            .logical_structured_invocation(tokens)
            .is_none()
    );
    assert!(
        CfgCommandClasses::from_registry(registry)
            .logical_structured_invocation(tokens)
            .is_some(),
        "explicit standalone control"
    );
    let mut incomplete = tokens.clone();
    incomplete.source_binding = None;
    assert!(
        prepared
            .command_classes
            .logical_structured_invocation(&incomplete)
            .is_none()
    );
}

#[test]
fn cfg_condition_possible_writes_keep_original_aliases_and_replacements() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // These are analysis-only <cond> footprints, not native stores/entry.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let registry = context.commands();
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    for (source, expected) in [
        (
            "if {[catch {set {café(open} VALUE} result]} {puts ${café(open}}",
            Some("café(open"),
        ),
        (
            "interp alias {} assign {} set {$literal}; if {[assign VALUE]} {puts ${$literal}}",
            Some("$literal"),
        ),
        (
            "proc catch args {}; if {[catch {set hidden VALUE} result]} {puts $hidden}",
            None,
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let defs = find_call_defs(&unit.top_level.cfg, "<cond>").unwrap_or_default();
        if let Some(expected) = expected {
            assert!(
                defs.iter().any(|name| name == expected),
                "{source}: {defs:?}"
            );
        } else {
            assert!(
                !defs.iter().any(|name| name == "hidden" || name == "result"),
                "known replacement: {defs:?}"
            );
        }
    }
}

#[test]
fn prepared_cfg_refuses_changed_module_profile_and_body_configuration() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let registry = context.commands();
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
        "if {[set output VALUE]} {puts $output}",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    );
    let module = &unit.ir_module;
    assert!(
        prepare_cfg_context_bundle(module, registry)
            .command_classes
            .metadata_context()
            .is_some()
    );
    let mut wrong_profile = module.clone();
    wrong_profile.dialect_profile = Some(tcl_dialect::DialectProfile::find("tcl9.0").unwrap());
    assert!(
        prepare_cfg_context_bundle(&wrong_profile, registry)
            .command_classes
            .metadata_context()
            .is_none()
    );
    let prepared = prepare_cfg_context_bundle(module, registry);
    let bad_config = tcl_lexer::LexerConfig {
        expand_syntax: !config.expand_syntax,
        ..config
    };
    let builder = CfgBuilder::new_with_upvars_and_classes(
        true,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        prepared.context.3.clone(),
        registry,
        prepared.command_classes.clone(),
    )
    .with_lexer_config(bad_config);
    assert!(builder.command_classes.metadata_context().is_none());
    // Complete source metadata is still independently retained by its owner.
    assert_eq!(wrong_profile.source_metadata_input.as_ref(), Some(&input));
}

#[test]
fn original_cfg_metadata_preserves_explicit_standalone_and_missing_purposes() {
    let context = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let registry = context.commands();
    let mut lowerer = crate::lowering::Lowerer::with_config(
        registry,
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    )
    .with_dialect(registry.profile());
    let module = lowerer.lower("set value 1; puts $value").clone();
    assert!(module.source_entry.metadata_context.is_standalone());
    assert!(module.source_metadata_input.is_none());
    let original = prepare_cfg_context_bundle(&module, registry);
    assert!(matches!(
        original.command_classes.metadata,
        CfgMetadataContext::Standalone
    ));
    let explicit = PreparedCfgContext::from_metadata_input(
        original.context.clone(),
        registry,
        crate::registry_invocation::InvocationMetadataInput::Retained(
            &module.source_entry.metadata_context,
        ),
    );
    assert!(matches!(
        explicit.command_classes.metadata,
        CfgMetadataContext::Standalone
    ));
    let missing = PreparedCfgContext::from_source_input(original.context.clone(), registry, None);
    assert!(matches!(
        missing.command_classes.metadata,
        CfgMetadataContext::Unavailable
    ));
    let mut changed = module.clone();
    changed.source_entry.metadata_context = CfgMetadataContext::Unavailable;
    assert!(matches!(
        prepare_cfg_context_bundle(&changed, registry)
            .command_classes
            .metadata,
        CfgMetadataContext::Unavailable
    ));
    let foreign = CommandRegistry::build_default();
    assert!(matches!(
        prepare_cfg_context_bundle(&module, &foreign)
            .command_classes
            .metadata,
        CfgMetadataContext::Unavailable
    ));
}

#[test]
fn prepared_cfg_source_metadata_is_shared_by_module_and_body_builders() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let registry = context.commands();
    let mut lowerer = crate::lowering::Lowerer::with_config(
        registry,
        tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
    )
    .with_dialect(registry.profile())
    .with_context_registry(Arc::clone(&context));
    let module = lowerer.lower("throw ERROR payload; set reached 1").clone();
    let prepared = prepare_cfg_context_bundle(&module, registry);
    let shared = PreparedCfgContext::from_source_input(
        prepared.context.clone(),
        registry,
        module.source_metadata_input.as_ref(),
    );
    let CfgMetadataContext::SuppliedSource(retained) = &shared.command_classes.metadata else {
        panic!("the actual body request retains its original context");
    };
    assert!(Arc::ptr_eq(&retained.context_registry(), &context));
    let cfg = build_cfg_with_registry(&module, false, registry);
    let body = build_cfg_function_with_prepared_context(
        "::top",
        &module.top_level,
        true,
        registry,
        module.plain_command_dispatch,
        &shared,
        module.lexer_config,
    );
    let expected = Some(Terminator::Complete {
        route: tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
            tcl_registry::CompletionCode::Error,
        ),
        span: Some(module.top_level.statements[0].span()),
    });
    assert_eq!(
        cfg.top_level.blocks[&cfg.top_level.entry].terminator,
        expected
    );
    assert_eq!(body.blocks[&body.entry].terminator, expected);
}

#[test]
fn whole_module_cfg_uses_the_exact_custom_registry_traits() {
    let mut registry = native_registry();
    let mut custom = registry.get("puts").expect("puts spec").clone();
    custom.traits |= Traits::TERMINATES_BLOCK;
    custom.completion = Some(tcl_registry::CompletionDescriptor::exact(&[
        tcl_registry::CompletionCode::Error,
    ]));
    registry.insert(custom);

    let module = crate::lowering::lower_to_ir("puts before\nset reached 1", &registry);
    let cfg = build_cfg_with_registry(&module, false, &registry);
    let entry = &cfg.top_level.blocks[&cfg.top_level.entry];
    assert!(
        matches!(
            entry.terminator,
            Some(Terminator::Complete {
                route: tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::CompletionCode::Error
                ),
                ..
            })
        ),
        "the custom registry's completion must shape the whole-module CFG: {entry:?}"
    );
    assert!(
        cfg.top_level.blocks.values().any(|block| {
            block.name.starts_with("unreachable")
                && block.statements.iter().any(|statement| {
                    matches!(statement, Statement::AssignConst { name, .. } if name == "reached")
                        || matches!(statement, Statement::Call { command, args, .. }
                                if command == "set" && args == &["reached", "1"])
                })
        }),
        "the statement after the custom terminator must be unreachable"
    );
}

#[test]
fn condition_var_writes_use_the_exact_custom_registry_roles() {
    let mut registry = native_registry();
    let mut custom = registry.get("list").expect("list spec").clone();
    custom.arg_roles = &[(0, tcl_registry::ArgRole::VarWrite)];
    custom.arg_role_resolver = None;
    registry.insert(custom);

    let module = crate::lowering::lower_to_ir("if {[list slot]} {puts yes}", &registry);
    let cfg = build_cfg_with_registry(&module, false, &registry);
    let condition_defs = cfg
        .top_level
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find_map(|statement| match statement {
            Statement::Call { command, defs, .. } if command == "<cond>" => Some(defs),
            _ => None,
        })
        .expect("condition command substitutions produce a synthetic CFG call");
    assert!(
        condition_defs.iter().any(|name| name == "slot"),
        "custom VarWrite role was replaced by a default Tcl registry: {condition_defs:?}"
    );
}

/// The classification helpers keep each site's own name
/// normalisation: the terminator / throw / tailcall checks trim
/// leading `:` runs (so canonical `::error` classifies), while
/// [`CfgBuilder::lower_loop_jump`] matches the raw word (so
/// `::break` stays a plain call there).
#[test]
fn cfg_class_helpers_keep_site_normalisation() {
    let classes = CfgCommandClasses::for_dialect(Some("tcl9.0"));
    assert!(classes.is_block_terminating_command("error"));
    assert!(classes.is_block_terminating_command("::throw"));
    assert!(classes.is_block_terminating_command("exit"));
    assert!(!classes.is_block_terminating_command("return"));
    assert!(!classes.is_block_terminating_command("break"));
    assert!(
        classes
            .semantics
            .command("error")
            .is_some_and(|facts| facts.has_traits(Traits::CATCHABLE_THROW))
    );
    assert!(
        !classes
            .semantics
            .command("exit")
            .is_some_and(|facts| facts.has_traits(Traits::CATCHABLE_THROW))
    );
    assert!(classes.is_tailcall_command("::tailcall"));
    assert!(!classes.is_tailcall_command("error"));
    assert!(classes.is_loop_break_command("break"));
    assert!(!classes.is_loop_break_command("::break"));
    assert!(classes.is_loop_continue_command("continue"));
    assert!(!classes.is_loop_continue_command("break"));
}

#[test]
fn opaque_host_barriers_match_calls_for_every_registry_proc_exit_command() {
    // This is deliberately registry-driven: today the traits yield
    // `error`, `exit`, `throw`, and `tailcall`, but a trait change must
    // extend this test automatically. Test both source spellings the CFG
    // accepts, including the global-qualified form.
    for command in proc_exit_host_commands() {
        for spelling in [command.clone(), format!("::{command}")] {
            for (kind, statement) in [
                ("Call", host_call(spelling.clone())),
                ("Barrier", host_barrier(spelling.clone())),
            ] {
                assert_eq!(
                    flow_facts_stmt(&statement).1,
                    Completion::ProcExit,
                    "{kind} {spelling} must leave the procedure",
                );

                let script = Script::from_statements(vec![
                    statement,
                    Statement::AssignConst {
                        span: Span::new(2, 3),
                        name: "dead".into(),
                        name_braced: false,
                        value: "1".into(),
                        value_span: None,
                    },
                ]);
                let func = build_analysis_cfg("::test", &script);
                let entry = &func.blocks[&func.entry];
                assert_eq!(
                    entry.statements.len(),
                    1,
                    "{kind} {spelling} must keep following statements unreachable",
                );
                assert!(
                    matches!(entry.terminator, Some(Terminator::Complete { .. })),
                    "{kind} {spelling} must terminate its CFG block, got {:?}",
                    entry.terminator,
                );
            }
        }
    }
}

#[test]
fn proc_exit_host_barriers_terminate_direct_and_opaque_switch_forms() {
    // Exact switches lower each arm directly; glob switches remain an
    // opaque Statement::Switch. In both representations, barriers whose
    // host command is a registry-described proc exit must retain the same
    // completion fact as a Call.
    let commands = proc_exit_host_commands();
    let direct = build_analysis_cfg("::direct", &proc_exit_barrier_switch(SwitchMode::Exact));
    assert!(matches!(
        direct.blocks[&direct.entry].terminator,
        Some(Terminator::Branch { .. })
    ));
    let direct_barriers: Vec<_> = direct
        .blocks
        .values()
        .filter(|block| {
            block
                .statements
                .iter()
                .any(|stmt| matches!(stmt, Statement::Barrier { .. }))
        })
        .collect();
    assert_eq!(direct_barriers.len(), commands.len() + 1);
    assert!(
        direct_barriers
            .iter()
            .all(|block| { matches!(block.terminator, Some(Terminator::Complete { .. })) })
    );

    let opaque = build_analysis_cfg("::opaque", &proc_exit_barrier_switch(SwitchMode::Glob));
    let entry = &opaque.blocks[&opaque.entry];
    assert!(matches!(
        entry.statements.as_slice(),
        [Statement::Switch { .. }]
    ));
    assert!(
        matches!(entry.terminator, Some(Terminator::Complete { .. })),
        "an all-proc-exit opaque switch must not fall through, got {:?}",
        entry.terminator,
    );
}

#[test]
fn structured_cfg_writes_keep_literal_name_data_and_dynamic_unknowns() {
    for (name, root) in [
        ("", ""),
        ("$counter", "$counter"),
        ("${braced}", "${braced}"),
        ("$array(key)", "$array"),
    ] {
        let statement = Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: true,
            value: "1".into(),
            value_span: None,
        };
        assert_eq!(
            flow_facts_stmt(&statement).0,
            BTreeSet::from([root.to_owned()])
        );
        let mutation = Statement::Incr {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: true,
            amount: Some("1".into()),
            amount_braced: true,
            safe_on_uninit: false,
        };
        let registry = CommandRegistry::build_default();
        assert_eq!(
            CfgBuilder::new(true, &registry).init_written_names(&mutation),
            Some(vec![root.to_owned()])
        );
    }
    let dynamic = Statement::Incr {
        span: Span::new(0, 0),
        name: "$counter".into(),
        name_braced: false,
        amount: Some("1".into()),
        amount_braced: true,
        safe_on_uninit: false,
    };
    assert!(flow_facts_stmt(&dynamic).0.is_empty());
    let registry = CommandRegistry::build_default();
    assert_eq!(
        CfgBuilder::new(true, &registry).init_written_names(&dynamic),
        None
    );
}

#[test]
fn for_initialisation_does_not_publish_array_elements_as_scalar_constants() {
    let registry = native_registry();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let builder = CfgBuilder::new(true, &registry).with_lexer_config(config);
    for (name, condition, expected) in [
        ("a", "$a == 1", true),
        ("$a", "${$a} == 1", true),
        ("", "${} == 1", true),
        ("a(key)", "$a == 1", false),
        ("$a(key)", "${$a} == 1", false),
    ] {
        let source = format!("for {{set {{{name}}} 1}} {{{condition}}} {{}} {{}}");
        let module = crate::lowering::lower_procedure_target_module_for_bytecode_with_options(
            tcl_runtime_api::ProcedureCompileTarget {
                source: &source,
                namespace: "",
                parameters: &[],
            },
            &registry,
            config,
            Some(profile),
            false,
            None,
        );
        let statement = &module.top_level.statements[0];
        let Statement::For { raw_tokens, .. } = statement else {
            panic!("original bytecode for grammar: {statement:?}");
        };
        assert!(raw_tokens.as_deref().is_some());
        assert_eq!(
            builder.for_runs_at_least_once(statement),
            expected,
            "{source}"
        );
    }
}

#[test]
fn empty_script_produces_entry_exit() {
    let func = build_test_cfg_function("::test", &Script::new(), true);
    assert_eq!(func.name, "::test");
    assert!(func.blocks.contains_key(&func.entry));
    // entry → exit
    assert!(func.blocks.len() >= 2);
}

#[test]
fn linear_script() {
    let script = Script::from_statements(vec![
        Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        Statement::AssignConst {
            span: Span::new(8, 15),
            name: "y".into(),
            name_braced: false,
            value: "2".into(),
            value_span: None,
        },
    ]);
    let func = build_test_cfg_function("::test", &script, true);
    // Entry block should have both statements.
    let entry = &func.blocks[&func.entry];
    assert_eq!(entry.statements.len(), 2);
}

#[test]
fn return_terminates() {
    let script = Script::from_statements(vec![
        Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        Statement::Return {
            expr_base: None,
            tokens: None,
            span: Span::new(8, 16),
            value: Some("$x".into()),
            value_word: None,
            expr: None,
            command_binding: None,
            braced: false,
        },
        Statement::AssignConst {
            span: Span::new(17, 24),
            name: "y".into(),
            name_braced: false,
            value: "2".into(), // dead code
            value_span: None,
        },
    ]);
    let func = build_test_cfg_function("::test", &script, true);
    let entry = &func.blocks[&func.entry];
    // Only one statement before the return terminator.
    assert_eq!(entry.statements.len(), 1);
    assert!(matches!(entry.terminator, Some(Terminator::Return { .. })));
}

#[test]
fn if_creates_branches() {
    let script = Script::from_statements(vec![Statement::If {
        span: Span::new(0, 30),
        clauses: vec![IfClause {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            condition_span: Span::new(3, 5),
            body: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(7, 14),
                name: "y".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            }]),
            body_span: Span::new(6, 15),
            condition_base: None,
        }],
        else_body: None,
        else_span: None,
    }]);
    let func = build_test_cfg_function("::test", &script, true);
    // Should have at least: entry, if_then, if_next, if_end, exit
    assert!(func.blocks.len() >= 4);
    // Entry block should have a Branch terminator.
    let entry = &func.blocks[&func.entry];
    assert!(
        matches!(entry.terminator, Some(Terminator::Branch { .. })),
        "entry should branch; got {:?}",
        entry.terminator
    );
}

#[test]
fn build_cfg_module() {
    let module = Module::default();
    let cfg = build_cfg(&module, false);
    assert_eq!(cfg.top_level.name, "::top");
    assert!(cfg.procedures.is_empty());
}

#[test]
fn prepared_context_cfg_matches_compatibility_entry_point() {
    let registry = native_registry();
    let module = crate::lowering::lower_to_ir(
        "proc writer {} { global answer; set answer 42 }\n\
             interp alias {} write-answer {} writer\n\
             write-answer\n",
        &registry,
    );
    let context = prepare_cfg_context_bundle(&module, &registry);

    assert_eq!(
        build_cfg_with_registry_and_context(
            &module,
            false,
            &registry,
            &context,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
        ),
        build_cfg_with_registry(&module, false, &registry),
    );
}

#[test]
fn drop_word_keeps_structured_words_aligned() {
    let tokens = CommandTokens::from_lossy_parts(
        vec![Span::new(0, 4), Span::new(5, 8), Span::new(9, 13)],
        vec!["dict".into(), "for".into(), "body".into()],
        vec![TokenType::Esc, TokenType::Esc, TokenType::Str],
        vec![true, true, true],
        Vec::new(),
        None,
    );
    let dropped = drop_word(&tokens, 1);
    assert_eq!(dropped.argv_texts, vec!["dict", "body"]);
    assert!(dropped.words_align_with_argv_text());
    assert_eq!(dropped.words()[1].legacy_text(), "body");
}

/// A `catch` whose body is one straight-line block is lowered into real
/// blocks, so the ordinary emitters compile it: its variables reach the
/// LVT and the optimiser can see inside (#2207).
#[test]
fn straight_line_catch_lowers_to_real_blocks() {
    let func = build_test_cfg_function("::test", &straight_line_catch_script(), true);
    let body_id = func
        .blocks
        .keys()
        .find(|id| func.block_name(**id).starts_with("catch_body_"))
        .copied()
        .expect("expected a catch_body block");
    // The body's own statement is in the body block, not summarised onto
    // an opaque call.
    assert!(
        func.blocks[&body_id]
            .statements
            .iter()
            .any(|s| matches!(s, Statement::AssignConst { name, .. } if name == "inner")),
        "the body's write belongs to the body block",
    );
    // The result variable is still defined for SSA, on the block both
    // paths reach.
    assert!(
            func.blocks.iter().any(|(id, blk)| {
                func.block_name(*id).starts_with("catch_end_")
                    && blk.statements.iter().any(|s| {
                        matches!(
                            s,
                            Statement::Call { tokens: Some(tokens), defs, .. }
                                if tokens.synthetic == Some(crate::ir::SyntheticMarker::CapturedCatchOutputs)
                                    && defs.iter().any(|d| d == "result")
                        )
                    })
            }),
            "result var must be defined at the merge",
        );
}

/// A body that terminates its block — here an `error` — keeps the opaque
/// form: the split-off part would fall outside the exception range.
#[test]
fn catch_with_terminating_body_stays_opaque() {
    let script = Script::from_statements(vec![Statement::Catch {
        span: Span::new(0, 30),
        body: Script::from_statements(vec![Statement::Call {
            span: Span::new(7, 14),
            command: "error".into(),
            canonical_command: None,
            args: vec!["boom".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }]),
        body_span: Span::new(6, 15),
        result_var: Some("result".into()),
        options_var: None,
        raw_args: vec!["{error boom}".into(), "result".into()],
        tokens: None,
    }]);
    let registry = native_registry();
    let func = CfgBuilder::new(true, &registry)
        .with_lexer_config(tcl_lexer::LexerConfig::for_profile(registry.profile()))
        .build_function("::test", &script);
    let entry = &func.blocks[&func.entry];
    assert!(
        entry.statements.iter().any(|s| matches!(
            s,
            Statement::Call { command, defs, .. } if command == "catch" && !defs.is_empty()
        )),
        "a terminating body keeps the opaque call with summarised defs",
    );
    assert!(
        !func
            .blocks
            .keys()
            .any(|id| func.block_name(*id).starts_with("catch_body_")),
        "and is not lowered into blocks",
    );
}

fn straight_line_catch_script() -> Script {
    Script::from_statements(vec![Statement::Catch {
        span: Span::new(0, 30),
        body: Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(7, 14),
            name: "inner".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }]),
        body_span: Span::new(6, 15),
        result_var: Some("result".into()),
        options_var: None,
        raw_args: vec!["{set inner 1}".into(), "result".into()],
        tokens: None,
    }])
}

#[test]
fn foreach_synthetic_call_records_iterator_group_sizes() {
    // ``foreach {a b} L1 c L2 { … }`` has two iterator groups
    // — the first binds two vars, the second binds one.  The
    // synthesised header `Statement::Call` must record the
    // group sizes via ``foreach_groups`` so codegen can
    // reconstruct the original pairing.
    let body = Script::from_statements(vec![Statement::AssignConst {
        span: Span::new(0, 0),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    }]);
    let script = Script::from_statements(vec![Statement::Foreach {
        span: Span::new(0, 0),
        iterators: vec![
            ForeachIterator {
                vars: vec!["a".into(), "b".into()],
                list_arg: "L1".into(),
                list_braced: false,
            },
            ForeachIterator {
                vars: vec!["c".into()],
                list_arg: "L2".into(),
                list_braced: false,
            },
        ],
        body,
        body_span: Span::new(0, 0),
        is_lmap: false,
        raw_args: vec![],
        is_dict_iteration: false,
        is_array_iteration: false,
        raw_tokens: None,
    }]);
    let func = build_test_cfg_function("::test", &script, true);
    let mut found_groups: Option<Vec<usize>> = None;
    for block in func.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call {
                command,
                foreach_groups,
                ..
            } = stmt
                && command == "foreach"
            {
                found_groups = foreach_groups.clone();
            }
        }
    }
    assert_eq!(found_groups, Some(vec![2, 1]));
}

#[test]
fn dedup_preserves_order() {
    let mut v = vec!["a".into(), "b".into(), "a".into(), "c".into(), "b".into()];
    dedup_preserve_order(&mut v);
    assert_eq!(v, vec!["a", "b", "c"]);
}

// Uplevel-literal-set wiring tests.
//
// Each test drives the full pipeline:
// `lower_to_ir` → `build_cfg` (which calls `prepare_cfg_context`).
// The assertions inspect the resulting CFG to confirm that calls
// to upvar-using procs carry the expected caller-side defs.

fn lower_module(src: &str) -> Module {
    crate::lowering::lower_to_ir(src, &native_registry())
}

// --- escaping_loop_jumps: try propagation and
// dead-code early-stop
//
// These exercise the helper directly: their end-to-end W210 effect is
// masked by a separate, pre-existing `while 1` exit-reachability behaviour
// (Rust treats the post-loop block of an infinite loop as reachable), so we
// assert the `(can_break, can_continue)` result the wiring keys off.

#[test]
fn escaping_loop_jumps_follows_try_but_not_catch() {
    // `try { break }` (no matching handler) propagates the break to the
    // enclosing loop; `catch { break }` absorbs it (yields code 3). Confirmed
    // against tclsh 9.0.3.
    assert_eq!(
        escaping_loop_jumps(&lower_module("try { break }").top_level, 0),
        (true, false),
        "try propagates break",
    );
    assert_eq!(
        escaping_loop_jumps(&lower_module("try { continue } finally { }").top_level, 0),
        (false, true),
        "try (with finally) propagates continue",
    );
    assert_eq!(
        escaping_loop_jumps(
            &lower_module("try { puts x } on error {} { break }").top_level,
            0
        ),
        (true, false),
        "a break in a try handler body propagates",
    );
    assert_eq!(
        escaping_loop_jumps(&lower_module("catch { break }").top_level, 0),
        (false, false),
        "catch absorbs break (does not propagate)",
    );
}

#[test]
fn escaping_loop_jumps_stops_after_non_completing_stmt() {
    // A `break`/`continue` after a statement that cannot complete normally
    // is dead code and must not be collected as an escaping jump (else it
    // forges a spurious loop-exit edge).
    assert_eq!(
        escaping_loop_jumps(&lower_module("error bad\nbreak").top_level, 0),
        (false, false),
        "dead break after error is not collected",
    );
    assert_eq!(
        escaping_loop_jumps(&lower_module("return\ncontinue").top_level, 0),
        (false, false),
        "dead continue after return is not collected",
    );
    // A live jump before dead code IS collected (and stops the scan there).
    assert_eq!(
        escaping_loop_jumps(&lower_module("break\nset x 1").top_level, 0),
        (true, false),
        "live break is collected",
    );
}

/// `escaping_loop_jumps` and the
/// mutually-recursive `switch_escaping_jumps` recurse once per nested
/// `if`/`switch`/`Block`/`UpFrame`/`try` body, with no depth cap of
/// their own before this fix. Transitively bounded to
/// `MAX_LOWER_DEPTH` (256) by the lowering pass today, so this is
/// defence-in-depth / consistency with every other full-tree walker in
/// this crate, not a currently-reproducible crash. 1000 levels of
/// source nesting is comfortably past this new cap; the assertion is
/// that `escaping_loop_jumps` returns at all, not what it returns.
/// Spawns its own big-stack thread since the lexer/CST/segmenter
/// stages upstream of the lowering cap still walk the full
/// un-truncated source nesting before that cap trims it — same
/// rationale as
/// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
fn deeply_nested_if_survives_escaping_loop_jumps() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = String::new();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("break\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let _ = escaping_loop_jumps(&lower_module(&src).top_level, 0);
        })
        .unwrap()
        .join()
        .unwrap();
}

fn find_call_defs<'a>(func: &'a Function, command: &str) -> Option<&'a [String]> {
    for block in func.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call {
                command: c, defs, ..
            } = stmt
                && c == command
            {
                return Some(defs);
            }
        }
    }
    None
}

#[test]
fn rooted_regexp_value_form_does_not_widen_runtime_selected_method_frame() {
    let registry = native_registry();
    let module = crate::lowering::lower_to_ir(
        "oo::class create C {\n\
                 method value {re s} {::regexp $re $s}\n\
                 method capture {re s target} {::regexp $re $s $target}\n\
             }",
        &registry,
    );
    let context = prepare_cfg_context_with_registry(&module, &registry);
    let build = |name: &str| {
        let method = &module
            .methods
            .get(&fixture_id(&module.methods, name))
            .expect("authored fixture body");
        build_cfg_method_function_with_upvars(
            name,
            &method.body,
            method.execution_namespace.clone(),
            true,
            &registry,
            module.plain_command_dispatch,
            context.clone(),
            false,
        )
    };
    let has_opaque_variable_barrier = |function: &Function| {
        function.blocks.values().any(|block| {
            block.statements.iter().any(|statement| {
                matches!(
                    statement,
                    Statement::Barrier { reason, .. }
                        if reason.contains("writes a source-opaque variable name")
                )
            })
        })
    };

    let value = build("::C::value");
    assert!(
        !has_opaque_variable_barrier(&value),
        "an absolute two-operand regexp has no capture variable"
    );

    let capture = build("::C::capture");
    assert!(
        has_opaque_variable_barrier(&capture),
        "a dynamic capture name must still widen the variable frame"
    );
}

#[test]
fn detect_upvar_procs_retains_issued_body_effects() {
    let module = lower_module(
        "namespace eval ::ns {}\nproc ::ns::p {} { upvar 1 caller_x x }\nproc ::ns::p2 {} {}",
    );
    let upvar_procs = detect_upvar_procs(&module);
    assert!(!upvar_procs[&summary_id(&module, "::ns::p")].is_empty());
    assert!(!upvar_procs[&summary_id(&module, "p")].is_empty());
    // The empty body has an independently retained empty effect row.
    assert!(upvar_procs[&summary_id(&module, "::ns::p2")].is_empty());
    assert!(upvar_procs[&summary_id(&module, "p2")].is_empty());
}

/// #1797. tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 all print `2 0`
/// for `list [caller] [caller_old]`, with
/// `proc caller {} {p; return $x}` and
/// `proc caller_old {} {old; info exists x}`: each call reaches its own
/// generation, and only the redefined one writes the caller's `x`.
#[test]
fn each_call_joins_only_its_own_procedure_generation() {
    let module = lower_module(
        "proc p {} {}\n\
             rename p old\n\
             proc p {} {upvar 1 x y; set y 2}\n\
             proc caller {} {p}\n\
             proc caller_old {} {old}",
    );
    let summaries = detect_upvar_procs(&module);
    let generation = |needle: &str| {
        let start = u32::try_from(module.source.try_text().unwrap().find(needle).unwrap())
            .expect("fixture source offset fits u32");
        let (id, _) = module
            .procedures
            .iter()
            .find(|(_, procedure)| procedure.span.start() == start)
            .expect("authored generation");
        *id
    };
    let original = generation("proc p {} {}");
    let redefined = generation("proc p {} {upvar");
    assert!(summaries[&original].is_empty());
    assert_eq!(
        summaries[&redefined]
            .literal_targets
            .get("y")
            .map(String::as_str),
        Some("x")
    );
    assert!(!summaries[&redefined].caller_frame_barrier().writes);

    let cfg = build_cfg(&module, false);
    let function = |name: &str| {
        cfg.procedures
            .get(&fixture_id(&cfg.procedures, name))
            .expect("authored fixture body")
    };
    let caller = function("::caller");
    assert!(!caller.caller_frame_barrier.writes);
    assert!(caller.alias_observed_vars.contains("x"));
    let caller_old = function("::caller_old");
    assert!(!caller_old.caller_frame_barrier.writes);
    assert!(!caller_old.alias_observed_vars.contains("x"));
}

/// tclsh 8.4.20 through 9.1.0 all return `2` from `caller` once the
/// computed `namespace eval` has defined `::n::writer`. A computed script
/// may rebind `proc` itself, so lowering publishes no `proc` written after
/// it; `caller` is written first so that its own body is retained.
#[test]
fn recovered_procedure_without_a_published_summary_is_opaque() {
    let module = lower_module(
        "proc caller {} {::n::writer}\n\
             namespace eval ::n [list proc writer {} {upvar 1 x y; ::set y 2}]",
    );
    assert!(fixture_id_if_present(&module.procedures, "::n::writer").is_none());
    let cfg = build_cfg(&module, false);
    assert!(
        cfg.procedures
            .get(&fixture_id(&cfg.procedures, "::caller"))
            .expect("authored fixture body")
            .caller_frame_barrier
            .writes
    );
}

// A wrapper proc
// that reaches an already-known upvar proc through a *plain* call
// (`real_worker $fvar $nvar $script`, not `uplevel`) does NOT itself
// become an upvar-write target for its own caller — tclsh9.0/8.6-
// verified (`can't read "myf": no such variable"` when the caller reads
// the variable in a statement separate from the call, i.e. genuinely
// outside any `uplevel`'d script argument). A plain call only shares
// *values*, not stack frames: `real_worker`'s own `upvar 1` reaches the
// wrapper's frame, not the wrapper's caller's frame; treating such a
// pass-through as transitive is wrong, as reading the variable outside
// any uplevel'd script shows. The tcllib idiom
// (`page::util::flow`) reaches its own worker via `uplevel 1 [list
// ... ]`, not a plain call — that genuinely propagates one frame further,
// confirmed against tclsh9.0 — but modelling it soundly (composing the
// wrapper's own uplevel level with the callee's upvar level) is not done
// here.

#[test]
fn detect_upvar_procs_does_not_propagate_through_a_plain_call_wrapper() {
    // TN — `wrapper` has no `upvar` of its own, and its call to the
    // known upvar proc `real_worker` is a plain call (not `uplevel`),
    // so it must NOT be registered as a transitive upvar proc.
    let module = lower_module(
        "proc real_worker {fvar nvar script} {\n\
             upvar 1 $fvar f\n\
             upvar 1 $nvar n\n\
             set f 1\n\
             set n 2\n\
             uplevel 1 $script\n\
             }\n\
             proc wrapper {fvar nvar script} {\n\
             real_worker $fvar $nvar $script\n\
             }",
    );
    let upvar_procs = detect_upvar_procs(&module);
    assert!(
        upvar_procs[&summary_id(&module, "wrapper")].is_empty(),
        "a plain-call wrapper must not be treated as a transitive upvar proc",
    );
}

#[test]
fn detect_upvar_procs_does_not_propagate_when_args_are_not_passed_through() {
    // TN — `unrelated_wrapper` calls the known upvar proc `real_worker`,
    // but with literal args ("x"/"y"), not its own parameters passed
    // through unchanged — real tclsh9.0/8.6 confirms this genuinely
    // errors ("can't read \"myf\": no such variable"), so it must NOT
    // be registered.
    let module = lower_module(
        "proc real_worker {fvar nvar script} {\n\
             upvar 1 $fvar f\n\
             upvar 1 $nvar n\n\
             set f 1\n\
             set n 2\n\
             uplevel 1 $script\n\
             }\n\
             proc unrelated_wrapper {a b c} {\n\
             real_worker x y $c\n\
             }",
    );
    let upvar_procs = detect_upvar_procs(&module);
    assert!(
        upvar_procs[&summary_id(&module, "unrelated_wrapper")].is_empty(),
        "unrelated_wrapper passes literal args, not its own params — must not propagate",
    );
}

/// The *other* half of the rule
/// the two tests above pin. A level-**1** effect stops at the wrapper's
/// own frame, but a level that lands past the callee's caller reaches
/// this proc's caller through an ordinary call, so the wrapper really
/// does become a caller-frame writer.
///
/// Byte-identical on tclsh 9.0.4 and 8.6.16:
///
/// ```tcl
/// proc setUp2 {var} { uplevel 2 [list set $var 99] }
/// proc middle {}    { setUp2 answer }
/// proc outer  {}    { middle ; return $answer }
/// puts [outer]        ;# -> 99
/// ```
#[test]
fn detect_upvar_procs_propagates_a_beyond_caller_effect_one_plain_hop() {
    let module = lower_module(
        "proc setUp2 {var} {\n             uplevel 2 [list set $var 99]\n             }\n             proc middle {} {\n             setUp2 answer\n             }",
    );
    let upvar_procs = detect_upvar_procs(&module);
    let middle = upvar_procs
        .get(&summary_id(&module, "middle"))
        .expect("a plain call to a beyond-caller writer registers the wrapper");
    assert!(
        middle.caller_frame_opaque_writes,
        "setUp2's `uplevel 2` lands in middle's caller: {middle:?}"
    );
    assert!(
        middle.caller_frame_opaque_reads,
        "the same script may read there too: {middle:?}"
    );
}

/// On tclsh 9.0.4:
///
/// ```tcl
/// proc far {} {uplevel 2 {set x 2}}
/// interp alias {} via {} far
/// proc middle {} {via}
/// proc outer {} {set x 1; middle; return $x}
/// puts [outer] ;# -> 2
/// ```
#[test]
fn alias_terminal_proc_propagates_a_beyond_caller_effect() {
    let module = lower_module(
        "proc far {} {uplevel 2 {set x 2}}\n\
             interp alias {} via {} far\n\
             proc middle {} {via}",
    );
    let upvar_procs = detect_upvar_procs(&module);
    let middle = upvar_procs
        .get(&summary_id(&module, "::middle"))
        .expect("the alias resolves to far's beyond-caller effect");
    assert!(middle.caller_frame_opaque_writes, "{middle:?}");
    assert!(middle.caller_frame_opaque_reads, "{middle:?}");
}

/// TN — `uplevel #0` (global) and `uplevel 0` (the callee's own frame)
/// reach no caller at all, so neither is a beyond-caller effect and
/// neither makes a plain-call wrapper one.
#[test]
fn detect_upvar_procs_does_not_propagate_a_global_or_own_frame_level() {
    for level in ["#0", "0"] {
        let module = lower_module(&format!(
            "proc setSomewhere {{var}} {{\n                 uplevel {level} [list set $var 99]\n                 }}\n                 proc wrapper {{}} {{\n                 setSomewhere answer\n                 }}"
        ));
        let upvar_procs = detect_upvar_procs(&module);
        assert!(
            upvar_procs[&summary_id(&module, "wrapper")].is_empty(),
            "`uplevel {level}` never touches a caller frame: {upvar_procs:?}"
        );
    }
}

/// The published summaries carry no composition bookkeeping — they are
/// folded into every procedure's `function_lattice` memo key, so only
/// what a call site actually reads may travel in them.
#[test]
fn detect_upvar_procs_publishes_no_plain_call_bookkeeping() {
    let module = lower_module(
        "proc setUp2 {var} {\n             uplevel 2 [list set $var 99]\n             }\n             proc middle {} {\n             setUp2 answer\n             }",
    );
    for (name, info) in detect_upvar_procs(&module) {
        assert!(
            info.plain_calls.is_empty(),
            "{name:?} published composition input: {info:?}"
        );
    }
}

#[test]
fn prepare_cfg_context_registers_params_for_all_procs() {
    let module = lower_module("proc ::ns::p {a b} { upvar 1 $a x }\nproc q {c} {}");
    let (_upvar_procs, proc_params, _global_write_procs, _command_bindings, _owners) =
        prepare_cfg_context(&module);
    assert_eq!(
        proc_params.get(&summary_id(&module, "::ns::p")),
        Some(&vec!["a".to_string(), "b".to_string()]),
    );
    assert_eq!(
        proc_params.get(&summary_id(&module, "::ns::p")),
        Some(&vec!["a".to_string(), "b".to_string()]),
    );
    // q has no upvar but its params should still be in proc_params.
    assert_eq!(
        proc_params.get(&summary_id(&module, "::q")),
        Some(&vec!["c".to_string()])
    );
}

#[test]
fn literal_upvar_proc_call_augments_defs() {
    // Caller invokes a proc whose body has `upvar 1 caller_x x`.
    // The call should land with `caller_x` in its defs.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             setter",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x in defs, got {defs:?}",
    );
}

#[test]
fn alias_to_upvar_uses_its_prepended_frame_level() {
    // Tcl 9.0.4: `bind` executes `upvar 1 target local`; assigning
    // `local` in `setter` therefore assigns `target` in its caller.
    // The frame descriptor and argv are both supplied by the closed
    // command binding, not by the source spelling `bind`.
    let module = lower_module(
        "interp alias {} bind {} upvar 1\n\
             proc setter {} { bind target local; set local 99 }\n\
             setter",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.contains(&"target".to_owned()),
        "the aliased upvar target must invalidate its caller: {defs:?}"
    );
}

#[test]
fn param_upvar_proc_call_resolves_call_site_arg() {
    // `proc setter {name} { upvar 1 $name x }` aliased to whatever
    // the caller passes for `name`.  Call `setter my_var` should
    // augment the call with `my_var` in defs.
    let module = lower_module(
        "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             setter my_var",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.contains(&"my_var".to_string()),
        "expected my_var in defs, got {defs:?}",
    );
}

#[test]
fn early_and_late_aliases_to_upvar_proc_prepend_the_caller_target() {
    for source in [
        "interp alias {} assign {} setter target\n\
             proc setter {name} { upvar 1 $name x; set x 1 }\n\
             proc outer {} { assign }",
        "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             proc outer {} { assign }\n\
             interp alias {} assign {} setter target",
    ] {
        let module = lower_module(source);
        let cfg = build_cfg(&module, false);
        let outer = fixture_id_if_present(&cfg.procedures, "::outer")
            .and_then(|id| cfg.procedures.get(&id))
            .expect("outer proc CFG should exist");
        let defs = find_call_defs(outer, "assign").expect("alias call should be in outer CFG");
        assert!(
            defs.contains(&"target".to_owned()),
            "alias definition order must not hide its prepended upvar target: {defs:?}",
        );
    }
}

#[test]
fn alias_chain_preserves_prefix_order_for_upvar_proc_arguments() {
    let module = lower_module(
        "proc setter {first name} { upvar 1 $name x; set x 1 }\n\
             interp alias {} inner {} setter first\n\
             interp alias {} outer {} inner second\n\
             outer",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "outer")
        .expect("outer alias call should be in top-level CFG");
    assert!(
        defs.contains(&"second".to_owned()),
        "inner then outer alias prefixes must remain in runtime argv order: {defs:?}",
    );
    assert!(
        !defs.contains(&"first".to_owned()),
        "the first fixed argument must not be projected onto the second parameter: {defs:?}",
    );
}

#[test]
fn substituted_upvar_actual_is_opaque_not_a_literal_def() {
    // The source spelling `$caller_var` is not the variable name passed
    // at runtime. No precise def may be manufactured from that spelling.
    let module = lower_module(
        "proc setter {name} { upvar 1 $name x }\n\
             setter $caller_var",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        !defs.contains(&"caller_var".to_string()),
        "substituted source text became a literal caller def: {defs:?}",
    );
    assert_eq!(
        cfg.top_level.caller_frame_barrier,
        crate::dynamic_names::DynamicNameBarrier {
            writes: true,
            destroys: false,
            reads: false,
        }
    );
}

#[test]
fn trailing_expansion_preserves_a_known_upvar_actual_prefix() {
    let module = lower_module(
        "proc setter {name args} { upvar 1 $name x; set x 1 }\n\
             setter my_var {*}$rest",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(defs.contains(&"my_var".to_string()), "defs={defs:?}");
}

#[test]
fn expanded_upvar_actual_is_opaque_not_a_literal_def() {
    let module = lower_module(
        "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             setter {*}$actual",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.is_empty(),
        "expanded source text became defs: {defs:?}"
    );
    assert_eq!(
        cfg.top_level.caller_frame_barrier,
        crate::dynamic_names::DynamicNameBarrier {
            writes: true,
            destroys: false,
            reads: false,
        }
    );
}

/// A call omitting a `Name` parameter defines the place the parameter's
/// default names, as Tcl binds the default; a default that names no
/// variable the analysis can place widens the caller's frame, as a
/// substituted argument does.
#[test]
fn an_omitted_upvar_parameter_defines_its_default() {
    let module = lower_module(
        "proc setter {{name n}} { upvar 1 $name x; set x 1 }\n\
             setter",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "setter").expect("setter call should be in top-level CFG");
    assert_eq!(defs, vec!["n".to_owned()]);
    assert_eq!(
        cfg.top_level.caller_frame_barrier,
        crate::dynamic_names::DynamicNameBarrier::default()
    );
    let module = lower_module(
        "proc setter {{name a(1)}} { upvar 1 $name x; set x 1 }\n\
             setter",
    );
    let cfg = build_cfg(&module, false);
    assert_eq!(
        cfg.top_level.caller_frame_barrier,
        crate::dynamic_names::DynamicNameBarrier {
            writes: true,
            destroys: false,
            reads: false,
        }
    );
}

#[test]
fn computed_or_expanded_direct_head_is_not_misidentified_as_a_static_callee() {
    for call in ["$command target", "{*}$invocation"] {
        let module = lower_module(&format!(
            "proc setter {{name}} {{ upvar 1 $name x; set x 1 }}\n{call}"
        ));
        let cfg = build_cfg(&module, false);
        assert_eq!(
            cfg.top_level.caller_frame_barrier,
            crate::dynamic_names::DynamicNameBarrier::default(),
            "call={call}"
        );
    }
}

#[test]
fn non_upvar_proc_call_unchanged() {
    // No upvar in the callee — the call's defs should be empty.
    let module = lower_module("proc no_upvar {} { set x 1 }\nno_upvar");
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "no_upvar")
        .expect("no_upvar call should be in top-level CFG");
    assert!(defs.is_empty(), "expected no augmented defs, got {defs:?}");
}

#[test]
fn qualified_call_resolves_via_qualified_key() {
    // `proc ::ns::setter` is registered under both `::ns::setter`
    // and `setter`.  A qualified call site `::ns::setter` should
    // resolve via the qualified key.
    let module = lower_module(
        "namespace eval ::ns {}\n\
             proc ::ns::setter {} { upvar 1 caller_x x }\n\
             ::ns::setter",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "::ns::setter")
        .expect("::ns::setter call should be in top-level CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x in defs (qualified call), got {defs:?}",
    );
}

#[test]
fn cross_proc_call_inside_proc_body_augments_defs() {
    // Outer proc calls an inner upvar-using proc.  The outer
    // proc's CFG should reflect the augmented defs.
    let module = lower_module(
        "proc inner {} { upvar 1 caller_x x }\n\
             proc outer {} { inner }",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("outer proc CFG should exist");
    let defs = find_call_defs(outer, "inner").expect("inner call should be in outer CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x in inner's defs (called from outer), got {defs:?}",
    );
}

#[test]
fn late_alias_call_inside_proc_uses_projected_global_write_summary() {
    let module = lower_module(
        "proc ::mutate {} { global x; set x 99 }\n\
             namespace eval ::z {}\n\
             proc ::z::mutate {} {}\n\
             proc ::outer {} { call_mutate }\n\
             interp alias {} call_mutate {} mutate",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("outer proc CFG should exist");
    let defs =
        find_call_defs(outer, "call_mutate").expect("late alias call should be in outer CFG");
    assert!(
        defs.contains(&"x".to_owned()),
        "projected alias summary must invalidate x at the inner call: {defs:?}",
    );
}

#[test]
fn upvar_call_inside_if_branch_augments_defs() {
    // Calls inside structured constructs (if branches, while
    // bodies, ...) must also be augmented — the wiring runs in
    // `lower_script`, which is invoked recursively for every
    // body.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_y y }\n\
             if {1} { setter }",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "setter")
        .expect("setter call in if-branch should be in top-level CFG");
    assert!(
        defs.contains(&"caller_y".to_string()),
        "expected caller_y in defs (call inside if branch), got {defs:?}",
    );
}

#[test]
fn empty_module_no_upvar_context_no_panic() {
    // Empty module → empty upvar_procs and proc_params.  Building
    // a CFG should still succeed.
    let module = Module::default();
    let cfg = build_cfg(&module, false);
    assert_eq!(cfg.top_level.name, "::top");
    assert!(cfg.procedures.is_empty());
}

#[test]
fn build_cfg_function_does_not_apply_upvar_wiring() {
    // `build_cfg_function` is the no-context variant used by
    // tests and one-off CFG construction.  It MUST NOT augment
    // defs even when the script calls a known upvar proc —
    // the function only sees the script, not a module.
    let module = lower_module("proc setter {} { upvar 1 caller_x x }\nsetter");
    // Build only the top-level CFG via the no-context API.
    let func = build_cfg_function(
        "::top",
        &module.top_level,
        true,
        &native_registry(),
        module.plain_command_dispatch,
    );
    let defs = find_call_defs(&func, "setter").expect("setter call should be in top-level CFG");
    assert!(
        defs.is_empty(),
        "build_cfg_function without context should leave defs empty, got {defs:?}",
    );
}

// Embedded-substitution form: `[upvar_proc arg]` in every evaluated
// statement value, argument, expression, return, or condition surface.

/// Walk every block looking for a Call whose `defs` contain
/// *def*.  Returns the command name when found.
fn find_call_with_def<'a>(func: &'a Function, def: &str) -> Option<&'a str> {
    for block in func.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call { command, defs, .. } = stmt
                && defs.iter().any(|d| d == def)
            {
                return Some(command);
            }
        }
    }
    None
}

#[test]
fn embedded_subst_in_assign_value_emits_its_word_effects() {
    // `set foo [setter]` where setter upvars caller_x.  The
    // resulting CFG should have a word-effects `<word-effects>`
    // Call with `caller_x` in its defs, emitted BEFORE the
    // `set foo ...` AssignValue.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; return $x }\n\
             set foo [setter]",
    );
    let cfg = build_cfg(&module, false);
    let cmd = find_call_with_def(&cfg.top_level, "caller_x")
        .expect("expected a Call carrying caller_x in defs");
    assert_eq!(cmd, "<word-effects>");
}

/// The `(defs, reads)` of the first call named `command` in `func`.
fn defs_and_reads<'a>(func: &'a Function, command: &str) -> Option<(&'a [String], &'a [String])> {
    func.blocks.values().find_map(|block| {
        block.statements.iter().find_map(|stmt| match stmt {
            Statement::Call {
                command: name,
                defs,
                reads,
                ..
            } if name == command => Some((defs.as_slice(), reads.as_slice())),
            _ => None,
        })
    })
}

/// Where a statement's `[…]` substitutions write a place its own words read,
/// the call that carries the write reads it too — the version before the
/// write feeds the word that runs ahead of it. A read inside a command the
/// statement runs, or inside the expression word of one, is a read of the
/// statement.
#[test]
fn the_call_carrying_a_write_reads_the_place_its_statement_reads() {
    for (body, carrier) in [
        ("set r [expr {$x + [set x 10] + $x}]", "<word-effects>"),
        ("expr {$x + [set x 10]}", "<word-effects>"),
        ("set r \"$x [set x 10]\"", "<word-effects>"),
        ("incr r [expr {$x + [set x 10]}]", "<word-effects>"),
        ("return [expr {$x + [set x 10]}]", "<word-effects>"),
        (
            "set r [expr {[string length $x] + [set x 10]}]",
            "<word-effects>",
        ),
        (
            "set r [expr {[expr {$x + 1}] + [set x 10]}]",
            "<word-effects>",
        ),
        ("if {$x + [set x 10] > 3} {puts a}", "<cond>"),
        ("while {$x + [set x 10] < 3} {break}", "<cond>"),
        ("puts [expr {$x + [set x 10] + $x}]", "puts"),
        ("foo $x [incr x] $x", "foo"),
    ] {
        let module = lower_module(&format!("proc p {{}} {{ set x 1; set r 5; {body} }}"));
        let cfg = build_cfg(&module, false);
        let p = fixture_id_if_present(&cfg.procedures, "::p")
            .and_then(|id| cfg.procedures.get(&id))
            .expect("::p CFG");
        let (defs, reads) = defs_and_reads(p, carrier).expect(body);
        assert!(defs.iter().any(|name| name == "x"), "{body}: {defs:?}");
        assert!(reads.iter().any(|name| name == "x"), "{body}: {reads:?}");
    }
}

/// A place no word of the statement reads is no read of the call that
/// carries its write: the store the write overwrites stays dead, as it was
/// before a statement's reads were told from its writes, and a `gets` that
/// fills a loop's variable reads nothing of it.
#[test]
fn the_call_carrying_a_write_leaves_a_place_its_statement_never_reads_alone() {
    for (body, carrier, place) in [
        ("set r [expr {[set x 10] + 1}]", "<word-effects>", "x"),
        ("puts [set x 2]", "puts", "x"),
        (
            "while {[gets $fd line] >= 0} {puts $line}",
            "<cond>",
            "line",
        ),
        ("if {[set x 10] > $r} {puts a}", "<cond>", "x"),
    ] {
        let module = lower_module(&format!(
            "proc p {{fd}} {{ set x 1; set r 5; set line {{}}; {body} }}"
        ));
        let cfg = build_cfg(&module, false);
        let p = fixture_id_if_present(&cfg.procedures, "::p")
            .and_then(|id| cfg.procedures.get(&id))
            .expect("::p CFG");
        let (defs, reads) = defs_and_reads(p, carrier).expect(body);
        assert!(defs.iter().any(|name| name == place), "{body}: {defs:?}");
        assert!(!reads.iter().any(|name| name == place), "{body}: {reads:?}");
    }
}

/// The `(command, defs)` of every call in `func`'s blocks, in order.
fn calls_in_order(func: &Function) -> Vec<(&str, &[String])> {
    func.blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(|stmt| match stmt {
            Statement::Call { command, defs, .. } => Some((command.as_str(), defs.as_slice())),
            _ => None,
        })
        .collect()
}

fn position_of(calls: &[(&str, &[String])], command: &str) -> usize {
    calls
        .iter()
        .position(|(name, _)| *name == command)
        .unwrap_or_else(|| panic!("no {command} in {calls:?}"))
}

/// An opaque `catch` keeps its body inside the statement and stops at the
/// body's first error, so what the body writes is a may-definition stated
/// ahead of the call, and only the result and options variables the call
/// assigns once the body has finished are its own definitions.
#[test]
fn an_opaque_catch_states_what_its_body_writes_as_may_definitions_ahead_of_the_call() {
    let module = lower_module("set g 5\ncatch { if {[gets stdin] eq {q}} { set g 0 } } msg opts\n");
    let cfg = build_cfg(&module, false);
    let calls = calls_in_order(&cfg.top_level);
    let marker = position_of(&calls, "<arm-writes>");
    let catch = position_of(&calls, "catch");
    assert!(marker < catch, "{calls:?}");
    assert_eq!(calls[marker].1, ["g"]);
    assert_eq!(calls[catch].1, ["msg", "opts"]);
}

/// A callee an opaque `catch` body calls writes the caller's name through
/// `upvar`, as it would at a call the graph lowers: the name joins the
/// may-definitions.
#[test]
fn a_callee_an_opaque_catch_body_calls_may_define_the_name_it_writes() {
    let module = lower_module(
        "proc zero {v} {upvar 1 $v x; set x 0}\n\
             proc p {} {\n set go 1\n catch { if {1} { zero go } }\n puts $go\n}\n",
    );
    let cfg = build_cfg(&module, false);
    let p = fixture_id_if_present(&cfg.procedures, "::p")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::p CFG");
    let calls = calls_in_order(p);
    assert_eq!(calls[position_of(&calls, "<arm-writes>")].1, ["go"]);
}

/// A top-level replay starts from the entry's package rows as it starts
/// from the entry's command rows. A package the module's own walk loaded
/// is not loaded before the document's first command, so `package require
/// Tk` installs the supplied Tk 8.6 provider's widget commands instead of
/// counting as satisfied with none of them bound. A package no loader
/// declares stays unseen. Tcl 8.6 (#2324).
#[test]
fn a_top_level_replay_loads_a_supplied_package_from_the_entry_state() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let unit = |source: &str| {
        let context = std::sync::Arc::new(
            crate::environment_ingress::context_for_profile(registry.profile().unwrap())
                .with_command_store(registry.snapshot().shared_registry()),
        );
        crate::compilation_unit::CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
                dialect: registry.profile(),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&crate::provider_fixtures::entry(
                &registry,
                &[crate::provider_fixtures::Provider::Tk],
            )),
            context,
        )
    };
    let unseen = |unit: &crate::compilation_unit::CompilationUnit| {
        unit.top_level
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter(|stmt| {
                stmt.tokens().is_some_and(|tokens| {
                    tokens.synthetic == Some(crate::ir::SyntheticMarker::UnseenCall)
                })
            })
            .count()
    };
    assert_eq!(
        unseen(&unit(
            "package require Tk\nset v 1\nttk::entry .f\nputs $v\n"
        )),
        0,
        "a supplied Tk provider binds package, ttk::entry and puts"
    );
    assert!(
        unseen(&unit("package require Mystery\nset v 1\nputs $v\n")) > 0,
        "a package no loader declares runs code the module cannot see"
    );
}

/// The marker for where code the module cannot see runs, and the barrier
/// for a command that may write any name, stand ahead of the opaque
/// `catch` call whose body runs it: at the top level a plain name is the
/// global the code reaches, in a procedure a local is in the reach of a
/// callee that runs `upvar 1`, and a sourced file runs in the frame of the
/// call.
#[test]
fn an_opaque_catch_marks_the_unseen_code_and_the_barrier_its_body_runs() {
    let has_marker = |func: &Function, marker: crate::ir::SyntheticMarker| {
        func.blocks
            .values()
            .flat_map(|block| &block.statements)
            .any(|stmt| {
                matches!(
                    stmt,
                    Statement::Call { tokens: Some(tokens), .. }
                        | Statement::Barrier { tokens: Some(tokens), .. }
                        if tokens.synthetic == Some(marker)
                )
            })
    };
    let top = |src: &str| build_cfg(&lower_module(src), false).top_level;
    let in_proc = |src: &str| {
        let module = lower_module(&format!("proc p {{}} {{\n{src}\n}}"));
        build_cfg(&module, false)
            .procedures
            .get(&fixture_id(&module.procedures, "::p"))
            .expect("authored fixture body")
            .clone()
    };
    for src in [
        "catch {foo}",
        "catch {foo} msg",
        "catch { if {1} { foo } }",
        "catch { puts [foo] }",
        "catch { source other.tcl }",
        "if {[catch {foo}]} { puts bad }",
        "set rc [catch {foo} msg]",
        "puts [catch { puts [foo] }]",
        "while {[catch {if {1} {foo}}]} { break }",
        "set rc [catch $script]",
    ] {
        let func = top(src);
        assert!(
            has_marker(&func, crate::ir::SyntheticMarker::UnseenCall),
            "{src}"
        );
    }
    // An autoloaded or unknown-handled callee reaches a procedure's local
    // through `upvar 1`; a sourced file runs in its frame.
    for src in [
        "catch { if {1} { foo } }",
        "if {[catch {foo}]} { puts bad }",
        "set rc [catch {foo} msg]",
        "puts [catch { puts [foo] }]",
    ] {
        assert!(
            has_marker(&in_proc(src), crate::ir::SyntheticMarker::UnseenCall),
            "proc: {src}"
        );
    }
    assert!(has_marker(
        &in_proc("if {[catch {source other.tcl}]} { puts bad }"),
        crate::ir::SyntheticMarker::UnseenCall
    ));
    assert!(has_marker(
        &in_proc("catch { if {1} { source other.tcl } }"),
        crate::ir::SyntheticMarker::UnseenCall
    ));
    for (func, src) in [
        (top("catch { namespace eval :: {set go 0} }"), "top"),
        (
            in_proc("catch { if {1} { namespace eval :: {set go 0} } }"),
            "proc",
        ),
        (
            in_proc("catch { if {1} { dict with d { set go 0 } } }"),
            "dict with",
        ),
    ] {
        assert!(
            has_marker(&func, crate::ir::SyntheticMarker::CallerFrameOpaque),
            "{src}"
        );
    }
    // A body that writes another name and runs only commands the module
    // knows states no marker.
    let func = top("catch { if {1} { set other 1 } }");
    assert!(!has_marker(&func, crate::ir::SyntheticMarker::UnseenCall));
    assert!(!has_marker(
        &func,
        crate::ir::SyntheticMarker::CallerFrameOpaque
    ));
}

/// A loop that only writes variables leaves the command table it found:
/// the commands after it are the module's own, with no unseen code
/// (#2523, #2531).
#[test]
fn a_loop_that_writes_only_variables_keeps_the_commands_after_it_named() {
    let markers = |src: &str| -> Vec<Statement> {
        build_cfg(&lower_module(src), false)
            .top_level
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .filter(|stmt| {
                stmt.tokens().is_some_and(|tokens| {
                    matches!(
                        tokens.synthetic,
                        Some(
                            crate::ir::SyntheticMarker::UnseenCall
                                | crate::ir::SyntheticMarker::CallerFrameOpaque
                        )
                    )
                })
            })
            .cloned()
            .collect()
    };
    for src in [
        "for {set i 0} {$i < 10} {incr i} {}\nproc p {} { set i 0; incr i }\n",
        "set i 0\nwhile {$i < 5} {incr i}\nset r [lindex {a b} $i]\n",
        "foreach x {1 2} {set y $x}\nputs $y\n",
    ] {
        let found = markers(src);
        assert!(found.is_empty(), "{src}: {found:?}");
    }
    // A loop that rebinds a command leaves the commands after it unknown.
    assert_ne!(
        markers("foreach n {a b} {proc puts args {}}\nputs hi\n"),
        []
    );
}

/// A body runs once its procedure is invoked, so a call to a procedure the
/// file defines later is the module's own when no statement between the two
/// definitions can invoke the caller (#2407). A call at top level before
/// the callee's definition, or a definition a loop may reach again, keeps
/// the call unseen.
#[test]
fn a_forward_call_in_a_body_nothing_runs_early_is_named() {
    let unseen_in = |src: &str, name: &str| {
        let cfg = build_cfg(&lower_module(src), false);
        calls_in_order(
            cfg.procedures
                .get(&fixture_id(&cfg.procedures, name))
                .expect("authored fixture body"),
        )
        .iter()
        .any(|(command, _)| *command == "<unseen-call>")
    };
    let twice = "proc twice {name} {upvar 1 $name w; bump w; bump w}\n";
    let bump = "proc bump {name} {upvar 1 $name v; incr v}\n";
    for between in ["", "set x 0\n", "variable y 1\n"] {
        let src = format!("{twice}{between}{bump}proc c {{}} {{set x 0; twice x; return $x}}\n");
        assert!(!unseen_in(&src, "::twice"), "{src}");
    }
    for src in [
        format!("{twice}set x 0\ntwice x\n{bump}"),
        format!("{twice}puts hi\n{bump}"),
        format!("foreach i {{1 2}} {{\n{twice}}}\n{bump}"),
    ] {
        assert!(unseen_in(&src, "::twice"), "{src}");
    }
}

/// The marker for a call whose own head runs code the module cannot see
/// follows the call and carries nothing: the names the call's words read,
/// which may be rewritten as it runs, are the SSA's to state.
#[test]
fn the_marker_for_a_call_follows_it_and_states_no_name() {
    for (src, host) in [
        ("set cmd puts\n$cmd hi", "${cmd}"),
        ("set suffix _hi\nfoo$suffix", "foo${suffix}"),
        ("set a 1\nget $a", "get"),
        ("set f x.tcl\nsource $f", "source"),
    ] {
        let cfg = build_cfg(&lower_module(src), false);
        let calls = calls_in_order(&cfg.top_level);
        assert_eq!(
            calls,
            [(host, &[][..]), ("<unseen-call>", &[][..])],
            "{src}"
        );
    }
}

/// A computed head runs whatever its value names, so it is code the module
/// cannot see, as a literal head it does not define is, at the top level
/// and in a procedure body alike.
#[test]
fn a_computed_head_is_an_unseen_call() {
    let unseen = |func: &Function| {
        calls_in_order(func)
            .iter()
            .any(|(command, _)| *command == "<unseen-call>")
    };
    for src in [
        "$cmd",
        "$cmd arg",
        "puts [$cmd]",
        "set x [$cmd]",
        "if {[$cmd]} { puts a }",
    ] {
        assert!(
            unseen(&build_cfg(&lower_module(src), false).top_level),
            "{src}"
        );
    }
    for src in ["$cmd", "puts [$cmd]", "set x [$cmd]"] {
        let module = lower_module(&format!("proc p {{}} {{\n{src}\n}}"));
        let cfg = build_cfg(&module, false);
        assert!(
            unseen(
                cfg.procedures
                    .get(&fixture_id(&cfg.procedures, "::p"))
                    .expect("authored fixture body")
            ),
            "proc: {src}"
        );
    }
}

/// A command the document declares as a plain call that states its frame
/// effect is one the module can name: under `-frame own` and `-frame none`
/// its call, direct or in a substitution, marks nothing, and under
/// `-frame caller` it brings `argparse`'s caller-frame effect to the
/// computed-name walk. A declaration that states no frame effect, or
/// names a word the flow graph would have to follow, stays a call to code
/// the module cannot see; one with a body word lowers to the barrier
/// `time {…}` lowers to, whatever its frame effect.
#[test]
fn a_declared_plain_call_brings_the_frame_effect_it_states() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let build = |stub: &str, body: &str| {
        let src = format!(
            "# tcl-lsp: stubs-begin\n# tcl-lsp: stub {stub}\n# tcl-lsp: stubs-end\n\
                 proc p {{}} {{\n    set g 5\n    {body}\n    puts $g\n}}\n"
        );
        let declared = crate::analyser::utils::document_declared_surface(&src, None, "tcl8.6");
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(registry, config)
                .with_dialect(registry.profile())
                .with_declared_commands(Some(&declared)),
            &src,
        );
        build_cfg_with_registry(&module, false, registry)
            .procedures
            .get(&fixture_id(&module.procedures, "::p"))
            .expect("authored fixture body")
            .clone()
    };
    let unseen = |func: &Function| {
        calls_in_order(func)
            .iter()
            .any(|(command, _)| *command == "<unseen-call>")
    };
    let writes =
        |func: &Function| crate::dynamic_names::dynamic_name_barrier(func, registry, config).writes;
    for stub in [
        "db_query {sql} -frame own",
        "db_query {sql:pattern} -pure -frame none",
    ] {
        for body in ["db_query {select 1}", "set n [db_query {select 1}]"] {
            let func = build(stub, body);
            assert!(!unseen(&func), "{stub}: {body}");
            assert!(!writes(&func), "{stub}: {body}");
            assert_eq!(
                func.declared_frame_effects.get("::db_query"),
                Some(&None),
                "{stub}"
            );
        }
    }
    for body in ["db_query {select 1}", "set n [db_query {select 1}]"] {
        let func = build("db_query {sql} -frame caller", body);
        assert!(!unseen(&func), "caller: {body}");
        assert!(writes(&func), "caller: {body}");
    }
    for stub in [
        "db_query {sql}",
        "db_query {sql} -frame bogus",
        "db_query {row:var} -frame own",
        "db_query {sql:expr} -frame own",
        "db_query {sql} -barrier -frame own",
        "db_query {sql} -extension -frame own",
    ] {
        // #2427: an autoload index entry keeps the undeclared `db_query` unseen.
        let func = build(
            stub,
            "set ::auto_index(db_query) {source lib.tcl}; db_query {select 1}",
        );
        assert!(unseen(&func), "{stub}");
        assert!(func.declared_frame_effects.is_empty(), "{stub}");
    }
    let barrier_reason = |func: &Function, head: &str| {
        func.blocks
            .values()
            .flat_map(|block| &block.statements)
            .find_map(|stmt| match stmt {
                Statement::Barrier {
                    command, reason, ..
                } if command == head => Some(reason.clone()),
                _ => None,
            })
    };
    let registry_body = build("db_query {sql} -frame own", "time {set x 1}");
    assert!(barrier_reason(&registry_body, "time").is_some());
    for stub in [
        "db_eval {sql script:body} -frame own",
        "db_eval {sql script:body} -frame none",
        "db_eval {sql script:body}",
        "db_eval {sql cb:command_prefix} -frame own",
    ] {
        let declared_body = build(stub, "db_eval {select 1} {set x 1}");
        assert_eq!(
            barrier_reason(&declared_body, "db_eval"),
            barrier_reason(&registry_body, "time"),
            "{stub}"
        );
        assert!(declared_body.declared_frame_effects.is_empty(), "{stub}");
    }
}

/// A substitution's command reaches code the module cannot see through every
/// word it runs as code: a lambda's body, a body that runs in another frame
/// (`uplevel`, `namespace eval`), the commands in a `subst` text and in an
/// expression word of a body, as through a body of this frame, in a
/// procedure as at the top level. A body that holds no such code marks
/// nothing.
#[test]
fn a_substitution_marks_the_unseen_code_in_every_word_its_command_runs() {
    let unseen = |func: &Function| {
        calls_in_order(func)
            .iter()
            .any(|(command, _)| *command == "<unseen-call>")
    };
    for src in [
        "set x [apply {{} {foo}}]",
        "set x [apply {{x} {foo $x}} 1]",
        "set x [apply {{} {if {1} {foo}}}]",
        "set x [namespace eval ns {foo}]",
        "set x [namespace eval ns {namespace eval inner {foo}}]",
        "set x [uplevel #0 {foo}]",
        "set x [subst {[foo]}]",
        "set x [subst -nobackslashes {a [foo] b}]",
        "set x [catch {apply {{} {foo}}}]",
        "set x [catch {if {[foo]} {puts a}}]",
        "set x [catch {while {[foo]} {puts a}}]",
        "set x [catch {expr {[foo] + 1}}]",
        "set x [apply {{} {if {[foo]} {puts a}}}]",
        "puts [apply {{} {foo}}]",
        "if {[apply {{} {foo}}]} {puts a}",
        "set x [apply $lambda]",
        "set x [namespace eval ns $script]",
    ] {
        assert!(
            unseen(&build_cfg(&lower_module(src), false).top_level),
            "{src}"
        );
    }
    for src in [
        "set x [apply {{} {set y 1}}]",
        "set x [apply {{x} {set y $x}} 1]",
        "set x [namespace eval ns {set y 1}]",
        "set x [uplevel #0 {set y 1}]",
        "set x [subst {abc}]",
        "set x [subst {[set y 1]}]",
        "set x [subst -nocommands {a [foo] b}]",
        "set x [catch {apply {{} {set y 1}}}]",
        "set x [catch {if {[set y 1]} {puts a}}]",
        "set x [apply {{} {if {[set y 1]} {puts a}}}]",
    ] {
        assert!(
            !unseen(&build_cfg(&lower_module(src), false).top_level),
            "{src}"
        );
    }
    // A callee the code reaches runs `upvar 1` into a procedure's frame too.
    for src in [
        "set x [apply {{} {foo}}]",
        "set x [namespace eval ns {foo}]",
        "set x [subst {[foo]}]",
    ] {
        let module = lower_module(&format!("proc p {{}} {{\n{src}\n}}"));
        let cfg = build_cfg(&module, false);
        assert!(
            unseen(
                cfg.procedures
                    .get(&fixture_id(&cfg.procedures, "::p"))
                    .expect("authored fixture body")
            ),
            "proc: {src}"
        );
    }
}

/// A `try` the codegen shape keeps as one call states what its body, its
/// handlers and its `finally` script write as may-definitions, and the
/// variables its handlers bind are among them: a handler runs only when it
/// is selected.
#[test]
fn a_deferred_try_states_what_its_scripts_write_as_may_definitions() {
    let module = lower_module("try { foo } on error {msg opts} { set a 1 } finally { set b 2 }\n");
    let cfg = build_cfg(&module, true);
    let calls = calls_in_order(&cfg.top_level);
    let marker = position_of(&calls, "<arm-writes>");
    let call = position_of(&calls, "try");
    assert!(marker < call, "{calls:?}");
    assert_eq!(calls[marker].1, ["a", "b", "msg", "opts"]);
    assert!(calls[call].1.is_empty(), "{calls:?}");
    assert!(calls.iter().any(|(command, _)| *command == "<unseen-call>"));
}

#[test]
fn embedded_subst_in_return_emits_its_word_effects_before_terminator() {
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1; return 0 }\n\
             proc outer {} { return [setter] }",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::outer CFG");
    assert_eq!(
        find_call_with_def(outer, "caller_x"),
        Some("<word-effects>")
    );
    assert!(
        outer
            .blocks
            .values()
            .any(|block| { matches!(block.terminator, Some(Terminator::Return { .. })) })
    );
}

#[test]
fn switch_word_substitutions_invalidate_before_dispatch() {
    let module = lower_module(
        "proc setter {name} { upvar 1 $name value; set value 1; return 0 }\n\
             proc outer {} {\n\
                 set subject 1\n\
                 set pattern 1\n\
                 switch [setter subject] [setter pattern] { return ok } default { return no }\n\
             }",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::outer CFG");
    let boundary = outer
        .blocks
        .values()
        .find_map(|block| {
            block.statements.iter().find(|statement| {
                matches!(statement, Statement::Call { defs, .. }
                    if defs.iter().any(|name| name == "subject")
                        && defs.iter().any(|name| name == "pattern"))
            })
        })
        .expect("both argument writes must precede body dispatch");
    let Statement::Call {
        command, tokens, ..
    } = boundary
    else {
        unreachable!()
    };
    if command == "switch" {
        // An unresolved compiler protocol retains one runtime invocation.
        // Its argv effects occur before any entered script phase.
        let tokens = tokens.as_ref().expect("original argv carrier");
        assert!(tokens.evaluates_words());
        assert!(tokens.words()[1].sole_command_substitution().is_some());
        assert!(tokens.words()[2].sole_command_substitution().is_some());
        assert_eq!(outer.blocks.values().flat_map(|block| &block.statements)
                .filter(|statement| matches!(statement, Statement::Call {command, tokens, ..}
                    if command == "switch" && tokens.as_ref().is_none_or(crate::ir::CommandTokens::evaluates_words)))
                .count(), 1, "original invocation must execute once");
    } else {
        assert!(boundary.synthetic_marker().is_some());
        assert!(outer.blocks.values().any(|block| {
            block
                .statements
                .iter()
                .any(|statement| std::ptr::eq(statement, boundary))
                && matches!(block.terminator, Some(Terminator::Branch { .. }))
        }));
    }
}

#[test]
fn braced_switch_words_do_not_invalidate_before_dispatch() {
    let module = lower_module(
        "proc setter {name} { upvar 1 $name value; set value 1; return 0 }\n\
             proc outer {} {\n\
                 switch {[setter subject]} {[setter pattern]} { return ok }\n\
             }",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::outer CFG");
    assert_eq!(find_call_with_def(outer, "subject"), None);
    assert_eq!(find_call_with_def(outer, "pattern"), None);
}

#[test]
fn embedded_subst_in_assign_expr_and_expr_eval_emits_invalidation() {
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1; return 0 }\n\
             proc assign_expr {} { set result [expr {[setter] + 1}] }\n\
             proc expr_eval {} { expr {[setter] + 1} }",
    );
    let cfg = build_cfg(&module, false);
    for proc_name in ["::assign_expr", "::expr_eval"] {
        let function = cfg
            .procedures
            .get(&fixture_id(&module.procedures, proc_name))
            .expect("procedure CFG");
        assert_eq!(
            find_call_with_def(function, "caller_x"),
            Some("<word-effects>"),
            "missing embedded invalidation in {proc_name}"
        );
    }
}

#[test]
fn embedded_subst_in_call_arg_merges_into_call_defs() {
    // `puts [setter]` — Call host with embedded substitution.
    // The defs should merge into the existing Call's defs (no
    // word effects needed since the host is a Call).
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; return $x }\n\
             puts [setter]",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "puts").expect("puts call should be in top-level CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x merged into puts's defs, got {defs:?}",
    );
    // No word effects should appear (the Call branch
    // merged in place).
    let synthetic = find_call_with_def(&cfg.top_level, "caller_x");
    assert_eq!(
        synthetic,
        Some("puts"),
        "embedded extras should merge into the Call host, not a synthetic",
    );
}

#[test]
fn embedded_subst_param_form_resolves_call_site_arg() {
    // setter takes a parameter `name`; its upvar source is `$name`.
    // `set foo [setter myvar]` — the embedded call passes "myvar",
    // which becomes the caller-side def.
    let module = lower_module(
        "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             set foo [setter myvar]",
    );
    let cfg = build_cfg(&module, false);
    let cmd =
        find_call_with_def(&cfg.top_level, "myvar").expect("expected word effects carrying myvar");
    assert_eq!(cmd, "<word-effects>");
}

/// On tclsh 9.0.4:
///
/// ```tcl
/// proc far {name} {upvar 1 $name v; set v 2; return ok}
/// interp alias {} via {} far x
/// proc p {} {set x 1; set y [via]; return $x}
/// puts [p] ;# -> 2
/// ```
#[test]
fn embedded_alias_prefix_terminal_proc_invalidates_its_caller_target() {
    let module = lower_module(
        "proc far {name} {upvar 1 $name v; set v 2; return ok}\n\
             interp alias {} via {} far x\n\
             proc p {} {set x 1; set y [via]; return $x}",
    );
    let cfg = build_cfg(&module, false);
    let p = fixture_id_if_present(&cfg.procedures, "::p")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::p CFG");
    assert_eq!(find_call_with_def(p, "x"), Some("<word-effects>"));
}

#[test]
fn embedded_terminal_proc_honours_namespace_and_unknown_resolution() {
    let namespace_module = lower_module(
        "namespace eval ::a {\n\
                 proc mutate {} {upvar 1 x v; set v 2; return ok}\n\
                 proc p {} {set x 1; set y [mutate]; return $x}\n\
             }\n\
             namespace eval ::b {\n\
                 proc mutate {} {upvar 1 y v; set v 3; return ok}\n\
             }",
    );
    let namespace_cfg = build_cfg(&namespace_module, false);
    let p = fixture_id_if_present(&namespace_cfg.procedures, "::a::p")
        .and_then(|id| namespace_cfg.procedures.get(&id))
        .expect("::a::p CFG");
    assert_eq!(find_call_with_def(p, "x"), Some("<word-effects>"));
    assert_eq!(find_call_with_def(p, "y"), None);

    let unknown_module = lower_module(
        "proc unknown {name args} {upvar 1 $name v; set v 2; return ok}\n\
             proc p {} {set missing 1; set y [missing]; return $missing}",
    );
    let unknown_cfg = build_cfg(&unknown_module, false);
    let p = fixture_id_if_present(&unknown_cfg.procedures, "::p")
        .and_then(|id| unknown_cfg.procedures.get(&id))
        .expect("::p CFG");
    assert_eq!(find_call_with_def(p, "missing"), Some("<word-effects>"));
}

#[test]
fn embedded_subst_unknown_command_ignored() {
    // `[not_upvar]` — unknown command, should produce no
    // word effects.
    let module = lower_module("proc setter {} { set x 1 }\nset foo [setter]");
    let cfg = build_cfg(&module, false);
    // setter has no upvar, so neither direct nor embedded form
    // contributes — the only Call in the CFG should be
    // for the literal `setter` lookup (none here) or nothing.
    for block in cfg.top_level.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call { command, .. } = stmt {
                assert_ne!(
                    command, "<word-effects>",
                    "no word effects should appear for non-upvar embedded calls",
                );
            }
        }
    }
}

#[test]
fn embedded_subst_no_bracket_in_text_short_circuits() {
    // `set foo "plain string"` — value has no `[`, so the
    // embedded-substitution scan should short-circuit on the
    // `text.contains('[')` guard and produce no extras.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x }\n\
             set foo plain",
    );
    let cfg = build_cfg(&module, false);
    let synthetic = find_call_with_def(&cfg.top_level, "caller_x");
    assert!(
        synthetic.is_none(),
        "no word effects expected when text has no `[`, got {synthetic:?}",
    );
}

#[test]
fn embedded_subst_synthetic_appears_before_host_assign() {
    // The word effects must land BEFORE the host
    // AssignValue in program order, so SSA / dataflow correctly
    // see the invalidation before any later use of the variable.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x }\n\
             set foo [setter]",
    );
    let cfg = build_cfg(&module, false);
    let entry = &cfg.top_level.blocks[&cfg.top_level.entry];
    // Find the word effects' index and the AssignValue's index; assert
    // ordering.
    let mut synthetic_idx = None;
    let mut assign_idx = None;
    for (i, stmt) in entry.statements.iter().enumerate() {
        match stmt {
            _ if stmt.word_effects_host() == Some(crate::ir::WordEffectsHost::Statement) => {
                synthetic_idx = Some(i);
            }
            Statement::AssignValue { name, .. } if name == "foo" => {
                assign_idx = Some(i);
            }
            _ => {}
        }
    }
    let s = synthetic_idx.expect("<word-effects> should be in entry block");
    let a = assign_idx.expect("set foo AssignValue should be in entry block");
    assert!(s < a, "word effects at {s} should precede assign at {a}");
}

// A known upvar proc's call-by-name write reached
// only through a nested `[...]` (a wrapping command around it, or a
// loop/branch condition) must still be recovered — real tcllib
// `cmdline::getoptions` repro: `while {[set err [getopt argv $opts
// opt arg]]} { ... }`.

#[test]
fn embedded_subst_recovers_a_wrapped_upvar_proc_call() {
    // TP — `set x [set err [setter]]`: `setter` is nested one bracket
    // deeper than the embedded-substitution scan's own top-level word
    // check reaches (`set` itself is never a known upvar proc), so
    // recovering it requires recursing into the inner text rather than
    // pattern-matching the (bracket-stripped) word list.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             set out [set err [setter]]",
    );
    let cfg = build_cfg(&module, false);
    let cmd = find_call_with_def(&cfg.top_level, "caller_x")
        .expect("expected word effects recovering caller_x through the wrapper");
    assert_eq!(cmd, "<word-effects>");
}

#[test]
fn condition_out_vars_recovers_catch_scan_gets_regexp_and_upvar_procs() {
    // TP — `condition_out_vars` unions the hardcoded builtin scan with
    // the upvar-proc / global-write-proc resolution; a known upvar
    // proc's write reached through a condition must appear in the
    // union exactly like a `catch` result var already does.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             if {[setter]} { puts $caller_x }",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x in the <cond> Call's defs, got {defs:?}",
    );
}

#[test]
fn frozen_while_condition_carries_upvar_proc_defs() {
    // TP — `while {[getopt ...]}`'s condition is purely a command
    // substitution, so `lower_while_or_frozen` freezes the whole loop
    // as an opaque `Statement::Barrier` (no `defs` field at all) rather
    // than calling `lower_while`. Without a separate synthetic `<cond>`
    // Call carrying the condition's defs, `getopt`'s upvar write to
    // `opt` would be invisible to the def-use graph even though the
    // barrier's own `uses_of` textually scans the (un-lowered) body
    // for `$opt` and attributes the read to the same statement.
    let module = lower_module(
        "proc getopt {ovar} { upvar 1 $ovar opt; set opt 1 }\n\
             while {[getopt opt]} { puts $opt }",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
    assert!(
        defs.contains(&"opt".to_string()),
        "expected opt in the frozen while's <cond> Call defs, got {defs:?}",
    );
    // The barrier itself must still be there too — freezing, not
    // ordinary `lower_while`, is exactly the shape under test.
    let has_barrier = cfg.top_level.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Barrier { command, .. } if command == "while"))
    });
    assert!(
        has_barrier,
        "expected the loop to still freeze as a Barrier"
    );
}

#[test]
fn frozen_for_condition_carries_upvar_proc_defs() {
    // TP — the `for` loop's identical frozen-barrier path: the same rule
    // applies to `for {...} [cond] {...} {...}`.
    let module = lower_module(
        "proc getopt {ovar} { upvar 1 $ovar opt; set opt 1 }\n\
             for {} {[getopt opt]} {} { puts $opt }",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
    assert!(
        defs.contains(&"opt".to_string()),
        "expected opt in the frozen for's <cond> Call defs, got {defs:?}",
    );
}

#[test]
fn frozen_while_condition_with_no_upvar_proc_omits_synthetic_cond() {
    // TN — a frozen loop whose condition calls an *ordinary* (non-
    // upvar, non-`catch`/`scan`/`gets`/`regexp`) command must not gain
    // a synthetic `<cond>` Call at all — nothing for it to carry.
    let module = lower_module("proc pureCheck {} { return 1 }\nwhile {[pureCheck]} { }");
    let cfg = build_cfg(&module, false);
    assert!(
        find_call_defs(&cfg.top_level, "<cond>").is_none(),
        "no synthetic <cond> Call expected when the condition defines nothing",
    );
}

#[test]
fn try_handler_return_options_terminates() {
    // A `try` handler ending in `return -code error …` (lowered to a
    // "return with options" barrier) returns from the proc, so it must
    // *terminate* its block rather than fall through to the post-`try`
    // join. Regression for the spurious `try_handler → try_end` edge +
    // phi (e.g. `auto_mkindex`). Analysis builds only (`build_cfg` sets
    // `faithful_exceptions`); codegen leaves the barrier as a plain stmt.
    let module =
        lower_module("proc f {} { try { set a 1 } on error {} { return -code error boom } }");
    let cfg = build_cfg(&module, false);
    let func = fixture_id_if_present(&cfg.procedures, "::f")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::f cfg");
    let handler = func
        .blocks
        .values()
        .find(|b| b.name.starts_with("try_handler"))
        .expect("try_handler block");
    assert!(
        matches!(
            handler.terminator,
            Some(Terminator::Complete {
                route: tcl_registry::completion_route::InvocationCompletionRoute::Return(
                    tcl_registry::completion_route::ReturnCompletionRoute {
                        eventual_code: tcl_registry::CompletionCode::Error,
                        remaining_level: 1,
                    }
                ),
                ..
            })
        ),
        "return-options handler must terminate (no fall-through to try_end), got {:?}",
        handler.terminator,
    );
}

#[test]
fn distinct_body_summaries_preserve_conflicting_alias_targets() {
    // naming.analysis.procedure-summary-candidate-keys
    // docs/design/analysis/name-resolution-proofs/procedure-summary-candidate-keys.md
    // Actual whole-module source and existing frame-summary producer.
    let module = lower_module(
        "namespace eval a {proc bind {} {upvar 1 left local; set local 1}}\n\
             namespace eval b {proc bind {} {upvar 1 right local; set local 2}}\n\
             proc forward {} {uplevel 1 [list bind]}",
    );
    let summaries = detect_upvar_procs(&module);
    assert_eq!(
        summaries[&summary_id(&module, "::a::bind")]
            .literal_targets
            .get("local")
            .map(String::as_str),
        Some("left")
    );
    assert_eq!(
        summaries[&summary_id(&module, "::b::bind")]
            .literal_targets
            .get("local")
            .map(String::as_str),
        Some("right")
    );
    assert_eq!(summaries.len(), module.procedures.len());
    let a = summary_id(&module, "::a::bind");
    let b = summary_id(&module, "::b::bind");
    assert_ne!(a, b);
    // The ambiguous constructed call belongs to forward's source receipt.
    let forwarded = &summaries[&summary_id(&module, "::forward")];
    assert!(forwarded.literal_targets.is_empty());
    assert!(forwarded.caller_frame_opaque_writes && forwarded.caller_frame_opaque_reads);
}

#[test]
fn summary_parameters_retain_each_original_signature() {
    // naming.analysis.procedure-summary-candidate-keys
    // docs/design/analysis/name-resolution-proofs/procedure-summary-candidate-keys.md
    // Defaults, order and variadic position belong to the original header.
    let module = lower_module(
        "namespace eval a {proc p {{name left} args} {}; proc q {{name shared} args} {}; proc r {a b} {}; proc s {a args} {}}\n\
             namespace eval b {proc p {{name right} args} {}; proc q {{name shared} args} {}; proc r {b a} {}; proc s {args a} {}}",
    );
    let (_, params, _, _, _) = prepare_cfg_context(&module);
    assert_eq!(params.len(), 8);
    for label in ["::a::p", "::b::p", "::a::q", "::b::q"] {
        assert_eq!(
            params.get(&summary_id(&module, label)),
            Some(&vec!["name".into(), "args".into()])
        );
    }
    assert_eq!(
        params.get(&summary_id(&module, "::a::r")),
        Some(&vec!["a".into(), "b".into()])
    );
    assert_eq!(
        params.get(&summary_id(&module, "::b::r")),
        Some(&vec!["b".into(), "a".into()])
    );
    assert_eq!(
        params.get(&summary_id(&module, "::a::s")),
        Some(&vec!["a".into(), "args".into()])
    );
    assert_eq!(
        params.get(&summary_id(&module, "::b::s")),
        Some(&vec!["args".into(), "a".into()])
    );
    let original = module
        .procedures
        .get(&summary_id(&module, "::a::p"))
        .unwrap();
    let distinct = module
        .procedures
        .get(&summary_id(&module, "::b::p"))
        .unwrap();
    assert_ne!(original.params_raw, distinct.params_raw);
}

#[test]
fn body_summary_selection_declines_foreign_id_at_the_original_declaration() {
    let mut module = lower_module("proc writer {} {upvar 1 x local; set local 1}; writer");
    let foreign = lower_module("proc writer {} {upvar 1 x local; set local 1}; writer");
    let own_id = summary_id(&module, "::writer");
    let foreign_id = summary_id(&foreign, "::writer");
    assert_ne!(own_id, foreign_id);
    let body = module.procedures.get(&own_id).unwrap().clone();
    let call = module.top_level.statements.last().unwrap().clone();
    let original = module
        .top_level
        .retained_source_tokens_for_statement(module.top_level.statements.last().unwrap())
        .unwrap()
        .clone();
    let registry = module.resolved_registry().snapshot().shared_registry();
    let owners = OriginalProcedureSummaryOwners::for_module(&module, &registry);
    assert_eq!(owners.calls(&original, &registry).unwrap()[0].0, own_id);
    module.procedures.insert(foreign_id, body);
    let owners = OriginalProcedureSummaryOwners::for_module(&module, &registry);
    assert!(owners.calls(&original, &registry).is_none());
    let prepared = prepare_cfg_context_bundle(&module, &registry);
    let mut builder = CfgBuilder::new_with_upvars_and_classes(
        true,
        prepared.context.0.clone(),
        prepared.context.1.clone(),
        prepared.context.2.clone(),
        prepared.context.3.clone(),
        &registry,
        prepared.command_classes.clone(),
    )
    .with_procedure_summary_owners(prepared.context.4.clone());
    builder
        .command_binding_sites
        .extend(module.top_level.command_binding_sites.iter().cloned());
    assert!(builder.direct_upvar_effects(&call).opaque_arguments);
}

#[test]
fn read_beside_writes_uses_original_roles_and_preserves_refusals() {
    for (source, expected) in [
        ("puts [set x 2] [info exists x]", vec!["x".to_owned()]),
        ("puts [set x 2]", Vec::new()),
    ] {
        let module = lower_module(source);
        let registry = module.resolved_registry();
        let prepared = prepare_cfg_context_bundle(&module, registry);
        let builder = CfgBuilder::new_with_upvars_and_classes(
            false,
            prepared.context.0.clone(),
            prepared.context.1.clone(),
            prepared.context.2.clone(),
            prepared.context.3.clone(),
            registry,
            prepared.command_classes.clone(),
        )
        .with_procedure_summary_owners(prepared.context.4.clone());
        let statement = module.top_level.statements.last().unwrap();
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let written = vec!["x".to_owned(), "unrelated".to_owned()];
        let reads = builder.read_beside_writes(&written, Some(tokens), |scanner| {
            scanner.scan_lexical_word("", registry)
        });
        assert_eq!(reads, expected, "{source}");
        assert_eq!(
            builder.read_beside_writes(&written, None, |scanner| {
                scanner.scan_lexical_word("", registry)
            }),
            written
        );
        let mut changed = tokens.clone();
        changed.source_binding = None;
        assert_eq!(
            builder.read_beside_writes(&written, Some(&changed), |scanner| {
                scanner.scan_lexical_word("", registry)
            }),
            written
        );
    }
}

#[test]
fn summary_keys_decline_one_changed_member_and_foreign_module_source() {
    // naming.analysis.procedure-summary-candidate-keys
    // docs/design/analysis/name-resolution-proofs/procedure-summary-candidate-keys.md
    // Removing original correspondence cannot leave a false unique winner.
    let module = lower_module(
        "namespace eval a {proc p {name} {upvar 1 $name local}}\n\
             namespace eval b {proc p {name} {upvar 1 $name local}}",
    );
    let mut changed = module.clone();
    {
        let id = fixture_id(&changed.procedures, "::b::p");
        changed.procedures.get_mut(&id)
    }
    .unwrap()
    .params_raw
    .push_str(" extra");
    let (summaries, params, _, _, _) = prepare_cfg_context(&changed);
    let unchanged = summary_id(&changed, "::a::p");
    let changed_id = summary_id(&changed, "::b::p");
    assert!(params.contains_key(&unchanged));
    assert!(!params.contains_key(&changed_id));
    assert!(summaries[&changed_id].literal_targets.is_empty());
    assert!(summaries[&changed_id].param_targets.is_empty());
    assert!(summaries[&changed_id].caller_frame_opaque_reads);
    let mut foreign = module;
    foreign.source = tcl_lexer::SourceImage::document("proc replacement {} {}");
    let (summaries, params, _, _, _) = prepare_cfg_context(&foreign);
    assert!(params.is_empty());
    assert!(
        summaries
            .values()
            .all(|summary| summary.literal_targets.is_empty()
                && summary.param_targets.is_empty()
                && summary.caller_frame_opaque_writes
                && summary.caller_frame_opaque_reads)
    );
}
