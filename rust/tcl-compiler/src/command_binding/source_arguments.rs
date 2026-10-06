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

//! Original operand freezing shared by argv-first and ordered opcode drivers.

use super::*;

pub(super) struct SourceArgumentSlice<'a> {
    pub source: &'a str,
    pub base: u32,
    pub words: &'a [crate::ir::WordExpr],
    pub selected: std::ops::Range<usize>,
}

impl SourceCommandBindings {
    /// Evaluate only the selected unchanged words, retaining their complete original vector.
    #[inline(never)]
    pub(super) fn prepare_source_argument_range(
        &mut self,
        operands: SourceArgumentSlice<'_>,
        state: &mut ModuleCommandBindings,
        segment: &crate::segmenter::SegmentedCommand,
        context: &SourceExecutionContext<'_>,
    ) -> Box<PreparedSourceArguments> {
        let SourceArgumentSlice {
            source,
            base,
            words,
            selected,
        } = operands;
        let context = *context;
        let argument_owner = source_invocation_argument_owner(
            state,
            segment.span.start(),
            context.variable_read_owner,
        );
        let mut prepared = PreparedSourceArguments::boxed(words.len());
        let mut written_arguments = Vec::with_capacity(words.len());
        for word in &words[selected] {
            let substitutions = self.walk_substitutions(
                word,
                source,
                base,
                state,
                SourceExecutionContext {
                    depth: context.depth + 1,
                    variable_read_owner: argument_owner.as_ref(),
                    ..context
                },
            );
            prepared.complete_normally &=
                substitutions.normal_completion.is_some() && substitutions.abrupt.is_empty();
            for (route, state) in &substitutions.abrupt {
                prepared.outcomes.add_abrupt(*route, state);
            }
            let object = match word {
                crate::ir::WordExpr::Variable { .. }
                | crate::ir::WordExpr::CommandSubstitution { .. } => {
                    substitutions.normal_object.clone()
                }
                _ => None,
            };
            let result = substitutions.normal_value.clone();
            let rhs_read = word
                .sole_command_substitution()
                .and(substitutions.normal_rhs_read.clone());
            let prefix = match word {
                crate::ir::WordExpr::Variable { .. }
                | crate::ir::WordExpr::CommandSubstitution { .. } => {
                    substitutions.normal_method_prefix.clone()
                }
                _ => None,
            };
            // An unstamped constructor shape is an immediate result, not a
            // frozen shared-object receipt. Only the last plain substitution
            // can carry it straight into an unobserved store.
            let representation = substitutions.normal_representation.filter(|receipt| {
                substitutions
                    .normal
                    .as_ref()
                    .is_some_and(|normal| receipt.is_current(&normal.source_variables))
                    || (receipt.is_immediate_created_result()
                        && word.sole_command_substitution().is_some()
                        && std::ptr::eq(word, words.last().unwrap()))
            });
            let Some(continuing) = substitutions.normal else {
                prepared.outcomes.publish(state);
                return prepared;
            };
            publish_source_branch(state, continuing);
            let Ok(frozen) =
                frozen_arguments::freeze_word(word, result.as_deref(), state, context.registry)
            else {
                prepared.outcomes.add_abrupt(
                    tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::completion::CompletionCode::Error,
                    ),
                    state,
                );
                prepared.outcomes.publish(state);
                return prepared;
            };
            prepared
                .effective
                .extend(frozen_arguments::runtime_words(&frozen));
            written_arguments.push(frozen);
            prepared.written_values.push(result);
            let representation = representation.or_else(|| {
                source_representation::FrozenSourceRepresentation::capture_stock_literal(
                    literal_object_pool::SourceOrdinaryLiteralObject::capture_word(word, state)?,
                    &state.source_variables,
                )
                .map(Arc::new)
            });
            prepared
                .written_representations
                .push(representation.map(|receipt| *receipt));
            prepared.written_objects.push(object);
            prepared.written_method_prefixes.push(prefix);
            prepared.written_variable_reads.push(
                self.capture_argument_read(word, state, context.registry)
                    .or_else(|| {
                        argument_reads::FrozenSourceArgumentRead::from_expression(
                            rhs_read,
                            state,
                            context.registry,
                        )
                    }),
            );
        }
        prepared.written_arguments = written_arguments.into();
        prepared.ready = true;
        prepared
    }
}
