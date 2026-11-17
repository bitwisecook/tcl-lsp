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

//! Function and unit reports: retained source owners, sink candidates, replay availability and deterministic order.

use super::*;

#[test]
fn original_module_sink_candidates_keep_conditional_source_and_refuse_foreign_owners() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let build = |source| {
        crate::compilation_unit::CompilationUnit::build_with_analysis_input(
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
        )
        .with_interprocedural(registry, Some(profile))
    };
    let source =
        "proc pass {x} {return $x}; proc danger {} {set u [gets stdin]; set p [pass $u]; exec $p}";
    let unit = build(source);
    let (body, procedure) = unit
        .ir_module
        .original_source_procedure_bodies()
        .find(|(_, procedure)| {
            procedure
                .body_source
                .as_deref()
                .is_some_and(|source| source.contains("exec $p"))
        })
        .unwrap();
    assert_eq!(
        procedure.body_kind,
        crate::ir::ProcedureBodyKind::OriginalDeclaration
    );
    assert!(!unit.ir_module.procedures.contains_key(body));
    assert!(unit.ir_module.source_entry.native_entry.is_none());
    let warnings = |unit: &crate::compilation_unit::CompilationUnit, registry: &CommandRegistry| {
        let solved =
            crate::taint_interproc::solve_interprocedural_taints(unit, registry, Some(profile));
        let mut warnings = Vec::new();
        let _ = crate::compiler_checks::push_taint_and_module_checks(
            unit,
            registry,
            Some(profile),
            &solved,
            None,
            &mut warnings,
        );
        warnings
    };
    assert!(
        warnings(&unit, registry)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
    let mut missing = unit.clone();
    missing.ir_module.retained_source_bindings = None;
    assert!(
        !warnings(&missing, registry)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
    let mut changed = unit.clone();
    changed.ir_module.source = tcl_lexer::SourceImage::document("proc danger {} {return SAFE}");
    assert!(
        !warnings(&changed, registry)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    assert!(
        !warnings(&unit, foreign)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
    let shadowed =
        build("proc exec args {return SAFE}; proc danger {} {set u [gets stdin]; exec $u}");
    assert!(
        !warnings(&shadowed, registry)
            .iter()
            .any(|warning| warning.code == DiagCode::T100)
    );
}

#[test]
fn whole_unit_warning_query_requires_its_retained_positioned_source_owner() {
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // Explicit source ingress supplies the tainted value; no external evaluation is performed.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let (_owner, unit) = retained_warning_unit(
        "set payload [gets stdin]\nregexp -- $payload haystack",
        &current,
    );
    let positive = find_taint_warnings_for_cu_base(&unit, registry, None);
    assert!(
        positive
            .iter()
            .any(|warning| { warning.code == DiagCode::T103 && warning.variable == "payload" }),
        "the actual original source keeps its warning: {positive:?}"
    );
    let mut missing = unit.clone();
    missing.ir_module.retained_source_bindings = None;
    let mut changed = unit.clone();
    changed.ir_module.source = tcl_lexer::SourceImage::document("regexp -- fixed haystack");
    let mut missing_snapshot = unit.clone();
    missing_snapshot.ir_module.registry_snapshot = None;
    for refused in vec![missing, changed, missing_snapshot].into_boxed_slice() {
        assert_eq!(
            find_taint_warnings_for_cu_base(&refused, registry, None),
            [] as [crate::taint::TaintWarning; 0]
        );
    }
    assert_eq!(
        find_taint_warnings_for_cu_base(&unit, registry, None),
        positive
    );
}

#[test]
fn actual_warning_wrapper_uses_the_retained_context_and_lexer() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The sink's nested wrapper uses the same source-lattice owner as an
    // assigned value. No source-channel execution or mitigation is observed.
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "set payload [gets stdin]; puts [string length $payload]",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&current),
    );
    let tokens = unit
        .ir_module
        .top_level
        .retained_source_tokens_for_statement(unit.ir_module.top_level.statements.last().unwrap())
        .unwrap();
    let args = vec![tokens.argv_texts.last().unwrap().clone()];
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    let call = SinkCall {
        command: "puts",
        args: &args,
        registry,
        source,
        tokens: Some(tokens),
        effective: None,
        source_read: None,
        argument_offset: 0,
    };
    assert!(var_consumed_by_sanitiser(&call, "payload"));
    let missing = SinkCall {
        source: TaintSourceContext::for_input(registry, None, source.config),
        ..call
    };
    assert!(!var_consumed_by_sanitiser(&missing, "payload"));
    let mut stale = source.config;
    stale.strict_quoting = !stale.strict_quoting;
    let stale = SinkCall {
        source: TaintSourceContext::for_input(
            registry,
            unit.top_level.source_metadata_input(),
            stale,
        ),
        ..call
    };
    assert!(!var_consumed_by_sanitiser(&stale, "payload"));
}

#[test]
fn public_module_warning_projection_requires_matching_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // These conditional source warnings retain metadata; no channel read
    // or output handler is executed by the source analysis.
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let mut unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        "set payload [gets stdin]; puts $payload",
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    let input = unit.ir_module.source_metadata_input.clone().unwrap();
    let has_sink = |unit: &crate::compilation_unit::CompilationUnit| {
        let traces = crate::compilation_unit::ModuleTraceFacts {
            traced_variables: &unit.ir_module.traced_variables,
            has_dynamic_variable_trace: unit.ir_module.has_dynamic_variable_trace,
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
        };
        find_taint_warnings_for_module_function(
            &unit.top_level,
            &unit.top_level.taints,
            &unit.top_level.sccp.executable_blocks,
            registry,
            &unit.ir_module,
            &HashSet::<String>::new(),
            traces,
        )
        .iter()
        .any(|warning| warning.code == DiagCode::T101)
    };
    assert!(has_sink(&unit));
    unit.ir_module.source_metadata_input = None;
    assert!(!has_sink(&unit), "missing module owner");
    unit.ir_module.source_metadata_input = Some(input.clone());
    unit.ir_module.lexer_config.strict_quoting = !input.lexer_config().strict_quoting;
    assert!(!has_sink(&unit), "stale module lexer");
    unit.ir_module.lexer_config = input.lexer_config();
    unit.ir_module.dialect_profile = Some(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    assert!(!has_sink(&unit), "stale module profile");
    unit.ir_module.dialect_profile = Some(profile);
    assert!(has_sink(&unit));
}

#[test]
fn taint_warnings_emitted_in_deterministic_proc_order() {
    use crate::compilation_unit::CompilationUnit;
    for (release, registry) in release_registries() {
        // `bbb` is defined *before* `aaa` in source. Per-proc warning order
        // must follow qualified name (aaa, then bbb) — not source order and
        // not the (random) HashMap order — so the output is reproducible.
        let source = "proc bbb {p} { set v [exec $p]\n eval $v }\n\
                      proc aaa {p} { set v [exec $p]\n eval $v }\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert_eq!(
            warnings.len(),
            2,
            "{release}: one eval sink per proc: {warnings:?}"
        );
        // `aaa` is defined later in source (larger span) yet sorts first.
        assert!(
            warnings[0].span.start() > warnings[1].span.start(),
            "{release}: expected name order (aaa before bbb) regardless of source order: {warnings:?}",
        );
    }
}

#[test]
fn hosted_taint_diagnostics_use_current_source_metadata_without_native_handler_claims() {
    // Implementation contract: naming.consumer.hosted-source-taint-descriptor
    // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
    let context =
        tcl_registry::model::ingress::resolve_environment("f5-irules").default_context_registry();
    let registry = context.commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let source = "when HTTP_REQUEST {set x [HTTP::query]; HTTP::respond 200 content $x}";
    let options = crate::compilation_unit::UnitBuildOptions {
        registry,
        defer_top_level: false,
        config,
        dialect: Some(tcl_dialect::DialectProfile::irules()),
        external_call_sites: None,
        declared_commands: None,
    };
    let cu = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        source,
        options,
        None,
        std::sync::Arc::clone(&context),
    )
    .with_interprocedural(registry, Some(tcl_dialect::DialectProfile::irules()));
    let diagnostics = crate::compiler_checks::run_all_checks(
        &cu,
        registry,
        Some(tcl_dialect::DialectProfile::irules()),
    );
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::Irule3001)
        .expect("genuine conditional hosted sink");
    assert_eq!(
        diagnostic.taint_subject().unwrap().sink_command(),
        "HTTP::respond"
    );
    assert_eq!(
        diagnostic.source_context.as_ref().unwrap().image().bytes(),
        source.as_bytes()
    );
    for negative in [
        "when HTTP_REQUEST {HTTP::respond 200 content literal}",
        "when HTTP_REQUEST {HTTP::respond 200 content {$x}}",
        "when HTTP_REQUEST {HTTP::respond 200 content \"$x$tail\"}",
        "proc HTTP::respond {args} {}; when HTTP_REQUEST {HTTP::respond 200 content $x}",
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            negative,
            options,
            None,
            std::sync::Arc::clone(&context),
        );
        let warnings = find_taint_warnings_for_cu(
            &unit,
            registry,
            Some(tcl_dialect::DialectProfile::irules()),
        );
        assert!(
            warnings
                .iter()
                .all(|warning| warning.code != DiagCode::Irule3001),
            "{negative}: {warnings:?}"
        );
    }
    // An unavailable TMM transition does not prove that a later event's
    // command is deleted. Its conditional hosted descriptor remains advice.
    let uncertain = "rename HTTP::respond {}; when HTTP_REQUEST {HTTP::respond 200 content $x}";
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        uncertain,
        options,
        None,
        std::sync::Arc::clone(&context),
    );
    let warnings =
        find_taint_warnings_for_cu(&unit, registry, Some(tcl_dialect::DialectProfile::irules()));
    assert!(
        warnings
            .iter()
            .any(|warning| warning.code == DiagCode::Irule3001)
    );
}

#[test]
fn required_replay_empty_success_differs_from_withdrawn_source_owner() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let (_owner, unit) = retained_warning_unit("set value 1", &context);
    let current = taint_warning_report_for_cu(&unit, registry, None);
    assert_eq!(current.warnings, [] as [crate::taint::TaintWarning; 0]);
    assert!(current.replays.is_complete());
    let mut withdrawn = unit.clone();
    withdrawn.ir_module.retained_source_bindings = None;
    let missing = taint_warning_report_for_cu(&withdrawn, registry, None);
    assert_eq!(missing.warnings, [] as [crate::taint::TaintWarning; 0]);
    assert!(!missing.replays.is_complete());
    assert!(matches!(
        missing.replays,
        TaintReplayAnalysis::Unavailable { .. }
    ));
}

#[test]
fn unavailable_nested_replay_retains_original_site_and_independent_warnings() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let source = "set payload [gets stdin]\nregexp $payload haystack\n[eval $payload]";
    let (_owner, unit) = retained_warning_unit(source, &context);
    let proven = taint_warning_report_for_cu(&unit, registry, None).warnings;
    assert!(proven.iter().any(|warning| warning.code == DiagCode::T103));
    let statement = unit
        .top_level
        .ssa
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find(|statement| {
            statement
                .statement
                .tokens()
                .and_then(|tokens| tokens.word_exprs.first())
                .is_some_and(|word| {
                    let mut children = Vec::new();
                    command_substitutions_in_word(word, &mut children);
                    !children.is_empty()
                })
        })
        .expect("genuine original command-head substitution");
    let word = statement
        .statement
        .tokens()
        .unwrap()
        .word_exprs
        .first()
        .unwrap();
    let mut children = Vec::new();
    command_substitutions_in_word(word, &mut children);
    let (spelling, span) = children.first().unwrap();
    let script = spelling
        .strip_prefix('[')
        .unwrap()
        .strip_suffix(']')
        .unwrap();
    let unavailable = std::cell::RefCell::new(Vec::new());
    let shadowed = HashSet::new();
    let sink = SinkWarningContext {
        registry,
        source: TaintSourceContext::for_input(registry, None, unit.ir_module.lexer_config),
        dialect: unit.ir_module.dialect_profile,
        shadowed_builtins: &shadowed,
        original_module: Some(&unit.ir_module),
        unavailable_replays: Some(&unavailable),
    };
    let mut warnings = proven.clone();
    emit_isolated_script_warnings(
        script,
        *span,
        statement,
        &unit.top_level.taints,
        &mut warnings,
        &unit.top_level.ssa,
        &sink,
        None,
    );
    assert_eq!(warnings, proven);
    let unavailable = unavailable.into_inner();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0].span, *span);
    assert_eq!(
        unavailable[0].reason,
        UnavailableTaintReplayReason::SourceMetadata
    );
    let captured = unavailable[0].source.as_ref().unwrap();
    assert!(captured.matches_module(&unit.ir_module, registry));
    let mut changed = unit.ir_module.clone();
    changed.source = tcl_lexer::SourceImage::document("set payload fixed");
    assert!(!captured.matches_module(&changed, registry));
    let mut changed_entry = unit.ir_module.clone();
    changed_entry.source_entry.unknown_entry = !changed_entry.source_entry.unknown_entry;
    assert!(!captured.matches_module(&changed_entry, registry));
    let mut changed_namespace = unit.ir_module.clone();
    changed_namespace.top_level_namespace = "::different_replay_source".to_owned();
    assert!(!captured.matches_module(&changed_namespace, registry));
    let mut foreign = tcl_registry::CommandRegistry::build_default();
    foreign.insert(tcl_registry::CommandSpec {
        name: "__foreign_replay_store",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    assert!(!captured.matches_module(&unit.ir_module, &foreign));
}

#[test]
fn scaffold_constructors_require_actual_selected_descriptor_identity() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let actual = ScaffoldConstructors::required(registry, None, true, true, true).unwrap();
    assert_eq!(actual.rebinding_priority(registry, None, "proc"), 2);
    assert_eq!(actual.rebinding_priority(registry, None, "::proc"), 2);
    assert_eq!(actual.rebinding_priority(registry, None, "interp"), 1);
    assert_eq!(actual.rebinding_priority(registry, None, "ordinary"), 0);
    let mut replacement = tcl_registry::CommandRegistry::build_default();
    replacement.insert(registry.get("proc").unwrap().clone());
    assert_eq!(actual.rebinding_priority(&replacement, None, "proc"), 0);
    assert!(matches!(
        ScaffoldConstructors::required(registry, None, false, true, false),
        Err(UnavailableTaintReplayReason::SourceMetadata)
    ));
    assert!(ScaffoldConstructors::required(registry, None, false, false, false).is_ok());
    let mut unavailable = tcl_registry::CommandRegistry::build_default();
    unavailable.insert(tcl_registry::CommandSpec {
        state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
        ..registry.get("proc").unwrap().clone()
    });
    assert!(matches!(
        ScaffoldConstructors::required(&unavailable, None, true, true, false),
        Err(UnavailableTaintReplayReason::ProcedureConstructor)
    ));
}

#[test]
fn public_report_keeps_constructor_refusal_beside_an_independent_warning() {
    let baseline =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let mut supplied = tcl_registry::CommandRegistry::build_default();
    supplied.insert(tcl_registry::CommandSpec {
        state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
        ..baseline.commands().get("proc").unwrap().clone()
    });
    let context = std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(supplied)));
    let registry = context.commands();
    let source = "set payload [gets stdin]\nregexp $payload haystack\nrename puts replacement\n[puts $payload]";
    let (_owner, unit) = retained_warning_unit(source, &context);
    let report = taint_warning_report_for_cu(&unit, registry, None);
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.code == DiagCode::T103)
    );
    assert!(!report.replays.is_complete());
    let refusal = report
        .replays
        .unavailable_replays()
        .iter()
        .find(|residual| residual.reason == UnavailableTaintReplayReason::ProcedureConstructor)
        .expect("actual required scaffold constructor is unavailable");
    assert!(
        refusal
            .source
            .as_ref()
            .unwrap()
            .matches_module(&unit.ir_module, registry)
    );
    assert_eq!(
        unit.ir_module
            .source
            .bytes()
            .get(refusal.span.as_range())
            .unwrap(),
        b"[puts $payload]"
    );
}
