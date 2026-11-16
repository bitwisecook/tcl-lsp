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

use super::super::{Arc, SourceAnalysisOptions, SourceCommandBindings};
use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationMode};

fn analyse(
    source: &str,
    dialect: tcl_registry::InvocationDialect,
) -> (SourceCommandBindings, tcl_lexer::LexerConfig) {
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
    let registry = tcl_registry::CommandRegistry::build_default();
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        config,
        &registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(dialect),
            native_compilation: NativeCompilationContext {
                mode: NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    (bindings, config)
}

fn tokens(source: &str, config: tcl_lexer::LexerConfig, offset: u32) -> crate::ir::CommandTokens {
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let segment = segments
        .iter()
        .find(|segment| segment.span.start() == offset)
        .unwrap();
    crate::ir::CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, segment)
}

fn dialects() -> impl Iterator<Item = tcl_registry::InvocationDialect> {
    tcl_dialect::TclVersion::ALL
        .into_iter()
        .map(tcl_registry::InvocationDialect::for_version)
        .chain([tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
        )])
}

#[test]
fn logical_original_word_geometry_includes_genuine_declaration_vectors() {
    let source = "namespace eval N {variable x FIRST; set x AGAIN; puts $x}\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let realm = analysis.retained_command_realm().unwrap();
    let config = analysis.body_lexer_config.unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    let start = u32::try_from(source.find("set x AGAIN").unwrap() + "set ".len()).unwrap();
    let span = tcl_lexer::Span::new(start, start + 1);
    let (word, head) = realm
        .original_written_word_at_span_in_source(&image, span, config)
        .unwrap();
    assert_eq!(head, start - 4);
    assert_eq!(&image.bytes()[word.span().as_range()], b"x");
    assert!(
        realm
            .original_written_word_at_span_in_source(
                &tcl_lexer::SourceImage::document(&format!("{source} ")),
                span,
                config,
            )
            .is_none()
    );
    let mut changed = config;
    changed.strict_quoting = !changed.strict_quoting;
    assert!(
        realm
            .original_written_word_at_span_in_source(&image, span, changed)
            .is_none()
    );
}

#[test]
fn logical_receiver_word_geometry_and_write_card_keep_exact_source_currency() {
    let source = "namespace eval N {variable x FIRST; set x AGAIN; puts $x}\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let realm = analysis.retained_command_realm().unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config.unwrap();
    let head = u32::try_from(source.find("set x AGAIN").unwrap()).unwrap();
    let span = tcl_lexer::Span::new(head + 4, head + 5);
    let original = realm
        .original_written_word_at_span_in_source(&image, span, config)
        .unwrap();
    assert_eq!(original.1, head);
    assert_eq!(&image.bytes()[original.0.span().as_range()], b"x");
    let card = analysis
        .original_variable_write_advice
        .iter()
        .find(|card| card.span() == span)
        .unwrap();
    assert!(analysis.logical_original_word_input_matches_source(
        &image,
        config,
        span,
        card.original_name_input()
    ));
    let stale = tcl_lexer::SourceImage::document(&source.replace("AGAIN", "OTHER"));
    assert!(
        realm
            .original_written_word_at_span_in_source(&stale, span, config)
            .is_none()
    );
    assert!(!analysis.logical_original_word_input_matches_source(
        &stale,
        config,
        span,
        card.original_name_input()
    ));
    let mut changed = config;
    changed.strict_quoting = !changed.strict_quoting;
    assert!(
        realm
            .original_written_word_at_span_in_source(&image, span, changed)
            .is_none()
    );
    assert!(!analysis.logical_original_word_input_matches_source(
        &image,
        changed,
        span,
        card.original_name_input()
    ));
}

#[test]
fn logical_point_names_keep_original_source_words_and_strict_native_purpose() {
    let source = "namespace eval N {variable x FIRST; set x AGAIN; puts $x}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let realm = analysis.retained_command_realm().unwrap();
    let bindings = realm.source_bindings_ref();
    let config = analysis.body_lexer_config.unwrap();
    let image = tcl_lexer::SourceImage::document(source);
    let offset = u32::try_from(source.find("set x AGAIN").unwrap()).unwrap();
    let span = tcl_lexer::Span::new(offset + 4, offset + 5);
    let input = bindings
        .original_written_name_input_at_span_in_source(&image, span, config)
        .expect("original static Logical point name");
    assert_eq!(input.bytes(), b"x");
    assert_eq!(
        input.policy().authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    assert_eq!(
        input.original_word_key().unwrap().original_word().span(),
        span
    );
    let observations = bindings
        .original_written_word_observations_at_span_with(span, config, None)
        .expect("independently retained point geometry");
    assert!(observations.iter().all(|(_, _, binding, _)| {
        binding
            .variable_context
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .is_none()
    }));
    assert!(
        bindings
            .original_written_name_input_at_span_in_source(
                &tcl_lexer::SourceImage::document(&format!("{source} ")),
                span,
                config,
            )
            .is_none()
    );
    assert!(
        bindings
            .original_written_name_input_at_span_in_source(
                &tcl_lexer::SourceImage::native(source.as_bytes()),
                span,
                config,
            )
            .is_none()
    );
    let mut changed = config;
    changed.strict_quoting = !changed.strict_quoting;
    assert!(
        bindings
            .original_written_name_input_at_span_in_source(&image, span, changed)
            .is_none()
    );
}

#[test]
fn logical_conditional_receiver_point_names_keep_source_only_authority() {
    let source = include_str!(
        "../../../../../experiments/name-handling/patterns/composed-callback-prefix/probes/positive.tcl"
    );
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let owner = analysis.resolved_input.as_ref().unwrap();
    assert!(owner.has_logical_source_name_context());
    let realm = analysis.retained_command_realm().unwrap();
    let bindings = realm.source_bindings_ref();
    let image = tcl_lexer::SourceImage::document(source);
    let start = u32::try_from(source.find("after idle $cb").unwrap() + "after ".len()).unwrap();
    let span = tcl_lexer::Span::new(start, start + 4);
    let observations = bindings
        .original_written_word_observations_at_span_with(span, owner.lexer_config(), None)
        .expect("genuine retained receiver source vector");
    let (word, written, binding, _) = observations.iter().find(|(_, _, binding, _)| {
            binding.declaration_layout_observations.as_deref().is_some_and(|rows| {
                rows.iter().any(|row| row.issuer == super::super::declaration_layout::DeclarationLayoutIssuer::ConditionalReceiverSource)
            })
        }).expect("actual independently retained ConditionalReceiverSource observation");
    assert!(binding.original_recorded_name_words().is_none());
    let input = bindings
        .original_written_name_input_at_span_in_source(&image, span, owner.lexer_config())
        .expect("static name does not require declaration-only invocation words");
    assert_eq!(input.bytes(), b"idle");
    assert_eq!(input.original_word_key().unwrap().original_word(), word);
    assert_eq!(
        input.policy().authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    assert!(
        binding
            .original_retained_written_name_input(*written)
            .is_none()
    );
    assert!(
        bindings
            .original_namespace_operand_at_span_in_source(&image, span, owner.lexer_config())
            .is_none()
    );
    let mut changed = binding.clone();
    let rows = std::sync::Arc::make_mut(changed.declaration_layout_observations.as_mut().unwrap());
    rows[0].config.strict_quoting = !rows[0].config.strict_quoting;
    assert!(
        changed
            .original_logical_source_written_name_input(owner, *written)
            .is_none()
    );
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        owner.analyser_profile(),
        owner.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        owner.lexer_config(),
    );
    assert!(
        binding
            .original_logical_source_written_name_input(&foreign, *written)
            .is_none()
    );
}

fn retained_registered_instance_after_realm_drop(
    source: &str,
) -> (
    crate::command_binding::SourceCommandBindings,
    crate::ir::CommandTokens,
    crate::analyser::ResolvedAnalysisInput,
) {
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap().clone();
    let realm = analysis.retained_command_realm().unwrap();
    let offset = u32::try_from(source.find(".e configure").unwrap()).unwrap();
    let command = crate::segmenter::segment_commands_with_offset_and_config(
        &source[offset as usize..],
        offset,
        input.lexer_config(),
    )
    .remove(0);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        input.lexer_config(),
        &command,
    );
    realm.stamp_original_tokens(&mut tokens);
    let before = tokens.source_binding.clone();
    realm.stamp_original_tokens(&mut tokens);
    assert_eq!(
        tokens.source_binding, before,
        "cache warmth cannot change invocation identity"
    );
    (realm.source_bindings_ref().clone(), tokens, input)
}

fn assert_native_record_cannot_be_logical_geometry() {
    let native = crate::analyser::Analyser::new().analyse("list VALUE", "tcl8.6");
    let input = native.resolved_input.as_ref().unwrap();
    let bindings = native
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref();
    assert!(
        bindings
            .invocation_at_source("", 0)
            .original_recorded_command()
            .is_some()
    );
    assert!(
        bindings
            .original_logical_source_command_at_in_source(
                input,
                &tcl_lexer::SourceImage::document("list VALUE"),
                0,
                input.lexer_config()
            )
            .is_none()
    );
}

fn assert_conflicting_receiver_geometry_refuses(
    bindings: &crate::command_binding::SourceCommandBindings,
    binding: &crate::command_binding::SourceInvocationBinding,
    input: &crate::analyser::ResolvedAnalysisInput,
    image: &tcl_lexer::SourceImage,
    offset: u32,
) {
    for change_config in [true, false] {
        let mut changed = bindings.clone();
        let rows = changed
            .declaration_layouts
            .get_mut(binding.invocation_site().unwrap())
            .unwrap();
        let mut conflict = rows.iter().find(|row| row.issuer == super::super::declaration_layout::DeclarationLayoutIssuer::ConditionalReceiverSource).unwrap().clone();
        if change_config {
            conflict.config.strict_quoting = !conflict.config.strict_quoting;
        } else {
            conflict.words = std::sync::Arc::from(&conflict.words[..conflict.words.len() - 1]);
        }
        rows.push(conflict);
        assert!(
            changed
                .original_logical_source_command_at_in_source(
                    input,
                    image,
                    offset,
                    input.lexer_config()
                )
                .is_none(),
            "conflicting retained config/vector withdraws the projection"
        );
    }
}

fn assert_receiver_original_vector(
    original: &crate::registry_invocation::OriginalSourceCommandProjection,
    image: &tcl_lexer::SourceImage,
    input: &crate::analyser::ResolvedAnalysisInput,
) {
    assert_eq!(
        original
            .native_words()
            .iter()
            .map(|word| word.try_text().unwrap())
            .collect::<Vec<_>>(),
        ["after", "idle", "$cb"]
    );
    assert_eq!(original.command().argv.len(), original.native_words().len());
    assert!(
        original
            .native_words()
            .iter()
            .all(|word| word.image() == image && word.config() == input.lexer_config())
    );
}

#[test]
fn registered_instance_sources_keep_original_identity_and_source_owner_lifetime() {
    let source = "entry .e; .e configure -textvariable value";
    let (bindings, tokens, input) = retained_registered_instance_after_realm_drop(source);
    let registry = input.borrowed_context_registry().commands();
    let metadata =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(registry, &input)
            .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    let receipt = binding
        .original_registered_instance_source_for_tokens(&tokens, metadata, registry)
        .expect("the retained source bindings keep original instance metadata after realm drop");
    assert_eq!(receipt.instance().factory().command(), "entry");
    assert_eq!(receipt.original_words().len(), 4);
    assert_eq!(
        receipt.argument_word(2).unwrap().try_text().unwrap(),
        "value"
    );
    assert_eq!(
        receipt.with_source_schema(input.borrowed_context_registry(), |schema| schema
            .authored_source_option_variable_scope_at(2)),
        Some(Some(tcl_registry::VariableScope::Global))
    );
    let mut foreign_cache = bindings.clone();
    foreign_cache.original_registered_instance_sources = std::sync::Arc::default();
    assert!(
        foreign_cache
            .retain_original_registered_instance_source(&input, receipt.as_ref().clone())
            .is_none(),
        "an established owner link cannot be overwritten by another optional cache"
    );
    assert!(
        binding
            .original_registered_instance_source_for_tokens(&tokens, metadata, registry)
            .is_some()
    );
    let mut changed = tokens.clone();
    changed.argv_texts[3] = "foreign".into();
    assert!(
        changed
            .source_binding
            .as_ref()
            .unwrap()
            .original_registered_instance_source_for_tokens(&changed, metadata, registry)
            .is_none()
    );
    let standalone = crate::registry_invocation::InvocationMetadataContext::from(
        input.borrowed_context_registry(),
    );
    assert!(
        binding
            .original_registered_instance_source_for_tokens(&tokens, standalone, registry)
            .is_none()
    );
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    let metadata_foreign =
        crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, &foreign,
        )
        .unwrap();
    assert!(
        binding
            .original_registered_instance_source_for_tokens(&tokens, metadata_foreign, registry)
            .is_none()
    );
    let cache = std::sync::Arc::downgrade(&bindings.original_registered_instance_sources);
    drop(foreign_cache);
    drop(bindings);
    assert!(
        cache.upgrade().is_none(),
        "no receipt/analysis-owner strong cycle retains derived source authority"
    );
    assert!(
        binding
            .original_registered_instance_source_for_tokens(&tokens, metadata, registry)
            .is_none(),
        "the surviving token and readonly receipt cannot upgrade a dropped source owner"
    );
}

#[test]
fn logical_source_body_vectors_reborrow_existing_tape_owners_without_native_entry() {
    for (source, command, expected) in [
        (
            "namespace eval N {set x VALUE}",
            "set x VALUE",
            vec!["set", "x", "VALUE"],
        ),
        (
            "proc p {} {$dispatch $value}",
            "$dispatch $value",
            vec!["$dispatch", "$value"],
        ),
        (
            "oo::class create C {method run {} {set local VALUE}}",
            "set local VALUE",
            vec!["set", "local", "VALUE"],
        ),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let image = tcl_lexer::SourceImage::document(source);
        let offset = u32::try_from(source.find(command).unwrap()).unwrap();
        let original = bindings
            .original_logical_source_command_at_in_source(
                input,
                &image,
                offset,
                input.lexer_config(),
            )
            .expect("genuine original body vector remains independent of invocation selection");
        assert_eq!(
            original
                .native_words()
                .iter()
                .map(|word| word.try_text().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(
            original
                .native_words()
                .iter()
                .all(|word| word.image() == &image && word.config() == input.lexer_config())
        );
        assert!(
            bindings
                .invocation_at_source("", offset)
                .original_recorded_command()
                .is_none(),
            "readonly geometry grants no declaration or invocation words"
        );
        for stale in [
            tcl_lexer::SourceImage::native(source.as_bytes()),
            tcl_lexer::SourceImage::document(&format!("{source} ")),
        ] {
            assert!(
                bindings
                    .original_logical_source_command_at_in_source(
                        input,
                        &stale,
                        offset,
                        input.lexer_config()
                    )
                    .is_none()
            );
        }
        let mut config = input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        assert!(
            bindings
                .original_logical_source_command_at_in_source(input, &image, offset, config)
                .is_none()
        );
    }
}

#[test]
fn logical_source_command_vectors_keep_genuine_conditional_receiver_geometry() {
    let source = include_str!(
        "../../../../../experiments/name-handling/patterns/composed-callback-prefix/probes/positive.tcl"
    );
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let bindings = analysis
        .retained_command_realm()
        .unwrap()
        .source_bindings_ref();
    let image = tcl_lexer::SourceImage::document(source);
    let offset = u32::try_from(source.find("after idle $cb").unwrap()).unwrap();
    let binding = bindings.invocation_at_source("", offset);
    assert!(binding.declaration_layout_observations.as_deref().unwrap().iter().any(|row| {
            row.issuer == super::super::declaration_layout::DeclarationLayoutIssuer::ConditionalReceiverSource
        }), "actual independently retained receiver source issuer");
    assert!(binding.original_recorded_command().is_none());
    let original = bindings
        .original_logical_source_command_at_in_source(input, &image, offset, input.lexer_config())
        .expect("one genuine complete receiver source vector");
    assert_receiver_original_vector(&original, &image, input);
    assert!(
        binding.original_recorded_command().is_none(),
        "geometry cannot promote declaration authority"
    );
    for image in [
        tcl_lexer::SourceImage::document(&format!("{source} ")),
        tcl_lexer::SourceImage::native(source.as_bytes()),
    ] {
        assert!(
            bindings
                .original_logical_source_command_at_in_source(
                    input,
                    &image,
                    offset,
                    input.lexer_config()
                )
                .is_none()
        );
    }
    let mut changed = input.lexer_config();
    changed.strict_quoting = !changed.strict_quoting;
    assert!(
        bindings
            .original_logical_source_command_at_in_source(input, &image, offset, changed)
            .is_none()
    );
    assert!(
        bindings
            .original_logical_source_command_at_in_source(
                input,
                &image,
                offset + 6,
                input.lexer_config()
            )
            .is_none()
    );
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    );
    assert!(
        bindings
            .original_logical_source_command_at_in_source(
                &foreign,
                &image,
                offset,
                input.lexer_config()
            )
            .is_none()
    );
    assert_conflicting_receiver_geometry_refuses(bindings, &binding, input, &image, offset);
    assert_native_record_cannot_be_logical_geometry();
}

#[test]
fn logical_namespace_point_operands_use_independent_original_frame_recipes() {
    use crate::signature_scan::scope::SignatureNamespaceScope;
    for (source, home) in [
        ("namespace eval Target {}", None),
        ("namespace eval N {namespace eval Target {}}", Some("N")),
        (
            "namespace eval N {proc p {} {namespace eval Target {}}}",
            Some("N"),
        ),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let config = analysis.body_lexer_config.unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let start = u32::try_from(source.find("Target").unwrap()).unwrap();
        let span = tcl_lexer::Span::new(start, start + 6);
        let operand = bindings
            .original_namespace_operand_at_span_in_source(&image, span, config)
            .unwrap_or_else(|| panic!("genuine source namespace recipe: {source}"));
        assert_eq!(operand.name_input().bytes(), b"Target");
        let SignatureNamespaceScope::C(namespace) = operand.context() else {
            panic!("Logical C source recipe");
        };
        assert_eq!(
            namespace.as_segments(),
            home.into_iter()
                .map(tcl_core_types::NameBytes::from)
                .collect::<Vec<_>>()
        );
        assert!(
            bindings
                .original_namespace_operand_at_span_in_source(
                    &tcl_lexer::SourceImage::document(&format!("{source} ")),
                    span,
                    config,
                )
                .is_none()
        );
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(
            bindings
                .original_namespace_operand_at_span_in_source(&image, span, changed)
                .is_none()
        );
    }
    for source in [
        "namespace eval $unknown {namespace eval Target {}}",
        "oo::class create C {method m {} {namespace eval Target {}}}",
        "set inert {namespace eval Target {}}",
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let config = analysis.body_lexer_config.unwrap();
        let start = u32::try_from(source.find("Target").unwrap()).unwrap();
        assert!(
            analysis
                .retained_command_realm()
                .unwrap()
                .source_bindings_ref()
                .original_namespace_operand_at_span_in_source(
                    &tcl_lexer::SourceImage::document(source),
                    tcl_lexer::Span::new(start, start + 6),
                    config,
                )
                .is_none(),
            "{source}"
        );
    }
}

#[test]
fn logical_recorded_name_operands_use_the_checked_source_purpose() {
    let source = "proc f {value} {return $value}\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let realm = analysis.retained_command_realm().unwrap();
    let binding = realm.invocation_at_source("", 0);
    let (_, tokens) = binding.original_recorded_command().unwrap();
    let input = binding.original_written_name_input(&tokens, 1).unwrap();
    assert_eq!(input.bytes(), b"f");
    assert_eq!(
        input.policy().authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    let mut changed = tokens.clone();
    changed.argv_texts[1] = "other".to_owned();
    assert!(binding.original_written_name_input(&changed, 1).is_none());
}

#[test]
fn original_word_issuers_require_the_complete_current_compatibility_vector() {
    // naming.source.original-direct-token-owner
    // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
    for dialect in dialects() {
        let source = "list ONE";
        let (bindings, config) = analyse(source, dialect);
        let original = tokens(source, config, 0);
        let binding = bindings.invocation_at_source("list", 0);
        assert_eq!(
            binding.original_lexer_config_for_tokens(&original),
            Some(config)
        );
        assert!(binding.original_written_name_input(&original, 1).is_some());
        assert!(binding.original_head_name_input(&original).is_some());
        for axis in 0..4 {
            let mut changed = original.clone();
            match axis {
                0 => changed.argv_texts[1] = "OTHER".to_owned(),
                1 => {
                    changed.argv_texts.pop();
                }
                2 => {
                    changed.word_exprs.pop();
                }
                _ => {
                    let source = changed.word_exprs[1].source().clone();
                    changed.word_exprs[1] = crate::ir::WordExpr::Opaque {
                        text: "OTHER".to_owned(),
                        source,
                        reason: crate::ir::WordOpacity::LossySnapshot,
                    };
                }
            }
            assert!(
                binding.original_lexer_config_for_tokens(&changed).is_none(),
                "axis={axis}"
            );
            assert!(
                binding.original_written_name_input(&changed, 1).is_none(),
                "axis={axis}"
            );
            assert!(
                binding.original_head_name_input(&changed).is_none(),
                "axis={axis}"
            );
        }
    }
}

#[test]
fn original_frozen_list_result_keeps_native_units_and_full_vector_owner() {
    let source = r"list [list p\u0000tail p\uD800]";
    for dialect in dialects() {
        let (bindings, config) = analyse(source, dialect);
        let original = tokens(source, config, 0);
        let binding = bindings.invocation_at_source("list", 0);
        let input = binding
            .original_written_name_input(&original, 1)
            .expect("exact nested List result");
        assert!(input.original_word_key().is_none());
        assert!(input.bytes().starts_with(
            if dialect.authored_name_policy().unwrap().recipe()
                == tcl_syntax::naming::NativeNameProtocol::Jim084
            {
                b"p\0tail".as_slice()
            } else {
                b"p\xc0\x80tail".as_slice()
            }
        ));
        let shortened = tokens("list", config, 0);
        assert!(binding.original_written_name_input(&shortened, 0).is_none());
    }
}

#[test]
fn original_copied_store_keeps_data_separate_from_read_and_word_authority() {
    let source = r"set name p\uD800; set copy $name; list $copy";
    for dialect in dialects() {
        let (bindings, config) = analyse(source, dialect);
        let offset = u32::try_from(source.rfind("list").unwrap()).unwrap();
        let original = tokens(source, config, offset);
        let binding = bindings.invocation_at_source("list", offset);
        let input = binding
            .original_written_name_input(&original, 1)
            .expect("current exact stored byte read");
        assert!(input.original_word_key().is_none());
        assert_ne!(input.bytes(), [] as [u8; 0]);
        let held = input.produced_value().unwrap();
        assert!(held.is_current(&binding.variable_context));
        let mut changed = binding.clone();
        Arc::make_mut(&mut changed.variable_context).invalidate_original_contents();
        assert!(changed.original_written_name_input(&original, 1).is_none());
    }
}

#[test]
fn original_retained_span_input_requires_exact_image_config_and_word() {
    // Implementation contract: naming.source.readonly-original-operand-projections (docs/design/analysis/name-resolution-proofs/readonly-original-operand-projections.md).
    let source = "set name {::A}; list $name";
    for dialect in dialects() {
        let (bindings, config) = analyse(source, dialect);
        let offset = u32::try_from(source.rfind("list").unwrap()).unwrap();
        let original = tokens(source, config, offset);
        let binding = bindings.invocation_at_source("list", offset);
        let expected = binding.original_written_name_input(&original, 1).unwrap();
        let span = original.words()[1].source().span;
        let image = tcl_lexer::SourceImage::document(source);
        assert!(bindings.matches_original_source_image(&image, config));
        assert!(!bindings.matches_original_source_image(
            &tcl_lexer::SourceImage::native(source.as_bytes()),
            config,
        ));
        assert!(!bindings.matches_original_source_image(
            &image,
            tcl_lexer::LexerConfig {
                strict_quoting: !config.strict_quoting,
                ..config
            },
        ));
        let input = bindings
            .original_written_name_input_at_span_in_source(&image, span, config)
            .unwrap();
        assert_eq!(input, expected);
        assert!(input.original_word_key().is_none());
        assert_eq!(input.bytes(), b"::A");
        assert!(
            bindings
                .original_written_name_input_at_span_in_source(
                    &tcl_lexer::SourceImage::native(source.as_bytes()),
                    span,
                    config,
                )
                .is_none()
        );
        assert!(
            bindings
                .original_written_name_input_at_span_in_source(
                    &tcl_lexer::SourceImage::document(&source.replace("::A", "::B")),
                    span,
                    config,
                )
                .is_none()
        );
        assert!(
            bindings
                .original_written_name_input_at_span(
                    span,
                    tcl_lexer::LexerConfig {
                        strict_quoting: !config.strict_quoting,
                        ..config
                    },
                )
                .is_none()
        );
        assert!(
            bindings
                .original_written_name_input_at_span(
                    tcl_lexer::Span::new(span.start() + 1, span.end()),
                    config,
                )
                .is_none()
        );
    }
}

#[test]
fn expanded_dispatch_head_uses_its_original_written_extent() {
    for dialect in dialects() {
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        if !config.expand_syntax {
            continue;
        }
        for source in ["{*}{list value} extra", "{*}{} {*}{list value} extra"] {
            let (bindings, config) = analyse(source, dialect);
            let point = bindings
                .dispatch_points_at(0)
                .next()
                .expect("dispatch point");
            let input = point.original_head_input.as_ref().expect("original head");
            assert_eq!(input.bytes(), b"list", "{source}");
            let original = tokens(source, config, 0);
            let binding = bindings.invocation_at_source("list", 0);
            assert!(binding.original_head_name_input(&original).is_some());
            let mut stale = original.clone();
            stale.argv_texts[0].push('x');
            assert!(binding.original_head_name_input(&stale).is_none());
            let foreign = tokens("{*}{puts value} extra", config, 0);
            assert!(binding.original_head_name_input(&foreign).is_none());
        }
        let (bindings, _) = analyse("{*}$unknown extra", dialect);
        assert!(
            bindings
                .dispatch_points_at(0)
                .all(|point| point.original_head_input.is_none())
        );
    }
}

#[test]
fn original_effective_head_keeps_computed_and_expanded_producers() {
    // Implementation contract: naming.source.readonly-original-operand-projections (docs/design/analysis/name-resolution-proofs/readonly-original-operand-projections.md).
    for dialect in dialects() {
        let source = "set head list; $head VALUE";
        let (bindings, config) = analyse(source, dialect);
        let offset = u32::try_from(source.find("$head").unwrap()).unwrap();
        let original = tokens(source, config, offset);
        let binding = bindings.invocation_at_source("list", offset);
        let input = binding.original_head_name_input(&original).unwrap();
        assert_eq!(input.bytes(), b"list");
        assert!(input.original_word_key().is_none());
        let mut changed = binding.clone();
        Arc::make_mut(&mut changed.variable_context).invalidate_original_contents();
        assert!(changed.original_head_name_input(&original).is_none());
        if !config.expand_syntax {
            continue;
        }
        let source = "{*}{} {*}{list} VALUE";
        let (bindings, config) = analyse(source, dialect);
        let original = tokens(source, config, 0);
        let binding = bindings.invocation_at_source("list", 0);
        let input = binding.original_head_name_input(&original).unwrap();
        assert_eq!(input.bytes(), b"list");
        assert!(input.original_word_key().is_none());
        let container = input.original_static_list_container().unwrap();
        assert_eq!(container.ordinals(), &[0]);
        assert_eq!(
            container.parent_word().span(),
            crate::registry_invocation::original_native_compiler_words(
                &tcl_lexer::SourceImage::document(source),
                original.words(),
                0,
                config,
            )
            .unwrap()[1]
                .span()
        );
    }
}
