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

//! Entering a called procedure body or a lambda: frame, namespace, formals and parameter binding.

use super::super::source_origin::{source_activation_name, source_called_body_activation_name};
use super::super::{
    DeferredSourceBody, MayBinding, ModuleCommandBindings, SourceCommandBindings,
    SourceCommandTarget, SourceExecutionContext, SourceNativeInvocation, SourceOutcomes,
    native_result, object_instance, original_receiver_body_context, receiver_self,
    source_representation,
};
use super::body_flow::evaluated_script_realm;
use super::execution_context::SourceBodyEntryPurpose;
use super::outcomes::opaque_source_invocation;
use crate::command_binding::ExecutedScriptWords;
use std::collections::BTreeSet;
use std::sync::Arc;
use tcl_names::command::{BindingKind, CommandAllocationSite};
use tcl_names::namespace::SourceNamespaceKey;
use tcl_names::origin::ExecutedScriptSource;
use tcl_registry::CommandRegistry;

/// Keep formal value representation and object identity from the same call operands.
fn retain_called_body_value_formals(
    state: &mut ModuleCommandBindings,
    called: &mut crate::var_resolve::ResolveContext,
    body: &DeferredSourceBody,
    actual_count: usize,
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) {
    source_representation::retain_value_formals(
        called,
        &body.parameters,
        actual_count,
        1,
        target,
        context,
    );
    object_instance::retain_value_formals(
        state,
        called,
        &body.parameters,
        actual_count,
        1,
        target,
        context,
    );
}

enum PreparedSourceLambda {
    Ready {
        lambda: tcl_registry::lambda_invocation::LambdaInvocation,
        parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    },
    Invalid,
    Unknown,
}

/// Selection and formal parsing precede entry into a lambda frame.
fn prepare_source_lambda(
    selection: tcl_registry::lambda_invocation::LambdaInvocationSelection,
    dialect: Option<tcl_native_model::InvocationDialect>,
) -> PreparedSourceLambda {
    use tcl_registry::lambda_invocation::LambdaInvocationSelection;
    let lambda = match selection {
        LambdaInvocationSelection::Selected(lambda) => lambda,
        LambdaInvocationSelection::Invalid => return PreparedSourceLambda::Invalid,
        LambdaInvocationSelection::Unknown => return PreparedSourceLambda::Unknown,
    };
    let Some(grammar) = dialect.and_then(tcl_native_model::InvocationDialect::parameter_grammar)
    else {
        return PreparedSourceLambda::Unknown;
    };
    let Ok(parameters) =
        tcl_syntax::formal_params::parse_formal_parameters_in(&lambda.parameters, grammar)
    else {
        return PreparedSourceLambda::Invalid;
    };
    PreparedSourceLambda::Ready { lambda, parameters }
}

/// Carry both value-formal receipts through the same original lambda argv.
/// Preparation ends before recursive body descent.
#[inline(never)]
fn retain_lambda_value_formals(
    state: &mut ModuleCommandBindings,
    variables: &mut crate::var_resolve::ResolveContext,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
    actual_count: usize,
    lambda: &tcl_registry::lambda_invocation::LambdaInvocation,
    native: SourceNativeInvocation<'_>,
    context: SourceExecutionContext<'_>,
) {
    let first_actual_word = lambda.call_arguments_from + 1;
    source_representation::retain_value_formals(
        variables,
        parameters,
        actual_count,
        first_actual_word,
        native.target,
        context,
    );
    object_instance::retain_value_formals(
        state,
        variables,
        parameters,
        actual_count,
        first_actual_word,
        native.target,
        context,
    );
}

fn retained_lambda_body(
    lambda: &tcl_registry::lambda_invocation::LambdaInvocation,
    native: SourceNativeInvocation<'_>,
    state: &ModuleCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<ExecutedScriptSource> {
    let parent = CommandAllocationSite {
        source: Arc::clone(state.current_source_origin.as_ref()?),
        offset: native.segment.span.start(),
    };
    let dialect = native.invocation.arguments().dialect()?;
    let value = native
        .invocation
        .arguments()
        .literal_at(lambda.lambda_argument)?;
    let list = lambda
        .lambda_argument
        .checked_sub(native.target.prepended.len())
        .and_then(|written| native.words.get(written + 1))
        .map_or_else(
            || {
                ExecutedScriptSource::materialised(
                    parent.clone(),
                    vec![lambda.lambda_argument],
                    value,
                )
            },
            |word| {
                ExecutedScriptSource::from_word(
                    parent.clone(),
                    lambda.lambda_argument,
                    word,
                    value,
                    config,
                )
            },
        );
    let body = list.list_element(parent, lambda.lambda_argument, 1, dialect.word_values)?;
    (body.text.bytes() == lambda.body.as_bytes()).then_some(body)
}

fn entered_receiver_body_context(
    body: &DeferredSourceBody,
    target: &SourceCommandTarget,
    receiver: Option<receiver_self::CalledBodyReceiver>,
    frame: &tcl_names::variable::VariableExecutionFrame,
    called: &crate::var_resolve::ResolveContext,
    config: tcl_lexer::LexerConfig,
) -> Option<Arc<original_receiver_body_context::OriginalReceiverBodyContext>> {
    receiver
        .and_then(|receiver| {
            original_receiver_body_context::OriginalReceiverBodyContext::at_entered_call(
                body, target, receiver, frame, called, config,
            )
        })
        .map(Arc::new)
}

fn source_lambda_frame(
    state: &ModuleCommandBindings,
    namespace: &str,
    namespace_key: &SourceNamespaceKey,
    site: u32,
) -> tcl_names::variable::VariableExecutionFrame {
    tcl_names::variable::VariableExecutionFrame::Procedure {
        namespace: namespace.to_owned(),
        identity: source_activation_name(state.current_source_origin.as_ref(), "lambda", site),
    }
    .with_namespace_identity(namespace_key.clone())
}

fn called_body_frame(
    body: &DeferredSourceBody,
    identity: String,
    namespace: &str,
    namespace_key: &SourceNamespaceKey,
) -> tcl_names::variable::VariableExecutionFrame {
    let frame = if body.receiver_method {
        tcl_names::variable::VariableExecutionFrame::ReceiverMethod { identity }
    } else {
        tcl_names::variable::VariableExecutionFrame::Procedure {
            namespace: namespace.to_owned(),
            identity,
        }
    };
    frame.with_namespace_identity(namespace_key.clone())
}

pub(in super::super) fn called_body_namespace(
    state: &ModuleCommandBindings,
    target: &SourceCommandTarget,
    body: &DeferredSourceBody,
) -> Option<SourceNamespaceKey> {
    Some(if body.receiver_method {
        body.namespace_key.clone()
    } else {
        let holders = state
            .bindings
            .iter()
            .filter(|&(_, bindings)| {
                bindings.iter().any(|binding| matches!(binding,
                    MayBinding::Target(installed) if installed.terminal
                        && installed.kind == BindingKind::Proc
                        && installed.implementation_generation == target.implementation_generation
                        && installed.implementation_allocation == target.implementation_allocation
                        && installed.token == target.identity
                ))
            })
            .map(|(slot, _)| slot.holder().into_owned())
            .collect::<BTreeSet<_>>();
        let mut holders = holders.into_iter();
        let holder = holders.next()?;
        if holders.next().is_some() {
            return None;
        }
        holder
    })
}

impl SourceCommandBindings {
    pub(in super::super) fn walk_called_body(
        &mut self,
        site: u32,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        selected: receiver_self::CalledBodyTarget<'_>,
        body: &DeferredSourceBody,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let target = selected.target;
        if effective.iter().skip(1).any(|word| {
            matches!(
                word,
                crate::registry_invocation::EffectiveInvocationWord::Expanded
                    | crate::registry_invocation::EffectiveInvocationWord::Opaque
            )
        }) {
            self.record_unrepresented_called_body(site, state, target, body);
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let Some(namespace_key) = called_body_namespace(state, target, body) else {
            self.record_unrepresented_called_body(site, state, target, body);
            return opaque_source_invocation(state);
        };
        let holder = namespace_key.display().unwrap_or_default();
        let identity = source_called_body_activation_name(
            state.current_source_origin.as_ref(),
            target,
            body,
            site,
        );
        let frame = called_body_frame(body, identity, &holder, &namespace_key);
        let parent = Arc::clone(&state.source_variables);
        let previous_frame = state.variable_frame.clone();
        let mut called = parent.enter_called_frame(&frame);
        if let Some(statics) = &body.statics {
            called.install_callable_statics(statics, context.registry);
        }
        let actual = source_call_arguments(target, effective);
        match bind_called_body_parameters(&mut called, body, &actual, target, context) {
            Some(true) => {}
            Some(false) => {
                return SourceOutcomes::invocation(
                    state,
                    Route::Tcl(tcl_registry::completion::CompletionCode::Error),
                );
            }
            None => {
                self.record_unrepresented_called_body(site, state, target, body);
                return opaque_source_invocation(state);
            }
        }
        state.bind_called_receiver_variables(
            &mut called,
            body,
            target,
            selected.receiver,
            context.registry,
        );
        retain_called_body_value_formals(state, &mut called, body, actual.len(), target, context);
        let receiver_context = entered_receiver_body_context(
            body,
            target,
            selected.receiver,
            &frame,
            &called,
            context.config,
        );
        self.record_called_body_source(
            body,
            context.body_entry(),
            &frame,
            Some(&namespace_key),
            receiver_context,
        );
        state.source_variables = Arc::new(called);
        state.variable_frame = frame.clone();
        let previous_origin = state.current_source_origin.clone();
        state.current_source_origin.clone_from(&body.source_origin);
        let mut outcomes = self
            .walk_source(
                &body.source,
                body.offset,
                state,
                &SourceExecutionContext {
                    control: (context)
                        .control
                        .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript)
                        .with_compilation(state.baseline.body_compilation()),
                    compilation_snapshot: None,
                    selected_compilation: None,
                    original_variable_compilation: None,
                    namespace: &holder,
                    namespace_key: Some(&namespace_key),
                    depth: context.depth + 1,
                    frame: &frame,
                    ..context
                },
            )
            .through_procedure_boundary(state.baseline.dialect);
        outcomes.restore_frame(&parent, &previous_frame);
        outcomes.restore_source_origin(previous_origin.as_ref());
        outcomes.publish(state);
        outcomes
    }

    fn record_called_body_source(
        &mut self,
        body: &DeferredSourceBody,
        entry: SourceBodyEntryPurpose,
        frame: &tcl_names::variable::VariableExecutionFrame,
        namespace: Option<&SourceNamespaceKey>,
        receiver_context: Option<Arc<original_receiver_body_context::OriginalReceiverBodyContext>>,
    ) {
        if let Some((parent, argument, script)) = &body.executed_script {
            self.record_executed_script(
                parent.clone(),
                *argument,
                script.clone(),
                (entry == SourceBodyEntryPurpose::Selected).then_some(frame),
                namespace,
            );
            self.record_original_receiver_body_context(
                parent,
                *argument,
                script,
                frame,
                (entry == SourceBodyEntryPurpose::Selected)
                    .then_some(receiver_context)
                    .flatten(),
            );
            if let Some((origin, span)) = &body.executed_word {
                self.record_executed_word(origin, *span, None, script.clone());
            }
        }
    }

    pub(super) fn walk_lambda_invocation(
        &mut self,
        selection: tcl_registry::lambda_invocation::LambdaInvocationSelection,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion::CompletionCode as Code;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let (lambda, parameters) = match prepare_source_lambda(selection, state.baseline.dialect) {
            PreparedSourceLambda::Ready { lambda, parameters } => (lambda, parameters),
            PreparedSourceLambda::Invalid => {
                return SourceOutcomes::invocation(state, Route::Tcl(Code::Error));
            }
            PreparedSourceLambda::Unknown => return opaque_source_invocation(state),
        };
        let arguments = native.invocation.arguments();
        let actual = (lambda.call_arguments_from..arguments.exact_argv_len().unwrap())
            .map(|index| arguments.literal_at(index))
            .collect::<Vec<_>>();
        let Some(namespace_key) = native
            .original_variable_operands
            .input(lambda.lambda_argument, &state.source_variables)
            .and_then(|input| state.namespace_for_lambda_operand(input, context.config))
        else {
            return opaque_source_invocation(state);
        };
        let namespace = namespace_key.display().unwrap_or_default();
        let frame = source_lambda_frame(
            state,
            &namespace,
            &namespace_key,
            native.segment.span.start(),
        );
        let parent_variables = Arc::clone(&state.source_variables);
        let parent_frame = state.variable_frame.clone();
        let mut variables = parent_variables.enter_called_frame(&frame);
        match bind_source_parameters(
            &mut variables,
            &parameters,
            &actual,
            lambda.body.is_empty(),
            context.registry,
            None,
        ) {
            Some(true) => {}
            Some(false) => return SourceOutcomes::invocation(state, Route::Tcl(Code::Error)),
            None => return opaque_source_invocation(state),
        }
        retain_lambda_value_formals(
            state,
            &mut variables,
            &parameters,
            actual.len(),
            &lambda,
            native,
            context,
        );
        let Some(script) = retained_lambda_body(&lambda, native, state, context.config) else {
            return opaque_source_invocation(state);
        };
        self.record_lambda_body(
            &lambda,
            native,
            state,
            &script,
            (context.body_entry() == SourceBodyEntryPurpose::Selected).then_some(&frame),
            Some(&namespace_key),
        );
        let realm = evaluated_script_realm(
            lambda.lambda_argument..lambda.lambda_argument + 1,
            native.script_operands(),
            context,
        );
        let offset = script.base();
        let parent_origin = state.current_source_origin.replace(script.origin);
        state.source_variables = Arc::new(variables);
        state.variable_frame = frame.clone();
        let mut outcomes = self
            .walk_source(
                &lambda.body,
                offset,
                state,
                &SourceExecutionContext {
                    control: (context)
                        .control
                        .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript)
                        .with_realm(realm)
                        .with_compilation(state.baseline.body_compilation()),
                    compilation_snapshot: None,
                    selected_compilation: None,
                    original_variable_compilation: None,
                    namespace: &namespace,
                    namespace_key: Some(&namespace_key),
                    frame: &frame,
                    depth: context.depth + 1,
                    ..context
                },
            )
            .through_procedure_boundary(state.baseline.dialect);
        outcomes.restore_frame(&parent_variables, &parent_frame);
        outcomes.restore_source_origin(parent_origin.as_ref());
        outcomes.publish(state);
        outcomes
    }

    fn record_lambda_body(
        &mut self,
        lambda: &tcl_registry::lambda_invocation::LambdaInvocation,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        script: &ExecutedScriptSource,
        frame: Option<&tcl_names::variable::VariableExecutionFrame>,
        namespace: Option<&SourceNamespaceKey>,
    ) {
        let parent = CommandAllocationSite {
            source: Arc::clone(state.current_source_origin.as_ref().unwrap()),
            offset: native.segment.span.start(),
        };
        if let Some(word) = lambda
            .lambda_argument
            .checked_sub(native.target.prepended.len())
            .and_then(|written| native.words.get(written + 1))
        {
            self.record_executed_word(&parent.source, word.source().span, Some(1), script.clone());
        }
        self.record_executed_script(
            parent,
            lambda.lambda_argument,
            script.clone(),
            frame,
            namespace,
        );
    }
}

fn bind_called_body_parameters(
    called: &mut crate::var_resolve::ResolveContext,
    body: &DeferredSourceBody,
    actual: &[Option<&str>],
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) -> Option<bool> {
    if let Some(topology) = &body.original_parameters {
        if called
            .invocation_dialect?
            .parameter_grammar()?
            .skips_empty_body_activation()
            && body.source.is_empty()
        {
            return Some(topology.bindings(actual.len()).is_ok());
        }
        let original_actual = original_positional_arguments(target, context, actual.len());
        return topology.bind(called, actual, original_actual, context.registry);
    }
    bind_source_parameters(
        called,
        &body.parameters,
        actual,
        body.source.is_empty(),
        context.registry,
        native_result::numeric_call_arguments(context, target),
    )
}

fn original_positional_arguments<'a>(
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'a>,
    count: usize,
) -> Option<&'a [Option<Arc<super::super::original_name_value::OriginalProducedNameValue>>]> {
    use crate::registry_invocation::EffectiveInvocationWord as Word;
    let words = context.written_arguments?.get(1..)?;
    if !target.prepended.is_empty()
        || words.len() != count
        || words.iter().any(|word| {
            matches!(
                word,
                Word::Expanded | Word::KnownExpansion(_) | Word::KnownByteExpansion(_)
            )
        })
    {
        return None;
    }
    context
        .written_name_values?
        .get(1..)
        .filter(|values| values.len() == count)
}

fn bind_source_parameters(
    context: &mut crate::var_resolve::ResolveContext,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
    actual: &[Option<&str>],
    body_is_empty: bool,
    registry: &CommandRegistry,
    numeric_values: Option<&[Option<Arc<native_result::EvaluatedSourceValue>>]>,
) -> Option<bool> {
    use tcl_syntax::formal_params::FormalArgumentBinding as Binding;
    let grammar = context.invocation_dialect?.parameter_grammar()?;
    let Ok(bindings) =
        tcl_syntax::formal_params::bind_formal_arguments(parameters, actual.len(), grammar)
    else {
        return Some(false);
    };
    if grammar.skips_empty_body_activation() && body_is_empty {
        return Some(true);
    }
    for binding in bindings {
        match binding {
            Binding::Value {
                parameter,
                argument,
            } => {
                let name = &parameters[parameter].name;
                if let Some(value) = actual[argument] {
                    context.bind_literal_incoming(name, value, registry);
                    if let Some(receipt) = numeric_values
                        .and_then(|values| values.get(argument))
                        .and_then(Option::as_deref)
                        .filter(|receipt| receipt.text == value)
                        .and_then(|receipt| receipt.numeric.as_ref())
                    {
                        context.retain_incoming_native_numeric(name, receipt, registry);
                    }
                } else {
                    context.bind_unknown_incoming(name, registry);
                }
            }
            Binding::Default { parameter } => {
                context.bind_literal_incoming(
                    &parameters[parameter].name,
                    parameters[parameter].default.as_deref()?,
                    registry,
                );
            }
            Binding::Rest {
                name, start, len, ..
            } => {
                if let Some(list) = actual
                    .get(start..start + len)
                    .and_then(|values| values.iter().copied().collect::<Option<Vec<_>>>())
                    .and_then(|values| context.render_rest_list(&values))
                {
                    context.bind_literal_incoming(&name, &list, registry);
                } else {
                    context.bind_unknown_incoming(&name, registry);
                }
            }
            Binding::CallerLink { name, argument, .. } => {
                match crate::variable_bindings::bind_caller_reference(
                    context,
                    &name,
                    actual[argument],
                    registry,
                ) {
                    crate::variable_bindings::CallerReferenceBinding::Bound => {}
                    crate::variable_bindings::CallerReferenceBinding::Missing => {
                        return Some(false);
                    }
                    crate::variable_bindings::CallerReferenceBinding::Unknown => return None,
                }
            }
        }
    }
    Some(true)
}

/// Build called-frame operands before descending into the original body.
#[inline(never)]
fn source_call_arguments<'a>(
    target: &'a SourceCommandTarget,
    effective: &'a [crate::registry_invocation::EffectiveInvocationWord],
) -> Vec<Option<&'a str>> {
    let mut actual = target
        .prepended
        .iter()
        .map(|value| value.as_registry_word().literal())
        .collect::<Vec<_>>();
    actual.extend(
        effective
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word().literal()),
    );
    actual
}
