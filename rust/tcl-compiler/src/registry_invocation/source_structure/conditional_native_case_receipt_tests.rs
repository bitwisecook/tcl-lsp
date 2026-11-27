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

fn source_context(source: &str) -> OriginalSourceRegistryContext {
    source_context_in(source, "tcl9.0")
}

fn source_context_in(source: &str, dialect: &str) -> OriginalSourceRegistryContext {
    let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let generation = crate::environment_ingress::context_for_profile(profile);
    let context =
        std::sync::Arc::new(generation.with_command_store(registry.snapshot().shared_registry()));
    OriginalSourceRegistryContext::capture(
        source,
        crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_dialect(dialect),
        ),
    )
}

fn child_command(syntax: &OriginalSourceRegistryContext) -> tcl_source::segment::SegmentedCommand {
    let source = syntax.source_image().try_text().unwrap();
    let config = syntax.editing_input().lexer_config();
    let plan = tcl_lexer::native_script_words_in(
        syntax.source_image().clone(),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .unwrap();
    let parent = plan.commands.last().unwrap();
    let child = original_single_source_command_substitution(parent.words.last().unwrap()).unwrap();
    let original = &child.command().words;
    let start = original.first().unwrap().span().start();
    let end = original.last().unwrap().span().end();
    let mut commands = tcl_source::segment::segment_commands_with_offset_and_config(
        &source[usize::try_from(start).unwrap()..usize::try_from(end).unwrap()],
        start,
        config,
    );
    assert_eq!(commands.len(), 1);
    commands.remove(0)
}

#[test]
fn nested_native_case_receipt_retains_actual_vector_unknown_selection_and_debts() {
    let source = "mystery; set result [switch $kind {alpha {# first\nputs first} beta {# second\nputs second}}]";
    let syntax = source_context_in(source, "tcl8.4");
    let command = child_command(&syntax);
    let context = syntax.editing_input().context_registry();
    let words = syntax.conditional_native_case_words(&command).unwrap();
    let OriginalRegistrySource::ConditionalNativeSource(applicability) = words.source() else {
        panic!("case syntax cannot borrow ordinary invocation selection");
    };
    let (input, advice) = applicability.source_advice.as_ref().unwrap();
    assert_eq!(input, syntax.editing_input());
    assert!(applicability.has_unknown_residual());
    assert!(applicability.may_be_absent());
    assert!(words.roles().is_none());
    assert!(!words.operands_preserve_source_lookup());
    assert!(matches!(
        words.arguments().first(),
        Some(crate::registry_invocation::EffectiveInvocationWord::Dynamic)
    ));
    assert_eq!(advice.original_words().len(), command.argv.len());
    assert!(advice.obligations().contains(
        &crate::command_binding::SourceCommandTransitionObligation::UnavailableActualLookup
    ));
    assert!(advice.obligations().contains(
        &crate::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation
    ));
    assert_ne!(advice.uncertain_operations(), []);
    let candidates = words.conditional_source_case_bodies(&context).unwrap();
    assert!(candidates.selection_unknown());
    assert!(candidates.matches_source(syntax.source_image(), input.lexer_config()));
    assert_eq!(
        candidates
            .bodies()
            .iter()
            .map(|body| &source[body.content_span().as_range()])
            .collect::<Vec<_>>(),
        vec!["# first\nputs first", "# second\nputs second"]
    );
    assert!(
        super::super::source_scoped_body::source_script_bodies_for_mutation_coverage(
            &words, &context
        )
        .is_none()
    );
}

#[test]
fn nested_native_case_receipt_refuses_changed_input_vector_and_actual_replacement() {
    let source = "set result [switch $kind {alpha {# child\nputs child}}]";
    let syntax = source_context(source);
    let command = child_command(&syntax);
    let input = syntax.editing_input();
    let context = input.context_registry();
    assert!(syntax.conditional_native_case_words(&command).is_some());
    assert!(
        conditional_native_case_source_words(
            &format!("{source}\n"),
            input.lexer_config(),
            syntax.retained_source_realm(),
            input,
            &context,
            &command
        )
        .is_none()
    );
    let mut changed = input.lexer_config();
    changed.leading_bom = tcl_lexer::LeadingBom::Content;
    if changed == input.lexer_config() {
        changed.leading_bom = tcl_lexer::LeadingBom::Skip;
    }
    assert!(
        conditional_native_case_source_words(
            source,
            changed,
            syntax.retained_source_realm(),
            input,
            &context,
            &command
        )
        .is_none()
    );
    let foreign = source_context(source);
    assert!(
        conditional_native_case_source_words(
            source,
            input.lexer_config(),
            syntax.retained_source_realm(),
            foreign.editing_input(),
            &foreign.editing_input().context_registry(),
            &command
        )
        .is_none()
    );
    let mut cut = command;
    let _ = cut.argv.pop();
    let _ = cut.word_fragments.pop();
    let _ = cut.texts.pop();
    let _ = cut.single_token_word.pop();
    assert!(syntax.conditional_native_case_words(&cut).is_none());
    let replaced = source_context(
        "proc switch args {return other}; set result [switch $kind {alpha {# data\nputs data}}]",
    );
    assert!(
        replaced
            .conditional_native_case_words(&child_command(&replaced))
            .is_none()
    );
}
#[test]
fn nested_native_two_argument_case_retains_selected_optionless_protocol() {
    let source = "mystery; set result [switch $kind {alpha {# first\nputs first} beta {# second\nputs second}}]";
    let syntax = source_context(source);
    let command = child_command(&syntax);
    let context = syntax.editing_input().context_registry();
    let words = syntax.conditional_native_case_words(&command).unwrap();
    let OriginalRegistrySource::ConditionalNativeSource(applicability) = words.source() else {
        panic!("case syntax cannot borrow ordinary invocation selection");
    };
    assert!(applicability.has_unknown_residual());
    assert!(applicability.may_be_absent());
    assert!(words.roles().is_none());
    assert!(!words.operands_preserve_source_lookup());
    assert!(matches!(
        words.arguments().first(),
        Some(crate::registry_invocation::EffectiveInvocationWord::Dynamic)
    ));
    let candidates = words.conditional_source_case_bodies(&context).unwrap();
    assert!(!candidates.selection_unknown());
    assert_eq!(
        candidates
            .bodies()
            .iter()
            .map(|body| &source[body.content_span().as_range()])
            .collect::<Vec<_>>(),
        vec!["# first\nputs first", "# second\nputs second"]
    );
    assert!(
        super::super::source_scoped_body::source_script_bodies_for_mutation_coverage(
            &words, &context
        )
        .is_none()
    );
}
