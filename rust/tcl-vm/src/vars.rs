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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Stable variable-cell storage shared by frames and namespace tokens.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use tcl_core_types::{NameBytes, NsId, VarId};
use tcl_runtime_api::FrameLinkOrigin;

use crate::interp::VarBinding;
use crate::value::Value;

/// A name in the already selected activation, resolved anew on each access.
/// This does not own the variable currently bound to that name.
#[derive(Clone)]
pub(crate) struct NameLink {
    /// Actual Jim link owns the original target-name object and weak target frame.
    pub(crate) original_target: Option<JimNameLinkOriginal>,
    /// The table selected at registration, with its unqualified storage key.
    pub(crate) binding: VarBinding,
    /// The optional literal element key, looked up after the array name.
    pub(crate) element: Option<NameBytes>,
}

#[derive(Clone)]
pub(crate) struct JimNameLinkOriginal {
    pub(crate) name: Value,
    pub(crate) frame: std::rc::Weak<crate::frame::ActivationIdentity>,
}

/// A name table owns bindings, while the arena owns the cells they identify.
/// Removing a binding never makes its `VarId` identify a later variable.
#[derive(Clone, Default)]
pub(crate) struct VarTable {
    bindings: BTreeMap<NameBytes, VarId>,
    native_keys: BTreeMap<NameBytes, Rc<NativeHashEntryKey>>,
    order: RefCell<tcl_core_types::NativeEntryLedger>,
}

impl VarTable {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn select_hash_recipe(&self, recipe: Option<tcl_core_types::NativeHashRecipe>) {
        self.order.borrow_mut().select_recipe(recipe);
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
    pub(crate) fn get(&self, name: impl AsRef<[u8]>) -> Option<&VarId> {
        self.bindings.get(name.as_ref())
    }
    pub(crate) fn contains_key(&self, name: impl AsRef<[u8]>) -> bool {
        self.bindings.contains_key(name.as_ref())
    }
    pub(crate) fn insert(&mut self, name: impl Into<NameBytes>, cell: VarId) -> Option<VarId> {
        let name = name.into();
        self.order.get_mut().insert(name.as_bytes());
        self.bindings.insert(name, cell)
    }
    pub(crate) fn remove(&mut self, name: impl AsRef<[u8]>) -> Option<VarId> {
        let old = self.bindings.remove(name.as_ref());
        if old.is_some() {
            self.order.get_mut().remove(name.as_ref());
            self.native_keys.remove(name.as_ref());
        }
        old
    }
    pub(crate) fn retain_original_native_key(&mut self, name: &NameBytes, original: Value) -> bool {
        if !self.bindings.contains_key(name) {
            return false;
        }
        self.native_keys.entry(name.clone()).or_insert_with(|| {
            Rc::new(NativeHashEntryKey {
                original: RefCell::new(Some(original)),
                destruction_owners: std::cell::Cell::new(0),
            })
        });
        true
    }
    #[cfg(test)]
    pub(crate) fn native_key_is(&self, name: &[u8], original: &Value) -> bool {
        self.native_keys.get(name).is_some_and(|key| {
            key.original
                .borrow()
                .as_ref()
                .is_some_and(|key| key.is_same_object(original))
        })
    }
    pub(crate) fn destruction_key_drain(&self) -> NativeArrayKeyDrain {
        let keys = self.native_keys.values().cloned().collect::<Vec<_>>();
        for entry in &keys {
            entry.destruction_owners.set(
                entry
                    .destruction_owners
                    .get()
                    .checked_add(1)
                    .expect("table key drain overflow"),
            );
        }
        NativeArrayKeyDrain(keys)
    }
    pub(crate) fn retain_original_jim_key(&mut self, name: &NameBytes, original: Value) -> bool {
        self.retain_original_native_key(name, original)
    }
    pub(crate) fn keys(&self) -> std::vec::IntoIter<&NameBytes> {
        let order = self.order.borrow();
        let keys: Vec<&NameBytes> = match order.keys() {
            Some(keys) => keys
                .into_iter()
                .filter_map(|key| self.bindings.get_key_value(key).map(|(key, _)| key))
                .collect(),
            // Unselected analytical tables have no native ordering authority.
            None => self.bindings.keys().collect(),
        };
        keys.into_iter()
    }
    pub(crate) fn values(&self) -> impl Iterator<Item = &VarId> {
        self.keys().filter_map(|key| self.bindings.get(key))
    }
    pub(crate) fn into_values(self) -> impl Iterator<Item = VarId> {
        self.into_iter().map(|(_, value)| value)
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&NameBytes, &VarId)> {
        self.keys()
            .filter_map(|key| self.bindings.get_key_value(key))
    }
}
impl IntoIterator for VarTable {
    type Item = (NameBytes, VarId);
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(mut self) -> Self::IntoIter {
        let keys: Vec<_> = self.keys().cloned().collect();
        keys.into_iter()
            .filter_map(|key| self.bindings.remove(&key).map(|value| (key, value)))
            .collect::<Vec<_>>()
            .into_iter()
    }
}
impl<K: Into<NameBytes>, const N: usize> From<[(K, VarId); N]> for VarTable {
    fn from(entries: [(K, VarId); N]) -> Self {
        let mut table = Self::new();
        for (key, value) in entries {
            table.insert(key, value);
        }
        table
    }
}

/// Persistent native procedure bindings retain cells, independently of call locals.
pub(crate) struct StaticVariables {
    pub(crate) table: VarTable,
    released: Rc<RefCell<Vec<VarId>>>,
}

impl Drop for StaticVariables {
    fn drop(&mut self) {
        self.released
            .borrow_mut()
            .extend(self.table.values().copied());
    }
}

/// The mutable state carried by one Tcl `Var` cell.
pub(crate) enum VarState {
    /// Materialised but unset (for `variable` and `trace add variable`).
    Undefined,
    /// A scalar value.
    Scalar(Value),
    /// An array whose element names bind stable cells of their own.
    Array(VarTable),
    /// An `upvar`/`global`/`variable` alias to its target cell.
    Link(VarId),
    /// A dialect-defined name link, rather than a retained C Tcl cell.
    NameLink(NameLink),
}

/// One stable variable cell. Cells are never reused for a later binding.
pub(crate) struct VarCell {
    /// Birth identity owned only by the actual cell, independently of arena IDs.
    jim_birth: Rc<()>,
    state: VarState,
    /// The containing array cell and element key, for Tcl's alias reporting.
    /// This remains on a detached element so an `upvar` alias cannot silently
    /// retarget a later same-name element.
    parent: Option<(VarId, NameBytes)>,
    bindings: u64,
    link_refs: u64,
    pins: u64,
    operation_refs: u64,
    static_refs: u64,
    array_revision: u64,
    array_searches: tcl_core_types::NativeArraySearchChain<Value>,
    /// Tcl 9's array table owns two references to its default object.
    array_default: Option<[Value; 2]>,
    link_origin: FrameLinkOrigin,
    namespace_owner: Option<NsId>,
    namespace_declared: bool,
    native_entry: Option<VarBinding>,
    /// The actual element entry shares its one original object-key owner with
    /// the table. Whole-table retirement releases the header while aliases retain the Var.
    native_element_key: Option<Rc<NativeHashEntryKey>>,
    element_destruction: Option<bool>,
    element_definition_pending: bool,
}

struct NativeHashEntryKey {
    original: RefCell<Option<Value>>,
    destruction_owners: std::cell::Cell<usize>,
}

/// The old hash table owns its CPP keys through all element callbacks. This
/// drain retains key capsules only, independently of the old Var nodes.
pub(crate) struct NativeArrayKeyDrain(Vec<Rc<NativeHashEntryKey>>);
impl Drop for NativeArrayKeyDrain {
    fn drop(&mut self) {
        for entry in &self.0 {
            let count = entry
                .destruction_owners
                .get()
                .checked_sub(1)
                .expect("table key drain underflow");
            entry.destruction_owners.set(count);
            if count == 0 {
                let original = entry.original.borrow_mut().take();
                drop(original);
            }
        }
    }
}

/// Jim's cached `VarVal` pointer has no native cell/value ownership.
#[derive(Clone)]
pub(crate) struct WeakJimVariableCell {
    id: VarId,
    birth: std::rc::Weak<()>,
}

impl VarCell {
    pub(crate) const fn namespace_declared(&self) -> bool {
        self.namespace_declared
    }

    pub(crate) const fn namespace_owner(&self) -> Option<NsId> {
        self.namespace_owner
    }

    pub(crate) const fn state(&self) -> &VarState {
        &self.state
    }

    pub(crate) const fn link_origin(&self) -> FrameLinkOrigin {
        self.link_origin
    }
}

/// Monotonic arena for every variable cell in one interpreter.
#[derive(Default)]
pub(crate) struct VarArena {
    hash_recipe: std::cell::Cell<Option<tcl_core_types::NativeHashRecipe>>,
    next: u64,
    cells: HashMap<VarId, VarCell>,
    released_statics: Rc<RefCell<Vec<VarId>>>,
    released_native_alias_entries: Vec<(VarBinding, VarId)>,
}

/// The reached original search header and its already selected byte/cache views.
#[derive(Clone, Copy)]
pub(crate) struct NativeArraySearchHandle<'a> {
    pub(crate) original: Option<&'a Value>,
    pub(crate) bytes: Option<&'a [u8]>,
    pub(crate) cache: Option<tcl_core_types::NativeArraySearchCache>,
}

impl VarArena {
    pub(crate) fn select_hash_recipe(&self, recipe: Option<tcl_core_types::NativeHashRecipe>) {
        if self.hash_recipe.replace(recipe) == recipe {
            return;
        }
        for cell in self.cells.values() {
            if let VarState::Array(table) = &cell.state {
                table.select_hash_recipe(recipe);
            }
        }
    }

    pub(crate) fn array_cells(&self) -> Vec<VarId> {
        self.cells
            .iter()
            .filter_map(|(&id, cell)| matches!(cell.state(), VarState::Array(_)).then_some(id))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.cells.len()
    }

    pub(crate) fn alloc(&mut self, state: VarState) -> Option<VarId> {
        self.collect_released_statics();
        self.alloc_with_parent(state, None, FrameLinkOrigin::Ordinary)
    }

    pub(crate) fn alloc_link(&mut self, target: VarId, origin: FrameLinkOrigin) -> Option<VarId> {
        self.alloc_with_parent(VarState::Link(target), None, origin)
    }

    pub(crate) fn alloc_element(
        &mut self,
        state: VarState,
        parent: VarId,
        key: impl Into<NameBytes>,
    ) -> Option<VarId> {
        self.alloc_with_parent(state, Some((parent, key.into())), FrameLinkOrigin::Ordinary)
    }

    fn alloc_with_parent(
        &mut self,
        state: VarState,
        parent: Option<(VarId, NameBytes)>,
        link_origin: FrameLinkOrigin,
    ) -> Option<VarId> {
        let state = match state {
            VarState::Scalar(value) => VarState::Scalar(value.into_native_reference()),
            state => state,
        };
        if let VarState::Link(target) = &state
            && self.resolve(*target).is_none()
        {
            return None;
        }
        if let VarState::Array(table) = &state {
            table.select_hash_recipe(self.hash_recipe.get());
        }
        let raw = u32::try_from(self.next).ok()?;
        self.next = self.next.checked_add(1)?;
        let id = VarId(raw);
        let link = match &state {
            VarState::Link(target) => Some(*target),
            _ => None,
        };
        self.cells.insert(
            id,
            VarCell {
                jim_birth: Rc::new(()),
                state,
                parent,
                bindings: 0,
                link_refs: 0,
                pins: 0,
                operation_refs: 0,
                static_refs: 0,
                array_revision: 0,
                array_searches: tcl_core_types::NativeArraySearchChain::default(),
                array_default: None,
                link_origin,
                namespace_owner: None,
                namespace_declared: false,
                native_entry: None,
                native_element_key: None,
                element_destruction: None,
                element_definition_pending: false,
            },
        );
        let children = match self.cells.get(&id)?.state() {
            VarState::Array(elements) => elements.clone(),
            _ => VarTable::new(),
        };
        for (name, child) in children {
            self.cells.get_mut(&child)?.parent = Some((id, name));
        }
        if let Some(target) = link
            && let Some(cell) = self.cells.get_mut(&target)
        {
            cell.link_refs = cell
                .link_refs
                .checked_add(1)
                .expect("variable link reference count overflow");
        }
        Some(id)
    }

    pub(crate) fn get(&self, id: VarId) -> Option<&VarCell> {
        self.cells.get(&id)
    }

    pub(crate) fn weak_jim_cell(&self, id: VarId) -> Option<WeakJimVariableCell> {
        let cell = self.cells.get(&id)?;
        Some(WeakJimVariableCell {
            id,
            birth: Rc::downgrade(&cell.jim_birth),
        })
    }

    pub(crate) fn validate_jim_cell(&self, receipt: &WeakJimVariableCell) -> Option<VarId> {
        let birth = receipt.birth.upgrade()?;
        let actual = self.cells.get(&receipt.id)?;
        Rc::ptr_eq(&birth, &actual.jim_birth).then_some(receipt.id)
    }

    pub(crate) fn array_default(&self, id: VarId) -> Option<&Value> {
        let cell = self.cells.get(&id)?;
        if !matches!(cell.state, VarState::Array(_)) {
            return None;
        }
        cell.array_default.as_ref().map(|values| &values[0])
    }

    pub(crate) fn set_array_default(&mut self, id: VarId, value: Option<Value>) -> bool {
        let Some(cell) = self.cells.get_mut(&id) else {
            return false;
        };
        if !matches!(cell.state, VarState::Array(_)) {
            return false;
        }
        cell.array_default = value.map(|value| [value.clone(), value]);
        true
    }

    pub(crate) fn mark_namespace_declared(&mut self, id: VarId) {
        if let Some(cell) = self.cells.get_mut(&id) {
            cell.namespace_declared = true;
        }
    }

    /// Name-table ownership is stamped at allocation and survives token teardown.
    pub(crate) fn set_namespace_owner(&mut self, id: VarId, namespace: NsId) {
        if let Some(cell) = self.cells.get_mut(&id) {
            assert!(cell.namespace_owner.is_none_or(|owner| owner == namespace));
            cell.namespace_owner = Some(namespace);
        }
    }

    pub(crate) fn array_insert(
        &mut self,
        id: VarId,
        key: impl Into<NameBytes>,
        element: VarId,
    ) -> bool {
        let Some(cell) = self.cells.get_mut(&id) else {
            return false;
        };
        let VarState::Array(elements) = &mut cell.state else {
            return false;
        };
        let key = key.into();
        if elements.contains_key(&key) {
            return false;
        }
        elements.insert(key, element);
        cell.array_revision = cell.array_revision.wrapping_add(1);
        cell.array_searches.clear();
        true
    }

    /// Actual object-key ownership belongs to the selected element table.
    pub(crate) fn retain_original_array_key(
        &mut self,
        array: VarId,
        key: &NameBytes,
        original: Value,
    ) -> bool {
        let Some(cell) = self.cells.get_mut(&array) else {
            return false;
        };
        let VarState::Array(elements) = &mut cell.state else {
            return false;
        };
        let Some(member) = elements.get(key).copied() else {
            return false;
        };
        if !elements.retain_original_native_key(key, original) {
            return false;
        }
        let owner = Rc::clone(elements.native_keys.get(key).expect("actual key owner"));
        if let Some(member) = self.cells.get_mut(&member) {
            member.native_element_key = Some(owner);
            true
        } else {
            false
        }
    }

    /// Hash entry destruction releases its key even if aliases retain the Var.
    pub(crate) fn retire_array_element_key(&mut self, id: VarId) {
        if let Some(cell) = self.cells.get_mut(&id)
            && let Some(entry) = cell.native_element_key.take()
            && entry.destruction_owners.get() == 0
        {
            let key = entry.original.borrow_mut().take();
            drop(key);
        }
    }

    pub(crate) fn record_native_entry(&mut self, id: VarId, binding: VarBinding) {
        if let Some(cell) = self.cells.get_mut(&id) {
            cell.native_entry.get_or_insert(binding);
        }
    }

    pub(crate) fn take_released_native_alias_entries(&mut self) -> Vec<(VarBinding, VarId)> {
        std::mem::take(&mut self.released_native_alias_entries)
    }

    fn note_native_alias_release(&mut self, target: VarId) {
        if let Some(cell) = self.cells.get(&target)
            && cell.link_refs == 0
            && let Some(binding) = &cell.native_entry
        {
            self.released_native_alias_entries
                .push((binding.clone(), target));
        }
    }

    pub(crate) fn array_remove(&mut self, id: VarId, key: impl AsRef<[u8]>) -> Option<VarId> {
        let cell = self.cells.get_mut(&id)?;
        let VarState::Array(elements) = &mut cell.state else {
            return None;
        };
        let removed = elements.remove(key)?;
        cell.array_revision = cell.array_revision.wrapping_add(1);
        cell.array_searches.clear();
        self.retire_array_element_key(removed);
        Some(removed)
    }

    /// Discard an element only while its table slot still names `expected`.
    /// Garbage-collecting an undefined trace shell is not a Tcl-level array
    /// mutation and therefore does not invalidate an active search.
    pub(crate) fn array_discard_if(
        &mut self,
        id: VarId,
        key: impl AsRef<[u8]>,
        expected: VarId,
    ) -> Option<VarId> {
        let cell = self.cells.get_mut(&id)?;
        let VarState::Array(elements) = &mut cell.state else {
            return None;
        };
        if elements.get(key.as_ref()) != Some(&expected) {
            return None;
        }
        let removed = elements.remove(key)?;
        self.retire_array_element_key(removed);
        Some(removed)
    }

    /// Execute a cursor against this exact cell without another name lookup.
    pub(crate) fn native_array_search(
        &mut self,
        id: VarId,
        sub: &str,
        name: &[u8],
        input: NativeArraySearchHandle<'_>,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<
        Result<Value, tcl_syntax::native_array_search::NativeArraySearchFailure>,
        tcl_syntax::value::ValueError,
    > {
        use tcl_syntax::value::ValueError;
        let NativeArraySearchHandle {
            original: handle,
            bytes,
            cache,
        } = input;
        let cell = self
            .cells
            .get_mut(&id)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original array search cell",
            ))?;
        let VarState::Array(table) = &cell.state else {
            return Err(ValueError::CommandProtocolUnavailable(
                "original array search cell",
            ));
        };
        let keys = table
            .order
            .borrow()
            .keys()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "native array search table",
            ))?
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect();
        let mut chain = std::mem::take(&mut cell.array_searches);
        // No guest callback occurs inside this closed cursor operation. Taking
        // the chain permits actual member-cell inspection without owning Values.
        let result = (|| {
            if sub == "startsearch" {
                let next = chain
                    .next_id()
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "native array search id overflow",
                    ))?;
                let value = Value::new_native_string_bytes(protocol.handle(next, name));
                chain.insert(
                    next,
                    (!protocol.caches_handle()).then(|| value.clone()),
                    keys,
                );
                return Ok(Ok(value));
            }
            let handle = handle.ok_or(ValueError::CommandProtocolUnavailable(
                "original search handle",
            ))?;
            let found = tcl_cmd_core::native_array_search::resolve(
                &chain,
                protocol,
                name,
                &tcl_cmd_core::native_array_search::NativeArraySearchOperand {
                    original: handle,
                    bytes: bytes.unwrap_or_default(),
                    cache,
                },
                |held, original| held.native_object_identity() == original.native_object_identity(),
                |held| {
                    Ok(held
                        .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::C(
                            protocol.version(),
                        ))
                        .map_err(|_| {
                            ValueError::CommandProtocolUnavailable("native retained search handle")
                        })?
                        .to_vec())
                },
            )?;
            let search = match found {
                Ok(search) => search,
                Err(failure) => return Ok(Err(failure)),
            };
            if sub == "donesearch" {
                chain.remove(search);
                return Ok(Ok(Value::new_native_string_bytes(Vec::new())));
            }
            let next = chain.next_defined(search, sub == "nextelement", |key| {
                let Some(cell) = self.cells.get(&id) else {
                    return false;
                };
                let VarState::Array(table) = &cell.state else {
                    return false;
                };
                table
                    .get(key)
                    .and_then(|id| self.resolve(*id))
                    .and_then(|id| self.cells.get(&id))
                    .is_some_and(|cell| matches!(cell.state, VarState::Scalar(_)))
            });
            Ok(Ok(if sub == "anymore" {
                Value::int(i64::from(next.is_some()))
            } else {
                Value::new_native_string_bytes(next.unwrap_or_default())
            }))
        })();
        self.cells
            .get_mut(&id)
            .expect("closed search keeps original cell")
            .array_searches = chain;
        // Failure is returned independently of the chain's lifetime.
        result
    }

    pub(crate) fn array_revision(&self, id: VarId) -> Option<u64> {
        let cell = self.cells.get(&id)?;
        matches!(cell.state, VarState::Array(_)).then_some(cell.array_revision)
    }

    pub(crate) fn retain_statics(&mut self, table: VarTable) -> Rc<StaticVariables> {
        for id in table.values() {
            let cell = self.cells.get_mut(id).expect("static cell exists");
            cell.static_refs = cell
                .static_refs
                .checked_add(1)
                .expect("static reference overflow");
        }
        Rc::new(StaticVariables {
            table,
            released: Rc::clone(&self.released_statics),
        })
    }

    pub(crate) fn has_static_refs(&self, id: VarId) -> bool {
        self.cells
            .get(&id)
            .is_some_and(|cell| cell.static_refs != 0)
    }

    pub(crate) fn collect_released_statics(&mut self) {
        let released = std::mem::take(&mut *self.released_statics.borrow_mut());
        for id in released {
            let cell = self
                .cells
                .get_mut(&id)
                .expect("retained static cell exists");
            cell.static_refs = cell
                .static_refs
                .checked_sub(1)
                .expect("static reference underflow");
            self.collect(id);
        }
    }

    /// Snapshot the value of a native static initializer, including array contents.
    pub(crate) fn copy_cell(&mut self, id: VarId) -> Option<VarId> {
        let state = match self.get(id)?.state() {
            VarState::Scalar(value) => VarState::Scalar(value.clone()),
            VarState::Array(elements) => {
                let elements = elements.clone();
                let mut copied = VarTable::new();
                for (name, id) in elements {
                    let id = self.copy_cell(id)?;
                    self.bind(id);
                    copied.insert(name, id);
                }
                VarState::Array(copied)
            }
            VarState::Undefined | VarState::Link(_) | VarState::NameLink(_) => return None,
        };
        self.alloc(state)
    }

    pub(crate) fn has_link_refs(&self, id: VarId) -> bool {
        self.cells.get(&id).is_some_and(|cell| cell.link_refs != 0)
    }

    pub(crate) fn element_parent(&self, id: VarId) -> Option<(VarId, &NameBytes)> {
        self.cells
            .get(&id)?
            .parent
            .as_ref()
            .map(|(parent, key)| (*parent, key))
    }

    pub(crate) fn begin_element_destruction(&mut self, ids: impl Iterator<Item = VarId>) {
        for id in ids {
            if let Some(cell) = self.cells.get_mut(&id) {
                cell.element_destruction = Some(false);
            }
        }
    }
    pub(crate) fn invalidate_destroyed_element(&mut self, id: VarId, preserve_definition: bool) {
        if let Some(cell) = self.cells.get_mut(&id) {
            cell.element_destruction = Some(true);
            // This is the C84 Var flag, not a value or an existence result.
            cell.element_definition_pending = preserve_definition
                && matches!(cell.state, VarState::Scalar(_) | VarState::Array(_));
        }
    }
    pub(crate) fn finish_destroyed_element_trace(&mut self, id: VarId) {
        if let Some(cell) = self.cells.get_mut(&id) {
            cell.element_definition_pending = false;
        }
    }
    #[cfg(test)]
    pub(crate) fn observe_native_element(&self, id: VarId, object_table: bool) -> (i32, i32, i32) {
        let Some(cell) = self.cells.get(&id) else {
            return (-1, -1, -1);
        };
        let defined = cell.element_definition_pending
            || matches!(cell.state, VarState::Scalar(_) | VarState::Array(_));
        let references =
            cell.link_refs + cell.operation_refs + if object_table { cell.bindings } else { 0 };
        (
            i32::from(cell.element_destruction == Some(true)),
            i32::from(defined),
            i32::try_from(references).unwrap(),
        )
    }

    pub(crate) fn element_is_attached(&self, id: VarId) -> bool {
        if let Some(retired) = self.get(id).and_then(|cell| cell.element_destruction) {
            return !retired;
        }
        let Some((parent, key)) = self.element_parent(id) else {
            return true;
        };
        let Some(VarState::Array(elements)) = self.get(parent).map(VarCell::state) else {
            return false;
        };
        elements
            .get(key)
            .copied()
            .and_then(|element| self.resolve(element))
            == Some(id)
    }

    pub(crate) fn replace_state(&mut self, id: VarId, state: VarState) -> bool {
        let state = match state {
            VarState::Scalar(value) => VarState::Scalar(value.into_native_reference()),
            state => state,
        };
        if let VarState::Array(table) = &state {
            table.select_hash_recipe(self.hash_recipe.get());
        }
        let new_link = match &state {
            VarState::Link(target) => Some(*target),
            _ => None,
        };
        if let Some(target) = new_link
            && self.resolve(target).is_none_or(|resolved| resolved == id)
        {
            return false;
        }
        let Some(cell) = self.cells.get_mut(&id) else {
            return false;
        };
        let old_was_array = matches!(cell.state, VarState::Array(_));
        let new_is_array = matches!(state, VarState::Array(_));
        let old = std::mem::replace(&mut cell.state, state);
        if old_was_array {
            cell.array_default = None;
        }
        cell.link_origin = FrameLinkOrigin::Ordinary;
        if old_was_array || new_is_array {
            cell.array_revision = cell.array_revision.wrapping_add(1);
            cell.array_searches.clear();
        }
        if let Some(target) = new_link
            && let Some(target_cell) = self.cells.get_mut(&target)
        {
            target_cell.link_refs = target_cell
                .link_refs
                .checked_add(1)
                .expect("variable link reference count overflow");
        }
        match old {
            VarState::Link(target) => {
                if let Some(target_cell) = self.cells.get_mut(&target) {
                    target_cell.link_refs = target_cell
                        .link_refs
                        .checked_sub(1)
                        .expect("variable link reference count underflow");
                }
                self.note_native_alias_release(target);
                self.collect(target);
            }
            VarState::Array(elements) => {
                for element in elements.into_values() {
                    self.retire_array_element_key(element);
                    self.unbind(element);
                }
            }
            VarState::Undefined | VarState::Scalar(_) | VarState::NameLink(_) => {}
        }
        true
    }

    /// Move a newly allocated state's ownership into an existing compiled cell.
    pub(crate) fn transfer_unbound_state(&mut self, source: VarId, receiver: VarId) -> bool {
        let Some(source_cell) = self.cells.get(&source) else {
            return false;
        };
        assert_eq!(
            source_cell.bindings, 0,
            "state transfer requires an unbound source"
        );
        if source == receiver || !self.cells.contains_key(&receiver) {
            return false;
        }
        if let VarState::Link(target) = source_cell.state()
            && self
                .resolve(*target)
                .is_none_or(|resolved| resolved == receiver)
        {
            return false;
        }
        let origin = source_cell.link_origin;
        let default = self
            .cells
            .get_mut(&source)
            .expect("source cell exists")
            .array_default
            .take();
        let state = self.take_state(source).expect("source cell exists");
        if let VarState::Link(target) = &state {
            let target_cell = self.cells.get_mut(target).expect("new link target exists");
            target_cell.link_refs = target_cell
                .link_refs
                .checked_sub(1)
                .expect("transferred link reference");
        }
        if !self.replace_state(receiver, state) {
            return false;
        }
        let receiver = self
            .cells
            .get_mut(&receiver)
            .expect("compiled receiver exists");
        receiver.link_origin = origin;
        receiver.array_default = default;
        true
    }

    /// Apply Tcl's unset transition and invalidate an active parent-array
    /// search whenever this is an attached element. Even an already undefined
    /// trace/link shell cancels the search; a generic state replacement does
    /// not carry that operation-level meaning.
    pub(crate) fn unset_state(&mut self, id: VarId) -> Option<VarState> {
        let attached_parent = self
            .element_is_attached(id)
            .then(|| self.element_parent(id).map(|(parent, _)| parent))
            .flatten();
        let old = self.take_state(id)?;
        if let Some(parent) = attached_parent
            && let Some(parent_cell) = self.cells.get_mut(&parent)
        {
            parent_cell.array_revision = parent_cell.array_revision.wrapping_add(1);
            parent_cell.array_searches.clear();
        }
        Some(old)
    }

    /// Detach a cell's current state without releasing bindings owned by an
    /// array table inside it. Destructive lifecycle code walks those element
    /// bindings one at a time because callbacks can observe and mutate the
    /// remaining old cells.
    pub(crate) fn take_state(&mut self, id: VarId) -> Option<VarState> {
        let cell = self.cells.get_mut(&id)?;
        let old = std::mem::replace(&mut cell.state, VarState::Undefined);
        if matches!(old, VarState::Array(_)) {
            cell.array_default = None;
            cell.array_revision = cell.array_revision.wrapping_add(1);
            cell.array_searches.clear();
        }
        Some(old)
    }

    /// Follow link cells to the storage cell Tcl operations observe.
    pub(crate) fn resolve(&self, mut id: VarId) -> Option<VarId> {
        let mut visited = HashSet::new();
        while visited.insert(id) {
            match self.get(id).map(|cell| &cell.state) {
                Some(VarState::Link(target)) => id = *target,
                // The interpreter's shared resolver owns selected-frame name
                // lookup; the arena alone cannot resolve such a link.
                Some(VarState::NameLink(_)) | None => return None,
                Some(_) => return Some(id),
            }
        }
        None
    }

    pub(crate) fn bind(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("binding must refer to a live variable cell");
        cell.bindings = cell
            .bindings
            .checked_add(1)
            .expect("variable binding count overflow");
    }

    pub(crate) fn unbind(&mut self, id: VarId) {
        self.collect_released_statics();
        let cell = self
            .cells
            .get_mut(&id)
            .expect("binding must refer to a live variable cell");
        cell.bindings = cell
            .bindings
            .checked_sub(1)
            .expect("variable binding count underflow");
        self.collect(id);
    }

    /// Discard a cell that was allocated but never inserted into a name table.
    /// This is deliberately distinct from [`Self::unbind`]: no binding count
    /// exists to decrement on the failed-installation path.
    pub(crate) fn discard_unbound(&mut self, id: VarId) {
        let cell = self
            .cells
            .get(&id)
            .expect("discard must refer to a live variable cell");
        assert_eq!(cell.bindings, 0, "bound variable cell cannot be discarded");
        self.collect(id);
    }

    pub(crate) fn pin(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("pin must refer to a live variable cell");
        cell.pins = cell
            .pins
            .checked_add(1)
            .expect("variable pin count overflow");
    }

    pub(crate) fn unpin(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("pin must refer to a live variable cell");
        cell.pins = cell
            .pins
            .checked_sub(1)
            .expect("variable pin count underflow");
        self.collect(id);
    }

    pub(crate) fn retain_operation(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("operation reference must refer to a live variable cell");
        cell.operation_refs = cell
            .operation_refs
            .checked_add(1)
            .expect("variable operation reference count overflow");
    }

    pub(crate) fn release_operation(&mut self, id: VarId) {
        let cell = self
            .cells
            .get_mut(&id)
            .expect("operation reference must refer to a live variable cell");
        cell.operation_refs = cell
            .operation_refs
            .checked_sub(1)
            .expect("variable operation reference count underflow");
        self.collect(id);
    }

    pub(crate) fn has_operation_refs(&self, id: VarId) -> bool {
        self.cells
            .get(&id)
            .is_some_and(|cell| cell.operation_refs != 0)
    }

    pub(crate) fn is_final_operation_ref(&self, id: VarId) -> bool {
        self.cells
            .get(&id)
            .is_some_and(|cell| cell.operation_refs == 1)
    }

    fn collect(&mut self, id: VarId) {
        let collectable = self.cells.get(&id).is_some_and(|cell| {
            cell.bindings == 0
                && cell.link_refs == 0
                && cell.pins == 0
                && cell.operation_refs == 0
                && cell.static_refs == 0
        });
        if !collectable {
            return;
        }
        let Some(cell) = self.cells.remove(&id) else {
            return;
        };
        match cell.state {
            VarState::Link(target) => {
                if let Some(target_cell) = self.cells.get_mut(&target) {
                    target_cell.link_refs = target_cell
                        .link_refs
                        .checked_sub(1)
                        .expect("variable link reference count underflow");
                }
                self.note_native_alias_release(target);
                self.collect(target);
            }
            VarState::Array(elements) => {
                for element in elements.into_values() {
                    self.retire_array_element_key(element);
                    self.unbind(element);
                }
            }
            VarState::Undefined | VarState::Scalar(_) | VarState::NameLink(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeArraySearchHandle, VarArena, VarState, VarTable};
    use crate::value::Value;
    use tcl_core_types::NameBytes;

    #[test]
    fn search_chain_owns_modern_handle_and_retires_on_original_table_mutation() {
        use tcl_core_types::{
            NativeArraySearchAbi, NativeHashBytePromotion, NativeHashRecipe, NativeHashWordWidth,
        };
        use tcl_dialect::TclVersion;
        use tcl_syntax::native_array_search::NativeArraySearchProtocol;
        let mut arena = VarArena::default();
        arena.select_hash_recipe(Some(NativeHashRecipe::Tcl {
            promotion: NativeHashBytePromotion::Unsigned,
            width: NativeHashWordWidth::Bits64,
        }));
        let root = arena.alloc(VarState::Array(VarTable::new())).unwrap();
        let member = arena
            .alloc(VarState::Scalar(Value::string("value")))
            .unwrap();
        assert!(arena.array_insert(root, b"key", member));
        let protocol = NativeArraySearchProtocol::for_tcl_version(
            TclVersion::V9_0,
            NativeArraySearchAbi {
                unsigned_long: NativeHashWordWidth::Bits64,
                int_bits: 32,
            },
        )
        .unwrap();
        let handle = arena
            .native_array_search(
                root,
                "startsearch",
                b"a",
                NativeArraySearchHandle {
                    original: None,
                    bytes: None,
                    cache: None,
                },
                protocol,
            )
            .unwrap()
            .unwrap();
        let retained = handle.downgrade_native_object();
        assert!(handle.native_object_is_shared());
        drop(handle);
        assert!(retained.upgrade().is_some());
        let shell = arena.alloc(VarState::Undefined).unwrap();
        assert!(arena.array_insert(root, b"new", shell));
        assert!(retained.upgrade().is_none());
        let handle = arena
            .native_array_search(
                root,
                "startsearch",
                b"a",
                NativeArraySearchHandle {
                    original: None,
                    bytes: None,
                    cache: None,
                },
                protocol,
            )
            .unwrap()
            .unwrap();
        let retained = handle.downgrade_native_object();
        drop(handle);
        assert_eq!(arena.array_discard_if(root, b"new", shell), Some(shell));
        assert!(retained.upgrade().is_some());
        assert_eq!(arena.array_remove(root, b"key"), Some(member));
        assert!(retained.upgrade().is_none());
    }

    #[test]
    fn detached_array_entry_releases_its_key_while_actual_aliases_keep_the_var() {
        let mut arena = VarArena::default();
        let array = arena.alloc(VarState::Array(VarTable::new())).unwrap();
        arena.bind(array);
        let key = Value::from_native_string_bytes(b"k\0z".to_vec());
        let name = NameBytes::from(b"k\0z".as_slice());
        let member = arena
            .alloc_element(VarState::Scalar(Value::string("old")), array, name.clone())
            .unwrap();
        arena.bind(member);
        assert!(arena.array_insert(array, name.clone(), member));
        assert!(arena.retain_original_array_key(array, &name, key.clone()));
        assert_eq!(key.native_object_reference_count(), 2);
        let first = arena.alloc(VarState::Link(member)).unwrap();
        let second = arena.alloc(VarState::Link(member)).unwrap();
        arena.bind(first);
        arena.bind(second);
        assert_eq!(key.native_object_reference_count(), 2);
        assert!(arena.replace_state(array, VarState::Array(VarTable::new())));
        assert!(arena.get(member).is_some());
        assert_eq!(key.native_object_reference_count(), 1);
        let new_key = Value::from_native_string_bytes(b"k\0z".to_vec());
        let replacement = arena
            .alloc_element(VarState::Scalar(Value::string("new")), array, name.clone())
            .unwrap();
        arena.bind(replacement);
        assert!(arena.array_insert(array, name.clone(), replacement));
        assert!(arena.retain_original_array_key(array, &name, new_key.clone()));
        assert_ne!(member, replacement);
        arena.unbind(first);
        assert_eq!(key.native_object_reference_count(), 1);
        arena.unbind(second);
        assert!(arena.get(member).is_none());
        assert_eq!(key.native_object_reference_count(), 1);
        assert_eq!(new_key.native_object_reference_count(), 2);
        let VarState::Array(table) = arena.get(array).unwrap().state() else {
            panic!("actual array");
        };
        assert_eq!(table.get(&name), Some(&replacement));
    }

    #[test]
    fn array_default_owns_two_references_and_expires_with_its_generation() {
        let mut arena = VarArena::default();
        let array = arena.alloc(VarState::Array(VarTable::new())).unwrap();
        arena.bind(array);
        let original = Value::string("DEFAULT");
        assert!(arena.set_array_default(array, Some(original.clone())));
        assert_eq!(original.native_object_reference_count(), 3);
        assert!(
            arena
                .array_default(array)
                .unwrap()
                .is_same_object(&original)
        );
        arena.take_state(array).unwrap();
        assert_eq!(original.native_object_reference_count(), 1);
        assert!(arena.replace_state(array, VarState::Array(VarTable::new())));
        assert!(arena.array_default(array).is_none());
    }

    #[test]
    fn static_storage_retains_cells_until_the_last_command_or_activation() {
        let mut arena = VarArena::default();
        let id = arena
            .alloc(VarState::Scalar(Value::string("KEEP")))
            .unwrap();
        arena.bind(id);
        let statics = arena.retain_statics(VarTable::from([("x".to_owned(), id)]));
        let activation = std::rc::Rc::clone(&statics);
        arena.unbind(id);
        drop(statics);
        arena.collect_released_statics();
        assert!(arena.get(id).is_some());
        drop(activation);
        arena.collect_released_statics();
        assert!(arena.get(id).is_none());
    }

    #[test]
    fn replacing_an_array_releases_its_element_bindings() {
        let mut arena = VarArena::default();
        let child = arena
            .alloc(VarState::Scalar(Value::string("child")))
            .expect("child cell");
        arena.bind(child);
        let mut elements = VarTable::new();
        elements.insert("key".to_owned(), child);
        let parent = arena.alloc(VarState::Array(elements)).expect("parent cell");
        arena.bind(parent);

        assert!(arena.replace_state(parent, VarState::Scalar(Value::string("scalar"))));
        assert!(arena.get(child).is_none());
    }

    #[test]
    fn a_link_keeps_its_unbound_target_alive_then_releases_it() {
        let mut arena = VarArena::default();
        let target = arena.alloc(VarState::Undefined).expect("target cell");
        arena.bind(target);
        let link = arena.alloc(VarState::Link(target)).expect("link cell");
        arena.bind(link);

        arena.unbind(target);
        assert_eq!(arena.resolve(link), Some(target));
        arena.unbind(link);
        assert!(arena.get(link).is_none());
        assert!(arena.get(target).is_none());
    }

    #[test]
    fn link_cycles_and_missing_targets_are_rejected() {
        let mut arena = VarArena::default();
        let first = arena.alloc(VarState::Undefined).expect("first cell");
        arena.bind(first);
        let second = arena.alloc(VarState::Link(first)).expect("second cell");
        arena.bind(second);

        assert!(!arena.replace_state(first, VarState::Link(second)));
        assert!(
            arena
                .alloc(VarState::Link(tcl_core_types::VarId(u32::MAX)))
                .is_none()
        );
        assert_eq!(arena.resolve(second), Some(first));
    }

    #[test]
    fn compiled_cell_transfer_retains_receiver_identity_and_one_link_reference() {
        let mut arena = VarArena::default();
        let target = arena
            .alloc(VarState::Scalar(Value::string("TARGET")))
            .unwrap();
        arena.bind(target);
        let receiver = arena.alloc(VarState::Undefined).unwrap();
        arena.bind(receiver);
        let source = arena.alloc(VarState::Link(target)).unwrap();
        arena.unbind(target);
        assert!(arena.transfer_unbound_state(source, receiver));
        arena.discard_unbound(source);
        assert_eq!(arena.resolve(receiver), Some(target));
        assert!(arena.get(target).is_some());
        arena.unbind(receiver);
        assert!(arena.get(receiver).is_none());
        assert!(arena.get(target).is_none());
    }

    #[test]
    fn invalid_transfer_preserves_source_and_compiled_receiver() {
        let mut arena = VarArena::default();
        let receiver = arena
            .alloc(VarState::Scalar(Value::string("ORIGINAL")))
            .unwrap();
        arena.bind(receiver);
        let source = arena.alloc(VarState::Link(receiver)).unwrap();
        assert!(!arena.transfer_unbound_state(source, receiver));
        assert_eq!(arena.resolve(source), Some(receiver));
        assert!(matches!(
            arena.get(receiver).unwrap().state(),
            VarState::Scalar(_)
        ));
        arena.discard_unbound(source);
        arena.unbind(receiver);
        assert!(arena.get(receiver).is_none());
    }

    #[test]
    fn an_uninstalled_allocation_has_a_distinct_discard_path() {
        let mut arena = VarArena::default();
        let id = arena.alloc(VarState::Undefined).expect("cell");
        arena.discard_unbound(id);
        assert!(arena.get(id).is_none());
    }
}

#[cfg(test)]
mod native_table_order_tests {
    use super::*;
    #[test]
    fn physical_variable_table_uses_retained_births_and_original_c_hash_order() {
        let mut table = VarTable::new();
        for index in 0..13_u32 {
            table.insert(format!("k{index:02}"), VarId(index));
        }
        table.select_hash_recipe(Some(tcl_core_types::NativeHashRecipe::Tcl {
            promotion: tcl_core_types::NativeHashBytePromotion::Unsigned,
            width: tcl_core_types::NativeHashWordWidth::Bits32,
        }));
        let keys: Vec<_> = table.keys().map(NameBytes::as_bytes).collect();
        let native = b"k05 k06 k07 k08 k09 k10 k11 k12 k00 k01 k02 k03 k04";
        assert_eq!(keys, native.split(|byte| *byte == b' ').collect::<Vec<_>>());
        table.remove(b"k03");
        table.insert("k03", VarId(100));
        assert_eq!(table.get(b"k03"), Some(&VarId(100)));
        assert_eq!(table.order.borrow().bucket_count(), Some(16));
        assert_eq!(
            table.clone().keys().cloned().collect::<Vec<_>>(),
            table.keys().cloned().collect::<Vec<_>>()
        );
    }
}
