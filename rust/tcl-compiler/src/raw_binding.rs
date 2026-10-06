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

//! Retained variable wrappers and callable-owned static storage.
//!
//! Jim can retain a `VarVal` containing either direct contents or a logical-level
//! name link. Retaining that wrapper differs from retaining its current target.
//! Named slots and retained allocations have typed, separate storage keys;
//! namespace component geometry and incarnation survive every capture and join.

use crate::command_binding::{AllocationIncarnation, CommandAllocationSite};
use crate::place::{self, CellIdentity, CellOwner, Place};
use crate::var_resolve::{
    ResolveContext, VariableCellKey, VariableCellSet, VariableCellTable, VariableProofRelocation,
    canonical_place_key,
};
use std::collections::BTreeSet;

/// Immutable allocation of one callable implementation's persistent storage.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallableStorageIdentity {
    /// Source allocation instruction, independent of the callable's current name.
    pub site: CommandAllocationSite,
    /// Repeated allocation has no unique retained runtime implementation.
    pub incarnation: AllocationIncarnation,
    /// Implementation generation at this allocation.
    pub implementation_generation: u32,
}

/// Identity of a retained variable wrapper, rather than its current link target.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RawBindingSlotId {
    /// Wrapper allocated in a proved variable table and lifetime.
    Variable(Box<CellIdentity>),
    /// Private persistent variable allocated by one callable implementation.
    Callable {
        /// Immutable callable allocation.
        allocation: CallableStorageIdentity,
        /// Static declaration's name.
        name: String,
    },
}

impl RawBindingSlotId {
    /// Exact retained wrapper allocation, distinct from its named source slot.
    #[must_use]
    pub fn storage_key(&self) -> VariableCellKey {
        VariableCellKey::RetainedSlot(Box::new(self.clone()))
    }

    /// Alpha-relocate variable allocations while retaining external callable allocations.
    #[must_use]
    pub fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        match self {
            Self::Variable(cell) => {
                let mut original = place::scalar(&cell.name, place::LOCAL_NS, false);
                original.cell = Some((**cell).clone());
                Self::Variable(Box::new(relocation.place(&original).cell.unwrap()))
            }
            callable @ Self::Callable { .. } => callable.clone(),
        }
    }
}

/// Contents of a raw wrapper. A name link is resolved at access time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RawBindingContents {
    /// A retained direct value cell.
    Direct(Box<Place>),
    /// Jim stores an absolute logical level, whose activation can later be reused.
    SelectedName {
        /// Absolute interpreter frame level.
        level: u32,
        /// Already evaluated target variable name.
        name: String,
    },
    /// Unproved retargeting, allocation multiplicity or incoming wrapper contents.
    Unknown,
}

/// One typed wrapper allocation and its current contents.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RawBindingSlot {
    /// Retained allocation identity.
    pub identity: RawBindingSlotId,
    /// Direct contents or selected-name link.
    pub contents: RawBindingContents,
}

/// Shared wrapper world; frame views retain only binding tables in the contexts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct RawBindingArena {
    /// Physical allocations, including wrappers retained after name deletion.
    pub slots: VariableCellTable<RawBindingSlot>,
    /// Named variable slots to their raw wrapper identities.
    pub bindings: VariableCellTable<VariableCellKey>,
    /// Allocations retained by callable statics.
    pub pinned: VariableCellSet,
    /// Local fallback entries installed from a callable static table.
    pub static_bindings: VariableCellSet,
    /// Unique retained array roots whose original members are undergoing teardown.
    pub retiring_arrays: VariableCellSet,
    /// Members retired before their frozen callbacks; indices are literal keys,
    /// separate from the typed retained-root allocation identity.
    pub retired_array_members: VariableCellTable<BTreeSet<String>>,
}

impl RawBindingArena {
    pub(crate) fn active_array_member(&self, receiver: &Place) -> bool {
        let Some(CellOwner::RetainedSlot(identity)) =
            receiver.cell.as_ref().map(|cell| &cell.owner)
        else {
            return false;
        };
        let key = identity.storage_key();
        receiver.index.is_some() && self.retiring_arrays.contains(&key)
            && self.slots.get(&key).is_some_and(|slot| {
                matches!(&slot.contents, RawBindingContents::Direct(root)
                    if root.kind == crate::place::PlaceKind::ArrayWhole && root.cell == receiver.cell)
            })
    }

    /// Follow an existing direct element alias to its uniquely retained old root.
    /// Callable statics and selected-name wrappers do not supply this identity.
    pub(crate) fn retained_array_member(&self, original: &Place) -> Option<Place> {
        original.index.as_ref()?;
        let cell = original.cell.as_ref()?;
        let key = RawBindingSlotId::Variable(Box::new(cell.clone())).storage_key();
        let slot = self.slots.get(&key)?;
        if slot.identity != RawBindingSlotId::Variable(Box::new(cell.clone())) {
            return None;
        }
        let RawBindingContents::Direct(root) = &slot.contents else {
            return None;
        };
        if root.kind != crate::place::PlaceKind::ArrayWhole {
            return None;
        }
        let mut member = original.clone();
        member.cell.clone_from(&root.cell);
        Some(member)
    }

    /// Publish a wrapper allocation. Conflicting identities never become equality proof.
    pub fn insert(&mut self, slot: RawBindingSlot) -> VariableCellKey {
        let key = slot.identity.storage_key();
        if let Some(existing) = self.slots.get_mut(&key) {
            if *existing != slot {
                existing.contents = RawBindingContents::Unknown;
            }
        } else {
            self.slots.insert(key.clone(), slot);
        }
        key
    }

    /// Join wrapper contents and named-slot ownership across executable paths.
    pub fn join(&mut self, other: &Self) {
        for (key, slot) in &mut self.slots {
            if other.slots.get(key) != Some(slot) {
                slot.contents = RawBindingContents::Unknown;
            }
        }
        for (key, slot) in &other.slots {
            self.slots
                .entry(key.clone())
                .or_insert_with(|| RawBindingSlot {
                    identity: slot.identity.clone(),
                    contents: RawBindingContents::Unknown,
                });
        }
        self.bindings
            .retain(|key, value| other.bindings.get(key) == Some(value));
        self.pinned.extend(other.pinned.iter().cloned());
        self.static_bindings
            .retain(|key| other.static_bindings.contains(key) && self.bindings.contains_key(key));
        self.retiring_arrays.retain(|key| {
            other.retiring_arrays.contains(key) && self.slots.get(key) == other.slots.get(key)
        });
        for (key, members) in &mut self.retired_array_members {
            if other.retired_array_members.get(key) != Some(members) {
                self.retiring_arrays.remove(key);
                if let Some(slot) = self.slots.get_mut(key) {
                    slot.contents = RawBindingContents::Unknown;
                }
            }
        }
        for (key, members) in &other.retired_array_members {
            self.retired_array_members
                .entry(key.clone())
                .or_insert_with(|| members.clone());
        }
    }

    /// Resolve a retained wrapper against the current logical frame stack.
    #[must_use]
    pub fn resolve(
        &self,
        key: &VariableCellKey,
        context: &ResolveContext,
        registry: &tcl_registry::CommandRegistry,
    ) -> Place {
        let mut current = key.clone();
        let mut visited = VariableCellSet::default();
        loop {
            if !visited.insert(current.clone()) {
                return place::unknown_top();
            }
            match self.slots.get(&current).map(|slot| &slot.contents) {
                Some(RawBindingContents::Direct(target)) => return (**target).clone(),
                Some(RawBindingContents::SelectedName { level, name }) => {
                    let Some(frame) =
                        context.selected_frame_context(tcl_registry::FrameLevel::Absolute(*level))
                    else {
                        return place::unknown_top();
                    };
                    let mut selected = frame;
                    selected.raw_bindings = self.clone();
                    let slot = crate::var_resolve::resolve_alias_destination_slot(
                        name, &selected, registry,
                    );
                    if let Some(next) = self.bindings.get(&crate::var_resolve::cell_key(&slot)) {
                        current.clone_from(next);
                    } else {
                        selected.raw_bindings = Self::default();
                        return crate::var_resolve::resolve_literal_place(
                            name, &selected, false, registry,
                        );
                    }
                }
                _ => return place::unknown_top(),
            }
        }
    }

    /// Relocate the complete retained world without erasing external allocations.
    #[must_use]
    pub fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        let slots = self
            .slots
            .values()
            .map(|slot| {
                let identity = slot.identity.relocated(relocation);
                let contents = match &slot.contents {
                    RawBindingContents::Direct(target) => {
                        RawBindingContents::Direct(Box::new(relocation.place(target)))
                    }
                    other => other.clone(),
                };
                (
                    identity.storage_key(),
                    RawBindingSlot { identity, contents },
                )
            })
            .collect();
        let relocate_id = |key: &VariableCellKey| {
            self.slots.get(key).map_or_else(
                || relocation.cell_key(key),
                |slot| slot.identity.relocated(relocation).storage_key(),
            )
        };
        Self {
            slots,
            bindings: self
                .bindings
                .iter()
                .map(|(name, key)| (relocation.cell_key(name), relocate_id(key)))
                .collect(),
            pinned: self.pinned.iter().map(relocate_id).collect(),
            static_bindings: self
                .static_bindings
                .iter()
                .map(|key| relocation.cell_key(key))
                .collect(),
            retiring_arrays: self.retiring_arrays.iter().map(relocate_id).collect(),
            retired_array_members: self
                .retired_array_members
                .iter()
                .map(|(key, members)| (relocate_id(key), members.clone()))
                .collect(),
        }
    }
}

/// Static local names installed before formal parameter assignments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CapturedStaticBindings {
    /// Evaluated local name and retained wrapper allocation.
    pub bindings: Vec<(String, VariableCellKey)>,
}

/// Definition-time static preparation preserves possible native errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticCaptureResult {
    /// Successful definition installs these retained wrappers.
    Prepared {
        /// Captured static bindings.
        bindings: CapturedStaticBindings,
        /// Incoming contents can still make definition fail.
        may_error: bool,
    },
    /// A definitely missing copy/capture cannot define the callable.
    Invalid,
    /// Selected dialect, callable allocation or frame cannot prove this storage contract.
    Unknown,
}

impl ResolveContext {
    /// Prepare static wrappers at the definition's actual variable point.
    /// Literal/copy statics allocate private storage; `&name` retains a raw wrapper.
    pub fn capture_callable_statics(
        &mut self,
        allocation: &CallableStorageIdentity,
        declarations: &[tcl_registry::native_procedure::StaticVariableDeclaration],
        registry: &tcl_registry::CommandRegistry,
    ) -> StaticCaptureResult {
        use tcl_registry::native_procedure::StaticVariableInitialiser as Initialiser;
        if self
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::family)
            != Some(tcl_dialect::model::Family::Jim)
            || allocation.incarnation == AllocationIncarnation::RepeatedFresh
        {
            return StaticCaptureResult::Unknown;
        }
        let mut candidate = self.clone();
        let mut bindings = CapturedStaticBindings::default();
        let mut may_error = false;
        for declaration in declarations {
            let captured = match &declaration.initialiser {
                Initialiser::CaptureCurrentCell(name) => candidate.capture_raw_slot(name, registry),
                Initialiser::CopyCurrent(name) => {
                    let target = crate::var_resolve::resolve_literal_place(
                        name, &candidate, false, registry,
                    );
                    match candidate.contents_presence(&target) {
                        crate::var_resolve::ContentsPresence::Undefined => {
                            return StaticCaptureResult::Invalid;
                        }
                        crate::var_resolve::ContentsPresence::Unknown
                        | crate::var_resolve::ContentsPresence::DefinedOrUndefined => {
                            may_error = true;
                        }
                        crate::var_resolve::ContentsPresence::Defined => {}
                    }
                    let value = candidate.literal_value(name, registry).map(str::to_owned);
                    let representation = candidate.contents_representation_at(&target);
                    Some(candidate.allocate_private_static(
                        allocation,
                        &declaration.name,
                        value.as_deref(),
                        representation,
                    ))
                }
                Initialiser::Literal(value) => Some(candidate.allocate_private_static(
                    allocation,
                    &declaration.name,
                    Some(value),
                    tcl_syntax::value::ValueRepresentation::Unknown,
                )),
            };
            let Some((key, possible_error)) = captured else {
                return StaticCaptureResult::Invalid;
            };
            may_error |= possible_error;
            candidate.raw_bindings.pinned.insert(key.clone());
            bindings.bindings.push((declaration.name.clone(), key));
        }
        *self = candidate;
        StaticCaptureResult::Prepared {
            bindings,
            may_error,
        }
    }

    fn allocate_private_static(
        &mut self,
        allocation: &CallableStorageIdentity,
        name: &str,
        value: Option<&str>,
        representation: tcl_syntax::value::ValueRepresentation,
    ) -> (VariableCellKey, bool) {
        let identity = RawBindingSlotId::Callable {
            allocation: allocation.clone(),
            name: name.to_owned(),
        };
        let mut target = place::scalar(name, place::LOCAL_NS, false);
        target.cell = Some(CellIdentity {
            owner: CellOwner::RetainedSlot(Box::new(identity.clone())),
            name: name.to_owned(),
            generation: crate::place::CellGeneration::Incoming,
            interpreter: self.interpreter.clone(),
            storage_domain: None,
            execution: self.execution,
        });
        let key = self.raw_bindings.insert(RawBindingSlot {
            identity,
            contents: RawBindingContents::Direct(Box::new(target.clone())),
        });
        let contents = crate::var_resolve::canonical_binding_value_key(&target).unwrap();
        self.record_presence_slot(&target);
        self.contents_presence.insert(
            contents.clone(),
            crate::var_resolve::ContentsPresence::Defined,
        );
        self.contents_kinds.insert(
            contents.clone(),
            crate::var_resolve::RootContentsKind::Scalar,
        );
        self.contents_origins.insert(
            contents.clone(),
            crate::var_resolve::ContentsOrigin::Incoming,
        );
        if let Some(value) = value {
            self.store_literal_value(contents.clone(), value.to_owned());
        }
        if representation != tcl_syntax::value::ValueRepresentation::Unknown {
            self.value_representations
                .insert(contents, representation.into());
        }
        (key, false)
    }

    fn capture_raw_slot(
        &mut self,
        name: &str,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(VariableCellKey, bool)> {
        let slot = crate::var_resolve::resolve_alias_destination_slot(name, self, registry);
        let slot_key = crate::var_resolve::cell_key(&slot);
        if let Some(existing) = self.raw_bindings.bindings.get(&slot_key) {
            return Some((existing.clone(), false));
        }
        let target = crate::var_resolve::resolve_literal_place(name, self, false, registry);
        let presence = self.contents_presence(&target);
        if presence == crate::var_resolve::ContentsPresence::Undefined {
            return None;
        }
        let identity = RawBindingSlotId::Variable(Box::new(slot.cell.clone()?));
        let contents = if canonical_place_key(&slot).is_none() {
            RawBindingContents::Unknown
        } else if let Some(link) = self
            .namespace_name_alias_bindings
            .get(&slot_key)
            .or_else(|| self.name_alias_bindings.get(name))
        {
            self.raw_name_link(link)
        } else if self.namespace_alias_bindings.contains_key(&slot_key)
            || self.alias_bindings.contains_key(name)
        {
            RawBindingContents::Unknown
        } else {
            let mut retained = target.clone();
            retained.cell.as_mut()?.owner = CellOwner::RetainedSlot(Box::new(identity.clone()));
            retained.cell.as_mut()?.generation = crate::place::CellGeneration::Incoming;
            self.copy_retained_contents(&target, &retained);
            RawBindingContents::Direct(Box::new(retained))
        };
        let key = self
            .raw_bindings
            .insert(RawBindingSlot { identity, contents });
        self.raw_bindings.bindings.insert(slot_key, key.clone());
        Some((
            key,
            presence != crate::var_resolve::ContentsPresence::Defined,
        ))
    }

    pub(crate) fn copy_retained_contents(&mut self, original: &Place, retained: &Place) {
        self.record_presence_slot(retained);
        let Some(original) = crate::var_resolve::canonical_binding_value_key(original) else {
            return;
        };
        let Some(retained) = crate::var_resolve::canonical_binding_value_key(retained) else {
            return;
        };
        if let Some(value) = self.constant_values.get(&original).cloned() {
            self.store_literal_value(retained.clone(), value);
        }
        if let Some(value) = self.closed_literal_contents.get(&original).cloned() {
            let target = self.contents_presence_slots.get(&retained);
            if let Some(target) = target {
                self.closed_literal_contents
                    .insert(retained.clone(), value.at_receiver(target));
            }
        }
        if let Some(representation) = self.value_representations.get(&original).cloned() {
            self.value_representations
                .insert(retained.clone(), representation);
        }
        if let Some(presence) = self.contents_presence.get(&original).copied() {
            self.contents_presence.insert(retained.clone(), presence);
        }
        if let Some(kind) = self.contents_kinds.get(&original).copied() {
            self.contents_kinds.insert(retained.clone(), kind);
        }
        self.contents_source_proofs.copy_slot(&original, &retained);
        if let Some(origin) = self.contents_origins.get(&original).cloned() {
            self.contents_origins.insert(retained, origin);
        }
    }

    fn raw_name_link(&self, target: &Place) -> RawBindingContents {
        if target.index.is_some() || target.dynamic {
            return RawBindingContents::Unknown;
        }
        let level = match target.cell.as_ref().map(|cell| &cell.owner) {
            Some(CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_)) => Some(0),
            Some(CellOwner::Activation(identity)) => self.absolute_activation_level(identity),
            _ => None,
        };
        let Some(level) = level else {
            return RawBindingContents::Unknown;
        };
        let name = match target.cell.as_ref().map(|cell| &cell.owner) {
            Some(CellOwner::NamespaceIdentity(namespace)) => {
                // Jim namespace variables occupy the flat root table. The
                // exact root receipt permits an absolute name for that flat
                // key; a C tree namespace or its display cannot supply it.
                if !namespace
                    .exact_native_path()
                    .is_some_and(tcl_core_types::ByteNamespacePath::is_root)
                {
                    return RawBindingContents::Unknown;
                }
                tcl_syntax::naming::qualify("::", &target.name)
            }
            _ if target.is_global() => tcl_syntax::naming::qualify(&target.ns, &target.name),
            _ => target.name.clone(),
        };
        RawBindingContents::SelectedName { level, name }
    }

    fn absolute_activation_level(&self, identity: &str) -> Option<u32> {
        let mut chain = Vec::new();
        let mut current = self;
        while !current.global_frame() {
            chain.push(current.activation.as_deref()?);
            current = current.caller.as_deref()?;
        }
        chain
            .iter()
            .rev()
            .position(|activation| *activation == identity)
            .and_then(|index| u32::try_from(index).ok()?.checked_add(1))
    }

    /// Install retained statics before assigning actual/default formal arguments.
    pub fn install_callable_statics(
        &mut self,
        statics: &CapturedStaticBindings,
        registry: &tcl_registry::CommandRegistry,
    ) {
        for (name, wrapper) in &statics.bindings {
            // Jim absolute names bypass the procedure static table.
            if name.starts_with("::") {
                continue;
            }
            let slot = crate::var_resolve::resolve_alias_destination_slot(name, self, registry);
            if slot.cell.is_none() || !self.raw_bindings.slots.contains_key(wrapper) {
                self.unknown_bindings.insert(name.clone());
            } else {
                let named = crate::var_resolve::cell_key(&slot);
                self.raw_bindings.static_bindings.insert(named.clone());
                self.raw_bindings.bindings.insert(named, wrapper.clone());
            }
        }
    }

    /// Retarget the existing wrapper, so an earlier raw capture observes its new link.
    pub fn retarget_raw_binding(
        &mut self,
        name: &str,
        target: &Place,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let slot = crate::var_resolve::resolve_alias_destination_slot(name, self, registry);
        let named = crate::var_resolve::cell_key(&slot);
        let Some(key) = self.raw_bindings.bindings.get(&named).cloned() else {
            return;
        };
        let contents = self.raw_name_link(target);
        if let Some(wrapper) = self.raw_bindings.slots.get_mut(&key) {
            wrapper.contents = contents;
        }
    }

    /// Prove Jim's direct static fallback cannot be removed from the local
    /// variable table. A linked wrapper instead delegates unset to its target.
    #[must_use]
    pub fn raw_static_unset_error(
        &self,
        name: &str,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        let slot = crate::var_resolve::resolve_alias_destination_slot(name, self, registry);
        let named = crate::var_resolve::cell_key(&slot);
        self.raw_bindings.static_bindings.contains(&named)
            && self
                .raw_bindings
                .bindings
                .get(&named)
                .and_then(|key| self.raw_bindings.slots.get(key))
                .is_some_and(|slot| matches!(slot.contents, RawBindingContents::Direct(_)))
    }

    /// Detach a pinned named scalar without destroying the retained direct `VarVal`.
    /// Array element deletion and observer callbacks require the ordinary mutation path.
    pub fn detach_retained_binding(
        &mut self,
        name: &str,
        source: u32,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        let slot = crate::var_resolve::resolve_alias_destination_slot(name, self, registry);
        if slot.index.is_some() || slot.observed {
            return false;
        }
        let named = crate::var_resolve::cell_key(&slot);
        if self.raw_bindings.static_bindings.contains(&named) {
            return false;
        }
        let Some(key) = self.raw_bindings.bindings.get(&named) else {
            return false;
        };
        if !self.raw_bindings.pinned.contains(key)
            || !matches!(
                self.raw_bindings.slots.get(key),
                Some(RawBindingSlot {
                    identity: RawBindingSlotId::Variable(_),
                    contents: RawBindingContents::Direct(_),
                    ..
                })
            )
        {
            return false;
        }
        self.raw_bindings.bindings.remove(&named);
        self.alias_bindings.remove(name);
        self.name_alias_bindings.remove(name);
        self.namespace_alias_bindings.remove(&named);
        self.namespace_name_alias_bindings.remove(&named);
        self.generations
            .insert(named.clone(), crate::place::CellGeneration::After(source));
        self.forget_literal_value(&named);
        self.value_representations.remove(&named);
        self.contents_presence
            .insert(named, crate::var_resolve::ContentsPresence::Undefined);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::var_resolve::{VariableExecutionFrame, restore_execution_frame};

    #[test]
    fn retained_wrappers_keep_namespace_incarnation_and_component_boundaries() {
        use crate::command_binding::SourceNamespaceKey;
        use tcl_core_types::ByteNamespacePath;
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeNamespaceContext,
        };

        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let context = root(registry);
        let mut arena = RawBindingArena::default();
        let paths = [
            ByteNamespacePath::from_segments([b"A".as_slice(), b":q".as_slice()]),
            ByteNamespacePath::from_segments([b"A:".as_slice(), b"q".as_slice()]),
        ];
        let mut keys = Vec::new();
        for (token, path) in paths.into_iter().enumerate() {
            let namespace = SourceNamespaceKey::Native(NativeNamespaceContext {
                interpreter: NativeInterpreterIdentity {
                    owner: 10,
                    interpreter: 0,
                },
                token: token as u64 + 1,
                path,
            });
            let mut original = place::scalar("x", namespace.display().unwrap(), false);
            original.cell = Some(CellIdentity {
                owner: CellOwner::NamespaceIdentity(Box::new(namespace)),
                name: "x".into(),
                generation: crate::place::CellGeneration::Incoming,
                interpreter: None,
                storage_domain: None,
                execution: None,
            });
            let identity = RawBindingSlotId::Variable(Box::new(original.cell.clone().unwrap()));
            let key = arena.insert(RawBindingSlot {
                identity,
                contents: RawBindingContents::Direct(Box::new(original.clone())),
            });
            assert_eq!(arena.resolve(&key, &context, registry), original);
            assert!(!arena.slots.contains_key(&key.compatibility_name()));
            keys.push(key);
        }
        assert_ne!(keys[0], keys[1]);
        assert_eq!(arena.slots.len(), 2);
        let left = arena.resolve(&keys[0], &context, registry);
        let right = arena.resolve(&keys[1], &context, registry);
        assert_eq!(left.ns, right.ns);
        assert_ne!(left.cell, right.cell);
    }

    fn allocation() -> CallableStorageIdentity {
        CallableStorageIdentity {
            site: CommandAllocationSite {
                source: std::sync::Arc::new(crate::command_binding::SourceOriginId::authored(
                    &"statics".into(),
                )),
                offset: 10,
            },
            incarnation: AllocationIncarnation::First,
            implementation_generation: 1,
        }
    }

    fn root(registry: &tcl_registry::CommandRegistry) -> ResolveContext {
        let mut context = ResolveContext::for_function("::top");
        context.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        context
    }

    fn entered(context: &ResolveContext, identity: &str) -> ResolveContext {
        context.enter_called_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".into(),
            identity: identity.into(),
        })
    }

    fn capture(
        context: &mut ResolveContext,
        declarations: &str,
        registry: &tcl_registry::CommandRegistry,
    ) -> CapturedStaticBindings {
        let declarations =
            tcl_registry::native_procedure::parse_static_variables(declarations).unwrap();
        match context.capture_callable_statics(&allocation(), &declarations, registry) {
            StaticCaptureResult::Prepared {
                bindings,
                may_error: false,
            } => bindings,
            result => panic!("unproved preparation: {result:?}"),
        }
    }

    #[test]
    fn direct_capture_survives_unset_and_named_recreation() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut context = root(registry);
        context.define_literal("x", "OLD", registry);
        let statics = capture(&mut context, "&x", registry);
        assert!(context.detach_retained_binding("x", 20, registry));
        context.define_literal("x", "NEW", registry);
        let mut call = entered(&context, "p");
        call.install_callable_statics(&statics, registry);
        assert_eq!(call.literal_value("x", registry), Some("OLD"));
        assert_eq!(context.literal_value("x", registry), Some("NEW"));
    }

    #[test]
    fn copied_static_retains_representation_until_shared_coercion() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut context = root(registry);
        context.define_literal_with_representation(
            "x",
            "3",
            tcl_syntax::value::ValueRepresentation::List,
            registry,
        );
        let statics = capture(&mut context, "x", registry);
        let mut call = entered(&context, "p");
        call.install_callable_statics(&statics, registry);
        let retained = crate::var_resolve::resolve_literal_place("x", &call, false, registry);
        assert_eq!(
            call.contents_representation_at(&retained),
            tcl_syntax::value::ValueRepresentation::List
        );
        call.invalidate_shared_representations();
        let restored = restore_execution_frame(&context, &call);
        let mut later = entered(&restored, "p-later");
        later.install_callable_statics(&statics, registry);
        let retained = crate::var_resolve::resolve_literal_place("x", &later, false, registry);
        assert_eq!(later.literal_contents_at(&retained, registry), Some("3"));
        assert_eq!(
            later.contents_representation_at(&retained),
            tcl_syntax::value::ValueRepresentation::Unknown
        );
    }

    #[test]
    fn formal_assignment_updates_private_static_before_later_calls() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut context = root(registry);
        let statics = capture(&mut context, "{x STATIC}", registry);
        let mut first = entered(&context, "p-first");
        first.install_callable_statics(&statics, registry);
        first.define_literal("x", "PARAM", registry);
        let restored = restore_execution_frame(&context, &first);
        let mut second = entered(&restored, "p-second");
        second.install_callable_statics(&statics, registry);
        assert_eq!(second.literal_value("x", registry), Some("PARAM"));
    }

    #[test]
    fn captured_link_observes_retargeting_of_the_raw_wrapper() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut context = root(registry);
        context.define_literal("a", "OLD", registry);
        context.define_literal("b", "NEW", registry);
        let original = crate::var_resolve::resolve_literal_place("a", &context, false, registry);
        context.name_alias_bindings.insert("x", original);
        let statics = capture(&mut context, "&x", registry);
        let replacement = crate::var_resolve::resolve_literal_place("b", &context, false, registry);
        context.retarget_raw_binding("x", &replacement, registry);
        let mut call = entered(&context, "p");
        call.install_callable_statics(&statics, registry);
        assert_eq!(call.literal_value("x", registry), Some("NEW"));
    }

    #[test]
    fn escaped_name_link_uses_current_logical_level_not_retired_activation() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let context = root(registry);
        let mut maker = entered(&context, "maker");
        maker.define_literal("a", "KEEP", registry);
        let target = crate::var_resolve::resolve_literal_place("a", &maker, false, registry);
        maker.name_alias_bindings.insert("x", target);
        let statics = capture(&mut maker, "&x", registry);
        let restored = restore_execution_frame(&context, &maker);
        let mut other = entered(&restored, "other");
        other.define_literal("a", "OTHER", registry);
        let mut call = entered(&other, "p");
        call.install_callable_statics(&statics, registry);
        assert_eq!(call.literal_value("x", registry), Some("OTHER"));
        let mut lexical = restored.in_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".into(),
            identity: "unproved-entry".into(),
        });
        lexical.install_callable_statics(&statics, registry);
        assert_eq!(lexical.literal_value("x", registry), None);
    }
}
