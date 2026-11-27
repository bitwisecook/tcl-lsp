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

use super::*;
use crate::compilation_unit::CompilationUnit;
use crate::ir::Statement;

fn original_logical_tokens(
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> CommandTokens {
    let image = tcl_lexer::SourceImage::document(source);
    original_logical_tokens_in_image(&image, input)
}

fn original_logical_tokens_in_image(
    image: &tcl_lexer::SourceImage,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> CommandTokens {
    let config = input.lexer_config();
    let registry = input.borrowed_context_registry().commands();
    let bindings =
        crate::command_binding::SourceCommandBindings::analyse_image_in_frame_with_options(
            image,
            &crate::var_resolve::VariableExecutionFrame::Unknown,
            config,
            registry,
            crate::command_binding::SourceAnalysisOptions::for_logical_source(input).unwrap(),
        )
        .unwrap();
    let segments =
        tcl_source::segment::segment_commands_image_with_offset_and_config(image, 0, config)
            .unwrap();
    let [segment] = segments.as_slice() else {
        panic!("one original command")
    };
    let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, segment);
    bindings.stamp_original_tokens(&mut tokens);
    tokens
}

#[test]
fn original_dependency_regions_refuse_captured_and_expanded_role_ordinals() {
    let mut analyser = crate::analyser::Analyser::new();
    let input = analyser.prepare_analysis_input("tcl").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let source = "interp alias {} calculate {} expr {[dict size DATA]}; calculate";
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .with_resolved_analysis_input(input.clone());
    let module = lowerer.lower(source);
    let script = &module.top_level;
    let tokens = script
        .retained_source_tokens_for_statement(script.statements.last().unwrap())
        .unwrap();
    let roles = crate::registry_invocation::original_source_name_roles(registry, tokens)
        .expect("actual selected alias source roles");
    assert!(!roles.direct_written_arguments());
    let plan = tcl_lexer::native_script_words_in(
        module.source.clone(),
        Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    let words = &plan.commands.last().unwrap().words;
    let inventory =
        original_source_dependency_regions(tokens, words, registry, Some(input.analyser_profile()))
            .unwrap();
    assert!(inventory.unknown);
    assert!(inventory.substitutions.is_empty() && inventory.scripts.is_empty());
    for source in ["expr {*}{[dict size DATA]}", "expr {*}$arguments"] {
        let tokens = original_logical_tokens(source, &input);
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        assert!(
            plan.commands[0]
                .words
                .iter()
                .any(|word| word.group().expand)
        );
        assert!(
            original_source_dependency_regions(
                &tokens,
                &plan.commands[0].words,
                registry,
                Some(input.analyser_profile())
            )
            .unwrap()
            .unknown
        );
    }
}

#[test]
fn original_expression_substitutions_keep_braced_string_operands_inert() {
    let mut analyser = crate::analyser::Analyser::new();
    let input = analyser.prepare_analysis_input("tcl").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    for (source, expected) in [
        ("expr {{[literal]} eq {[also_literal]}}", 0),
        ("expr {[info exists value] + 1}", 1),
        ("expr {\"[info exists value]\" eq {literal}}", 1),
    ] {
        let tokens = original_logical_tokens(source, &input);
        let calls = checked_original_lifted_calls_with_metadata_context(
            &tokens, config, registry, metadata,
        )
        .expect("actual original configured expression inventory");
        assert_eq!(calls.len(), expected, "{source}");
        let mut changed = tokens.clone();
        changed.argv_texts[1].push_str(" changed");
        assert!(
            checked_original_lifted_calls_with_metadata_context(
                &changed, config, registry, metadata
            )
            .is_none()
        );
    }
}

#[test]
fn original_dependency_regions_keep_index_children_and_expression_data_purpose() {
    let mut analyser = crate::analyser::Analyser::new();
    let input = analyser.prepare_analysis_input("tcl").unwrap();
    let config = input.lexer_config();
    let registry = input.borrowed_context_registry().commands();
    for (source, expected) in [
        ("puts {[literal]}", vec![]),
        ("puts $array([selected])", vec!["selected"]),
        ("puts [first; second]", vec!["first; second"]),
        ("expr {{[literal]} eq {[also_literal]}}", vec![]),
        (
            "expr {[selected] + $array([index])}",
            vec!["selected", "index"],
        ),
        ("expr {\"[selected]\" eq {literal}}", vec!["selected"]),
    ] {
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let tokens = original_logical_tokens(source, &input);
        let inventory = original_source_dependency_regions(
            &tokens,
            &plan.commands[0].words,
            registry,
            Some(input.analyser_profile()),
        )
        .unwrap();
        let mut actual = inventory
            .substitutions
            .iter()
            .map(|span| source.get(span.as_range()).unwrap())
            .collect::<Vec<_>>();
        actual.sort_unstable();
        let mut expected = expected;
        expected.sort_unstable();
        assert_eq!(actual, expected, "{source}");
        let mut changed = tokens.clone();
        changed.argv_texts[0].push_str(" changed");
        assert!(
            original_source_dependency_regions(
                &changed,
                &plan.commands[0].words,
                registry,
                Some(input.analyser_profile())
            )
            .is_none()
        );
    }
    let source = "apply {{} {puts [selected]}}";
    let image = tcl_lexer::SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image,
        Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    let tokens = original_logical_tokens(source, &input);
    assert!(
        original_source_dependency_regions(
            &tokens,
            &plan.commands[0].words,
            registry,
            Some(input.analyser_profile())
        )
        .unwrap()
        .unknown
    );
    let source = "expr {[selected]}";
    let image = tcl_lexer::SourceImage::document(source);
    let plan = tcl_lexer::native_script_words_in(
        image,
        Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    let tokens = original_logical_tokens(source, &input);
    assert!(
        original_source_dependency_regions(&tokens, &plan.commands[0].words, registry, None)
            .unwrap()
            .unknown
    );
    let mut missing = tokens;
    missing.source_binding = None;
    assert!(
        original_source_dependency_regions(
            &missing,
            &plan.commands[0].words,
            registry,
            Some(input.analyser_profile())
        )
        .unwrap()
        .unknown
    );
}

#[test]
fn single_command_parts_keep_selected_grammar_and_original_word_geometry() {
    let tcl = tcl_lexer::LexerConfig::default();
    let jim = tcl_lexer::LexerConfig::for_file_grammar(
        tcl_registry::model::ingress::static_context_for("jimtcl")
            .commands()
            .profile()
            .expect("selected Jim fixture profile")
            .grammar,
    );
    let tcl_parts = single_command_source_parts(
        "[list a\u{b}b]",
        tcl,
        SingleCommandSourcePurpose::CommandSubstitution,
    )
    .unwrap();
    let jim_parts = single_command_source_parts(
        "[list a\u{b}b]",
        jim,
        SingleCommandSourcePurpose::CommandSubstitution,
    )
    .unwrap();
    assert_eq!(
        tcl_parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>(),
        ["list", "a", "b"]
    );
    assert_eq!(
        jim_parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<Vec<_>>(),
        ["list", "a\u{b}b"]
    );
    let parts = single_command_source_parts(
        r#"[list {a [inert]} "prefix[set x {]}]suffix" {*}$args]"#,
        tcl,
        SingleCommandSourcePurpose::CommandSubstitution,
    )
    .unwrap();
    assert_eq!(
        parts,
        vec![
            SingleCommandSourcePart {
                text: "list".into(),
                braced: false,
                expand: false
            },
            SingleCommandSourcePart {
                text: "a [inert]".into(),
                braced: true,
                expand: false
            },
            SingleCommandSourcePart {
                text: "prefix[set x {]}]suffix".into(),
                braced: false,
                expand: false
            },
            SingleCommandSourcePart {
                text: "$args".into(),
                braced: false,
                expand: true
            },
        ]
    );
    assert!(
        single_command_source_parts(
            "[list {missing]",
            tcl,
            SingleCommandSourcePurpose::CommandSubstitution
        )
        .is_none()
    );
    assert!(
        single_command_source_parts(
            "[list a; list b]",
            tcl,
            SingleCommandSourcePurpose::CommandSubstitution
        )
        .is_none()
    );
    assert!(
        single_command_source_parts(
            r#"[list "a"b]"#,
            tcl,
            SingleCommandSourcePurpose::CommandSubstitution
        )
        .is_none()
    );
    let script =
        single_command_source_parts("[set x]", tcl, SingleCommandSourcePurpose::Script).unwrap();
    assert_eq!(script[0].text, "[set x]");
    let substitution = single_command_source_parts(
        "[set x]",
        tcl,
        SingleCommandSourcePurpose::CommandSubstitution,
    )
    .unwrap();
    assert_eq!(substitution[0].text, "set");
}

#[test]
fn original_nested_word_parser_keeps_body_bom_parent_metadata_and_child_geometry() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Conditional original geometry/inventory, never Native child dispatch or result.
    let mut prepared = crate::analyser::Analyser::new();
    // The authored Logical owner retains this independently selected grammar.
    let base_input = prepared.prepare_analysis_input("tcl").unwrap();
    let selected_config = tcl_lexer::LexerConfig::for_file_grammar(
        tcl_dialect::DialectProfile::find("tcl9.1").unwrap().grammar,
    );
    for leading_bom in [tcl_lexer::LeadingBom::Skip, tcl_lexer::LeadingBom::Content] {
        let mut config = selected_config;
        config.leading_bom = leading_bom;
        let input = crate::analyser::ResolvedAnalysisInput::new(
            base_input.analyser_profile(),
            base_input.unit_profile(),
            base_input.context_registry(),
            config,
        );
        let config = input.lexer_config();
        let registry = input.borrowed_context_registry().commands();
        let source = "puts [\u{feff}unknown argument]";
        let tokens = original_logical_tokens(source, &input);
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, &input,
        )
        .unwrap();
        let calls = checked_original_lifted_calls_with_metadata_context(
            &tokens, config, registry, metadata,
        )
        .unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].command, "\u{feff}unknown");
        let child = calls[0].tokens.as_ref().unwrap();
        assert_eq!(child.argv_texts, ["\u{feff}unknown", "argument"]);
        assert_eq!(child.argv[0].start(), 6);
        assert_eq!(
            source.get(child.argv[0].as_range()),
            Some("\u{feff}unknown")
        );
        let site = child
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap();
        assert_eq!(site.offset, 6);
        assert_eq!(site.source.source_image().bytes(), source.as_bytes());
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(
            checked_original_lifted_calls_with_metadata_context(
                &tokens, changed, registry, metadata
            )
            .is_none()
        );
        let mut missing = tokens.clone();
        missing.nested_bindings_mut().clear();
        assert!(
            checked_original_lifted_calls_with_metadata_context(
                &missing, config, registry, metadata
            )
            .is_none()
        );
    }
    // The explicit opaque utility keeps selected non-BOM grammar axes;
    // a span alone cannot manufacture an original source point.
    let spelling = "[\u{feff}unknown ${outer{inner}}]";
    let site = SourceSite::opaque(Span::new(0, u32::try_from(spelling.len()).unwrap()));
    let nested = nested_command_words(spelling, &site, base_input.lexer_config()).unwrap();
    assert!(matches!(nested.words()[1], WordExpr::Variable { .. }));
    let config = tcl_lexer::LexerConfig {
        braced_var: tcl_lexer::BracedVarStyle::FirstClose,
        ..base_input.lexer_config()
    };
    let first_close = nested_command_words(spelling, &site, config).unwrap();
    assert!(matches!(first_close.words()[1], WordExpr::Template { .. }));
    assert!(first_close.source_binding.is_none());
    assert!(
        first_close
            .words()
            .iter()
            .all(|word| word.source().provenance == Provenance::Opaque)
    );
}

#[test]
fn original_extracted_surface_keeps_bom_before_comment_and_no_point_receipt() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Lexical extracted-surface utility; no original point/ExprBool/Normal receipt.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl9.1").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let surface = DocumentCommandSurface::new(registry, None);
    let whole = "expr {\u{feff}# [dict size DATA]}";
    let text = whole
        .strip_prefix("expr {")
        .unwrap()
        .strip_suffix('}')
        .unwrap();
    let calls = lifted_calls_in_text(text, Some(6), input.lexer_config(), &surface);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command, "dict");
    assert_eq!(calls[0].args, ["size", "DATA"]);
    assert_eq!(
        whole.get(calls[0].span.as_range()),
        Some("[dict size DATA]")
    );
    assert!(calls[0].tokens.as_ref().unwrap().source_binding.is_none());
}

#[test]
fn original_nested_call_inventory_keeps_actual_roles_and_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source geometry only: no handler, body entry or result is proved.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&current),
        config,
    );
    let source = "expr {[dict size [dict create key value]]}";
    let tokens = original_logical_tokens(source, &input);
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    let calls =
        checked_original_lifted_calls_with_metadata_context(&tokens, config, registry, metadata)
            .unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].args.first().map(String::as_str), Some("create"));
    assert_eq!(calls[1].args.first().map(String::as_str), Some("size"));
    assert!(
        calls
            .iter()
            .all(|call| call.tokens.as_ref().is_some_and(|tokens| {
                tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| binding.invocation_site())
                    .is_some()
            }))
    );
    let older = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(registry)),
    );
    let older_input = crate::analyser::ResolvedAnalysisInput::new(profile, profile, older, config);
    let older_metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
        registry,
        &older_input,
    )
    .unwrap();
    assert!(
        checked_original_lifted_calls_with_metadata_context(
            &tokens,
            config,
            registry,
            older_metadata,
        )
        .is_none()
    );
    assert!(
        checked_original_lifted_calls_with_metadata_context(
            &tokens,
            config,
            registry,
            current.as_ref().into(),
        )
        .is_none()
    );
    let foreign =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    assert!(
        checked_original_lifted_calls_with_metadata_context(
            &tokens,
            config,
            registry,
            foreign.as_ref().into(),
        )
        .is_none()
    );
}

#[test]
fn original_logical_inventory_uses_source_roles_without_native_role_completion() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional original geometry; no Native frame/handler or result.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    let tokens = original_logical_tokens("uplevel #0 {puts VALUE}", &input);
    let selected =
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            registry, metadata, &tokens,
        )
        .unwrap();
    assert!(
        !selected.facts.arg_roles_complete,
        "Native frame grammar is independently unavailable"
    );
    let realm = tokens
        .source_binding
        .as_ref()
        .unwrap()
        .original_source_realm_for_input(&tokens, &input, registry)
        .unwrap();
    let roles = selected
        .with_metadata_schema(registry, metadata, realm, |schema| {
            schema.authored_logical_source_role_projection()
        })
        .unwrap();
    assert!(
        roles
            .argument_roles()
            .contains(&(1, tcl_registry::ArgRole::Body))
    );
    assert_eq!(
        checked_original_lifted_calls_with_metadata_context(&tokens, config, registry, metadata)
            .unwrap(),
        [] as [crate::word_subst::LiftedCall; 0]
    );
    assert!(selected.dialect.unwrap().native_name_protocol().is_none());

    // The genuine written Expr producer retains its original child receipt.
    let tokens = original_logical_tokens("expr {[dict size DATA]}", &input);
    let calls =
        checked_original_lifted_calls_with_metadata_context(&tokens, config, registry, metadata)
            .unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command, "dict");
    assert_eq!(calls[0].args, ["size", "DATA"]);
    assert_eq!(calls[0].span.start(), 6);
    assert_eq!(
        calls[0]
            .tokens
            .as_ref()
            .unwrap()
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .unwrap()
            .source
            .source_image()
            .bytes(),
        b"expr {[dict size DATA]}"
    );
}

#[test]
fn original_logical_inventory_keeps_alias_ordinals_and_uncertain_geometry_refusals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    for (source, expected) in [
        (
            "interp alias {} calculate {} expr; calculate {[dict size DATA]}",
            true,
        ),
        (
            "interp alias {} calculate {} expr {[dict size DATA]}; calculate",
            false,
        ),
        ("proc expr args {}; expr {[dict size DATA]}", false),
        ("switch $option $cmd {foo {[dict size DATA]}}", false),
        ("regexp $option PATTERN TEXT", false),
        ("expr {*}$arguments", false),
    ] {
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input.clone());
        let module = lowerer.lower(source);
        let script = &module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let calls =
            checked_original_lifted_calls_with_metadata_context(tokens, config, registry, metadata);
        assert_eq!(calls.is_some(), expected, "{source}");
        if expected {
            let calls = calls.unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].command, "dict");
            assert_eq!(
                calls[0]
                    .tokens
                    .as_ref()
                    .unwrap()
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .invocation_site()
                    .unwrap()
                    .source
                    .source_image()
                    .bytes(),
                source.as_bytes()
            );
            let mut missing = tokens.clone();
            missing.nested_bindings_mut().clear();
            assert!(
                checked_original_lifted_calls_with_metadata_context(
                    &missing, config, registry, metadata
                )
                .is_none()
            );
        }
    }
}

#[test]
fn original_nested_call_inventory_withdraws_changed_geometry_and_missing_sidecars() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&current),
        config,
    );
    let tokens = original_logical_tokens("expr {[dict size [dict create key value]]}", &input);
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    assert!(
        checked_original_lifted_calls_with_metadata_context(&tokens, config, registry, metadata,)
            .is_some()
    );
    let mut changed_config = config;
    changed_config.expand_syntax = !changed_config.expand_syntax;
    assert!(
        checked_original_lifted_calls_with_metadata_context(
            &tokens,
            changed_config,
            registry,
            metadata,
        )
        .is_none()
    );
    let mut missing = tokens.clone();
    missing.nested_bindings_mut().clear();
    assert!(
        checked_original_lifted_calls_with_metadata_context(&missing, config, registry, metadata,)
            .is_none()
    );
    let mut changed = tokens;
    let WordExpr::BracedLiteral { text, .. } = &mut changed.word_exprs[1] else {
        panic!("original braced expression")
    };
    *text = "[dict size OTHER]".into();
    assert!(
        checked_original_lifted_calls_with_metadata_context(&changed, config, registry, metadata,)
            .is_none()
    );
}

fn entered_operand_tokens(
    source: &str,
    command_index: usize,
    dialect: tcl_native_model::InvocationDialect,
) -> (
    CommandTokens,
    tcl_registry::CommandRegistry,
    tcl_lexer::LexerConfig,
) {
    let registry = registry();
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let bindings = crate::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        &registry,
        crate::command_binding::SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..crate::command_binding::SourceAnalysisOptions::default()
        },
    );
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &commands[command_index],
    );
    bindings.stamp_original_tokens(&mut tokens);
    (tokens, registry, config)
}

#[test]
fn entered_expression_operand_survives_absent_parent_dispatch() {
    let source = "return [expr {1 / 0}]";
    let actual = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_6),
    );
    let (tokens, registry, config) = entered_operand_tokens(source, 0, actual);
    assert_ne!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .runtime_reachability(),
        crate::command_binding::SourceRuntimeReachability::Reached
    );
    let evaluations = entered_expression_evaluations(Some(&tokens), config, &registry);
    assert_eq!(evaluations.len(), 1);
    let evaluation = &evaluations[0];
    assert_eq!(
        &source[evaluation.expression_base as usize..evaluation.expression_base as usize + 5],
        "1 / 0"
    );
    assert_eq!(&source[evaluation.span.as_range()], "expr {1 / 0}");
    assert_eq!(evaluation.numbers, actual.numbers);
}

#[test]
fn entered_expression_operands_decline_unreached_and_replaced_evaluators() {
    let actual = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_6),
    );
    for (source, command) in [
        ("error STOP; return [expr {1 / 0}]", 1),
        ("return [error STOP] [expr {1 / 0}]", 0),
        (
            "rename expr saved; proc expr args {return CUSTOM}; return [expr {1 / 0}]",
            2,
        ),
        ("return {[expr {1 / 0}]}", 0),
    ] {
        let (tokens, registry, config) = entered_operand_tokens(source, command, actual);
        assert!(
            entered_expression_evaluations(Some(&tokens), config, &registry).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn entered_expression_operands_retain_actual_grammar_and_original_bytes() {
    let actual = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_4),
    );
    let (tokens, registry, config) = entered_operand_tokens("return [expr {1 / 0o0}]", 0, actual);
    let evaluations = entered_expression_evaluations(
        Some(&tokens),
        tcl_lexer::LexerConfig::for_dialect("tcl9.1"),
        &registry,
    );
    assert_eq!(evaluations.len(), 1);
    assert_eq!(evaluations[0].numbers, tcl_dialect::NumberSyntax::Tcl84);
    assert!(matches!(
        evaluations[0].expression,
        crate::expr_ast::ExprNode::Raw { .. }
    ));

    let (tokens, registry, _) = entered_operand_tokens("return [expr {1 / 0}]", 0, actual);
    let mut child = lifted_calls(Some(&tokens), config)
        .pop()
        .unwrap()
        .tokens
        .unwrap();
    let WordExpr::BracedLiteral { text, .. } = &mut child.word_exprs[1] else {
        panic!("original expression word");
    };
    *text = "5 / 0".into();
    assert!(entered_expression_evaluation(&child, &registry).is_none());
}

#[test]
fn actual_entry_grammar_precedes_assistance_during_lowering_and_nested_lifting() {
    let registry = registry();
    let assistance = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let jim = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    let mut lowerer = crate::lowering::Lowerer::with_config(&registry, assistance);
    lowerer.set_source_analysis_options(crate::command_binding::SourceAnalysisOptions {
        invocation_dialect: Some(jim),
        ..crate::command_binding::SourceAnalysisOptions::default()
    });
    let module = lowerer.lower("puts $café [puts $café]");
    assert_eq!(
        module.native_lexer_config(),
        assistance.with_grammar(jim.lexer_grammar)
    );
    let tokens = module.top_level.statements[0]
        .tokens()
        .expect("original words");
    assert_eq!(
        tokens.words()[1]
            .sole_variable_substitution()
            .map(|(text, _)| text),
        Some("$café")
    );
    let lifted = super::lifted_calls(Some(tokens), assistance);
    assert_eq!(lifted.len(), 1);
    assert_eq!(
        lifted[0].arg_words[0]
            .sole_variable_substitution()
            .map(|(text, _)| text),
        Some("$café")
    );
    let nested = lifted[0].tokens.as_ref().expect("nested source carrier");
    assert_eq!(
        nested.native_lexer_config(assistance),
        assistance.with_grammar(jim.lexer_grammar)
    );
}

#[test]
fn unknown_execution_dialect_keeps_the_supplied_lexical_policy() {
    let registry = registry();
    let config = tcl_lexer::LexerConfig::for_dialect("jim");
    let mut lowerer = crate::lowering::Lowerer::with_config(&registry, config);
    lowerer.set_source_analysis_options(crate::command_binding::SourceAnalysisOptions::default());
    let module = lowerer.lower("puts $café");
    assert_eq!(module.native_lexer_config(), config);
    let tokens = module.top_level.statements[0]
        .tokens()
        .expect("original words");
    assert_eq!(tokens.native_lexer_config(config), config);
    assert_eq!(
        tokens.words()[1]
            .sole_variable_substitution()
            .map(|(text, _)| text),
        Some("$café")
    );
}

fn registry() -> tcl_registry::CommandRegistry {
    tcl_registry::CommandRegistry::build_default()
}

/// Every substitution lifted from the single `Call` in a one-statement proc.
fn lift_calls(body: &str) -> Vec<LiftedCall> {
    let reg = registry();
    let src = format!("proc f {{x}} {{\n {body}\n}}");
    let cu = CompilationUnit::build_for(&src, &reg, false);
    let fu = cu.function("::f").expect("proc lowered");
    let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
    let mut out = Vec::new();
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call { tokens, .. } = stmt {
                out.extend(lifted_calls(tokens.as_ref(), config));
            }
        }
    }
    out
}

/// Lift the substitutions of the single `Call` in a one-statement proc.
fn lift(body: &str) -> Vec<(String, Vec<String>)> {
    lift_calls(body)
        .into_iter()
        .map(|c| (c.command, c.args))
        .collect()
}

/// The whole point of reading `word_exprs` rather than the argument text:
/// Tcl substitutes `[…]` in a bare or quoted word and **not** in a braced
/// one, but `Statement::Call::args` renders all three identically as
/// `[lindex $x 0]`. Lifting from that text would report a command Tcl
/// never runs.
#[test]
fn braced_word_is_not_a_substitution() {
    assert_eq!(
        lift("puts [lindex $x 0]"),
        vec![("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()])],
        "a bare `[…]` word runs"
    );
    assert_eq!(
        lift("puts \"[lindex $x 0]\""),
        vec![("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()])],
        "a quoted `[…]` runs too"
    );
    assert_eq!(
        lift("puts {[lindex $x 0]}").len(),
        0,
        "a braced `[…]` is literal text, never run"
    );
}

/// Lift with a registry, so brace-quoted expression words are descended.
fn lift_with_registry(body: &str) -> Vec<(String, Vec<String>)> {
    let reg = registry().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile(),
    );
    lift_with_selected_registry(body, &reg)
}

fn lift_with_selected_registry(
    body: &str,
    reg: &tcl_registry::CommandRegistry,
) -> Vec<(String, Vec<String>)> {
    let surface = tcl_registry::model::DocumentCommandSurface::new(reg, None);
    let src = format!("proc f {{x}} {{\n {body}\n}}");
    let cu = CompilationUnit::build_for(&src, reg, false);
    let fu = cu.function("::f").expect("proc lowered");
    let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
    let mut out = Vec::new();
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            let tokens = match stmt {
                Statement::Call { tokens, .. }
                | Statement::Barrier { tokens, .. }
                | Statement::AssignValue { tokens, .. } => tokens.as_ref(),
                _ => None,
            };
            out.extend(
                lifted_calls_with_surface(tokens, config, &surface)
                    .into_iter()
                    .map(|c| (c.command, c.args)),
            );
        }
    }
    out
}

/// A brace-quoted word is inert to the command parser but not to `expr`,
/// which re-parses it as an expression and runs the `[…]` in it. Which
/// words those are is the registry's answer, so only the registry-aware
/// lift takes them — `lifted_calls` has no way to tell `puts {[f]}` from
/// `expr {[f]}`.
#[test]
fn a_braced_expression_word_runs_its_substitutions() {
    assert_eq!(
        lift_with_registry("puts [expr {$x + [incr x]}]"),
        vec![
            ("incr".to_owned(), vec!["x".to_owned()]),
            ("expr".to_owned(), vec!["{$x + [incr x]}".to_owned()]),
        ],
        "the `incr` runs, and innermost-first puts it before its `expr`"
    );
    assert!(
        lift("puts [expr {$x + [incr x]}]")
            .iter()
            .all(|(command, _)| command != "incr"),
        "without a registry the braced word stays literal"
    );
}

/// `expr` concatenates its words into one expression before parsing it, so
/// a braced word anywhere in the list contributes to it. tclsh 8.6.18 and
/// 9.0.4 both print `3` then `2` for
/// `set x 1; puts [expr 1 + {[incr x]}]; puts $x`.
#[test]
fn a_concatenated_expression_word_runs_its_substitutions() {
    assert_eq!(
        lift_with_registry("puts [expr 1 + {[incr x]}]"),
        vec![
            ("incr".to_owned(), vec!["x".to_owned()]),
            (
                "expr".to_owned(),
                vec!["1".to_owned(), "+".to_owned(), "{[incr x]}".to_owned()],
            ),
        ]
    );
}

#[test]
fn jim_does_not_evaluate_a_rejected_variadic_expression_word() {
    let reg = registry().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
    );
    assert_eq!(
        lift_with_selected_registry("puts [expr 1 + {[incr x]}]", &reg),
        vec![(
            "expr".to_owned(),
            vec!["1".to_owned(), "+".to_owned(), "{[incr x]}".to_owned()],
        )],
    );
}

/// The descent is the registry's rule and not the brace: a braced word a
/// command does *not* evaluate as an expression is still literal text.
#[test]
fn a_braced_word_that_is_not_an_expression_stays_literal() {
    assert_eq!(
        lift_with_registry("puts {[lindex $x 0]}").len(),
        0,
        "`puts` reads its word as a value, so the `[…]` never runs"
    );
    assert!(
        lift_with_registry("puts [list {[lindex $x 0]}]")
            .iter()
            .all(|(command, _)| command != "lindex"),
        "nor does `list`'s"
    );
}

/// A `"…"` operand substitutes inside an expression; a `{…}` one does
/// not. Reading every [`crate::expr_ast::ExprNode::String`] as inert
/// missed the call in `set r [expr {"[id 9]"}]`: tclsh 8.6.18 prints
/// `7` then `9` and `--profile standard` folded `id`'s body to
/// `return 7`, printing `7` twice.
#[test]
fn a_quoted_expression_operand_runs_its_substitutions() {
    let reg = registry();
    let surface = tcl_registry::model::DocumentCommandSurface::new(&reg, None);
    let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
    for (why, source, expected) in [
        (
            "a quoted operand substitutes",
            r#""[id 9]""#,
            vec!["id".to_owned()],
        ),
        ("a braced one does not", "{[id 9]}", Vec::new()),
        (
            "and so does one welded into a larger expression",
            r#""[id 9]" eq "9""#,
            vec!["id".to_owned()],
        ),
    ] {
        let expr = tcl_syntax::expr::parser::parse_expr_for_profile(source, reg.profile());
        assert_eq!(
            lifted_calls_in_expr(&expr, None, config, &surface)
                .iter()
                .map(|c| c.command.clone())
                .collect::<Vec<_>>(),
            expected,
            "{why}"
        );
    }
}

/// A substitution recovered from inside a brace-quoted word is anchored
/// back to the real source byte it came from.
///
/// Two offsets have to be right at once and neither shows up as wrong
/// behaviour, only as a wrong position: the word's site covers the `{…}`
/// that quoted it, so its content starts one byte in, and a lexer `Cmd`
/// token's span stops at the last byte *inside* the brackets, so the
/// closing `]` is taken back before parsing. The second was load-bearing
/// — without it the spelling came back as `[id 9` and every such
/// substitution was silently declined — and the first is invisible until
/// a consumer reports a position.
#[test]
fn a_substitution_inside_a_braced_word_keeps_its_source_offset() {
    let reg = registry();
    let surface = tcl_registry::model::DocumentCommandSurface::new(&reg, None);
    let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
    // `puts [expr {…}]` rather than a bare `expr {…}`: the latter fuses
    // into an `ExprEval` that keeps no words at all, which is the
    // `lifted_calls_in_expr` path rather than this one.
    let src = "proc f {x} {\n puts [expr {[id 9] > 5}]\n}";
    let want = src.find("[id 9]").expect("the call is in the source");
    let cu = CompilationUnit::build_for(src, &reg, false);
    let fu = cu.function("::f").expect("proc lowered");
    let lifted: Vec<LiftedCall> = fu
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(|stmt| match stmt {
            Statement::Call { tokens, .. } => tokens.as_ref(),
            _ => None,
        })
        .flat_map(|tokens| lifted_calls_with_surface(Some(tokens), config, &surface))
        .collect();

    // Both are real: `[expr …]` is a substitution the `puts` word runs,
    // and `id` is one the expression inside it runs.
    let mut names: Vec<String> = lifted.iter().map(|c| c.command.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["expr".to_owned(), "id".to_owned()]);
    let span = lifted
        .iter()
        .find(|c| c.command == "id")
        .expect("the nested call is lifted")
        .span;
    assert_eq!(
        (span.start() as usize, span.end() as usize),
        (want, want + "[id 9]".len()),
        "the span covers `[id 9]` exactly, brackets included: src[{}..{}] is {:?}",
        span.start(),
        span.end(),
        src.get(span.start() as usize..span.end() as usize)
    );
}

/// An expression surface with no words of its own — what a fused
/// `AssignExpr` or a `Branch` terminator keeps — still runs its `[…]`.
#[test]
fn an_expression_surface_runs_its_substitutions() {
    let reg = registry();
    let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
    let expr = tcl_syntax::expr::parser::parse_expr_for_profile("[id 9] > 5", reg.profile());
    let surface = tcl_registry::model::DocumentCommandSurface::new(&reg, None);
    let lifted = lifted_calls_in_expr(&expr, Some(100), config, &surface);
    assert_eq!(
        lifted
            .iter()
            .map(|c| (c.command.clone(), c.args.clone()))
            .collect::<Vec<_>>(),
        vec![("id".to_owned(), vec!["9".to_owned()])]
    );
    assert_eq!(
        lifted[0].span,
        tcl_lexer::Span::new(100, 106),
        "the base anchors the substitution back to its source offset"
    );
    assert_eq!(
        lifted_calls_in_text("[id 9] > 5", Some(100), config, &surface)
            .iter()
            .map(|c| c.command.clone())
            .collect::<Vec<_>>(),
        vec!["id".to_owned()],
        "the same holds for a surface read as raw text"
    );
}

/// Substitutions come back innermost-first — Tcl's evaluation order, so a
/// consumer replaying them moves its state exactly as the runtime does.
#[test]
fn nested_substitutions_are_innermost_first() {
    assert_eq!(
        lift("puts [list [lindex $x 0]]"),
        vec![
            ("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()]),
            ("list".to_owned(), vec!["[lindex $x 0]".to_owned()]),
        ]
    );
}

/// A substitution embedded in a larger word is still evaluated.
#[test]
fn substitution_inside_a_compound_word_is_lifted() {
    assert_eq!(
        lift("puts a[lindex $x 0]b"),
        vec![("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()])]
    );
}

/// A word with nothing to run costs nothing.
#[test]
fn words_without_substitutions_lift_nothing() {
    assert_eq!(
        lift("puts $x"),
        [] as [(std::string::String, std::vec::Vec<std::string::String>); 0]
    );
    assert_eq!(
        lift("puts plain"),
        [] as [(std::string::String, std::vec::Vec<std::string::String>); 0]
    );
}

/// A substitution nested in a *quoted* word of another substitution still
/// runs — Tcl substitutes inside `"…"`. Recovering the nested command's
/// own words is what sees it; the argument text `"[lindex $x 0]"` starts
/// with a quote, so no `[`-prefix test over that text ever could.
///
/// Asserted on the commands and the inner call's own arguments; the outer
/// `list`'s argument *text* keeps the raw spelling.
#[test]
fn substitution_inside_a_quoted_nested_word_is_lifted() {
    let calls = lift_calls("puts [list \"[lindex $x 0]\"]");
    assert_eq!(
        calls.iter().map(|c| c.command.as_str()).collect::<Vec<_>>(),
        vec!["lindex", "list"]
    );
    assert_eq!(calls[0].args, vec!["$x".to_owned(), "0".to_owned()]);
}

/// The same for a substitution welded into a larger nested word.
#[test]
fn substitution_welded_into_a_nested_word_is_lifted() {
    assert_eq!(
        lift("puts [list a[lindex $x 0]b]"),
        vec![
            ("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()]),
            ("list".to_owned(), vec!["a[lindex $x 0]b".to_owned()]),
        ]
    );
}

/// …and the brace rule holds one level down too: a braced argument of a
/// nested command is literal text, not a command to run.
#[test]
fn braced_word_of_a_nested_substitution_is_not_run() {
    assert_eq!(
        lift("puts [list {[lindex $x 0]}]"),
        vec![("list".to_owned(), vec!["{[lindex $x 0]}".to_owned()])]
    );
}

/// A substitution in the *command* position runs before the command it
/// spells is looked up.
#[test]
fn substitution_in_the_command_word_is_lifted() {
    assert_eq!(
        lift("puts [[lindex $x 0] 1]"),
        vec![
            ("lindex".to_owned(), vec!["$x".to_owned(), "0".to_owned()]),
            ("[lindex $x 0]".to_owned(), vec!["1".to_owned()]),
        ]
    );
}

/// `arg_words` is the segmenter's structure for the same words `args`
/// spells, index-aligned, so a consumer can tell a braced literal from a
/// word that substitutes without re-deciding it from the text.
#[test]
fn arg_words_align_with_args() {
    let calls = lift_calls("puts [list {a b} $x [set y] plain]");
    // `[set y]` is evaluated first, so the outer `list` is lifted last.
    let [_, lifted] = calls.as_slice() else {
        panic!("expected the inner `set` then the outer `list`, got {calls:?}");
    };
    assert_eq!(lifted.command, "list");
    assert_eq!(lifted.arg_words.len(), lifted.args.len());
    assert!(matches!(
        lifted.arg_words.as_slice(),
        [
            WordExpr::BracedLiteral { .. },
            WordExpr::Variable { .. },
            WordExpr::CommandSubstitution { .. },
            WordExpr::Literal { .. },
        ]
    ));
}

/// The value emitter needs the exact inner word forms for a whole nested
/// substitution, while a braced outer word must still decline.
#[test]
fn unpositioned_nested_words_preserve_their_source_provenance() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let spelling = r#"[f "$x[child]" [child]]"#;
        let site = SourceSite::opaque(Span::new(400, 424));
        let tokens = nested_command_words(spelling, &site, config).unwrap();
        assert!(
            tokens
                .words()
                .iter()
                .all(|word| word.source().provenance == Provenance::Opaque),
            "{engine}"
        );
        let WordExpr::Template { parts, .. } = &tokens.words()[1] else {
            panic!("{engine}: quoted template")
        };
        for part in parts {
            let source = match part {
                WordPart::Text { source, .. }
                | WordPart::Variable { source, .. }
                | WordPart::CommandSubstitution { source, .. }
                | WordPart::Opaque { source, .. } => source,
            };
            assert_eq!(source.provenance, Provenance::Opaque, "{engine}");
        }
        let child = whole_word_command_tokens(&tokens.words()[2], config).unwrap();
        assert_eq!(
            child.words()[0].source().provenance,
            Provenance::Opaque,
            "{engine}"
        );
        let original =
            nested_command_words(spelling, &SourceSite::source(site.span), config).unwrap();
        assert!(
            original
                .words()
                .iter()
                .all(|word| word.source().provenance == Provenance::Source),
            "{engine}"
        );
        assert_eq!(original.words()[0].source().span.start(), 401, "{engine}");
    }
}

#[test]
fn whole_word_command_tokens_preserve_nested_word_forms() {
    let reg = registry();
    let config = tcl_lexer::LexerConfig::for_profile(reg.profile());
    for (body, expected) in [
        ("puts [info exists {p\\x75b}]", "p\\x75b"),
        ("puts \"[info exists pub]\"", "pub"),
    ] {
        let src = format!("proc f {{}} {{{body}}}");
        let cu = CompilationUnit::build_for(&src, &reg, false);
        let fu = cu.function("::f").expect("proc lowered");
        let word = fu
            .cfg
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .find_map(|stmt| match stmt {
                Statement::Call {
                    tokens: Some(tokens),
                    ..
                } => tokens.words().get(1),
                _ => None,
            })
            .expect("puts argument word");
        let nested = whole_word_command_tokens(word, config).expect("nested command words");
        assert_eq!(
            nested.argv_texts,
            vec!["info".to_owned(), "exists".to_owned(), expected.to_owned()]
        );
    }

    let src = "proc f {} {puts {[info exists pub]}}";
    let cu = CompilationUnit::build_for(src, &reg, false);
    let fu = cu.function("::f").expect("proc lowered");
    let word = fu
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find_map(|stmt| match stmt {
            Statement::Call {
                tokens: Some(tokens),
                ..
            } => tokens.words().get(1),
            _ => None,
        })
        .expect("puts argument word");
    assert!(whole_word_command_tokens(word, config).is_none());
}

#[test]
fn original_argument_index_children_keep_order_source_and_inherited_bindings() {
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl").unwrap();
    let config = input.lexer_config();
    let registry = input.borrowed_context_registry().commands();
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    let source = r#"puts "pre$a([first]$b([inner])[last])post[tail]""#;
    let tokens = original_logical_tokens(source, &input);
    let calls = checked_original_argument_lifted_calls(&tokens, registry, metadata).unwrap();
    assert_eq!(
        calls
            .iter()
            .map(|call| call.command.as_str())
            .collect::<Vec<_>>(),
        ["first", "inner", "last", "tail"]
    );
    for call in &calls {
        assert_eq!(
            source.get(call.span.as_range()),
            Some(format!("[{}]", call.command).as_str())
        );
        let child = call.tokens.as_ref().unwrap();
        assert_eq!(
            child
                .source_binding
                .as_ref()
                .unwrap()
                .original_argument_lexer_config(child),
            Some(config)
        );
        assert!(
            original_call_context(
                child,
                config,
                registry,
                OriginalLiftedCallContext::Metadata(metadata)
            )
            .is_some()
        );
    }
    for source in [
        r"puts $a(\[literal\])",
        "puts ${a[not_a_command]}",
        "puts {$a([not_a_command])}",
    ] {
        let tokens = original_logical_tokens(source, &input);
        assert!(
            checked_original_argument_lifted_calls(&tokens, registry, metadata)
                .unwrap()
                .is_empty(),
            "{source}"
        );
    }
    let native_image =
        tcl_lexer::SourceImage::native(std::sync::Arc::<[u8]>::from(source.as_bytes()));
    let native = original_logical_tokens_in_image(&native_image, &input);
    let collection = LiftedCallCollection::for_original_parent(&native, config);
    assert_eq!(collection.original_image.as_ref(), Some(&native_image));
    assert_eq!(
        checked_original_argument_lifted_calls(&native, registry, metadata)
            .unwrap()
            .len(),
        4
    );
    let mut unowned = tokens.clone();
    unowned.source_binding = None;
    unowned.nested_bindings_mut().clear();
    assert!(checked_argument_lifted_calls(&unowned, config).is_none());
    let mut missing = tokens.clone();
    missing.nested_bindings_mut().clear();
    assert!(checked_original_argument_lifted_calls(&missing, registry, metadata).is_none());
    let mut foreign = tokens;
    foreign.argv_texts[1].push('!');
    assert!(checked_original_argument_lifted_calls(&foreign, registry, metadata).is_none());
}

#[test]
fn original_lexical_bracket_inventory_does_not_require_parent_handler_roles() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Authenticated written substitutions only; no child dispatch or body entry.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    let source = "puts [\u{feff}unknown argument]";
    let tokens = original_logical_tokens(source, &input);
    assert!(
        original_call_context(
            &tokens,
            config,
            registry,
            OriginalLiftedCallContext::Metadata(metadata)
        )
        .is_some()
    );
    assert!(
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            registry, metadata, &tokens,
        )
        .is_none()
    );
    let calls =
        checked_original_lifted_calls_with_metadata_context(&tokens, config, registry, metadata)
            .unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command, "\u{feff}unknown");
    assert_eq!(calls[0].args, ["argument"]);
    assert_eq!(
        source.get(calls[0].span.as_range()),
        Some("[\u{feff}unknown argument]")
    );
    let child = calls[0].tokens.as_ref().unwrap();
    assert!(
        original_call_context(
            child,
            config,
            registry,
            OriginalLiftedCallContext::Metadata(metadata)
        )
        .is_some()
    );
    assert!(
        child
            .source_binding
            .as_ref()
            .unwrap()
            .proved_execution_target()
            .is_none()
    );
    let mut changed = config;
    changed.strict_quoting = !changed.strict_quoting;
    assert!(
        checked_original_lifted_calls_with_metadata_context(&tokens, changed, registry, metadata,)
            .is_none()
    );
    let mut withdrawn = tokens.clone();
    withdrawn.nested_bindings_mut().clear();
    assert!(
            checked_original_lifted_calls_with_metadata_context(
                &withdrawn, config, registry, metadata,
            )
            .is_none()
        );
    let mut changed_vector = tokens.clone();
    changed_vector.argv_texts[0].push('!');
    assert!(
        checked_original_lifted_calls_with_metadata_context(
            &changed_vector,
            config,
            registry,
            metadata,
        )
        .is_none()
    );
}

#[test]
fn original_braced_expression_inventory_still_requires_actual_selected_roles() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Braced Expr descent remains separate from lexical bracket substitutions.
    let mut prepared = crate::analyser::Analyser::new();
    let input = prepared.prepare_analysis_input("tcl").unwrap();
    let registry = input.borrowed_context_registry().commands();
    let config = input.lexer_config();
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    let unknown = original_logical_tokens("unknown {[dict size DATA]}", &input);
    assert!(
        original_call_context(
            &unknown,
            config,
            registry,
            OriginalLiftedCallContext::Metadata(metadata)
        )
        .is_some()
    );
    assert!(
        checked_original_lifted_calls_with_metadata_context(&unknown, config, registry, metadata,)
            .is_none()
    );
    let literal = original_logical_tokens("puts {[dict size DATA]}", &input);
    assert_eq!(
        checked_original_lifted_calls_with_metadata_context(&literal, config, registry, metadata,)
            .unwrap(),
        [] as [crate::word_subst::LiftedCall; 0]
    );
    let expression = original_logical_tokens("expr {[dict size DATA]}", &input);
    let calls = checked_original_lifted_calls_with_metadata_context(
        &expression,
        config,
        registry,
        metadata,
    )
    .unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command, "dict");
    assert_eq!(calls[0].args, ["size", "DATA"]);
    let mut withdrawn = expression.clone();
    withdrawn.source_binding = None;
    assert!(
            checked_original_lifted_calls_with_metadata_context(
                &withdrawn, config, registry, metadata,
            )
            .is_none()
        );
}
