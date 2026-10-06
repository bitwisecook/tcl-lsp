// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Execute retained constructors before publishing manufactured object results.

use super::{
    Arc, DeferredSourceBody, ModuleCommandBindings, SourceCommandBindings, SourceCommandTarget,
    SourceConstructorEntry, SourceExecutionContext, SourceObjectInstanceProof, SourceOutcomes,
};
use crate::registry_invocation::EffectiveInvocationWord;
use tcl_registry::{
    CompletionCode,
    completion_route::InvocationCompletionRoute as Route,
    definer::{ManufacturerMethod, MemberVisibility, NativeConstructorBoundary},
};

struct SelectedConstructor {
    body: Option<DeferredSourceBody>,
    destructor: Option<DeferredSourceBody>,
    manufacturer: ManufacturerMethod,
    boundary: NativeConstructorBoundary,
    name: Option<String>,
}

#[derive(Clone, Copy)]
struct ConstructorAllocation<'a> {
    selected: &'a SelectedConstructor,
    receiver: &'a Arc<SourceObjectInstanceProof>,
}

#[derive(Clone, Copy)]
struct ConstructorBodyCall<'a> {
    body: &'a DeferredSourceBody,
    receiver: &'a Arc<SourceObjectInstanceProof>,
}

impl SourceCommandBindings {
    pub(super) fn walk_original_constructor(
        &mut self,
        site: u32,
        effective: &[EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        let selected = self.select_original_constructor(effective, state, target, context)?;
        if !state.install_manufacture_slot(
            target,
            &selected.manufacturer,
            &effective[1..],
            context.namespace,
        ) {
            return Some(SourceOutcomes::invocation(
                state,
                Route::Tcl(CompletionCode::Error),
            ));
        }
        let Some(mut receiver) = state.manufacture_object_proof(site, target, context) else {
            return Some(super::opaque_source_invocation(state));
        };
        state.retain_named_manufacture(&mut receiver, &effective[1..], context);
        let from = usize::from(selected.manufacturer.constructor_args_from);
        let mut arguments = Vec::with_capacity(effective.len() - from);
        arguments.push(effective[0].clone());
        arguments.extend_from_slice(&effective[from + 1..]);
        let result = match selected.body.as_ref() {
            Some(body) => self.walk_constructor_body(
                site,
                &arguments,
                state,
                ConstructorBodyCall {
                    body,
                    receiver: &receiver,
                },
                constructor_arguments(context, from),
            ),
            None => SourceOutcomes::normal(state),
        };
        Some(self.finish_original_constructor(
            site,
            state,
            ConstructorAllocation {
                selected: &selected,
                receiver: &receiver,
            },
            result,
            context,
        ))
    }

    fn finish_original_constructor(
        &mut self,
        site: u32,
        state: &mut ModuleCommandBindings,
        allocation: ConstructorAllocation<'_>,
        mut result: SourceOutcomes,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let ConstructorAllocation { selected, receiver } = allocation;
        // The body result is discarded by the allocation boundary on success.
        result.normal_value = None;
        result.normal_representation = None;
        result.normal_object = None;
        result.normal_method_prefix = None;
        result.normal_rhs_read = None;
        if result.normal.as_ref().is_some_and(|normal| {
            selected.name.is_some()
                && normal.receiver_allocation_is_current(receiver)
                && normal.constructed_command_name(receiver).is_none()
        }) {
            let mut deleted = result
                .normal
                .take()
                .expect("selected named allocation branch");
            // Source command deletion did not retain an independently ordered
            // destructor callback receipt; its effects remain unresolved.
            deleted.mark_opaque_binding_mutation();
            result.add(Route::Tcl(CompletionCode::Error), &deleted);
        }
        if let Some(normal) = &result.normal
            && normal.receiver_allocation_is_current(receiver)
        {
            result.normal_object = Some(Arc::clone(receiver));
            result.normal_value = normal.constructed_command_name(receiver).map(|name| {
                Arc::new(super::native_result::EvaluatedSourceValue {
                    text: name,
                    representation: tcl_syntax::value::ValueRepresentation::Unknown,
                    numeric: None,
                })
            });
        }
        let failed = std::mem::take(&mut result.abrupt);
        let values = std::mem::take(&mut result.abrupt_values);
        for (route, mut branch) in failed {
            let completed = selected.boundary.completion(route);
            let value = values
                .iter()
                .find(|(known, _)| *known == route)
                .map(|(_, value)| Arc::clone(value));
            if completed == Route::ProcessExit {
                result.add(completed, &branch);
                continue;
            }
            let cleanup =
                self.walk_constructor_cleanup(site, &mut branch, selected, receiver, context);
            if let Some(normal) = cleanup.normal {
                result.join_abrupt_result(completed, &normal, value.as_ref(), None);
            }
            for (cleanup_route, state) in cleanup.abrupt {
                let route = if cleanup_route == Route::ProcessExit {
                    cleanup_route
                } else {
                    completed
                };
                result.join_abrupt_result(route, &state, value.as_ref(), None);
            }
        }
        result
            .abrupt_values
            .retain(|(route, _)| result.abrupt.iter().any(|(present, _)| present == route));
        result.abrupt_objects.clear();
        result.publish(state);
        result
    }

    fn select_original_constructor(
        &self,
        effective: &[EffectiveInvocationWord],
        state: &ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> Option<Box<SelectedConstructor>> {
        if target.kind != super::BindingKind::Class
            || !target.prepended.is_empty()
            || state.source_step_observed()
            || state.source_execution_observed(target.identity.as_ref())
            || effective.iter().any(|word| {
                matches!(
                    word,
                    EffectiveInvocationWord::Expanded | EffectiveInvocationWord::Opaque
                )
            })
        {
            return None;
        }
        let definition = state.class_definitions.get(target.identity.as_ref()?)?;
        if definition.dispatcher.is_some()
            || !definition.lifecycle_entries_closed
            || !definition.inherited_classes.is_empty()
            || definition.implementation_generation != target.implementation_generation
            || !state.class_definition_dependencies_hold(definition)
        {
            return None;
        }
        let dialect = state.baseline.dialect?;
        let grammar = context.registry.native_default_construction_grammar(
            &definition.factory,
            dialect,
            context.realm,
        )?;
        let boundary = grammar.native_constructor_boundary(dialect)?;
        if !state.default_construction_dependencies_hold(grammar) {
            return None;
        }
        let manufacturer =
            *grammar.manufacturer(effective.get(1)?.as_registry_word().literal()?)?;
        if manufacturer.visibility != MemberVisibility::Exported
            || effective.len().checked_sub(1)? < usize::from(manufacturer.constructor_args_from)
        {
            return None;
        }
        let name = match manufacturer.names_instance_at {
            Some(index) => {
                let original = effective
                    .get(usize::from(index) + 1)?
                    .as_registry_word()
                    .literal()?;
                if original.is_empty() || !state.source_variables.namespace_known {
                    return None;
                }
                Some(tcl_syntax::naming::qualify(context.namespace, original))
            }
            None => None,
        };
        let body = match definition.constructor_entry.as_deref() {
            Some(entry) => Some(self.original_lifecycle_body(entry)?),
            None => None,
        };
        let destructor = match definition.destructor_entry.as_deref() {
            Some(entry) => Some(self.original_lifecycle_body(entry)?),
            None => None,
        };
        Some(Box::new(SelectedConstructor {
            body,
            destructor,
            manufacturer,
            boundary,
            name,
        }))
    }

    fn original_lifecycle_body(
        &self,
        entry: &SourceConstructorEntry,
    ) -> Option<DeferredSourceBody> {
        self.deferred
            .values()
            .find(|body| {
                body.receiver_method
                    && body
                        .executed_script
                        .as_ref()
                        .is_some_and(|(site, _, source)| {
                            site == entry.declaration() && source == entry.body()
                        })
                    && body
                        .parameters
                        .iter()
                        .map(|parameter| (&parameter.name, &parameter.default))
                        .eq(entry
                            .formals()
                            .iter()
                            .map(|(name, default)| (name, default)))
            })
            .cloned()
    }

    fn walk_constructor_body(
        &mut self,
        site: u32,
        arguments: &[EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        call: ConstructorBodyCall<'_>,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let ConstructorBodyCall { body, receiver } = call;
        let implementation = body.implementation_id();
        if !self.active_calls.insert(implementation.clone()) {
            return super::opaque_source_invocation(state);
        }
        self.called_implementations.insert(implementation.clone());
        let target = receiver.class_target();
        let activation = super::source_called_body_activation_name(
            state.current_source_origin.as_ref(),
            target,
            body,
            site,
        );
        Arc::make_mut(&mut state.object_instances)
            .receivers
            .insert(activation.clone(), Arc::clone(receiver));
        let mut outcomes = self.walk_called_body(site, arguments, state, target, body, context);
        self.active_calls.remove(&implementation);
        if let Some(normal) = &mut outcomes.normal {
            Arc::make_mut(&mut normal.object_instances)
                .receivers
                .remove(&activation);
        }
        for (_, branch) in &mut outcomes.abrupt {
            Arc::make_mut(&mut branch.object_instances)
                .receivers
                .remove(&activation);
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_constructor_cleanup(
        &mut self,
        site: u32,
        state: &mut ModuleCommandBindings,
        selected: &SelectedConstructor,
        receiver: &Arc<SourceObjectInstanceProof>,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let current = state.receiver_allocation_is_current(receiver);
        state.remove_constructed_command(receiver);
        let mut cleanup = if let Some(body) = &selected.destructor {
            if current {
                self.walk_constructor_body(
                    site,
                    &[EffectiveInvocationWord::Literal(
                        receiver.class_target().command.clone(),
                    )],
                    state,
                    ConstructorBodyCall { body, receiver },
                    empty_constructor_arguments(context),
                )
            } else {
                super::opaque_source_invocation(state)
            }
        } else {
            SourceOutcomes::normal(state)
        };
        if let Some(normal) = &mut cleanup.normal {
            normal.retire_constructed_allocation(receiver, context.registry);
        }
        for (_, branch) in &mut cleanup.abrupt {
            branch.retire_constructed_allocation(receiver, context.registry);
        }
        cleanup.publish(state);
        cleanup
    }
}

fn constructor_arguments(
    context: SourceExecutionContext<'_>,
    from: usize,
) -> SourceExecutionContext<'_> {
    SourceExecutionContext {
        written_arguments: context
            .written_arguments
            .and_then(|values| values.get(from..)),
        written_values: context.written_values.and_then(|values| values.get(from..)),
        written_representations: context
            .written_representations
            .and_then(|values| values.get(from..)),
        written_objects: context
            .written_objects
            .and_then(|values| values.get(from..)),
        written_method_prefixes: context
            .written_method_prefixes
            .and_then(|values| values.get(from..)),
        written_variable_reads: context
            .written_variable_reads
            .and_then(|values| values.get(from..)),
        ..context
    }
}

fn empty_constructor_arguments(context: SourceExecutionContext<'_>) -> SourceExecutionContext<'_> {
    SourceExecutionContext {
        written_arguments: None,
        written_values: None,
        written_representations: None,
        written_objects: None,
        written_method_prefixes: None,
        written_variable_reads: None,
        ..context
    }
}

#[cfg(test)]
mod tests {
    use super::super::SourceAnalysisOptions;
    use super::*;

    fn analyse(source: &str, profile: &str) -> SourceCommandBindings {
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

    #[test]
    fn original_constructor_normal_completion_publishes_allocated_identity() {
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for body in ["", "return IGNORED", "set ::side CHANGED; return IGNORED"] {
                let source = format!(
                    "set ::side BEFORE; oo::class create C {{constructor args {{{body}}}; method ping {{}} {{}}}}; set object [C create c1 a b]; $object ping"
                );
                let bindings = analyse(&source, profile);
                let offset = u32::try_from(source.find("$object ping").unwrap()).unwrap();
                let access = bindings
                    .variable_accesses
                    .get(&offset)
                    .and_then(|reads| reads.first())
                    .expect(&source);
                let proof = access.proved_object_instance().expect(&source);
                assert_eq!(proof.class_target().command, "::C");
                assert_eq!(
                    proof.allocation().site.offset,
                    u32::try_from(source.find("C create c1").unwrap()).unwrap()
                );
                let dispatch = bindings.invocation_at_source("$object", offset);
                assert!(
                    dispatch.named_object_instance_at_dispatch().is_some(),
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn original_constructor_failure_runs_destructor_and_retires_named_object() {
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for body in ["set ::side CHANGED; error BOOM", "return -code 7 CUSTOM"] {
                let source = format!(
                    "set ::side BEFORE; oo::class create C {{constructor args {{{body}}}; destructor {{set ::side DESTRUCTOR}}}}; catch {{C create c1}}; puts $::side; c1 absent"
                );
                let bindings = analyse(&source, profile);
                let read = u32::try_from(source.find("puts $::side").unwrap()).unwrap();
                let observed = bindings.invocation_at_source("puts", read);
                assert_eq!(
                    observed
                        .evaluated_argument_values
                        .first()
                        .and_then(Option::as_deref),
                    Some("DESTRUCTOR"),
                    "{profile}: {source}"
                );
                let final_call = u32::try_from(source.find("c1 absent").unwrap()).unwrap();
                let missing = bindings.invocation_at_source("c1", final_call);
                assert!(missing.proved_target().is_none(), "{profile}: {source}");
                assert_eq!(
                    missing.selected_slot_presence(),
                    super::super::SourceCommandSlotPresence::Absent,
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn original_constructor_native_formals_control_entry_and_cleanup() {
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (arguments, expected) in [("CHANGED", "CHANGED"), ("", "DESTRUCTOR")] {
                let source = format!(
                    "set ::side BEFORE; oo::class create C {{constructor {{x}} {{set ::side $x}}; destructor {{set ::side DESTRUCTOR}}}}; catch {{C create c1 {arguments}}}; puts $::side"
                );
                let bindings = analyse(&source, profile);
                let read = u32::try_from(source.find("puts $::side").unwrap()).unwrap();
                let observed = bindings.invocation_at_source("puts", read);
                assert_eq!(
                    observed
                        .evaluated_argument_values
                        .first()
                        .and_then(Option::as_deref),
                    Some(expected),
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn empty_constructor_removes_previous_entry_without_parsing_formals() {
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for declarations in [
                "constructor {{a b c}} {}",
                "constructor args {error PREVIOUS}; constructor {{a b c}} {}",
            ] {
                let source = format!(
                    "oo::class create C {{{declarations}; method ping {{}} {{}}}}; set object [C create c1 one two]; $object ping"
                );
                let bindings = analyse(&source, profile);
                let offset = u32::try_from(source.find("$object ping").unwrap()).unwrap();
                assert!(
                    bindings
                        .variable_accesses
                        .get(&offset)
                        .into_iter()
                        .flatten()
                        .any(|read| read.proved_object_instance().is_some()),
                    "{profile}: {source}"
                );
                assert!(
                    bindings
                        .invocation_at_source("$object", offset)
                        .named_object_instance_at_dispatch()
                        .is_some(),
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn empty_destructor_removes_previous_cleanup_body() {
        let source = "set ::side BEFORE; oo::class create C {constructor args {error BOOM}; destructor {set ::side DESTRUCTOR}; destructor {}}; catch {C create c1}; puts $::side";
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let bindings = analyse(source, profile);
            let offset = u32::try_from(source.find("puts $::side").unwrap()).unwrap();
            assert_eq!(
                bindings
                    .invocation_at_source("puts", offset)
                    .evaluated_argument_values
                    .first()
                    .and_then(Option::as_deref),
                Some("BEFORE"),
                "{profile}"
            );
        }
    }

    #[test]
    fn constructor_entry_advice_never_donates_unknown_or_mutated_completion() {
        for source in [
            "set object [C create c1]; $object ping",
            "oo::class create C {constructor args {unknown_command}; method ping {} {}}; set object [C create c1]; $object ping",
            "oo::class create C {constructor args {}; method ping {} {}}; rename ::oo::class old; set object [C create c1]; $object ping",
            "oo::class create C {constructor args {my destroy}; method ping {} {}}; set object [C create c1]; $object ping",
        ] {
            let bindings = analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find("$object ping").unwrap()).unwrap();
            assert!(
                bindings
                    .variable_accesses
                    .get(&offset)
                    .into_iter()
                    .flatten()
                    .all(|read| read.proved_object_instance().is_none()),
                "{source}"
            );
        }
    }
}
