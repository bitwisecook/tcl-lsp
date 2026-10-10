// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Captured, staged teardown of a proved distinct-array allocation.

use crate::place::{CellGeneration, CellOwner, Place, PlaceKind};
use crate::raw_binding::{RawBindingContents, RawBindingSlot, RawBindingSlotId};
use crate::var_resolve::{
    ContentsWorld, ResolveContext, RootContentsKind, VariableObserverProjection,
    canonical_binding_value_key, cell_key, trace_key,
};
use tcl_registry::native_variable_destruction::VariableDestructionProtocol;
use tcl_registry::{CommandRegistry, TraceOperation};

/// Contents inventory for a reached native array operation. The member set is
/// unordered; value reads and observers remain separate reached operations.
#[derive(Debug)]
pub(crate) enum ArrayContentsInventory {
    /// The selected receiver is definitely absent or contains a scalar.
    Error,
    /// Complete bounded set of defined members, including an empty array.
    Known(Vec<Place>),
    /// Root kind, member presence or enumeration retains an unknown residual.
    Unknown,
}

/// Normal-edge transfer of a selected, frozen array-set operand list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArrayEntriesStoreOutcome {
    Stored,
    Error,
    Unknown,
}

/// One member captured from the original root, independent of later name lookup.
#[derive(Debug)]
pub(crate) struct DestructionMember {
    pub(crate) receiver: Place,
}

/// A complete bounded inventory. This licenses no particular member order.
#[derive(Debug)]
pub(crate) struct ArrayDestruction {
    pub(crate) original: Place,
    pub(crate) retained: Place,
    pub(crate) root_callbacks: VariableObserverProjection,
    pub(crate) members: Vec<DestructionMember>,
}

impl ResolveContext {
    /// Store exact entries in an already selected root without replacing or
    /// widening untouched members. The caller selects the native array-set
    /// contract and its frozen key/value list; no command is evaluated here.
    pub(crate) fn publish_array_entries(
        &mut self,
        root: &Place,
        entries: &[(String, String)],
        source: u32,
        registry: &CommandRegistry,
    ) -> ArrayEntriesStoreOutcome {
        use crate::var_resolve::{ContentsOrigin, ContentsPresence};
        if self
            .invocation_dialect
            .and_then(|dialect| dialect.variable_container_model)
            != Some(tcl_dialect::VariableContainerModel::DistinctArray)
            || root.dynamic
            || root.index.is_some()
            || root
                .cell
                .as_ref()
                .is_none_or(|cell| cell.generation == CellGeneration::Unknown)
            || root.observed
            || (root.is_global()
                && !root.cell.as_ref().is_some_and(|cell| match &cell.owner {
                    CellOwner::Namespace(namespace) => self.known_namespaces.contains(namespace),
                    CellOwner::NamespaceIdentity(namespace) => {
                        self.namespace_identities.contains(namespace.as_ref())
                    }
                    _ => false,
                }))
        {
            return ArrayEntriesStoreOutcome::Unknown;
        }
        let array_observers = self.variable_observers_at(root, TraceOperation::Array, registry);
        if array_observers.unknown_residual
            || !array_observers.callbacks.is_empty()
            || !array_observers.possible_callbacks.is_empty()
        {
            return ArrayEntriesStoreOutcome::Unknown;
        }
        let mut root = root.clone();
        root.kind = PlaceKind::ArrayWhole;
        let missing = self.contents_presence(&root) == ContentsPresence::Undefined;
        match self.root_contents_kind(&root) {
            Some(RootContentsKind::Scalar) => return ArrayEntriesStoreOutcome::Error,
            Some(RootContentsKind::Array) => {}
            None if missing => {}
            None => return ArrayEntriesStoreOutcome::Unknown,
        }
        let members = entries
            .iter()
            .map(|(key, _)| {
                let mut member = root.clone();
                member.kind = PlaceKind::ArrayElem;
                member.index = Some(crate::place::Index::literal(key));
                member
            })
            .collect::<Vec<_>>();
        if std::iter::once(&root).chain(&members).any(|receiver| {
            let observers = self.variable_observers_at(receiver, TraceOperation::Write, registry);
            observers.unknown_residual
                || !observers.callbacks.is_empty()
                || !observers.possible_callbacks.is_empty()
        }) {
            return ArrayEntriesStoreOutcome::Unknown;
        }
        self.record_presence_slot(&root);
        if missing {
            self.record_contents_source(&root, false);
        }
        let key = cell_key(&root);
        if missing && let Some(root) = crate::var_resolve::physical_array_key(&root) {
            self.closed_array_roots.insert(root);
        }
        self.contents_kinds
            .insert(key.clone(), RootContentsKind::Array);
        self.contents_presence
            .insert(key.clone(), ContentsPresence::Defined);
        if let Some(contents) = canonical_binding_value_key(&root) {
            self.contents_presence
                .insert(contents.clone(), ContentsPresence::Defined);
            if missing {
                self.contents_origins
                    .insert(contents, ContentsOrigin::WrittenAt(source));
            }
        }
        if root.is_global() {
            self.namespace_cells.present.insert(key);
        }
        for (member, (_, value)) in members.iter().zip(entries) {
            self.publish_captured_store(member, Some(value), source);
        }
        ArrayEntriesStoreOutcome::Stored
    }

    pub(crate) fn array_contents_inventory(&self, root: &Place) -> ArrayContentsInventory {
        use crate::var_resolve::ContentsPresence;
        if root.observed || root.dynamic || root.index.is_some() {
            return ArrayContentsInventory::Unknown;
        }
        if self.contents_presence(root) == ContentsPresence::Undefined
            || self.root_contents_kind(root) == Some(RootContentsKind::Scalar)
        {
            return ArrayContentsInventory::Error;
        }
        if self.contents_presence(root) != ContentsPresence::Defined
            || self.root_contents_kind(root) != Some(RootContentsKind::Array)
            || !self.closed_array_contents_world(root)
        {
            return ArrayContentsInventory::Unknown;
        }
        let mut members = Vec::<Place>::new();
        for candidate in self.array_member_candidates() {
            if candidate.cell != root.cell || candidate.index.is_none() {
                continue;
            }
            if candidate.dynamic
                || candidate
                    .index
                    .as_ref()
                    .is_none_or(|index| index.kind != crate::place::IndexKind::Literal)
            {
                return ArrayContentsInventory::Unknown;
            }
            let mut candidate = candidate.clone();
            candidate.observed = false;
            match self.closed_contents_presence(&candidate) {
                Some(ContentsPresence::Undefined) => continue,
                Some(ContentsPresence::Defined) => {}
                _ => return ArrayContentsInventory::Unknown,
            }
            if members.iter().any(|member| member.index == candidate.index) {
                continue;
            }
            if members.len() == 32 {
                return ArrayContentsInventory::Unknown;
            }
            members.push(candidate);
        }
        ArrayContentsInventory::Known(members)
    }

    fn closed_array_contents_world(&self, root: &Place) -> bool {
        (self.contents_world == ContentsWorld::Tracked
            || crate::var_resolve::physical_array_key(root)
                .is_some_and(|key| self.closed_array_roots.contains(&key)))
            && !self.dynamic_bindings
            && crate::var_resolve::physical_array_key(root)
                .is_some_and(|key| !self.contents_unknown_arrays.contains(&key))
            && !self
                .contents_unknown_namespaces
                .iter()
                .any(|namespace| cell_key(root).is_in_namespace(namespace))
    }
    /// Freeze receivers and root callbacks before detaching the original lookup.
    /// Each member's registrations are captured at its own retirement phase.
    /// Opaque contents, unenumerated members and repeated allocation families
    /// cannot supply a unique retained old-root receipt.
    pub(crate) fn begin_array_destruction(
        &mut self,
        receiver: &Place,
        source: u32,
        registry: &CommandRegistry,
    ) -> Option<ArrayDestruction> {
        let mut original = receiver.clone();
        original.observed = false;
        if original.index.is_some()
            || self.invocation_dialect?.variable_destruction_protocol(true)
                != Some(VariableDestructionProtocol::ArrayLookupThenRootCallbacksThenMembers)
            || self.root_contents_kind(&original) != Some(RootContentsKind::Array)
            || !self.closed_array_contents_world(&original)
            || self.dynamic_traces
            || !self.array_registration_receivers_known(&original)
        {
            return None;
        }
        let identity = RawBindingSlotId::Variable(Box::new(original.cell.clone()?));
        if self
            .raw_bindings
            .slots
            .contains_key(&identity.storage_key())
        {
            return None;
        }
        let root_callbacks =
            self.variable_observers_on_cell(&original, TraceOperation::Unset, registry);
        if !closed_callbacks(&root_callbacks) {
            return None;
        }
        let members = self.array_destruction_members(&original, registry)?;
        let lease = self.capture_protected_cell(&original)?;
        self.captured_cells.release(lease);
        let mut retained = original.clone();
        retained.kind = PlaceKind::ArrayWhole;
        let cell = retained.cell.as_mut()?;
        cell.owner = CellOwner::RetainedSlot(Box::new(identity.clone()));
        cell.generation = CellGeneration::Incoming;
        let key = self.raw_bindings.insert(RawBindingSlot {
            identity,
            contents: RawBindingContents::Direct(Box::new(retained.clone())),
        });
        self.raw_bindings.retiring_arrays.insert(key);
        self.copy_retained_contents(&original, &retained);
        if let (Some(original_key), Some(retained_key)) = (
            crate::var_resolve::physical_array_key(&original),
            crate::var_resolve::physical_array_key(&retained),
        ) && self.closed_array_roots.contains(&original_key)
        {
            self.closed_array_roots.insert(retained_key);
        }
        let members = self.retain_destruction_members(members, &retained);
        self.captured_cells
            .retain_array_members(&original, &retained);
        self.retain_existing_member_aliases();
        self.invalidate_contents_literals(&original);
        crate::variable_bindings::destroy_captured_cell(self, &original, source, registry);
        Some(ArrayDestruction {
            original,
            retained,
            root_callbacks,
            members,
        })
    }

    fn array_destruction_members(
        &self,
        root: &Place,
        registry: &CommandRegistry,
    ) -> Option<Vec<DestructionMember>> {
        let mut members = Vec::<DestructionMember>::new();
        for receiver in self.array_member_candidates() {
            if receiver.cell != root.cell || receiver.index.is_none() {
                continue;
            }
            if receiver.dynamic || receiver.index.as_ref()?.kind != crate::place::IndexKind::Literal
            {
                return None;
            }
            if members
                .iter()
                .any(|member| member.receiver.index == receiver.index)
            {
                continue;
            }
            let callbacks =
                self.variable_observers_on_cell(receiver, TraceOperation::Unset, registry);
            if !closed_callbacks(&callbacks) || members.len() == 4 {
                return None;
            }
            let mut receiver = receiver.clone();
            receiver.observed = false;
            members.push(DestructionMember { receiver });
        }
        Some(members)
    }

    fn array_member_candidates(&self) -> Vec<&Place> {
        let mut candidates = self
            .contents_presence_slots
            .values()
            .chain(self.trace_registration_receivers.values())
            .map(std::sync::Arc::as_ref)
            .chain(self.namespace_alias_bindings.values())
            .collect::<Vec<_>>();
        let mut frame = Some(self);
        while let Some(current) = frame {
            candidates.extend(current.alias_bindings.values());
            frame = current.caller.as_deref();
        }
        candidates
    }

    fn retain_destruction_members(
        &mut self,
        members: Vec<DestructionMember>,
        root: &Place,
    ) -> Vec<DestructionMember> {
        members
            .into_iter()
            .map(|mut member| {
                let original = member.receiver.clone();
                member.receiver.cell.clone_from(&root.cell);
                self.copy_retained_contents(&original, &member.receiver);
                let original_key = trace_key(&original);
                if let Some(registrations) = self.trace_registrations.get(&original_key).cloned() {
                    let key = trace_key(&member.receiver);
                    self.traced.insert(key.clone());
                    self.trace_registrations.insert(key.clone(), registrations);
                    self.trace_registration_receivers
                        .insert(key, std::sync::Arc::new(member.receiver.clone()));
                }
                member
            })
            .collect()
    }

    fn retain_existing_member_aliases(&mut self) {
        for target in self
            .alias_bindings
            .values_mut()
            .chain(self.namespace_alias_bindings.values_mut())
        {
            if let Some(retained) = self.raw_bindings.retained_array_member(target) {
                *target = retained;
            }
        }
        if let Some(caller) = &mut self.caller {
            let caller = std::sync::Arc::make_mut(caller);
            caller.raw_bindings.clone_from(&self.raw_bindings);
            caller.retain_existing_member_aliases();
        }
    }

    /// A member becomes missing immediately before its frozen unset callbacks.
    pub(crate) fn retire_array_member(
        &mut self,
        member: &Place,
        source: u32,
        registry: &CommandRegistry,
    ) {
        self.invalidate_contents_literals(member);
        crate::variable_bindings::destroy_captured_cell(self, member, source, registry);
        self.captured_cells.retire_array_member(member);
        if let Some(index) = &member.index {
            self.raw_bindings
                .retired_array_members
                .entry(cell_key(member))
                .or_default()
                .insert(index.value.clone());
        }
    }

    /// Retire the old array owner after every member callback has finished.
    /// Its tombstone remains a distinct identity from any recreated root.
    pub(crate) fn finish_array_destruction(&mut self, root: &Place) {
        self.captured_cells
            .destroy(root, Some(RootContentsKind::Array));
        if let Some(key) = canonical_binding_value_key(root) {
            self.contents_presence
                .insert(key, crate::var_resolve::ContentsPresence::Undefined);
        }
        self.contents_kinds.remove(&cell_key(root));
        if let Some(root) = crate::var_resolve::physical_array_key(root) {
            self.closed_array_roots.remove(&root);
        }
        self.raw_bindings.retiring_arrays.remove(&cell_key(root));
    }
}

fn closed_callbacks(projection: &VariableObserverProjection) -> bool {
    !projection.unknown_residual && projection.possible_callbacks.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::var_resolve::{
        BindingIdentity, ContentsPresence, VariableFrameKind, resolve_literal_place,
    };

    fn context() -> (ResolveContext, std::sync::Arc<CommandRegistry>) {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6")
            .commands()
            .clone();
        let mut state = ResolveContext::for_frame(
            VariableFrameKind::Global,
            BindingIdentity::Bound,
            Some(tcl_registry::InvocationDialect::of_profile(
                tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
            )),
        );
        state.define_literal("a(x)", "X", &registry);
        state.define_literal("a(y)", "Y", &registry);
        let x = resolve_literal_place("a(x)", &state, false, &registry);
        let y = resolve_literal_place("a(y)", &state, false, &registry);
        state.alias_bindings.insert("ex", x);
        state.alias_bindings.insert("ey", y);
        (state, registry)
    }

    #[test]
    fn array_entry_facts_keep_original_namespace_incarnations() {
        use crate::command_binding::SourceNamespaceKey;
        use crate::var_resolve::{ContentsPresence, cell_key};
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeNamespaceContext,
        };
        let (mut state, registry) = context();
        let mut root = resolve_literal_place("a", &state, false, &registry);
        let native = NativeNamespaceContext {
            interpreter: NativeInterpreterIdentity {
                owner: NativeInterpreterIdentity::fresh_owner(),
                interpreter: 0,
            },
            token: 1,
            path: tcl_core_types::ByteNamespacePath::from_segments(["a:", "b"]),
        };
        let owner = SourceNamespaceKey::Native(native.clone());
        root.ns = owner.display().unwrap();
        root.cell.as_mut().unwrap().owner = CellOwner::NamespaceIdentity(Box::new(owner.clone()));
        state.namespace_identities.insert(owner);
        state
            .contents_presence
            .insert(cell_key(&root), ContentsPresence::Undefined);
        assert_eq!(
            state.publish_array_entries(&root, &[("k".into(), "VALUE".into())], 70, &registry),
            ArrayEntriesStoreOutcome::Stored,
        );
        assert_eq!(
            state.root_contents_kind(&root),
            Some(RootContentsKind::Array)
        );
        assert_eq!(state.contents_presence(&root), ContentsPresence::Defined);
        assert!(
            matches!(state.array_contents_inventory(&root), ArrayContentsInventory::Known(members) if members.len() == 1)
        );
        let mut replacement = root.clone();
        replacement.cell.as_mut().unwrap().owner = CellOwner::NamespaceIdentity(Box::new(
            SourceNamespaceKey::Native(NativeNamespaceContext { token: 2, ..native }),
        ));
        assert_eq!(
            state.publish_array_entries(
                &replacement,
                &[("k".into(), "OTHER".into())],
                71,
                &registry
            ),
            ArrayEntriesStoreOutcome::Unknown,
        );
        assert_eq!(state.root_contents_kind(&replacement), None);
    }

    #[test]
    fn exact_array_entries_preserve_other_members_and_empty_allocation() {
        let (mut state, registry) = context();
        state.namespace_cells = crate::variable_bindings::fresh_namespace_cells(&registry);
        let root = resolve_literal_place("a", &state, true, &registry);
        assert_eq!(
            state.publish_array_entries(&root, &[("x".into(), "NEW".into())], 20, &registry),
            ArrayEntriesStoreOutcome::Stored
        );
        assert_eq!(state.literal_value("a(x)", &registry), Some("NEW"));
        assert_eq!(state.literal_value("a(y)", &registry), Some("Y"));
        assert!(
            matches!(state.array_contents_inventory(&root), ArrayContentsInventory::Known(members) if members.len() == 2)
        );
        let empty = resolve_literal_place("empty", &state, true, &registry);
        assert_eq!(
            state.publish_array_entries(&empty, &[], 21, &registry),
            ArrayEntriesStoreOutcome::Stored
        );
        assert!(
            matches!(state.array_contents_inventory(&empty), ArrayContentsInventory::Known(members) if members.is_empty())
        );
        state.record_contents_write(&root, 22, false);
        assert_eq!(
            state.publish_array_entries(&root, &[("x".into(), "KNOWN".into())], 23, &registry),
            ArrayEntriesStoreOutcome::Stored
        );
        assert!(matches!(
            state.array_contents_inventory(&root),
            ArrayContentsInventory::Unknown
        ));
    }

    #[test]
    fn old_member_aliases_remain_live_until_their_own_retirement() {
        let (mut state, registry) = context();
        let root = resolve_literal_place("a", &state, false, &registry);
        let plan = state.begin_array_destruction(&root, 20, &registry).unwrap();
        assert_eq!(state.literal_value("ex", &registry), Some("X"));
        assert_eq!(state.literal_value("ey", &registry), Some("Y"));
        assert_eq!(
            state.contents_presence(&resolve_literal_place("a", &state, false, &registry)),
            ContentsPresence::Undefined
        );
        let x = resolve_literal_place("ex", &state, false, &registry);
        state.publish_captured_store(&x, Some("OLD UPDATE"), 21);
        assert_eq!(state.literal_value("ex", &registry), Some("OLD UPDATE"));
        state.define_literal("a(x)", "NEW", &registry);
        state.retire_array_member(&x, 22, &registry);
        assert_eq!(state.contents_presence(&x), ContentsPresence::Undefined);
        assert!(state.store_would_error(&x));
        assert_eq!(state.literal_value("ey", &registry), Some("Y"));
        assert_eq!(state.literal_value("a(x)", &registry), Some("NEW"));
        let y = resolve_literal_place("ey", &state, false, &registry);
        state.retire_array_member(&y, 23, &registry);
        state.finish_array_destruction(&plan.retained);
        assert_eq!(state.contents_presence(&y), ContentsPresence::Undefined);
        assert_eq!(state.literal_value("a(x)", &registry), Some("NEW"));
    }

    #[test]
    fn active_read_receipts_follow_the_old_member_and_retire_permanently() {
        let (mut state, registry) = context();
        let x = resolve_literal_place("ex", &state, false, &registry);
        let lease = state.capture_protected_cell(&x).unwrap();
        let root = resolve_literal_place("a", &state, false, &registry);
        let plan = state.begin_array_destruction(&root, 20, &registry).unwrap();
        let protected = state.captured_cell_receiver(lease).unwrap();
        assert_eq!(
            protected,
            resolve_literal_place("ex", &state, false, &registry)
        );
        state.define_literal("a(x)", "NEW", &registry);
        assert_ne!(
            protected.cell,
            resolve_literal_place("a(x)", &state, false, &registry).cell
        );
        state.retire_array_member(&protected, 21, &registry);
        assert!(state.captured_cell_receiver(lease).is_none());
        state.finish_array_destruction(&plan.retained);
        assert!(state.captured_cell_receiver(lease).is_none());
    }

    #[test]
    fn repeated_family_unknown_members_and_opaque_worlds_decline_without_detaching() {
        let (mut state, registry) = context();
        let root = resolve_literal_place("a", &state, false, &registry);
        let identity = RawBindingSlotId::Variable(Box::new(root.cell.clone().unwrap()));
        state.raw_bindings.insert(RawBindingSlot {
            identity,
            contents: RawBindingContents::Unknown,
        });
        let before = state.clone();
        assert!(
            state
                .begin_array_destruction(&root, 20, &registry)
                .is_none()
        );
        assert_eq!(state, before);
        let (mut state, registry) = context();
        state.contents_world = ContentsWorld::Unknown;
        let before = state.clone();
        assert!(
            state
                .begin_array_destruction(&root, 20, &registry)
                .is_none()
        );
        assert_eq!(state, before);
        let (mut state, registry) = context();
        let mut root = resolve_literal_place("a", &state, false, &registry);
        root.kind = PlaceKind::ArrayWhole;
        state.record_contents_write(&root, 19, false);
        let before = state.clone();
        assert!(
            state
                .begin_array_destruction(&root, 20, &registry)
                .is_none()
        );
        assert_eq!(state, before);
    }
}
