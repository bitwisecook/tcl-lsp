// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Receiver allocations and advisory builtin selections retained separately.

use super::{
    Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext,
    SourceInvocationBinding, SourceObjectInstanceProof, SourceOutcomes,
};
use crate::var_resolve::VariableExecutionFrame;
use tcl_registry::definer::{BuiltinObjectMethodOperation, MethodReach, TCLOO_GRAMMAR};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct FrozenSourceObjectHead {
    pub(super) word: crate::ir::WordExpr,
    pub(super) object: Arc<SourceObjectInstanceProof>,
}

/// Possible native builtin operations in an original receiver method frame.
/// This is a hazard inventory, without execution, completion or alias authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceReceiverBuiltinCandidates {
    operation: Option<BuiltinObjectMethodOperation>,
    unknown: bool,
}

impl SourceReceiverBuiltinCandidates {
    /// Independently authored native operation still possible at this point.
    #[must_use]
    pub fn operation(&self) -> Option<BuiltinObjectMethodOperation> {
        self.operation
    }

    /// The receiving allocation or implementation has an unresolved residual.
    #[must_use]
    pub fn unknown(&self) -> bool {
        self.unknown
    }
}

impl ModuleCommandBindings {
    /// A fresh receiver's private namespace contains only its stock dispatcher.
    /// Its authored helper path precedes global lookup. No namespace name is
    /// invented; mutation of the receiver-local command world revokes this route.
    pub(super) fn receiver_command_lookup_path(&self, head: &str) -> Option<Vec<String>> {
        if head.starts_with("::") {
            return None;
        }
        let receiver = self.active_method_receiver(&self.variable_frame)?;
        let (first, remaining) = receiver.receiver_namespace_path?.split_first()?;
        Some(tcl_syntax::naming::command_resolution_candidates(
            first, remaining, head,
        ))
    }

    /// The audited helper path of a current fresh receiver uses original
    /// rooted recipe operands against this interpreter's actual namespace
    /// world. The receiver's display label supplies no namespace identity.
    pub(super) fn receiver_native_lookup_paths(
        &self,
        head: &str,
        namespace: &super::SourceNamespaceKey,
    ) -> Option<
        Result<
            Vec<Vec<super::SourceCommandKey>>,
            tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
        >,
    > {
        if self.baseline.native_entry.is_none()
            || self.variable_frame.namespace_identity() != Some(namespace)
        {
            return None;
        }
        let written = self.receiver_command_lookup_path(head)?;
        Some((|| {
            use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable as Unavailable;
            let root = self
                .source_root_namespace_key()
                .ok_or(Unavailable::Namespace)?;
            let mut result = Vec::new();
            let mut absent = None;
            for head in written {
                let paths = self.native_source_lookup_paths(&head, &root)?;
                let [path] = paths.as_slice() else {
                    return Err(Unavailable::Namespace);
                };
                for key in path {
                    let bindings = self.bindings.get(key);
                    let missing = bindings
                        .is_none_or(|bindings| bindings.contains(&super::MayBinding::Missing));
                    if missing {
                        absent = Some(key.clone());
                    }
                    if (!missing || bindings.is_some_and(|bindings| bindings.len() > 1))
                        && !result.contains(key)
                    {
                        result.push(key.clone());
                    }
                    if !missing {
                        return Ok(vec![result]);
                    }
                }
            }
            if let Some(key) = absent
                && !result.contains(&key)
            {
                result.push(key);
            }
            Ok(vec![result])
        })())
    }

    pub(super) fn receiver_allocation_is_current(&self, proof: &SourceObjectInstanceProof) -> bool {
        !self.opaque_domain
            && self.object_instances.generation == Some(proof.dispatch_generation())
            && self.retained_target_is_current(proof.class_target())
            && proof
                .class_target()
                .identity
                .as_ref()
                .and_then(|identity| self.class_definitions.get(identity))
                .is_some_and(|definition| self.class_definition_dependencies_hold(definition))
    }

    pub(super) fn active_method_receiver(
        &self,
        frame: &VariableExecutionFrame,
    ) -> Option<&Arc<SourceObjectInstanceProof>> {
        let VariableExecutionFrame::ReceiverMethod { identity } = frame.layout() else {
            return None;
        };
        self.object_instances
            .receivers
            .get(identity)
            .filter(|proof| {
                self.receiver_allocation_is_current(proof)
                    && proof.receiver_dispatcher_generation.is_some()
                    && proof.receiver_dispatcher_generation
                        == self.object_instances.receiver_dispatcher_generation
            })
    }
}

impl SourceInvocationBinding {
    /// Original self-method declarations possible in this declaration's own
    /// receiver frame. The receiving instance and dispatch may remain unknown;
    /// this grants result/navigation advice only, never executable effects.
    #[must_use]
    pub fn declared_self_method_candidates(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<&super::SourceReceiverMethodEntry> {
        let Some(snapshot) = self.lookup_state.as_ref() else {
            return Vec::new();
        };
        let state = &snapshot.state;
        let Some(head) = self
            .frozen_written_words()
            .and_then(|words| words.first())
            .and_then(|word| word.as_registry_word().literal())
        else {
            return Vec::new();
        };
        if head.contains("::")
            || registry.method_dispatch_keyword(head)
                != Some(tcl_registry::MethodDispatchKind::SelfDispatch)
            || !matches!(
                self.variable_frame.layout(),
                VariableExecutionFrame::ReceiverMethod { .. }
            )
            || state.source_step_observed()
            || state.source_execution_observed(None)
        {
            return Vec::new();
        }
        let Some(method) = self
            .evaluated_argument_values
            .first()
            .and_then(Option::as_deref)
        else {
            return Vec::new();
        };
        state
            .class_definitions
            .values()
            .filter(|definition| state.class_definition_dependencies_hold(definition))
            .flat_map(|definition| {
                definition
                    .receiver_method_entries
                    .iter()
                    .filter_map(|((kind, _), owner)| {
                        (owner.frame() == &self.variable_frame)
                            .then(|| {
                                definition
                                    .receiver_method_entries
                                    .get(&(*kind, method.to_owned()))
                            })
                            .flatten()
                    })
            })
            .collect()
    }

    /// Original declared method selected through the receiver-local self
    /// dispatcher of an actual current allocation. This navigation receipt
    /// retains the receiving class separately from the declaring class and
    /// grants no editable selector, visibility, completion or opcode proof.
    #[must_use]
    pub fn receiver_self_method_entry(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(
        &super::SourceCommandTarget,
        &super::SourceReceiverMethodEntry,
        u64,
    )> {
        let state = &self.lookup_state.as_ref()?.state;
        let head = self
            .frozen_written_words()?
            .first()?
            .as_registry_word()
            .literal()?;
        if head.contains("::")
            || registry.method_dispatch_keyword(head)
                != Some(tcl_registry::MethodDispatchKind::SelfDispatch)
            || self.entered_execution_observer.observed()
            || state.source_step_observed()
            || state.source_execution_observed(None)
        {
            return None;
        }
        let receiver = state.active_method_receiver(&self.variable_frame)?;
        let method = self.evaluated_argument_values.first()?.as_ref()?;
        let definition = state
            .class_definitions
            .get(receiver.class_target().identity.as_ref()?)?;
        let entry = definition
            .receiver_method_entries
            .get(&(super::SourceMethodReceiver::Instance, method.clone()))?;
        Some((
            receiver.class_target(),
            entry,
            receiver.dispatch_generation(),
        ))
    }

    /// Original method selected by an identity-bearing command result frozen
    /// in the original head word. This is navigation evidence only; bytes,
    /// visibility, editable selectors and command completion stay separate.
    #[must_use]
    pub fn frozen_object_receiver_method_entry(
        &self,
        head: &crate::ir::WordExpr,
    ) -> Option<(
        &super::SourceCommandTarget,
        &super::SourceReceiverMethodEntry,
        u64,
    )> {
        let frozen = self.frozen_head_object.as_ref()?;
        let proof = &frozen.object;
        let state = &self.lookup_state.as_ref()?.state;
        if head != &frozen.word
            || head.source().span.start() != self.dispatch_site.as_ref()?.offset
            || self.entered_execution_observer.observed()
            || state.source_step_observed()
            || state.source_execution_observed(None)
            || !state.receiver_allocation_is_current(proof)
            || proof.receiver_dispatcher_generation.is_none()
            || proof.receiver_dispatcher_generation
                != state.object_instances.receiver_dispatcher_generation
        {
            return None;
        }
        let method = self.evaluated_argument_values.first()?.as_ref()?;
        let definition = state
            .class_definitions
            .get(proof.class_target().identity.as_ref()?)?;
        let entry = definition
            .receiver_method_entries
            .get(&(super::SourceMethodReceiver::Instance, method.clone()))?;
        Some((proof.class_target(), entry, proof.dispatch_generation()))
    }

    /// Possible receiver-local builtin operation selected from the original
    /// method declaration or an actual reached receiver. A custom declared
    /// override excludes the builtin. Generic external activation retains an
    /// unknown receiver residual and supplies no physical alias proof.
    #[must_use]
    pub fn receiver_self_builtin_candidates(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<SourceReceiverBuiltinCandidates> {
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        let VariableExecutionFrame::ReceiverMethod { .. } = self.variable_frame.layout() else {
            return None;
        };
        let head = self
            .frozen_written_words()?
            .first()?
            .as_registry_word()
            .literal()?;
        if head.contains("::")
            || registry.method_dispatch_keyword(head)
                != Some(tcl_registry::MethodDispatchKind::SelfDispatch)
            || state.source_step_observed()
            || state.source_execution_observed(None)
        {
            return None;
        }
        let method = self.evaluated_argument_values.first()?.as_deref()?;
        let operation = TCLOO_GRAMMAR.builtin_method_operation(
            method,
            MethodReach::SelfDispatch,
            state.baseline.dialect?,
        )?;
        let reached = state.active_method_receiver(&self.variable_frame);
        let (definition, receiver_kind) = if let Some(receiver) = reached {
            (
                state
                    .class_definitions
                    .get(receiver.class_target().identity.as_ref()?)?,
                super::SourceMethodReceiver::Instance,
            )
        } else {
            state.class_definitions.values().find_map(|definition| {
                definition
                    .receiver_method_entries
                    .iter()
                    .find_map(|((kind, _), entry)| {
                        (entry.frame() == &self.variable_frame).then_some((definition, *kind))
                    })
            })?
        };
        if !state.class_definition_dependencies_hold(definition) {
            return None;
        }
        let overridden = definition
            .receiver_method_entries
            .contains_key(&(receiver_kind, method.to_owned()));
        Some(SourceReceiverBuiltinCandidates {
            operation: (!overridden).then_some(operation),
            unknown: reached.is_none(),
        })
    }
}

impl SourceCommandBindings {
    pub(super) fn retained_receiver_result(
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        let head = effective.first()?.as_registry_word().literal()?;
        let argument = match effective.get(1) {
            Some(word) => Some(word.as_registry_word().literal()?),
            None => None,
        };
        if effective.len() > 2
            || !context.registry.is_self_receiver_call(head, argument)
            || (!head.starts_with("::")
                && state
                    .unknown_lookup_namespaces
                    .contains(&context.namespace_identity()))
        {
            return None;
        }
        let receiver = state.active_method_receiver(context.frame)?;
        let root = state.source_root_namespace_key()?;
        let helper = super::source_binding(state, "::oo::Helpers::self", &root);
        let target = helper.proved_target()?;
        if !target.registry_backed
            || !target.prepended.is_empty()
            || super::nqn(&target.command) != "::oo::Helpers::self"
            || target.implementation_generation != 0
            || state.source_step_observed()
            || state.source_execution_observed(target.identity.as_ref())
        {
            return None;
        }
        let mut result = SourceOutcomes::normal(state);
        result.normal_object = Some(Arc::clone(receiver));
        Some(result)
    }

    pub(super) fn walk_receiver_builtin_links(
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        use crate::allocated_instance::AllocatedInstanceLinkOutcome;
        use tcl_registry::{CompletionCode, completion_route::InvocationCompletionRoute};
        let (receiver, self_dispatch) = selected_receiver(effective, state, context)?;
        if !self_dispatch {
            return None;
        }
        let method = effective.get(1)?.as_registry_word().literal()?;
        let operation = TCLOO_GRAMMAR.builtin_method_operation(
            method,
            MethodReach::SelfDispatch,
            state.baseline.dialect?,
        )?;
        let definition = state
            .class_definitions
            .get(receiver.class_target().identity.as_ref()?)?;
        if definition
            .receiver_method_entries
            .contains_key(&(super::SourceMethodReceiver::Instance, method.to_owned()))
        {
            return None;
        }
        let names = effective
            .iter()
            .skip(2)
            .map(|word| word.as_registry_word().literal().map(str::to_owned))
            .collect::<Vec<_>>();
        let outcome = match operation {
            BuiltinObjectMethodOperation::ObjectVariableLinks => Arc::make_mut(
                &mut state.source_variables,
            )
            .link_allocated_instance_variables(receiver.allocation(), &names, context.registry),
        };
        Some(match outcome {
            AllocatedInstanceLinkOutcome::Linked => SourceOutcomes::normal(state),
            AllocatedInstanceLinkOutcome::Error => SourceOutcomes::invocation(
                state,
                InvocationCompletionRoute::Tcl(CompletionCode::Error),
            ),
            AllocatedInstanceLinkOutcome::Unknown => super::opaque_source_invocation(state),
        })
    }

    /// Enter an original plain method on an actual object or native class
    /// delegate. This supplies no opcode or method navigation grant.
    pub(super) fn walk_retained_receiver_method(
        &mut self,
        site: u32,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        let (receiver, entry) = selected_plain_method(effective, state, context)?;
        let target = entry.declaring_class()?.clone();
        let body = self
            .deferred
            .values()
            .find(|body| {
                body.receiver_method
                    && body
                        .executed_script
                        .as_ref()
                        .is_some_and(|(_, _, source)| source == entry.body())
            })?
            .clone();
        let implementation = body.implementation_id();
        if !self.active_calls.insert(implementation.clone()) {
            return Some(super::opaque_source_invocation(state));
        }
        self.called_implementations.insert(implementation.clone());
        let activation = super::source_called_body_activation_name(
            state.current_source_origin.as_ref(),
            &target,
            &body,
            site,
        );
        if let Some(receiver) = receiver {
            Arc::make_mut(&mut state.object_instances)
                .receivers
                .insert(activation.clone(), receiver);
        }
        let mut arguments = Vec::with_capacity(effective.len() - 1);
        arguments.push(effective.first()?.clone());
        arguments.extend_from_slice(&effective[2..]);
        let mut result = self.walk_called_body(
            site,
            &arguments,
            state,
            &target,
            &body,
            SourceExecutionContext {
                written_arguments: context.written_arguments.and_then(|values| values.get(1..)),
                written_values: context.written_values.and_then(|values| values.get(1..)),
                written_representations: context
                    .written_representations
                    .and_then(|values| values.get(1..)),
                written_objects: context.written_objects.and_then(|values| values.get(1..)),
                written_method_prefixes: context
                    .written_method_prefixes
                    .and_then(|values| values.get(1..)),
                written_variable_reads: context
                    .written_variable_reads
                    .and_then(|values| values.get(1..)),
                ..context
            },
        );
        self.active_calls.remove(&implementation);
        if let Some(normal) = &mut result.normal {
            Arc::make_mut(&mut normal.object_instances)
                .receivers
                .remove(&activation);
        }
        for (_, abrupt) in &mut result.abrupt {
            Arc::make_mut(&mut abrupt.object_instances)
                .receivers
                .remove(&activation);
        }
        result.publish(state);
        Some(result)
    }
}

fn selected_plain_method(
    effective: &[crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<(
    Option<Arc<SourceObjectInstanceProof>>,
    super::SourceReceiverMethodEntry,
)> {
    let method = effective.get(1)?.as_registry_word().literal()?;
    if let Some((receiver, self_dispatch)) = selected_receiver(effective, state, context) {
        let definition = state
            .class_definitions
            .get(receiver.class_target().identity.as_ref()?)?;
        let entry = definition
            .receiver_method_entries
            .get(&(super::SourceMethodReceiver::Instance, method.to_owned()))?;
        return (self_dispatch || entry.is_exported()).then(|| (Some(receiver), entry.clone()));
    }
    if state.source_step_observed()
        || state.source_execution_observed(None)
        || effective.iter().any(|word| {
            matches!(
                word,
                crate::registry_invocation::EffectiveInvocationWord::Expanded
                    | crate::registry_invocation::EffectiveInvocationWord::Opaque
            )
        })
    {
        return None;
    }
    let head = effective.first()?.as_registry_word().literal()?;
    let binding = super::source_binding(state, head, &context.namespace_identity());
    let target = binding.proved_target()?;
    let definition = state.class_definitions.get(target.identity.as_ref()?)?;
    let entry = definition
        .receiver_method_entries
        .get(&(super::SourceMethodReceiver::Class, method.to_owned()))?;
    state
        .native_class_delegate_entry_is_current(target, entry)
        .then(|| (None, entry.clone()))
}

fn selected_receiver(
    effective: &[crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<(Arc<SourceObjectInstanceProof>, bool)> {
    if state.source_step_observed()
        || state.source_execution_observed(None)
        || effective.iter().any(|word| {
            matches!(
                word,
                crate::registry_invocation::EffectiveInvocationWord::Expanded
                    | crate::registry_invocation::EffectiveInvocationWord::Opaque
            )
        })
    {
        return None;
    }
    let head = effective.first()?.as_registry_word().literal();
    let self_dispatch = head.is_some_and(|head| {
        !head.contains("::")
            && context.registry.method_dispatch_keyword(head)
                == Some(tcl_registry::MethodDispatchKind::SelfDispatch)
    });
    let receiver = if self_dispatch {
        Arc::clone(state.active_method_receiver(context.frame)?)
    } else if let Some(proof) = context
        .written_objects
        .and_then(|words| words.first())
        .and_then(Option::as_ref)
    {
        Arc::clone(proof)
    } else {
        let binding = super::source_binding(state, head?, &context.namespace_identity());
        let target = binding.proved_target()?;
        if !target.prepended.is_empty() {
            return None;
        }
        Arc::clone(
            state
                .object_instances
                .named
                .get(target.identity.as_ref()?)?,
        )
    };
    state
        .receiver_allocation_is_current(&receiver)
        .then_some((receiver, self_dispatch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::SourceAnalysisOptions;

    fn analyse(source: &str) -> (SourceCommandBindings, tcl_registry::CommandRegistry) {
        analyse_for_profile(source, "tcl8.6")
    }

    fn analyse_for_profile(
        source: &str,
        dialect: &str,
    ) -> (SourceCommandBindings, tcl_registry::CommandRegistry) {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
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
        );
        (bindings, registry)
    }

    #[test]
    fn declared_receiver_factory_advice_keeps_unknown_execution_separate() {
        let source = "oo::class create D {}; oo::class create C {method obj {} {return [D new]}; method run {} {return [my obj]}}";
        let (bindings, registry) = analyse(source);
        let factory = bindings
            .invocation_at_source("D", u32::try_from(source.find("D new").unwrap()).unwrap());
        assert!(factory.proved_class_definition_factory().is_none());
        assert!(factory.unknown);
        let candidates = factory.class_factory_candidates(&registry);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].command, "::D");
        let dispatch = bindings
            .invocation_at_source("my", u32::try_from(source.find("my obj").unwrap()).unwrap());
        assert!(dispatch.receiver_self_method_entry(&registry).is_none());
        let methods = dispatch.declared_self_method_candidates(&registry);
        assert!(methods.iter().any(|entry| entry.name() == "obj"));
        assert!(dispatch.proved_construction_result(&registry).is_none());

        let source = "oo::class create D {}; rename D {}; proc D args {return plain}; oo::class create C {method obj {} {return [D new]}}";
        let (bindings, registry) = analyse(source);
        let factory = bindings
            .invocation_at_source("D", u32::try_from(source.find("D new").unwrap()).unwrap());
        assert!(factory.class_factory_candidates(&registry).is_empty());
    }

    #[test]
    fn captured_dynamic_factory_selector_does_not_borrow_a_written_member() {
        let source = "oo::class create D {}; oo::class create C {method run {} {D new}}";
        let (bindings, registry) = analyse(source);
        let mut factory = bindings
            .invocation_at_source("D", u32::try_from(source.find("D new").unwrap()).unwrap());
        assert_eq!(factory.class_factory_candidates(&registry).len(), 1);
        for target in &mut factory.targets {
            target.prepended.insert(
                0,
                crate::registry_invocation::EffectiveInvocationWord::Dynamic,
            );
        }
        assert!(factory.class_factory_candidates(&registry).is_empty());
    }

    #[test]
    fn native_class_delegate_body_preserves_later_original_method_selection() {
        for dialect in ["tcl9.0", "tcl9.1"] {
            let source = "oo::class create ActiveRecord {classmethod find {args} {return FOUND}}; oo::class create Table {superclass ActiveRecord}; Table find foo bar; ActiveRecord find foo bar";
            let (bindings, _) = analyse_for_profile(source, dialect);
            for head in ["Table", "ActiveRecord"] {
                let offset = u32::try_from(source.rfind(&format!("{head} find")).unwrap()).unwrap();
                let binding = bindings.invocation_at_source(head, offset);
                assert!(
                    binding.proved_class_definition_factory().is_some(),
                    "{dialect}: {head}"
                );
                assert!(
                    binding
                        .class_definition_method_entries()
                        .is_some_and(|(_, entries)| entries.contains_key(&(
                            super::super::SourceMethodReceiver::Class,
                            "find".to_owned()
                        ))),
                    "{dialect}: {head}"
                );
            }
        }
    }

    #[test]
    fn class_delegate_body_requires_current_factory_worker_and_dispatch_table() {
        for mutation in [
            "rename ::oo::define::classmethod savedWorker; proc ::oo::define::classmethod args {return OTHER};",
            "oo::define Table classmethod find args {return OTHER};",
            "rename ActiveRecord savedBase; oo::class create ActiveRecord {};",
            "rename oo::class savedFactory; proc oo::class args {return OTHER};",
        ] {
            let source = format!(
                "oo::class create ActiveRecord {{classmethod find {{args}} {{return FOUND}}}}; oo::class create Table {{superclass ActiveRecord}}; {mutation} Table find X; puts AFTER"
            );
            let (bindings, _) = analyse_for_profile(&source, "tcl9.1");
            let offset = u32::try_from(source.rfind("puts AFTER").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("puts", offset);
            assert!(
                binding.lookup_state.as_ref().unwrap().state.opaque_domain,
                "{mutation}"
            );
        }
    }

    #[test]
    fn deferred_self_builtin_inventory_has_no_receiving_allocation() {
        for (source, operation) in [
            (
                "oo::class create C {method p {} {my variable x; set x 1}}",
                true,
            ),
            (
                "oo::class create C {method variable args {return NOOP}; method p {} {my variable x; set x 1}}",
                false,
            ),
        ] {
            let (bindings, registry) = analyse(source);
            let offset = u32::try_from(source.find("my variable").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("my", offset);
            let candidate = binding
                .receiver_self_builtin_candidates(&registry)
                .expect(source);
            assert_eq!(candidate.operation().is_some(), operation, "{source}");
            assert!(candidate.unknown(), "{source}");
        }
        for source in [
            "my variable x",
            "oo::class create C {method p {} {::my variable x}}",
        ] {
            let (bindings, registry) = analyse(source);
            let offset = u32::try_from(source.find("my variable").unwrap()).unwrap();
            let head = if source.contains("::my") {
                "::my"
            } else {
                "my"
            };
            let offset = if head == "::my" { offset - 2 } else { offset };
            assert!(
                bindings
                    .invocation_at_source(head, offset)
                    .receiver_self_builtin_candidates(&registry)
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn reached_self_builtin_selection_retains_actual_receiver_and_override() {
        for (extra, builtin) in [("", true), ("method variable args {return NOOP};", false)] {
            let source = format!(
                "oo::class create C {{{extra} method p {{}} {{my variable x; set x 1}}}}; C create rex; rex p"
            );
            let (bindings, registry) = analyse(&source);
            let offset = u32::try_from(source.find("my variable").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("my", offset);
            let candidate = binding
                .receiver_self_builtin_candidates(&registry)
                .expect(&source);
            assert_eq!(candidate.operation().is_some(), builtin, "{source}");
            assert!(!candidate.unknown(), "{source}");
        }
    }

    #[test]
    fn internal_navigation_requires_the_reached_receiver_after_arguments() {
        for (middle, reached) in [("", true), (" [oo::objdefine rex class B]", false)] {
            let source = format!(
                "oo::class create C {{method Q {{args}} {{return PRIVATE}}; method p {{}} {{my Q{middle}}}}}; oo::class create B {{}}; C create rex; rex p"
            );
            let (bindings, registry) = analyse(&source);
            let offset = u32::try_from(source.find("my Q").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("my", offset);
            let entry = binding.receiver_self_method_entry(&registry);
            assert_eq!(entry.is_some(), reached, "{source}");
            if let Some((receiver, entry, _)) = entry {
                assert_eq!(receiver.command, "::C");
                assert_eq!(entry.name(), "Q");
                assert_eq!(entry.body().text.try_text().unwrap(), "return PRIVATE");
            }
        }
        let source = "oo::class create C {method Q {} {return PRIVATE}; method p {} {my Q}}";
        let (bindings, registry) = analyse(source);
        let offset = u32::try_from(source.find("my Q").unwrap()).unwrap();
        assert!(
            bindings
                .invocation_at_source("my", offset)
                .receiver_self_method_entry(&registry)
                .is_none()
        );
    }

    #[test]
    fn native_self_result_keeps_retained_class_and_actual_helper_namespace() {
        use tcl_core_types::{ByteNamespacePath, NativeByteCommandSlot};
        use tcl_runtime_api::native_compilation::{
            NativeCommandImplementation, NativeCompilerHookPresence,
        };
        let mut entry = crate::command_binding::named_invocation::tests::native_entry();
        for (token, path) in [
            (8, ByteNamespacePath::from_segments(["oo"])),
            (9, ByteNamespacePath::from_segments(["oo", "Helpers"])),
        ] {
            let mut namespace = entry.namespaces[0].clone();
            namespace.token = token;
            namespace.path = path;
            entry.namespaces.push(namespace);
        }
        for (token, namespace, path, simple, identity) in [
            (
                10,
                8,
                ByteNamespacePath::from_segments(["oo"]),
                "class",
                "oo::class",
            ),
            (
                11,
                9,
                ByteNamespacePath::from_segments(["oo", "Helpers"]),
                "self",
                "::oo::Helpers::self",
            ),
            (12, 0, ByteNamespacePath::root(), "return", "return"),
        ] {
            let mut row = entry.commands[1].clone();
            row.token = token;
            row.namespace_token = namespace;
            row.slot = NativeByteCommandSlot::new(path, simple.into());
            row.implementation = NativeCommandImplementation::Registry {
                identity: identity.into(),
                compiler_hook: false,
            };
            row.compiler_hook = NativeCompilerHookPresence::Absent;
            row.compiler = None;
            entry.commands.push(row);
        }
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let source = "oo::class create C {method ping {} {return ORIGINAL}; method p {} {[self] ping}}; C create rex; rex p";
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_entry: Some(&entry),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..SourceAnalysisOptions::default()
            },
        );
        let binding = bindings.invocation_at_source(
            "[self]",
            u32::try_from(source.find("[self] ping").unwrap()).unwrap(),
        );
        let frozen = binding
            .frozen_head_object
            .as_ref()
            .expect("same actual native receiver, without a reported-name lookup");
        let state = &binding.lookup_state.as_ref().unwrap().state;
        let namespace = binding.variable_frame.namespace_identity().unwrap();
        assert!(matches!(
            binding.variable_frame.layout(),
            VariableExecutionFrame::ReceiverMethod { .. }
        ));
        let helper = state.source_keys_checked("self", namespace).unwrap();
        assert_eq!(helper.len(), 1);
        assert_eq!(helper[0].holder().native_context().unwrap().token, 9);
        assert_eq!(helper[0].simple_utf8(), Some("self"));
        // A genuine global callback on the same live receiver must not borrow
        // its method-local Helpers path.
        let root = state.source_root_namespace_key().unwrap();
        assert!(state.receiver_native_lookup_paths("self", &root).is_none());
        assert!(
            binding
                .frozen_object_receiver_method_entry(&frozen.word)
                .is_some()
        );
        assert!(binding.evaluated_command_word().is_none());
    }

    #[test]
    fn self_result_freezes_the_actual_receiver_without_inventing_name_bytes() {
        for (mutation, reached) in [
            ("", true),
            (" [oo::objdefine rex class B]", false),
            (" [rename rex moved]", false),
        ] {
            let source = format!(
                "oo::class create C {{method ping {{args}} {{return ORIGINAL}}; method p {{}} {{[self] ping{mutation}}}}}; oo::class create B {{}}; C create rex; rex p"
            );
            let (bindings, _) = analyse(&source);
            let offset = u32::try_from(source.find("[self] ping").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("[self]", offset);
            let frozen = binding
                .frozen_head_object
                .as_ref()
                .expect("actual self head result");
            assert_eq!(
                binding
                    .frozen_object_receiver_method_entry(&frozen.word)
                    .is_some(),
                reached,
                "{source}"
            );
            assert!(
                binding.evaluated_command_word().is_none(),
                "no invented object name"
            );
            let hypothetical = binding.lookup_command_word("puts");
            assert!(hypothetical.frozen_head_object.is_none());
        }
        let source = "oo::class create C {method ping {} {}; method p {} {[self] ping}}";
        let (bindings, _) = analyse(source);
        let offset = u32::try_from(source.find("[self] ping").unwrap()).unwrap();
        assert!(
            bindings
                .invocation_at_source("[self]", offset)
                .frozen_head_object
                .is_none()
        );
    }

    #[test]
    fn actual_builtin_links_use_allocated_storage_and_survive_method_return() {
        for receiver in ["first", "second"] {
            let source = format!(
                "oo::class create C {{method put {{}} {{my variable x; set x 7}}; method get {{}} {{my variable x; set x}}}}; C create first; C create second; {receiver} put; set answer [{receiver} get]; puts $answer"
            );
            let (bindings, registry) = analyse(&source);
            let offset = u32::try_from(source.rfind("$answer").unwrap()).unwrap();
            let point = bindings
                .points
                .iter()
                .find(|point| point.dispatch && point.offset == offset.saturating_sub(5))
                .expect("puts dispatch");
            assert_eq!(
                point
                    .state
                    .source_variables
                    .literal_value("answer", &registry),
                Some("7"),
                "{source}"
            );
        }
    }

    #[test]
    fn private_dispatcher_replacement_is_distinct_from_class_dispatch() {
        for mutation in [
            "proc ::oo::Obj12::my args {return REPLACED}; ",
            "namespace eval [info object namespace rex] {proc my args {return REPLACED}}; ",
        ] {
            let source = format!(
                "oo::class create C {{method Q {{}} {{return ORIGINAL}}; method p {{}} {{my Q}}}}; C create rex; {mutation}rex p"
            );
            let (bindings, registry) = analyse(&source);
            let offset = u32::try_from(source.find("my Q").unwrap()).unwrap();
            assert!(
                bindings
                    .invocation_at_source("my", offset)
                    .receiver_self_method_entry(&registry)
                    .is_none(),
                "{source}"
            );
        }
    }
}
