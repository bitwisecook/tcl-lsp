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

use crate::analyser::ResolvedAnalysisInput;
use crate::ir::CommandTokens;
use tcl_registry::{ArgRole, CommandRegistry};

/// Selected original definition syntax, without a handler or entered frame.
pub(crate) struct OriginalLogicalSourceDefinitionLayout {
    pub(crate) descriptor: tcl_registry::CanonicalCommandDescriptorKey<'static>,
    pub(crate) grammar: &'static tcl_registry::definer::DefinitionBodyGrammar,
    pub(crate) name_word: usize,
    pub(crate) body_word: usize,
    pub(crate) per_object: bool,
}

pub(crate) fn original_logical_source_definition_layout(
    registry: &CommandRegistry,
    input: &ResolvedAnalysisInput,
    tokens: &CommandTokens,
) -> Option<OriginalLogicalSourceDefinitionLayout> {
    let binding = tokens.source_binding.as_ref()?;
    // A stronger layout keeps its own terminal selection route.
    if binding.has_original_source_layout_owner() {
        return None;
    }
    let words = binding.original_logical_source_words(tokens, input, registry)?;
    if words.origins().len() != tokens.words().len()
        || !words
            .origins()
            .iter()
            .enumerate()
            .all(|(index, origin)| *origin == super::InvocationWordOrigin::Written(index))
    {
        return None;
    }
    let mut bodies = words
        .written_argument_roles()
        .into_iter()
        .filter_map(|(index, role)| (role == ArgRole::Body).then_some(index));
    let body = bodies.next()?;
    if bodies.next().is_some() {
        return None;
    }
    let original = words.operands().get(body)?.as_ref()?.word()?;
    if original.group().kind != tcl_lexer::WordKind::Braced || original.group().expand {
        return None;
    }
    super::original_written_script_body_span(original)?;
    words.with_source_schema(input.borrowed_context_registry(), |schema| {
        let grammar = schema.authored_source_definition_body_grammar()?;
        let (_, complete) = schema.authored_logical_source_argument_roles();
        if !complete
            || schema.facts().arity_accepts_frozen_arguments() != Some(true)
            || !grammar.family.manufactures_runtime_commands()
            || !schema.authored_source_script_arguments()?.contains(&body)
        {
            return None;
        }
        let name = schema
            .state_transitions()
            .facts()
            .iter()
            .find_map(|fact| match &fact.transition {
                tcl_registry::StateTransition::ObjectDispatch(
                    tcl_registry::ObjectDispatchTransition::Configure { target, .. },
                ) => target.argument_index(),
                _ => None,
            })
            .or_else(|| {
                schema
                    .authored_source_command_publication()
                    .map(|name| name.argument)
            })?;
        let name_word = match words.origins().get(name.checked_add(1)?)? {
            super::InvocationWordOrigin::Written(written) => *written,
            _ => return None,
        };
        (name_word < tokens.words().len()).then_some(OriginalLogicalSourceDefinitionLayout {
            descriptor: schema.descriptor_key(),
            grammar,
            name_word,
            body_word: body.checked_add(1)?,
            per_object: schema.authored_source_descriptors().command.analyser_hook
                == Some(tcl_registry::hooks::AnalyserHookId::OoObjdefine),
        })
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_tokens(source: &str, input: &ResolvedAnalysisInput) -> CommandTokens {
        let image = tcl_lexer::SourceImage::document(source);
        let registry = input.borrowed_context_registry().commands();
        let config = input.lexer_config();
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse_image_in_frame_with_options(
                &image,
                &crate::var_resolve::VariableExecutionFrame::Unknown,
                config,
                registry,
                crate::command_binding::SourceAnalysisOptions::for_logical_source(input).unwrap(),
            )
            .unwrap();
        let commands =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap();
        let [command] = commands.as_slice() else {
            panic!("one original definition")
        };
        let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, command);
        crate::command_binding::RetainedSourceBodyInventory::from(bindings)
            .stamp_original_tokens(&mut tokens);
        tokens
    }

    #[test]
    fn logical_definition_layout_uses_original_source_without_a_handler_grant() {
        let mut analyser = crate::analyser::Analyser::new();
        let input = analyser.prepare_analysis_input("tcl8.6").unwrap();
        let registry = input.borrowed_context_registry().commands();
        let source = "oo::class create C {method m {} {return VALUE}}";
        let tokens = source_tokens(source, &input);
        let selected = original_logical_source_definition_layout(registry, &input, &tokens)
            .expect("actual selected source definition");
        assert_eq!(selected.name_word, 2);
        assert_eq!(selected.body_word, 3);
        assert_eq!(
            selected.grammar.family,
            tcl_registry::definer::DefinerFamily::TclOo
        );
        assert!(!selected.per_object);
        assert!(
            super::super::admitted_native_compiler_invocation(registry, None, &tokens).is_none()
        );
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(registry, input.lexer_config())
                .with_dialect(Some(input.unit_profile()))
                .with_resolved_analysis_input(input.clone()),
            source,
        );
        let method = module
            .methods
            .values()
            .next()
            .expect("original method inventory");
        let definition = module
            .original_definition_script_source(method.definition_scope.unwrap(), registry)
            .expect("same issued source definition");
        assert_eq!(
            &source[definition.content_span().as_range()],
            "method m {} {return VALUE}"
        );
        assert!(
            !module
                .procedures
                .contains_key(&method.definition_scope.unwrap())
        );
    }

    #[test]
    fn logical_definition_layout_refuses_missing_changed_and_expanded_source_owners() {
        let mut analyser = crate::analyser::Analyser::new();
        let input = analyser.prepare_analysis_input("tcl8.6").unwrap();
        let registry = input.borrowed_context_registry().commands();
        let tokens = source_tokens("oo::class create C {method m {} {return VALUE}}", &input);
        assert!(original_logical_source_definition_layout(registry, &input, &tokens).is_some());
        let mut missing = tokens.clone();
        missing.source_binding = None;
        assert!(original_logical_source_definition_layout(registry, &input, &missing).is_none());
        let mut changed = tokens.clone();
        changed.argv_texts[2] = "Other".to_owned();
        assert!(original_logical_source_definition_layout(registry, &input, &changed).is_none());
        let changed_input = ResolvedAnalysisInput::new(
            input.analyser_profile(),
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
            input.context_registry(),
            input.lexer_config(),
        );
        assert!(
            original_logical_source_definition_layout(registry, &changed_input, &tokens).is_none()
        );
        for source in [
            "oo::class create C $body",
            "oo::class {*}{create C} {method m {} {return VALUE}}",
        ] {
            let original = source_tokens(source, &input);
            assert!(
                original_logical_source_definition_layout(registry, &input, &original).is_none()
            );
        }
    }
}
