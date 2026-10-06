// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual bounded object allocations, distinct from nominal type labels.

use super::{
    AllocationIncarnation, Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandTarget,
    SourceExecutionContext, SourceInvocationBinding, SourceOutcomes, SourceVariableAccess,
};
use std::collections::BTreeMap;

/// One reached object allocation in its actual caller activation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceObjectAllocation {
    /// Exact source instruction that invoked the manufacturer.
    pub site: CommandAllocationSite,
    /// Frame in which that invocation was evaluated.
    pub frame: crate::var_resolve::VariableExecutionFrame,
    /// Bounded execution incarnation; repeated families supply no unique proof.
    pub incarnation: AllocationIncarnation,
}

/// A reached object allocation whose current dispatch is closed. Constructor
/// completion is independent; only a successful manufacture publishes a result.
/// A class label or unchanged variable version cannot create this witness.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceObjectInstanceProof {
    allocation: SourceObjectAllocation,
    class: SourceCommandTarget,
    dispatch_generation: u64,
    pub(super) receiver_dispatcher_generation: Option<u64>,
    pub(super) receiver_namespace_path: Option<&'static [&'static str]>,
    instance_methods: Option<Arc<super::BTreeSet<String>>>,
}

impl SourceObjectInstanceProof {
    /// Exact bounded instance allocation, independent of its rendered name.
    #[must_use]
    pub fn allocation(&self) -> &SourceObjectAllocation {
        &self.allocation
    }

    /// Live class command and its implementation allocation at construction.
    #[must_use]
    pub fn class_target(&self) -> &SourceCommandTarget {
        &self.class
    }

    /// Complete instance-method names retained at this dispatch generation.
    /// This inventory includes private names and proves only that an absent
    /// name cannot be supplied by this closed class definition. It does not
    /// license method execution, visibility, arity or constructor effects.
    #[must_use]
    pub fn instance_method_names(&self) -> Option<&super::BTreeSet<String>> {
        self.instance_methods.as_deref()
    }

    /// Current object-dispatch generation retained at the actual read.
    #[must_use]
    pub fn dispatch_generation(&self) -> u64 {
        self.dispatch_generation
    }
}

impl SourceInvocationBinding {
    /// Possible manufacturers from exact retained class allocations and their
    /// original declaration grammar. Unknown lookup and absence remain in this
    /// binding; candidates grant no constructor completion or instance proof.
    #[must_use]
    pub fn class_factory_candidates(
        &self,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<&SourceCommandTarget> {
        let Some(snapshot) = self.lookup_state.as_ref() else {
            return Vec::new();
        };
        let state = &snapshot.state;
        self.targets
            .iter()
            .filter(|target| {
                let Some(identity) = target.identity.as_ref() else {
                    return false;
                };
                let Some(definition) = state.class_definitions.get(identity) else {
                    return false;
                };
                if target.kind != super::BindingKind::Class
                    || definition.dispatcher.is_some()
                    || target.implementation_generation != definition.implementation_generation
                    || state.tainted_object_dispatch.contains("*")
                    || state.tainted_object_dispatch.contains(&target.command)
                    || !state.class_definition_dependencies_hold(definition)
                {
                    return false;
                }
                let Some(grammar) = state
                    .baseline
                    .dialect
                    .zip(self.invocation_realm())
                    .and_then(|(dialect, realm)| {
                        registry.native_default_construction_grammar(
                            &definition.factory,
                            dialect,
                            realm,
                        )
                    })
                else {
                    return false;
                };
                if !state.default_construction_dependencies_hold(grammar) {
                    return false;
                }
                let member = match target.prepended.first() {
                    Some(word) => word.as_registry_word().literal(),
                    None => self
                        .evaluated_argument_values
                        .first()
                        .and_then(Option::as_deref),
                };
                member.is_some_and(|member| grammar.manufacturer(member).is_some())
            })
            .collect()
    }

    /// Original declared method source of an actual head-word instance read,
    /// retained through this invocation's argv. This identifies a declaration
    /// for navigation only; private visibility and call completion stay separate.
    #[must_use]
    pub fn object_receiver_method_entry<'a>(
        &'a self,
        access: &'a SourceVariableAccess,
        head: &crate::ir::WordExpr,
    ) -> Option<(
        &'a SourceCommandTarget,
        &'a super::SourceReceiverMethodEntry,
        u64,
    )> {
        if !self.retains_object_instance_at_dispatch(access)
            || head.sole_variable_substitution()?.1 != &access.source
            || head.source().span.start() != self.dispatch_site.as_ref()?.offset
        {
            return None;
        }
        let proof = access.proved_object_instance()?;
        let method = self.evaluated_argument_values.first()?.as_ref()?;
        let definition = self
            .lookup_state
            .as_ref()?
            .state
            .class_definitions
            .get(proof.class_target().identity.as_ref()?)?;
        if !self
            .lookup_state
            .as_ref()?
            .state
            .class_definition_dependencies_hold(definition)
        {
            return None;
        }
        let entry = definition
            .receiver_method_entries
            .get(&(super::SourceMethodReceiver::Instance, method.clone()))?;
        Some((proof.class_target(), entry, proof.dispatch_generation()))
    }

    /// Actual named instance token selected after argv, retaining its bounded
    /// manufacturer allocation and current object-dispatch generation.
    #[must_use]
    pub fn named_object_instance_at_dispatch(&self) -> Option<&SourceObjectInstanceProof> {
        let target = self.proved_target()?;
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        if self.runtime_reachability() != super::SourceRuntimeReachability::Reached
            || self.entered_execution_observer.observed()
            || !target.prepended.is_empty()
            || state.opaque_domain
            || state.source_step_observed()
            || state.source_execution_observed(target.identity.as_ref())
        {
            return None;
        }
        let identity = target.identity.as_ref()?;
        let proof = state.object_instances.named.get(identity)?;
        if identity.allocation.as_ref().is_none_or(|allocation| {
            allocation.site != proof.allocation.site
                || allocation.incarnation != proof.allocation.incarnation
        }) || state.object_instances.generation != Some(proof.dispatch_generation)
            || !state.retained_target_is_current(&proof.class)
        {
            return None;
        }
        Some(proof)
    }

    /// Original method declaration of the exact named object command selected
    /// after argv. Prefix aliases and mutable receiver worlds supply no receipt.
    #[must_use]
    pub fn named_object_receiver_method_entry(
        &self,
    ) -> Option<(&SourceCommandTarget, &super::SourceReceiverMethodEntry, u64)> {
        let proof = self.named_object_instance_at_dispatch()?;
        let method = self.evaluated_argument_values.first()?.as_ref()?;
        let definition = self
            .lookup_state
            .as_ref()?
            .state
            .class_definitions
            .get(proof.class.identity.as_ref()?)?;
        if !self
            .lookup_state
            .as_ref()?
            .state
            .class_definition_dependencies_hold(definition)
        {
            return None;
        }
        let entry = definition
            .receiver_method_entries
            .get(&(super::SourceMethodReceiver::Instance, method.clone()))?;
        Some((&proof.class, entry, proof.dispatch_generation))
    }

    /// Method declaration inventory of the exact live class selected after
    /// argv. Consumers must require the intended receiver kind and original
    /// source/frame; this is not a visibility, arity or method execution proof.
    #[must_use]
    pub fn class_definition_method_entries(
        &self,
    ) -> Option<(&SourceCommandTarget, &super::SourceReceiverMethodEntries)> {
        let (_, target) = self.proved_class_definition_factory()?;
        let definition = self
            .lookup_state
            .as_ref()?
            .state
            .class_definitions
            .get(target.identity.as_ref()?)?;
        Some((target, &definition.receiver_method_entries))
    }

    /// Validate an actual frozen argument's instance receipt at the later
    /// dispatch point. Argument callbacks cannot borrow the pre-read class.
    #[must_use]
    pub fn retains_object_instance_at_dispatch(&self, access: &SourceVariableAccess) -> bool {
        let Some(site) = &self.dispatch_site else {
            return false;
        };
        let Some(proof) = access.proved_object_instance() else {
            return false;
        };
        let Some(snapshot) = &self.lookup_state else {
            return false;
        };
        access.owner.is_argument_evaluation_of(site)
            && !snapshot.state.opaque_domain
            && snapshot.state.object_instances.generation == Some(proof.dispatch_generation)
            && snapshot.state.retained_target_is_current(&proof.class)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ObjectStore {
    receiver: crate::place::Place,
    origin: crate::var_resolve::ContentsOrigin,
    source: Option<Arc<super::SourceOriginId>>,
    proof: Arc<SourceObjectInstanceProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SourceObjectState {
    pub(super) generation: Option<u64>,
    pub(super) class_delegate_generation: Option<u64>,
    pub(super) receiver_dispatcher_generation: Option<u64>,
    allocations: Vec<(
        CommandAllocationSite,
        crate::var_resolve::VariableExecutionFrame,
        u8,
    )>,
    stores: crate::var_resolve::VariableCellTable<ObjectStore>,
    pub(super) prefix_stores:
        crate::var_resolve::VariableCellTable<super::method_prefix::MethodPrefixStore>,
    pub(super) named: BTreeMap<super::CommandIdentity, Arc<SourceObjectInstanceProof>>,
    pub(super) receivers: BTreeMap<String, Arc<SourceObjectInstanceProof>>,
}

impl Default for SourceObjectState {
    fn default() -> Self {
        Self {
            generation: Some(0),
            class_delegate_generation: Some(0),
            receiver_dispatcher_generation: Some(0),
            allocations: Vec::new(),
            stores: crate::var_resolve::VariableCellTable::default(),
            prefix_stores: crate::var_resolve::VariableCellTable::default(),
            named: BTreeMap::new(),
            receivers: BTreeMap::new(),
        }
    }
}

impl SourceObjectState {
    pub(super) fn invalidate_class_delegates(&mut self, opaque: bool) {
        self.class_delegate_generation = if opaque {
            None
        } else {
            self.class_delegate_generation
                .and_then(|epoch| epoch.checked_add(1))
        };
    }

    pub(super) fn invalidate_receiver_dispatcher(&mut self) {
        self.receiver_dispatcher_generation = self
            .receiver_dispatcher_generation
            .and_then(|generation| generation.checked_add(1));
        self.receivers.clear();
    }

    pub(super) fn invalidate_dispatch(&mut self, opaque: bool) {
        if opaque {
            self.invalidate_class_delegates(true);
            self.receiver_dispatcher_generation = None;
        }
        self.invalidate_receiver_dispatcher();
        self.generation = if opaque {
            None
        } else {
            self.generation.and_then(|n| n.checked_add(1))
        };
        self.stores.clear();
        self.prefix_stores.clear();
        self.named.clear();
        self.receivers.clear();
    }

    pub(super) fn invalidate_writes(&mut self, writes: &[crate::place::Place]) {
        self.prefix_stores.retain(|_, store| {
            !writes.iter().any(|write| {
                write.dynamic
                    || write.observed
                    || write.kind == crate::place::PlaceKind::Unknown
                    || (write.cell == store.receiver.cell
                        && (write.index.is_none() || write.index == store.receiver.index))
            })
        });
        self.stores.retain(|_, store| {
            !writes.iter().any(|write| {
                write.dynamic
                    || write.observed
                    || write.kind == crate::place::PlaceKind::Unknown
                    || (write.cell == store.receiver.cell
                        && (write.index.is_none() || write.index == store.receiver.index))
            })
        });
    }

    pub(super) fn join(&mut self, other: &Self) {
        if self.class_delegate_generation != other.class_delegate_generation {
            self.class_delegate_generation = None;
        }
        if self.receiver_dispatcher_generation != other.receiver_dispatcher_generation {
            self.receiver_dispatcher_generation = None;
        }
        if self.generation != other.generation {
            self.generation = None;
        }
        self.stores
            .retain(|key, value| other.stores.get(key) == Some(value));
        self.prefix_stores
            .retain(|key, value| other.prefix_stores.get(key) == Some(value));
        self.named
            .retain(|key, value| other.named.get(key) == Some(value));
        self.receivers
            .retain(|key, value| other.receivers.get(key) == Some(value));
        for (site, frame, count) in &other.allocations {
            if let Some((_, _, current)) = self
                .allocations
                .iter_mut()
                .find(|(s, f, _)| s == site && f == frame)
            {
                *current = (*current).max(*count);
            } else {
                self.allocations.push((site.clone(), frame.clone(), *count));
            }
        }
    }
}

impl ModuleCommandBindings {
    pub(super) fn constructed_command_name(
        &self,
        proof: &SourceObjectInstanceProof,
    ) -> Option<String> {
        let mut slots = self.bindings.iter().filter_map(|(slot, bindings)| {
            let mut bindings = bindings.iter();
            let Some(super::MayBinding::Target(target)) = bindings.next() else {
                return None;
            };
            if bindings.next().is_some() || !target.prepended.is_empty() {
                return None;
            }
            let receiver = self.object_instances.named.get(target.token.as_ref()?)?;
            (receiver.allocation() == proof.allocation())
                .then(|| self.callable_spelling_for_key(slot))
                .flatten()
        });
        let original = slots.next()?;
        slots.next().is_none().then_some(original)
    }

    pub(super) fn remove_constructed_command(&mut self, proof: &SourceObjectInstanceProof) {
        let identities = self
            .object_instances
            .named
            .iter()
            .filter(|(_, instance)| instance.allocation() == proof.allocation())
            .map(|(identity, _)| identity.clone())
            .collect::<super::BTreeSet<_>>();
        let slots = self
            .bindings
            .iter()
            .filter(|&(_, targets)| {
                targets.iter().any(|target| {
                    matches!(target, super::MayBinding::Target(target)
                if target.token.as_ref().is_some_and(|token| identities.contains(token)))
                })
            })
            .map(|(slot, _)| slot.clone())
            .collect::<Vec<_>>();
        let dispatcher = self.object_instances.receiver_dispatcher_generation;
        let receivers = self.object_instances.receivers.clone();
        for slot in slots {
            self.remove(slot);
        }
        // Deleting this independently allocated command does not replace another
        // live object's private dispatcher or the active destructor's receiver.
        let objects = Arc::make_mut(&mut self.object_instances);
        objects.receiver_dispatcher_generation = dispatcher;
        objects.receivers = receivers;
    }

    pub(super) fn retire_constructed_allocation(
        &mut self,
        proof: &SourceObjectInstanceProof,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let allocation = proof.allocation();
        let callbacks = self
            .source_variables
            .contents_presence_slots
            .values()
            .any(|place| {
                if !matches!(place.cell.as_ref().map(|cell| &cell.owner),
                Some(crate::place::CellOwner::AllocatedInstance(owner)) if **owner == *allocation)
                {
                    return false;
                }
                let selected = self.source_variables.variable_observers_at(
                    place,
                    tcl_registry::TraceOperation::Unset,
                    registry,
                );
                selected.unknown_residual
                    || !selected.callbacks.is_empty()
                    || !selected.possible_callbacks.is_empty()
            });
        if callbacks {
            // Object namespace teardown has an unresolved native cell/callback
            // order. It cannot contribute a successful callback effect summary.
            self.mark_opaque_binding_mutation();
        }
        Arc::make_mut(&mut self.source_variables).retire_allocated_instance_variables(allocation);
        let objects = Arc::make_mut(&mut self.object_instances);
        objects
            .stores
            .retain(|_, store| store.proof.allocation() != allocation);
        objects
            .prefix_stores
            .retain(|_, store| store.prefix.receiver().allocation() != allocation);
        objects
            .named
            .retain(|_, receiver| receiver.allocation() != allocation);
        objects
            .receivers
            .retain(|_, receiver| receiver.allocation() != allocation);
    }
    pub(super) fn object_read_proof(
        &self,
        place: &crate::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<SourceObjectInstanceProof>> {
        if place.dynamic || place.observed || self.opaque_domain {
            return None;
        }
        let key = crate::var_resolve::canonical_binding_value_key(place)?;
        let store = self.object_instances.stores.get(&key)?;
        if &store.receiver != place
            || self.source_variables.read_contents_origin(place, registry) != store.origin
            || store
                .source
                .as_ref()
                .is_some_and(|source| !self.source_variables.contents_have_source(place, source))
            || self.source_variables.contents_presence(place)
                != crate::var_resolve::ContentsPresence::Defined
            || self.object_instances.generation != Some(store.proof.dispatch_generation)
            || !self.retained_target_is_current(&store.proof.class)
        {
            return None;
        }
        Some(Arc::clone(&store.proof))
    }

    pub(super) fn retain_named_manufacture(
        &mut self,
        proof: &mut Arc<SourceObjectInstanceProof>,
        arguments: &[crate::registry_invocation::EffectiveInvocationWord],
        context: SourceExecutionContext<'_>,
    ) {
        let Some(grammar) = proof
            .class
            .identity
            .as_ref()
            .and_then(|identity| self.class_definitions.get(identity))
            .and_then(|definition| {
                context.registry.native_default_construction_grammar(
                    &definition.factory,
                    self.baseline.dialect?,
                    context.realm,
                )
            })
        else {
            return;
        };
        let Some(name_at) = arguments
            .first()
            .and_then(|word| word.as_registry_word().literal())
            .and_then(|name| grammar.manufacturer(name))
            .and_then(|member| member.names_instance_at)
        else {
            return;
        };
        let Some(name) = arguments
            .get(usize::from(name_at))
            .and_then(|word| word.as_registry_word().literal())
        else {
            return;
        };
        if !self.source_variables.namespace_known || self.opaque_domain {
            return;
        }
        let slot = super::qualify_execution_name(
            &crate::ir_helpers::ExecutionNamespace::exact(context.namespace),
            name,
        );
        let Some(slot) = slot else {
            return;
        };
        let Some(target) = self.closed_installed_instance(&slot) else {
            return;
        };
        let allocation = super::CommandAllocation {
            site: proof.allocation.site.clone(),
            incarnation: proof.allocation.incarnation,
            command: slot.clone(),
            namespace: context.namespace_identity(),
        };
        let identity = super::CommandIdentity {
            runtime: None,
            origin: slot.clone(),
            declaration: proof.allocation.site.offset,
            allocation: Some(allocation.clone()),
        };
        let previous = target.token.clone();
        let mut installed = target;
        installed.token = Some(identity.clone());
        installed.implementation_generation = proof.allocation.site.offset;
        installed.implementation_allocation = Some(allocation);
        if let Some(previous) = previous {
            Arc::make_mut(&mut self.objects).remove(&previous);
        }
        let implementation = super::MayBinding::Target(installed);
        Arc::make_mut(&mut self.objects).insert(
            identity.clone(),
            super::BTreeSet::from([implementation.clone()]),
        );
        let dispatcher_generation = self.object_instances.receiver_dispatcher_generation;
        let receivers = self.object_instances.receivers.clone();
        self.replace(slot, super::BTreeSet::from([implementation]));
        // Restamping an already installed allocation is not an interpreter
        // mutation, so it cannot retire other objects' private dispatchers.
        let objects = Arc::make_mut(&mut self.object_instances);
        objects.receiver_dispatcher_generation = dispatcher_generation;
        objects.receivers = receivers;
        Arc::make_mut(proof).receiver_dispatcher_generation = dispatcher_generation;
        Arc::make_mut(&mut self.object_instances)
            .named
            .insert(identity, Arc::clone(proof));
    }

    fn closed_installed_instance(&self, slot: &str) -> Option<super::ResolvedCommandTarget> {
        let bindings = self
            .bindings
            .get(slot)
            .filter(|bindings| bindings.len() == 1)?;
        match bindings.iter().next()? {
            super::MayBinding::Target(target)
                if target.kind == super::BindingKind::Command
                    && !target.registry_backed
                    && target.prepended.is_empty() =>
            {
                Some(target.clone())
            }
            super::MayBinding::Target(_)
            | super::MayBinding::Imported(_)
            | super::MayBinding::Missing
            | super::MayBinding::Unknown => None,
        }
    }

    pub(super) fn manufacture_object_proof(
        &mut self,
        site: u32,
        class: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> Option<Arc<SourceObjectInstanceProof>> {
        if self.opaque_domain
            || !class.prepended.is_empty()
            || matches!(
                context.frame,
                crate::var_resolve::VariableExecutionFrame::Unknown
            )
        {
            return None;
        }
        let site = CommandAllocationSite {
            source: Arc::clone(self.current_source_origin.as_ref()?),
            offset: site,
        };
        let instance_methods = class
            .identity
            .as_ref()
            .and_then(|identity| self.class_definitions.get(identity))
            .filter(|definition| {
                definition.dispatcher.is_none()
                    && definition.implementation_generation == class.implementation_generation
                    && self.class_definition_dependencies_hold(definition)
            })
            .and_then(|definition| definition.instance_methods.clone());
        let receiver_namespace_path = class
            .identity
            .as_ref()
            .and_then(|identity| self.class_definitions.get(identity))
            .and_then(|definition| context.registry.get(&definition.factory))
            .and_then(|spec| spec.definition_body)
            .filter(|grammar| {
                grammar.member_current_namespace()
                    == tcl_registry::definer::MemberCurrentNamespace::RuntimeReceiver
            })
            .map(|grammar| grammar.member_body_namespace_path);
        let objects = Arc::make_mut(&mut self.object_instances);
        let generation = objects.generation?;
        let index = objects
            .allocations
            .iter()
            .position(|(s, frame, _)| s == &site && frame == context.frame);
        let count = if let Some(index) = index {
            let count = &mut objects.allocations[index].2;
            *count = count.saturating_add(1).min(3);
            *count
        } else {
            objects
                .allocations
                .push((site.clone(), context.frame.clone(), 1));
            1
        };
        let incarnation = match count {
            1 => AllocationIncarnation::First,
            2 => AllocationIncarnation::Second,
            _ => return None,
        };
        Some(Arc::new(SourceObjectInstanceProof {
            allocation: SourceObjectAllocation {
                site,
                frame: context.frame.clone(),
                incarnation,
            },
            class: class.clone(),
            dispatch_generation: generation,
            receiver_dispatcher_generation: objects.receiver_dispatcher_generation,
            receiver_namespace_path,
            instance_methods,
        }))
    }
}

pub(super) fn retain_object_store(
    outcomes: &mut SourceOutcomes,
    facts: &tcl_registry::InvocationFacts,
    target: &SourceCommandTarget,
    arguments: tcl_registry::InvocationArguments<'_>,
    context: SourceExecutionContext<'_>,
) {
    if facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Set,
        )
        || !target.prepended.is_empty()
        || arguments.exact_argv_len() != Some(2)
    {
        return;
    }
    let Some(normal) = &mut outcomes.normal else {
        return;
    };
    let Some(name) = arguments.literal_at(0) else {
        return;
    };
    let receiver = crate::var_resolve::resolve_literal_access(
        name,
        &normal.source_variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Write,
    );
    let Some(key) = crate::var_resolve::canonical_binding_value_key(&receiver) else {
        return;
    };
    Arc::make_mut(&mut normal.object_instances)
        .stores
        .remove(&key);
    let Some(proof) = context
        .written_objects
        .and_then(|values| values.get(2))
        .and_then(Option::as_ref)
    else {
        return;
    };
    if receiver.dynamic
        || receiver.observed
        || normal.opaque_domain
        || normal.object_instances.generation != Some(proof.dispatch_generation)
        || normal.source_variables.contents_presence(&receiver)
            != crate::var_resolve::ContentsPresence::Defined
        || normal
            .source_variables
            .read_contents_origin(&receiver, context.registry)
            != crate::var_resolve::ContentsOrigin::WrittenAt(context.invocation_offset)
    {
        return;
    }
    Arc::make_mut(&mut normal.object_instances).stores.insert(
        key,
        ObjectStore {
            receiver,
            origin: crate::var_resolve::ContentsOrigin::WrittenAt(context.invocation_offset),
            source: normal.current_source_origin.clone(),
            proof: Arc::clone(proof),
        },
    );
    outcomes.normal_object = Some(Arc::clone(proof));
}

/// Carry an actual frozen allocation into an ordinary Value formal's Incoming
/// cell. The activation identity replaces neither the allocation nor its
/// current class/dispatch dependencies.
pub(super) fn retain_value_formals(
    state: &mut ModuleCommandBindings,
    variables: &crate::var_resolve::ResolveContext,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
    actual_count: usize,
    first_actual_word: usize,
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) {
    use tcl_syntax::formal_params::FormalArgumentBinding;
    if !target.prepended.is_empty()
        || context.written_arguments.is_none_or(|words| {
            words.iter().any(|word| {
                matches!(
                    word,
                    crate::registry_invocation::EffectiveInvocationWord::Expanded
                        | crate::registry_invocation::EffectiveInvocationWord::KnownExpansion(_)
                )
            })
        })
    {
        return;
    }
    let Some(receipts) = context
        .written_objects
        .and_then(|values| values.get(first_actual_word..))
    else {
        return;
    };
    if receipts.len() != actual_count {
        return;
    }
    let Some(grammar) = variables
        .invocation_dialect
        .and_then(tcl_registry::InvocationDialect::parameter_grammar)
    else {
        return;
    };
    let Ok(plan) =
        tcl_syntax::formal_params::bind_formal_arguments(parameters, actual_count, grammar)
    else {
        return;
    };
    for binding in plan {
        let FormalArgumentBinding::Value {
            parameter,
            argument,
        } = binding
        else {
            continue;
        };
        let Some(proof) = receipts[argument]
            .as_ref()
            .filter(|proof| state.receiver_allocation_is_current(proof))
        else {
            continue;
        };
        let slot = crate::var_resolve::resolve_literal_access(
            &parameters[parameter].name,
            variables,
            false,
            context.registry,
            tcl_registry::TraceOperation::Read,
        );
        if slot.dynamic
            || slot.observed
            || slot.kind != crate::place::PlaceKind::Scalar
            || slot.index.is_some()
            || !variables.read_produces_value(&slot, context.registry)
            || !matches!(slot.cell.as_ref().map(|cell| &cell.owner),
                Some(crate::place::CellOwner::Activation(owner)) if variables.activation.as_ref() == Some(owner))
            || variables.contents_origin(&slot) != crate::var_resolve::ContentsOrigin::Incoming
        {
            continue;
        }
        if let Some(key) = crate::var_resolve::canonical_binding_value_key(&slot) {
            Arc::make_mut(&mut state.object_instances).stores.insert(
                key,
                ObjectStore {
                    receiver: slot,
                    origin: crate::var_resolve::ContentsOrigin::Incoming,
                    source: None,
                    proof: Arc::clone(proof),
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};

    #[test]
    fn opaque_dispatch_invalidation_is_idempotent() {
        let mut state = SourceObjectState::default();
        state.invalidate_dispatch(true);
        assert_eq!(state.generation, None);
        assert_eq!(state.class_delegate_generation, None);
        assert_eq!(state.receiver_dispatcher_generation, None);
        let unknown = state.clone();
        state.invalidate_dispatch(true);
        assert_eq!(state, unknown);
    }

    #[test]
    fn precise_dispatch_changes_retire_each_prior_generation() {
        let mut state = SourceObjectState::default();
        let original = state.clone();
        state.invalidate_dispatch(false);
        assert_eq!(state.generation, Some(1));
        assert_eq!(state.receiver_dispatcher_generation, Some(1));
        assert_ne!(state.generation, original.generation);
        assert_ne!(
            state.receiver_dispatcher_generation,
            original.receiver_dispatcher_generation
        );
        let prior = state.clone();
        state.invalidate_receiver_dispatcher();
        assert_eq!(state.generation, prior.generation);
        assert_eq!(state.receiver_dispatcher_generation, Some(2));
        state.join(&prior);
        assert_eq!(state.receiver_dispatcher_generation, None);
        state.invalidate_receiver_dispatcher();
        assert_eq!(state.receiver_dispatcher_generation, None);
    }

    #[test]
    fn opaque_dispatch_cannot_revive_a_frozen_precise_generation() {
        let mut state = SourceObjectState::default();
        state.invalidate_dispatch(false);
        let frozen = state.clone();
        state.invalidate_dispatch(true);
        state.join(&frozen);
        state.invalidate_dispatch(false);
        state.invalidate_class_delegates(false);
        state.invalidate_receiver_dispatcher();
        assert_eq!(state.generation, None);
        assert_eq!(state.class_delegate_generation, None);
        assert_eq!(state.receiver_dispatcher_generation, None);
        assert_ne!(state.generation, frozen.generation);
        assert_ne!(
            state.receiver_dispatcher_generation,
            frozen.receiver_dispatcher_generation
        );
    }

    #[test]
    fn object_read_requires_its_allocation_and_current_dispatch() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        for (middle, proved) in [
            ("", true),
            ("set unrelated VALUE; ", true),
            ("oo::objdefine $object class B; ", false),
            (
                "oo::objdefine $object method valid {} {return CUSTOM}; ",
                false,
            ),
            ("set object TEXT; ", false),
        ] {
            let source = format!(
                "oo::class create C {{method valid {{}} {{}}}}; oo::class create B {{}}; set object [C new]; {middle}$object valid"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("$object valid").unwrap()).unwrap();
            let access = bindings
                .variable_accesses
                .get(&offset)
                .and_then(|reads| reads.first())
                .expect(&source);
            assert_eq!(
                access.proved_object_instance().is_some(),
                proved,
                "{source}"
            );
            if let Some(proof) = access.proved_object_instance() {
                assert_eq!(proof.class_target().command, "::C");
                let methods = proof
                    .instance_method_names()
                    .expect("closed method inventory");
                assert!(methods.contains("valid"));
                assert!(!methods.contains("bogus"));
                assert_eq!(proof.allocation().incarnation, AllocationIncarnation::First);
                assert_eq!(
                    proof.allocation().site.offset,
                    u32::try_from(source.find("C new").unwrap()).unwrap()
                );
            }
        }
    }

    #[test]
    fn value_formals_retain_only_the_actual_incoming_object_allocation() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        for (declaration, invocation, proved) in [
            ("proc pass {value} {return $value}", "pass $original", true),
            (
                "proc pass {value} {set value TEXT; return $value}",
                "pass $original",
                false,
            ),
            (
                "proc pass {value} {oo::objdefine $value class B; return $value}",
                "pass $original",
                false,
            ),
            ("", "apply {{value} {return $value}} $original", true),
            (
                "oo::class create Relay {method pass {value} {return $value}}; Relay create relay",
                "relay pass $original",
                true,
            ),
            (
                "oo::class create Relay {method pass {value} {return $value}}; Relay create relay",
                "relay pass TEXT",
                false,
            ),
        ] {
            let source = format!(
                "oo::class create C {{method valid {{}} {{}}}}; oo::class create B {{}}; set original [C new]; {declaration}; set result [{invocation}]; $result valid"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("$result valid").unwrap()).unwrap();
            let proof = bindings
                .variable_accesses
                .get(&offset)
                .and_then(|reads| reads.first())
                .and_then(SourceVariableAccess::proved_object_instance);
            assert_eq!(proof.is_some(), proved, "{source}");
            if let Some(proof) = proof {
                assert_eq!(proof.class_target().command, "::C");
                assert_eq!(
                    proof.allocation().site.offset,
                    u32::try_from(source.find("C new").unwrap()).unwrap()
                );
            }
        }
    }

    #[test]
    fn procedure_result_preserves_only_actual_current_object_allocations() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        for (body, proved) in [
            ("return [C new]", true),
            ("C new", true),
            ("set o [C new]; return $o", true),
            ("set o [C new]; oo::objdefine $o class B; return $o", false),
            ("set o [C new]; error NO", false),
            ("return TEXT", false),
        ] {
            let source = format!(
                "oo::class create C {{method valid {{}} {{}}}}; oo::class create B {{}}; proc make {{}} {{{body}}}; set object [make]; $object valid"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("$object valid").unwrap()).unwrap();
            let access = bindings
                .variable_accesses
                .get(&offset)
                .and_then(|reads| reads.first());
            assert_eq!(
                access
                    .and_then(SourceVariableAccess::proved_object_instance)
                    .is_some(),
                proved,
                "{source}",
            );
            if let Some(proof) = access.and_then(SourceVariableAccess::proved_object_instance) {
                assert_eq!(proof.class_target().command, "::C");
                assert_eq!(
                    proof.allocation().site.offset,
                    u32::try_from(source.find("C new").unwrap()).unwrap()
                );
            }
        }
    }

    #[test]
    fn later_arguments_cannot_borrow_the_heads_class_receipt() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        for body in [
            "return X",
            "oo::objdefine $::object class B; return X",
            "rename $::object gone; return X",
        ] {
            let source = format!(
                "oo::class create C {{method valid {{arg}} {{}}}}; oo::class create B {{}}; proc mutate {{}} {{{body}}}; set object [C new]; $object valid [mutate]"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.find("$object valid").unwrap()).unwrap();
            let access = bindings
                .variable_accesses
                .get(&offset)
                .and_then(|reads| reads.first())
                .expect(&source);
            assert!(access.proved_object_instance().is_some(), "{source}");
            let binding = bindings.invocation_at_source("$object", offset);
            assert_eq!(
                binding.retains_object_instance_at_dispatch(access),
                body == "return X",
                "{source}"
            );
        }
    }

    #[test]
    fn named_instance_navigation_requires_actual_manufacture_and_dispatch() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        for (middle, proved) in [
            ("", true),
            ("set unrelated VALUE; ", true),
            ("oo::objdefine rex class B; ", false),
            ("rename rex gone; ", false),
            ("rename rex {}; proc rex args {return CUSTOM}; ", false),
            ("oo::define C method bark {} {return NEW}; ", false),
        ] {
            let source = format!(
                "oo::class create C {{method bark {{}} {{return WOOF}}}}; oo::class create B {{}}; C create rex; {middle}rex bark"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("rex bark").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("rex", offset);
            assert_eq!(
                binding.named_object_instance_at_dispatch().is_some(),
                proved,
                "{source}"
            );
            assert_eq!(
                binding.named_object_receiver_method_entry().is_some(),
                proved,
                "{source}"
            );
            if let Some((class, entry, _)) = binding.named_object_receiver_method_entry() {
                assert_eq!(class.command, "::C");
                assert_eq!(entry.name(), "bark");
                assert_eq!(entry.body().text.try_text().unwrap(), "return WOOF");
            }
        }
    }

    #[test]
    fn native_visibility_metadata_preserves_the_actual_named_manufacturer() {
        for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let owner = tcl_registry::model::ingress::static_context_for(version);
            let profile = owner.commands().profile().expect("actual native fixture");
            for (prefix, members, exported) in [
                (
                    "",
                    "method Ping {} {return ORIGINAL}; export Ping",
                    Some(true),
                ),
                (
                    "",
                    "method ping {} {return ORIGINAL}; unexport ping",
                    Some(false),
                ),
                (
                    "proc ::oo::define::export args {return CUSTOM}; ",
                    "method Ping {} {return ORIGINAL}; export Ping",
                    None,
                ),
                (
                    "proc ::oo::define::unexport args {return CUSTOM}; ",
                    "method ping {} {return ORIGINAL}; unexport ping",
                    None,
                ),
            ] {
                let method = if members.contains("method Ping") {
                    "Ping"
                } else {
                    "ping"
                };
                let source =
                    format!("{prefix}oo::class create C {{{members}}}; C create obj; obj {method}");
                let bindings = SourceCommandBindings::analyse_with_options(
                    &source,
                    tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                    owner.commands(),
                    SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            profile,
                        )),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        ..Default::default()
                    },
                );
                let offset = u32::try_from(source.rfind("obj ").unwrap()).unwrap();
                let binding = bindings.invocation_at_source("obj", offset);
                let entry = binding.named_object_receiver_method_entry();
                assert_eq!(
                    entry.map(|(_, entry, _)| entry.is_exported()),
                    exported,
                    "{version}: {source}"
                );
                if let Some((_, entry, _)) = entry {
                    assert_eq!(entry.body().text.try_text().unwrap(), "return ORIGINAL");
                }
            }
        }
    }

    #[test]
    fn named_instance_arguments_cannot_reuse_a_retired_receiver() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        for (mutation, proved) in [
            ("return X", true),
            ("oo::objdefine rex class B; return X", false),
            ("rename rex gone; return X", false),
        ] {
            let source = format!(
                "oo::class create C {{method bark {{arg}} {{return WOOF}}}}; oo::class create B {{}}; proc mutate {{}} {{{mutation}}}; C create rex; rex bark [mutate]"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("rex bark").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("rex", offset);
            assert_eq!(
                binding.named_object_receiver_method_entry().is_some(),
                proved,
                "{source}"
            );
        }
    }

    #[test]
    fn inherited_methods_keep_the_declaring_allocation_and_live_base_dependencies() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        for (base, child, middle, owner) in [
            (
                "method ping {} {return BASE}",
                "superclass Base",
                "",
                Some("::Base"),
            ),
            (
                "method ping {} {return BASE}",
                "superclass Base; method ping {} {return CHILD}",
                "",
                Some("::Child"),
            ),
            (
                "method ping {} {return BASE}",
                "superclass Base",
                "oo::define Base method ping {} {return NEW}; ",
                None,
            ),
            (
                "method ping {} {return BASE}",
                "superclass Base",
                "rename Base {}; oo::class create Base {method ping {} {return NEW}}; ",
                None,
            ),
            (
                "constructor {} {oo::objdefine [self] class B}; method ping {} {return BASE}",
                "superclass Base",
                "",
                None,
            ),
            (
                "self method ping {} {return BASE}",
                "superclass Base",
                "",
                None,
            ),
            (
                "method ping {} {return BASE}",
                "superclass Base Base",
                "",
                None,
            ),
        ] {
            let source = format!(
                "oo::class create B {{}}; oo::class create Base {{{base}}}; oo::class create Child {{{child}}}; {middle}Child create rex; rex ping"
            );
            let bindings = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("rex ping").unwrap()).unwrap();
            let binding = bindings.invocation_at_source("rex", offset);
            let entry = binding.named_object_receiver_method_entry();
            assert_eq!(
                entry.map(|(_, entry, _)| entry.declaring_class().unwrap().command.as_str()),
                owner,
                "{source}"
            );
            if let Some((receiver, entry, _)) = entry {
                assert_eq!(receiver.command, "::Child");
                assert_eq!(entry.name(), "ping");
                assert!(
                    entry
                        .declaring_class()
                        .unwrap()
                        .implementation_allocation
                        .is_some()
                );
            }
        }
    }
}
