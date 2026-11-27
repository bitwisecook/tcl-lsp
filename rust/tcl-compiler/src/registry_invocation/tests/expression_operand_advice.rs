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

//! Original expression contexts and operand advice: sites, availability and refusals.

use super::*;

#[test]
fn original_expression_context_keeps_logical_input_and_written_operands() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let owner =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    config.strict_quoting = !config.strict_quoting;
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&owner),
        config,
    );
    let registry = owner.commands();
    for source in [
        "expr {2 + 3}",
        "rename expr arithmetic; arithmetic {2 + 3}",
        "interp alias {} arithmetic {} expr; arithmetic {2 + 3}",
    ] {
        let tokens = logical_source_tokens(source, registry, &input);
        let advice = original_expression_operand_advice(registry, &tokens)
            .expect("the original Logical expression keeps its selected grammar");
        assert_eq!(advice.expression_text, "2 + 3");
        assert!(advice.lookup_closed);
        assert!(matches!(
            advice.frame,
            crate::var_resolve::VariableExecutionFrame::Unknown
        ));
        assert_eq!(
            source.get(
                advice.expression_base as usize
                    ..advice.expression_base as usize + advice.expression_text.len()
            ),
            Some("2 + 3"),
        );
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .native_compilation_admission
                .is_none()
        );
        let metadata = InvocationMetadataContext::for_analysis_input(registry, &input).unwrap();
        assert!(
            resolved_handler_invocation_with_metadata_context(registry, Some(metadata), &tokens)
                .is_none()
        );
    }
    let replaced = logical_source_tokens(
        "proc expr args {return STUB}; expr {2 + 3}",
        registry,
        &input,
    );
    assert!(original_expression_operand_advice(registry, &replaced).is_none());
    let captured = logical_source_tokens(
        "interp alias {} arithmetic {} expr {2 + 3}; arithmetic",
        registry,
        &input,
    );
    assert!(original_expression_operand_advice(registry, &captured).is_none());
}

#[test]
fn original_expression_context_keeps_availability_and_terminal_point_refusals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let mut registry = CommandRegistry::build_default();
    let mut descriptor = registry.get("expr").unwrap().clone();
    descriptor.name = "arithmetic";
    descriptor.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
    registry.insert(descriptor);
    let registry = std::sync::Arc::new(registry);
    let input_for = |environment| {
        let context = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context.with_command_store(std::sync::Arc::clone(&registry))),
            config,
        )
    };
    let current = input_for("tcl9.1");
    let older = input_for("tcl8.4");
    let source = "arithmetic {2 + 3}";
    let tokens = logical_source_tokens(source, &registry, &current);
    assert!(original_expression_operand_advice(&registry, &tokens).is_some());
    assert_eq!(
        current
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key(),
        older
            .borrowed_context_registry()
            .commands()
            .snapshot()
            .semantic_key()
    );
    let unavailable = logical_source_tokens(source, &registry, &older);
    assert!(original_expression_operand_advice(&registry, &unavailable).is_none());
    let mut missing = tokens.clone();
    missing.source_binding = None;
    assert!(original_expression_operand_advice(&registry, &missing).is_none());
    let mut changed = tokens.clone();
    changed.argv_texts[1] = "4 + 5".to_owned();
    changed.word_exprs[1] = literal("4 + 5");
    assert!(original_expression_operand_advice(&registry, &changed).is_none());
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    assert!(original_expression_operand_advice(foreign.commands(), &tokens).is_none());

    let image = tcl_lexer::SourceImage::document(source);
    let segment =
        tcl_source::segment::segment_commands_image_with_offset_and_config(&image, 0, config)
            .unwrap()
            .remove(0);
    let mut stale_config = config;
    stale_config.strict_quoting = !stale_config.strict_quoting;
    let stale = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        current.context_registry(),
        stale_config,
    );
    for metadata_context in [
        InvocationMetadataInput::SuppliedSource(None),
        InvocationMetadataInput::SuppliedSource(Some(&stale)),
    ] {
        let mut options =
            crate::command_binding::SourceAnalysisOptions::for_logical_source(&current).unwrap();
        options.metadata_context = metadata_context;
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse_image_in_frame_with_options(
                &image,
                &crate::var_resolve::VariableExecutionFrame::Unknown,
                config,
                &registry,
                options,
            )
            .unwrap();
        let mut refused = CommandTokens::from_segmented(&image.source_map(), config, &segment);
        bindings.stamp_original_tokens(&mut refused);
        assert!(original_expression_operand_advice(&registry, &refused).is_none());
    }
}

#[test]
fn expression_operand_advice_keeps_original_sites_and_replaced_handlers_separate() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        ("proc f {i} {set y [expr {$i + 0}]}", 1),
        (
            "interp alias {} arithmetic {} expr; proc f {i} {set y [arithmetic {$i + 0}]}",
            1,
        ),
        (
            "rename expr original_expr; proc expr args {list PLAIN}; proc f {i} {set y [expr {$i + 0}]}",
            0,
        ),
        ("proc f {i} {set y [unknown {$i + 0}]}", 0),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let mut count = 0;
        crate::ir::for_each_statement(
            &unit
                .ir_module
                .procedures
                .get(&fixture_id(&unit.ir_module.procedures, "::f"))
                .expect("authored fixture body")
                .body,
            &mut |statement| {
                for call in crate::word_subst::lifted_calls(
                    statement.tokens(),
                    tcl_lexer::LexerConfig::for_profile(registry.profile()),
                ) {
                    if let Some(tokens) = call.tokens
                        && let Some(advice) = original_expression_operand_advice(registry, &tokens)
                    {
                        assert!(
                            source
                                .get(advice.span.as_range())
                                .is_some_and(|text| text.contains("$i + 0"))
                        );
                        count += 1;
                    }
                }
            },
        );
        assert_eq!(count, expected, "{source}");
    }
}

#[test]
fn default_return_operand_advice_preserves_generic_calls_and_replacements() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, expected) in [
        ("proc make {} {return [list VALUE]}", vec!["[list VALUE]"]),
        ("proc make {} {return -code error VALUE}", vec![]),
        (
            "rename return stock_return; proc return args {list PLAIN}; proc make {} {return [list VALUE]}",
            vec![],
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let procedure = unit
            .ir_module
            .procedures
            .get(&fixture_id(&unit.ir_module.procedures, "::make"))
            .expect("authored fixture body");
        let mut values = Vec::new();
        crate::ir::for_each_statement(&procedure.body, &mut |statement| {
            if let Some(tokens) = procedure
                .body
                .retained_source_tokens_for_statement(statement)
            {
                values.extend(
                    advisory_return_values(registry, tokens)
                        .into_iter()
                        .map(|word| word.legacy_text()),
                );
            }
        });
        assert_eq!(values, expected, "{source}");
    }
}
