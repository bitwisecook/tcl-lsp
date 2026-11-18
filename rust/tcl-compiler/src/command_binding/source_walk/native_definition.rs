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

//! Native definitions: closed procedures, callable statics and the deferred bodies they register.

use super::super::command_identity::CommandAllocationSite;
use super::super::{
    DeferredImplementationId, DeferredSourceBody, ModuleCommandBindings, SourceCommandBindings,
    SourceCommandKey, SourceExecutionContext, SourceNativeInvocation, SourceOutcomes,
    formal_topology, original_class_factory_transfer, original_command_table,
};
use super::body_flow::{evaluated_script_realm, retained_script_operand};
use super::native_transfer::transfer_native_invocation;
use super::outcomes::{boxed_source_branch, opaque_source_invocation};
use std::sync::Arc;
use tcl_registry::{InvocationFacts, StateTransition};

pub(in super::super) fn native_procedure_parameters(
    facts: &InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
) -> Option<Vec<tcl_syntax::formal_params::FormalParameter>> {
    let grammar = arguments
        .dialect()
        .and_then(tcl_registry::InvocationDialect::parameter_grammar)?;
    let source = facts.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::ParamList)
            .then(|| arguments.literal_at(facts.argument_offset + usize::from(*index)))
            .flatten()
    })?;
    tcl_syntax::formal_params::parse_formal_parameters_in(source, grammar).ok()
}

struct SourceNativeDefinition {
    pub(in super::super) statics: Option<crate::raw_binding::CapturedStaticBindings>,
}

pub(in super::super) fn original_procedure_publication_key(
    native: SourceNativeInvocation<'_>,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
) -> Option<SourceCommandKey> {
    original_command_table::OriginalCommandOperands::capture(native, facts, state, context)
        .procedure_key(facts, state, &context.namespace_identity())
}

/// A valid `proc` whose name lies in a namespace a closed world proves
/// absent: tclsh 8.4 to 9.1 raise `can't create procedure "…": unknown
/// namespace` and define nothing.
fn procedure_into_absent_namespace(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    matches!(
        facts.procedure_definition,
        Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
    ) && super::super::original_namespace_body::closed_original_world(state)
        && original_command_table::OriginalCommandOperands::capture(native, facts, state, context)
            .procedure_namespace_missing(facts, state, &context.namespace_identity())
}

pub(in super::super) fn closed_procedure_definition(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some()
        || tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SCALAR_ALPHA").is_some()
    {
        let key = original_procedure_publication_key(native, state, context, facts);
        eprintln!(
            "ORIGINAL_PROCEDURE_CLOSURE offset={} opaque={} definition_valid={} topology={} key={} holder_exists={} installed_count={}",
            native.segment.span.start(),
            state.has_opaque_domain(),
            matches!(
                facts.procedure_definition,
                Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
            ),
            formal_topology::from_invocation(native, facts, state, context).is_some(),
            key.is_some(),
            key.as_ref()
                .is_some_and(|key| state.namespaces.contains(key.holder().as_ref())),
            key.as_ref().map_or(0, |key| state
                .installed_procedure_targets(key, native.segment.span.start())
                .len()),
        );
    }
    if state.has_opaque_domain()
        || !matches!(
            facts.procedure_definition,
            Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
        )
        || formal_topology::from_invocation(native, facts, state, context).is_none()
    {
        return false;
    }
    let Some(key) = original_procedure_publication_key(native, state, context, facts) else {
        return false;
    };
    let holder = key.holder();
    state.namespaces.contains(holder.as_ref())
        && !state
            .installed_procedure_targets(&key, native.segment.span.start())
            .is_empty()
}

/// The definition raised: nothing it would define is published.
fn raised(state: &ModuleCommandBindings) -> Box<SourceOutcomes> {
    Box::new(SourceOutcomes::invocation(
        state,
        tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
            tcl_registry::completion::CompletionCode::Error,
        ),
    ))
}

fn transfer_native_definition(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
) -> Result<SourceNativeDefinition, crate::raw_binding::StaticCaptureResult> {
    use tcl_registry::native_procedure::NativeProcedureDefinitionSelection as Definition;
    if !matches!(facts.procedure_definition, Some(Definition::Valid(definition)) if definition.statics_at.is_some())
    {
        transfer_native_invocation(native, state, context, facts);
        return Ok(SourceNativeDefinition { statics: None });
    }
    let mut candidate = boxed_source_branch(state);
    transfer_native_invocation(native, &mut candidate, context, facts);
    let statics = prepare_source_callable_statics(native, &mut candidate, context, facts)?;
    *state = *candidate;
    Ok(SourceNativeDefinition { statics })
}

fn prepare_source_callable_statics(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
) -> Result<
    Option<crate::raw_binding::CapturedStaticBindings>,
    crate::raw_binding::StaticCaptureResult,
> {
    use crate::raw_binding::{CallableStorageIdentity, StaticCaptureResult as Capture};
    use tcl_registry::native_procedure::NativeProcedureDefinitionSelection as Definition;
    let Some(Definition::Valid(definition)) = facts.procedure_definition else {
        return Ok(None);
    };
    let Some(index) = definition.statics_at else {
        return Ok(None);
    };
    let input = native
        .original_variable_operands
        .input(index, &state.source_variables)
        .ok_or(Capture::Unknown)?;
    tcl_registry::native_procedure::parse_static_variables_bytes(
        input.bytes(),
        state
            .source_variables
            .invocation_dialect
            .ok_or(Capture::Unknown)?,
    )
    .ok_or(Capture::Unknown)?
    .map_err(|_| Capture::Invalid)?;
    let key = original_procedure_publication_key(native, state, context, facts)
        .ok_or(Capture::Unknown)?;
    let targets = state.installed_procedure_targets(&key, native.segment.span.start());
    if targets.len() != 1 {
        return Err(Capture::Unknown);
    }
    let target = targets.first().ok_or(Capture::Unknown)?;
    let allocation = target
        .implementation_allocation
        .as_ref()
        .ok_or(Capture::Unknown)?;
    let identity = CallableStorageIdentity {
        site: allocation.site.clone(),
        incarnation: allocation.incarnation,
        implementation_generation: target.implementation_generation,
    };
    match Arc::make_mut(&mut state.source_variables).capture_original_callable_statics(
        &identity,
        input,
        context.registry,
    ) {
        Capture::Prepared {
            bindings,
            may_error: false,
        } => Ok(Some(bindings)),
        Capture::Prepared {
            may_error: true, ..
        } => Err(Capture::Unknown),
        outcome => Err(outcome),
    }
}

impl SourceCommandBindings {
    /// End definition and registration temporaries before recursive body execution.
    #[inline(never)]
    pub(super) fn prepare_native_definition(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        incoming: &ModuleCommandBindings,
    ) -> Result<tcl_registry::completion_route::InvocationCompletionRoute, Box<SourceOutcomes>>
    {
        if let Some(outcomes) =
            self.walk_closed_own_object_method_configuration(native, facts, state, context)
        {
            return Err(Box::new(outcomes));
        }
        let class_configuration = Self::closed_class_configuration(native, facts, state, context);
        let class_transfer = original_class_factory_transfer::OriginalClassFactoryTransfer::capture(
            native, facts, state, context,
        );
        self.retain_standalone_definition_references(native, facts, state, context);
        if procedure_into_absent_namespace(native, facts, state, context) {
            return Err(raised(state));
        }
        let definition = match transfer_native_definition(native, state, context, facts) {
            Ok(definition) => definition,
            Err(crate::raw_binding::StaticCaptureResult::Invalid) => {
                return Err(raised(state));
            }
            Err(_) => return Err(Box::new(opaque_source_invocation(state))),
        };
        let procedure_complete = closed_procedure_definition(native, facts, state, context);
        let route = if procedure_complete {
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Ok,
            )
        } else {
            route
        };
        if let Some(configuration) = class_configuration {
            self.register_closed_class_configuration(&configuration, state, context);
            let mut outcomes = SourceOutcomes::native_invocation(
                incoming,
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::CompletionCode::Ok,
                ),
            );
            outcomes.retain_closed_native_handler_completion(native, facts, context);
            return Err(Box::new(outcomes));
        }
        let deferred_class = self.register_bounded_methods(native, state, context, facts);
        if state.record_bounded_class(facts, native.words, native.target, context) || deferred_class
        {
            let complete = deferred_class
                && class_transfer.as_ref().is_some_and(|transfer| {
                    transfer.retain_completed_definition(native, facts, state, context)
                });
            let route = if complete {
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::CompletionCode::Ok,
                )
            } else {
                route
            };
            let mut outcomes = SourceOutcomes::native_invocation(incoming, state, route);
            if complete {
                outcomes.retain_closed_native_handler_completion(native, facts, context);
            }
            return Err(Box::new(outcomes));
        }
        if facts
            .traits
            .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
            && !facts
                .state_transitions
                .declared()
                .is_some_and(|transitions| {
                    transitions
                        .facts()
                        .iter()
                        .any(|fact| matches!(fact.transition, StateTransition::Package(_)))
                })
        {
            state.mark_opaque_binding_mutation();
            state.loader_handler_unknown = true;
        }
        if facts.traits.contains(tcl_registry::Traits::DEFERS_BODY) {
            self.register_deferred_native(
                native,
                state,
                context,
                facts,
                definition.statics.as_ref(),
            );
            let mut outcomes = SourceOutcomes::native_invocation(incoming, state, route);
            if procedure_complete {
                outcomes.retain_closed_native_handler_completion(native, facts, context);
            }
            return Err(Box::new(outcomes));
        }
        Ok(route)
    }

    fn register_deferred_native(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        statics: Option<&crate::raw_binding::CapturedStaticBindings>,
    ) {
        if matches!(
            facts.procedure_definition,
            Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
        ) {
            self.register_deferred_procedure(native, state, context, facts, statics);
        } else if facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::When,
            )
        {
            self.register_deferred_event(native, state, context, facts);
        } else {
            self.register_future_body(native, state, context, facts);
        }
    }

    fn register_deferred_procedure(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        statics: Option<&crate::raw_binding::CapturedStaticBindings>,
    ) {
        let SourceNativeInvocation {
            segment,
            invocation,
            ..
        } = native;
        let original_parameters = formal_topology::from_invocation(native, facts, state, context);
        let Some(parameters) = original_parameters
            .as_ref()
            .map(formal_topology::OriginalFormalTopology::advisory_parameters)
            .or_else(|| native_procedure_parameters(facts, invocation.arguments()))
        else {
            return;
        };
        let Some(key) = original_procedure_publication_key(native, state, context, facts) else {
            return;
        };
        let namespace_key = key.holder().into_owned();
        let holder = namespace_key.display().unwrap_or_default();
        for (index, role) in &facts.arg_roles {
            if *role != tcl_registry::ArgRole::Body {
                continue;
            }
            if let Some(script) = retained_script_operand(
                facts.argument_offset + usize::from(*index),
                native.script_operands(),
                state,
                segment.span.start(),
                context.config,
            ) {
                let Ok(source) = script.try_text() else {
                    continue;
                };
                let source = source.to_owned();
                let offset = script.base();
                let source_origin = Arc::clone(&script.origin);
                let entered = state.current_source_origin.as_ref().map(|origin| {
                    (
                        CommandAllocationSite {
                            source: Arc::clone(origin),
                            offset: segment.span.start(),
                        },
                        facts.argument_offset + usize::from(*index),
                        script.clone(),
                    )
                });
                for created in state.installed_procedure_targets(&key, segment.span.start()) {
                    let Some(identity) = created.token.map(|identity| identity.origin) else {
                        continue;
                    };
                    self.deferred.insert(
                        DeferredImplementationId {
                            command: identity.clone(),
                            generation: segment.span.start(),
                            allocation: created.implementation_allocation.clone(),
                        },
                        DeferredSourceBody {
                            realm: evaluated_script_realm(
                                facts.argument_offset + usize::from(*index)
                                    ..facts.argument_offset + usize::from(*index) + 1,
                                native.script_operands(),
                                context,
                            ),
                            identity,
                            implementation_generation: segment.span.start(),
                            source: source.clone(),
                            offset,
                            namespace: holder.clone(),
                            namespace_key: namespace_key.clone(),
                            event: None,
                            receiver_method: false,
                            receiver_namespace: None,
                            future_frame: None,
                            parameters: parameters.clone(),
                            original_parameters: original_parameters.clone(),
                            statics: statics.cloned(),
                            implementation_allocation: created.implementation_allocation,
                            source_origin: Some(Arc::clone(&source_origin)),
                            executed_script: entered.clone(),
                            executed_word: facts
                                .argument_offset
                                .checked_add(usize::from(*index))
                                .and_then(|argument| {
                                    argument.checked_sub(native.target.prepended.len())
                                })
                                .and_then(|written| native.words.get(written + 1))
                                .zip(state.current_source_origin.as_ref())
                                .map(|(word, origin)| (Arc::clone(origin), word.source().span)),
                        },
                    );
                }
            }
        }
    }

    pub(in super::super) fn register_deferred_event(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
    ) {
        let SourceNativeInvocation {
            segment,
            invocation,
            ..
        } = native;
        let event = invocation.arguments().literal_at(0).map(str::to_owned);
        for (index, role) in &facts.arg_roles {
            if *role != tcl_registry::ArgRole::Body {
                continue;
            }
            if let Some(script) = retained_script_operand(
                facts.argument_offset + usize::from(*index),
                native.script_operands(),
                state,
                segment.span.start(),
                context.config,
            ) {
                let Ok(source) = script.try_text() else {
                    continue;
                };
                let source = source.to_owned();
                let offset = script.base();
                let source_origin = Arc::clone(&script.origin);
                let entered = state.current_source_origin.as_ref().map(|origin| {
                    (
                        CommandAllocationSite {
                            source: Arc::clone(origin),
                            offset: segment.span.start(),
                        },
                        facts.argument_offset + usize::from(*index),
                        script.clone(),
                    )
                });
                let identity = format!("event:{}", segment.span.start());
                self.deferred.insert(
                    DeferredImplementationId {
                        command: identity.clone(),
                        generation: segment.span.start(),
                        allocation: None,
                    },
                    DeferredSourceBody {
                        realm: evaluated_script_realm(
                            facts.argument_offset + usize::from(*index)
                                ..facts.argument_offset + usize::from(*index) + 1,
                            native.script_operands(),
                            context,
                        ),
                        identity,
                        implementation_generation: segment.span.start(),
                        source,
                        offset,
                        namespace: "::".to_owned(),
                        namespace_key: state.native_root_namespace_key().unwrap_or_default(),
                        event: Some(event.clone().unwrap_or_default()),
                        receiver_method: false,
                        receiver_namespace: None,
                        future_frame: None,
                        parameters: Vec::new(),
                        original_parameters: None,
                        statics: None,
                        implementation_allocation: None,
                        source_origin: Some(source_origin),
                        executed_script: entered,
                        executed_word: facts
                            .argument_offset
                            .checked_add(usize::from(*index))
                            .and_then(|argument| {
                                argument.checked_sub(native.target.prepended.len())
                            })
                            .and_then(|written| native.words.get(written + 1))
                            .zip(state.current_source_origin.as_ref())
                            .map(|(word, origin)| (Arc::clone(origin), word.source().span)),
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod absent_namespace_tests {
    use super::super::super::{SourceAnalysisOptions, SourceCommandBindings};

    fn analyse(source: &str, dialect: &str) -> SourceCommandBindings {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = registry.profile().unwrap();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        )
    }

    fn after(source: &str) -> u32 {
        u32::try_from(source.rfind("puts hi").unwrap()).unwrap()
    }

    /// tclsh 8.4.20 to 9.1.0: `proc nope::f {} {}` and `proc ::T::D::define
    /// {} {}` (with only `::T`) raise `can't create procedure "…": unknown
    /// namespace`, so nothing after them runs; `proc ::T::ok {} {}` succeeds.
    /// jimsh 0.84 names commands flat: `proc nope::f {} {return ok}` defines
    /// it and `nope::f` returns ok.
    #[test]
    fn a_procedure_into_a_missing_namespace_raises() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in [
                "proc nope::f {} {}\nputs hi\n",
                "namespace eval ::T {}\nproc ::T::D::define {} {}\nputs hi\n",
            ] {
                assert!(
                    !analyse(source, dialect).entered_site(after(source)),
                    "{dialect}: {source:?}"
                );
            }
            let present = "namespace eval ::T {}\nproc ::T::ok {} {}\nputs hi\n";
            let bindings = analyse(present, dialect);
            assert!(bindings.reached_site(after(present)), "{dialect}");
            assert!(
                bindings.original_completed_command_world().is_some(),
                "{dialect}"
            );
        }
        let jim = "proc nope::f {} {return ok}\nputs hi\n";
        assert!(analyse(jim, "jim").reached_site(after(jim)));
    }
}
