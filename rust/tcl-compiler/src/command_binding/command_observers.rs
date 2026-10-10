// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Command trace installation retains a registration; effects occur at a reached event.

use super::{
    Arc, CommandIdentity, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext,
    SourceOutcomes, source_binding,
};
use tcl_registry::{TraceOperation, TraceOperationSet, TraceTarget, TraceTransition};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SourceCommandObserver {
    pub(super) token: Option<CommandIdentity>,
    operations: Option<Vec<TraceOperation>>,
    prefix: Option<String>,
    conditional: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(super) struct SourceCommandObservers {
    pub(super) registrations: Vec<SourceCommandObserver>,
    pub(super) pending_mutations: Vec<Arc<super::command_mutations::SourceCommandMutationReceipt>>,
    firing: Vec<SourceCommandObserver>,
    active_steps: Vec<SourceCommandObserver>,
}

impl SourceCommandObserver {
    fn same_registration(&self, other: &Self) -> bool {
        self.token == other.token
            && self.operations == other.operations
            && self.prefix == other.prefix
    }
}

impl SourceCommandObservers {
    /// No retained command observer, pending mutation or active callback.
    pub(super) fn is_quiet(&self) -> bool {
        self.registrations.is_empty()
            && self.pending_mutations.is_empty()
            && self.firing.is_empty()
            && self.active_steps.is_empty()
    }

    pub(super) fn join(&mut self, other: &Self) -> bool {
        let previous = self.clone();
        let mut registrations = Vec::new();
        for registration in self.registrations.iter().chain(&other.registrations) {
            if registrations
                .iter()
                .any(|known: &SourceCommandObserver| known.same_registration(registration))
            {
                continue;
            }
            let left = self
                .registrations
                .iter()
                .filter(|known| known.same_registration(registration))
                .count();
            let right = other
                .registrations
                .iter()
                .filter(|known| known.same_registration(registration))
                .count();
            for index in 0..left.max(right) {
                let mut joined = registration.clone();
                joined.conditional |= index >= left.min(right)
                    || self
                        .registrations
                        .iter()
                        .chain(&other.registrations)
                        .any(|known| known.same_registration(registration) && known.conditional);
                registrations.push(joined);
            }
        }
        self.registrations = registrations;
        for registration in &other.firing {
            if !self.firing.contains(registration) {
                self.firing.push(registration.clone());
            }
        }
        for registration in &other.active_steps {
            if !self.active_steps.contains(registration) {
                self.active_steps.push(registration.clone());
            }
        }
        for receipt in &other.pending_mutations {
            if !self.pending_mutations.contains(receipt) {
                self.pending_mutations.push(Arc::clone(receipt));
            }
        }
        *self != previous
    }
}

impl ModuleCommandBindings {
    pub(super) fn retain_command_observer(
        &mut self,
        transition: &TraceTransition,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) {
        let (target, operations, prefix, add) = match transition {
            TraceTransition::Add {
                target,
                operations,
                prefix,
            } => (target, operations, prefix, true),
            TraceTransition::Remove {
                target,
                operations,
                prefix,
            } => (target, operations, prefix, false),
        };
        let (TraceTarget::Command(subject) | TraceTarget::Execution(subject)) = target else {
            return;
        };
        let identities = subject
            .literal()
            .zip(namespace.for_head_context(subject.literal().unwrap_or("")))
            .map_or_else(
                || vec![None],
                |(head, namespace)| self.command_observer_identities(head, namespace.as_ref()),
            );
        let operations = match operations {
            TraceOperationSet::Known(operations) => Some(operations.clone()),
            TraceOperationSet::Unknown(_) => None,
        };
        let prefix = prefix.literal().map(str::to_owned);
        let conditional = identities.len() != 1;
        for token in identities {
            let registration = SourceCommandObserver {
                token: token.clone(),
                operations: operations.clone(),
                prefix: prefix.clone(),
                conditional: conditional || token.is_none(),
            };
            let observers = Arc::make_mut(&mut self.command_observers);
            if add {
                observers.registrations.insert(0, registration);
            } else if registration.token.is_some()
                && registration.operations.is_some()
                && registration.prefix.is_some()
            {
                if let Some(index) = observers
                    .registrations
                    .iter()
                    .position(|old| old.same_registration(&registration))
                {
                    observers.registrations.remove(index);
                }
            } else {
                for old in &mut observers.registrations {
                    if registration
                        .token
                        .as_ref()
                        .is_none_or(|token| old.token.as_ref() == Some(token))
                        && registration
                            .operations
                            .as_ref()
                            .is_none_or(|operations| old.operations.as_ref() == Some(operations))
                        && registration
                            .prefix
                            .as_ref()
                            .is_none_or(|prefix| old.prefix.as_ref() == Some(prefix))
                    {
                        old.conditional = true;
                    }
                }
            }
        }
    }

    fn command_observer_identities(
        &self,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> Vec<Option<CommandIdentity>> {
        let mut identities = Vec::new();
        let Ok(slots) = self.source_keys_checked(head, namespace) else {
            // Unavailable namespace geometry is an unknown event receiver,
            // never evidence that installing the original trace must fail.
            return vec![None];
        };
        for slot in slots {
            let bindings = self.binding_alternatives(&slot);
            for binding in &bindings {
                let identity = match binding {
                    super::MayBinding::Target(target) => target.token.clone(),
                    super::MayBinding::Missing => continue,
                    super::MayBinding::Imported(_) | super::MayBinding::Unknown => None,
                };
                if !identities.contains(&identity) {
                    identities.push(identity);
                }
            }
        }
        if self.has_opaque_domain() && !identities.contains(&None) {
            identities.push(None);
        }
        identities
    }

    pub(super) fn command_observer_registration_invalid(
        &self,
        facts: &tcl_registry::InvocationFacts,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> bool {
        facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions.facts().iter().any(|fact| {
                    let tcl_registry::StateTransition::Trace(TraceTransition::Add {
                        target: TraceTarget::Command(subject) | TraceTarget::Execution(subject),
                        ..
                    }) = &fact.transition
                    else {
                        return false;
                    };
                    subject.literal().is_some_and(|head| {
                        !self.has_opaque_domain()
                            && (head.starts_with("::") || self.source_variables.namespace_known)
                            && self.command_observer_identities(head, namespace).is_empty()
                    })
                })
            })
    }

    pub(super) fn source_step_observed(&self) -> bool {
        !self.command_observers.active_steps.is_empty()
    }

    pub(super) fn source_execution_observed(&self, token: Option<&CommandIdentity>) -> bool {
        self.command_observers.registrations.iter().any(|observer| {
            (observer.token.is_none() || observer.token.as_ref() == token)
                && observer.operations.as_ref().is_none_or(|operations| {
                    operations.iter().any(|operation| {
                        matches!(
                            operation,
                            TraceOperation::Enter
                                | TraceOperation::Leave
                                | TraceOperation::EnterStep
                                | TraceOperation::LeaveStep
                        )
                    })
                })
        })
    }
}

/// Heap-owned registrations remain live across recursive command/body traversal.
pub(super) struct SourceCommandObserverScope {
    pub(super) entry: SourceOutcomes,
    leaves: Vec<SourceCommandObserver>,
    step_leaves: Vec<SourceCommandObserver>,
    previous: Arc<SourceCommandObservers>,
    pub(super) was_observed: bool,
}

impl SourceCommandBindings {
    // The caller retains only a pointer across recursive source traversal.
    #[inline(never)]
    pub(super) fn prepare_command_observers(
        &mut self,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Box<SourceCommandObserverScope> {
        let context = *context;
        let head = effective
            .first()
            .and_then(|word| word.as_registry_word().literal());
        let leaves = Self::command_observers_for_event(
            state,
            head,
            &context.namespace_identity(),
            TraceOperation::Leave,
        );
        let step_leaves = Self::command_observers_for_event(
            state,
            head,
            &context.namespace_identity(),
            TraceOperation::LeaveStep,
        );
        let mut entry = SourceOutcomes::normal(state);
        let mut was_observed = false;
        for operation in [TraceOperation::EnterStep, TraceOperation::Enter] {
            let Some(mut continuing) = entry.normal.take() else {
                break;
            };
            let observers = Self::command_observers_for_event(
                &continuing,
                head,
                &context.namespace_identity(),
                operation,
            );
            was_observed |= !observers.is_empty();
            entry.join(&self.walk_command_observers(
                context.invocation_offset,
                &observers,
                operation,
                &mut continuing,
                context,
            ));
        }
        let previous = if let Some(continuing) = entry.normal.as_mut() {
            Self::enter_command_step_observers(continuing, head, &context.namespace_identity())
        } else {
            Arc::clone(&state.command_observers)
        };
        was_observed |= !leaves.is_empty() || !step_leaves.is_empty();
        Box::new(SourceCommandObserverScope {
            entry,
            leaves,
            step_leaves,
            previous,
            was_observed,
        })
    }

    pub(super) fn finish_command_observer_scope(
        &mut self,
        mut dispatched: SourceOutcomes,
        scope: &SourceCommandObserverScope,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let context = *context;
        Self::restore_command_step_observers(&mut dispatched, &scope.previous);
        for (observers, operation) in [
            (&scope.leaves, TraceOperation::Leave),
            (&scope.step_leaves, TraceOperation::LeaveStep),
        ] {
            dispatched = self.finish_command_observers(
                dispatched,
                context.invocation_offset,
                observers,
                operation,
                context,
            );
        }
        dispatched
    }

    pub(super) fn walk_source_target(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        target: &super::SourceCommandTarget,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let has_mutation_observers = state
            .command_observers
            .registrations
            .iter()
            .any(|observer| {
                observer.operations.as_ref().is_none_or(|operations| {
                    operations.iter().any(|operation| {
                        matches!(operation, TraceOperation::Rename | TraceOperation::Delete)
                    })
                })
            });
        if !has_mutation_observers {
            return self.walk_source_target_after_observers(
                segment, words, effective, state, target, context,
            );
        }
        let Ok(prepared) = super::prepare_source_native_invocation(
            target,
            effective,
            state,
            context.registry,
            context.realm,
        ) else {
            return self.walk_source_target_after_observers(
                segment, words, effective, state, target, context,
            );
        };
        if state.captured_move_destination_exists(&prepared.facts, *context) {
            return SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
            );
        }
        let receipt = state.begin_command_mutation_receipt(&prepared.facts, *context);
        let mut outcomes = self.walk_command_mutation_observers(&prepared.facts, state, *context);
        if let Some(mut continuing) = outcomes.normal.take() {
            outcomes.normal_value = None;
            outcomes.normal_representation = None;
            outcomes.normal_method_prefix = None;
            outcomes.normal_rhs_read = None;
            outcomes.join(&self.walk_source_target_after_observers(
                segment,
                words,
                effective,
                &mut continuing,
                target,
                context,
            ));
        }
        Self::finish_command_mutation_receipt(&mut outcomes, receipt.as_ref());
        outcomes.publish(state);
        outcomes
    }

    pub(super) fn command_observers_for_event(
        state: &ModuleCommandBindings,
        head: Option<&str>,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
        operation: TraceOperation,
    ) -> Vec<SourceCommandObserver> {
        let identities = head.map_or_else(
            || vec![None],
            |head| state.command_observer_identities(head, namespace),
        );
        let registrations = if matches!(
            operation,
            TraceOperation::EnterStep | TraceOperation::LeaveStep
        ) {
            &state.command_observers.active_steps
        } else {
            &state.command_observers.registrations
        };
        registrations
            .iter()
            .filter(|observer| {
                !state
                    .command_observers
                    .firing
                    .iter()
                    .any(|firing| firing.same_registration(observer))
                    && observer
                        .operations
                        .as_ref()
                        .is_none_or(|operations| operations.contains(&operation))
                    && (matches!(
                        operation,
                        TraceOperation::EnterStep | TraceOperation::LeaveStep
                    ) || observer.token.is_none()
                        || identities.contains(&observer.token)
                        || identities.contains(&None))
            })
            .map(|observer| {
                let mut selected = observer.clone();
                selected.conditional |= observer.operations.is_none()
                    || (!matches!(
                        operation,
                        TraceOperation::EnterStep | TraceOperation::LeaveStep
                    ) && (identities.len() != 1 || identities.contains(&None)));
                selected
            })
            .collect()
    }

    pub(super) fn walk_command_observers(
        &mut self,
        site: u32,
        observers: &[SourceCommandObserver],
        operation: TraceOperation,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::normal(state);
        for observer in observers {
            let Some(mut continuing) = outcomes.normal.take() else {
                break;
            };
            if observer.conditional {
                outcomes.join(&SourceOutcomes::normal(&continuing));
            }
            let entered =
                self.walk_command_observer(site, observer, operation, &mut continuing, context);
            outcomes.join(&entered);
        }
        outcomes.publish(state);
        outcomes
    }

    pub(super) fn enter_command_step_observers(
        state: &mut ModuleCommandBindings,
        head: Option<&str>,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> Arc<SourceCommandObservers> {
        let previous = Arc::clone(&state.command_observers);
        let mut steps =
            Self::command_observers_for_event(state, head, namespace, TraceOperation::EnterStep);
        steps.extend(Self::command_observers_for_event(
            state,
            head,
            namespace,
            TraceOperation::LeaveStep,
        ));
        // Step registrations are selected on the outer command, rather than its children.
        let identities = head.map_or_else(
            || vec![None],
            |head| state.command_observer_identities(head, namespace),
        );
        for observer in &state.command_observers.registrations {
            if observer.operations.as_ref().is_none_or(|operations| {
                operations.iter().any(|operation| {
                    matches!(
                        operation,
                        TraceOperation::EnterStep | TraceOperation::LeaveStep
                    )
                })
            }) && (observer.token.is_none()
                || identities.contains(&observer.token)
                || identities.contains(&None))
                && !steps.contains(observer)
            {
                steps.push(observer.clone());
            }
        }
        Arc::make_mut(&mut state.command_observers).active_steps = steps;
        previous
    }

    pub(super) fn restore_command_step_observers(
        outcomes: &mut SourceOutcomes,
        previous: &SourceCommandObservers,
    ) {
        for branch in outcomes
            .normal
            .iter_mut()
            .chain(outcomes.abrupt.iter_mut().map(|(_, branch)| branch))
        {
            Arc::make_mut(&mut branch.command_observers)
                .active_steps
                .clone_from(&previous.active_steps);
        }
    }

    pub(super) fn walk_command_mutation_observers(
        &mut self,
        facts: &tcl_registry::InvocationFacts,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::{CommandBindingTransition as Binding, StateTransition};
        let mut events = Vec::new();
        if let Some(transitions) = facts.state_transitions.declared() {
            for transition in transitions.command_bindings() {
                match transition {
                    Binding::Move { from, .. } => {
                        events.push((from.literal().map(str::to_owned), TraceOperation::Rename));
                    }
                    Binding::Define { name, .. } => {
                        events.push((name.literal().map(str::to_owned), TraceOperation::Delete));
                    }
                    Binding::Delete { interpreter, name }
                        if interpreter
                            .as_ref()
                            .is_none_or(|interpreter| interpreter.literal() == Some("")) =>
                    {
                        events.push((name.literal().map(str::to_owned), TraceOperation::Delete));
                    }
                    Binding::Alias {
                        source_interpreter,
                        alias,
                        ..
                    } if source_interpreter.literal() == Some("") => events.push((
                        alias
                            .literal()
                            .map(|name| tcl_syntax::naming::qualify("::", name)),
                        TraceOperation::Delete,
                    )),
                    Binding::Unknown { .. } => events.push((None, TraceOperation::Delete)),
                    _ => {}
                }
            }
            for fact in transitions.facts() {
                if let StateTransition::Namespace(tcl_registry::NamespaceTransition::Delete {
                    namespace,
                }) = &fact.transition
                {
                    let root =
                        state.namespace_target_key_at(namespace, &context.namespace_identity());
                    if let Some(root) = root {
                        if !matches!(root, super::SourceNamespaceKey::Authored(_))
                            && !state.namespaces.contains(&root)
                        {
                            continue;
                        }
                        let names = state.all_command_keys().into_iter().filter(|key| {
                            let holder = key.holder();
                            holder.as_ref() == &root || holder.is_descendant_of(&root)
                        });
                        events.extend(names.map(|key| {
                            (
                                state.callable_spelling_for_key(&key),
                                TraceOperation::Delete,
                            )
                        }));
                    } else {
                        events.push((None, TraceOperation::Delete));
                    }
                }
            }
        }
        let mut outcomes = SourceOutcomes::normal(state);
        for (name, operation) in events {
            let Some(mut continuing) = outcomes.normal.take() else {
                break;
            };
            let observers = Self::command_observers_for_event(
                &continuing,
                name.as_deref(),
                &context.namespace_identity(),
                operation,
            );
            let callbacks = self
                .walk_command_observers(
                    context.invocation_offset,
                    &observers,
                    operation,
                    &mut continuing,
                    context,
                )
                .capture_tcl_completions();
            outcomes.join(&callbacks);
        }
        outcomes.publish(state);
        outcomes
    }

    pub(super) fn finish_command_observers(
        &mut self,
        outcomes: SourceOutcomes,
        site: u32,
        observers: &[SourceCommandObserver],
        operation: TraceOperation,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        if observers.is_empty() {
            return outcomes;
        }
        let mut finished = SourceOutcomes::default();
        let paths = outcomes
            .normal
            .iter()
            .map(|state| {
                (
                    tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::completion::CompletionCode::Ok,
                    ),
                    state,
                )
            })
            .chain(outcomes.abrupt.iter().map(|(route, state)| (*route, state)));
        for (route, state) in paths {
            if route == tcl_registry::completion_route::InvocationCompletionRoute::ProcessExit {
                finished.add_with_result(route, state, outcomes.result_for(route));
                continue;
            }
            let mut state = super::boxed_source_branch(state);
            let callbacks =
                self.walk_command_observers(site, observers, operation, &mut state, context);
            if let Some(state) = callbacks.normal {
                finished.add_with_result(
                    route,
                    &state,
                    if route.normal_possible() {
                        outcomes.normal_value.as_ref()
                    } else {
                        outcomes.result_for(route)
                    },
                );
            }
            for (route, state) in callbacks.abrupt {
                finished.add_abrupt(route, &state);
            }
        }
        finished.normal_representation = outcomes.normal_representation.filter(|receipt| {
            finished
                .normal
                .as_ref()
                .is_some_and(|normal| receipt.is_current(&normal.source_variables))
        });
        finished
    }

    fn walk_command_observer(
        &mut self,
        site: u32,
        observer: &SourceCommandObserver,
        operation: TraceOperation,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let (unknown_arguments, name) = match operation {
            TraceOperation::Enter => (1, "enter"),
            TraceOperation::Leave => (3, "leave"),
            TraceOperation::EnterStep => (1, "enterstep"),
            TraceOperation::LeaveStep => (3, "leavestep"),
            TraceOperation::Rename => (2, "rename"),
            TraceOperation::Delete => (2, "delete"),
            _ => return super::opaque_source_invocation(state),
        };
        let mut arguments =
            vec![crate::registry_invocation::EffectiveInvocationWord::Dynamic; unknown_arguments];
        arguments
            .push(crate::registry_invocation::EffectiveInvocationWord::Literal(name.to_owned()));
        Arc::make_mut(&mut state.command_observers)
            .firing
            .push(observer.clone());
        let mut outcomes = self.walk_reached_callback_prefix(
            site,
            observer.prefix.as_deref(),
            &arguments,
            state,
            context,
        );
        for branch in outcomes
            .normal
            .iter_mut()
            .chain(outcomes.abrupt.iter_mut().map(|(_, branch)| branch))
        {
            Arc::make_mut(&mut branch.command_observers)
                .firing
                .retain(|firing| !firing.same_registration(observer));
        }
        outcomes.normal_value = None;
        outcomes.normal_representation = None;
        outcomes.normal_method_prefix = None;
        outcomes.normal_rhs_read = None;
        outcomes.publish(state);
        outcomes
    }

    pub(super) fn walk_reached_original_callback_prefix(
        &mut self,
        site: u32,
        prefix: Option<&crate::signature_scan::scope::SignatureSourceNameInput>,
        arguments: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(callback) = original_trace_callback_words(prefix, &state.source_variables) else {
            return super::opaque_source_invocation(state);
        };
        let namespace = context.namespace_identity();
        let Some(target) =
            super::source_binding_from_original_input(state, &callback.inputs[0], &namespace)
                .and_then(|binding| {
                    binding
                        .proved_handler_target()
                        .filter(|target| !target.registry_backed)
                        .cloned()
                })
        else {
            return super::opaque_source_invocation(state);
        };
        let mut words = callback.words;
        words.extend_from_slice(arguments);
        let mut outcomes = self.walk_document_target(
            site,
            None,
            &words,
            state,
            &target,
            SourceExecutionContext {
                namespace_key: Some(&namespace),
                depth: context.depth + 1,
                compilation: callback.compilation,
                config: callback.config,
                selected_compilation: None,
                original_variable_compilation: None,
                written_arguments: None,
                written_name_values: None,
                written_representations: None,
                written_objects: None,
                written_method_prefixes: None,
                written_variable_reads: None,
                ..context
            },
        );
        outcomes.normal_value = None;
        outcomes.normal_name_value = None;
        outcomes.normal_representation = None;
        outcomes.publish(state);
        outcomes
    }

    pub(super) fn walk_reached_callback_prefix(
        &mut self,
        site: u32,
        prefix: Option<&str>,
        arguments: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let prefix = prefix
            .zip(state.baseline.dialect)
            .and_then(|(prefix, dialect)| {
                tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar)
                    .split_list(prefix)
                    .ok()
                    .map(|words| {
                        words
                            .into_iter()
                            .map(std::borrow::Cow::into_owned)
                            .collect::<Vec<_>>()
                    })
            });
        let Some(prefix) = prefix.filter(|prefix| !prefix.is_empty()) else {
            return super::opaque_source_invocation(state);
        };
        let Some(root) = state.source_root_namespace_key() else {
            return super::opaque_source_invocation(state);
        };
        let callback = source_binding(state, &prefix[0], &root);
        let Some(target) = callback
            .proved_handler_target()
            .filter(|target| !target.registry_backed)
            .cloned()
        else {
            return super::opaque_source_invocation(state);
        };
        let mut words = prefix
            .into_iter()
            .map(crate::registry_invocation::EffectiveInvocationWord::Literal)
            .collect::<Vec<_>>();
        words.extend_from_slice(arguments);
        let mut outcomes = self.walk_document_target(
            site,
            None,
            &words,
            state,
            &target,
            SourceExecutionContext {
                namespace: "::",
                namespace_key: Some(&root),
                depth: context.depth + 1,
                written_arguments: None,
                ..context
            },
        );
        outcomes.normal_value = None;
        outcomes.normal_representation = None;
        outcomes.publish(state);
        outcomes
    }
}

/// A list projection is admitted only when the actual counted callback script,
/// with one appended quoted operand, has the same single static command. This
/// excludes substitutions, comments, separators and expansion that change argv.
pub(super) struct OriginalTraceCallbackWords {
    pub(super) inputs: Vec<crate::signature_scan::scope::SignatureSourceNameInput>,
    words: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    compilation: tcl_registry::native_compilation::NativeCompilationContext,
    config: tcl_lexer::LexerConfig,
}

fn original_trace_callback_words(
    prefix: Option<&crate::signature_scan::scope::SignatureSourceNameInput>,
    variables: &crate::var_resolve::ResolveContext,
) -> Option<OriginalTraceCallbackWords> {
    original_trace_callback_words_for_target(prefix, variables, OriginalTraceCallbackKind::Variable)
}

#[derive(Clone, Copy)]
pub(super) enum OriginalTraceCallbackKind {
    Variable,
    Command,
    Execution,
}

pub(super) fn original_trace_callback_words_for_target(
    prefix: Option<&crate::signature_scan::scope::SignatureSourceNameInput>,
    variables: &crate::var_resolve::ResolveContext,
    kind: OriginalTraceCallbackKind,
) -> Option<OriginalTraceCallbackWords> {
    let prefix = prefix?;
    prefix.is_current(variables).then_some(())?;
    let dialect = variables.invocation_dialect?;
    original_trace_callback_words_in_dialect(prefix, dialect, kind)
}

pub(super) fn original_trace_callback_words_in_dialect(
    prefix: &crate::signature_scan::scope::SignatureSourceNameInput,
    dialect: tcl_registry::InvocationDialect,
    kind: OriginalTraceCallbackKind,
) -> Option<OriginalTraceCallbackWords> {
    const SUFFIX: &[u8] = b"__tcl_lsp_trace_suffix__";
    (dialect.native_string_protocol() == Some(prefix.policy().string_protocol())).then_some(())?;
    let protocol = dialect.native_variable_trace_protocol()?;
    let actual = tcl_registry::InvocationDialect::for_version(protocol.version());
    let config = tcl_lexer::LexerConfig::from_grammar(actual.lexer_grammar);
    let inputs = prefix.original_list_elements()?;
    if inputs.is_empty() {
        return None;
    }
    let mut source = prefix.bytes().to_vec();
    source.push(b' ');
    source.extend_from_slice(SUFFIX);
    let source = match kind {
        OriginalTraceCallbackKind::Variable => protocol.variable_callback_source(&source),
        OriginalTraceCallbackKind::Command => protocol.command_callback_source(false, &source),
        OriginalTraceCallbackKind::Execution => protocol.command_callback_source(true, &source),
    };
    let image = tcl_lexer::SourceImage::native(source);
    let extent = tcl_lexer::Span::new(0, u32::try_from(image.len()).ok()?);
    let plan = tcl_lexer::native_script_words_in(image, extent, config).ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    let command = &plan.commands[0];
    if command.words.len() != inputs.len() + 1
        || command.words.iter().any(|word| word.group().expand)
    {
        return None;
    }
    let native = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &command.words,
        prefix.policy().string_protocol(),
    )
    .ok()?;
    for (ordinal, input) in inputs.iter().enumerate() {
        (native.literal(ordinal) == Some(input.bytes())).then_some(())?;
    }
    (native.literal(inputs.len()) == Some(SUFFIX)).then_some(())?;
    let words = inputs
        .iter()
        .map(|input| crate::registry_invocation::EffectiveInvocationWord::from_bytes(input.bytes()))
        .collect();
    Some(OriginalTraceCallbackWords {
        inputs,
        words,
        compilation: protocol.callback_compilation(),
        config,
    })
}

#[cfg(test)]
mod tests {
    use super::SourceCommandBindings;
    use crate::command_binding::SourceAnalysisOptions;
    use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationMode};

    fn analyse(source: &str) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    fn original_length_at(bindings: &SourceCommandBindings, offset: usize) -> bool {
        bindings
            .invocation_at_source("llength", u32::try_from(offset).unwrap())
            .proved_handler_target()
            .is_some_and(|target| target.registry_backed && target.command == "::llength")
    }

    #[test]
    fn original_trace_prefix_projection_requires_actual_script_argv_equivalence() {
        // Implementation contract: naming.source.readonly-original-operand-projections (docs/design/analysis/name-resolution-proofs/readonly-original-operand-projections.md).
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            let registry = tcl_registry::CommandRegistry::build_default();
            for (prefix, accepted) in [
                ("callback {baked value}", true),
                ("callback", true),
                ("callback; other", false),
                ("callback;", false),
                ("callback\n", false),
                ("# ignored\ncallback", false),
                ("$command baked", false),
                ("callback [list value]", false),
            ] {
                let source = format!("list {{{prefix}}}");
                let bindings = SourceCommandBindings::analyse_with_options(
                    &source,
                    config,
                    &registry,
                    SourceAnalysisOptions {
                        invocation_dialect: Some(dialect),
                        native_compilation: NativeCompilationContext {
                            mode: NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                );
                let segment =
                    crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config)
                        .remove(0);
                let tokens = crate::ir::CommandTokens::from_segmented(
                    &tcl_lexer::SourceMap::new(&source),
                    config,
                    &segment,
                );
                let binding = bindings.invocation_at_source("list", 0);
                let input = binding.original_written_name_input(&tokens, 1).unwrap();
                let copied = crate::signature_scan::scope::SignatureSourceNameValue::copied_variable_trace_prefix(&input, &binding.variable_context).unwrap();
                let copied =
                    crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(copied);
                let projected =
                    super::original_trace_callback_words(Some(&copied), &binding.variable_context);
                assert_eq!(projected.is_some(), accepted, "{version:?}: {prefix:?}");
                if let Some(projected) = projected {
                    assert_eq!(projected.inputs[0].bytes(), b"callback");
                    assert!(
                        projected
                            .inputs
                            .iter()
                            .all(|input| input.original_word_key().is_none())
                    );
                    assert_eq!(projected.compilation.mode, NativeCompilationMode::Direct);
                    assert!(
                        super::original_trace_callback_words(None, &binding.variable_context)
                            .is_none()
                    );
                }
            }
        }
    }

    #[test]
    fn native_global_callback_keeps_actual_root_lookup_and_context_after_substitutions() {
        use tcl_runtime_api::native_compilation::{
            NativeCommandImplementation, NativeCompilerHookPresence,
        };
        let mut entry = crate::command_binding::named_invocation::tests::native_entry();
        for (token, name) in [(8, "trace"), (9, "llength")] {
            let mut row = entry.commands[1].clone();
            row.slot.simple = name.into();
            row.token = token;
            row.implementation = NativeCommandImplementation::Registry {
                identity: name.into(),
                compiler_hook: false,
            };
            row.compiler_hook = NativeCompilerHookPresence::Absent;
            row.compiler = None;
            entry.commands.push(row);
        }
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        // The observed procedure lives in the colon-named current namespace;
        // the original unqualified callback prefix resolves at actual root.
        let source = "proc ::cb args {rename llength savedLength}; proc observed args {}; trace add execution observed enter cb; observed [llength {a b}]; savedLength {a b}";
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_entry: Some(&entry),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..SourceAnalysisOptions::default()
            },
        );
        for (word, offset) in [
            ("llength", source.find("[llength").unwrap() + 1),
            ("savedLength", source.rfind("savedLength").unwrap()),
        ] {
            let binding = bindings.invocation_at_source(word, u32::try_from(offset).unwrap());
            let target = binding
                .proved_handler_target()
                .expect("actual callback preserves native target selection");
            assert!(target.registry_backed);
            assert_eq!(target.identity.as_ref().unwrap().runtime.unwrap().token, 9);
            assert_eq!(super::super::nqn(&target.command), "::llength");
        }
    }

    #[test]
    fn installing_an_unknown_callback_preserves_unrelated_command_lookup() {
        let source = "proc observed args {}; trace add execution observed enter absentCallback; llength {a b}";
        let bindings = analyse(source);
        assert!(original_length_at(
            &bindings,
            source.find("llength").unwrap()
        ));
    }

    #[test]
    fn a_closed_empty_callback_preserves_the_reached_command_world() {
        let source = "proc cb args {}; proc observed args {}; trace add execution observed enter cb; observed; llength {a b}";
        let bindings = analyse(source);
        assert!(original_length_at(
            &bindings,
            source.find("llength").unwrap()
        ));
    }

    #[test]
    fn callback_mutation_occurs_after_original_argument_substitutions() {
        let source = "proc cb args {rename llength savedLength}; proc observed args {}; trace add execution observed enter cb; observed [llength {a b}]; llength {a b}";
        let bindings = analyse(source);
        let before = source.find("[llength").unwrap() + 1;
        let after = source.rfind("llength").unwrap();
        assert!(original_length_at(&bindings, before));
        assert!(!original_length_at(&bindings, after));
    }

    #[test]
    fn removing_the_exact_registration_prevents_callback_effects() {
        let source = "proc observed args {}; trace add execution observed enter absentCallback; trace remove execution observed enter absentCallback; observed; llength {a b}";
        let bindings = analyse(source);
        assert!(original_length_at(
            &bindings,
            source.find("llength").unwrap()
        ));
    }

    #[test]
    fn command_deletion_callback_errors_do_not_discard_the_mutation() {
        let source = "proc observed args {}; proc cb args {rename llength savedLength; error callback}; trace add command observed delete cb; rename observed {}; llength {a b}";
        let bindings = analyse(source);
        assert!(!original_length_at(
            &bindings,
            source.rfind("llength").unwrap()
        ));
    }

    #[test]
    fn identical_trace_registrations_are_distinct_and_remove_retires_one() {
        let source = "set n 0; proc cb args {incr ::n}; proc observed {} {}; trace add execution observed enter cb; trace add execution observed enter cb; observed; trace remove execution observed enter cb; observed; list $n";
        let bindings = analyse(source);
        let result = bindings.invocation_at_source(
            "list",
            u32::try_from(source.rfind("list").unwrap()).unwrap(),
        );
        assert_eq!(result.evaluated_argument_values, vec![Some("3".to_owned())]);
    }

    #[test]
    fn absent_traced_target_rejects_installation_before_later_dispatch() {
        let source = "trace add execution absentTarget enter absentCallback; llength {a b}";
        let bindings = analyse(source);
        let binding = bindings.invocation_at_source(
            "llength",
            u32::try_from(source.find("llength").unwrap()).unwrap(),
        );
        assert_eq!(
            binding.runtime_reachability(),
            super::super::SourceRuntimeReachability::NotEntered
        );
    }
    #[test]
    fn a_reached_math_trace_cannot_erase_its_own_removal() {
        let source = "proc cb args {trace remove execution ::tcl::mathfunc::abs enter cb}; trace add execution ::tcl::mathfunc::abs enter cb; expr {abs(-3)}";
        let bindings = analyse(source);
        let calls = bindings
            .implicit_math_invocations
            .values()
            .flatten()
            .collect::<Vec<_>>();
        assert_ne!(
            calls,
            [] as [&crate::command_binding::SourceInvocationBinding; 0]
        );
        assert!(
            calls
                .iter()
                .all(|binding| !binding.unobserved_native_dispatch())
        );
    }

    #[test]
    fn implicit_math_dispatch_retains_the_post_trace_handler() {
        let source = "proc cb args {rename ::tcl::mathfunc::abs ::savedAbs; proc ::tcl::mathfunc::abs args {return NEW}}; trace add execution ::tcl::mathfunc::abs enter cb; expr {abs(-3)}";
        let bindings = analyse(source);
        let calls = bindings
            .implicit_math_invocations
            .values()
            .flatten()
            .collect::<Vec<_>>();
        assert_ne!(
            calls,
            [] as [&crate::command_binding::SourceInvocationBinding; 0]
        );
        assert!(calls.iter().all(|binding| {
            binding
                .proved_target()
                .is_some_and(|target| !target.registry_backed)
                && !binding.unobserved_native_dispatch()
        }));
    }
}
