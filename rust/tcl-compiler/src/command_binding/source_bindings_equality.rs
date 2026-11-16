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

//! Exact equality of two source interpretations and the test mirror naming the differing field.

use super::SourceCommandBindings;
#[cfg(test)]
use crate::command_binding::invocation_reads;

// Derived lookup caches do not change the retained source interpretation.
impl PartialEq for SourceCommandBindings {
    fn eq(&self, other: &Self) -> bool {
        self.analyser_source_settings == other.analyser_source_settings
            && self.conditional_source == other.conditional_source
            && self.declaration_preview_depth == other.declaration_preview_depth
            && self.logical_conditional_source_control == other.logical_conditional_source_control
            && self.lexer_config == other.lexer_config
            && self.declaration_layouts == other.declaration_layouts
            && self.pre_handler_failures == other.pre_handler_failures
            && self.unrepresented_entries == other.unrepresented_entries
            && self.root_origin == other.root_origin
            && self.original_argument_syntax_commands == other.original_argument_syntax_commands
            && self.origin_points == other.origin_points
            && self.implicit_math_invocations == other.implicit_math_invocations
            && self.invocation_reads == other.invocation_reads
            && self.normal_variable_continuations == other.normal_variable_continuations
            && self.original_variable_invocations == other.original_variable_invocations
            && self.invocation_normal_results == other.invocation_normal_results
            && self.conditional_procedure_results == other.conditional_procedure_results
            && self.object_callback_effects == other.object_callback_effects
            && self.rhs_read_store_observations == other.rhs_read_store_observations
            && self.definition_method_references == other.definition_method_references
            && self.expression_preparations == other.expression_preparations
            && self.conditional_expression_evaluations == other.conditional_expression_evaluations
            && self.origin_variable_accesses == other.origin_variable_accesses
            && self.implicit_fixed_math_invocations == other.implicit_fixed_math_invocations
            && self.fixed_math_diagnostic_presence == other.fixed_math_diagnostic_presence
            && self.entered_scripts == other.entered_scripts
            && self.entered_words == other.entered_words
            && self.points == other.points
            && self.dispatch_points == other.dispatch_points
            && self.final_state == other.final_state
            && self.deferred == other.deferred
            && self.rule_declarations == other.rule_declarations
            && self.deferred_outcomes == other.deferred_outcomes
            && self.future_bodies == other.future_bodies
            && self.active_calls == other.active_calls
            && self.recursive_assumptions == other.recursive_assumptions
            && self.recursive_assumption_refused == other.recursive_assumption_refused
            && self.world_preserving_bodies == other.world_preserving_bodies
            && self.receiver_body_changes_command_world == other.receiver_body_changes_command_world
            && self.active_variable_observers == other.active_variable_observers
            && self.called_implementations == other.called_implementations
            && self.phases == other.phases
            && self.lifecycle_coverage == other.lifecycle_coverage
            && self.variable_accesses == other.variable_accesses
            && self.compilation_failures == other.compilation_failures
            && self.compilation_boundaries == other.compilation_boundaries
            && self.compilation_provider_required == other.compilation_provider_required
            && self.compilation_sources == other.compilation_sources
            && self.compiled_children == other.compiled_children
            && self.expression_syntax_children == other.expression_syntax_children
            && self.compiler_invocations == other.compiler_invocations
            && self.runtime_coverage == other.runtime_coverage
    }
}

impl Eq for SourceCommandBindings {}

// Report-bound diagnostic mirror; the ordinary PartialEq implementation stays exact.
#[cfg(test)]
impl SourceCommandBindings {
    pub(crate) fn exact_eq_for_report_trace(&self, other: &Self) -> bool {
        macro_rules! same_fields {
            ($($field:ident),+ $(,)?) => { $(
                let same = self.$field == other.$field;
                if !same {
                    let field = stringify!($field);
                    eprintln!("REPORT_CAPTURE stage=compare.owner.command_realm.bindings field={field} refuses=true");
                    return false;
                }
            )+ };
        }
        same_fields!(
            analyser_source_settings,
            conditional_source,
            declaration_preview_depth,
            logical_conditional_source_control,
            lexer_config,
            declaration_layouts,
            pre_handler_failures,
            unrepresented_entries,
            root_origin,
            original_argument_syntax_commands,
            origin_points,
            implicit_math_invocations,
        );
        if self.invocation_reads != other.invocation_reads {
            let part = invocation_reads::report_trace_invocation_reads_part(
                &self.invocation_reads,
                &other.invocation_reads,
            );
            eprintln!(
                "REPORT_CAPTURE stage=compare.owner.command_realm.bindings.invocation_reads {part} refuses=true"
            );
        }
        same_fields!(
            invocation_reads,
            normal_variable_continuations,
            original_variable_invocations,
            invocation_normal_results,
            conditional_procedure_results,
            object_callback_effects,
            rhs_read_store_observations,
            definition_method_references,
            expression_preparations,
            conditional_expression_evaluations,
            origin_variable_accesses,
            implicit_fixed_math_invocations,
            fixed_math_diagnostic_presence,
            entered_scripts,
            entered_words,
            points,
            dispatch_points,
            final_state,
            deferred,
            rule_declarations,
            deferred_outcomes,
            future_bodies,
            active_calls,
            active_variable_observers,
            called_implementations,
            phases,
            lifecycle_coverage,
            variable_accesses,
            compilation_failures,
            compilation_boundaries,
            compilation_provider_required,
            compilation_sources,
            compiled_children,
            expression_syntax_children,
            compiler_invocations,
            runtime_coverage,
        );
        true
    }
}
