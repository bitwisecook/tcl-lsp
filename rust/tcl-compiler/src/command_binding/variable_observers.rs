// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reached variable callbacks retain the selected physical receiver.

use super::{
    Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceOutcomes,
};
use crate::place::Place;
use tcl_registry::{
    TraceOperation, completion::CompletionCode,
    completion_route::InvocationCompletionRoute as Route,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActiveVariableObserver {
    lease: crate::captured_cell::CapturedCellLeaseId,
}

impl ActiveVariableObserver {
    fn protects(&self, access: &Place, state: &crate::var_resolve::ResolveContext) -> bool {
        state
            .captured_cell_receiver(self.lease)
            .is_some_and(|receiver| receiver.cell == access.cell && receiver.index == access.index)
    }
}

struct CapturedStorePublication {
    receiver: Place,
    value: Option<String>,
    route: Route,
}

impl SourceCommandBindings {
    pub(super) fn walk_observed_native_store(
        &mut self,
        native: super::SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        route: Route,
        _incoming: &ModuleCommandBindings,
    ) -> Option<SourceOutcomes> {
        if facts
            .traits
            .contains(tcl_registry::traits::Traits::DESTROYS_VARIABLE)
        {
            return self.walk_observed_native_unset(native, facts, state, context);
        }
        if let Some(outcomes) = self.walk_observed_native_getter(native, facts, state, context) {
            return Some(outcomes);
        }
        if !captured_store_contract(facts, native.invocation.arguments()) {
            return None;
        }
        if self.invalid_increment_amount_input(
            native,
            facts,
            state,
            context,
            tcl_registry::native_rmw::NativeRmwAmountValidation::BeforeRead,
        ) {
            return Some(SourceOutcomes::invocation(
                state,
                Route::Tcl(CompletionCode::Error),
            ));
        }
        let (receiver, reads) = captured_native_receiver(
            native.invocation.arguments(),
            facts,
            &state.source_variables,
            context.registry,
        )?;
        let receiver = &receiver;
        if reads.iter().all(|read| !read.observed)
            && increment_requires_missing_error(
                facts,
                receiver,
                &state.source_variables,
                native.invocation.arguments(),
            )
        {
            return Some(SourceOutcomes::invocation(
                state,
                Route::Tcl(CompletionCode::Error),
            ));
        }
        if reads
            .iter()
            .any(|read| read.cell != receiver.cell || read.index != receiver.index)
        {
            return None;
        }
        if !closed_receiver_observers(receiver, &state.source_variables, context.registry) {
            return None;
        }
        let mut unobserved = receiver.clone();
        unobserved.observed = false;
        if state.source_variables.store_would_error(&unobserved)
            && !(receiver.index.is_none() && reads.iter().any(|read| read.observed))
        {
            return Some(SourceOutcomes::invocation(
                state,
                Route::Tcl(CompletionCode::Error),
            ));
        }
        let lease = Arc::make_mut(&mut state.source_variables).capture_protected_cell(receiver)?;
        let mut outcomes = if reads.is_empty() {
            SourceOutcomes::normal(state)
        } else {
            self.walk_variable_observers(
                native.segment.span.start(),
                receiver,
                native
                    .invocation
                    .arguments()
                    .literal_at(facts.argument_offset),
                TraceOperation::Read,
                state,
                context,
            )
        };
        let continuing = take_read_continuations(
            &mut outcomes,
            facts,
            &state.source_variables,
            native.invocation.arguments(),
        );
        for (branch, read_failed) in continuing {
            *state = *branch;
            outcomes.join(&self.finish_captured_native_store(
                native,
                facts,
                state,
                context,
                route,
                (lease, read_failed),
            ));
        }
        release_captured_lease(&mut outcomes, lease);
        outcomes.publish(state);
        Some(outcomes)
    }

    fn walk_observed_native_getter(
        &mut self,
        native: super::SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        use tcl_registry::native_result::{NativeResultContract, VariableResultPhase};
        let NativeResultContract::VariableValue {
            variable_at,
            phase: VariableResultPhase::AfterRead,
        } = facts.native_result?
        else {
            return None;
        };
        if !closed_variable_effects(facts)
            || !crate::variable_bindings::source_variable_write_places(
                facts,
                native.invocation.arguments(),
                &state.source_variables,
                context.registry,
            )
            .is_empty()
        {
            return None;
        }
        let reads = crate::variable_bindings::source_variable_read_places(
            facts,
            native.invocation.arguments(),
            &state.source_variables,
            context.registry,
        );
        let [receiver] = reads.as_slice() else {
            return None;
        };
        Some(
            self.walk_captured_observed_read(
                native.segment.span.start(),
                receiver,
                native
                    .invocation
                    .arguments()
                    .literal_at(facts.argument_offset + usize::from(variable_at)),
                state,
                context,
            ),
        )
    }

    pub(super) fn walk_captured_observed_read(
        &mut self,
        site: u32,
        receiver: &Place,
        name: Option<&str>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(lease) =
            Arc::make_mut(&mut state.source_variables).capture_protected_cell(receiver)
        else {
            return super::opaque_source_invocation(state);
        };
        let mut outcomes = self.walk_variable_observers(
            site,
            receiver,
            name,
            TraceOperation::Read,
            state,
            context,
        );
        if let Some(normal) = outcomes.normal.take() {
            *state = *normal;
            outcomes.join(&captured_read_result(lease, state, context.registry));
        }
        release_captured_lease(&mut outcomes, lease);
        outcomes.publish(state);
        outcomes
    }

    fn invalid_increment_amount_input(
        &mut self,
        native: super::SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        phase: tcl_registry::native_rmw::NativeRmwAmountValidation,
    ) -> bool {
        self.prepare_increment_amount_hooks(
            native,
            facts.increment_object_protocol(native.invocation.arguments()),
            state,
            context,
            phase,
        );
        invalid_increment_amount(
            facts,
            native.invocation.arguments(),
            &state.source_variables,
            phase,
        )
    }

    fn prepare_increment_amount_hooks(
        &mut self,
        native: super::SourceNativeInvocation<'_>,
        protocol: Option<tcl_registry::native_rmw::NativeIncrementObjectProtocol>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        phase: tcl_registry::native_rmw::NativeRmwAmountValidation,
    ) {
        if let Some(protocol) = protocol
            && protocol.amount_validation() == phase
            && let Some(argument) = protocol.amount_argument()
        {
            let shape = native.script_operands().written_word(argument).map_or(
                tcl_registry::native_compilation::NativeCompilationWordShape::Opaque,
                crate::registry_invocation::native_compilation_word_shape,
            );
            let closed = protocol.embeds_amount(
                native.invocation.arguments(),
                *native.compilation_selection,
                shape,
            ) || super::object_callbacks::scalar_math_input_is_closed(
                native, argument, state, context,
            );
            self.record_object_callback_effects(state, context.invocation_offset, closed);
        }
    }

    fn prepare_increment_old_object_hooks(
        &mut self,
        protocol: Option<tcl_registry::native_rmw::NativeIncrementObjectProtocol>,
        lease: crate::captured_cell::CapturedCellLeaseId,
        read_failed: bool,
        state: &mut ModuleCommandBindings,
        offset: u32,
    ) {
        let variables = &state.source_variables;
        let missing_scalar = variables
            .captured_cell_receiver(lease)
            .is_some_and(|receiver| {
                receiver.index.is_none()
                    && variables.binding_identity == crate::var_resolve::BindingIdentity::Bound
                    && variables.contents_presence(&receiver)
                        == crate::var_resolve::ContentsPresence::Undefined
            });
        let closed = protocol.is_some_and(|protocol| {
            ((read_failed || missing_scalar)
                && protocol.read_policy()
                    == tcl_registry::native_rmw::NativeRmwReadPolicy::InitialiseZero)
                || variables.captured_numeric_input_hooks_closed(lease)
        });
        self.record_object_callback_effects(state, offset, closed);
    }

    fn finish_captured_native_store(
        &mut self,
        native: super::SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        route: Route,
        read: (crate::captured_cell::CapturedCellLeaseId, bool),
    ) -> SourceOutcomes {
        use crate::captured_cell::CapturedCellState;
        let (lease, read_failed) = read;
        let status = state.source_variables.captured_cells.state(lease);
        if matches!(status, CapturedCellState::Retired(_)) {
            return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
        }
        let Some(receiver) = state.source_variables.captured_cell_receiver(lease) else {
            return super::opaque_source_invocation(state);
        };
        let object_protocol = facts.increment_object_protocol(native.invocation.arguments());
        if self.invalid_increment_amount_input(
            native,
            facts,
            state,
            context,
            tcl_registry::native_rmw::NativeRmwAmountValidation::AfterRead,
        ) {
            return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
        }
        if state.source_variables.store_would_error(&receiver) {
            return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
        }
        let increment = facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Incr,
            );
        if increment {
            self.prepare_increment_old_object_hooks(
                object_protocol,
                lease,
                read_failed,
                state,
                context.invocation_offset,
            );
        }
        let value = if increment {
            match captured_increment_value(native, facts, &receiver, read_failed, state, context) {
                Ok(value) => value,
                Err(CapturedStoreFailure::MissingContents) => {
                    return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
                }
                Err(CapturedStoreFailure::UnknownPolicy) => {
                    return super::opaque_source_invocation(state);
                }
            }
        } else {
            native
                .invocation
                .arguments()
                .literal_at(facts.argument_offset + 1)
                .map(str::to_owned)
        };
        let store_route = if !increment || value.is_some() {
            Route::Tcl(CompletionCode::Ok)
        } else {
            route
        };
        self.publish_captured_native_store(
            native,
            facts,
            state,
            context,
            CapturedStorePublication {
                receiver,
                value,
                route: store_route,
            },
        )
    }

    fn publish_captured_native_store(
        &mut self,
        native: super::SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        publication: CapturedStorePublication,
    ) -> SourceOutcomes {
        let CapturedStorePublication {
            receiver,
            value,
            route: store_route,
        } = publication;
        Arc::make_mut(&mut state.source_variables)
            .set_contents_write_source(state.current_source_origin.clone());
        let before_store = state.clone();
        state.record_provider_state_writes(facts, *native.invocation, context.registry);
        Arc::make_mut(&mut state.source_variables).publish_captured_store(
            &receiver,
            value.as_deref(),
            native.segment.span.start(),
        );
        super::source_representation::publish_numeric_store_representation(
            state,
            facts,
            native.invocation.arguments(),
            &receiver,
            native.segment.span.start(),
        );
        super::source_representation::publish_store_representation(
            state,
            facts,
            native.target,
            native.invocation.arguments(),
            context,
            &receiver,
        );
        let mut callbacks = self.walk_variable_observers(
            native.segment.span.start(),
            &receiver,
            native
                .invocation
                .arguments()
                .literal_at(facts.argument_offset),
            TraceOperation::Write,
            state,
            context,
        );
        if let Some(normal) = callbacks.normal.take() {
            callbacks.join(&SourceOutcomes::native_invocation(
                &before_store,
                &normal,
                store_route,
            ));
        }
        callbacks.publish(state);
        callbacks
    }

    fn walk_observed_native_unset(
        &mut self,
        native: super::SourceNativeInvocation<'_>,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        if !closed_variable_effects(facts) {
            return None;
        }
        let arguments = native.invocation.arguments();
        let roles: Vec<_> = facts
            .arg_roles
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
            .collect();
        let [(index, _)] = roles.as_slice() else {
            return None;
        };
        let name = arguments.literal_at(facts.argument_offset + usize::from(*index))?;
        if state
            .source_variables
            .raw_static_unset_error(name, context.registry)
        {
            return None;
        }
        let receiver = crate::var_resolve::resolve_literal_access(
            name,
            &state.source_variables,
            false,
            context.registry,
            TraceOperation::Unset,
        );
        if let Some(plan) = Arc::make_mut(&mut state.source_variables).begin_array_destruction(
            &receiver,
            native.segment.span.start(),
            context.registry,
        ) {
            return Some(self.walk_staged_array_destruction(
                native.segment.span.start(),
                name,
                &plan,
                state,
                context,
            ));
        }
        let projection = state.source_variables.variable_observers_at(
            &receiver,
            TraceOperation::Unset,
            context.registry,
        );
        if projection.unknown_residual {
            return None;
        }
        let mut unobserved = receiver.clone();
        unobserved.observed = false;
        if state.source_variables.contents_presence(&unobserved)
            != crate::var_resolve::ContentsPresence::Defined
            || (unobserved.index.is_some() && state.source_variables.store_would_error(&unobserved))
        {
            return None;
        }
        let variables = Arc::make_mut(&mut state.source_variables);
        variables.invalidate_contents_literals(&unobserved);
        crate::variable_bindings::destroy_captured_cell(
            variables,
            &unobserved,
            native.segment.span.start(),
            context.registry,
        );
        Some(self.walk_selected_variable_observers(
            (
                native.segment.span.start(),
                &receiver,
                Some(name),
                TraceOperation::Unset,
            ),
            &projection,
            state,
            context,
        ))
    }

    pub(super) fn walk_variable_observers(
        &mut self,
        site: u32,
        access: &Place,
        argument_name: Option<&str>,
        operation: TraceOperation,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let projection =
            state
                .source_variables
                .variable_observers_at(access, operation, context.registry);
        self.walk_selected_variable_observers(
            (site, access, argument_name, operation),
            &projection,
            state,
            context,
        )
    }

    pub(super) fn walk_selected_variable_observers(
        &mut self,
        request: (u32, &Place, Option<&str>, TraceOperation),
        projection: &crate::var_resolve::VariableObserverProjection,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let (site, access, argument_name, operation) = request;
        let name = match operation {
            TraceOperation::Read => "read",
            TraceOperation::Write => "write",
            TraceOperation::Unset => "unset",
            _ => return super::opaque_source_invocation(state),
        };
        if operation != TraceOperation::Unset
            && self
                .active_variable_observers
                .iter()
                .any(|observer| observer.protects(access, &state.source_variables))
        {
            return SourceOutcomes::normal(state);
        }
        if projection.unknown_residual {
            return super::opaque_source_invocation(state);
        }
        if projection.callbacks.is_empty() && projection.possible_callbacks.is_empty() {
            return SourceOutcomes::normal(state);
        }
        let lease = if operation == TraceOperation::Unset {
            None
        } else {
            let Some(lease) =
                Arc::make_mut(&mut state.source_variables).capture_protected_cell(access)
            else {
                return super::opaque_source_invocation(state);
            };
            Some(lease)
        };
        let registrations = Arc::clone(&state.source_variables);
        if let Some(lease) = lease {
            self.active_variable_observers
                .push(ActiveVariableObserver { lease });
        }
        if !self.possible_observers_are_neutral(
            request,
            &projection.possible_callbacks,
            state,
            context,
        ) {
            if let Some(lease) = lease {
                self.active_variable_observers.pop();
                Arc::make_mut(&mut state.source_variables)
                    .captured_cells
                    .release(lease);
            }
            return super::opaque_source_invocation(state);
        }
        let mut outcomes = SourceOutcomes::normal(state);
        for callback in &projection.callbacks {
            let Some(continuing) = outcomes.normal.take() else {
                break;
            };
            *state = *continuing;
            if operation != TraceOperation::Unset
                && projection.callbacks.iter().any(|callback| {
                    registrations.trace_registrations.get(&callback.key)
                        != state
                            .source_variables
                            .trace_registrations
                            .get(&callback.key)
                })
            {
                outcomes.join(&super::opaque_source_invocation(state));
                break;
            }
            let (name_argument, index_argument) =
                observer_name_arguments(argument_name, access, context.config);
            let arguments = [
                name_argument,
                index_argument,
                crate::registry_invocation::EffectiveInvocationWord::Literal(name.to_owned()),
            ];
            let mut delivered = self.walk_reached_callback_prefix(
                site,
                Some(&callback.prefix),
                &arguments,
                state,
                context,
            );
            for (route, _) in &mut delivered.abrupt {
                if matches!(route, Route::Tcl(_) | Route::Return(_)) {
                    *route = Route::Tcl(CompletionCode::Error);
                }
            }
            if operation == TraceOperation::Unset {
                delivered = delivered.capture_tcl_completions();
            }
            outcomes.join(&delivered);
        }
        if let Some(lease) = lease {
            self.active_variable_observers.pop();
            release_captured_lease(&mut outcomes, lease);
        }
        outcomes.normal_value = None;
        outcomes.normal_representation = None;
        outcomes.publish(state);
        outcomes
    }

    fn possible_observers_are_neutral(
        &mut self,
        request: (u32, &Place, Option<&str>, TraceOperation),
        prefixes: &[String],
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        let (site, receiver, argument_name, operation) = request;
        let operation_name = match operation {
            TraceOperation::Read => "read",
            TraceOperation::Write => "write",
            TraceOperation::Unset => "unset",
            _ => return false,
        };
        for prefix in prefixes {
            // An unenumerated target may fire or skip. Unknown ordering is
            // irrelevant only when the actual callback has no world effects
            // and cannot fail. This licenses neither a definite registration
            // nor unconditional execution of its body.
            let mut possible = state.clone();
            let (name, index) = observer_name_arguments(argument_name, receiver, context.config);
            let arguments = [
                name,
                index,
                crate::registry_invocation::EffectiveInvocationWord::Literal(
                    operation_name.to_owned(),
                ),
            ];
            let mut delivered = self.walk_reached_callback_prefix(
                site,
                Some(prefix),
                &arguments,
                &mut possible,
                context,
            );
            if operation == TraceOperation::Unset {
                delivered = delivered.capture_tcl_completions();
            }
            if !delivered.abrupt.is_empty()
                || !delivered
                    .normal
                    .as_ref()
                    .is_some_and(|normal| **normal == *state)
            {
                return false;
            }
        }
        true
    }
}

fn captured_read_result(
    lease: crate::captured_cell::CapturedCellLeaseId,
    state: &mut ModuleCommandBindings,
    registry: &tcl_registry::CommandRegistry,
) -> SourceOutcomes {
    use crate::captured_cell::CapturedCellState;
    use crate::var_resolve::ContentsPresence;
    if matches!(
        state.source_variables.captured_cells.state(lease),
        CapturedCellState::Retired(_)
    ) {
        return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
    }
    let Some(receiver) = state.source_variables.captured_cell_receiver(lease) else {
        return super::opaque_source_invocation(state);
    };
    let presence = state.source_variables.contents_presence(&receiver);
    if presence == ContentsPresence::Undefined
        || state.source_variables.store_would_error(&receiver)
    {
        return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
    }
    let mut outcomes = SourceOutcomes::invocation(
        state,
        if presence == ContentsPresence::Defined {
            Route::Tcl(CompletionCode::Ok)
        } else {
            Route::Unknown
        },
    );
    if presence == ContentsPresence::Defined {
        outcomes.normal_representation =
            super::source_representation::FrozenSourceRepresentation::capture(
                state.source_variables.contents_representation_at(&receiver),
                &state.source_variables,
            )
            .map(Arc::new);
    }
    if let Some(value) = state
        .source_variables
        .literal_contents_at(&receiver, registry)
    {
        outcomes.normal_value = Some(Arc::new(super::native_result::EvaluatedSourceValue {
            text: value.to_owned(),
            representation: state.source_variables.contents_representation_at(&receiver),
            numeric: None,
        }));
    }
    outcomes
}

fn take_read_continuations(
    outcomes: &mut SourceOutcomes,
    facts: &tcl_registry::InvocationFacts,
    state: &crate::var_resolve::ResolveContext,
    arguments: tcl_registry::InvocationArguments<'_>,
) -> Vec<(Box<ModuleCommandBindings>, bool)> {
    let mut continuing = Vec::new();
    if let Some(normal) = outcomes.normal.take() {
        continuing.push((normal, false));
    }
    if increment_initialises_zero(facts, state, arguments) {
        for (completion, failed) in &outcomes.abrupt {
            if *completion == Route::Tcl(CompletionCode::Error) {
                continuing.push((failed.clone(), true));
            }
        }
        outcomes
            .abrupt
            .retain(|(completion, _)| *completion != Route::Tcl(CompletionCode::Error));
    }
    continuing
}

fn invalid_increment_amount(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    state: &crate::var_resolve::ResolveContext,
    phase: tcl_registry::native_rmw::NativeRmwAmountValidation,
) -> bool {
    use tcl_registry::native_rmw::{NativeRmwAmountGrammar, NativeRmwOperation};
    if facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Incr,
        )
    {
        return false;
    }
    let Some(dialect) = state.invocation_dialect.or_else(|| arguments.dialect()) else {
        return false;
    };
    if dialect.native_rmw_amount_validation(NativeRmwOperation::Increment) != Some(phase)
        || dialect.native_rmw_amount_grammar(NativeRmwOperation::Increment)
            != Some(NativeRmwAmountGrammar::Integer)
    {
        return false;
    }
    let Some(amount) = arguments.literal_at(facts.argument_offset + 1) else {
        return false;
    };
    tcl_syntax::number::parse_whole_with(
        amount,
        tcl_syntax::number::ParseFlags {
            integer_only: true,
            ..tcl_syntax::number::ParseFlags::for_syntax(dialect.numbers)
        },
    )
    .is_none()
}

fn captured_native_receiver(
    arguments: tcl_registry::InvocationArguments<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &crate::var_resolve::ResolveContext,
    registry: &tcl_registry::CommandRegistry,
) -> Option<(Place, Vec<Place>)> {
    let writes =
        crate::variable_bindings::source_variable_write_places(facts, arguments, state, registry);
    let [receiver] = writes.as_slice() else {
        return None;
    };
    let reads =
        crate::variable_bindings::source_variable_read_places(facts, arguments, state, registry);
    Some((receiver.clone(), reads))
}

fn increment_requires_missing_error(
    facts: &tcl_registry::InvocationFacts,
    receiver: &Place,
    state: &crate::var_resolve::ResolveContext,
    arguments: tcl_registry::InvocationArguments<'_>,
) -> bool {
    facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Incr,
        )
        && state.contents_presence(receiver) == crate::var_resolve::ContentsPresence::Undefined
        && state
            .invocation_dialect
            .or_else(|| arguments.dialect())
            .and_then(|dialect| {
                dialect
                    .native_rmw_read_policy(tcl_registry::native_rmw::NativeRmwOperation::Increment)
            })
            == Some(tcl_registry::native_rmw::NativeRmwReadPolicy::RequireContents)
}

fn increment_initialises_zero(
    facts: &tcl_registry::InvocationFacts,
    state: &crate::var_resolve::ResolveContext,
    arguments: tcl_registry::InvocationArguments<'_>,
) -> bool {
    facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Incr,
        )
        && state
            .invocation_dialect
            .or_else(|| arguments.dialect())
            .and_then(|dialect| {
                dialect
                    .native_rmw_read_policy(tcl_registry::native_rmw::NativeRmwOperation::Increment)
            })
            == Some(tcl_registry::native_rmw::NativeRmwReadPolicy::InitialiseZero)
}

fn closed_receiver_observers(
    receiver: &Place,
    variables: &crate::var_resolve::ResolveContext,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    [TraceOperation::Read, TraceOperation::Write]
        .into_iter()
        .all(|operation| {
            !variables
                .variable_observers_at(receiver, operation, registry)
                .unknown_residual
        })
}

enum CapturedStoreFailure {
    MissingContents,
    UnknownPolicy,
}

fn captured_increment_value(
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    receiver: &Place,
    read_failed: bool,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Result<Option<String>, CapturedStoreFailure> {
    use crate::var_resolve::ContentsPresence;
    let dialect = state
        .source_variables
        .invocation_dialect
        .or_else(|| native.invocation.arguments().dialect());
    let policy = dialect
        .and_then(|dialect| {
            dialect.native_rmw_read_policy(tcl_registry::native_rmw::NativeRmwOperation::Increment)
        })
        .ok_or(CapturedStoreFailure::UnknownPolicy)?;
    let presence = state.source_variables.contents_presence(receiver);
    if presence == ContentsPresence::Undefined
        && policy == tcl_registry::native_rmw::NativeRmwReadPolicy::RequireContents
    {
        return Err(CapturedStoreFailure::MissingContents);
    }
    let old = if read_failed || presence == ContentsPresence::Undefined {
        Some("0")
    } else {
        state
            .source_variables
            .literal_contents_at(receiver, context.registry)
    };
    Ok(old.and_then(|value| {
        crate::variable_bindings::source_increment_contents(
            value,
            native.invocation.arguments(),
            facts.argument_offset,
            &state.source_variables,
            context.registry,
        )
    }))
}

fn release_captured_lease(
    outcomes: &mut SourceOutcomes,
    lease: crate::captured_cell::CapturedCellLeaseId,
) {
    for state in outcomes
        .normal
        .iter_mut()
        .chain(outcomes.abrupt.iter_mut().map(|(_, state)| state))
    {
        Arc::make_mut(&mut state.source_variables)
            .captured_cells
            .release(lease);
    }
}

fn observer_name_arguments(
    reference: Option<&str>,
    receiver: &Place,
    config: tcl_lexer::LexerConfig,
) -> (
    crate::registry_invocation::EffectiveInvocationWord,
    crate::registry_invocation::EffectiveInvocationWord,
) {
    use crate::registry_invocation::EffectiveInvocationWord::{Dynamic, Literal};
    let Some(reference) = reference else {
        return (Dynamic, Dynamic);
    };
    let (root, index) =
        tcl_syntax::naming::split_array_name_braced_for_style(reference, true, config.braced_var);
    let index = if index.is_none() {
        Literal(String::new())
    } else {
        receiver
            .index
            .as_ref()
            .filter(|index| index.kind == crate::place::IndexKind::Literal)
            .map_or(Dynamic, |index| Literal(index.value.clone()))
    };
    (Literal(root.to_owned()), index)
}

fn captured_store_contract(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
) -> bool {
    let count = arguments
        .exact_argv_len()
        .and_then(|length| length.checked_sub(facts.argument_offset));
    let primitive = match facts.operation {
        tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Set,
        ) => count == Some(2),
        tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Incr,
        ) => matches!(count, Some(1 | 2)),
        _ => false,
    };
    primitive && closed_variable_effects(facts)
}

fn closed_variable_effects(facts: &tcl_registry::InvocationFacts) -> bool {
    use tcl_registry::world_effect::{CallbackKinds, EffectAccessMode, WorldStateDomain};
    facts.arg_roles_complete
        && facts
            .state_transitions
            .declared()
            .is_none_or(|transitions| transitions.facts().is_empty())
        && facts.effects.callback().kinds == CallbackKinds::TRACE
        && facts
            .effects
            .accesses()
            .iter()
            .all(|access| match access.domain {
                WorldStateDomain::VariableStore
                | WorldStateDomain::InterpreterResult
                | WorldStateDomain::CompletionState => true,
                WorldStateDomain::VariableTraces => access.mode == EffectAccessMode::Read,
                _ => false,
            })
}

#[cfg(test)]
mod tests {
    use super::SourceCommandBindings;
    use crate::command_binding::SourceAnalysisOptions;

    fn analyse(source: &str) -> SourceCommandBindings {
        analyse_for(source, "tcl8.6")
    }

    fn analyse_for(source: &str, profile: &str) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::find(profile).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    fn original_length_at(bindings: &SourceCommandBindings, source: &str) -> bool {
        bindings
            .invocation_at_source(
                "llength",
                u32::try_from(source.rfind("llength").unwrap()).unwrap(),
            )
            .proved_handler_target()
            .is_some_and(|target| target.registry_backed && target.command == "::llength")
    }

    #[test]
    fn stock_variable_primitive_metadata_closes_the_captured_store_contract() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let arguments = ["a", "1"];
        for name in ["set", "incr"] {
            let words = tcl_registry::InvocationWords::literals(name, &arguments)
                .with_dialect(tcl_registry::InvocationDialect::of_profile(profile));
            let facts = registry
                .resolve_structured_invocation(words, None)
                .resolved()
                .unwrap()
                .facts();
            assert!(
                super::captured_store_contract(&facts, words.arguments()),
                "{name}: operation={:?}, roles={:?}/{}, transitions={:?}, effects={:?}",
                facts.operation,
                facts.arg_roles,
                facts.arg_roles_complete,
                facts.state_transitions,
                facts.effects
            );
        }
    }

    #[test]
    fn known_empty_write_callback_preserves_unrelated_command_identity() {
        let source = "proc watch args {}; trace add variable a write watch; set a 1; llength {a b}";
        assert!(original_length_at(&analyse(source), source));
    }

    #[test]
    fn write_only_observer_preserves_the_increment_command_world() {
        let source =
            "proc watch args {}; trace add variable a write watch; set a 1; incr a; llength {a b}";
        assert!(original_length_at(&analyse(source), source));
    }

    #[test]
    fn array_kind_error_precedes_a_scalar_write_callback() {
        let source = "proc watch args {rename llength savedLength}; set a(k) VALUE; trace add variable a write watch; catch {set a scalar}; llength {a b}";
        assert!(original_length_at(&analyse(source), source));
    }

    #[test]
    fn write_callback_reads_the_already_stored_value() {
        let source = "proc watch args {if {$::a == 1} {rename llength savedLength}}; trace add variable a write watch; set a 1; llength {a b}";
        assert!(!original_length_at(&analyse(source), source));
    }

    #[test]
    fn known_empty_read_callback_preserves_unrelated_command_identity() {
        let source = "proc watch args {}; set a 1; trace add variable a read watch; set copy $a; llength {a b}";
        assert!(original_length_at(&analyse(source), source));
    }

    #[test]
    fn unknown_read_callback_retains_the_opaque_residual() {
        let source =
            "set a 1; trace add variable a read absentCallback; set copy $a; llength {a b}";
        assert!(!original_length_at(&analyse(source), source));
    }

    #[test]
    fn read_callback_command_mutation_is_visible_after_the_read() {
        let source = "proc watch args {rename llength savedLength}; set a 1; trace add variable a read watch; set copy $a; llength {a b}";
        assert!(!original_length_at(&analyse(source), source));
    }
    #[test]
    fn read_callback_recreation_updates_the_original_protected_scalar() {
        let source = "set a 10; proc refill args {unset ::a; set ::a 100}; trace add variable a read refill; incr a; llength stable";
        let bindings = analyse(source);
        let final_call = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        assert!(original_length_at(&bindings, source));
        assert_eq!(
            final_call
                .variable_context
                .constant_values
                .get("::a")
                .map(String::as_str),
            Some("101")
        );
    }

    #[test]
    fn possible_empty_write_callback_does_not_withdraw_the_native_increment() {
        let source = "proc watch args {}; set name [read stdin]; trace add variable $name write watch; set a 1; incr a; llength stable";
        let bindings = analyse(source);
        assert!(original_length_at(&bindings, source));
        let final_call = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        assert!(!final_call.variable_context.dynamic_traces);
        assert_eq!(
            final_call
                .variable_context
                .possible_trace_registrations
                .len(),
            1
        );
    }

    #[test]
    fn possible_mutating_write_callback_retains_the_opaque_obligation() {
        let source = "proc watch args {rename llength saved}; set name [read stdin]; trace add variable $name write watch; set a 1; incr a; llength stable";
        assert!(!original_length_at(&analyse(source), source));
    }

    #[test]
    fn read_callback_alias_retarget_does_not_redirect_the_captured_increment() {
        let source = "set a 10; set b 100; upvar 0 a link; proc retarget args {uplevel 1 {upvar 0 b link}}; trace add variable a read retarget; incr link; llength stable";
        let bindings = analyse(source);
        let final_call = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        assert!(original_length_at(&bindings, source));
        assert_eq!(
            final_call
                .variable_context
                .constant_values
                .get("::a")
                .map(String::as_str),
            Some("11")
        );
        assert_eq!(
            final_call
                .variable_context
                .constant_values
                .get("::b")
                .map(String::as_str),
            Some("100")
        );
        assert_eq!(
            final_call.variable_context.literal_value(
                "link",
                tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
            ),
            Some("100")
        );
    }

    #[test]
    fn deleting_the_original_array_in_a_read_callback_preserves_the_new_root() {
        let source = "set a(k) 10; proc replace args {unset ::a; set ::a(k) 100}; trace add variable a(k) read replace; catch {incr a(k)}; llength stable";
        let bindings = analyse(source);
        let final_call = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        assert!(original_length_at(&bindings, source));
        assert_eq!(
            final_call.variable_context.literal_value(
                "a(k)",
                tcl_registry::model::ingress::static_context_for("tcl8.6").commands()
            ),
            Some("100")
        );
    }
    #[test]
    fn amount_errors_preserve_the_native_read_callback_phase() {
        let source = "proc watch args {rename llength saved}; set a 10; trace add variable a read watch; catch {incr a BAD}; llength stable";
        for (profile, original) in [("tcl8.4", true), ("tcl8.6", false)] {
            assert_eq!(
                original_length_at(&analyse_for(source, profile), source),
                original,
                "{profile}"
            );
        }
    }

    #[test]
    fn root_read_callback_can_replace_array_contents_with_a_scalar() {
        let source = "set a(k) 10; proc replace args {unset ::a; set ::a 20}; trace add variable a read replace; incr a; llength stable";
        let bindings = analyse(source);
        assert!(original_length_at(&bindings, source));
        let final_call = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        assert_eq!(
            final_call
                .variable_context
                .constant_values
                .get("::a")
                .map(String::as_str),
            Some("21")
        );
    }
    #[test]
    fn observed_substitution_returns_the_original_cell_after_its_callback() {
        let source = "set a 10; proc replace args {set ::a 100}; trace add variable a read replace; set copy $a; llength stable";
        let bindings = analyse(source);
        assert!(original_length_at(&bindings, source));
        let final_call = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        assert_eq!(
            final_call
                .variable_context
                .constant_values
                .get("::copy")
                .map(String::as_str),
            Some("100")
        );
    }

    #[test]
    fn observed_native_getter_keeps_the_protected_cell_value() {
        let source = "set a 10; proc replace args {set ::a 100}; trace add variable a read replace; set copy [set a]; llength stable";
        let bindings = analyse(source);
        assert!(original_length_at(&bindings, source));
        let final_call = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.rfind("llength").unwrap()).unwrap(),
        );
        assert_eq!(
            final_call
                .variable_context
                .constant_values
                .get("::copy")
                .map(String::as_str),
            Some("100")
        );
    }
    #[test]
    fn recreated_root_read_trace_is_independent_of_the_retired_root_guard() {
        let source = "set a(k) 10; proc new args {rename llength savedLength}; proc old args {unset ::a; set ::a(k) 100; trace add variable ::a(k) read new; set ::seen [set ::a(k)]}; trace add variable a(k) read old; catch {incr a(k)}; savedLength stable";
        let bindings = analyse(source);
        let final_call = bindings.invocation_at_source(
            "savedLength",
            u32::try_from(source.rfind("savedLength").unwrap()).unwrap(),
        );
        assert!(
            final_call
                .proved_target()
                .is_some_and(|target| target.registry_backed && target.command == "::llength"),
            "{final_call:?}"
        );
    }
}
