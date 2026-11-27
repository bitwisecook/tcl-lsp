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
use tcl_compiler::analyser::Analyser;

mod call_sites;
mod callback_targets;
mod dispatch_regions;
mod ensemble_subcommands;
mod highlights;
mod next_dispatch;
mod object_method_sites;
mod self_dispatch_scan;
mod variables;

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn execution_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    written: &str,
) -> &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation {
    let offset = u32::try_from(source.rfind(written).unwrap()).unwrap();
    analysis
        .command_invocations
        .iter()
        .find(|invocation| {
            invocation.lookup.is_execution_site() && invocation.range.start() == offset
        })
        .expect("actual command head must retain its positioned lookup")
}

fn retained_method_sites(source: &str, is_classmethod: bool) -> Vec<tcl_lexer::Span> {
    let analysis = analyse(source);
    find_obj_method_call_sites(
        source,
        analysis.resolved_profile().unwrap(),
        &analysis,
        "::C",
        "ping",
        is_classmethod,
    )
}

mod composed_callback_prefix_tests {
    use super::*;
    use tcl_compiler::analyser::ResolvedAnalysisInput;
    use tcl_compiler::registry_invocation::source_structure::{
        original_single_source_command_substitution, source_authored_command_prefix,
        source_registry_words,
    };

    const POSITIVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../experiments/name-handling/patterns/composed-callback-prefix/probes/positive.tcl"
    ));
    const UNKNOWN_WRITER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../experiments/name-handling/patterns/composed-callback-prefix/probes/unknown-writer.tcl"
    ));
    const PRIVATE_POSITIVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../experiments/name-handling/patterns/composed-callback-prefix/fixtures/private-positive.tcl"
    ));

    fn analyse_source(source: &str, release: &str) -> AnalysisResult {
        let environment = tcl_registry::model::ingress::resolve_environment(release);
        let profile = environment.analyser_profile();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            environment.default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    fn original_builder(
        source: &str,
        analysis: &AnalysisResult,
    ) -> (
        tcl_source::segment::SegmentedCommand,
        tcl_lexer::NativeWord,
        tcl_lexer::NativeWord,
    ) {
        let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
        let body = analysis.all_classes["::C"].methods["setup"].body_span;
        let (start, end) = strip_outer_braces(source, body);
        let region =
            tcl_lexer::Span::new(u32::try_from(start).unwrap(), u32::try_from(end).unwrap());
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            region,
            config,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_none());
        let setter = &plan.commands[0];
        assert_eq!(setter.words[0].bytes(), b"set");
        assert_eq!(setter.words[1].bytes(), b"cb");
        let wrapper = original_single_source_command_substitution(&setter.words[2]).unwrap();
        assert_eq!(wrapper.command().words[0].bytes(), b"namespace");
        assert_eq!(wrapper.command().words[1].bytes(), b"code");
        let child =
            original_single_source_command_substitution(&wrapper.command().words[2]).unwrap();
        assert_eq!(child.command().words.len(), 4);
        assert_eq!(child.command().words[0].bytes(), b"::makePrefix");
        let selector = child.command().words[2].clone();
        let fixed = child.command().words[3].clone();
        let span = child.command().span;
        let mut commands = tcl_source::segment::segment_commands_with_offset_and_config(
            &source[span.as_range()],
            span.start(),
            config,
        );
        assert_eq!(commands.len(), 1);
        (commands.remove(0), selector, fixed)
    }

    fn reference_ranges(
        source: &str,
        analysis: &AnalysisResult,
        include_declaration: bool,
    ) -> Vec<LspRange> {
        let profile = analysis.resolved_input.as_ref().unwrap().unit_profile();
        let declaration = analysis.all_classes["::C"].methods["tick"].name_span;
        let index = LineIndex::new(source);
        let at = index.position_at_utf16(declaration.start(), source);
        references(
            source,
            profile,
            at.line,
            at.character.get(),
            analysis,
            include_declaration,
        )
    }

    #[test]
    fn composed_callback_prefix_original_advice_keeps_builder_and_each_written_operand() {
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (case, source) in [
                ("positive", POSITIVE),
                ("unknown-writer", UNKNOWN_WRITER),
                ("private-positive", PRIVATE_POSITIVE),
            ] {
                let analysis = analyse_source(source, release);
                let (producer, selector, fixed) = original_builder(source, &analysis);
                let input = analysis.resolved_input.as_ref().unwrap();
                let context = input.context_registry();
                let words = source_registry_words(source, &analysis, &producer)
                    .unwrap_or_else(|| panic!("{release}/{case}: missing original builder schema"));
                assert_eq!(
                        words.with_source_schema(&context, |schema| {
                            schema
                                .semantics
                                .traits
                                .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
                                && schema.semantics.native_result
                                    == Some(
                                        tcl_registry::native_result::NativeResultContract::ListArguments {
                                            from: 0,
                                        },
                                    )
                                && schema.semantics.argument_offset == 0
                                && schema.facts().arity_accepts_frozen_arguments() == Some(true)
                        }),
                        Some(true),
                        "{release}/{case}"
                    );
                let prefix = source_authored_command_prefix(source, &analysis, &producer)
                    .unwrap_or_else(|| panic!("{release}/{case}: missing authored prefix receipt"));
                assert_eq!(
                    prefix.producer().site().offset,
                    producer.argv[0].span.start()
                );
                assert_eq!(prefix.producer().original_words().len(), 4);
                assert_eq!(
                    prefix.producer().original_words()[0].bytes(),
                    b"::makePrefix"
                );
                assert_ne!(prefix.producer().lineage(), []);
                assert!(prefix.obligations().contains(
                        &tcl_compiler::command_binding::SourceCommandTransitionObligation::AuthoredCommandPrefixApplicability
                    ));
                assert!(!prefix.obligations().contains(
                        &tcl_compiler::command_binding::SourceCommandTransitionObligation::ProducedCommandPrefixApplicability
                    ));
                assert_eq!(
                    prefix.original_head().original_word().unwrap().bytes(),
                    b"my"
                );
                assert_eq!(prefix.original_argument(0).unwrap(), &selector);
                assert_eq!(prefix.original_argument(1).unwrap(), &fixed);
                assert_eq!(selector.bytes(), b"tick");
                assert_eq!(fixed.bytes(), b"{a b}");
                assert_eq!(
                    prefix.argument_origin(0),
                    Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(2))
                );
                assert_eq!(
                    prefix.argument_origin(1),
                    Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(3))
                );
                assert_eq!(
                    prefix.with_source_schema(&context, |schema| {
                        schema
                            .semantics
                            .traits
                            .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
                    }),
                    Some(true),
                    "{release}/{case}"
                );
                assert!(prefix.matches_source_context(
                    &tcl_lexer::SourceImage::document(source),
                    input.lexer_config(),
                    &context,
                ));
                assert!(!prefix.matches_source_context(
                    &tcl_lexer::SourceImage::native(source.as_bytes()),
                    input.lexer_config(),
                    &context,
                ));
            }
        }
    }

    #[test]
    fn composed_callback_prefix_references_keep_public_and_private_original_selectors() {
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in [POSITIVE, PRIVATE_POSITIVE] {
                let analysis = analyse_source(source, release);
                let (_, selector, _) = original_builder(source, &analysis);
                let declaration = analysis.all_classes["::C"].methods["tick"].name_span;
                let index = LineIndex::new(source);
                let declaration_range = span_to_range(source, &index, declaration);
                let selector_range = span_to_range(source, &index, selector.span());
                if source == PRIVATE_POSITIVE {
                    assert!(analysis.all_classes["::C"].unexports.contains("tick"));
                }
                let refs = reference_ranges(source, &analysis, true);
                assert!(refs.contains(&declaration_range), "{release}: {refs:?}");
                assert!(
                    refs.contains(&selector_range),
                    "{release}: original stored selector missing: {refs:?}"
                );
                let refs = reference_ranges(source, &analysis, false);
                assert!(!refs.contains(&declaration_range), "{release}: {refs:?}");
                assert!(refs.contains(&selector_range), "{release}: {refs:?}");
            }
        }
    }

    #[test]
    fn composed_callback_prefix_stored_sink_withdraws_unknown_and_opaque_writers() {
        let opaque = POSITIVE.replacen("after idle $cb", "opaque $cb; after idle $cb", 1);
        let nested = POSITIVE.replacen(
            "after idle $cb",
            "if 1 {set cb {my unrelated}}; after idle $cb",
            1,
        );
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (case, source) in [
                ("unknown-writer", UNKNOWN_WRITER),
                ("opaque-writer", opaque.as_str()),
                ("nested-reassignment", nested.as_str()),
            ] {
                let analysis = analyse_source(source, release);
                let (_, selector, _) = original_builder(source, &analysis);
                let declaration = analysis.all_classes["::C"].methods["tick"].name_span;
                let index = LineIndex::new(source);
                let refs = reference_ranges(source, &analysis, true);
                assert!(
                    refs.contains(&span_to_range(source, &index, declaration)),
                    "{release}/{case}: declaration missing: {refs:?}"
                );
                assert!(
                    !refs.contains(&span_to_range(source, &index, selector.span())),
                    "{release}/{case}: stale stored selector survived: {refs:?}"
                );
            }
        }
    }

    #[test]
    fn composed_callback_prefix_original_context_withdraws_stale_source_and_configuration() {
        let source = POSITIVE;
        let analysis = analyse_source(source, "tcl8.6");
        let (producer, selector, _) = original_builder(source, &analysis);
        let index = LineIndex::new(source);
        let selector_range = span_to_range(source, &index, selector.span());
        assert!(source_authored_command_prefix(source, &analysis, &producer).is_some());
        assert!(reference_ranges(source, &analysis, true).contains(&selector_range));

        let changed = format!("{source}# different complete source\n");
        assert!(source_authored_command_prefix(&changed, &analysis, &producer).is_none());
        assert!(!reference_ranges(&changed, &analysis, true).contains(&selector_range));

        let mut different_config = analysis.clone();
        different_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .expand_syntax ^= true;
        assert!(source_authored_command_prefix(source, &different_config, &producer).is_none());
        assert!(!reference_ranges(source, &different_config, true).contains(&selector_range));

        let mut missing_input = analysis.clone();
        missing_input.resolved_input = None;
        assert!(source_authored_command_prefix(source, &missing_input, &producer).is_none());
        let declaration = analysis.all_classes["::C"].methods["tick"].name_span;
        let at = index.position_at_utf16(declaration.start(), source);
        let profile = analysis.resolved_input.as_ref().unwrap().unit_profile();
        assert!(
            !references(
                source,
                profile,
                at.line,
                at.character.get(),
                &missing_input,
                true,
            )
            .contains(&selector_range)
        );
    }
}

fn callback_source_store(gate: &str) -> std::sync::Arc<tcl_registry::CommandRegistry> {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let gated = registry.get(gate).unwrap();
    let surface = gated.surface;
    let required_package = gated.required_package;
    registry.insert(tcl_registry::CommandSpec {
        name: "source_callback",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::CommandPrefix)],
        command_prefixes: &[(0, tcl_registry::AppendedArity::Exactly(0))],
        traits: tcl_registry::Traits::DEFERS_BODY,
        surface,
        required_package,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    registry.insert(tcl_registry::CommandSpec {
        name: "source_dispatch",
        arity: tcl_registry::Arity::at_least(1),
        traits: tcl_registry::Traits::TCLOO_SELF_DISPATCH,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    std::sync::Arc::new(registry)
}

fn callback_source_analysis_with_store(
    source: &str,
    environment: &str,
    store: std::sync::Arc<tcl_registry::CommandRegistry>,
) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for(environment).with_command_store(store),
    );
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

fn callback_source_analysis(source: &str, release: &str) -> AnalysisResult {
    callback_source_analysis_with_store(source, release, callback_source_store("dict"))
}

fn callback_source_spans(
    source: &str,
    analysis: &AnalysisResult,
    method: &str,
) -> Vec<tcl_lexer::Span> {
    let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
    scan_method_sites_by_kind(source, analysis, &[body], method, None, true, true)
}
