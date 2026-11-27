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

#[test]
fn original_body_parameter_source_refuses_unknown_lookup_and_replaced_handlers() {
    for source in [
        "proc p {name} {unknown_effect; upvar 1 $name cell}",
        "proc upvar args {}; proc p {name} {upvar 1 $name cell}",
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "jimtcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        let realm = analysis.retained_command_realm().unwrap();
        let commands = tcl_source::segment::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        );
        let script = realm
            .executed_script_for_word(commands.last().unwrap().argv[3].span)
            .unwrap();
        let body = script.try_text().unwrap();
        let segments = tcl_source::segment::segment_commands_with_offset_and_config(
            body,
            0,
            input.lexer_config().nested(),
        );
        let view = segmented_body_assistance(
            &tcl_registry::model::DocumentCommandSurface::new(registry, None),
            realm.source_bindings_ref(),
            body,
            input.lexer_config(),
            segments.last().unwrap(),
            script.base(),
            Some(input.borrowed_context_registry()),
        )
        .unwrap();
        assert!(
            view.conditional_parameter_source_invocation(&["name"])
                .is_none(),
            "{source}"
        );
        assert!(view.body_parameter_source.is_none(), "{source}");
    }
}

fn assert_jim_parameter_source_refuses_foreign_inputs(
    source: &str,
    registry: &CommandRegistry,
    input: &crate::analyser::ResolvedAnalysisInput,
    metadata: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
    segments: &[tcl_source::segment::SegmentedCommand],
) {
    let second_offset = u32::try_from(source.rfind("upvar").unwrap()).unwrap();
    let mut second_tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        input.lexer_config(),
        &segments[0].clone().shifted_by(second_offset),
    );
    bindings.stamp_original_tokens(&mut second_tokens);
    let second = second_tokens
        .source_binding
        .as_ref()
        .unwrap()
        .original_body_parameter_source_scope(&second_tokens)
        .expect("actual original second Jim source body scope");
    assert!(
        second
            .original_words(tokens, input.lexer_config(), registry)
            .is_none()
    );
    let mut changed = (*tokens).clone();
    changed.argv_texts[2] = "foreign".to_owned();
    assert!(
        original_body_parameter_source_invocation(
            registry,
            metadata,
            &changed,
            bindings,
            input.lexer_config()
        )
        .is_none()
    );
    let mut config = input.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    assert!(
        original_body_parameter_source_invocation(registry, metadata, tokens, bindings, config,)
            .is_none()
    );
    let changed = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        input.context_registry(),
        config,
    );
    let metadata = InvocationMetadataContext::for_analysis_input(registry, &changed).unwrap();
    assert!(
        original_body_parameter_source_invocation(
            registry,
            metadata,
            tokens,
            bindings,
            input.lexer_config()
        )
        .is_none()
    );
}

#[test]
fn original_body_parameter_source_keeps_jim_formals_scope_and_execution_refusals() {
    let source = "proc p {name spare} {upvar 1 $name cell; set cell VALUE}\nproc q {name spare} {upvar 1 $name cell; set cell VALUE}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "jimtcl");
    let input = analysis.resolved_input.as_ref().unwrap();
    let registry = input.borrowed_context_registry().commands();
    let realm = analysis.retained_command_realm().unwrap();
    let bindings = realm.source_bindings_ref();
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(
        source,
        0,
        input.lexer_config(),
    );
    let script = realm
        .executed_script_for_word(commands[0].argv[3].span)
        .unwrap();
    let body = script.try_text().unwrap();
    let segments = tcl_source::segment::segment_commands_with_offset_and_config(
        body,
        0,
        input.lexer_config().nested(),
    );
    let view = segmented_body_assistance(
        &tcl_registry::model::DocumentCommandSurface::new(registry, None),
        bindings,
        body,
        input.lexer_config(),
        &segments[0],
        script.base(),
        Some(input.borrowed_context_registry()),
    )
    .unwrap();
    let selected = view
        .conditional_parameter_source_invocation(&["name", "spare"])
        .expect("actual original Jim body/formal source template");
    assert!(!view.parameter_role_advice().is_closed());
    assert!(view.parameter_invocation(&["name", "spare"]).is_none());
    assert!(view.caller_name_template_invocation().is_none());
    assert!(view.definite_invocation.is_none());
    assert!(view.unknown_residual || view.may_be_absent);
    assert!(
        view.conditional_parameter_source_invocation(&["spare", "name"])
            .is_none()
    );
    assert!(
        view.conditional_parameter_source_invocation(&["name"])
            .is_none()
    );
    assert_eq!(
        view.caller_template_literal_data(selected.retained_invocation(), 2)
            .as_deref(),
        Some("cell")
    );
    assert!(
        selected
            .written_argument_roles()
            .contains(&(2, tcl_registry::ArgRole::VarWrite))
    );
    let held = view.body_parameter_source.as_ref().unwrap();
    assert_eq!(held.words.len(), 4);
    assert!(
        held.words
            .iter()
            .all(|word| word.image().bytes() == source.as_bytes())
    );
    assert!(
        held.words
            .iter()
            .all(|word| word.config() == input.lexer_config())
    );
    assert!(!bindings.has_actual_procedure_entry(script));

    let segment = segments[0].clone().shifted_by(script.base());
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        input.lexer_config(),
        &segment,
    );
    bindings.stamp_original_tokens(&mut tokens);
    let metadata = InvocationMetadataContext::for_analysis_input(registry, input).unwrap();
    assert!(
        original_body_parameter_source_invocation(
            registry,
            metadata,
            &tokens,
            bindings,
            input.lexer_config()
        )
        .is_some()
    );
    assert_jim_parameter_source_refuses_foreign_inputs(
        source, registry, input, metadata, &tokens, bindings, &segments,
    );
}
