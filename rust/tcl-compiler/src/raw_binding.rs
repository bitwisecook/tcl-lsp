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
use tcl_core_types::NameBytes;

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
        /// Counted static-table name, independent of its source presentation.
        name: NameBytes,
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
                let mut original = place::scalar(
                    cell.name.try_utf8().unwrap_or_default(),
                    place::LOCAL_NS,
                    false,
                );
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
        /// Already evaluated counted target, resolved at the retained logical level.
        name: NameBytes,
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
    pub retired_array_members: VariableCellTable<BTreeSet<tcl_core_types::NameBytes>>,
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
                    let slot = raw_destination_bytes(name.as_bytes(), &selected, registry);
                    if let Some(next) = self.bindings.get(&crate::var_resolve::cell_key(&slot)) {
                        current.clone_from(next);
                    } else {
                        selected.raw_bindings = Self::default();
                        return raw_value_bytes(name.as_bytes(), &selected, registry);
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

fn raw_destination_bytes(
    name: &[u8],
    context: &ResolveContext,
    registry: &tcl_registry::CommandRegistry,
) -> Place {
    if context.execution_name_policy.is_some() {
        crate::var_resolve::resolve_original_alias_destination_bytes(name, context, registry)
    } else {
        std::str::from_utf8(name).map_or_else(
            |_| place::unknown_top(),
            |name| crate::var_resolve::resolve_alias_destination_slot(name, context, registry),
        )
    }
}

fn raw_value_bytes(
    name: &[u8],
    context: &ResolveContext,
    registry: &tcl_registry::CommandRegistry,
) -> Place {
    if context.execution_name_policy.is_some() {
        crate::var_resolve::resolve_evaluated_variable_input(
            tcl_syntax::naming::NativeVariableInputForm::Combined(name),
            context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        )
    } else {
        std::str::from_utf8(name).map_or_else(
            |_| place::unknown_top(),
            |name| crate::var_resolve::resolve_literal_place(name, context, false, registry),
        )
    }
}

/// Static local names installed before formal parameter assignments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct CapturedStaticBindings {
    /// Evaluated local name and retained wrapper allocation.
    pub bindings: Vec<(NameBytes, VariableCellKey)>,
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
                        &NameBytes::from(declaration.name.as_str()),
                        value.as_deref(),
                        representation,
                    ))
                }
                Initialiser::Literal(value) => Some(candidate.allocate_private_static(
                    allocation,
                    &NameBytes::from(declaration.name.as_str()),
                    Some(value),
                    tcl_syntax::value::ValueRepresentation::Unknown,
                )),
            };
            let Some((key, possible_error)) = captured else {
                return StaticCaptureResult::Invalid;
            };
            may_error |= possible_error;
            candidate.raw_bindings.pinned.insert(key.clone());
            bindings
                .bindings
                .push((declaration.name.as_str().into(), key));
        }
        *self = candidate;
        StaticCaptureResult::Prepared {
            bindings,
            may_error,
        }
    }

    /// Capture the actual Jim static-list operand at the definition point.
    /// Its list children retain original producers; copied contents and raw
    /// wrappers have separate lifetime owners. Preparation remains conditional
    /// on the native definition succeeding and supplies no Normal certificate.
    pub(crate) fn capture_original_callable_statics(
        &mut self,
        allocation: &CallableStorageIdentity,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> StaticCaptureResult {
        let Some(dialect) = self.invocation_dialect else {
            return StaticCaptureResult::Unknown;
        };
        if self
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            != Some(input.policy())
            || !input.policy().recipe().is_jim084()
            || !input.is_current(self)
            || allocation.incarnation == AllocationIncarnation::RepeatedFresh
        {
            return StaticCaptureResult::Unknown;
        }
        let declarations = match tcl_registry::native_procedure::parse_static_variables_bytes(
            input.bytes(),
            dialect,
        ) {
            Some(Ok(declarations)) => declarations,
            Some(Err(_)) => return StaticCaptureResult::Invalid,
            None => return StaticCaptureResult::Unknown,
        };
        let Some(children) = input.original_list_elements() else {
            return StaticCaptureResult::Unknown;
        };
        if declarations.len() != children.len() {
            return StaticCaptureResult::Unknown;
        }
        let mut candidate = self.clone();
        let mut bindings = CapturedStaticBindings::default();
        let mut may_error = false;
        for (declaration, child) in declarations.iter().zip(&children) {
            let (name, key, possible_error) = match candidate.capture_original_static_declaration(
                allocation,
                declaration,
                child,
                registry,
            ) {
                Ok(binding) => binding,
                Err(outcome) => return outcome,
            };
            may_error |= possible_error;
            candidate.raw_bindings.pinned.insert(key.clone());
            bindings.bindings.push((name, key));
        }
        *self = candidate;
        StaticCaptureResult::Prepared {
            bindings,
            may_error,
        }
    }

    fn capture_original_static_declaration(
        &mut self,
        allocation: &CallableStorageIdentity,
        declaration: &tcl_registry::native_procedure::ByteStaticVariableDeclaration,
        child: &crate::signature_scan::scope::SignatureSourceNameInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Result<(NameBytes, VariableCellKey, bool), StaticCaptureResult> {
        use tcl_registry::native_procedure::StaticVariableInitialiser as Initialiser;
        let fields = child
            .original_list_elements()
            .ok_or(StaticCaptureResult::Unknown)?;
        let first = fields.first().ok_or(StaticCaptureResult::Unknown)?;
        let name = NameBytes::from(declaration.name.as_slice());
        let selected_name = match declaration.initialiser {
            Initialiser::CaptureCurrentCell(_) => first.bytes().strip_prefix(b"&"),
            _ => Some(first.bytes()),
        };
        // The shared grammar selected the marker/name role. This byte
        // correspondence authenticates its actual child, not a new Word.
        if selected_name != Some(name.as_bytes())
            || fields.iter().any(|field| !field.is_current(self))
        {
            return Err(StaticCaptureResult::Unknown);
        }
        let (key, possible_error) = match &declaration.initialiser {
            Initialiser::CopyCurrent(_) | Initialiser::CaptureCurrentCell(_) => self
                .capture_original_static_variable(
                    allocation,
                    &name,
                    matches!(declaration.initialiser, Initialiser::CaptureCurrentCell(_)),
                    registry,
                )?,
            Initialiser::Literal(value) => {
                let input = fields.get(1).ok_or(StaticCaptureResult::Unknown)?;
                if fields.len() != 2 || input.bytes() != value {
                    return Err(StaticCaptureResult::Unknown);
                }
                let held = crate::command_binding::original_name_value::OriginalProducedNameValue::from_source_input(
                    input, self,
                ).ok_or(StaticCaptureResult::Unknown)?;
                let captured = self.allocate_private_static(
                    allocation,
                    &name,
                    std::str::from_utf8(held.bytes()).ok(),
                    tcl_syntax::value::ValueRepresentation::Unknown,
                );
                self.retain_private_static_value(&captured.0, Some(&held), registry);
                captured
            }
        };
        Ok((name, key, possible_error))
    }

    fn capture_original_static_variable(
        &mut self,
        allocation: &CallableStorageIdentity,
        name: &NameBytes,
        reference: bool,
        registry: &tcl_registry::CommandRegistry,
    ) -> Result<(VariableCellKey, bool), StaticCaptureResult> {
        if reference {
            return self.capture_original_static_reference(name, registry);
        }
        let target = raw_value_bytes(name.as_bytes(), self, registry);
        if target.kind == crate::place::PlaceKind::Unknown || target.dynamic {
            return Err(StaticCaptureResult::Unknown);
        }
        let observers =
            self.variable_observers_at(&target, tcl_registry::TraceOperation::Read, registry);
        if target.observed
            || observers.unknown_residual
            || !observers.callbacks.is_empty()
            || !observers.possible_callbacks.is_empty()
        {
            return Err(StaticCaptureResult::Unknown);
        }
        let presence = self.contents_presence(&target);
        if presence == crate::var_resolve::ContentsPresence::Undefined {
            return Err(StaticCaptureResult::Invalid);
        }
        let held = self
            .original_name_read_result(&target, registry)
            .map(|read| read.value().clone());
        let value = self
            .literal_contents_at(&target, registry)
            .map(str::to_owned);
        let representation = self.contents_representation_at(&target);
        let (key, _) =
            self.allocate_private_static(allocation, name, value.as_deref(), representation);
        self.retain_private_static_value(&key, held.as_ref(), registry);
        Ok((
            key,
            presence != crate::var_resolve::ContentsPresence::Defined,
        ))
    }

    fn capture_original_static_reference(
        &mut self,
        name: &NameBytes,
        registry: &tcl_registry::CommandRegistry,
    ) -> Result<(VariableCellKey, bool), StaticCaptureResult> {
        let slot = raw_destination_bytes(name.as_bytes(), self, registry);
        let key = crate::var_resolve::cell_key(&slot);
        if slot.kind == crate::place::PlaceKind::Unknown
            || slot.cell.is_none()
            || self.dynamic_bindings
            || self.unknown_bindings.contains(&key)
        {
            return Err(StaticCaptureResult::Unknown);
        }
        if let Some(existing) = self.raw_bindings.bindings.get(&key) {
            return if self.raw_bindings.slots.contains_key(existing) {
                Ok((existing.clone(), false))
            } else {
                Err(StaticCaptureResult::Unknown)
            };
        }
        // JimCreateProcedureStatics retains the VarVal selected by
        // SetVariableFromAny. It does not read an existing link's target.
        let link = self
            .namespace_name_alias_bindings
            .get(&key)
            .or_else(|| self.name_alias_bindings.get(&key))
            .cloned();
        let target = link.unwrap_or_else(|| raw_value_bytes(name.as_bytes(), self, registry));
        if target.kind == crate::place::PlaceKind::Unknown {
            return Err(StaticCaptureResult::Unknown);
        }
        self.capture_raw_slot_at(&slot, &target, None, registry)
            .ok_or(StaticCaptureResult::Invalid)
    }

    fn retain_private_static_value(
        &mut self,
        key: &VariableCellKey,
        value: Option<&crate::command_binding::original_name_value::OriginalProducedNameValue>,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let Some(value) = value else {
            return;
        };
        let receiver = match self.raw_bindings.slots.get(key).map(|slot| &slot.contents) {
            Some(RawBindingContents::Direct(receiver)) => (**receiver).clone(),
            _ => return,
        };
        self.retain_original_name_value(&receiver, value, registry);
    }

    fn allocate_private_static(
        &mut self,
        allocation: &CallableStorageIdentity,
        name: &NameBytes,
        value: Option<&str>,
        representation: tcl_syntax::value::ValueRepresentation,
    ) -> (VariableCellKey, bool) {
        let identity = RawBindingSlotId::Callable {
            allocation: allocation.clone(),
            name: name.clone(),
        };
        let mut target = place::scalar(name.try_utf8().unwrap_or_default(), place::LOCAL_NS, false);
        target.cell = Some(CellIdentity {
            owner: CellOwner::RetainedSlot(Box::new(identity.clone())),
            name: name.clone(),
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
        let target = crate::var_resolve::resolve_literal_place(name, self, false, registry);
        self.capture_raw_slot_at(&slot, &target, Some(name), registry)
    }

    fn capture_raw_slot_at(
        &mut self,
        slot: &Place,
        target: &Place,
        logical_name: Option<&str>,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(VariableCellKey, bool)> {
        let slot_key = crate::var_resolve::cell_key(slot);
        if let Some(existing) = self.raw_bindings.bindings.get(&slot_key) {
            return Some((existing.clone(), false));
        }
        let link = self
            .namespace_name_alias_bindings
            .get(&slot_key)
            .or_else(|| self.name_alias_bindings.get(&slot_key))
            .or_else(|| logical_name.and_then(|name| self.name_alias_bindings.get(name)))
            .cloned();
        let presence = self.contents_presence(target);
        if link.is_none() && presence == crate::var_resolve::ContentsPresence::Undefined {
            return None;
        }
        let identity = RawBindingSlotId::Variable(Box::new(slot.cell.clone()?));
        let contents = if canonical_place_key(slot).is_none() {
            RawBindingContents::Unknown
        } else if let Some(link) = &link {
            self.raw_name_link(link)
        } else if self.namespace_alias_bindings.contains_key(&slot_key)
            || self.alias_bindings.contains_key(&slot_key)
            || logical_name.is_some_and(|name| self.alias_bindings.contains_key(name))
        {
            RawBindingContents::Unknown
        } else {
            let mut retained = target.clone();
            retained.cell.as_mut()?.owner = CellOwner::RetainedSlot(Box::new(identity.clone()));
            retained.cell.as_mut()?.generation = crate::place::CellGeneration::Incoming;
            self.copy_retained_contents(target, &retained);
            RawBindingContents::Direct(Box::new(retained))
        };
        let held = self
            .original_name_read_result(target, registry)
            .map(|read| read.value().clone());
        let key = self
            .raw_bindings
            .insert(RawBindingSlot { identity, contents });
        self.retain_private_static_value(&key, held.as_ref(), registry);
        self.raw_bindings.bindings.insert(slot_key, key.clone());
        Some((
            key,
            link.is_none() && presence != crate::var_resolve::ContentsPresence::Defined,
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
        let Some(cell) = target.cell.as_ref() else {
            return RawBindingContents::Unknown;
        };
        let name = match &cell.owner {
            CellOwner::NamespaceIdentity(namespace) => {
                // Jim namespace variables occupy the flat root table. The
                // exact root receipt permits an absolute name for that flat
                // key; a C tree namespace or its display cannot supply it.
                if !namespace
                    .exact_native_path()
                    .is_some_and(tcl_core_types::ByteNamespacePath::is_root)
                {
                    return RawBindingContents::Unknown;
                }
                let mut absolute = b"::".to_vec();
                absolute.extend_from_slice(cell.name.as_bytes());
                absolute.into()
            }
            CellOwner::Namespace(namespace) if self.execution_name_policy.is_none() => {
                tcl_syntax::naming::qualify(namespace, &target.name).into()
            }
            CellOwner::Activation(_) => cell.name.clone(),
            _ => return RawBindingContents::Unknown,
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
            if name.as_bytes().starts_with(b"::") {
                continue;
            }
            let slot = raw_destination_bytes(name.as_bytes(), self, registry);
            if slot.cell.is_none() || !self.raw_bindings.slots.contains_key(wrapper) {
                self.dynamic_bindings = true;
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
        self.retarget_raw_binding_at_slot(&slot, target);
    }

    /// Retarget a separately selected direct slot without parsing a label or
    /// changing the identity of an already captured raw wrapper.
    pub(crate) fn retarget_raw_binding_at_slot(&mut self, slot: &Place, target: &Place) {
        let named = crate::var_resolve::cell_key(slot);
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
        self.raw_static_unset_error_at_slot(&slot)
    }

    /// Classify the separately selected named slot without reading its target.
    #[must_use]
    pub(crate) fn raw_static_unset_error_at_slot(&self, slot: &Place) -> bool {
        if slot.kind == place::PlaceKind::Unknown || slot.index.is_some() {
            return false;
        }
        let named = crate::var_resolve::cell_key(slot);
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
        if !self.detach_retained_binding_at_slot(&slot, source) {
            return false;
        }
        self.alias_bindings.remove(name);
        self.name_alias_bindings.remove(name);
        true
    }

    /// Detach one actual named slot while the independently pinned wrapper
    /// keeps its direct contents. A static-table fallback is not that slot.
    pub(crate) fn detach_retained_binding_at_slot(&mut self, slot: &Place, source: u32) -> bool {
        if slot.kind == place::PlaceKind::Unknown || slot.index.is_some() || slot.observed {
            return false;
        }
        let named = crate::var_resolve::cell_key(slot);
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
        self.alias_bindings.remove(&named);
        self.name_alias_bindings.remove(&named);
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

#[cfg(test)]
mod original_callable_static_tests {
    use super::*;
    use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
    use crate::var_resolve::{VariableExecutionFrame, restore_execution_frame};
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    use tcl_syntax::{naming::ExecutionNamePolicy, word_rules::WordValueRules};

    fn context() -> ResolveContext {
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap(),
        );
        let mut root = ResolveContext::default().in_frame(&VariableExecutionFrame::Global);
        root.invocation_dialect = Some(dialect);
        root.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(
            dialect.authored_name_policy().unwrap(),
        ));
        root.enter_called_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".into(),
            identity: "maker".into(),
        })
    }

    fn input(bytes: &[u8], context: &ResolveContext) -> SignatureSourceNameInput {
        let config = LexerConfig::from_grammar(context.invocation_dialect.unwrap().lexer_grammar);
        let image = SourceImage::native(bytes);
        let parsed = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(bytes.len()).unwrap()),
            config,
        )
        .unwrap();
        assert_eq!(parsed.commands.len(), 1);
        assert_eq!(parsed.commands[0].words.len(), 1);
        SignatureSourceNameInput::OriginalWord(
            SignatureSourceNameKey::from_original_native_word(
                &parsed.commands[0].words[0],
                WordValueRules::from_config(&config),
                context
                    .execution_name_policy
                    .unwrap()
                    .native_recipe()
                    .unwrap(),
            )
            .unwrap(),
        )
    }

    fn allocation(offset: u32) -> CallableStorageIdentity {
        CallableStorageIdentity {
            site: CommandAllocationSite {
                source: std::sync::Arc::new(crate::command_binding::SourceOriginId::authored(
                    &"static declarations".into(),
                )),
                offset,
            },
            incarnation: AllocationIncarnation::First,
            implementation_generation: 1,
        }
    }

    fn capture(context: &mut ResolveContext, bytes: &[u8], offset: u32) -> CapturedStaticBindings {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let operand = input(bytes, context);
        match context.capture_original_callable_statics(&allocation(offset), &operand, registry) {
            StaticCaptureResult::Prepared {
                bindings,
                may_error: false,
            } => bindings,
            other => panic!("unexpected capture: {other:?}"),
        }
    }

    fn store(context: &mut ResolveContext, name: &[u8], value: &[u8], offset: u32) {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let receiver = raw_value_bytes(name, context, registry);
        assert_eq!(receiver.kind, place::PlaceKind::Scalar);
        let value = crate::command_binding::original_name_value::OriginalProducedNameValue::from_source_input(
            &input(value, context), context,
        ).unwrap();
        context.record_contents_write(&receiver, offset, false);
        assert!(context.retain_original_name_value(&receiver, &value, registry));
    }

    fn read(context: &ResolveContext, name: &[u8]) -> Vec<u8> {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let receiver = raw_value_bytes(name, context, registry);
        let read = context.original_name_read_result(&receiver, registry).unwrap_or_else(|| {
            let owner = receiver.cell.as_ref().map(|cell| match cell.owner {
                CellOwner::RetainedSlot(_) => "retained",
                CellOwner::Activation(_) => "activation",
                CellOwner::NamespaceIdentity(_) => "namespace",
                _ => "other",
            });
            panic!("static read {name:?}: frame={:?} kind={:?} owner={owner:?} presence={:?} normal={} stored={} generation={:?} current={:?}",
                context.activation, receiver.kind, context.contents_presence(&receiver),
                context.read_produces_value(&receiver, registry),
                crate::var_resolve::canonical_binding_value_key(&receiver).is_some_and(|key| context.original_name_values.contains_key(&key)),
                receiver.cell.as_ref().map(|cell| cell.generation),
                context.generations.get(&crate::var_resolve::cell_key(&receiver)));
        });
        assert!(read.is_current(context, registry));
        read.value().bytes().to_vec()
    }

    fn called(
        context: &ResolveContext,
        statics: &CapturedStaticBindings,
        identity: &str,
    ) -> ResolveContext {
        let mut call = context.enter_called_frame(&VariableExecutionFrame::Procedure {
            namespace: "::".into(),
            identity: identity.into(),
        });
        call.install_callable_statics(
            statics,
            tcl_registry::model::ingress::static_context_for("jim").commands(),
        );
        call
    }

    #[test]
    fn original_static_literals_keep_counted_names_and_opaque_value_children() {
        // Implementation contract: naming.variable.original-callable-static-capture (docs/design/analysis/name-resolution-proofs/original-callable-static-capture.md).
        let mut context = context();
        let statics = capture(&mut context, b"{{x\0a FIRST} {x\0b \xff}}", 10);
        assert_eq!(statics.bindings.len(), 2);
        assert_eq!(statics.bindings[0].0.as_bytes(), b"x\0a");
        assert_eq!(statics.bindings[1].0.as_bytes(), b"x\0b");
        assert_ne!(statics.bindings[0].1, statics.bindings[1].1);
        let mut first = called(&context, &statics, "first");
        assert_eq!(read(&first, b"x\0a"), b"FIRST");
        assert_eq!(read(&first, b"x\0b"), b"\xff");
        store(&mut first, b"x\0a", b"SECOND", 20);
        first.invalidate_shared_representations();
        let restored = restore_execution_frame(&context, &first);
        let later = called(&restored, &statics, "later");
        assert_eq!(read(&later, b"x\0a"), b"SECOND");
        assert_eq!(read(&later, b"x\0b"), b"\xff");
    }

    #[test]
    fn original_static_copy_and_reference_have_distinct_content_lifetimes() {
        // Implementation contract: naming.variable.original-callable-static-capture (docs/design/analysis/name-resolution-proofs/original-callable-static-capture.md).
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut context = context();
        store(&mut context, b"x\0tail", b"OLD", 1);
        let copied = capture(&mut context, b"{x\0tail}", 10);
        let linked = capture(&mut context, b"{&x\0tail}", 20);
        assert_ne!(copied.bindings[0].1, linked.bindings[0].1);
        store(&mut context, b"x\0tail", b"NEW", 2);
        assert_eq!(read(&called(&context, &copied, "copy"), b"x\0tail"), b"OLD");
        assert_eq!(
            read(&called(&context, &linked, "reference"), b"x\0tail"),
            b"NEW"
        );
        store(&mut context, b"other\0tail", b"TARGET", 3);
        let target = raw_value_bytes(b"other\0tail", &context, registry);
        let target_cell = target.cell.clone();
        let slot = raw_destination_bytes(b"x\0tail", &context, registry);
        context.retarget_raw_binding_at_slot(&slot, &target);
        let retargeted = called(&context, &linked, "retargeted");
        assert_eq!(
            raw_value_bytes(b"x\0tail", &retargeted, registry).cell,
            target_cell
        );
        assert_eq!(read(&retargeted, b"x\0tail"), b"TARGET");
        assert_eq!(
            read(&called(&context, &copied, "copied-later"), b"x\0tail"),
            b"OLD"
        );
    }

    #[test]
    fn original_reference_capture_owns_wrapper_presence_before_link_target_contents() {
        // Implementation contract: naming.variable.original-callable-static-capture (docs/design/analysis/name-resolution-proofs/original-callable-static-capture.md).
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut context = context();
        let slot = raw_destination_bytes(b"linked", &context, registry);
        let target = raw_value_bytes(b"missing", &context, registry);
        assert_eq!(
            context.contents_presence(&target),
            crate::var_resolve::ContentsPresence::Undefined
        );
        context.record_presence_slot(&slot);
        context
            .name_alias_bindings
            .insert(crate::var_resolve::cell_key(&slot), target);
        let copied_input = input(b"{linked}", &context);
        let before = context.clone();
        assert_eq!(
            context.capture_original_callable_statics(&allocation(10), &copied_input, registry),
            StaticCaptureResult::Invalid
        );
        assert_eq!(context, before);
        let linked = capture(&mut context, b"{&linked}", 20);
        store(&mut context, b"missing", b"LATER", 1);
        let target_cell = raw_value_bytes(b"missing", &context, registry).cell;
        let later = called(&context, &linked, "later");
        assert_eq!(
            raw_value_bytes(b"linked", &later, registry).cell,
            target_cell
        );
        assert_eq!(read(&later, b"linked"), b"LATER");
    }

    #[test]
    fn original_static_unset_preserves_fallback_and_detaches_only_the_named_wrapper() {
        // Implementation contract: naming.variable.original-callable-static-capture (docs/design/analysis/name-resolution-proofs/original-callable-static-capture.md).
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let name = b"x\0tail";
        let mut maker = context();
        store(&mut maker, name, b"OLD", 1);
        let retained = capture(&mut maker, b"{&x\0tail}", 10);
        let words = [tcl_registry::InvocationWord::KnownBytes(name)];
        let arguments = tcl_registry::InvocationArguments::structured(&words)
            .with_dialect(maker.invocation_dialect.unwrap());
        let facts = registry
            .resolve_structured_invocation(
                tcl_registry::InvocationWords::from_arguments(
                    tcl_registry::InvocationWord::Literal("unset"),
                    arguments,
                ),
                maker.invocation_dialect.unwrap().authoring_query(),
            )
            .resolved()
            .unwrap()
            .facts();
        let operands = crate::variable_bindings::OriginalVariableInvocation::from_original_inputs(
            vec![Some(input(name, &maker))],
            Vec::new(),
        );
        crate::variable_bindings::transfer_source_namespace_cells_with_input(
            &mut maker,
            &facts,
            arguments,
            registry,
            crate::variable_bindings::SourceNamespaceTransfer::new(false, 20, Some(&[0]))
                .with_original_operands(&operands),
        );
        let named = raw_value_bytes(name, &maker, registry);
        assert_eq!(
            maker.contents_presence(&named),
            crate::var_resolve::ContentsPresence::Undefined
        );
        store(&mut maker, name, b"NEW", 21);
        assert_eq!(read(&maker, name), b"NEW");
        assert_eq!(read(&called(&maker, &retained, "held"), name), b"OLD");

        let fallback = capture(&mut maker, b"{{x\0tail KEEP}}", 30);
        let mut entered = called(&maker, &fallback, "fallback");
        let operands = crate::variable_bindings::OriginalVariableInvocation::from_original_inputs(
            vec![Some(input(name, &entered))],
            Vec::new(),
        );
        let slot = operands.raw_unset_slot(0, &entered, registry).unwrap();
        assert!(entered.raw_static_unset_error_at_slot(&slot));
        assert!(
            crate::variable_bindings::source_variable_write_places_with_original_operands(
                &facts, arguments, &entered, registry, &operands,
            )
            .is_empty()
        );
        crate::variable_bindings::transfer_source_namespace_cells_with_input(
            &mut entered,
            &facts,
            arguments,
            registry,
            crate::variable_bindings::SourceNamespaceTransfer::new(false, 40, Some(&[0]))
                .with_original_operands(&operands),
        );
        assert_eq!(read(&entered, name), b"KEEP");
        assert!(entered.raw_static_unset_error_at_slot(&slot));
    }

    #[test]
    fn original_static_capture_withdraws_atomically_for_unowned_or_observed_inputs() {
        // Implementation contract: naming.variable.original-callable-static-capture (docs/design/analysis/name-resolution-proofs/original-callable-static-capture.md).
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut context = context();
        store(&mut context, b"x", b"VALUE", 1);
        let operand = input(b"{{first FIRST} x}", &context);
        let mut observed = context.clone();
        observed.mark_unenumerated_variable_observers();
        let before = observed.clone();
        assert_eq!(
            observed.capture_original_callable_statics(&allocation(10), &operand, registry),
            StaticCaptureResult::Unknown
        );
        assert_eq!(observed, before);
        let malformed = input(b"{{first FIRST} {x VALUE EXTRA}}", &context);
        let before = context.clone();
        assert_eq!(
            context.capture_original_callable_statics(&allocation(10), &malformed, registry),
            StaticCaptureResult::Invalid
        );
        assert_eq!(context, before);
        let held = crate::command_binding::original_name_value::OriginalProducedNameValue::from_source_input(
            &operand, &context,
        ).unwrap();
        let stale = SignatureSourceNameInput::OriginalValue(
            crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(
                &held,
            ),
        );
        context.invalidate_original_contents();
        let before = context.clone();
        assert_eq!(
            context.capture_original_callable_statics(&allocation(10), &stale, registry),
            StaticCaptureResult::Unknown
        );
        assert_eq!(context, before);
        context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6),
        ));
        let before = context.clone();
        assert_eq!(
            context.capture_original_callable_statics(&allocation(10), &operand, registry),
            StaticCaptureResult::Unknown
        );
        assert_eq!(context, before);
    }
}
