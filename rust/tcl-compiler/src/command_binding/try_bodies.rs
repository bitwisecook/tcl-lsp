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

//! Reached try completion routing through the registry-owned clause layout.

use super::{
    ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceNativeInvocation,
    SourceOutcomes, boxed_source_branch, opaque_source_invocation,
};
use tcl_registry::completion::CompletionCode;
use tcl_registry::completion_route::InvocationCompletionRoute as Route;
use tcl_registry::{TryClauseKind, TryControlClause, TryControlInvocation};

impl SourceCommandBindings {
    pub(super) fn walk_try_bodies(
        &mut self,
        selected: Option<&TryControlInvocation>,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(selected) = selected else {
            return opaque_source_invocation(state);
        };
        let body = self.walk_body_operand(
            selected.body_index,
            native.script_operands(),
            state,
            &context,
        );
        let mut handled = SourceOutcomes::default();
        if let Some(normal) = &body.normal {
            handled.join(&self.walk_try_route(
                selected,
                native,
                Route::Tcl(CompletionCode::Ok),
                normal,
                context,
                body.normal_value.as_ref(),
            ));
        }
        for (route, branch) in &body.abrupt {
            handled.join(&self.walk_try_route(
                selected,
                native,
                *route,
                branch,
                context,
                body.result_for(*route),
            ));
        }
        let Some(finally) = selected
            .clauses
            .iter()
            .find(|clause| clause.kind == TryClauseKind::Finally)
        else {
            handled.publish(state);
            return handled;
        };
        let mut completed = SourceOutcomes::default();
        if let Some(normal) = &handled.normal {
            completed.join(&self.walk_try_finally(
                finally.body_index,
                native,
                Route::Tcl(CompletionCode::Ok),
                normal,
                context,
                handled.normal_value.as_ref(),
            ));
        }
        for (route, branch) in &handled.abrupt {
            completed.join(&self.walk_try_finally(
                finally.body_index,
                native,
                *route,
                branch,
                context,
                handled.result_for(*route),
            ));
        }
        completed.publish(state);
        completed
    }

    fn walk_try_route(
        &mut self,
        selected: &TryControlInvocation,
        native: SourceNativeInvocation<'_>,
        route: Route,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        value: Option<&std::sync::Arc<super::native_result::EvaluatedSourceValue>>,
    ) -> SourceOutcomes {
        if matches!(route, Route::ProcessExit) {
            return SourceOutcomes::invocation(state, route);
        }
        let mut outcomes = SourceOutcomes::default();
        let mut unmatched = true;
        for (index, clause) in selected.clauses.iter().enumerate() {
            let matches = match clause.kind {
                TryClauseKind::On(selector) => selector.matches_route(route),
                TryClauseKind::Trap => route
                    .immediate_code()
                    .and_then(|code| (code != CompletionCode::Error).then_some(false)),
                TryClauseKind::Finally => continue,
            };
            if matches == Some(false) {
                continue;
            }
            let mut branch = boxed_source_branch(state);
            if bind_try_outputs(clause, native, &mut branch, context) {
                if let Some(body) = selected.clauses[index..].iter().find(|candidate| {
                    !candidate.fallthrough && candidate.kind != TryClauseKind::Finally
                }) {
                    outcomes.join(&self.walk_body_operand(
                        body.body_index,
                        native.script_operands(),
                        &mut branch,
                        &context,
                    ));
                }
            } else {
                outcomes.join(&SourceOutcomes::invocation(
                    &branch,
                    Route::Tcl(CompletionCode::Error),
                ));
            }
            if matches == Some(true) {
                unmatched = false;
                break;
            }
        }
        if unmatched {
            outcomes.add_with_result(route, state, value);
        }
        outcomes
    }

    fn walk_try_finally(
        &mut self,
        operand: usize,
        native: SourceNativeInvocation<'_>,
        pending: Route,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        value: Option<&std::sync::Arc<super::native_result::EvaluatedSourceValue>>,
    ) -> SourceOutcomes {
        if matches!(pending, Route::ProcessExit) {
            return SourceOutcomes::invocation(state, pending);
        }
        let mut branch = boxed_source_branch(state);
        let mut completed =
            self.walk_body_operand(operand, native.script_operands(), &mut branch, &context);
        if let Some(normal) = completed.normal.take() {
            completed.normal_value = None;
            completed.normal_representation = None;
            completed.add_with_result(pending, &normal, value);
        }
        // Unknown abrupt completion also includes the process-termination
        // alternative, which never executes Tcl finally code.
        if pending.immediate_code().is_none() {
            completed.add_abrupt(pending, state);
        }
        completed
    }
}

fn bind_try_outputs(
    clause: &TryControlClause,
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    let Some(index) = clause.variable_list_index else {
        return true;
    };
    let Some(dialect) = native.invocation.arguments().dialect() else {
        state.mark_opaque_binding_mutation();
        return true;
    };
    let Some(names) = native
        .invocation
        .arguments()
        .literal_at(index)
        .and_then(|text| dialect.word_values.split_list(text).ok())
    else {
        state.mark_opaque_binding_mutation();
        return true;
    };
    for name in names {
        let target = crate::var_resolve::resolve_literal_access(
            &name,
            &state.source_variables,
            false,
            context.registry,
            tcl_registry::TraceOperation::Write,
        );
        if state.source_variables.store_would_error(&target) {
            return false;
        }
        crate::variable_bindings::transfer_captured_variable_outputs(
            std::sync::Arc::make_mut(&mut state.source_variables),
            std::slice::from_ref(&target),
            native.segment.span.start(),
        );
        if target.observed || target.kind == crate::place::PlaceKind::Unknown {
            state.mark_opaque_binding_mutation();
        }
    }
    true
}
