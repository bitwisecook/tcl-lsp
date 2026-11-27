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

//! The walk of one prepared target: its native invocation and the receipts settled around it.

#[cfg(any(test, debug_assertions))]
use super::super::root_owner_work;
use super::super::{
    ModuleCommandBindings, SourceCommandBindings, SourceCommandTarget, SourceExecutionContext,
    SourceNativeInvocation, SourceOutcomes, SourceScriptOperands, container_coercion,
    keyed_update_store, logical_definition, logical_operation, method_prefix, native_result,
    native_string_access, numeric_operand_cache, original_delegated_script, original_name_value,
    source_command_world, source_representation,
};
use super::completion_receipts::{
    CapturedNativeCompletionReceipts, command_world_closed_on_normal_return,
    creates_literal_ensemble, requires_reentry_guard,
};
use super::native_target::{delegates_stock_script_effects, is_implicit_stock_stdout_write};
use super::native_transfer::select_contents_write_source;
use super::outcomes::boxed_source_branch;
use std::sync::OnceLock;
use tcl_registry::{CommandRegistry, InvocationFacts};

/// Resolve the potentially large descriptor snapshot outside recursive source
/// frames. Only its owned heap pointer remains live while nested scripts run.
#[inline(never)]
pub(in super::super) fn resolve_source_invocation_facts(
    registry: &CommandRegistry,
    context: Option<tcl_registry::model::semantic::SemanticContext>,
    invocation: tcl_registry::InvocationWords<'_>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<Box<tcl_registry::InvocationFacts>> {
    tcl_registry::model::semantic::resolve_structured_invocation_in_realm(
        registry, context, invocation, realm,
    )
    .resolved()
    .map(|resolved| Box::new(resolved.facts()))
    .or_else(|| {
        registry
            .native_registration_invocation_facts(invocation)
            .map(Box::new)
    })
}

/// The selected instance method's facts for a call through an instance of
/// the registry constructor `factory`, under the supplied availability when
/// the walk retains one.
#[inline(never)]
fn resolve_source_instance_facts(
    state: &ModuleCommandBindings,
    registry: &CommandRegistry,
    factory: &str,
    invocation: tcl_registry::InvocationWords<'_>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<Box<tcl_registry::InvocationFacts>> {
    if let crate::registry_invocation::OwnedInvocationMetadataContext::SuppliedSource(input) =
        &state.baseline.metadata_context
    {
        let context = input.context_registry();
        let spec = context.commands().get(factory)?;
        return context
            .commands()
            .resolve_structured_instance_invocation_for_descriptor_in_context(
                spec, invocation, &context,
            )
            .ok()
            .flatten()
            .map(|resolved| Box::new(resolved.facts()));
    }
    let query = invocation
        .arguments()
        .dialect()
        .and_then(tcl_native_model::InvocationDialect::authoring_query)
        .map(|query| query.with_realm(realm));
    registry
        .resolve_structured_instance_invocation(factory, invocation, query)
        .map(|resolved| Box::new(resolved.facts()))
}

#[inline(never)]
fn resolve_source_success_facts(
    registry: &CommandRegistry,
    context: Option<tcl_registry::model::semantic::SemanticContext>,
    invocation: tcl_registry::InvocationWords<'_>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<Box<tcl_registry::InvocationFacts>> {
    tcl_registry::model::semantic::resolve_structured_invocation_in_realm(
        registry, context, invocation, realm,
    )
    .resolved()
    .map(|resolved| Box::new(resolved.facts_after_success()))
    .or_else(|| {
        registry
            .native_registration_success_facts(invocation)
            .map(Box::new)
    })
}

pub(in super::super) struct PreparedSourceNativeInvocation<'a> {
    pub(in super::super) arguments: Vec<tcl_registry::InvocationWord<'a>>,
    pub(in super::super) dialect: Option<tcl_native_model::InvocationDialect>,
    pub(in super::super) route: tcl_registry::completion_route::InvocationCompletionRoute,
    pub(in super::super) facts: Box<InvocationFacts>,
    pub(in super::super) original_variable_operands:
        OnceLock<crate::variable_bindings::OriginalVariableInvocation>,
}

impl PreparedSourceNativeInvocation<'_> {
    pub(in super::super) fn completion_route(
        &self,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> tcl_registry::completion_route::InvocationCompletionRoute {
        // Source-inspection proof: naming.list.constructor-intrinsic-return-code
        // docs/design/analysis/name-resolution-proofs/list-constructor-intrinsic-return-code.md
        if self.facts.successful_handler
            == Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::Leaf)
            && let Some(code) = self.facts.native_result.and_then(|contract| {
                contract.list_constructor_completion_code(
                    self.invocation("").arguments(),
                    self.facts.argument_offset,
                )
            })
        {
            return tcl_registry::completion_route::InvocationCompletionRoute::Tcl(code);
        }
        if state.closed_rename_receiver(&self.facts, context)
            || crate::variable_bindings::alias_registration_is_closed(
                &state.source_variables,
                &self.facts,
                self.invocation("").arguments(),
                context.registry,
            )
            || self.original_variable_operands.get().is_some_and(|operands| {
                crate::variable_bindings::variable_trace_registration_is_closed_with_original_operands(
                    &state.source_variables, &self.facts, context.registry, operands,
                )
            })
        {
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Ok,
            )
        } else if creates_literal_ensemble(&self.facts) {
            // It evaluates no script: it installs its one command or raises.
            tcl_registry::completion_route::InvocationCompletionRoute::TclAlternatives(&[
                tcl_registry::completion::CompletionCode::Ok,
                tcl_registry::completion::CompletionCode::Error,
            ])
        } else {
            self.route
        }
    }

    pub(in super::super) fn invocation<'a>(
        &'a self,
        head: &'a str,
    ) -> tcl_registry::InvocationWords<'a> {
        let words = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(head),
            &self.arguments,
        );
        self.dialect
            .map_or(words, |dialect| words.with_dialect(dialect))
    }

    /// Keep the dialect-bearing argv view on the heap through body traversal.
    #[inline(never)]
    pub(in super::super) fn invocation_boxed<'a>(
        &'a self,
        head: &'a str,
    ) -> Box<tcl_registry::InvocationWords<'a>> {
        Box::new(self.invocation(head))
    }
}

pub(in super::super) enum SourceNativePreparationError {
    Unresolved(tcl_registry::completion_route::InvocationCompletionRoute),
    InvalidArguments,
}

/// Materialise argv and descriptors outside frames retained through descent.
#[inline(never)]
pub(in super::super) fn prepare_source_native_invocation<'a>(
    target: &'a SourceCommandTarget,
    effective: &'a [crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    registry: &CommandRegistry,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<Box<PreparedSourceNativeInvocation<'a>>, SourceNativePreparationError> {
    #[cfg(any(test, debug_assertions))]
    let _timing = root_owner_work::Timing::at(root_owner_work::Query::NativeInvocationPreparation);
    let mut arguments = target
        .prepended
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    arguments.extend(
        effective
            .iter()
            .skip(1)
            .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word),
    );
    let dialect = state.baseline.dialect;
    if state.baseline.native_entry.is_some() && dialect.is_none() {
        return Err(SourceNativePreparationError::Unresolved(
            tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
        ));
    }
    let mut invocation = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(&target.command),
        &arguments,
    );
    if let Some(dialect) = dialect {
        invocation = invocation.with_dialect(dialect);
    }
    let route = registry
        .invocation_completion_route_in_frame(
            &target.command,
            invocation.arguments(),
            invocation
                .arguments()
                .dialect()
                .and_then(tcl_native_model::InvocationDialect::authoring_query)
                .map(|query| query.with_realm(realm)),
            state.source_variables.alias_frame(),
        )
        .or_else(|| {
            registry.native_registration_completion_route(
                invocation,
                state.source_variables.alias_frame(),
            )
        })
        .unwrap_or(tcl_registry::completion_route::InvocationCompletionRoute::Unknown);
    let semantic_context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    if let tcl_names::command::BindingKind::Instance { factory } = target.kind {
        // A widget call takes its selected instance method's declared facts;
        // an unknown or computed method stays an unresolved call.
        let facts = resolve_source_instance_facts(state, registry, factory, invocation, realm)
            .ok_or(SourceNativePreparationError::Unresolved(route))?;
        if source_target_arguments_invalid(&facts, invocation.arguments()) {
            return Err(SourceNativePreparationError::InvalidArguments);
        }
        return Ok(Box::new(PreparedSourceNativeInvocation {
            arguments,
            dialect,
            route,
            facts,
            original_variable_operands: OnceLock::new(),
        }));
    }
    let facts = resolve_source_invocation_facts(registry, semantic_context, invocation, realm)
        .ok_or(SourceNativePreparationError::Unresolved(route))?;
    if source_target_arguments_invalid(&facts, invocation.arguments()) {
        return Err(SourceNativePreparationError::InvalidArguments);
    }
    let facts = resolve_source_success_facts(registry, semantic_context, invocation, realm)
        .unwrap_or(facts);
    Ok(Box::new(PreparedSourceNativeInvocation {
        arguments,
        dialect,
        route,
        facts,
        original_variable_operands: OnceLock::new(),
    }))
}

/// What a prepared source target carries across its recursive handler walk,
/// boxed so the recursive frame does not hold it.
struct PreparedSourceTargetWalk {
    completions: CapturedNativeCompletionReceipts,
    receipts: Box<CapturedNativeResultReceipts>,
    read_complete: bool,
    write_complete: bool,
}

struct CapturedNativeResultReceipts {
    string_append: Option<Box<native_string_access::CapturedAppend>>,
    normal_read: Option<crate::var_resolve::OriginalNormalValueRead>,
    normal_write: Option<crate::var_resolve::OriginalNormalValueWrite>,
    range: Option<Box<source_representation::CapturedRangeResult>>,
    pub(in super::super) coercion: Option<Box<container_coercion::CapturedContainerCoercion>>,
    numeric_operands: Vec<numeric_operand_cache::CapturedNumericOperandCache>,
}

/// Build physical operand receipts outside recursive body-driver frames.
#[inline(never)]
fn capture_source_native_results_boxed(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
) -> Box<CapturedNativeResultReceipts> {
    let context = *context;
    Box::new(CapturedNativeResultReceipts {
        string_append: native_string_access::capture_append(native, facts, state, context)
            .map(Box::new),
        normal_read: native_result::capture_original_read_completion(
            native,
            facts,
            state,
            context.registry,
        ),
        normal_write: native_result::original_store_receiver(
            facts,
            native.target,
            native.invocation.arguments(),
            context,
            &state.source_variables,
        )
        .and_then(|(receiver, _)| {
            state
                .source_variables
                .original_normal_value_write(&receiver, context.registry)
        }),
        range: source_representation::capture_range_result(native, facts, state, context),
        coercion: container_coercion::capture(native, facts, state, context).map(Box::new),
        numeric_operands: numeric_operand_cache::capture_indices(native, facts, state, context),
    })
}

/// Unpack post-handler receipts only after recursive body traversal has ended.
#[inline(never)]
fn finish_source_native_results(
    outcomes: &mut SourceOutcomes,
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    incoming: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
    receipts: &mut CapturedNativeResultReceipts,
) {
    let context = *context;
    let range_result = receipts.range.take();
    let coercion = receipts.coercion.take();
    let numeric_operands = std::mem::take(&mut receipts.numeric_operands);
    numeric_operand_cache::finish_indices(outcomes, numeric_operands, context.registry);
    container_coercion::finish(outcomes, coercion.map(|receipt| *receipt), context);
    keyed_update_store::retain_keyed_update_store(outcomes, native, facts, incoming, context);
    source_representation::retain_created_result(
        outcomes,
        native,
        facts,
        context,
        range_result
            .as_deref()
            .map(source_representation::CapturedRangeResult::representation),
    );
    source_representation::retain_range_result_bytes(outcomes, range_result);
    if let Some(append) = receipts.string_append.take() {
        append.finish(outcomes, context);
    }
    native_result::retain_store_results(
        outcomes,
        facts,
        native.target,
        native.invocation.arguments(),
        context,
    );
    method_prefix::retain_created_prefix(outcomes, native, facts, context);
    method_prefix::retain_scoped_prefix(outcomes, native, facts, context);
    method_prefix::retain_prefix_store(
        outcomes,
        facts,
        native.target,
        native.invocation.arguments(),
        context,
    );
}

fn source_target_arguments_invalid(
    facts: &InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
) -> bool {
    use tcl_registry::native_procedure::NativeProcedureDefinitionSelection as Selection;
    if facts.arity_accepts_frozen_arguments() == Some(false) {
        return true;
    }
    let Some(selection) = facts.procedure_definition else {
        return false;
    };
    let definition = match selection {
        Selection::Invalid => return true,
        Selection::Unknown => return false,
        Selection::Valid(definition) => definition,
    };
    let invalid_parameters = arguments
        .literal_at(definition.parameters_at)
        .zip(
            arguments
                .dialect()
                .and_then(tcl_native_model::InvocationDialect::parameter_grammar),
        )
        .is_some_and(|(parameters, grammar)| {
            tcl_syntax::formal_params::parse_formal_parameters_in(parameters, grammar).is_err()
        });
    invalid_parameters
        || definition
            .statics_at
            .and_then(|index| arguments.native_bytes_at(index))
            .zip(arguments.dialect())
            .is_some_and(|(statics, dialect)| {
                tcl_registry::native_procedure::parse_static_variables_bytes(statics, dialect)
                    .is_some_and(|result| result.is_err())
            })
}

/// Whether a step may give the process `stdout` a Tcl callback. Only a
/// channel created or stacked with a Tcl handler (`chan create`, `chan push`)
/// carries one, and those retain a callback; a channel moved from another
/// interpreter may carry one too. Opening, closing, configuring and writing
/// channels, and running subprocesses, cannot: a channel that `open` or
/// `socket` places in a closed `stdout` slot runs no Tcl when written.
/// Unmodelled code reaches the walk as an opaque step, which withdraws the
/// fact on its own.
fn may_change_stock_stdout(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    use tcl_registry::side_effects::SideEffectTarget;
    use tcl_registry::world_effect::WorldStateDomain;
    (requires_reentry_guard(facts)
        && native_result::stock_scoped_prefix_operand(native, facts).is_none())
        || facts.effects.accesses().iter().any(|access| {
            access.domain == WorldStateDomain::LegacyExternal(SideEffectTarget::InterpState)
        })
        || (facts.effects.accesses().iter().any(|access| {
            matches!(
                access.domain,
                WorldStateDomain::InterpreterTopology
                    | WorldStateDomain::InterpreterPolicy
                    | WorldStateDomain::PackageState
                    | WorldStateDomain::HostCapabilities
            ) && access.mode != tcl_registry::world_effect::EffectAccessMode::Read
        }) && !delegates_stock_script_effects(native, facts, state, context))
}

impl SourceCommandBindings {
    pub(in super::super) fn walk_source_target_after_observers(
        &mut self,
        segment: &tcl_source::segment::SegmentedCommand,
        words: &[tcl_source::words::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        select_contents_write_source(state);
        if let Some(outcomes) = Self::prepare_logical_definition_boxed(
            words,
            target,
            segment.span.start(),
            state,
            context,
        ) {
            return *outcomes;
        }
        if self.declaration_preview_depth > 0
            && let Some(outcomes) = logical_operation::hosted_scalar_transfer(
                words,
                target,
                segment.span.start(),
                state,
                *context,
            )
        {
            return outcomes;
        }
        let incoming = boxed_source_branch(state);
        let prepared = match prepare_source_native_invocation(
            target,
            effective,
            state,
            context.registry,
            context.realm(),
        ) {
            Ok(prepared) => prepared,
            Err(SourceNativePreparationError::Unresolved(route)) => {
                state.mark_opaque_binding_mutation();
                return SourceOutcomes::native_invocation(&incoming, state, route);
            }
            Err(SourceNativePreparationError::InvalidArguments) => {
                return SourceOutcomes::invocation(
                    state,
                    tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::completion::CompletionCode::Error,
                    ),
                );
            }
        };
        let invocation = prepared.invocation_boxed(&target.command);
        let facts = prepared.facts.as_ref();
        let selection = context
            .selected_compilation
            .unwrap_or(&tcl_registry::native_compilation::NativeCompilationSelection::Unknown);
        let operands = SourceScriptOperands {
            compilation_spec: facts.native_compilation.as_ref(),
            compilation_selection: selection,
            words,
            written_arguments: context.written_arguments,
            target,
            arguments: invocation.arguments_ref(),
        };
        let original_variable_operands = prepared.original_variable_operands.get_or_init(|| {
            original_name_value::original_variable_invocation(
                operands,
                segment.span.start(),
                state,
                *context,
            )
        });
        self.retain_original_variable_operands(
            state.current_source_origin.as_ref(),
            segment.span.start(),
            original_variable_operands,
        );
        let native = SourceNativeInvocation {
            compilation_spec: facts.native_compilation.as_ref(),
            compilation_selection: selection,
            segment,
            words,
            written_arguments: context.written_arguments,
            target,
            invocation: &invocation,
            original_variable_operands,
        };
        self.walk_prepared_source_target(native, &prepared, effective, state, context, &incoming)
    }

    /// Keep logical definition result construction outside recursive source frames.
    #[inline(never)]
    fn prepare_logical_definition_boxed(
        words: &[tcl_source::words::WordExpr],
        target: &SourceCommandTarget,
        offset: u32,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        logical_definition::transfer(words, target, offset, state, *context).map(Box::new)
    }

    #[cfg(debug_assertions)]
    fn trace_prepared_source_receiver(
        native: &SourceNativeInvocation<'_>,
        receipts: &CapturedNativeResultReceipts,
        read_complete: bool,
        write_complete: bool,
        route: &tcl_registry::completion_route::InvocationCompletionRoute,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_HANDLER_CLOSED_RECEIVER site={} read_captured={} read_current={} write_current={} route={route:?}",
                native.segment.span.start(),
                receipts.normal_read.is_some(),
                read_complete,
                write_complete
            );
        }
    }

    #[cfg(debug_assertions)]
    fn trace_prepared_source_return(
        native: &SourceNativeInvocation<'_>,
        outcomes: &SourceOutcomes,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_HANDLER_RETURN site={} normal={} abrupt={} normal_dynamic={}",
                native.segment.span.start(),
                outcomes.normal.is_some(),
                outcomes.abrupt.len(),
                outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| normal.source_variables.dynamic_bindings)
            );
        }
    }

    fn walk_prepared_source_target(
        &mut self,
        native: SourceNativeInvocation<'_>,
        prepared: &PreparedSourceNativeInvocation<'_>,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        incoming: &ModuleCommandBindings,
    ) -> SourceOutcomes {
        // Everything before and after the recursive handler walk runs in
        // non-inlined leaf helpers, so this frame holds only what crosses the
        // walk: one route and one boxed receipt bundle per nesting level.
        let (route, mut walk) =
            match self.prepare_source_target_walk(native, prepared, effective, state, context) {
                Ok(prepared_walk) => prepared_walk,
                Err(outcomes) => return *outcomes,
            };
        let facts = prepared.facts.as_ref();
        let mut outcomes =
            self.walk_prepared_native_target(native, state, context, facts, route, incoming);
        self.finish_source_target_walk(
            &mut outcomes,
            native,
            facts,
            state,
            context,
            incoming,
            &mut walk,
        );
        *outcomes
    }

    /// The source target's receipts and completion route, captured before its
    /// recursive handler walk; an early answer when no walk is needed.
    #[inline(never)]
    fn prepare_source_target_walk(
        &mut self,
        native: SourceNativeInvocation<'_>,
        prepared: &PreparedSourceNativeInvocation<'_>,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Result<
        (
            tcl_registry::completion_route::InvocationCompletionRoute,
            Box<PreparedSourceTargetWalk>,
        ),
        Box<SourceOutcomes>,
    > {
        let facts = prepared.facts.as_ref();
        let completions = CapturedNativeCompletionReceipts::capture(native, facts, state, context);
        if state.stock_tclsh_stdout
            && !is_implicit_stock_stdout_write(native, facts)
            && !completions.captured_alias_creation()
            && may_change_stock_stdout(native, facts, state, *context)
        {
            state.stock_tclsh_stdout = false;
        }
        if let Some(outcomes) = Self::prepare_jim_stock_class_boxed(native, facts, state, context) {
            return Err(outcomes);
        }
        if let Some(mut outcomes) =
            Self::prepare_non_body_source_result(native, facts, state, context)
        {
            if (requires_reentry_guard(facts)
                || (outcomes.normal_completion.is_none()
                    && !command_world_closed_on_normal_return(native.target, facts)
                    && !source_command_world::namespace_body_world_closed_on_normal_return(
                        native, facts, &outcomes, *context,
                    )))
                && let Some(normal) = outcomes.normal.as_mut()
            {
                normal.withhold_command_world(false);
            }
            return Err(outcomes);
        }
        let mut receipts = capture_source_native_results_boxed(native, facts, state, context);
        let (route, object_protocol) =
            self.prepare_native_handler(prepared, native, effective, state, context)?;
        if let Some(append) = receipts.string_append.as_mut() {
            append.after_preparation(state, native, facts, *context, object_protocol);
        }
        let read_complete = receipts
            .normal_read
            .as_ref()
            .is_some_and(|proof| proof.is_current(&state.source_variables, context.registry));
        let write_complete = receipts
            .normal_write
            .as_ref()
            .is_some_and(|proof| proof.is_current(&state.source_variables, context.registry));
        let route = if read_complete || write_complete || completions.captured() {
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Ok,
            )
        } else {
            route
        };
        #[cfg(debug_assertions)]
        Self::trace_prepared_source_receiver(
            &native,
            &receipts,
            read_complete,
            write_complete,
            &route,
        );
        Ok((
            route,
            Box::new(PreparedSourceTargetWalk {
                completions,
                receipts,
                read_complete,
                write_complete,
            }),
        ))
    }

    /// Settle the source target's receipts against its walked outcomes and
    /// publish them.
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn finish_source_target_walk(
        &mut self,
        outcomes: &mut SourceOutcomes,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        incoming: &ModuleCommandBindings,
        walk: &mut PreparedSourceTargetWalk,
    ) {
        #[cfg(debug_assertions)]
        Self::trace_prepared_source_return(&native, outcomes);
        // Implementation contract: naming.variable.original-set-read-completion
        // docs/design/analysis/name-resolution-proofs/original-set-read-completion.md
        if walk.read_complete
            || walk.write_complete
            || walk
                .completions
                .completed(outcomes, native, facts, *context)
        {
            outcomes.retain_closed_native_handler_completion(native, facts, *context);
        }
        finish_source_native_results(
            outcomes,
            native,
            facts,
            incoming,
            context,
            &mut walk.receipts,
        );
        let write_complete = walk.write_complete;
        self.finish_native_source_completion(
            outcomes,
            native,
            facts,
            incoming,
            context,
            walk.receipts
                .normal_write
                .as_ref()
                .filter(|_| write_complete),
        );
        let reentry = requires_reentry_guard(facts);
        if !native_result::scoped_prefix_preserves_world(
            native, facts, incoming, outcomes, *context,
        ) && (reentry
            || (outcomes.normal_completion.is_none()
                && !command_world_closed_on_normal_return(native.target, facts)
                && !source_command_world::namespace_body_world_closed_on_normal_return(
                    native, facts, outcomes, *context,
                )
                && !original_delegated_script::world_closed_on_normal_return(
                    native, facts, incoming, outcomes, *context,
                )))
        {
            let modelled = (!reentry
                && (walk
                    .completions
                    .completed_for_diagnostics(outcomes, native, facts, *context)
                    || outcomes.normal.as_ref().is_some_and(|normal| {
                        CapturedNativeCompletionReceipts::defined_one_class_for_diagnostics(
                            facts, incoming, normal,
                        ) || CapturedNativeCompletionReceipts::deleted_one_alias_for_diagnostics(
                            facts, incoming, normal,
                        )
                    })))
                || outcomes.normal.as_ref().is_some_and(|normal| {
                    CapturedNativeCompletionReceipts::created_one_alias_for_diagnostics(
                        facts, incoming, normal,
                    )
                });
            if let Some(normal) = outcomes.normal.as_mut() {
                normal.withhold_command_world(modelled);
            }
        }
        outcomes.publish(state);
    }

    /// Keep the stock-class result construction out of recursive source frames.
    #[inline(never)]
    fn prepare_jim_stock_class_boxed(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        Self::walk_jim_stock_class(native, facts, state, *context).map(Box::new)
    }

    #[inline(never)]
    fn prepare_non_body_source_result(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        // An attested definition recipe owns its installation prerequisites;
        // scripted providers do not borrow a stock normal-handler envelope.
        if let Some(outcomes) = Self::walk_snit_definition(native, state, *context) {
            return Some(Box::new(outcomes));
        }
        source_representation::opaque_result_producer(native, facts, state, *context).map(Box::new)
    }

    fn finish_native_source_completion(
        &mut self,
        outcomes: &mut SourceOutcomes,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        incoming: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        write: Option<&crate::var_resolve::OriginalNormalValueWrite>,
    ) {
        // A selected total arithmetic expression owns this certificate.
        // Other native body handlers have independent condition, iteration,
        // restoration and completion obligations, even with a normal child.
        let handler_complete = outcomes
            .native_normal_completion
            .as_ref()
            .is_some_and(|proof| {
                outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| proof.matches(native, facts, normal, *context))
            });
        let conditional_complete =
            outcomes
                .conditional_normal_completion
                .as_ref()
                .is_some_and(|proof| {
                    outcomes
                        .normal
                        .as_ref()
                        .is_some_and(|normal| proof.matches(native, facts, normal, *context))
                });
        if handler_complete && let Some(write) = write {
            self.retain_quiet_normal_variables(
                native,
                facts,
                incoming,
                outcomes,
                write,
                context.registry,
            );
        }
        if facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Expr,
            )
            && !handler_complete
            && !conditional_complete
        {
            outcomes.normal_completion = None;
        }
        outcomes.native_normal_completion = None;
        outcomes.conditional_normal_completion = None;
        // The default value-only Return has no code/level conversion. Its
        // exact pending successful route becomes Normal only at the actual
        // procedure boundary; options and configured returns remain separate.
        if facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Return,
            )
            && native
                .invocation
                .arguments()
                .exact_argv_len()
                .is_some_and(|count| count <= 1)
            && outcomes.normal.is_none()
            && outcomes.abrupt.len() == 1
            && matches!(
                outcomes.abrupt[0].0,
                tcl_registry::completion_route::InvocationCompletionRoute::Return(
                    tcl_registry::completion_route::ReturnCompletionRoute {
                        eventual_code: tcl_registry::completion::CompletionCode::Ok,
                        remaining_level: 1,
                    }
                )
            )
        {
            outcomes.complete_procedure_return = Some(outcomes.abrupt[0].0);
        }
    }
}
