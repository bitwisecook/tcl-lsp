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

//! The command a sink statement really runs: constant and colon-run heads, instance receivers, bindings, aliases, renames and shadowed builtins.

use super::*;

#[test]
fn a_colon_run_head_still_names_its_own_global_target() {
    // tclsh 8.6/9.0/9.1: `interp alias {} :::puts {} puts` aliases `::puts`
    // onto itself and deletes it; the next `puts` is `invalid command name`.
    assert!(super::super::global_heads_name_one_command(
        "puts", ":::puts"
    ));
    assert!(super::super::global_heads_name_one_command(
        "::puts", "puts"
    ));
    assert!(!super::super::global_heads_name_one_command(
        "puts", ":puts"
    ));
    assert!(!super::super::global_heads_name_one_command(
        "puts", "ns::puts"
    ));
}

#[test]
fn original_instance_receiver_names_preserve_head_kind_and_actual_grammar() {
    let classes = LocalInstanceClasses {
        at: HashMap::from([(
            42,
            HashMap::from([
                ("$literal".into(), HashSet::from(["LiteralClass".into()])),
                ("${literal}".into(), HashSet::from(["BraceClass".into()])),
                ("literal".into(), HashSet::from(["ReferenceClass".into()])),
                (String::new(), HashSet::from(["EmptyClass".into()])),
                ("arr".into(), HashSet::from(["ScalarClass".into()])),
                ("arr(key)".into(), HashSet::from(["CommandClass".into()])),
                ("a{b}c".into(), HashSet::from(["NestedClass".into()])),
            ]),
        )]),
    };
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let config = TaintSourceContext::standalone(registry, registry.profile()).config;
        for (source, expected) in [
            ("{$literal}", "LiteralClass"),
            ("{${literal}}", "BraceClass"),
            ("$literal", "ReferenceClass"),
            ("${$literal}", "LiteralClass"),
            ("{}", "EmptyClass"),
            ("${}", "EmptyClass"),
            ("arr(key)", "CommandClass"),
        ] {
            let word = crate::value_shapes::value_word_with_config(source, config).unwrap();
            assert_eq!(
                unique_instance_class(Some((&word, config)), Some(&classes), Some(42)),
                Some(expected),
                "{label}: {source}"
            );
            assert!(
                unique_instance_class(Some((&word, config)), Some(&classes), Some(43)).is_none()
            );
        }
        for source in [
            "$arr(key)",
            "${arr(key)}",
            "prefix$literal",
            "[pick]",
            "{*}{literal}",
            "${nul\0name}",
        ] {
            let word = crate::value_shapes::value_word_with_config(source, config).unwrap();
            assert!(
                unique_instance_class(Some((&word, config)), Some(&classes), Some(42)).is_none(),
                "{label}: {source}"
            );
        }
        let nested = crate::value_shapes::value_word_with_config("${a{b}c}", config).unwrap();
        assert_eq!(
            unique_instance_class(Some((&nested, config)), Some(&classes), Some(42)),
            if label == "tcl9.1" {
                Some("NestedClass")
            } else {
                None
            }
        );
    }
    assert!(unique_instance_class(None, Some(&classes), Some(42)).is_none());
}

#[test]
fn constant_command_head_candidates_use_retained_escape_grammar() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // SSA source-value candidates only; no command lookup, target identity
    // or actual invocation follows from the decoded presentation value.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    for (escapes, expected) in [
        (tcl_dialect::EscapeSyntax::Tcl84, "U0001f600"),
        (tcl_dialect::EscapeSyntax::Tcl90, "😀"),
    ] {
        let config = tcl_lexer::LexerConfig {
            escapes,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            "set command \\U0001f600\n$command argument",
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
        let statement = unit.ir_module.top_level.statements.last().unwrap();
        let heads = resolve_taint_command_heads(
            "$command",
            statement,
            None,
            Some(CommandHeadContext {
                fu: &unit.top_level,
                module_traces: crate::compilation_unit::ModuleTraceFacts {
                    traced_variables: &unit.ir_module.traced_variables,
                    has_dynamic_variable_trace: unit.ir_module.has_dynamic_variable_trace,
                    deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
                },
                identities: crate::realm::CommandBindingRealm::none(),
            }),
        );
        assert_eq!(
            heads
                .iter()
                .map(|head| head.command.as_str())
                .collect::<Vec<_>>(),
            [expected]
        );
    }
}

#[test]
fn explicit_unknown_binding_cannot_recover_a_same_spelled_sink() {
    let source = "eval $x";
    let command = tcl_source::segment::segment_commands(source).remove(0);
    let tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        tcl_lexer::LexerConfig::default(),
        &command,
    )
    .with_source_binding(crate::command_binding::SourceInvocationBinding::unknown());
    let mut statement = call_stmt("eval", &["$x"]);
    match &mut statement {
        Statement::Call { tokens: target, .. } => *target = Some(tokens),
        _ => panic!("call fixture"),
    }
    let warnings = warnings_for_tainted_sink(statement, &[("x", 1)]);
    assert!(
        warnings
            .iter()
            .all(|warning| warning.sink_command != "eval"),
        "{warnings:?}"
    );
}

#[test]
fn point_binding_supplies_sink_alias_prefix_once() {
    for (release, registry) in release_registries() {
        let module = crate::lowering::lower_to_ir(
            "interp alias {} invoke {} exec safe-program\ninvoke $x",
            &registry,
        );
        let tokens = module
            .top_level
            .statements
            .last()
            .unwrap()
            .tokens()
            .unwrap();
        let statement = call_stmt("invoke", &["$x"]);
        let heads = resolve_taint_command_heads("invoke", &statement, Some(tokens), None);
        assert_eq!(heads.len(), 1, "{release}");
        assert_eq!(heads[0].command, "::exec", "{release}");
        assert_eq!(
            heads[0].effective_args(&["$x".into()]),
            Some(vec!["safe-program".into(), "$x".into()]),
            "{release}",
        );
        let arguments = vec!["safe-program".into(), "$x".into()];
        let call = SinkCall {
            source: TaintSourceContext::standalone(&registry, registry.profile()),
            command: &heads[0].command,
            args: &arguments,
            registry: &registry,
            tokens: Some(tokens),
            effective: None,
            source_read: None,
            argument_offset: 0,
        };
        assert_eq!(
            sink_arg_span(&call, "x", statement.span()),
            tokens.argv[1],
            "{release}",
        );
    }
}

/// TP: `interp alias {} myEval {} eval; myEval $x` still raises T100 —
/// `classify_sink` dispatches on `Statement::canonical_command_or_source()`,
/// so the alias resolves to `eval` even though the source spells the call
/// `myEval`, which is not itself a registered command.
#[test]
fn t100_fires_through_interp_alias_indirection() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "interp alias {} myEval {} eval\n\
                       set x [gets stdin]\n\
                       myEval $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 through the `myEval` alias for `eval`, got {warnings:?}",
    );
}

/// TP: `rename eval myEval; myEval $x` also raises T100 — the same
/// `CommandAliasMap` mechanism `interp alias` uses now also recognises
/// a static `rename oldName newName` (`detect_rename`), so a renamed
/// built-in resolves through `canonical_command_or_source()` exactly
/// like an `interp alias`.
#[test]
pub(super) fn t100_fires_through_rename_indirection() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let source = "rename eval myEval\n\
                       set x [gets stdin]\n\
                       myEval $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 through the `myEval` rename of `eval`, got {warnings:?}",
    );
}

/// TN: without the alias, a plain user command named `myEval` is not a
/// sink at all — proves the alias fix above isn't matching on the bare
/// name.
#[test]
pub(super) fn t100_silent_for_unaliased_lookalike_command() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "proc myEval {x} { return $x }\n\
                       set x [gets stdin]\n\
                       myEval $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "a user proc named myEval must not be treated as the eval sink, got {warnings:?}",
    );
}

/// An iRules event handler is a body with no command slot. A unit that
/// holds one still answers which builtins its procedures shadow, for the
/// file level, the procedure and the handler alike.
#[test]
fn an_event_handler_body_shadows_no_builtin() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for(
        "proc log {args} {}\nwhen HTTP_REQUEST {set x 1}\n",
        registry,
        false,
    );
    let mut handler = false;
    for function in unit.all_body_function_units() {
        let shadowed = shadowed_builtin_names_for_module_function(function, &unit, registry)
            .unwrap_or_else(|| panic!("{} has no answer", function.name));
        assert!(shadowed.contains("log"), "{}: {shadowed:?}", function.name);
        handler |= function.name.contains("HTTP_REQUEST");
    }
    assert!(handler, "the handler body is one of the unit's functions");
}
