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

//! Folds over the retained substitution inventory: availability, namespace paths, missing receipts, object callbacks and procedure arguments.

use super::*;

#[test]
fn retained_substitution_inventory_keeps_module_input_config_and_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional original child geometry, without handler or erasure permission.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig {
        expand_syntax: false,
        strict_quoting: true,
        ..tcl_lexer::LexerConfig::from_grammar(profile.grammar)
    };
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let unit = CompilationUnit::build_with_analysis_input(
        "expr {[dict size [dict create key value]]}",
        crate::compilation_unit::UnitBuildOptions {
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
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    let module = &unit.ir_module;
    let script = &module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .unwrap();
    let calls = retained_substitution_calls(tokens, module, context.commands()).unwrap();
    assert_eq!(calls.len(), 2);
    assert!(
        calls
            .iter()
            .all(|call| call.tokens.as_ref().is_some_and(|tokens| tokens
                .source_binding
                .as_ref()
                .is_some_and(|binding| binding.invocation_site().is_some())))
    );
    for change in 0..5 {
        let mut changed = module.clone();
        match change {
            0 => changed.retained_source_bindings = None,
            1 => changed.source_entry.unknown_entry = !changed.source_entry.unknown_entry,
            2 => changed.top_level_namespace = "::other".into(),
            3 => {
                changed.native_namespace =
                    Some(tcl_core_types::ByteNamespacePath::from_segments(["other"]));
            }
            4 => changed.top_level_kind = crate::ir::TopLevelKind::ProcedureBody,
            _ => unreachable!(),
        }
        assert!(
            crate::registry_invocation::InvocationMetadataContext::for_module(
                context.commands(),
                &changed
            )
            .is_some()
        );
        assert!(retained_substitution_calls(tokens, &changed, context.commands()).is_none());
    }
    let mut changed = module.clone();
    changed.lexer_config.expand_syntax = true;
    assert!(retained_substitution_calls(tokens, &changed, context.commands()).is_none());
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(context.commands())),
    );
    assert!(std::sync::Arc::ptr_eq(older.commands(), context.commands()));
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for supplied in [Some(older), Some(foreign), None] {
        changed = module.clone();
        changed.source_metadata_input = supplied.map(|availability| {
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, availability, config)
        });
        assert!(retained_substitution_calls(tokens, &changed, context.commands()).is_none());
    }
}

#[test]
fn retained_substitution_folds_use_the_selected_namespace_path_procedure() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let source = "proc ::p {} {return GLOBAL}\n\
                      namespace eval ::lib {proc p {} {return PATH}}\n\
                      namespace eval ::N {namespace path ::lib; proc caller {} {puts [p]}}\n\
                      ::N::caller";
    let cu = CompilationUnit::build_for_dialect(source, registry, false, "tcl9.0")
        .with_interprocedural(
            registry,
            Some(tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()),
        );
    let mut context = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
    context.registry = Some(registry);
    run(&mut context, &cu);
    assert!(
        context.optimisations.iter().all(|optimisation| {
            optimisation.code != DiagCode::O103
                || optimisation.hint_only
                || optimisation.replacement != "GLOBAL"
        }),
        "{:?}",
        context.optimisations
    );
}

#[test]
fn retained_substitution_folds_decline_namespaced_builtin_shadows() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for invocation in ["[expr {1 + 2}]", "[string length abc]", "[list a b]"] {
        let source = format!(
            "namespace eval N {{proc expr {{args}} {{return EXPR}}; proc string {{args}} {{return STRING}}; proc list {{args}} {{return LIST}}; proc run {{}} {{puts {invocation}}}}}\nN::run"
        );
        let cu = CompilationUnit::build_for_dialect(&source, registry, false, "tcl9.0")
            .with_interprocedural(
                registry,
                Some(
                    tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                ),
            );
        let mut context = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        context.registry = Some(registry);
        run(&mut context, &cu);
        assert!(
            context.optimisations.iter().all(|optimisation| {
                !matches!(
                    optimisation.code,
                    DiagCode::O101
                        | DiagCode::O115
                        | DiagCode::O116
                        | DiagCode::O118
                        | DiagCode::O129
                ) || optimisation.hint_only
                    || !source
                        .get(optimisation.span.as_range())
                        .is_some_and(|text| text.contains(invocation))
            }),
            "{source}: {:?}",
            context.optimisations
        );
    }
}

#[test]
fn retained_substitution_missing_child_receipt_withdraws_applicable_folding() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let source = "puts [list a b]";
    let cu = CompilationUnit::build_for_dialect(source, registry, false, "tcl9.0");
    let statement = &cu.ir_module.top_level.statements[0];
    let mut tokens = cu
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .expect("original carrier")
        .clone();
    tokens.source_binding = None;
    let mut context = PassContext::new(source, InterproceduralAnalysis::default());
    context.registry = Some(registry);
    visit_call_cmd_subst_folds(
        &mut context,
        &cu,
        &tokens,
        &std::collections::HashMap::new(),
        "::",
        statement.span(),
    );
    assert_eq!(context.optimisations.len(), 0);
}

#[test]
fn retained_substitution_procedure_folding_preserves_arity_and_argument_effects() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    for source in [
        "proc f {required} {return CONSTANT}; puts [f]",
        "proc f {} {return CONSTANT}; puts [f EXTRA]",
        "proc f {ignored} {return CONSTANT}; set x 0; puts [f [incr x]]; puts $x",
    ] {
        let cu = CompilationUnit::build_for_dialect(source, registry, false, "tcl9.0")
            .with_interprocedural(registry, registry.profile());
        let mut context = PassContext::new(&cu.source, cu.interproc.clone().unwrap_or_default());
        context.registry = Some(registry);
        run(&mut context, &cu);
        assert!(
            context
                .optimisations
                .iter()
                .all(|optimisation| optimisation.code != DiagCode::O103 || optimisation.hint_only),
            "{source}: {:?}",
            context.optimisations
        );
    }
}

#[test]
fn retained_substitution_native_object_callbacks_withdraw_builtin_erasure() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
    let source = "proc f {input} {puts [llength $input]}";
    let cu = CompilationUnit::build_for_dialect(source, registry, false, "tcl9.1");
    let statement = &cu
        .ir_module
        .procedures
        .get(&fixture_id(&cu.ir_module.procedures, "::f"))
        .expect("authored fixture body")
        .body
        .statements[0];
    let tokens = cu
        .ir_module
        .procedures
        .get(&fixture_id(&cu.ir_module.procedures, "::f"))
        .expect("authored fixture body")
        .body
        .retained_source_tokens_for_statement(statement)
        .expect("original carrier");
    let calls =
        retained_substitution_calls(tokens, &cu.ir_module, registry).expect("complete children");
    let call = calls
        .iter()
        .find(|call| {
            call.tokens.as_ref().is_some_and(|tokens| {
                tokens
                    .argv_texts
                    .first()
                    .is_some_and(|head| head == "llength")
            })
        })
        .expect("original length invocation");
    let metadata = Some(cu.top_level.invocation_metadata_context(registry).unwrap());
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry,
        metadata,
        call.tokens.as_ref().unwrap(),
    )
    .expect("selected builtin independent of native object effects");
    assert!(invocation.facts.effects.requires_world_barrier());
    assert!(
        fold_retained_builtin(
            registry,
            call,
            &calls,
            None,
            metadata,
            (
                None,
                None,
                tcl_registry::value_transfer::ScriptEncoding::Auto
            )
        )
        .is_none()
    );
}

#[test]
fn retained_substitution_formal_seeds_share_defaults_and_jim_rest_names() {
    use tcl_dialect::ParameterGrammar::{Jim, Tcl};
    let policy = crate::tcl_expr_eval::FoldPolicy::default();
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in("{value DEFAULT} args", Tcl).unwrap();
    let seed = seed_params_from_args(&parameters, &[], Tcl, policy).unwrap();
    assert_eq!(
        seed.get(&("value".into(), 0)),
        Some(&LatticeValue::Const(ConstValue::String("DEFAULT".into())))
    );
    assert_eq!(
        seed.get(&("args".into(), 0)),
        Some(&LatticeValue::Const(ConstValue::String(String::new())))
    );
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in("{args remaining} tail", Jim)
            .unwrap();
    let seed = seed_params_from_args(
        &parameters,
        &[ConstValue::Int(1), ConstValue::Int(2), ConstValue::Int(3)],
        Jim,
        policy,
    )
    .unwrap();
    assert_eq!(
        seed.get(&("remaining".into(), 0)),
        Some(&LatticeValue::Const(ConstValue::String("1 2".into())))
    );
    assert_eq!(
        seed.get(&("tail".into(), 0)),
        Some(&LatticeValue::Const(ConstValue::Int(3)))
    );
    let parameters =
        tcl_syntax::formal_params::parse_formal_parameters_in("&reference", Jim).unwrap();
    assert!(
        seed_params_from_args(&parameters, &[ConstValue::String("x".into())], Jim, policy)
            .is_none()
    );
}

#[test]
fn retained_expression_argument_folding_requires_actual_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual original registrations are retained independently of metadata
    // availability. The owner stays alive through every refusal control.
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut catalogue = baseline
        .commands()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut descriptor = catalogue.get("expr").unwrap().clone();
    descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    catalogue.insert(descriptor);
    let current = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(catalogue)));
    let registry = current.commands();
    let (_owner, native) =
        crate::environment_ingress::captured_native_entry_with_owner(registry.profile().unwrap());
    let entry = crate::command_binding::SourceAnalysisEntry {
        invocation_dialect: registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        native_entry: Some(std::sync::Arc::new(native)),
        ..crate::command_binding::SourceAnalysisEntry::default()
    };
    let mut unit = CompilationUnit::build_with_context_registry(
        "puts [expr {1 + 2}]",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
            dialect: registry.profile(),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&current),
    );
    let statement = &unit.ir_module.top_level.statements[0];
    let tokens = unit
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap()
        .clone();
    let calls = retained_substitution_calls(&tokens, &unit.ir_module, registry).unwrap();
    let call = calls.iter().find(|call| call.command == "expr").unwrap();
    let constants = std::collections::HashMap::new();
    let mut context = PassContext::new(&unit.source, InterproceduralAnalysis::default());
    context.registry = Some(registry);
    assert_eq!(
        try_o101_expr_arg_fold(&context, &unit, &tokens, call, &constants),
        Some("3".into())
    );
    let input = unit.ir_module.source_metadata_input.clone().unwrap();
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    assert!(std::sync::Arc::ptr_eq(older.commands(), registry));
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for withheld in [Some(older), Some(foreign), None] {
        unit.ir_module.source_metadata_input = withheld.map(|availability| {
            crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                availability,
                input.lexer_config(),
            )
        });
        let mut context = PassContext::new(&unit.source, InterproceduralAnalysis::default());
        context.registry = Some(registry);
        assert!(try_o101_expr_arg_fold(&context, &unit, &tokens, call, &constants).is_none());
    }
}

#[test]
fn retained_recursive_folding_preserves_actual_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = current.commands();
    let cu = CompilationUnit::build_for_dialect(
        "puts [list [dict create key value]]",
        registry,
        false,
        "tcl8.6",
    );
    let statement = &cu.ir_module.top_level.statements[0];
    let tokens = cu
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let calls = retained_substitution_calls(tokens, &cu.ir_module, registry).unwrap();
    let outer = calls.iter().find(|call| call.command == "list").unwrap();
    assert_eq!(
        fold_retained_builtin(
            registry,
            outer,
            &calls,
            None,
            Some(current.into()),
            (
                None,
                None,
                tcl_registry::value_transfer::ScriptEncoding::Auto
            )
        ),
        Some("{key value}".into())
    );
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    for metadata in [Some((&older).into()), Some(foreign.into()), None] {
        assert!(
            fold_retained_builtin(
                registry,
                outer,
                &calls,
                None,
                metadata,
                (
                    None,
                    None,
                    tcl_registry::value_transfer::ScriptEncoding::Auto
                )
            )
            .is_none()
        );
    }
    let tokens = tokens.clone();
    let span = statement.span();
    let mut missing = cu;
    missing.ir_module.source_metadata_input = None;
    let mut context = PassContext::new(&missing.source, InterproceduralAnalysis::default());
    context.registry = Some(registry);
    visit_call_cmd_subst_folds(
        &mut context,
        &missing,
        &tokens,
        &std::collections::HashMap::new(),
        "::",
        span,
    );
    assert_eq!(context.optimisations.len(), 0);
}
