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

//! The outcome set of a walked command: alternatives, completion receipts, joins and publication.

use super::super::command_identity::CommandAllocationSite;
use super::super::source_origin::SourceOriginId;
use super::super::source_point_entry::OriginalSourcePointEntry;
use super::super::{
    ModuleCommandBindings, SourceExecutionContext, SourceNativeInvocation, SourceNormalCompletion,
    SourceOutcomes, native_result, root_owner_work, source_command_world,
};
use super::execution_context::SourceBodyEntryPurpose;
use std::sync::Arc;
use tcl_registry::InvocationFacts;

pub(in super::super) fn boxed_source_branch(
    state: &ModuleCommandBindings,
) -> Box<ModuleCommandBindings> {
    #[cfg(any(test, debug_assertions))]
    let _timing = root_owner_work::Timing::at(root_owner_work::Query::Branch);
    Box::new(state.clone())
}

/// Move a completed branch without materialising its owned state in a driver
/// frame that remains live through the next recursive command/body descent.
#[inline(never)]
pub(in super::super) fn publish_source_branch(
    state: &mut ModuleCommandBindings,
    branch: Box<ModuleCommandBindings>,
) {
    *state = *branch;
}

pub(in super::super) fn opaque_source_invocation(
    state: &mut ModuleCommandBindings,
) -> SourceOutcomes {
    state.mark_opaque_binding_mutation();
    SourceOutcomes::invocation(
        state,
        tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
    )
}

/// A closed native leaf handler at this exact original invocation. A body's
/// normal child result cannot supply the enclosing handler's certificate.
#[derive(Clone, PartialEq, Eq)]
pub(in super::super) struct SourceNativeNormalCompletion {
    pub(in super::super) site: CommandAllocationSite,
    pub(in super::super) config: tcl_lexer::LexerConfig,
    pub(in super::super) words: Vec<crate::ir::WordExpr>,
    pub(in super::super) operation: tcl_registry::SemanticOperationId,
}

impl SourceNativeNormalCompletion {
    pub(in super::super) fn matches(
        &self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        self.config == context.config
            && self.words == native.words
            && self.operation == facts.operation
            && self.site.offset == native.segment.span.start()
            && state.current_source_origin.as_ref() == Some(&self.site.source)
            && !state.source_step_observed()
            && !state.source_execution_observed(None)
    }
}

/// Selected source-only handler's conditional normal route. This receipt has
/// no Native value/store, entered-frame or physical compiler interpretation.
#[derive(Clone, PartialEq, Eq)]
pub(in super::super) struct SourceConditionalHandlerNormal {
    pub(in super::super) site: CommandAllocationSite,
    pub(in super::super) config: tcl_lexer::LexerConfig,
    pub(in super::super) words: Vec<crate::ir::WordExpr>,
    pub(in super::super) operation: tcl_registry::SemanticOperationId,
    pub(in super::super) entry: OriginalSourcePointEntry,
}

impl SourceConditionalHandlerNormal {
    pub(in super::super) fn matches(
        &self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        state.baseline.source_point_entry.as_ref() == Some(&self.entry)
            && context.body_entry() == SourceBodyEntryPurpose::Conditional
            && context.compilation()
                == tcl_registry::native_compilation::NativeCompilationContext::default()
            && self.config == context.config
            && self.words == native.words
            && self.operation == facts.operation
            && self.site.offset == native.segment.span.start()
            && state.current_source_origin.as_ref() == Some(&self.site.source)
            && !state.source_step_observed()
            && !state.source_execution_observed(None)
    }
}

/// Allocate the outcome accumulator outside frames retained by live body
/// recursion. The heap owner preserves all normal/abrupt alternatives.
/// Preserve complete completion premises on the heap across body descent.
/// Construction temporaries end in this leaf before recursion begins.
#[inline(never)]
pub(super) fn boxed_source_completion_receipt<T>(
    capture: impl FnOnce() -> Option<T>,
) -> Option<Box<T>> {
    // Implementation contract: naming.source.recursive-driver-state-transport
    // docs/design/analysis/name-resolution-proofs/recursive-driver-state-transport.md
    capture().map(Box::new)
}

#[inline(never)]
pub(super) fn boxed_source_outcomes() -> Box<SourceOutcomes> {
    Box::new(SourceOutcomes::default())
}

/// Construct the special empty result before returning to native body dispatch.
#[inline(never)]
pub(super) fn join_source_procedure_noop(
    outcomes: &mut SourceOutcomes,
    incoming: &ModuleCommandBindings,
) {
    let mut result = SourceOutcomes::normal(incoming);
    result.normal_value = Some(Arc::new(native_result::EvaluatedSourceValue {
        text: String::new(),
        representation: tcl_syntax::value::ValueRepresentation::Unknown,
        numeric: None,
    }));
    outcomes.join(&result);
}

impl SourceOutcomes {
    #[cfg(debug_assertions)]
    pub(super) fn trace_original_namespace_child(&self, stage: &str, offset: u32) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMESPACE_CHILD stage={stage} site={offset} normal={} complete={} native_complete={} conditional_complete={} unclosed={:?} routes={:?}",
                self.normal.is_some(),
                self.normal_completion.is_some(),
                self.native_normal_completion.is_some(),
                self.conditional_normal_completion.is_some(),
                self.normal
                    .as_ref()
                    .map(|normal| normal.unclosed_command_world),
                self.abrupt
                    .iter()
                    .map(|(route, _)| route)
                    .collect::<Vec<_>>(),
            );
        }
    }

    pub(in super::super) fn retain_closed_native_handler_completion(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) {
        let Some(normal) = self.normal.as_ref().filter(|_| self.abrupt.is_empty()) else {
            return;
        };
        let Some(source) = normal.current_source_origin.as_ref() else {
            return;
        };
        if normal.source_step_observed() || normal.source_execution_observed(None) {
            return;
        }
        if normal.baseline.source_point_entry.is_some() {
            self.retain_conditional_handler_completion(native, facts, context);
            return;
        }
        self.native_normal_completion = Some(Arc::new(SourceNativeNormalCompletion {
            site: CommandAllocationSite {
                source: Arc::clone(source),
                offset: native.segment.span.start(),
            },
            config: context.config,
            words: native.words.to_vec(),
            operation: facts.operation,
        }));
        self.retain_complete_normal_evaluation();
    }

    pub(super) fn retain_conditional_handler_completion(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) {
        let Some(normal) = self.normal.as_ref().filter(|_| self.abrupt.is_empty()) else {
            return;
        };
        let Some(entry) = normal.baseline.source_point_entry.as_ref() else {
            return;
        };
        if context.body_entry() != SourceBodyEntryPurpose::Conditional
            || context.compilation()
                != tcl_registry::native_compilation::NativeCompilationContext::default()
            || normal.baseline.native_entry.is_some()
            || normal.source_step_observed()
            || normal.source_execution_observed(None)
        {
            return;
        }
        let Some(source) = &normal.current_source_origin else {
            return;
        };
        self.conditional_normal_completion = Some(Arc::new(SourceConditionalHandlerNormal {
            site: CommandAllocationSite {
                source: Arc::clone(source),
                offset: native.segment.span.start(),
            },
            config: context.config,
            words: native.words.to_vec(),
            operation: facts.operation,
            entry: entry.clone(),
        }));
        self.retain_complete_normal_evaluation();
    }

    pub(super) fn discard_normal_handler_entry(&mut self) {
        self.normal = None;
        self.normal_value = None;
        self.normal_name_value = None;
        self.normal_representation = None;
        self.normal_object = None;
        self.normal_method_prefix = None;
        self.normal_rhs_read = None;
    }

    /// The completed command world is the world on the normal-completion
    /// path of the whole document. A catalogued error never reaches it, so it
    /// does not withhold that world; any other abrupt route (a top-level
    /// `return`, `break`, exit or tail call) may also finish the document
    /// elsewhere and keeps it withheld. Unknown normal-return
    /// effects stay withheld by the opaque and observer premises.
    pub(in super::super) fn completed_normal_world(&self) -> Option<Arc<ModuleCommandBindings>> {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let normal = source_command_world::trace_completed_world_option(
            "root.normal-state",
            self.normal.as_ref(),
        )?;
        // An `Unknown` route (`puts` on a reflected or transformed channel)
        // may have run handler scripts that redefined commands, so only
        // catalogued errors leave the normal-path world complete.
        let errors_only = self.abrupt.iter().all(|(route, _)| {
            matches!(
                route,
                Route::Tcl(tcl_registry::completion::CompletionCode::Error)
            )
        });
        // `original_completed_command_world` separately refuses a withdrawn
        // command world and unknown entry.
        let errors_only =
            source_command_world::trace_completed_world_flag("root.errors-only", errors_only);
        let closed = (source_command_world::trace_completed_world_flag(
            "root.normal-completion",
            self.normal_completion.is_some(),
        ) && source_command_world::trace_completed_world_flag(
            "root.no-abrupt",
            self.abrupt.is_empty(),
        )) || (errors_only
            && !source_command_world::trace_completed_world_flag(
                "root.unclosed-world",
                normal.unclosed_command_world,
            )
            && !source_command_world::trace_completed_world_flag(
                "root.opaque-domain",
                normal.has_opaque_domain(),
            )
            && !source_command_world::trace_completed_world_flag(
                "root.source-step",
                normal.source_step_observed(),
            )
            && !source_command_world::trace_completed_world_flag(
                "root.source-execution",
                normal.source_execution_observed(None),
            )
            && source_command_world::trace_completed_world_flag(
                "root.original-unknown-handler",
                normal.unknown_handler_is_original(),
            ));
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_COMPLETED_NORMAL_WORLD complete={} unclosed={} routes={:?} opaque={} step={} observed={}",
                self.normal_completion.is_some(),
                normal.unclosed_command_world,
                self.abrupt
                    .iter()
                    .map(|(route, _)| route)
                    .collect::<Vec<_>>(),
                normal.has_opaque_domain(),
                normal.source_step_observed(),
                normal.source_execution_observed(None),
            );
        }
        closed.then(|| Arc::new((**normal).clone()))
    }

    pub(in super::super) fn retain_complete_normal_evaluation(&mut self) {
        self.normal_completion =
            (self.normal.is_some() && self.abrupt.is_empty()).then_some(SourceNormalCompletion);
    }

    pub(in super::super) fn normal(state: &ModuleCommandBindings) -> Self {
        Self {
            normal: Some(boxed_source_branch(state)),
            abrupt: Vec::new(),
            ..Self::default()
        }
    }

    pub(in super::super) fn invocation(
        state: &ModuleCommandBindings,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
    ) -> Self {
        let mut outcomes = Self::default();
        outcomes.add(route, state);
        outcomes
    }

    pub(in super::super) fn native_invocation(
        before: &ModuleCommandBindings,
        after: &ModuleCommandBindings,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
    ) -> Self {
        let mut outcomes = Self::default();
        if route.normal_possible() {
            outcomes.normal = Some(boxed_source_branch(after));
        }
        if route.abrupt_possible() {
            let mut partial = boxed_source_branch(before);
            partial.join(after);
            outcomes.add_abrupt(route, &partial);
        }
        outcomes
    }

    pub(in super::super) fn add(
        &mut self,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        state: &ModuleCommandBindings,
    ) {
        if route.normal_possible() {
            self.normal_value = None;
            self.normal_name_value = None;
            self.normal_representation = None;
            self.normal_object = None;
            self.normal_method_prefix = None;
            self.normal_rhs_read = None;
            if let Some(normal) = &mut self.normal {
                normal.join(state);
            } else {
                self.normal = Some(boxed_source_branch(state));
            }
        }
        if route.abrupt_possible() {
            self.add_abrupt(route, state);
        }
    }

    pub(in super::super) fn add_abrupt(
        &mut self,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        state: &ModuleCommandBindings,
    ) {
        self.normal_completion = None;
        self.complete_procedure_return = None;
        self.abrupt_values.retain(|(known, _)| *known != route);
        self.abrupt_name_values.retain(|(known, _)| *known != route);
        self.abrupt_objects.retain(|(known, _)| *known != route);
        if matches!(
            route,
            tcl_registry::completion_route::InvocationCompletionRoute::TailcallOrError { .. }
        ) {
            for alternative in route.alternatives() {
                self.add_abrupt(alternative, state);
            }
            return;
        }
        if let tcl_registry::completion_route::InvocationCompletionRoute::TclAlternatives(codes) =
            route
        {
            for code in codes {
                if *code != tcl_registry::completion::CompletionCode::Ok {
                    self.add_abrupt(
                        tcl_registry::completion_route::InvocationCompletionRoute::Tcl(*code),
                        state,
                    );
                }
            }
            return;
        }
        if let Some((_, previous)) = self
            .abrupt
            .iter_mut()
            .find(|(previous, _)| *previous == route)
        {
            previous.join(state);
        } else {
            self.abrupt.push((route, boxed_source_branch(state)));
        }
    }

    pub(in super::super) fn join(&mut self, other: &Self) {
        let complete_return = if self.normal.is_none() && self.abrupt.is_empty() {
            other.complete_procedure_return
        } else {
            self.complete_procedure_return
                .filter(|proof| Some(*proof) == other.complete_procedure_return)
        };
        let complete = if self.normal.is_none() && self.abrupt.is_empty() {
            other.normal_completion
        } else {
            self.normal_completion
                .filter(|proof| Some(*proof) == other.normal_completion)
        };
        if let Some(normal) = &other.normal {
            let representation = if self.normal.is_none()
                || self.normal_representation == other.normal_representation
            {
                other.normal_representation.clone()
            } else {
                None
            };
            let object = if self.normal.is_none() || self.normal_object == other.normal_object {
                other.normal_object.clone()
            } else {
                None
            };
            let rhs_read = if self.normal.is_none() || self.normal_rhs_read == other.normal_rhs_read
            {
                other.normal_rhs_read.clone()
            } else {
                None
            };
            let prefix = if self.normal.is_none()
                || self.normal_method_prefix == other.normal_method_prefix
            {
                other.normal_method_prefix.clone()
            } else {
                None
            };
            let name_value = if self.normal.is_none() {
                other.normal_name_value.clone()
            } else {
                self.normal_name_value
                    .as_ref()
                    .zip(other.normal_name_value.as_ref())
                    .and_then(|(left, right)| left.joined(right))
                    .map(Arc::new)
            };
            self.add_with_result(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Ok,
                ),
                normal,
                other.normal_value.as_ref(),
            );
            self.normal_name_value = name_value;
            self.normal_object = object;
            self.normal_method_prefix = prefix;
            self.normal_rhs_read = rhs_read;
            self.normal_representation = representation;
        }
        for (route, state) in &other.abrupt {
            let present = self.abrupt.iter().any(|(known, _)| known == route);
            let name_value = if present {
                self.abrupt_name_values
                    .iter()
                    .find(|(known, _)| known == route)
                    .map(|(_, value)| value)
                    .zip(
                        other
                            .abrupt_name_values
                            .iter()
                            .find(|(known, _)| known == route)
                            .map(|(_, value)| value),
                    )
                    .and_then(|(left, right)| left.joined(right))
                    .map(Arc::new)
            } else {
                other
                    .abrupt_name_values
                    .iter()
                    .find(|(known, _)| known == route)
                    .map(|(_, value)| Arc::clone(value))
            };
            self.join_abrupt_result(
                *route,
                state,
                other.result_for(*route),
                other.object_for(*route),
            );
            if let Some(value) = name_value {
                self.abrupt_name_values.push((*route, value));
            }
        }
        self.normal_completion = complete.filter(|_| self.abrupt.is_empty());
        self.complete_procedure_return = complete_return.filter(|route| {
            self.normal.is_none() && self.abrupt.len() == 1 && self.abrupt[0].0 == *route
        });
    }

    /// Whether an outcome of a member-body walk may hold a command its own
    /// frame published: a changed binding or object, or an unbounded effect
    /// reached in that frame.
    pub(in super::super) fn may_publish_in_receiver_frame(
        &self,
        entry: &ModuleCommandBindings,
    ) -> bool {
        self.normal
            .iter()
            .map(AsRef::as_ref)
            .chain(self.abrupt.iter().map(|(_, state)| state.as_ref()))
            .any(|state| {
                state.receiver_frame_opacity
                    || state.bindings != entry.bindings
                    || state.objects != entry.objects
            })
    }

    /// Whether every outcome leaves the command world exactly as `entry`
    /// held it: bindings, objects, namespaces, packages and class state.
    pub(in super::super) fn preserves_command_world(&self, entry: &ModuleCommandBindings) -> bool {
        self.normal
            .iter()
            .map(AsRef::as_ref)
            .chain(self.abrupt.iter().map(|(_, state)| state.as_ref()))
            .all(|state| state.same_command_world(entry))
    }

    pub(in super::super) fn publish(&self, state: &mut ModuleCommandBindings) {
        let mut combined = self.normal.clone();
        for (_, abrupt) in &self.abrupt {
            if let Some(combined) = &mut combined {
                combined.join(abrupt);
            } else {
                combined = Some(abrupt.clone());
            }
        }
        if let Some(combined) = combined {
            *state = *combined;
        }
    }

    pub(in super::super) fn through_procedure_boundary(
        &self,
        dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Self {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let mut outcomes = Self::default();
        let mut representation = self.normal.as_ref().and_then(|normal| {
            self.normal_representation
                .as_ref()
                .filter(|receipt| receipt.is_current(&normal.source_variables))
                .cloned()
        });
        if let Some(normal) = &self.normal {
            let name_value = self
                .normal_name_value
                .as_ref()
                .filter(|value| value.is_current(&normal.source_variables))
                .cloned();
            outcomes.add_with_object_result(
                Route::Tcl(tcl_registry::completion::CompletionCode::Ok),
                normal,
                self.normal_value.as_ref(),
                self.normal_object.as_ref(),
            );
            outcomes.normal_name_value = name_value;
        }
        for (route, state) in &self.abrupt {
            let converted = route.through_procedure_boundary_in(dialect);
            if converted.normal_possible() {
                // Abrupt results have no representation receipt. Their
                // conversion cannot inherit the fall-through result's object.
                representation = None;
            }
            let name_value = self
                .abrupt_name_values
                .iter()
                .find(|(known, _)| known == route)
                .map(|(_, value)| value)
                .filter(|value| value.is_current(&state.source_variables));
            let joined_name = if converted.normal_possible() {
                if outcomes.normal.is_some() {
                    outcomes
                        .normal_name_value
                        .as_ref()
                        .zip(name_value)
                        .and_then(|(left, right)| left.joined(right))
                        .map(Arc::new)
                } else {
                    name_value.cloned()
                }
            } else {
                outcomes.normal_name_value.clone()
            };
            outcomes.add_with_object_result(
                converted,
                state,
                self.result_for(*route),
                self.object_for(*route),
            );
            outcomes.normal_name_value = joined_name;
            if converted.abrupt_possible()
                && let Some(value) = name_value
            {
                outcomes
                    .abrupt_name_values
                    .push((converted, Arc::clone(value)));
            }
        }
        outcomes.normal_completion = if self.complete_procedure_return.is_some()
            && self.normal.is_none()
            && self.abrupt.len() == 1
            && outcomes.normal.is_some()
            && outcomes.abrupt.is_empty()
        {
            Some(SourceNormalCompletion)
        } else {
            self.normal_completion
                .filter(|_| self.abrupt.is_empty() && outcomes.abrupt.is_empty())
        };
        outcomes.normal_representation = representation.filter(|receipt| {
            outcomes
                .normal
                .as_ref()
                .is_some_and(|normal| receipt.is_current(&normal.source_variables))
        });
        outcomes
    }

    pub(in super::super) fn capture_with(
        &self,
        selection: tcl_registry::catch_invocation::CatchInvocation,
    ) -> Self {
        let mut captured = Self::default();
        if let Some(normal) = &self.normal {
            captured.join(&Self::normal(normal));
        }
        for (route, state) in &self.abrupt {
            let selected = selection.route(*route);
            if selected.captured {
                captured.join(&Self::normal(state));
            }
            for route in selected.propagated {
                captured.add(route, state);
            }
        }
        captured
    }

    pub(in super::super) fn capture_tcl_completions(&self) -> Self {
        self.capture_with(tcl_registry::catch_invocation::CatchInvocation::CAPTURE_TCL_PHASE)
    }

    pub(in super::super) fn restore_frame(
        &mut self,
        parent: &crate::var_resolve::ResolveContext,
        frame: &crate::var_resolve::VariableExecutionFrame,
    ) {
        for state in self
            .normal
            .iter_mut()
            .chain(self.abrupt.iter_mut().map(|(_, state)| state))
        {
            state.source_variables = Arc::new(crate::var_resolve::restore_execution_frame(
                parent,
                &state.source_variables,
            ));
            state.variable_frame = frame.clone();
        }
    }

    pub(in super::super) fn restore_source_origin(&mut self, origin: Option<&Arc<SourceOriginId>>) {
        for state in self
            .normal
            .iter_mut()
            .chain(self.abrupt.iter_mut().map(|(_, state)| state))
        {
            state.current_source_origin = origin.cloned();
        }
    }
}
