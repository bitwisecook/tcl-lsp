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

//! A native target: its handler contract, body admission and the frame of the body it selects.

use super::super::binding_projection::source_binding_from_original_input;
use super::super::namespace_context::SourceNamespaceKey;
use super::super::source_origin::source_activation_name;
use super::super::{
    ModuleCommandBindings, PreparedSourceNativeBody, SourceCommandBindings, SourceExecutionContext,
    SourceInvocationBinding, SourceLifecycleInvocation, SourceNativeFrameSelection,
    SourceNativeInvocation, SourceOutcomes, conditional_source, container_coercion,
    jim_empty_iteration, object_callbacks, original_command_table, original_delegated_script,
    original_name_value, original_namespace_body, trusted_package_transfer, variable_outputs,
};
use super::execution_context::{source_body_context_boxed, source_namespace_context_boxed};
use super::native_transfer::transfer_native_completion_outputs;
use super::outcomes::{boxed_source_completion_receipt, opaque_source_invocation};
use super::target_walk::PreparedSourceNativeInvocation;
use std::sync::Arc;
use tcl_registry::{InvocationFacts, StateTransition};

fn procedure_body_admission_refusal(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    realm: tcl_dialect::model::InvocationRealm,
    state: &mut ModuleCommandBindings,
) -> Option<SourceOutcomes> {
    match source_procedure_body_admission(native, facts, realm) {
        Some(true) => None,
        Some(false) => Some(SourceOutcomes::invocation(
            state,
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Error,
            ),
        )),
        None => Some(opaque_source_invocation(state)),
    }
}

fn source_procedure_body_admission(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<bool> {
    use tcl_registry::native_procedure::{
        NativeProcedureDefinitionSelection as Definition, procedure_definition_body_policy,
    };
    let Some(Definition::Valid(definition)) = facts.procedure_definition else {
        return Some(true);
    };
    let dialect = native.invocation.arguments().dialect()?;
    let policy = procedure_definition_body_policy(dialect, realm)?;
    if policy == tcl_registry::native_procedure::ProcedureDefinitionBodyPolicy::EvaluatedValue {
        return Some(true);
    }
    let word = native.script_operands().written_word(definition.body_at)?;
    policy.accepts(crate::registry_invocation::native_compilation_word_shape(
        word,
    ))
}

pub(super) fn is_implicit_stock_stdout_write(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
) -> bool {
    native.target.registry_backed
        && native.target.kind == super::super::BindingKind::Builtin
        && native.target.implementation_generation == 0
        && native.target.prepended.is_empty()
        && facts.operation
            == tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::ChannelWrite)
        && facts.arg_roles_complete
        && facts.argument_offset == 0
        && native.invocation.arguments().exact_argv_len() == Some(1)
}

/// The stock expression, conditional and capture handlers delegate re-entry
/// to their walked operands.
fn delegates_to_stock_operand_walk(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    context: SourceExecutionContext<'_>,
) -> bool {
    use tcl_registry::script_body_flow::{ScriptBodyFlow, script_body_flow};
    use tcl_registry::{SemanticOperationId, hooks::LoweringHookId};
    if !native.target.registry_backed
        || native.target.kind != super::super::BindingKind::Builtin
        || native.target.implementation_generation != 0
        || !native.target.prepended.is_empty()
    {
        return false;
    }
    match facts.operation {
        SemanticOperationId::StructuredLowering(LoweringHookId::Expr) => matches!(
            script_body_flow(facts),
            ScriptBodyFlow::Expressions(_) | ScriptBodyFlow::ConcatenatedExpression { .. }
        ),
        SemanticOperationId::StructuredLowering(LoweringHookId::If) => {
            matches!(script_body_flow(facts), ScriptBodyFlow::Conditional(_))
        }
        SemanticOperationId::StructuredLowering(LoweringHookId::Catch) => matches!(
            tcl_registry::case_bodies::script_body_flow_in_registry(
                context.registry,
                facts,
                native.invocation.arguments(),
            ),
            ScriptBodyFlow::Capture(
                tcl_registry::catch_invocation::CatchInvocationSelection::Valid(_)
            )
        ),
        _ => false,
    }
}

/// The selected body walker accounts for callbacks in these script operands.
pub(super) fn delegates_stock_script_effects(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    delegates_to_stock_operand_walk(native, facts, context)
        || original_namespace_body::has_original_namespace_body_delegation(
            native, facts, state, context,
        )
        || original_delegated_script::has_delegated_literal_script(native, facts, state, context)
}

/// How a native whose body the walk did not select may re-enter Tcl before
/// it completes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum NativeReentry {
    None,
    /// Host I/O can run a channel's transform or reflected-channel handler.
    /// Those handlers are installed by visible commands whose own walk
    /// accounts for them, so command lookups keep the document's command
    /// world, as they do at a procedure's entry; only proofs that no unseen
    /// code ran across this point withdraw.
    ChannelHandler,
    /// Callbacks or scripts this walk did not follow: the command world is
    /// unknown afterwards.
    Unwalked,
}

fn unmodelled_native_reentry(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    incoming: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> NativeReentry {
    use tcl_registry::resolved_invocation::SourceCallbackRetention;
    use tcl_registry::side_effects::SideEffectTarget;
    use tcl_registry::world_effect::WorldStateDomain;
    if is_implicit_stock_stdout_write(native, facts) {
        return if state.stock_tclsh_stdout {
            NativeReentry::None
        } else {
            NativeReentry::Unwalked
        };
    }
    if state.stock_tclsh_stdout && delegates_stock_script_effects(native, facts, incoming, context)
    {
        // This fallback did not walk the script's potential callbacks.
        return NativeReentry::Unwalked;
    }
    if native
        .invocation
        .arguments()
        .dialect()
        .and_then(|dialect| dialect.tcl_version)
        .is_none_or(|version| version < tcl_dialect::TclVersion::V9_0)
    {
        return NativeReentry::None;
    }
    if facts
        .state_transitions
        .declared()
        .is_none_or(|transitions| transitions.facts().is_empty())
        && (facts
            .traits
            .contains(tcl_registry::Traits::CREATES_DYNAMIC_BARRIER)
            || facts.source_callback_retention() != SourceCallbackRetention::Absent)
    {
        return NativeReentry::Unwalked;
    }
    let host_io = facts.effects.accesses().iter().any(|access| {
        matches!(
            access.domain,
            WorldStateDomain::LegacyExternal(SideEffectTarget::FileIo | SideEffectTarget::Process)
        )
    });
    if host_io {
        NativeReentry::ChannelHandler
    } else {
        NativeReentry::None
    }
}

#[inline(never)]
fn native_implementation_dependency_holds(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    effective: &[crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    use tcl_registry::native_compilation::NormalHandlerImplementationLookup;
    if matches!(
        context.selected_compilation,
        Some(tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. })
    ) {
        return true;
    }
    let binding = original_name_value::original_command_head_input(
        native.words,
        native.segment.span.start(),
        state,
        context,
    )
    .and_then(|input| {
        source_binding_from_original_input(state, &input, &context.namespace_identity())
    });
    let normal = facts.normal_handler_implementation_lookup(state.baseline.dialect);
    if match normal {
        NormalHandlerImplementationLookup::NoneRequired => false,
        NormalHandlerImplementationLookup::Required(lookup) => !binding
            .as_ref()
            .is_some_and(|binding| binding.proves_native_implementation_lookup(&lookup)),
        NormalHandlerImplementationLookup::RequiredPath(paths) => {
            let arguments = binding
                .as_ref()
                .and_then(SourceInvocationBinding::proved_handler_target)
                .into_iter()
                .flat_map(|target| target.prepended.iter())
                .chain(effective.iter().skip(1))
                .map(|word| word.as_registry_word())
                .collect::<Vec<_>>();
            let mut invocation = tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(""),
                &arguments,
            );
            if let Some(dialect) = state.baseline.dialect {
                invocation = invocation.with_dialect(dialect);
            }
            !binding.as_ref().is_some_and(|binding| {
                binding.proves_native_handler_path(
                    paths,
                    invocation.arguments(),
                    facts.argument_offset,
                )
            })
        }
        NormalHandlerImplementationLookup::Unknown => true,
    } {
        return false;
    }
    facts
        .native_compilation
        .and_then(|spec| {
            state
                .baseline
                .dialect
                .and_then(|dialect| spec.implementation_lookup(dialect))
        })
        .is_none_or(|lookup| {
            binding
                .as_ref()
                .is_some_and(|binding| binding.proves_native_implementation_lookup(&lookup))
        })
}

fn apply_selected_native_frame_effect(
    selection: &mut SourceNativeFrameSelection,
    invocation: &tcl_registry::InvocationWords<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
) -> Result<(), tcl_registry::completion_route::InvocationCompletionRoute> {
    if let Some(effect) = facts.frame_effect.filter(|effect| {
        effect.layout == tcl_registry::frame_effect::FrameArgLayout::ScriptInSelectedFrame
    }) {
        match effect.successful_layout(invocation.arguments()).layout {
            tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. }
                if level.is_current_frame() => {}
            tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. }
                if level.is_global_frame() =>
            {
                selection.frame = crate::var_resolve::VariableExecutionFrame::Global;
                selection.variables = state
                    .source_variables
                    .selected_frame_context(level)
                    .map(Arc::new);
                selection.namespace_key = state
                    .native_root_namespace_key()
                    .unwrap_or_else(|| SourceNamespaceKey::authored("::"));
                "::".clone_into(&mut selection.namespace);
            }
            tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. } => {
                if let Some(selected) = state.source_variables.selected_frame_context(level) {
                    selection.namespace.clone_from(&selected.namespace);
                    selection.namespace_key = selected
                        .namespace_identity
                        .clone()
                        .unwrap_or_else(|| SourceNamespaceKey::authored(&selected.namespace));
                    selection.variables = Some(Arc::new(selected));
                }
                selection.frame = crate::var_resolve::VariableExecutionFrame::Selected {
                    selector: level,
                    namespace: selection
                        .variables
                        .as_ref()
                        .map(|selected| selected.namespace.clone()),
                };
            }
            tcl_registry::frame_effect::FrameArgumentResolution::Invalid => {
                return Err(
                    tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::completion::CompletionCode::Error,
                    ),
                );
            }
            tcl_registry::frame_effect::FrameArgumentResolution::Unknown => {
                selection.frame = crate::var_resolve::VariableExecutionFrame::Unknown;
            }
        }
    }
    Ok(())
}

#[inline(never)]
fn select_native_frame(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
) -> Result<SourceNativeFrameSelection, tcl_registry::completion_route::InvocationCompletionRoute> {
    let SourceNativeInvocation {
        segment,
        invocation,
        ..
    } = native;
    let namespace = context.namespace;
    let mut namespace_key = context.namespace_identity();
    let ensures_namespace = facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::NamespaceEval,
        );
    if ensures_namespace {
        let Some(target) = facts.state_transitions.declared().and_then(|transitions| {
            let mut targets =
                transitions
                    .facts()
                    .iter()
                    .filter_map(|fact| match &fact.transition {
                        StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
                            namespace,
                        }) => Some(namespace),
                        _ => None,
                    });
            let selected = targets.next()?;
            targets.all(|target| target == selected).then_some(selected)
        }) else {
            state.mark_opaque_resolution();
            return Err(route);
        };
        let selected = match target {
            tcl_registry::NamespaceTransitionTarget::Named(subject)
                if matches!(
                    state
                        .baseline
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                        .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
                    Some(tcl_syntax::naming::NativeNameProtocol::C(_))
                ) && (subject.literal().is_none()
                    || !matches!(namespace_key, SourceNamespaceKey::Authored(_))) =>
            {
                original_command_table::original_operand(native, subject, state, context).and_then(
                    |input| state.original_namespace_key_for_input(&namespace_key, &input),
                )
            }
            _ => state.ensure_namespace_key_at(target, &namespace_key, segment.span.start()),
        };
        let Some(selected) = selected else {
            state.mark_opaque_resolution();
            return Err(route);
        };
        namespace_key = selected;
    } else if facts.existing_namespace_script.is_some() {
        let Some(selected) =
            original_delegated_script::existing_namespace(native, facts, state, context)
        else {
            state.mark_opaque_binding_mutation();
            return Err(tcl_registry::completion_route::InvocationCompletionRoute::Unknown);
        };
        namespace_key = selected;
    }
    let body_namespace = namespace_key
        .display()
        .unwrap_or_else(|| namespace.to_owned());
    let namespace_activation = ensures_namespace || facts.existing_namespace_script.is_some();
    let frame = if namespace_activation {
        crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
            namespace: body_namespace.clone(),
            identity: source_activation_name(
                state.current_source_origin.as_ref(),
                "namespace-eval",
                segment.span.start(),
            ),
        }
    } else {
        context.frame.clone()
    };
    let mut selection = SourceNativeFrameSelection {
        namespace: body_namespace,
        namespace_key,
        frame,
        variables: None,
        namespace_activation,
    };
    apply_selected_native_frame_effect(&mut selection, invocation, facts, state)?;
    if matches!(
        selection.frame.layout(),
        crate::var_resolve::VariableExecutionFrame::Unknown
            | crate::var_resolve::VariableExecutionFrame::Selected {
                namespace: None,
                ..
            }
    ) {
        return Err(tcl_registry::completion_route::InvocationCompletionRoute::Unknown);
    }
    selection.frame = selection
        .frame
        .with_namespace_identity(selection.namespace_key.clone());
    Ok(selection)
}

#[inline(never)]
fn install_selected_native_frame(
    state: &mut ModuleCommandBindings,
    frame: &crate::var_resolve::VariableExecutionFrame,
    namespace_key: &SourceNamespaceKey,
    namespace_activation: bool,
    selected_variables: Option<Arc<crate::var_resolve::ResolveContext>>,
) {
    if namespace_activation {
        let mut variables = state.source_variables.enter_called_frame(frame);
        // This exact selected Ensure/frame owns the source namespace identity.
        // Authored frame presentation deliberately supplies no native token;
        // retain the independent source key without decoding that presentation.
        variables.namespace_identity = Some(namespace_key.clone());
        state.source_variables = Arc::new(variables);
        state.variable_frame = frame.clone();
    } else if let Some(selected) = selected_variables {
        state.source_variables = selected;
        state.variable_frame = frame.clone();
    }
}

/// Namespace receipts a selected native body carries across its walk.
struct SelectedNativeBodyReceipts {
    namespace_body: Option<Box<original_namespace_body::OriginalNamespaceBody>>,
    delegated_script: Option<Box<original_delegated_script::OriginalDelegatedScript>>,
    previous_namespace_exit: Option<conditional_source::SourceConditionalNamespaceExit>,
    pub(in super::super) conditional_namespace:
        Option<conditional_source::SourceConditionalNamespaceExit>,
}

/// Construct frame and body descriptors before retaining recursive descent.
#[inline(never)]
fn prepare_selected_native_body(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
    facts: &InvocationFacts,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
) -> Result<Box<PreparedSourceNativeBody>, tcl_registry::completion_route::InvocationCompletionRoute>
{
    let context = *context;
    let flow = tcl_registry::case_bodies::script_body_flow_in_registry(
        context.registry,
        facts,
        native.invocation.arguments(),
    );
    if matches!(flow, tcl_registry::script_body_flow::ScriptBodyFlow::None) {
        return Err(route);
    }
    let parent_variables = Arc::clone(&state.source_variables);
    let parent_frame = state.variable_frame.clone();
    let mut selection = select_native_frame(native, state, context, facts, route)?;
    install_selected_native_frame(
        state,
        &selection.frame,
        &selection.namespace_key,
        selection.namespace_activation,
        selection.variables.take(),
    );
    Ok(Box::new(PreparedSourceNativeBody {
        selection,
        parent_variables,
        parent_frame,
        flow,
    }))
}

impl SourceCommandBindings {
    /// Retain only the selected handler contract through recursive execution.
    #[inline(never)]
    pub(super) fn prepare_native_handler(
        &mut self,
        prepared: &PreparedSourceNativeInvocation<'_>,
        native: SourceNativeInvocation<'_>,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Result<
        (
            tcl_registry::completion_route::InvocationCompletionRoute,
            object_callbacks::ObjectInputProtocol,
        ),
        Box<SourceOutcomes>,
    > {
        let context = *context;
        let facts = prepared.facts.as_ref();
        if state.command_observer_registration_invalid(facts, &context.namespace_identity()) {
            return Err(Box::new(SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
            )));
        }
        let route =
            container_coercion::ordinary_list_length_completion(native, facts, state, context)
                .unwrap_or_else(|| prepared.completion_route(state, context));
        let implementation =
            native_implementation_dependency_holds(native, facts, effective, state, context);
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_HANDLER_ENTRY site={} implementation={} route={route:?} dynamic={}",
                native.segment.span.start(),
                implementation,
                state.source_variables.dynamic_bindings
            );
        }
        if !implementation {
            return Err(Box::new(opaque_source_invocation(state)));
        }
        self.record_native_invocation_reads(
            facts,
            native.invocation.arguments(),
            state,
            context,
            native.original_variable_operands,
        );
        if let Some(outcomes) = jim_empty_iteration::transfer(native, state, context) {
            return Err(Box::new(outcomes));
        }
        let object_protocol = self.prepare_object_callbacks(native, facts, state, context);
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_HANDLER_OBJECTS site={} preserves={} dynamic={}",
                native.segment.span.start(),
                object_protocol.preserves_representation(),
                state.source_variables.dynamic_bindings
            );
        }
        if !object_protocol.preserves_representation() {
            object_callbacks::coerce_closed_list_representations(
                native,
                facts,
                state,
                context,
                object_protocol,
            );
        }
        self.record_rhs_read_store(native, facts, state, context);
        let writes = crate::variable_bindings::source_variable_write_places_with_original_operands(
            facts,
            native.invocation.arguments(),
            &state.source_variables,
            context.registry,
            native.original_variable_operands,
        );
        if !writes.is_empty() {
            Arc::make_mut(&mut state.object_instances).invalidate_writes(&writes);
        }
        Ok((route, object_protocol))
    }

    fn walk_native_body_protocol(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
    ) -> Option<SourceOutcomes> {
        if let Some(outcomes) = Self::walk_array_entries_store(native, facts, state, context) {
            return Some(outcomes);
        }
        match facts.body_execution {
            Some(tcl_registry::body_execution::BodyExecutionSpec::ArrayIteration) => {
                Some(self.walk_array_iteration(native, facts, state, context))
            }
            Some(tcl_registry::body_execution::BodyExecutionSpec::SourceFile) => {
                Some(self.walk_selected_source_file(native, state, context))
            }
            Some(tcl_registry::body_execution::BodyExecutionSpec::SubstitutionTemplate) => {
                Some(self.walk_native_substitution_template(native, state, context))
            }
            _ => None,
        }
    }

    pub(super) fn walk_prepared_native_target(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        incoming: &ModuleCommandBindings,
    ) -> Box<SourceOutcomes> {
        let route = match self.prepare_native_target(native, state, context, facts, route, incoming)
        {
            Ok(route) => route,
            Err(outcomes) => return outcomes,
        };
        if let Some(tcl_registry::body_execution::BodyExecutionSpec::CapturedLifecycle(spec)) =
            facts.body_execution.as_ref()
        {
            return Box::new(self.walk_prepared_native_lifecycle(spec, native, state, *context));
        }
        let mut outcomes = Box::new(
            self.walk_selected_native_body(native, state, context, facts, route, incoming),
        );
        transfer_native_completion_outputs(&mut outcomes, native, facts, route, *context);
        outcomes.publish(state);
        outcomes
    }

    /// Runtime body entry cannot prepare a child of a generic invocation.
    /// Native compiler-selected bodies retain their own traversal and checks.
    fn defers_runtime_body(
        &self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
    ) -> bool {
        use tcl_registry::{
            body_execution::BodyExecutionSpec,
            native_compilation::NativeCompilationSelection,
            script_body_flow::{ScriptBodyFlow, script_body_flow},
        };
        self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource
            && !matches!(
                native.compilation_selection,
                NativeCompilationSelection::Inline { .. }
            )
            && (matches!(
                facts.body_execution,
                Some(
                    BodyExecutionSpec::SourceFile
                        | BodyExecutionSpec::ArrayIteration
                        | BodyExecutionSpec::SubstitutionTemplate
                )
            ) || !matches!(
                script_body_flow(facts),
                ScriptBodyFlow::None
                    | ScriptBodyFlow::Deferred
                    | ScriptBodyFlow::Expressions(_)
                    | ScriptBodyFlow::ConcatenatedExpression { .. }
            ))
    }

    /// Finish cold handler/definition checks before retaining a recursive body.
    #[inline(never)]
    fn prepare_native_target(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        incoming: &ModuleCommandBindings,
    ) -> Result<tcl_registry::completion_route::InvocationCompletionRoute, Box<SourceOutcomes>>
    {
        let context = *context;
        if let Some(outcomes) = trusted_package_transfer::OriginalTrustedPackageTransfer::capture(
            native, facts, state, context,
        )
        .map(|transfer| transfer.finish(native, facts, state, context))
        {
            return Err(Box::new(outcomes));
        }
        if self.defers_runtime_body(native, facts) {
            return Err(Box::new(opaque_source_invocation(state)));
        }
        if let Some(outcomes) = self.walk_native_body_protocol(native, state, context, facts) {
            return Err(Box::new(outcomes));
        }
        if let Some(outcomes) = variable_outputs::walk_known_outputs(native, facts, state, context)
        {
            return Err(Box::new(outcomes));
        }
        if let Some(refusal) =
            procedure_body_admission_refusal(native, facts, context.realm(), state)
        {
            return Err(Box::new(refusal));
        }
        if let Some(outcomes) = Self::walk_native_constant(native, facts, state, context) {
            return Err(Box::new(outcomes));
        }
        if let Some(outcomes) =
            self.walk_observed_native_store(native, facts, state, context, route, incoming)
        {
            self.retain_captured_normal_variables(
                native,
                facts,
                incoming,
                &outcomes,
                context.registry,
            );
            return Err(Box::new(outcomes));
        }
        let route =
            self.prepare_native_definition(native, state, context, facts, route, incoming)?;
        Ok(route)
    }

    /// Only lifecycle entry retains its dialect-bearing argument construction.
    #[inline(never)]
    fn walk_prepared_native_lifecycle(
        &mut self,
        spec: &tcl_registry::body_execution::CapturedLifecycleSpec,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        self.walk_lifecycle(
            SourceLifecycleInvocation {
                spec,
                arguments: native.invocation.arguments(),
                site: native.segment.span.start(),
                words: native.words,
                written_arguments: context.written_arguments,
                target: native.target,
            },
            state,
            context,
        )
    }

    fn walk_selected_native_body(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        incoming: &ModuleCommandBindings,
    ) -> SourceOutcomes {
        let prepared = match prepare_selected_native_body(native, state, context, facts, route) {
            Ok(prepared) => prepared,
            Err(route) => {
                // Channel handlers, notifier callbacks, and selected callback
                // operands can re-enter Tcl before any returned completion.
                match unmodelled_native_reentry(native, facts, state, incoming, *context) {
                    NativeReentry::Unwalked => {
                        state.mark_opaque_binding_mutation();
                        return SourceOutcomes::invocation(state, route);
                    }
                    NativeReentry::ChannelHandler => state.mark_unmodelled_reentry(),
                    NativeReentry::None => {}
                }
                return SourceOutcomes::native_invocation(incoming, state, route);
            }
        };
        let body_context = source_body_context_boxed(
            context,
            &prepared.selection.namespace,
            &prepared.selection.frame,
        );
        let body_context =
            source_namespace_context_boxed(&body_context, &prepared.selection.namespace_key);
        // Receipt capture and settlement run in non-inlined leaf helpers, so
        // this recursive frame keeps only the boxed state crossing the walk.
        let receipts = self.enter_selected_native_body(native, facts, state, context, &prepared);
        let mut outcomes = self.walk_body_flow(
            &prepared.flow,
            native,
            state,
            &body_context,
            facts.existing_namespace_script.is_some(),
        );
        self.leave_selected_native_body(
            &mut outcomes,
            native,
            facts,
            state,
            context,
            &prepared,
            receipts,
        );
        outcomes
    }

    /// Capture the namespace-body receipts and install the conditional
    /// namespace exit before a selected native body is walked.
    #[inline(never)]
    fn enter_selected_native_body(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        prepared: &PreparedSourceNativeBody,
    ) -> Box<SelectedNativeBodyReceipts> {
        let namespace_body = boxed_source_completion_receipt(|| {
            original_namespace_body::OriginalNamespaceBody::capture(
                native, facts, state, *context, prepared,
            )
        });
        let delegated_script = boxed_source_completion_receipt(|| {
            original_delegated_script::OriginalDelegatedScript::capture(
                native, facts, state, *context, prepared,
            )
        });
        let previous_namespace_exit = self.conditional_source.pending_namespace.take();
        self.conditional_source.pending_namespace =
            original_namespace_body::conditional_source_namespace_exit(
                native, facts, state, *context, prepared,
            );
        let conditional_namespace = self.conditional_source.pending_namespace.clone();
        Box::new(SelectedNativeBodyReceipts {
            namespace_body,
            delegated_script,
            previous_namespace_exit,
            conditional_namespace,
        })
    }

    /// Restore the caller's namespace exit and frame, settle the receipts
    /// against the walked body, and publish.
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn leave_selected_native_body(
        &mut self,
        outcomes: &mut SourceOutcomes,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        prepared: &PreparedSourceNativeBody,
        receipts: Box<SelectedNativeBodyReceipts>,
    ) {
        let SelectedNativeBodyReceipts {
            namespace_body,
            delegated_script,
            previous_namespace_exit,
            conditional_namespace,
        } = *receipts;
        self.conditional_source.pending_namespace = previous_namespace_exit;
        if prepared.selection.namespace_activation
            || prepared.parent_frame != prepared.selection.frame
        {
            outcomes.restore_frame(&prepared.parent_variables, &prepared.parent_frame);
        }
        if namespace_body
            .as_ref()
            .is_some_and(|receipt| receipt.completed(native, outcomes, *context, prepared))
            || delegated_script
                .as_ref()
                .is_some_and(|receipt| receipt.completed(native, outcomes, *context, prepared))
        {
            outcomes.retain_closed_native_handler_completion(native, facts, *context);
        }
        if conditional_namespace.as_ref().is_some_and(|receipt| {
            original_namespace_body::conditional_namespace_completed(
                receipt, native, outcomes, *context, prepared,
            )
        }) {
            outcomes.retain_conditional_handler_completion(native, facts, *context);
        }
        outcomes.publish(state);
    }
}
