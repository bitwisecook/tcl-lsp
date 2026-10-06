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

//! Descriptor-selected case bodies, with exact entered list-element provenance.

use super::{
    Arc, CommandAllocationSite, ExecutedScriptSource, ModuleCommandBindings, SourceCommandBindings,
    SourceExecutionContext, SourceNativeInvocation, SourceOutcomes, boxed_source_branch,
    compiled_invocation, evaluated_script_realm, opaque_source_invocation,
};

impl SourceCommandBindings {
    pub(super) fn walk_case_bodies(
        &mut self,
        selection: &tcl_registry::case_bodies::CaseBodyOperands,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let incoming = boxed_source_branch(state);
        let mut outcomes = SourceOutcomes::normal(&incoming);
        for operand in &selection.bodies {
            let mut branch = boxed_source_branch(&incoming);
            outcomes.join(&self.walk_case_body(*operand, native, &mut branch, context));
        }
        if selection.selection_unknown {
            let mut uncertain = boxed_source_branch(&incoming);
            outcomes.join(&opaque_source_invocation(&mut uncertain));
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_case_body(
        &mut self,
        operand: tcl_registry::body_execution::BodyOperand,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(element) = operand.list_element else {
            return self.walk_body_operand(
                operand.argument,
                native.script_operands(),
                state,
                &context,
            );
        };
        let operands = native.script_operands();
        let realm =
            evaluated_script_realm(operand.argument..operand.argument + 1, operands, context);
        let context = compiled_invocation::body_context(operand.argument, operands, state, context);
        let context = SourceExecutionContext { realm, ..context };
        let Some(value) = operands.arguments.literal_at(operand.argument) else {
            return opaque_source_invocation(state);
        };
        let Some(origin) = state.current_source_origin.as_ref() else {
            return opaque_source_invocation(state);
        };
        let parent = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: context.invocation_offset,
        };
        let word = operand
            .argument
            .checked_sub(operands.target.prepended.len())
            .and_then(|index| operands.words.get(index + 1));
        let list = word.map_or_else(
            || ExecutedScriptSource::materialised(parent.clone(), vec![operand.argument], value),
            |word| {
                ExecutedScriptSource::from_word(
                    parent.clone(),
                    operand.argument,
                    word,
                    value,
                    context.config,
                )
            },
        );
        let Some(script) = list.list_element(
            parent.clone(),
            operand.argument,
            element,
            tcl_syntax::word_rules::WordValueRules::from_config(&context.config),
        ) else {
            return opaque_source_invocation(state);
        };
        if let Some(word) = word {
            self.record_executed_word(
                &parent.source,
                word.source().span,
                Some(element),
                script.clone(),
            );
        }
        let previous = state
            .current_source_origin
            .replace(Arc::clone(&script.origin));
        let mut outcomes = self.walk_source_image(&script.text, script.base(), state, &context);
        outcomes.restore_source_origin(previous.as_ref());
        outcomes.publish(state);
        outcomes
    }
}
