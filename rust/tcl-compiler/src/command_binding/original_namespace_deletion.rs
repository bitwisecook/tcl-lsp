// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Same-operation namespace retirement and independently closed fresh cleanup.

use super::{
    Arc, BTreeSet, BindingKind, CommandAllocationSite, MayBinding, ModuleCommandBindings,
    SourceCommandKey, SourceExecutionContext, SourceNamespaceKey, SourceNativeInvocation,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::{
    InvocationFacts, NamespaceTransition, NamespaceTransitionTarget, StateTransition,
    TransitionSubject,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OriginalNamespaceDeleteTarget {
    subject: TransitionSubject,
    input: SignatureSourceNameInput,
    namespace: SourceNamespaceKey,
}

/// Targets retained before the genuine command transfer. These do not prove
/// successful completion, cleanup closure or native execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalNamespaceDeletionTransfer {
    site: CommandAllocationSite,
    targets: Vec<OriginalNamespaceDeleteTarget>,
}

/// Exact namespaces already retired by the same canonical command operation.
/// Only the post-transfer check below issues this handoff to the variable owner.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AppliedOriginalNamespaceDeletions(OriginalNamespaceDeletionTransfer);

impl OriginalNamespaceDeletionTransfer {
    // Implementation contract: naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        let current = context.namespace_identity();
        let mut targets = Vec::new();
        for fact in facts.state_transitions.declared()?.facts() {
            let StateTransition::Namespace(NamespaceTransition::Delete {
                namespace: NamespaceTransitionTarget::Named(subject),
            }) = &fact.transition
            else {
                continue;
            };
            let input =
                super::original_command_table::original_operand(native, subject, state, context)?;
            let namespace = state.original_namespace_key_for_input(&current, &input)?;
            targets.push(OriginalNamespaceDeleteTarget {
                subject: subject.clone(),
                input,
                namespace,
            });
        }
        if targets.is_empty() {
            return None;
        }
        Some(Self {
            site: CommandAllocationSite {
                source: Arc::clone(state.current_source_origin.as_ref()?),
                offset: native.segment.span.start(),
            },
            targets,
        })
    }

    // Implementation contract: naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    pub(super) fn after_command_transfer(
        self,
        state: &ModuleCommandBindings,
    ) -> Option<AppliedOriginalNamespaceDeletions> {
        if state.has_opaque_domain()
            || state.current_source_origin.as_ref() != Some(&self.site.source)
            || self.targets.iter().any(|target| {
                state.namespaces.contains(&target.namespace)
                    || state
                        .source_variables
                        .namespace_addressable_identities
                        .contains(&target.namespace)
                    || !target.input.is_current(&state.source_variables)
            })
        {
            return None;
        }
        Some(AppliedOriginalNamespaceDeletions(self))
    }
}

impl AppliedOriginalNamespaceDeletions {
    // Implementation contract: naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    pub(crate) fn matches(
        &self,
        subject: &TransitionSubject,
        operands: Option<&crate::variable_bindings::OriginalVariableInvocation>,
        state: &crate::var_resolve::ResolveContext,
        offset: u32,
    ) -> bool {
        self.0.site.offset == offset
            && self.0.targets.iter().any(|target| {
                target.subject == *subject
                    && !state
                        .namespace_addressable_identities
                        .contains(&target.namespace)
                    && subject
                        .argument_index()
                        .and_then(|index| operands?.input(index, state))
                        == Some(&target.input)
            })
    }
}

/// A closed source handler envelope for a fresh namespace with no variable
/// cells and only intrinsic source-procedure cleanup. Source lineage alone is
/// insufficient: current tables, observers, frame lifetime and cleanup are
/// checked separately before and after the actual common transfer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OriginalNamespaceDeletion {
    site: CommandAllocationSite,
    words: Vec<crate::ir::WordExpr>,
    config: tcl_lexer::LexerConfig,
    handler: super::SourceCommandTarget,
    current: SourceNamespaceKey,
    targets: Vec<OriginalNamespaceDeleteTarget>,
    namespaces: BTreeSet<SourceNamespaceKey>,
    commands: Vec<DeletedCommand>,
}

fn quiet(state: &ModuleCommandBindings) -> bool {
    !state.has_opaque_domain()
        && state.baseline.native_entry.is_none()
        && !state.baseline.unknown_entry
        && !state.source_step_observed()
        && !state.source_execution_observed(None)
        && state.command_observers.is_quiet()
}

fn variables_are_empty(
    state: &crate::var_resolve::ResolveContext,
    namespaces: &BTreeSet<SourceNamespaceKey>,
) -> bool {
    let mut frame = Some(state);
    while let Some(state) = frame {
        if !state.namespace_cells.closed
            || state.dynamic_bindings
            || state.dynamic_traces
            || state
                .namespace_identity
                .as_ref()
                .is_some_and(|current| namespaces.contains(current))
            || state
                .pending_namespace_retirements
                .keys()
                .any(|key| namespaces.contains(key))
            || state
                .namespace_cells
                .present
                .iter()
                .chain(&state.namespace_cells.possible)
                .any(|key| {
                    namespaces
                        .iter()
                        .any(|namespace| key.is_in_namespace(namespace))
                })
            || state
                .traced
                .iter()
                .chain(&state.untracked_traces)
                .any(|key| {
                    namespaces
                        .iter()
                        .any(|namespace| key.is_in_namespace(namespace))
                })
            || !state.possible_trace_registrations.is_empty()
            || state.trace_registrations.keys().any(|key| {
                namespaces
                    .iter()
                    .any(|namespace| key.is_in_namespace(namespace))
            })
        {
            return false;
        }
        frame = state.caller.as_deref();
    }
    true
}

type DeletedCommand = (SourceCommandKey, super::CommandIdentity);

fn fresh_namespace_tree(
    state: &ModuleCommandBindings,
    transfer: &OriginalNamespaceDeletionTransfer,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<BTreeSet<SourceNamespaceKey>> {
    // naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    let root = state.source_root_namespace_key()?;
    let namespaces = state
        .namespaces
        .iter()
        .filter(|namespace| {
            transfer.targets.iter().any(|target| {
                *namespace == &target.namespace || namespace.is_descendant_of(&target.namespace)
            })
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    if namespaces.is_empty()
        || namespaces.contains(&root)
        || !variables_are_empty(&state.source_variables, &namespaces)
    {
        return None;
    }
    for namespace in &namespaces {
        let creation = state
            .original_command_world
            .source_namespace_creation(namespace, policy);
        let creation = creation?;
        if creation.source != transfer.site.source
            || creation.offset >= transfer.site.offset
            || state.unknown_lookup_namespaces.contains(namespace)
            || state.unknown_namespace_paths.contains(namespace)
            || state.namespace_unknown_handlers.contains(namespace)
            || matches!(
                namespace,
                SourceNamespaceKey::Allocated {
                    incarnation: super::AllocationIncarnation::RepeatedFresh,
                    ..
                }
            )
        {
            return None;
        }
    }
    Some(namespaces)
}

fn intrinsic_namespace_commands(
    state: &ModuleCommandBindings,
    namespaces: &BTreeSet<SourceNamespaceKey>,
    site: &CommandAllocationSite,
) -> Option<Vec<DeletedCommand>> {
    // naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    let mut commands = Vec::new();
    for (key, alternatives) in state.bindings.iter() {
        if !namespaces.contains(key.holder().as_ref()) {
            continue;
        }
        if alternatives == &BTreeSet::from([MayBinding::Missing]) {
            continue;
        }
        if alternatives.len() != 1 {
            return None;
        }
        let MayBinding::Target(target) = alternatives.first()? else {
            return None;
        };
        let identity = target.token.as_ref()?;
        let allocation = target.implementation_allocation.as_ref()?;
        if target.kind != BindingKind::Proc
            || !target.terminal
            || target.registry_backed
            || !target.prepended.is_empty()
            || target.implementation_generation != allocation.site.offset
            || identity.declaration != target.implementation_generation
            || state
                .runtime_implementation_generation(target.token.as_ref())
                .is_some()
            || identity.runtime.is_some()
            || identity.allocation.as_ref() != Some(allocation)
            || allocation.incarnation == super::AllocationIncarnation::RepeatedFresh
            || allocation.site.source != site.source
            || state
                .objects
                .get(identity)
                .is_some_and(|objects| objects != alternatives)
        {
            return None;
        }
        commands.push((key.clone(), identity.clone()));
    }
    Some(commands)
}

impl OriginalNamespaceDeletion {
    // Implementation contract: naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        if !quiet(state)
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || !facts.arg_roles_complete
            || facts.effects.callback().kinds != tcl_registry::world_effect::CallbackKinds::TRACE
            || facts.body_execution.is_some()
            || facts
                .effects
                .accesses()
                .iter()
                .any(|access| access.mode != tcl_registry::world_effect::EffectAccessMode::Read)
            || !native.target.registry_backed
            || native.target.kind != BindingKind::Builtin
            || native.target.implementation_generation != 0
            || native.target.runtime_implementation_generation.is_some()
            || !native.target.prepended.is_empty()
            || !state.retained_target_is_current(native.target)
        {
            return None;
        }
        let transfer = OriginalNamespaceDeletionTransfer::capture(native, facts, state, context)?;
        let policy = state.baseline.execution_name_policy?.native_recipe()?;
        let tcl_syntax::naming::NativeNameProtocol::C(version) = policy.recipe() else {
            return None;
        };
        let spec = context.registry.native_registration_source_descriptor(
            native.target.registry_identity()?,
            tcl_registry::InvocationDialect::for_version(version),
            context.realm,
        )?;
        if tcl_registry::registry::spec_pack_of(spec) != Some("tcl") {
            return None;
        }
        if facts
            .state_transitions
            .declared()?
            .facts()
            .iter()
            .any(|fact| {
                !matches!(
                    &fact.transition,
                    StateTransition::Namespace(NamespaceTransition::Delete {
                        namespace: NamespaceTransitionTarget::Named(_)
                    })
                )
            })
        {
            return None;
        }
        if transfer.targets.iter().any(|target| {
            target.input.policy() != policy
                || target.input.original_word_key().is_none()
                || !facts.arg_roles.iter().any(|(index, role)| {
                    *role == tcl_registry::ArgRole::NamespaceName
                        && facts.argument_offset.checked_add(usize::from(*index))
                            == target.subject.argument_index()
                })
        }) {
            return None;
        }
        // Deleting overlapping operands would make a later lookup fail after
        // an earlier operand has retired its tree.
        for (index, target) in transfer.targets.iter().enumerate() {
            if transfer.targets[..index].iter().any(|prior| {
                target.namespace == prior.namespace
                    || target.namespace.is_descendant_of(&prior.namespace)
                    || prior.namespace.is_descendant_of(&target.namespace)
            }) {
                return None;
            }
        }
        let current = context.namespace_identity();
        let namespaces = fresh_namespace_tree(state, &transfer, policy)?;
        let commands = intrinsic_namespace_commands(state, &namespaces, &transfer.site)?;
        Some(Self {
            site: transfer.site,
            words: native.words.to_vec(),
            config: context.config,
            handler: native.target.clone(),
            current,
            targets: transfer.targets,
            namespaces,
            commands,
        })
    }

    // Implementation contract: naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    pub(super) fn completed(
        &self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        quiet(state)
            && context.config == self.config
            && native.words == self.words
            && native.target == &self.handler
            && state.retained_target_is_current(&self.handler)
            && native.segment.span.start() == self.site.offset
            && state.current_source_origin.as_ref() == Some(&self.site.source)
            && context.namespace_identity() == self.current
            && facts.effects.callback().kinds == tcl_registry::world_effect::CallbackKinds::TRACE
            && variables_are_empty(&state.source_variables, &self.namespaces)
            && self
                .targets
                .iter()
                .all(|target| target.input.is_current(&state.source_variables))
            && self
                .namespaces
                .iter()
                .all(|namespace| !state.namespaces.contains(namespace))
            && self.commands.iter().all(|(key, identity)| {
                state.bindings.get(key) == Some(&BTreeSet::from([MayBinding::Missing]))
                    && !state.objects.contains_key(identity)
            })
    }
}

#[cfg(test)]
mod tests {
    fn analyse(
        source: &str,
        version: tcl_dialect::TclVersion,
    ) -> super::super::SourceCommandBindings {
        let owner = tcl_registry::model::ingress::static_context_for(version.dialect_name());
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        super::super::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            owner.commands(),
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        )
    }

    #[test]
    // Implementation contract: naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    fn original_fresh_namespace_delete_closes_only_empty_cells_and_intrinsic_procedures() {
        // naming.namespace.original-fresh-namespace-deletion
        // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "namespace eval N {proc p {} {}}; namespace delete N; namespace eval N {}",
                r"namespace eval N\uD800 {proc p\uD801 {} {}}; namespace delete N\uD800; namespace eval N\uD800 {}",
                "namespace eval A {proc p {} {}}; namespace eval B {proc q {} {}}; namespace delete A B",
            ] {
                let bindings = analyse(source, version);
                let world = bindings
                    .original_completed_command_world()
                    .unwrap_or_else(|| panic!("{}: {source}", version.dialect_name()));
                assert_eq!(
                    world.declarations().count(),
                    0,
                    "{}: {source}",
                    version.dialect_name()
                );
            }
        }
    }

    #[test]
    // Implementation contract: naming.namespace.original-fresh-namespace-deletion
    // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
    fn original_namespace_delete_refuses_missing_unknown_and_overlapping_targets() {
        // naming.namespace.original-fresh-namespace-deletion
        // docs/design/analysis/name-resolution-proofs/namespace-original-fresh-deletion.md
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "namespace eval N {proc p {} {}}; namespace delete M",
                "namespace eval N {proc p {} {}}; namespace delete $unknown",
                "namespace eval N {proc p {} {}}; namespace delete N N",
                "namespace eval N {namespace eval child {proc p {} {}}}; namespace delete N ::N::child",
                "namespace eval N {set value DATA}; namespace delete N",
            ] {
                assert!(
                    analyse(source, version)
                        .original_completed_command_world()
                        .is_none(),
                    "{}: {source}",
                    version.dialect_name()
                );
            }
        }
    }
}
