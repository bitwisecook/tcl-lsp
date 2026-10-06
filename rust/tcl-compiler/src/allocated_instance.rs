// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Physical storage of an actual bounded object allocation.

use crate::{
    command_binding::{AllocationIncarnation, CommandAllocationSite, SourceObjectAllocation},
    place::{CellGeneration, CellIdentity, CellOwner},
    var_resolve::{
        ContentsPresence, ResolveContext, VariableCellKey, VariableFrameKind,
        VariableProofRelocation,
    },
};
use std::collections::HashSet;
use tcl_registry::CommandRegistry;

/// Completion of a sequential native instance-variable link operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocatedInstanceLinkOutcome {
    /// Every selected local name is linked to the actual receiver's storage.
    Linked,
    /// A definitely invalid operand or occupied direct destination stopped linking.
    Error,
    /// An unresolved operand, destination, or owner retains a residual effect.
    Unknown,
}

pub(crate) fn storage_key(allocation: &SourceObjectAllocation, name: &str) -> VariableCellKey {
    VariableCellKey::AllocatedInstance {
        allocation: Box::new(allocation.clone()),
        simple: name.to_owned(),
    }
}

pub(crate) fn relocate_allocation(
    allocation: &SourceObjectAllocation,
    relocation: &VariableProofRelocation,
) -> SourceObjectAllocation {
    SourceObjectAllocation {
        site: CommandAllocationSite {
            source: relocation.source_origin(&allocation.site.source),
            offset: relocation.source_offset(allocation.site.offset),
        },
        frame: relocation.frame(&allocation.frame),
        incarnation: allocation.incarnation,
    }
}

/// Outward storage keys come from typed places, never object-name parsing.
pub(crate) fn outward_cell_keys(context: &ResolveContext) -> HashSet<VariableCellKey> {
    let mut keys = HashSet::new();
    for (key, place) in &context.contents_presence_slots {
        if matches!(
            place.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::AllocatedInstance(_))
        ) {
            keys.insert(key.clone());
            keys.insert(crate::var_resolve::cell_key(place));
            if let Some(key) = crate::var_resolve::canonical_binding_value_key(place) {
                keys.insert(key);
            }
            keys.insert(crate::var_resolve::trace_key(place));
        }
    }
    keys
}

pub(crate) fn relocation_with_cell_keys<'a>(
    context: &ResolveContext,
    relocation: &'a VariableProofRelocation,
) -> std::borrow::Cow<'a, VariableProofRelocation> {
    if !context.contents_presence_slots.values().any(|place| {
        matches!(
            place.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::AllocatedInstance(_))
        )
    }) {
        return std::borrow::Cow::Borrowed(relocation);
    }
    let mut expanded = relocation.clone();
    for place in context.contents_presence_slots.values() {
        if !matches!(
            place.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::AllocatedInstance(_))
        ) {
            continue;
        }
        let relocated = relocation.place(place);
        let mut retain = |old: VariableCellKey, new: VariableCellKey| {
            expanded.storage_keys.insert(old, new);
        };
        retain(
            crate::var_resolve::cell_key(place),
            crate::var_resolve::cell_key(&relocated),
        );
        retain(
            crate::var_resolve::trace_key(place),
            crate::var_resolve::trace_key(&relocated),
        );
        if let (Some(old), Some(new)) = (
            crate::var_resolve::canonical_binding_value_key(place),
            crate::var_resolve::canonical_binding_value_key(&relocated),
        ) {
            retain(old, new);
        }
    }
    std::borrow::Cow::Owned(expanded)
}

impl ResolveContext {
    /// Withdraw storage evidence for the exact object whose destruction was
    /// reached. Names and class labels cannot retire another allocation.
    pub fn retire_allocated_instance_variables(&mut self, allocation: &SourceObjectAllocation) {
        self.closed_observer_allocations.remove(allocation);
        let places = self
            .contents_presence_slots
            .values()
            .filter(|place| {
                matches!(place.cell.as_ref().map(|cell| &cell.owner),
                Some(CellOwner::AllocatedInstance(owner)) if **owner == *allocation)
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut retired_keys = HashSet::new();
        for place in places {
            let root = crate::var_resolve::cell_key(&place);
            self.generations
                .insert(root.clone(), CellGeneration::Unknown);
            self.contents_kinds.remove(&root);
            if let Some(key) = crate::var_resolve::canonical_binding_value_key(&place) {
                retired_keys.insert(key.clone());
                self.forget_literal_value(&key);
                self.value_representations.remove(&key);
                self.contents_presence
                    .insert(key.clone(), ContentsPresence::Unknown);
                self.contents_origins
                    .insert(key, crate::var_resolve::ContentsOrigin::Unknown);
            }
        }
        self.contents_source_proofs.restore(
            &crate::contents_source::ContentsSourceProofs::default(),
            &|key| retired_keys.contains(key),
        );
        for (name, target) in &self.alias_bindings {
            if matches!(target.cell.as_ref().map(|cell| &cell.owner),
                Some(CellOwner::AllocatedInstance(owner)) if **owner == *allocation)
            {
                self.unknown_bindings.insert(name.clone());
            }
        }
    }

    /// Link original local operands to one current, uniquely bounded receiver.
    /// The source dispatcher proves the live receiver and builtin operation;
    /// this owner validates each local destination and preserves earlier links
    /// if a later operand fails. No namespace name or class label supplies storage.
    pub fn link_allocated_instance_variables(
        &mut self,
        allocation: &SourceObjectAllocation,
        names: &[Option<String>],
        registry: &CommandRegistry,
    ) -> AllocatedInstanceLinkOutcome {
        if allocation.incarnation == AllocationIncarnation::RepeatedFresh
            || self.frame_kind != VariableFrameKind::Local
            || self.activation.is_none()
            || self.dynamic_bindings
            || self.invocation_dialect.is_none_or(|dialect| {
                dialect.variable_lookup_policy != Some(tcl_dialect::VariableLookupPolicy::Tcl)
            })
        {
            return AllocatedInstanceLinkOutcome::Unknown;
        }
        for name in names {
            let Some(name) = name else {
                return AllocatedInstanceLinkOutcome::Unknown;
            };
            if !tcl_registry::definer::BuiltinObjectMethodOperation::ObjectVariableLinks
                .accepts_variable_link_name(name)
            {
                return AllocatedInstanceLinkOutcome::Error;
            }
            let slot = crate::var_resolve::resolve_alias_destination_slot(name, self, registry);
            if slot.dynamic || slot.cell.is_none() || self.unknown_bindings.contains(name) {
                return AllocatedInstanceLinkOutcome::Unknown;
            }
            if crate::variable_bindings::alias_destination_is_direct(self, name, &slot) {
                if self.captured_contents_presence(&slot) == ContentsPresence::Defined {
                    return AllocatedInstanceLinkOutcome::Error;
                }
                if self.traced.contains(&crate::var_resolve::cell_key(&slot))
                    || self
                        .trace_registrations
                        .contains_key(&crate::var_resolve::trace_key(&slot))
                {
                    return AllocatedInstanceLinkOutcome::Error;
                }
                if self.unenumerated_observers_may_run(&slot)
                    || self
                        .untracked_traces
                        .contains(&crate::var_resolve::cell_key(&slot))
                    || self.captured_contents_presence(&slot) != ContentsPresence::Undefined
                {
                    return AllocatedInstanceLinkOutcome::Unknown;
                }
            }
            let mut target = crate::place::scalar(name, crate::place::LOCAL_NS, false);
            target.cell = Some(CellIdentity {
                owner: CellOwner::AllocatedInstance(Box::new(allocation.clone())),
                name: name.clone(),
                generation: CellGeneration::Incoming,
                interpreter: self.interpreter.clone(),
                storage_domain: None,
                execution: self.execution,
            });
            let generation = self
                .generations
                .get(&crate::var_resolve::cell_key(&target))
                .copied()
                .unwrap_or_default();
            target
                .cell
                .as_mut()
                .expect("instance owner was installed")
                .generation = generation;
            self.record_presence_slot(&target);
            let local_key = crate::var_resolve::cell_key(&slot);
            if let Some(wrapper) = self
                .raw_bindings
                .bindings
                .get(&local_key)
                .and_then(|key| self.raw_bindings.slots.get_mut(key))
            {
                wrapper.contents =
                    crate::raw_binding::RawBindingContents::Direct(Box::new(target.clone()));
            }
            self.name_alias_bindings.remove(name);
            self.unknown_bindings.remove(name);
            self.alias_bindings.insert(name.clone(), target);
        }
        AllocatedInstanceLinkOutcome::Linked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        command_binding::SourceOriginId,
        var_resolve::{
            VariableExecutionFrame, canonical_binding_value_key, resolve_literal_place,
            restore_execution_frame,
        },
    };
    use std::{collections::BTreeMap, sync::Arc};

    fn fixture() -> (ResolveContext, SourceObjectAllocation) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        state.interpreter = Some("actual-interpreter".to_owned());
        let allocation = SourceObjectAllocation {
            site: CommandAllocationSite {
                source: Arc::new(SourceOriginId::authored(&Arc::from("actual source"))),
                offset: 10,
            },
            frame: VariableExecutionFrame::Procedure {
                namespace: "::".to_owned(),
                identity: "manufacturer".to_owned(),
            },
            incarnation: AllocationIncarnation::First,
        };
        (state, allocation)
    }

    #[test]
    fn fresh_receiver_observers_require_the_exact_live_allocation() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (incoming, allocation) = fixture();
        let frame = VariableExecutionFrame::ReceiverMethod {
            identity: "constructor-call".to_owned(),
        };
        let mut incoming = incoming;
        incoming.dynamic_traces = true;
        let mut state = incoming.enter_called_frame(&frame);
        state.closed_observer_allocations.insert(allocation.clone());
        assert_eq!(
            state.link_allocated_instance_variables(&allocation, &[Some("x".to_owned())], registry),
            AllocatedInstanceLinkOutcome::Linked
        );
        let x = resolve_literal_place("x", &state, false, registry);
        assert!(!x.observed);
        state.define_literal("x", "42", registry);
        assert_eq!(state.contents_presence(&x), ContentsPresence::Defined);
        let mut foreign = state.clone();
        foreign.closed_observer_allocations.clear();
        assert!(resolve_literal_place("x", &foreign, false, registry).observed);
        let mut traced = state.clone();
        traced.traced.insert(crate::var_resolve::cell_key(&x));
        assert!(resolve_literal_place("x", &traced, false, registry).observed);
        let mut unknown_write = state.clone();
        unknown_write.record_contents_write(&crate::place::unknown_top(), 0, true);
        assert!(
            !unknown_write
                .closed_observer_allocations
                .contains(&allocation)
        );
        assert!(resolve_literal_place("x", &unknown_write, false, registry).observed);
        let mut mixed = state.clone();
        mixed.join(&unknown_write);
        assert!(resolve_literal_place("x", &mixed, false, registry).observed);
        state.mark_unenumerated_variable_observers();
        assert!(resolve_literal_place("x", &state, false, registry).observed);
        let mut retired = incoming.enter_called_frame(&frame);
        retired
            .closed_observer_allocations
            .insert(allocation.clone());
        retired.retire_allocated_instance_variables(&allocation);
        assert!(!retired.closed_observer_allocations.contains(&allocation));
    }

    #[test]
    fn instance_links_preserve_partial_prefix_and_exact_allocation_identity() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for incarnation in [AllocationIncarnation::First, AllocationIncarnation::Second] {
            let (mut state, mut allocation) = fixture();
            allocation.incarnation = incarnation;
            assert_eq!(
                state.link_allocated_instance_variables(
                    &allocation,
                    &[Some("value".to_owned()), Some("bad(k)".to_owned())],
                    registry
                ),
                AllocatedInstanceLinkOutcome::Error
            );
            let target = resolve_literal_place("value", &state, false, registry);
            assert!(matches!(target.cell.as_ref().map(|cell| &cell.owner),
                Some(CellOwner::AllocatedInstance(owner)) if **owner == allocation));
            assert_eq!(
                target.cell.as_ref().unwrap().interpreter.as_deref(),
                Some("actual-interpreter")
            );
            state.define_literal("value", "OLD", registry);
            assert_eq!(
                state.link_allocated_instance_variables(
                    &allocation,
                    &[Some("value".to_owned()), Some(String::new())],
                    registry
                ),
                AllocatedInstanceLinkOutcome::Linked
            );
            assert_eq!(state.literal_contents_at(&target, registry), Some("OLD"));
            assert!(state.alias_bindings.contains_key(""));
            let mut other = allocation.clone();
            other.incarnation = if incarnation == AllocationIncarnation::First {
                AllocationIncarnation::Second
            } else {
                AllocationIncarnation::First
            };
            state.retire_allocated_instance_variables(&other);
            assert_eq!(state.literal_contents_at(&target, registry), Some("OLD"));
            state.retire_allocated_instance_variables(&allocation);
            assert!(state.literal_contents_at(&target, registry).is_none());
            assert_eq!(
                resolve_literal_place("value", &state, false, registry).kind,
                crate::place::PlaceKind::Unknown
            );
        }
        let (mut state, mut repeated) = fixture();
        repeated.incarnation = AllocationIncarnation::RepeatedFresh;
        assert_eq!(
            state.link_allocated_instance_variables(&repeated, &[Some("v".to_owned())], registry),
            AllocatedInstanceLinkOutcome::Unknown
        );
        assert!(state.alias_bindings.is_empty());
    }

    #[test]
    fn direct_values_and_traces_reject_links_but_existing_aliases_can_rebind() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (mut state, allocation) = fixture();
        state.bind_unknown_incoming("formal", registry);
        assert_eq!(
            state.link_allocated_instance_variables(
                &allocation,
                &[Some("formal".to_owned())],
                registry
            ),
            AllocatedInstanceLinkOutcome::Error
        );
        let slot = resolve_literal_place("traced", &state, false, registry);
        state.trace_registrations.insert(
            crate::var_resolve::trace_key(&slot),
            vec![(
                vec![tcl_registry::TraceOperation::Write],
                "watch".to_owned(),
            )],
        );
        assert_eq!(
            state.link_allocated_instance_variables(
                &allocation,
                &[Some("traced".to_owned())],
                registry
            ),
            AllocatedInstanceLinkOutcome::Error
        );
        let global = resolve_literal_place("::foreign", &state, false, registry);
        state
            .alias_bindings
            .insert("view".to_owned(), global.clone());
        assert_eq!(
            state.link_allocated_instance_variables(
                &allocation,
                &[Some("view".to_owned())],
                registry
            ),
            AllocatedInstanceLinkOutcome::Linked
        );
        assert_ne!(
            resolve_literal_place("view", &state, false, registry).cell,
            global.cell
        );
    }

    #[test]
    fn instance_storage_survives_method_frames_and_typed_relocation() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (parent, allocation) = fixture();
        let mut method = parent.enter_called_frame(&VariableExecutionFrame::ReceiverMethod {
            identity: "actual-method".to_owned(),
        });
        assert_eq!(
            method.link_allocated_instance_variables(
                &allocation,
                &[Some("v".to_owned())],
                registry
            ),
            AllocatedInstanceLinkOutcome::Linked
        );
        method.define_literal("v", "VALUE", registry);
        let target = resolve_literal_place("v", &method, false, registry);
        let restored = restore_execution_frame(&parent, &method);
        let mut next = restored.enter_called_frame(&VariableExecutionFrame::ReceiverMethod {
            identity: "next-method".to_owned(),
        });
        assert_eq!(
            next.link_allocated_instance_variables(&allocation, &[Some("v".to_owned())], registry),
            AllocatedInstanceLinkOutcome::Linked
        );
        assert_eq!(
            next.literal_contents_at(
                &resolve_literal_place("v", &next, false, registry),
                registry
            ),
            Some("VALUE")
        );
        let relocation = VariableProofRelocation {
            source_offsets: BTreeMap::from([(10, 20)]),
            activations: BTreeMap::from([(
                "manufacturer".to_owned(),
                "template-manufacturer".to_owned(),
            )]),
            ..Default::default()
        };
        let moved = next.relocated(&relocation);
        let relocated_target = relocation.place(&target);
        assert_ne!(
            canonical_binding_value_key(&relocated_target),
            canonical_binding_value_key(&target)
        );
        assert_eq!(
            moved.literal_contents_at(&relocated_target, registry),
            Some("VALUE")
        );
        assert_eq!(moved.relocated(&relocation.inverse().unwrap()), next);
    }

    #[test]
    fn retained_local_wrapper_follows_the_new_physical_instance_link() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (mut state, allocation) = fixture();
        let original = resolve_literal_place("v", &state, false, registry);
        let identity = crate::raw_binding::RawBindingSlotId::Variable(Box::new(
            original.cell.clone().unwrap(),
        ));
        let key = state
            .raw_bindings
            .insert(crate::raw_binding::RawBindingSlot {
                identity,
                contents: crate::raw_binding::RawBindingContents::Direct(Box::new(
                    original.clone(),
                )),
            });
        state
            .raw_bindings
            .bindings
            .insert(crate::var_resolve::cell_key(&original), key);
        assert_eq!(
            state.link_allocated_instance_variables(&allocation, &[Some("v".to_owned())], registry),
            AllocatedInstanceLinkOutcome::Linked
        );
        let target = resolve_literal_place("v", &state, false, registry);
        assert!(matches!(
            target.cell.as_ref().map(|cell| &cell.owner),
            Some(CellOwner::AllocatedInstance(_))
        ));
        state.define_literal("v", "VALUE", registry);
        assert_eq!(
            state.link_allocated_instance_variables(&allocation, &[Some("v".to_owned())], registry),
            AllocatedInstanceLinkOutcome::Linked
        );
    }
}
