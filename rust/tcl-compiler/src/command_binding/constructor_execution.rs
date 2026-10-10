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

struct SelectedLifecycleBody {
    body: DeferredSourceBody,
    provider: SourceCommandTarget,
}

struct SelectedConstructor {
    body: Option<SelectedLifecycleBody>,
    destructor: Option<SelectedLifecycleBody>,
    manufacturer: ManufacturerMethod,
    boundary: NativeConstructorBoundary,
    name: Option<super::named_manufacture::OriginalNamedManufacture>,
    default_completion: Option<DefaultConstructorCompletion>,
}

/// Independently selected absence of constructor/filter/initializer callbacks,
/// with the complete original scalar argv retained before fresh allocation.
// Implementation contract: naming.tcloo.original-named-manufacture-cell-transfer
// docs/design/analysis/name-resolution-proofs/tcloo-original-named-manufacture-cell-transfer.md
struct DefaultConstructorCompletion {
    site: super::CommandAllocationSite,
    class: SourceCommandTarget,
    inputs: Vec<crate::signature_scan::scope::SignatureSourceNameInput>,
}

impl DefaultConstructorCompletion {
    fn for_absent_body(
        body: Option<&SelectedLifecycleBody>,
        site: u32,
        class: &SourceCommandTarget,
        state: &ModuleCommandBindings,
        inputs: &crate::variable_bindings::OriginalVariableInvocation,
        argument_count: usize,
    ) -> Option<Self> {
        body.is_none()
            .then(|| {
                Some(DefaultConstructorCompletion {
                    site: super::CommandAllocationSite {
                        source: Arc::clone(state.current_source_origin.as_ref()?),
                        offset: site,
                    },
                    class: class.clone(),
                    inputs: (0..argument_count)
                        .map(|index| inputs.input(index, &state.source_variables).cloned())
                        .collect::<Option<Vec<_>>>()?,
                })
            })
            .flatten()
            .filter(|completion| completion.is_current(state))
    }

    fn is_current(&self, state: &ModuleCommandBindings) -> bool {
        if state.baseline.native_entry.is_some()
            || state.baseline.unknown_entry
            || state.has_opaque_domain()
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || !state.command_observers.is_quiet()
            || state.source_step_observed()
            || state.source_execution_observed(None)
            || state.runtime_execution_observed(self.class.identity.as_ref())
            || !state.retained_target_is_current(&self.class)
            || self.inputs.iter().any(|input| {
                input.original_word_key().is_none()
                    || !input.is_current(&state.source_variables)
                    || state
                        .baseline
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                        != Some(input.policy())
            })
        {
            return false;
        }
        self.class
            .identity
            .as_ref()
            .and_then(|identity| state.class_definitions.get(identity))
            .is_some_and(|class| {
                class.dispatcher.is_none()
                    && class.implementation_generation == self.class.implementation_generation
                    && class.lifecycle_entries_closed
                    && class.constructor_entry.is_none()
                    && class.constructor_provider.is_none()
                    && class.instance_methods.is_some()
                    && state.class_definition_dependencies_hold(class)
                    && class.original_factory.as_ref().is_some_and(|factory| {
                        factory.is_current(state)
                            && state
                                .default_construction_dependencies_hold(factory.recipe().grammar())
                    })
            })
    }

    fn completed(
        &self,
        state: &ModuleCommandBindings,
        receiver: &SourceObjectInstanceProof,
    ) -> bool {
        self.is_current(state)
            && receiver.allocation().site == self.site
            && receiver.class_target() == &self.class
            && state.receiver_allocation_is_current(receiver)
    }
}

#[derive(Clone, Copy)]
struct ConstructorAllocation<'a> {
    selected: &'a SelectedConstructor,
    receiver: &'a Arc<SourceObjectInstanceProof>,
}

#[derive(Clone, Copy)]
struct ConstructorBodyCall<'a> {
    selected: &'a SelectedLifecycleBody,
    receiver: &'a Arc<SourceObjectInstanceProof>,
}

impl SourceCommandBindings {
    pub(super) fn walk_original_constructor(
        &mut self,
        site: u32,
        words: &[crate::ir::WordExpr],
        effective: &[EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        let selected =
            self.select_original_constructor(site, words, effective, state, target, context)?;
        if selected
            .name
            .as_ref()
            .is_some_and(|name| name.install(site, state).is_none())
        {
            return Some(SourceOutcomes::invocation(
                state,
                Route::Tcl(CompletionCode::Error),
            ));
        }
        let Some(mut receiver) = state.manufacture_object_proof(site, target, context) else {
            return Some(super::opaque_source_invocation(state));
        };
        if selected
            .name
            .as_ref()
            .is_some_and(|name| name.retain(&mut receiver, state).is_none())
        {
            return Some(super::opaque_source_invocation(state));
        }
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_CONSTRUCTOR_ALLOCATION site={site} retained_named={} current={}",
                selected.name.is_some(),
                state.receiver_allocation_is_current(&receiver)
            );
        }
        let from = usize::from(selected.manufacturer.constructor_args_from);
        let mut arguments = Vec::with_capacity(effective.len() - from);
        arguments.push(effective[0].clone());
        arguments.extend_from_slice(&effective[from + 1..]);
        let result = if let Some(body) = selected.body.as_ref() {
            self.walk_constructor_body(
                site,
                &arguments,
                state,
                ConstructorBodyCall {
                    selected: body,
                    receiver: &receiver,
                },
                constructor_arguments(context, from),
            )
        } else {
            let mut outcomes = SourceOutcomes::normal(state);
            if selected
                .default_completion
                .as_ref()
                .is_some_and(|completion| completion.completed(state, &receiver))
            {
                // The independently quiet absent constructor has no body
                // or residual callback to execute after this fresh install.
                outcomes.retain_complete_normal_evaluation();
            }
            outcomes
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
        result.normal_name_value = None;
        result.normal_representation = None;
        result.normal_object = None;
        result.normal_method_prefix = None;
        result.normal_rhs_read = None;
        if result.normal.as_ref().is_some_and(|normal| {
            selected.name.is_some()
                && normal.receiver_allocation_is_current(receiver)
                && normal.constructed_command_key(receiver).is_none()
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
        let name_result = selected
            .name
            .as_ref()
            .and_then(|name| name.result(receiver, &result));
        if let Some(normal) = &result.normal
            && normal.receiver_allocation_is_current(receiver)
        {
            result.normal_object = Some(Arc::clone(receiver));
            result.normal_name_value = name_result.as_ref().and_then(|receipt| {
                super::original_name_value::OriginalProducedNameValue::from_object_command_name_result(
                    receipt,
                    &normal.source_variables,
                )
                .map(Arc::new)
            });
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
        site: u32,
        words: &[crate::ir::WordExpr],
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
        #[cfg(debug_assertions)]
        Self::trace_original_constructor_definition(
            site, definition, target, words, state, context,
        );
        if definition.dispatcher.is_some()
            || !definition.lifecycle_entries_closed
            || definition.implementation_generation != target.implementation_generation
            || !state.class_definition_dependencies_hold(definition)
        {
            return None;
        }
        let dialect = state.baseline.dialect?;
        let original_factory = definition.original_factory.as_ref()?;
        if !original_factory.is_current(state) {
            return None;
        }
        let recipe = original_factory.recipe();
        if recipe.version() != dialect.tcl_version? {
            return None;
        }
        let grammar = recipe.grammar();
        let boundary = grammar.native_constructor_boundary(dialect)?;
        if !state.default_construction_dependencies_hold(grammar) {
            return None;
        }
        let (inputs, argument_count) = Self::original_constructor_inputs(
            site, words, effective, state, target, context, dialect,
        );
        let selector = inputs.input(0, &state.source_variables)?;
        let selected = selector
            .policy()
            .recipe()
            .oo_method_input(selector.bytes())
            .ok()?;
        let manufacturer = *grammar.manufacturer(std::str::from_utf8(selected.selected()).ok()?)?;
        if manufacturer.visibility != MemberVisibility::Exported
            || effective.len().checked_sub(1)? < usize::from(manufacturer.constructor_args_from)
        {
            return None;
        }
        let name = match manufacturer.names_instance_at {
            Some(index) => Some(super::named_manufacture::OriginalNamedManufacture::capture(
                inputs.input(usize::from(index), &state.source_variables)?,
                state,
                context,
            )?),
            None => None,
        };
        let (body, destructor) = self.original_constructor_lifecycle_bodies(definition, state)?;
        let default_completion = DefaultConstructorCompletion::for_absent_body(
            body.as_ref(),
            site,
            target,
            state,
            &inputs,
            argument_count,
        );
        Some(Box::new(SelectedConstructor {
            body,
            destructor,
            manufacturer,
            boundary,
            name,
            default_completion,
        }))
    }

    #[cfg(debug_assertions)]
    fn trace_original_constructor_definition(
        site: u32,
        definition: &super::ClassDefinitionReceipt,
        target: &SourceCommandTarget,
        words: &[crate::ir::WordExpr],
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_CONSTRUCTOR_DEFINITION site={site} dispatcher={} lifecycle_closed={} generation_match={} dependencies={} constructor={} destructor={} factory={} words={} written={:?}",
                definition.dispatcher.is_some(),
                definition.lifecycle_entries_closed,
                definition.implementation_generation == target.implementation_generation,
                state.class_definition_dependencies_hold(definition),
                definition.constructor_entry.is_some(),
                definition.destructor_entry.is_some(),
                definition.original_factory.is_some(),
                words.len(),
                context.written_arguments.map(<[_]>::len)
            );
        }
    }

    fn original_constructor_inputs(
        site: u32,
        words: &[crate::ir::WordExpr],
        effective: &[EffectiveInvocationWord],
        state: &ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
        dialect: tcl_registry::InvocationDialect,
    ) -> (crate::variable_bindings::OriginalVariableInvocation, usize) {
        let arguments = effective
            .iter()
            .skip(1)
            .map(EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        let invocation = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Dynamic,
            &arguments,
        )
        .with_dialect(dialect);
        let generic = tcl_registry::native_compilation::NativeCompilationSelection::Generic;
        let inputs = super::original_name_value::original_variable_invocation(
            super::SourceScriptOperands {
                compilation_spec: None,
                compilation_selection: &generic,
                words,
                written_arguments: context.written_arguments,
                target,
                arguments: invocation.arguments_ref(),
            },
            site,
            state,
            context,
        );
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_CONSTRUCTOR_INPUTS site={site} selector={:?} name={:?}",
                inputs
                    .input(0, &state.source_variables)
                    .map(crate::signature_scan::scope::SignatureSourceNameInput::bytes),
                inputs
                    .input(1, &state.source_variables)
                    .map(crate::signature_scan::scope::SignatureSourceNameInput::bytes)
            );
        }
        (inputs, arguments.len())
    }

    fn original_constructor_lifecycle_bodies(
        &self,
        definition: &super::ClassDefinitionReceipt,
        state: &ModuleCommandBindings,
    ) -> Option<(Option<SelectedLifecycleBody>, Option<SelectedLifecycleBody>)> {
        let body = match definition.constructor_entry.as_deref() {
            Some(entry) => Some(self.original_lifecycle_body(
                entry,
                definition.constructor_provider.as_ref()?,
                state,
            )?),
            None => None,
        };
        let destructor = match definition.destructor_entry.as_deref() {
            Some(entry) => Some(self.original_lifecycle_body(
                entry,
                definition.destructor_provider.as_ref()?,
                state,
            )?),
            None => None,
        };
        Some((body, destructor))
    }

    fn original_lifecycle_body(
        &self,
        entry: &SourceConstructorEntry,
        provider: &SourceCommandTarget,
        state: &ModuleCommandBindings,
    ) -> Option<SelectedLifecycleBody> {
        if !state.retained_target_is_current(provider)
            || state.tainted_object_dispatch.contains("*")
            || state.tainted_object_dispatch.contains(&provider.command)
        {
            return None;
        }
        let body = self
            .deferred
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
            .cloned()?;
        Some(SelectedLifecycleBody {
            body,
            provider: provider.clone(),
        })
    }

    fn walk_constructor_body(
        &mut self,
        site: u32,
        arguments: &[EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        call: ConstructorBodyCall<'_>,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let ConstructorBodyCall { selected, receiver } = call;
        let body = &selected.body;
        let implementation = body.implementation_id();
        if !self.active_calls.insert(implementation.clone()) {
            return super::opaque_source_invocation(state);
        }
        self.called_implementations.insert(implementation.clone());
        let target = &selected.provider;
        let activation = super::source_called_body_activation_name(
            state.current_source_origin.as_ref(),
            target,
            body,
            site,
        );
        Arc::make_mut(&mut state.object_instances)
            .receivers
            .insert(activation.clone(), Arc::clone(receiver));
        let mut outcomes = self.walk_called_body(
            site,
            arguments,
            state,
            super::receiver_self::CalledBodyTarget {
                target,
                receiver: Some(super::receiver_self::CalledBodyReceiver::Instance),
            },
            body,
            context,
        );
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
                    ConstructorBodyCall {
                        selected: body,
                        receiver,
                    },
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
        written_name_values: context
            .written_name_values
            .and_then(|values| values.get(from..)),
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
        written_name_values: None,
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

    // Native proof: naming.tcloo.inherited-constructor-provider
    // docs/design/analysis/name-resolution-proofs/inherited-constructor-provider.md
    #[test]
    fn inherited_lifecycle_uses_declaring_provider_and_actual_receiver() {
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for relation in ["superclass", "mixin"] {
                let source = format!(
                    "set ::side BEFORE; oo::class create B {{constructor {{x}} {{set ::side $x}}; destructor {{set ::side CLEAN}}}}; oo::class create C {{{relation} B}}; set object [C create c1 AFTER]; puts $::side; $object missing"
                );
                let bindings = analyse(&source, profile);
                let factory = u32::try_from(source.find("C create c1").unwrap()).unwrap();
                let invocation = bindings.invocation_at_source("C", factory);
                let (provider, entry, _) = invocation
                    .constructor_entry(
                        &tcl_registry::CommandRegistry::build_default().project_for_profile(
                            tcl_dialect::DialectProfile::find(profile).unwrap(),
                        ),
                    )
                    .expect(&source);
                assert_eq!(provider.command, "::B", "{profile}: {source}");
                assert_eq!(
                    entry.declaration().offset,
                    u32::try_from(source.find("constructor {x}").unwrap()).unwrap()
                );
                let read = u32::try_from(source.find("puts $::side").unwrap()).unwrap();
                assert_eq!(
                    bindings
                        .invocation_at_source("puts", read)
                        .evaluated_argument_values
                        .first()
                        .and_then(Option::as_deref),
                    Some("AFTER"),
                    "{profile}: {source}"
                );
                let call = u32::try_from(source.find("$object missing").unwrap()).unwrap();
                let proof = bindings
                    .variable_accesses
                    .get(&call)
                    .and_then(|reads| reads.first())
                    .and_then(|read| read.proved_object_instance())
                    .expect(&source);
                assert_eq!(proof.class_target().command, "::C");
            }
        }
    }

    // Native proof: naming.tcloo.inherited-destructor-cleanup
    // docs/design/analysis/name-resolution-proofs/inherited-destructor-cleanup.md
    #[test]
    fn inherited_constructor_failure_uses_inherited_destructor() {
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "set ::side BEFORE; oo::class create B {constructor {} {error BOOM}; destructor {set ::side CLEAN}}; oo::class create C {superclass B}; catch {C create c1}; puts $::side; c1 absent";
            let bindings = analyse(source, profile);
            let read = u32::try_from(source.find("puts $::side").unwrap()).unwrap();
            assert_eq!(
                bindings
                    .invocation_at_source("puts", read)
                    .evaluated_argument_values
                    .first()
                    .and_then(Option::as_deref),
                Some("CLEAN"),
                "{profile}"
            );
            let missing = u32::try_from(source.find("c1 absent").unwrap()).unwrap();
            assert!(
                bindings
                    .invocation_at_source("c1", missing)
                    .proved_target()
                    .is_none()
            );
        }
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
