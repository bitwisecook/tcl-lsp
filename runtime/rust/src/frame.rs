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

//! Variable tables + call frames, extended for namespaces.
//!
//! Canonical model: `tclInt.h`'s `Var` is a tagged union
//! `{ scalar objPtr | array tablePtr | linkPtr }` held in a hash table
//! (`tmp/tcl9.0.4/generic/tclVar.c`). In C every variable lives in *some*
//! [`VarTable`]: a proc call frame's locals, or a **namespace** var table
//! (`Namespace.varTable`) — the global frame's table *is* the global
//! namespace's. This module owns the cell mechanics ([`Var`], [`VarTable`]) and
//! the call-frame container ([`FrameStack`]); the **classification + link walk**
//! that ties frames and namespaces together (the variable parallel of the
//! command resolver) lives in [`crate::vars`].
//!
//! Representation decisions (see namespace-tree.md §5.3):
//! - **`BTreeMap`** indexes exact variable and element keys. Native iteration
//!   comes from persistent entry ledgers with independently selected ABI/hash
//!   recipes; compiled declarations keep their own ordered cell inventory.
//! - **C links retain actual cells and element entries**. Stable identities
//!   survive unset and refill; retirement invalidates the old array generation.
//!   Jim links own the original target-name object and weak selected frame,
//!   and repeat that engine's original-object lookup. Reporting paths carry
//!   no authority to replace either selected receiver.
//! - **`VarTable` releases on `Drop`** (matches `TclFreeVar` and keeps every
//!   refcount move visible to the leak counters, `crate::counters`) — so a
//!   dropped frame *or* namespace never leaks, with no hand-written cleanup.
//!
//! Refcount discipline (`memory-management.md` MM-B, `refcount-contract.md`):
//! a scalar cell / array element owns **+1** of its object; storing retains,
//! overwriting/unsetting/dropping releases. Persistent procedure statics retain
//! the same raw cell through `Rc<RefCell<_>>`; removing its ordinary name retires
//! that binding without destroying captured contents. The last cell owner
//! releases its Tcl objects. A captured alias wrapper retains its link value,
//! whose Jim target is still a logical level/name lookup on each access.

use std::cell::{Cell as PolicyCell, Ref, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use tcl_runtime_api::{FrameLinkOrigin, VarId};

use crate::namespace::NsId;
use crate::obj::{self, TclObj};

mod jim_lookup;
pub(crate) use jim_lookup::{JimVariableRead, WeakJimVariableCell};
mod container;
mod destruction;
pub use destruction::{RetainedArrayCell, WeakArraySearchCell};
mod native_element_entry;
pub(crate) use native_element_entry::NativeElementAliasEntry;
#[cfg(test)]
pub(crate) use native_element_entry::NativeElementEntryObserver;
mod rmw;
pub(crate) use rmw::VariableReceiver;
mod native_array_target;
mod native_scalar_alias;
pub(crate) use native_array_target::NativeArrayTargetCell;
mod native_variable_names;
pub(crate) use native_scalar_alias::NativeScalarAliasEntry;

static NEXT_VAR_ID: AtomicU32 = AtomicU32::new(1);

fn fresh_var_id() -> VarId {
    let raw = NEXT_VAR_ID.fetch_add(1, Ordering::Relaxed);
    assert_ne!(raw, u32::MAX, "standalone variable identity exhausted");
    VarId(raw)
}

/// A variable cell: the `tclInt.h` `Var` union as an enum.
pub enum Var {
    /// A scalar — owns **+1** of the object.
    Scalar(*mut TclObj),
    /// An associative array: element key → value (each owns **+1**).
    Array(BTreeMap<Vec<u8>, *mut TclObj>),
    /// C aliases retain variable cells; Jim aliases own their original target name.
    Link(Link),
}

/// Where a variable physically lives: a proc-call frame, or a namespace var
/// table. `upvar` produces `Frame`; `global`/`variable` produce `Namespace`.
/// The global frame and the global namespace share one table, so a level-0
/// frame target is canonicalised to `Namespace(GLOBAL)` at the link site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarHome {
    /// A proc call frame's local table, by absolute level.
    Frame(usize),
    /// A namespace's variable table, by arena id.
    Namespace(NsId),
}

/// A path-resolved alias target (an `upvar`/`global`/`variable` link).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// Jim owns one original recursive target-name object and a weak frame receipt.
    pub(crate) original_jim_target: Option<Rc<OriginalJimLinkTarget>>,
    /// Actual C scalar alias retains its original target entry, not its spelling.
    pub(crate) native_scalar_entry: Option<Rc<NativeScalarAliasEntry>>,
    /// One actual element-entry alias role, independent of borrowed Link views.
    pub(crate) native_element_entry: Option<Rc<NativeElementAliasEntry>>,
    /// Captured C Tcl array-root identity. Jim links follow the selected name.
    pub array_identity: Option<VarId>,
    /// Physical C array generation retained by an element alias.
    pub array_cell: Option<RetainedArrayCell>,
    /// Where the target variable lives.
    pub home: VarHome,
    /// Target variable name (simple, within `home`'s table).
    pub name: Vec<u8>,
    /// Target array element, for `upvar … a(b) x`.
    pub elem: Option<Vec<u8>>,
}

pub(crate) struct OriginalJimLinkTarget {
    pub(crate) name: crate::obj::Owned,
    pub(crate) frame: tcl_runtime_api::jim_call_frame::JimCallFrameStorageReference,
}
impl std::fmt::Debug for OriginalJimLinkTarget {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("OriginalJimLinkTarget")
            .field("original", &self.name.as_ptr())
            .finish_non_exhaustive()
    }
}
impl PartialEq for OriginalJimLinkTarget {
    fn eq(&self, other: &Self) -> bool {
        self.name.as_ptr() == other.name.as_ptr() && self.frame == other.frame
    }
}
impl Eq for OriginalJimLinkTarget {}

impl Var {
    /// Release every object reference this var owns. Consumes `self`; call
    /// exactly once when the cell leaves its table.
    fn release(self) {
        match self {
            // SAFETY: a stored cell owns a +1 on each object; releasing balances
            // the retain taken when it was stored.
            Var::Scalar(p) => unsafe { obj::decr_ref_count(p) },
            Var::Array(map) => {
                for (_, p) in map {
                    unsafe { obj::decr_ref_count(p) }
                }
            }
            Var::Link(_) => {}
        }
    }
}

/// Why a variable write failed (the type-mismatch / namespace cases Tcl reports).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarError {
    /// The actual invocation supplies no variable-name protocol.
    NameProtocolUnavailable,
    /// A captured alias whose namespace token has lost its variable cells.
    DeletedNamespace,
    /// An element alias whose original array root was deleted.
    DeletedArray,
    /// `set a v` where `a` is an array (`can't set "a": variable is array`).
    IsArray,
    /// `set a(k) v` where `a` is a scalar (`can't set "a(k)": variable isn't array`).
    IsScalar,
    /// `set ::nosuch::x v` — a qualified write whose namespace doesn't exist
    /// (`can't set "::nosuch::x": parent namespace doesn't exist`). Only the
    /// *create* path raises this; reads/unsets of the same name report
    /// `no such variable` instead.
    NoSuchNamespace,
    /// A write trace's callback errored: the set fails with `can't set "name":
    /// <msg>`, the trace message carried out-of-band in `TraceTable::pending_err`
    /// (kept unit so `VarError` stays `Copy`; the `var_error` callers propagate
    /// it unchanged).
    TraceError,
    /// A write/unset of a `const` variable (`can't set "name": variable is a
    /// constant`; the command supplies the `set`/`incr`/`unset` verb).
    IsConstant,
}

/// Original indexed alias settlement rejects an occupied or observed local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeCompiledAliasError {
    Unavailable,
    SelfLink,
    Traced,
    Exists,
}

// VarTable — the per-frame / per-namespace name→cell store + cell mechanics.

/// One addressable cell of a [`VarTable`]: the name it is bound to and the
/// variable currently in it, if any.
///
/// `var` is `None` for a *reserved but undefined* cell — a compiled slot bound
/// before its first assignment, or a local that has been `unset`. Reserving
/// rather than removing is what makes a slot index stable: the next write of
/// that name refills this same cell, so an indexed and a named access can never
/// end up looking at two different variables.
struct Cell {
    name: Vec<u8>,
    /// Actual unnamed compiler local, never a dynamic hash-table entry.
    anonymous: bool,
    contents: Rc<RefCell<CellContents>>,
}

struct CellContents {
    /// Native operations holding this exact receiver across guest callbacks.
    rmw_refs: usize,
    /// Scalar-root operations retain the root shell through contents reset.
    rmw_root_refs: usize,
    /// Whole-array teardown invalidates captured elements independently of roots.
    rmw_array_epoch: u64,
    /// Native search invalidation, independent of bucket changes from shell GC.
    array_search_epoch: u64,
    array_searches: tcl_core_types::NativeArraySearchChain<crate::obj::Owned>,
    /// Owner destruction differs from resetting contents of a retained shell.
    rmw_retirement: Option<VarError>,
    var: Option<Var>,
    namespace_declared: bool,
    undefined_shell: bool,
    /// Actual selected home which issued an undefined physical self-link marker.
    undefined_root_home: Option<VarHome>,
    /// The default belongs to this physical array, with two owning references.
    array_default: Option<*mut TclObj>,
    /// Detached array generations whose members retire after the root callback.
    detached_arrays: std::collections::HashMap<VarId, destruction::DetachedArray>,
    /// Identity of the currently bound variable. The slot survives `unset` for
    /// compiled access, while this token normally does not. An in-flight
    /// operation retains the exact Tcl `Var` cell, so recreation during that
    /// operation refills this identity instead of capturing an unrelated one.
    binding_id: Option<VarId>,
    /// This physical cell belongs to the activation's declaration-ordered array.
    compiled_declaration: bool,
    /// In-flight array ensemble operations retaining this binding. A direct
    /// unset leaves the Tcl cell available for same-name recreation until the
    /// final operation releases it, while an ordinary unset starts a new
    /// binding immediately.
    operation_refs: usize,
    native_alias_refs: usize,
    /// Stable identities of the current array elements. Values stay in
    /// [`Var::Array`]; this parallel identity table lets a trace-aware read tell
    /// an unset-and-recreated element from the one selected before its callback.
    element_ids: BTreeMap<Vec<u8>, VarId>,
    member_order: tcl_core_types::NativeEntryLedger,
    native_member_keys: native_element_entry::NativeElementEntries,
    /// Undefined member births owned by original read/modify/write operations.
    rmw_member_shells: BTreeSet<Vec<u8>>,
    /// Original root entry receipt; contains no table or value owning reference.
    rmw_shell_entry: Option<rmw::RootShellEntry>,
    /// Per-element trace reads currently retaining the selected cell. Tcl
    /// refills that cell when its own callback unsets and recreates the element.
    element_operation_refs: BTreeMap<Vec<u8>, usize>,
    /// Flagged `const` (TIP 677): a write or unset errors with `variable is a
    /// constant`. On the cell rather than in a side set so a compiled slot's
    /// write check is the same O(1) index as the write itself.
    constant: bool,
    /// Why a link occupies this frame cell. `TclOO`'s automatic instance
    /// projections participate in `info consts`; ordinary aliases do not.
    link_origin: FrameLinkOrigin,
    /// **The per-cell trace bit**: whether a variable trace can observe this
    /// cell, together with the variable-trace epoch that answer was computed
    /// for.
    ///
    /// The answer itself is derived — resolving the cell's name the way trace
    /// firing does, so an `upvar` link reports its *target*'s traces and an
    /// array reports its elements' — and this caches it. Every add, remove,
    /// frame teardown, and unset that touches the trace set bumps the epoch
    /// through the interpreter's one `VariableTrace` invalidation chokepoint,
    /// so a stale entry is always *recomputed* rather than trusted. A bit
    /// maintained by hand at each of those sites could drift, and a wrong
    /// "untraced" is a silently missed trace.
    ///
    /// Epoch `0` is the never-computed sentinel; the interpreter's epoch starts
    /// at `1`.
    traced: core::cell::Cell<(u64, bool)>,
}

/// A raw physical variable slot whose contents survive removal of its name.
#[derive(Clone)]
pub(crate) struct RetainedVariableCell(Rc<RefCell<CellContents>>);

impl StaticVariables {
    pub(crate) fn insert_capture(&mut self, name: &[u8], cell: RetainedVariableCell) {
        self.cells.insert(name.to_vec(), cell.0);
    }

    pub(crate) fn insert_copy(&mut self, name: &[u8], source: RetainedVariableCell) -> bool {
        let mut table = VarTable::default();
        let source = source.0.borrow();
        match source.var.as_ref() {
            Some(Var::Scalar(value)) => {
                table.store_scalar(name, *value).expect("fresh scalar");
            }
            Some(Var::Array(values)) => {
                table.ensure_array(name).expect("fresh array");
                for (key, value) in values {
                    table.store_elem(name, key, *value).expect("fresh element");
                }
            }
            Some(Var::Link(_)) | None => return false,
        }
        self.insert_capture(name, table.capture_cell(name).expect("initialised cell"));
        true
    }

    pub(crate) fn insert_literal(&mut self, name: &[u8], value: *mut TclObj) {
        let mut table = VarTable::default();
        table
            .store_scalar(name, value)
            .expect("fresh static scalar");
        self.insert_capture(name, table.capture_cell(name).expect("initialised cell"));
    }
}

impl Cell {
    fn empty(name: &[u8]) -> Self {
        Self {
            name: name.to_vec(),
            anonymous: false,
            contents: Rc::new(RefCell::new(CellContents {
                rmw_refs: 0,
                rmw_root_refs: 0,
                rmw_array_epoch: 0,
                array_search_epoch: 0,
                array_searches: tcl_core_types::NativeArraySearchChain::default(),
                detached_arrays: std::collections::HashMap::new(),
                rmw_retirement: None,
                var: None,
                namespace_declared: false,
                undefined_shell: false,
                undefined_root_home: None,
                array_default: None,
                binding_id: None,
                compiled_declaration: false,
                operation_refs: 0,
                native_alias_refs: 0,
                element_ids: BTreeMap::new(),
                member_order: tcl_core_types::NativeEntryLedger::default(),
                native_member_keys: Default::default(),
                rmw_member_shells: BTreeSet::new(),
                rmw_shell_entry: None,
                element_operation_refs: BTreeMap::new(),
                constant: false,
                link_origin: FrameLinkOrigin::Ordinary,
                traced: core::cell::Cell::new((0, false)),
            })),
        }
    }
}

impl CellContents {
    /// A selected undefined self-link is a physical root, not a scalar value.
    /// The original slot spelling is marker geometry, never a fresh lookup.
    fn array_materialisation_needed(&self, root: Option<&[u8]>) -> Result<bool, VarError> {
        match self.var.as_ref() {
            Some(Var::Array(_)) => Ok(false),
            Some(Var::Link(link))
                if root == Some(link.name.as_slice())
                    && self.undefined_root_home == Some(link.home)
                    && link.elem.is_none() =>
            {
                Ok(true)
            }
            Some(_) => Err(VarError::IsScalar),
            None => Ok(true),
        }
    }

    fn materialise_selected_array(&mut self, root: Option<&[u8]>) -> Result<(), VarError> {
        if self.array_materialisation_needed(root)? {
            if let Some(old) = self.var.replace(Var::Array(Default::default())) {
                old.release();
            }
            self.undefined_shell = false;
            self.undefined_root_home = None;
            self.rmw_shell_entry = None;
        }
        Ok(())
    }

    fn insert_member_entry(&mut self, key: &[u8]) {
        if !self.member_order.contains_key(key) {
            self.array_search_epoch = self.array_search_epoch.wrapping_add(1);
            self.array_searches.clear();
        }
        self.member_order.insert(key);
        let id = *self
            .element_ids
            .entry(key.to_vec())
            .or_insert_with(fresh_var_id);
        self.native_member_keys.ensure(id, key, None);
    }

    fn retire_rmw_shell(&mut self) {
        if self.var.is_some()
            || self.operation_refs != 0
            || self.rmw_refs != 0
            || self.native_alias_refs != 0
            || self.namespace_declared
            || self.undefined_shell
            || self.rmw_retirement.is_some()
        {
            return;
        }
        if let Some(entry) = self.rmw_shell_entry.take() {
            entry.retire(self.binding_id);
        }
    }
}

impl Drop for CellContents {
    fn drop(&mut self) {
        if let Some(var) = self.var.take() {
            var.release();
        }
        if let Some(value) = self.array_default.take() {
            unsafe {
                obj::decr_ref_count(value);
                obj::decr_ref_count(value);
            }
        }
        for (_, members) in std::mem::take(&mut self.detached_arrays) {
            Var::Array(members.values).release();
        }
    }
}

/// Persistent raw variable slots retained independently of their original names.
/// A captured alias retains its mutable link cell, rather than resolving/copying
/// its current destination. The last cell owner releases its Tcl objects.
#[derive(Clone, Default)]
pub(crate) struct StaticVariables {
    cells: BTreeMap<Vec<u8>, Rc<RefCell<CellContents>>>,
}

/// A variable table: simple-name → [`Var`] cell, with the refcount discipline
/// for the objects its scalars/arrays own. **Direct** ops only — no link
/// following (that crosses tables and is the [`crate::vars`] coordinator's job).
/// Used by both a call [`Frame`] and a namespace (`namespace.rs`).
///
/// Cells live in a slot-indexed array with an ordered name → slot side table.
/// Named access costs the same one ordered lookup it always did; a compiled
/// local that has resolved its slot once addresses the cell directly, which is
/// what makes `tcl_codegen_slot_get`/`slot_set` O(1) while `info vars`,
/// `upvar`, `trace`, and `unset` still reach that very cell by name.
#[derive(Default)]
pub struct VarTable {
    hash_recipe: PolicyCell<Option<tcl_core_types::NativeHashRecipe>>,
    entry_order: Rc<RefCell<tcl_core_types::NativeEntryLedger>>,
    native_keys: native_variable_names::NativeHashKeys,
    container_model: PolicyCell<tcl_dialect::VariableContainerModel>,
    string_protocol: PolicyCell<Option<tcl_syntax::native_string::NativeStringProtocol>>,
    /// The cells, addressed by a stable slot index. A cell is never moved or
    /// dropped while the table lives, so a slot index stays valid.
    cells: Vec<Cell>,
    /// Simple name → slot. Ordered, so `info vars` / `array names` iterate
    /// deterministically (the reason this was a `BTreeMap` to begin with).
    slots: BTreeMap<Vec<u8>, usize>,
    declared_slots: Vec<usize>,
    compiled_protocol: Option<tcl_syntax::naming::NativeCompiledVariableProtocol>,
    static_slots: BTreeMap<Vec<u8>, usize>,
    statics: Option<Rc<StaticVariables>>,
}

impl VarTable {
    /// Settle an alias in this already-selected physical table cell.
    pub(crate) fn bind_original_alias(
        &mut self,
        name: &[u8],
        link: Link,
        traced: bool,
    ) -> Result<(), NativeCompiledAliasError> {
        let contents = self
            .contents(name)
            .ok_or(NativeCompiledAliasError::Unavailable)?;
        FrameStack::bind_original_alias_cell(contents, link, traced)
    }
    pub(crate) fn set_hash_recipe(&self, recipe: Option<tcl_core_types::NativeHashRecipe>) {
        if self.hash_recipe.replace(recipe) == recipe {
            return;
        }
        self.entry_order.borrow_mut().select_recipe(recipe);
        for cell in &self.cells {
            cell.contents
                .borrow_mut()
                .member_order
                .select_recipe(recipe);
        }
    }

    fn ordered_names(&self) -> Vec<&[u8]> {
        match self.entry_order.borrow().keys() {
            Some(keys) => keys
                .into_iter()
                .filter_map(|key| self.slots.get_key_value(key).map(|(key, _)| key.as_slice()))
                .collect(),
            None => self
                .entry_order
                .borrow()
                .physical_keys()
                .into_iter()
                .filter_map(|key| self.slots.get_key_value(key).map(|(key, _)| key.as_slice()))
                .collect(),
        }
    }

    fn lookup_slot(&self, name: &[u8]) -> Option<usize> {
        if let Some(protocol) = self.compiled_protocol {
            if let Some(slot) = self
                .declared_slots
                .iter()
                .find(|&&slot| protocol.dynamic_local_names_equal(&self.cells[slot].name, name))
            {
                return Some(*slot);
            }
        }
        self.slots
            .get(name)
            .or_else(|| self.static_slots.get(name))
            .copied()
    }

    pub(crate) fn declare_compiled_cell(&mut self, name: &[u8]) -> usize {
        let slot = self.cells.len();
        let cell = Cell::empty(name);
        cell.contents.borrow_mut().compiled_declaration = true;
        cell.contents
            .borrow_mut()
            .member_order
            .select_recipe(self.hash_recipe.get());
        self.cells.push(cell);
        self.declared_slots.push(slot);
        slot
    }

    /// Allocate the actual value cell for one unnamed native compiler slot.
    /// It has no table key, name-lookup entry, or introspection declaration.
    pub(crate) fn declare_compiled_temporary(&mut self) -> usize {
        let slot = self.cells.len();
        let mut cell = Cell::empty(b"");
        cell.anonymous = true;
        cell.contents.borrow_mut().compiled_declaration = true;
        self.cells.push(cell);
        slot
    }

    pub(crate) fn mark_namespace_declared(&mut self, name: &[u8]) {
        if let Some(contents) = self.contents(name) {
            contents.borrow_mut().namespace_declared = true;
        }
    }

    /// The resolved producer retains its actual home, independently of name text.
    pub(crate) fn mark_undefined_root(&mut self, name: &[u8], home: VarHome) -> bool {
        let Some(contents) = self.contents(name) else {
            return false;
        };
        let mut cell = contents.borrow_mut();
        if matches!(cell.var.as_ref(), Some(Var::Link(link))
            if link.home == home && link.name == name && link.elem.is_none())
        {
            cell.undefined_root_home = Some(home);
            return true;
        }
        false
    }

    pub(crate) fn mark_trace_shell(&mut self, name: &[u8], home: VarHome) {
        if self.mark_undefined_root(name, home) {
            self.contents(name).unwrap().borrow_mut().undefined_shell = true;
        }
    }

    pub(crate) fn ensure_element_shell(&mut self, name: &[u8], key: &[u8]) -> Result<(), VarError> {
        self.ensure_array(name)?;
        let Some(contents) = self.contents(name) else {
            return Err(VarError::IsScalar);
        };
        let mut cell = contents.borrow_mut();
        cell.insert_member_entry(key);
        cell.rmw_member_shells.remove(key);
        cell.element_ids
            .entry(key.to_vec())
            .or_insert_with(fresh_var_id);
        Ok(())
    }

    pub(crate) fn cleanup_trace_shell(
        &mut self,
        name: &[u8],
        key: Option<&[u8]>,
        id: Option<VarId>,
    ) {
        let Some(contents) = self.contents(name) else {
            return;
        };
        let mut cell = contents.borrow_mut();
        if cell.binding_id != id {
            return;
        }
        if let Some(key) = key {
            let defined =
                matches!(cell.var.as_ref(), Some(Var::Array(map)) if map.contains_key(key));
            if !defined {
                if !cell.element_operation_refs.contains_key(key)
                    && cell.rmw_refs == 0
                    && !cell.native_member_keys.has_aliases(key)
                {
                    cell.member_order.remove(key);
                    cell.native_member_keys.remove(key);
                    cell.element_ids.remove(key);
                } else {
                    cell.rmw_member_shells.insert(key.to_vec());
                }
            }
        } else if cell.undefined_shell
            && matches!(cell.var.as_ref(), None | Some(Var::Link(_)))
            && !cell.namespace_declared
            && (cell.operation_refs != 0 || cell.rmw_refs != 0 || cell.native_alias_refs != 0)
        {
            if let Some(old) = cell.var.take() {
                old.release();
            }
            cell.undefined_shell = false;
            if let Some(binding) = cell.binding_id {
                if !self
                    .lookup_slot(name)
                    .is_some_and(|slot| self.declared_slots.contains(&slot))
                {
                    cell.rmw_shell_entry = Some(rmw::RootShellEntry::new(
                        &self.entry_order,
                        &self.native_keys,
                        name,
                        binding,
                    ));
                }
            }
        } else if cell.undefined_shell
            && matches!(cell.var.as_ref(), None | Some(Var::Link(_)))
            && !cell.namespace_declared
            && cell.operation_refs == 0
            && cell.rmw_refs == 0
            && Rc::strong_count(contents) == 1
        {
            if let Some(old) = cell.var.take() {
                old.release();
            }
            cell.binding_id = None;
            cell.undefined_shell = false;
            self.retire_native_entry(name);
        }
    }

    fn visible_cell(&self, slot: usize, include_links: bool) -> bool {
        let cell = self.cells[slot].contents.borrow();
        match cell.var.as_ref() {
            Some(Var::Scalar(_) | Var::Array(_)) => true,
            Some(Var::Link(_)) => {
                include_links && (!cell.undefined_shell || cell.namespace_declared)
            }
            None => include_links && cell.namespace_declared,
        }
    }

    /// The slot `name` occupies, reserving an empty one if it has none yet.
    ///
    /// This is the compiled-local binding operation: after it, the slot and the
    /// name are two ways of addressing one cell, for the table's lifetime.
    pub(crate) fn slot_for(&mut self, name: &[u8]) -> usize {
        if let Some(slot) = self.lookup_slot(name) {
            return slot;
        }
        if let Some(contents) = self
            .statics
            .as_ref()
            .and_then(|s| s.cells.get(name))
            .cloned()
        {
            let slot = self.cells.len();
            self.cells.push(Cell {
                name: name.to_vec(),
                anonymous: false,
                contents,
            });
            self.static_slots.insert(name.to_vec(), slot);
            return slot;
        }
        self.local_slot_for(name)
    }

    fn local_slot_for(&mut self, name: &[u8]) -> usize {
        if let Some(slot) = self.lookup_slot(name) {
            return slot;
        }
        self.counted_dynamic_slot(name)
    }

    fn counted_dynamic_slot(&mut self, name: &[u8]) -> usize {
        if let Some(slot) = self.slots.get(name) {
            return *slot;
        }
        let slot = self.cells.len();
        let cell = Cell::empty(name);
        cell.contents
            .borrow_mut()
            .member_order
            .select_recipe(self.hash_recipe.get());
        self.cells.push(cell);
        self.slots.insert(name.to_vec(), slot);
        slot
    }

    fn contents(&self, name: &[u8]) -> Option<&Rc<RefCell<CellContents>>> {
        if let Some(slot) = self.lookup_slot(name) {
            return Some(&self.cells[slot].contents);
        }
        self.statics.as_ref()?.cells.get(name)
    }

    pub(crate) fn capture_cell(&self, name: &[u8]) -> Option<RetainedVariableCell> {
        let contents = self.contents(name)?;
        contents.borrow().var.as_ref()?;
        Some(RetainedVariableCell(contents.clone()))
    }

    pub(crate) fn install_statics(&mut self, statics: Rc<StaticVariables>) {
        self.statics = Some(statics);
    }

    /// Borrow one cell for a synchronous read. Callers release the guard before
    /// dispatching callbacks, so native reentry never retains a cell borrow.
    pub(crate) fn cell_at(&self, slot: usize) -> Option<Ref<'_, Var>> {
        Ref::filter_map(self.cells.get(slot)?.contents.borrow(), |c| c.var.as_ref()).ok()
    }

    pub(crate) fn slot_name(&self, slot: usize) -> Option<&[u8]> {
        self.cells
            .get(slot)
            .filter(|cell| !cell.anonymous)
            .map(|cell| cell.name.as_slice())
    }

    pub(crate) fn cached_trace_flag(&self, slot: usize, epoch: u64) -> Option<bool> {
        let (cached_epoch, traced) = self.cells.get(slot)?.contents.borrow().traced.get();
        (cached_epoch == epoch).then_some(traced)
    }

    pub(crate) fn set_cached_trace_flag(&self, slot: usize, epoch: u64, traced: bool) {
        if let Some(cell) = self.cells.get(slot) {
            cell.contents.borrow().traced.set((epoch, traced));
        }
    }

    fn get(&self, name: &[u8]) -> Option<Ref<'_, Var>> {
        Ref::filter_map(self.contents(name)?.borrow(), |c| c.var.as_ref()).ok()
    }

    fn put(&mut self, name: &[u8], var: Var) -> Option<Var> {
        let slot = self.slot_for(name);
        if !self.declared_slots.contains(&slot) {
            self.entry_order.borrow_mut().insert(name);
        }
        self.put_at(slot, var)
    }

    fn put_at(&mut self, slot: usize, var: Var) -> Option<Var> {
        let mut cell = self.cells[slot].contents.borrow_mut();
        cell.undefined_shell = false;
        cell.undefined_root_home = None;
        cell.rmw_shell_entry = None;
        cell.link_origin = FrameLinkOrigin::Ordinary;
        if cell.binding_id.is_none() {
            cell.binding_id = Some(fresh_var_id());
        }
        cell.element_ids.clear();
        cell.native_member_keys.clear();
        cell.rmw_member_shells.clear();
        cell.element_operation_refs.clear();
        cell.array_search_epoch = cell.array_search_epoch.wrapping_add(1);
        cell.array_searches.clear();
        cell.member_order = tcl_core_types::NativeEntryLedger::default();
        cell.member_order.select_recipe(self.hash_recipe.get());
        if let Var::Array(elements) = &var {
            for key in elements.keys() {
                cell.insert_member_entry(key);
            }
            cell.element_ids
                .extend(elements.keys().cloned().map(|key| (key, fresh_var_id())));
        }
        cell.var.replace(var)
    }

    /// Identity of the variable currently bound at `name`.
    pub(crate) fn binding_id(&self, name: &[u8]) -> Option<VarId> {
        let cell = self.contents(name)?.borrow();
        cell.var.as_ref()?;
        cell.binding_id
    }

    /// Actual allocated binding for trace routing, including undefined roots.
    /// Definition and value-existence queries keep their separate predicates.
    pub(crate) fn trace_binding_identity(&self, name: &[u8]) -> Option<VarId> {
        self.contents(name)?.borrow().binding_id
    }

    /// Existing member allocation of the original root, including undefined shells.
    /// The root's retained identity selects the cell, rather than its display name.
    pub(crate) fn native_trace_element_identity(&self, root: VarId, key: &[u8]) -> Option<VarId> {
        self.cells.iter().find_map(|cell| {
            let contents = cell.contents.borrow();
            (contents.binding_id == Some(root))
                .then(|| contents.element_ids.get(key).copied())
                .flatten()
        })
    }

    /// Identity of a defined element of the current array binding.
    #[cfg(test)]
    pub(crate) fn element_id(&self, name: &[u8], key: &[u8]) -> Option<VarId> {
        let cell = self.contents(name)?.borrow();
        matches!(cell.var.as_ref(), Some(Var::Array(map)) if map.contains_key(key))
            .then(|| cell.element_ids.get(key).copied())
            .flatten()
    }

    /// A real namespace hash cell, including a retained undefined shell.
    pub(crate) fn has_native_namespace_cell(&self, name: &[u8]) -> bool {
        self.lookup_slot(name)
            .is_some_and(|slot| self.cells[slot].contents.borrow().binding_id.is_some())
    }

    pub(crate) fn cell(&self, name: &[u8]) -> Option<Ref<'_, Var>> {
        self.get(name)
    }

    pub(crate) fn is_constant(&self, name: &[u8]) -> bool {
        self.contents(name)
            .is_some_and(|cell| cell.borrow().constant)
    }

    pub(crate) fn const_names(&self) -> Vec<&[u8]> {
        self.slots
            .iter()
            .filter(|(_, slot)| self.cells[**slot].contents.borrow().constant)
            .map(|(name, _)| name.as_slice())
            .collect()
    }

    /// Set array `name`'s default value (`array default set`), releasing any
    /// prior default. The table pins the value with **+2**: one for ownership,
    /// and one so a read of the default always sees it as *shared* — a
    /// read-modify-write (`lappend`/`append`/`dict`) then copies rather than
    /// mutating the stored default in place (TIP 508 copy-on-write).
    pub(crate) fn set_array_default(&mut self, name: &[u8], obj: *mut TclObj) {
        // SAFETY: retain the new default twice, release any prior one twice.
        unsafe {
            obj::incr_ref_count(obj);
            obj::incr_ref_count(obj);
        }
        let slot = self.slot_for(name);
        if let Some(old) = self.cells[slot]
            .contents
            .borrow_mut()
            .array_default
            .replace(obj)
        {
            unsafe {
                obj::decr_ref_count(old);
                obj::decr_ref_count(old);
            }
        }
    }

    /// Array `name`'s default value, if one is set (`array default get`, and the
    /// fallback for a read of a missing element). Borrowed; the table keeps its +1.
    pub(crate) fn array_default(&self, name: &[u8]) -> Option<*mut TclObj> {
        self.contents(name)?.borrow().array_default
    }

    /// Remove array `name`'s default value (`array default unset`).
    pub(crate) fn unset_array_default(&mut self, name: &[u8]) {
        let Some(contents) = self.contents(name) else {
            return;
        };
        if let Some(old) = contents.borrow_mut().array_default.take() {
            // SAFETY: balances the +2 taken in `set_array_default`.
            unsafe {
                obj::decr_ref_count(old);
                obj::decr_ref_count(old);
            }
        }
    }

    /// Flag the scalar `name` `const` (the `const` command, after its value is
    /// stored). A no-op if already flagged.
    pub(crate) fn mark_constant(&mut self, name: &[u8]) {
        let slot = self.slot_for(name);
        self.cells[slot].contents.borrow_mut().constant = true;
    }

    /// `set name value` into this table directly. The cell takes a **+1**.
    pub(crate) fn store_scalar(&mut self, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
        if self.is_constant(name) {
            return Err(VarError::IsConstant);
        }
        if self.dictionary_variables() {
            return self.store_dictionary_root(name, obj);
        }
        let slot = self.slot_for(name);
        let contents = self.cells[slot].contents.clone();
        let mut cell = contents.borrow_mut();
        match cell.var.as_mut() {
            Some(Var::Array(_)) => Err(VarError::IsArray),
            Some(Var::Scalar(old)) => {
                unsafe {
                    obj::incr_ref_count(obj);
                    obj::decr_ref_count(*old);
                }
                *old = obj;
                Ok(())
            }
            Some(Var::Link(link)) if link.name == name && link.elem.is_none() => {
                drop(cell);
                unsafe {
                    obj::incr_ref_count(obj);
                }
                self.put(name, Var::Scalar(obj));
                Ok(())
            }
            Some(Var::Link(_)) => unreachable!("the coordinator never lands on a link"),
            None => {
                drop(cell);
                unsafe {
                    obj::incr_ref_count(obj);
                }
                self.put(name, Var::Scalar(obj));
                Ok(())
            }
        }
    }

    /// `set name` — the scalar's value (borrowed; the table keeps its +1).
    pub(crate) fn load_scalar(&self, name: &[u8]) -> Option<*mut TclObj> {
        match self.get(name).as_deref() {
            Some(Var::Scalar(p)) => Some(*p),
            _ => None,
        }
    }

    /// `set name(key) value`. Errors if `name` is a scalar.
    pub(crate) fn store_elem(
        &mut self,
        name: &[u8],
        key: &[u8],
        obj: *mut TclObj,
    ) -> Result<(), VarError> {
        if self.dictionary_variables() {
            return self.store_dictionary_element(name, key, obj);
        }
        if let Some(contents) = self.contents(name).cloned() {
            let mut borrowed = contents.borrow_mut();
            let cell = &mut *borrowed;
            match cell.var.as_mut() {
                Some(Var::Scalar(_)) => return Err(VarError::IsScalar),
                Some(Var::Array(map)) => {
                    // SAFETY: retain the new element value; release any prior one.
                    unsafe { obj::incr_ref_count(obj) };
                    let fresh = !map.contains_key(key);
                    if let Some(old) = map.insert(key.to_vec(), obj) {
                        unsafe { obj::decr_ref_count(old) };
                    }
                    cell.insert_member_entry(key);
                    cell.rmw_member_shells.remove(key);
                    if fresh && !cell.element_ids.contains_key(key) {
                        cell.element_ids.insert(key.to_vec(), fresh_var_id());
                    }
                    return Ok(());
                }
                // The declared-but-undefined marker: the first element write
                // defines the array over it (see `store_scalar`).
                Some(Var::Link(l)) if l.name == name && l.elem.is_none() => {}
                Some(Var::Link(_)) => unreachable!("the coordinator never lands on a link"),
                None => {}
            }
        }
        let mut map = BTreeMap::new();
        unsafe { obj::incr_ref_count(obj) };
        map.insert(key.to_vec(), obj);
        self.put(name, Var::Array(map));
        Ok(())
    }

    /// `set name(key)` — borrowed.
    pub(crate) fn load_elem(&self, name: &[u8], key: &[u8]) -> Option<*mut TclObj> {
        if self.dictionary_variables() {
            let root = self.load_scalar(name)?;
            self.prepare_dictionary_root(root).ok()?;
            return crate::dict::dict_get(root, key).ok().flatten();
        }
        match self.get(name).as_deref() {
            Some(Var::Array(map)) => map.get(key).copied(),
            _ => None,
        }
    }

    /// Remove the whole variable `name` (scalar or array); returns whether it
    /// existed. Releases every object it owned.
    pub(crate) fn remove(&mut self, name: &[u8]) -> bool {
        let Some(slot) = self.lookup_slot(name) else {
            return false;
        };
        self.unset_array_default(name);
        {
            let mut cell = self.cells[slot].contents.borrow_mut();
            cell.array_search_epoch = cell.array_search_epoch.wrapping_add(1);
            cell.array_searches.clear();
        }
        self.retire_jim_key(name);
        self.cells[slot].contents.borrow_mut().namespace_declared = false;
        self.cells[slot].contents.borrow_mut().undefined_shell = false;
        {
            let cell = self.cells[slot].contents.borrow();
            if cell.operation_refs == 0 && cell.rmw_root_refs == 0 && cell.native_alias_refs == 0 {
                self.retire_native_entry(name);
            }
        }
        if self.cells[slot].contents.borrow().rmw_refs != 0
            || self.cells[slot].contents.borrow().native_alias_refs != 0
        {
            let is_array = matches!(
                self.cells[slot].contents.borrow().var.as_ref(),
                Some(Var::Array(_))
            );
            let mut contents = self.cells[slot].contents.borrow_mut();
            if (contents.rmw_root_refs != 0 || contents.native_alias_refs != 0)
                && contents.rmw_shell_entry.is_none()
                && !self.declared_slots.contains(&slot)
            {
                if let Some(binding) = contents.binding_id {
                    contents.rmw_shell_entry = Some(rmw::RootShellEntry::new(
                        &self.entry_order,
                        &self.native_keys,
                        name,
                        binding,
                    ));
                }
            }
            let old = contents.var.take();
            contents.native_member_keys.clear();
            let existed = old.is_some();
            if let Some(old) = old {
                old.release();
            }
            if is_array {
                contents.rmw_array_epoch = contents.rmw_array_epoch.wrapping_add(1);
                if contents.rmw_root_refs == 0 && contents.native_alias_refs == 0 {
                    contents.rmw_retirement = Some(VarError::DeletedArray);
                    drop(contents);
                    self.cells[slot] = Cell::empty(name);
                }
            }
            return existed;
        }
        if self.cells[slot].contents.borrow().operation_refs != 0 {
            let mut cell = self.cells[slot].contents.borrow_mut();
            cell.element_ids.clear();
            cell.native_member_keys.clear();
            cell.element_operation_refs.clear();
            return cell
                .var
                .take()
                .map(|var| {
                    var.release();
                    true
                })
                .unwrap_or(false);
        }
        if Rc::strong_count(&self.cells[slot].contents) > 1 {
            let existed = self.cells[slot].contents.borrow().var.is_some();
            self.cells[slot] = Cell::empty(name);
            return existed;
        }
        let mut cell = self.cells[slot].contents.borrow_mut();
        if cell.operation_refs == 0 {
            cell.binding_id = None;
        }
        cell.element_ids.clear();
        cell.native_member_keys.clear();
        cell.rmw_member_shells.clear();
        cell.element_operation_refs.clear();
        if let Some(var) = cell.var.take() {
            var.release();
            true
        } else {
            false
        }
    }

    /// Ensure `name` is an (at least empty) array — `array default set` creates
    /// the array even with no elements. Returns `Err(IsScalar)` if `name` is a
    /// scalar. (`IsScalar` reuses the closest existing variant; the caller maps
    /// it to the `array default set` message.)
    pub(crate) fn ensure_array(&mut self, name: &[u8]) -> Result<(), VarError> {
        if self.dictionary_variables() {
            return self.ensure_dictionary_root(name);
        }
        let contents = self.contents(name).cloned();
        if let Some(contents) = contents {
            let cell = contents.borrow();
            if !cell.array_materialisation_needed(Some(name))? {
                return Ok(());
            }
        }
        self.put(name, Var::Array(BTreeMap::new()));
        Ok(())
    }

    /// Remove one array element `name(key)`; returns whether it existed.
    pub(crate) fn remove_elem(&mut self, name: &[u8], key: &[u8]) -> bool {
        if self.dictionary_variables() {
            return self.remove_dictionary_element(name, key);
        }
        let Some(contents) = self.contents(name) else {
            return false;
        };
        let mut borrowed = contents.borrow_mut();
        let cell = &mut *borrowed;
        if cell.member_order.contains_key(key) {
            cell.array_search_epoch = cell.array_search_epoch.wrapping_add(1);
            cell.array_searches.clear();
        }
        if let Some(Var::Array(map)) = cell.var.as_mut() {
            if let Some(old) = map.remove(key) {
                if !cell.element_operation_refs.contains_key(key)
                    && !cell.native_member_keys.has_aliases(key)
                {
                    cell.element_ids.remove(key);
                    cell.member_order.remove(key);
                    cell.native_member_keys.remove(key);
                }
                // SAFETY: the array element owned a +1; releasing balances it.
                unsafe { obj::decr_ref_count(old) };
                return true;
            }
        }
        false
    }

    /// Install `link` under `name`, releasing any cell it replaces.
    pub(crate) fn insert_link(&mut self, name: &[u8], link: Link) {
        self.insert_link_with_origin(name, link, FrameLinkOrigin::Ordinary);
    }

    /// Install a link with its typed frame-binding origin.
    pub(crate) fn insert_link_with_origin(
        &mut self,
        name: &[u8],
        link: Link,
        origin: FrameLinkOrigin,
    ) {
        let slot = self.local_slot_for(name);
        if !self.declared_slots.contains(&slot) {
            self.entry_order.borrow_mut().insert(name);
        }
        let mut cell = self.cells[slot].contents.borrow_mut();
        cell.undefined_shell = false;
        cell.undefined_root_home = None;
        cell.link_origin = origin;
        if cell.binding_id.is_none() {
            cell.binding_id = Some(fresh_var_id());
        }
        cell.element_ids.clear();
        cell.native_member_keys.clear();
        cell.rmw_member_shells.clear();
        cell.element_operation_refs.clear();
        if let Some(old) = cell.var.replace(Var::Link(link)) {
            old.release();
        }
    }

    /// Whether `name` is an array variable here (the `set a` array-vs-scalar
    /// diagnostic; `array exists`).
    pub(crate) fn is_array(&self, name: &[u8]) -> bool {
        if self.dictionary_variables() {
            return self
                .load_scalar(name)
                .is_some_and(|value| self.prepare_dictionary_root(value).is_ok());
        }
        matches!(self.get(name).as_deref(), Some(Var::Array(_)))
    }

    /// Whether `name` is a defined scalar or array here (not a link) — the
    /// terminal check behind `info exists`.
    pub(crate) fn is_set(&self, name: &[u8]) -> bool {
        matches!(
            self.get(name).as_deref(),
            Some(Var::Scalar(_) | Var::Array(_))
        )
    }

    pub(crate) fn teardown_names(&self) -> Vec<&[u8]> {
        self.declared_slots
            .iter()
            .copied()
            .filter(|&slot| self.cells[slot].contents.borrow().binding_id.is_some())
            .map(|slot| self.cells[slot].name.as_slice())
            .chain(self.ordered_names())
            .collect()
    }

    /// Defined cells and declared namespace variables in original physical entry order.
    pub(crate) fn names(&self) -> Vec<&[u8]> {
        self.declared_slots
            .iter()
            .copied()
            .filter(|&slot| self.visible_cell(slot, true))
            .map(|slot| self.cells[slot].name.as_slice())
            .chain(self.ordered_names().into_iter().filter(|name| {
                self.lookup_slot(name)
                    .is_some_and(|slot| self.visible_cell(slot, true))
            }))
            .collect()
    }

    /// Defined direct compiled declarations followed by the dynamic hash table.
    pub(crate) fn non_link_names(&self) -> Vec<&[u8]> {
        self.declared_slots
            .iter()
            .copied()
            .filter(|&slot| self.visible_cell(slot, false))
            .map(|slot| self.cells[slot].name.as_slice())
            .chain(self.ordered_names().into_iter().filter(|name| {
                self.lookup_slot(name)
                    .is_some_and(|slot| self.visible_cell(slot, false))
            }))
            .collect()
    }

    pub(crate) fn array_names(&self, name: &[u8]) -> Option<Vec<Vec<u8>>> {
        if self.dictionary_variables() {
            let root = self.load_scalar(name)?;
            self.prepare_dictionary_root(root).ok()?;
            return crate::dict::dict_keys(root)
                .ok()
                .map(|keys| keys.into_iter().map(obj::bytes_of).collect());
        }
        match self.get(name).as_deref() {
            Some(Var::Array(map)) => {
                let cell = self.contents(name)?.borrow();
                Some(match cell.member_order.keys() {
                    Some(keys) => keys
                        .into_iter()
                        .filter(|key| map.contains_key(*key))
                        .map(<[u8]>::to_vec)
                        .collect(),
                    None => map.keys().cloned().collect(),
                })
            }
            _ => None,
        }
    }
}

impl Drop for VarTable {
    /// Release every object every cell owns, so a dropped frame/namespace never
    /// leaks.
    fn drop(&mut self) {
        for cell in &self.cells {
            let mut contents = cell.contents.borrow_mut();
            if contents.rmw_refs != 0
                || contents.native_alias_refs != 0
                || contents.native_member_keys.any_aliases()
            {
                contents.rmw_retirement = Some(VarError::DeletedNamespace);
                contents.native_member_keys.clear();
                if let Some(old) = contents.var.take() {
                    old.release();
                }
            }
        }
    }
}

// FrameStack — the proc call frames (level 0 is the global context).

/// One call frame: its local variable table, absolute level, and the namespace
/// it runs in (so `uplevel` can restore the target frame's namespace context —
/// the `CallFrame.nsPtr` analogue).
pub(crate) struct PendingTailcall {
    pub(crate) namespace: NsId,
    pub(crate) namespace_name: Vec<u8>,
    pub(crate) words: Vec<crate::obj::Owned>,
    pub(crate) original_list: Option<crate::obj::Owned>,
}

struct Frame {
    jim_link_birth: Rc<()>,
    jim_storage: tcl_runtime_api::jim_call_frame::JimCallFrameStorageSlot,
    c_procedure: Option<Rc<crate::interp::ProcDef>>,
    oo_variable_resolver: Option<crate::cmd_oo::native_variables::NativeOoVariableResolver>,
    native_procedure_execution: Option<
        tcl_runtime_api::native_procedure_roles::NativeProcedureReference<crate::interp::ProcDef>,
    >,
    native_local_names:
        Option<Rc<tcl_runtime_api::native_literal::NativeLocalNameTable<crate::obj::Owned>>>,
    jim_parameters: Option<crate::obj::Owned>,
    jim_body: Option<crate::obj::Owned>,
    jim_namespace: Option<Rc<crate::obj::Owned>>,
    jim_local_commands: Vec<crate::obj::Owned>,
    jim_id: u64,
    tailcall: Option<PendingTailcall>,
    table: VarTable,
    /// Compile-time slot index → the [`VarTable`] cell slot it is bound to.
    /// Indexed and named access address the *same* cell, so dynamic Tcl code
    /// (`info vars`, `upvar`, `trace`, `unset`) observes exactly what generated
    /// `tcl_codegen_slot_*` calls do — and an indexed access costs one array
    /// index rather than a name clone plus an ordered lookup.
    compiled_slots: Vec<Option<usize>>,
    original_error_stack_argv: Option<Vec<*mut TclObj>>,
    /// The logical call level (`info level`, `upvar`/`uplevel` arithmetic) — the
    /// invoking var-frame's level + 1, **not** the stack index. They diverge
    /// when a proc is invoked while `uplevel` has redirected the active frame:
    /// e.g. `uplevel 1 [list SomeProc …]` (tcltest's idiom) pushes `SomeProc` at
    /// a deeper stack index but logical level `target+1`.
    level: usize,
    ns: NsId,
    /// The command words that invoked this frame (`info level N`): the proc name
    /// followed by its arguments. Empty for the global frame.
    words: Vec<Vec<u8>>,
    /// Whether this is a *proc* call frame (its own local variables) versus a
    /// *namespace* frame (`namespace eval`/`inscope` — unqualified names resolve
    /// to the namespace, not frame-local). The global frame is non-proc.
    is_proc: bool,
    /// The `active_level` to restore when this frame is popped (the var-frame in
    /// effect when it was pushed). Restores correctly even when the push
    /// happened under an `uplevel` redirection.
    saved_active: usize,
}

impl Drop for Frame {
    fn drop(&mut self) {
        drop(self.native_procedure_execution.take());
        tcl_runtime_api::jim_interpreter::release_jim_call_frame_objects(
            self.jim_parameters.take(),
            self.jim_body.take(),
            self.jim_namespace.take(),
        );
    }
}

impl Frame {
    fn new(level: usize, ns: NsId) -> Self {
        Frame {
            jim_link_birth: Rc::new(()),
            jim_storage: tcl_runtime_api::jim_call_frame::JimCallFrameStorageSlot::default(),
            c_procedure: None,
            oo_variable_resolver: None,
            native_procedure_execution: None,
            native_local_names: None,
            jim_parameters: None,
            jim_body: None,
            jim_namespace: None,
            jim_local_commands: Vec::new(),
            jim_id: 1,
            tailcall: None,
            table: VarTable::default(),
            compiled_slots: Vec::new(),
            original_error_stack_argv: None,
            level,
            ns,
            words: Vec::new(),
            is_proc: false,
            saved_active: 0,
        }
    }
}

/// The call-frame stack. `frames[0]` is the global *level* (its own table stays
/// empty — global variables live in the global **namespace's** table, which the
/// coordinator routes to). `active_level` is the frame whose variables are
/// currently visible (the `varFramePtr` analogue): normally the top, but
/// `uplevel` points it at an enclosing frame for the duration of a body. Frame
/// index == level.
pub struct FrameStack {
    next_jim_frame_id: u64,
    jim_storage: tcl_runtime_api::jim_call_frame::JimCallFrameStoragePool,
    pub(crate) variable_container_model: tcl_dialect::VariableContainerModel,
    pub(crate) variable_string_protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    pub(crate) variable_hash_recipe: Option<tcl_core_types::NativeHashRecipe>,
    pub(crate) variable_lookup_policy: tcl_dialect::VariableLookupPolicy,
    frames: Vec<Frame>,
    active_level: usize,
}

impl Default for FrameStack {
    fn default() -> Self {
        Self::new()
    }
}

/// A departing frame's concrete local and namespace owners.
pub(crate) struct RetiredJimFrame {
    _frame: Frame,
}

impl RetiredJimFrame {
    pub(crate) fn release(self) -> tcl_runtime_api::jim_call_frame::JimCallFrameStorageSlot {
        let storage = self._frame.jim_storage.clone();
        drop(self);
        storage
    }
}

impl FrameStack {
    pub(crate) fn recycle_jim_storage(
        &mut self,
        storage: tcl_runtime_api::jim_call_frame::JimCallFrameStorageSlot,
    ) {
        if self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim {
            self.jim_storage.recycle(storage);
        }
    }
    pub(crate) fn jim_link_storage(
        &self,
        level: usize,
    ) -> Option<tcl_runtime_api::jim_call_frame::JimCallFrameStorageReference> {
        Some(
            self.frames
                .get(self.index_of_level(level)?)?
                .jim_storage
                .reference(),
        )
    }
    pub(crate) fn jim_link_level(
        &self,
        retained: &tcl_runtime_api::jim_call_frame::JimCallFrameStorageReference,
    ) -> Option<usize> {
        let frame = self
            .frames
            .iter()
            .find(|frame| frame.jim_storage.matches(retained))?;
        self.frames
            .get(self.index_of_level(frame.level)?)
            .filter(|selected| selected.jim_storage.matches(retained))
            .map(|_| frame.level)
    }
    pub(crate) fn retain_c_procedure(&mut self, procedure: Rc<crate::interp::ProcDef>) {
        self.frames
            .last_mut()
            .expect("actual C activation")
            .c_procedure = Some(procedure);
    }

    pub(crate) fn retain_native_procedure_execution(
        &mut self,
        role: tcl_runtime_api::native_procedure_roles::NativeProcedureReference<
            crate::interp::ProcDef,
        >,
    ) {
        let frame = self.frames.last_mut().expect("actual procedure activation");
        assert!(frame.native_procedure_execution.is_none());
        frame.native_procedure_execution = Some(role);
    }

    pub(crate) fn take_native_procedure_execution(
        &mut self,
    ) -> Option<
        tcl_runtime_api::native_procedure_roles::NativeProcedureReference<crate::interp::ProcDef>,
    > {
        self.frames.last_mut()?.native_procedure_execution.take()
    }

    pub(crate) fn current_c_procedure(&self) -> Option<&Rc<crate::interp::ProcDef>> {
        self.frames
            .get(self.current_frame_index())?
            .c_procedure
            .as_ref()
    }

    #[cfg(test)]
    pub(crate) fn jim_activation_objects(
        &self,
    ) -> Option<(
        *mut crate::obj::TclObj,
        *mut crate::obj::TclObj,
        *mut crate::obj::TclObj,
    )> {
        let frame = self.frames.last()?;
        Some((
            frame.jim_parameters.as_ref()?.as_ptr(),
            frame.jim_body.as_ref()?.as_ptr(),
            frame.jim_namespace.as_ref()?.as_ptr(),
        ))
    }

    pub(crate) fn retain_jim_frame_body(&mut self, body: crate::obj::Owned) {
        self.frames
            .last_mut()
            .expect("native namespace frame")
            .jim_body = Some(body);
    }

    pub(crate) fn retain_jim_root_namespace(&mut self, namespace: Rc<crate::obj::Owned>) {
        self.frames.first_mut().expect("global frame").jim_namespace = Some(namespace);
    }

    pub(crate) fn retain_jim_activation_objects(
        &mut self,
        parameters: Option<crate::obj::Owned>,
        body: Option<crate::obj::Owned>,
        namespace: Rc<crate::obj::Owned>,
    ) {
        let frame = self.frames.last_mut().expect("actual activation frame");
        frame.jim_parameters = parameters;
        frame.jim_body = body;
        frame.jim_namespace = Some(namespace);
    }

    pub(crate) fn take_frame_for_pop(&mut self) -> Option<(NsId, RetiredJimFrame)> {
        if self.frames.len() <= 1 {
            return None;
        }
        let frame = self.frames.pop().expect("non-global frame");
        self.active_level = frame.saved_active;
        Some((frame.ns, RetiredJimFrame { _frame: frame }))
    }

    pub(crate) fn select_top_for_jim_retirement(&mut self) -> Option<usize> {
        let level = self.frames.last()?.level;
        self.active_level = level;
        Some(level)
    }

    pub(crate) fn take_top_for_jim_retirement(&mut self) -> Option<RetiredJimFrame> {
        let frame = self.frames.pop()?;
        self.active_level = self.frames.last().map_or(0, |frame| frame.level);
        Some(RetiredJimFrame { _frame: frame })
    }

    /// A new stack with just the global level (0), in the global namespace.
    pub fn new() -> Self {
        FrameStack {
            next_jim_frame_id: 2,
            jim_storage: tcl_runtime_api::jim_call_frame::JimCallFrameStoragePool::default(),
            variable_container_model: tcl_dialect::VariableContainerModel::DistinctArray,
            variable_string_protocol: None,
            variable_hash_recipe: None,
            variable_lookup_policy: tcl_dialect::VariableLookupPolicy::Tcl,
            frames: vec![Frame::new(0, crate::namespace::GLOBAL)],
            active_level: 0,
        }
    }

    pub(crate) fn retain_jim_local_command(&mut self, name: crate::obj::Owned) {
        let index = self.current_frame_index();
        self.frames[index].jim_local_commands.push(name);
    }

    pub(crate) fn take_jim_local_commands(&mut self) -> Vec<crate::obj::Owned> {
        let index = self.current_frame_index();
        std::mem::take(&mut self.frames[index].jim_local_commands)
    }

    /// Install the resolver only in the actual reached method activation.
    pub(crate) fn install_tcloo_variable_resolver(
        &mut self,
        resolver: crate::cmd_oo::native_variables::NativeOoVariableResolver,
    ) -> Result<(), VarError> {
        let frame = self
            .frames
            .last_mut()
            .ok_or(VarError::NameProtocolUnavailable)?;
        if !frame.is_proc || frame.ns != resolver.namespace() {
            return Err(VarError::NameProtocolUnavailable);
        }
        frame.oo_variable_resolver = Some(resolver);
        Ok(())
    }

    /// Runtime CString resolution is independent of the installed local table.
    pub(crate) fn tcloo_variable_target(
        &self,
        level: usize,
        protocol: tcl_syntax::naming::NativeNameProtocol,
        supplied: &[u8],
    ) -> Result<Option<(NsId, Vec<u8>)>, VarError> {
        let Some(frame) = self
            .index_of_level(level)
            .and_then(|index| self.frames.get(index))
        else {
            return Err(VarError::NameProtocolUnavailable);
        };
        let Some(resolver) = &frame.oo_variable_resolver else {
            return Ok(None);
        };
        if !frame.is_proc || frame.ns != resolver.namespace() {
            return Err(VarError::NameProtocolUnavailable);
        }
        resolver
            .runtime_target(protocol, supplied)
            .map_err(|_| VarError::NameProtocolUnavailable)
    }

    pub(crate) fn declared_tcloo_variable_bindings(
        &self,
    ) -> Result<Vec<crate::cmd_oo::native_variables::DeclaredVariableBinding>, VarError> {
        let Some(frame) = self
            .index_of_level(self.active_level)
            .and_then(|index| self.frames.get(index))
        else {
            return Err(VarError::NameProtocolUnavailable);
        };
        frame.oo_variable_resolver.as_ref().map_or_else(
            || Ok(Vec::new()),
            |resolver| {
                if !frame.is_proc || frame.ns != resolver.namespace() {
                    return Err(VarError::NameProtocolUnavailable);
                }
                resolver
                    .reported_bindings()
                    .map_err(|_| VarError::NameProtocolUnavailable)
            },
        )
    }

    pub(crate) fn declared_tcloo_variable_names(&self) -> Result<Vec<Vec<u8>>, VarError> {
        self.declared_tcloo_variable_bindings()
            .map(|bindings| bindings.into_iter().map(|(_, name, _)| name).collect())
    }

    /// The active logical variable frame level (`varFramePtr`) used by names.
    pub fn current_level(&self) -> usize {
        self.active_level
    }

    /// Whether the active frame is a proc call frame. Unqualified names resolve
    /// frame-local only here; at global / `namespace eval` scope they resolve to
    /// the current namespace.
    pub fn in_proc(&self) -> bool {
        self.index_of_level(self.active_level)
            .is_some_and(|i| self.frames[i].is_proc)
    }

    /// Whether the frame at logical `level` is a proc call frame (vs the global
    /// or a `namespace eval` frame) — the frame-addressed analogue of
    /// [`in_proc`](Self::in_proc), for `FrameId`-addressed variable access.
    pub(crate) fn is_proc_at(&self, level: usize) -> bool {
        self.index_of_level(level)
            .is_some_and(|i| self.frames[i].is_proc)
    }

    /// Whether the selected engine stores ordinary variables in this activation.
    pub(crate) fn owns_local_variables_at(&self, level: usize) -> bool {
        self.is_proc_at(level)
            || (self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim && level > 0)
    }

    /// The true top-of-stack level (`framePtr`), independent of any `uplevel`
    /// redirection of the active level.
    pub fn top_level(&self) -> usize {
        self.frames.last().map_or(0, |f| f.level)
    }

    /// Push a new proc call frame running in namespace `ns`; returns its level
    /// and makes it the active frame.
    pub fn push(&mut self, ns: NsId) -> usize {
        let level = self.active_level + 1;
        let mut f = Frame::new(level, ns);
        if self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim {
            f.jim_storage = self.jim_storage.acquire();
        }
        f.jim_id = self.next_jim_frame_id;
        self.next_jim_frame_id = self
            .next_jim_frame_id
            .checked_add(1)
            .expect("Jim frame identity exhausted");
        f.table.set_hash_recipe(self.variable_hash_recipe);
        f.table
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        f.is_proc = true;
        f.saved_active = self.active_level;
        self.frames.push(f);
        self.active_level = level;
        level
    }

    /// Push a proc call frame that shares the *current* level (it still gets its
    /// own local-variable table). Used for a TclOO method invoked via `next`:
    /// the whole call chain runs at the level of the original invocation, so
    /// `info level` / `upvar` / `uplevel` resolve through the chain. Returns the
    /// (unchanged) level; the new frame becomes active and, being topmost at
    /// that level, is what unqualified names and `info level` resolve to.
    pub fn push_same_level(&mut self, ns: NsId) -> usize {
        let level = self.active_level;
        let mut f = Frame::new(level, ns);
        if self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim {
            f.jim_storage = self.jim_storage.acquire();
        }
        f.jim_id = self.next_jim_frame_id;
        self.next_jim_frame_id = self
            .next_jim_frame_id
            .checked_add(1)
            .expect("Jim frame identity exhausted");
        f.table.set_hash_recipe(self.variable_hash_recipe);
        f.table
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        f.is_proc = true;
        f.saved_active = self.active_level;
        self.frames.push(f);
        self.active_level = level;
        level
    }

    /// Push a *namespace* frame (`namespace eval`/`inscope`): a new scope whose
    /// unqualified variables resolve to the namespace (not frame-local). Returns
    /// its level and makes it active.
    pub fn push_namespace(&mut self, ns: NsId) -> usize {
        let level = self.active_level + 1;
        let mut f = Frame::new(level, ns);
        if self.variable_lookup_policy == tcl_dialect::VariableLookupPolicy::Jim {
            f.jim_storage = self.jim_storage.acquire();
        }
        f.jim_id = self.next_jim_frame_id;
        self.next_jim_frame_id = self
            .next_jim_frame_id
            .checked_add(1)
            .expect("Jim frame identity exhausted");
        f.table.set_hash_recipe(self.variable_hash_recipe);
        f.table
            .set_container_model(self.variable_container_model, self.variable_string_protocol); // is_proc = false
        f.saved_active = self.active_level;
        self.frames.push(f);
        self.active_level = level;
        level
    }

    /// Pop the current (top) frame, releasing its locals (via `VarTable::drop`),
    /// and restore the active level to whatever was in effect when it was pushed
    /// (correct even under an `uplevel` redirection). The global level is never
    /// popped. Returns the namespace the popped frame was running in — the
    /// activation C's `Tcl_PopCallFrame` gives back to the namespace token.
    pub fn pop(&mut self) -> Option<NsId> {
        let (namespace, owners) = self.take_frame_for_pop()?;
        let storage = owners.release();
        self.recycle_jim_storage(storage);
        Some(namespace)
    }

    /// The stack index of the topmost frame with logical `level` (levels are not
    /// stack indices once `uplevel` is in play; the most recent wins).
    fn index_of_level(&self, level: usize) -> Option<usize> {
        self.frames.iter().rposition(|f| f.level == level)
    }

    /// The stack index of the current active (var) frame — the identity a new
    /// `CmdFrame` records as its CallFrame (C's `framePtr->framePtr`).
    pub(crate) fn current_frame_index(&self) -> usize {
        self.index_of_level(self.active_level).unwrap_or(0)
    }

    /// Issued activation identity of the current variable frame, including uplevel selection.
    pub(crate) fn current_activation(&self) -> u64 {
        self.frames[self.current_frame_index()].jim_id
    }

    pub(crate) fn current_activation_owner(&self) -> std::rc::Weak<()> {
        Rc::downgrade(&self.frames[self.current_frame_index()].jim_link_birth)
    }

    /// The set of stack indices on the active frame's caller chain (C's
    /// `callerVarPtr` walk from `varFramePtr`). Each frame's caller is the topmost
    /// frame *below* it at the level that was active when it was pushed
    /// (`saved_active`) — so an `uplevel`-redirected call's chain skips the frame
    /// it bypassed, even when that frame shares a level. Used by `info frame` to
    /// decide whether to report a frame's `level`.
    pub(crate) fn caller_chain_indices(&self) -> std::collections::HashSet<usize> {
        let mut set = std::collections::HashSet::new();
        let mut idx = self.current_frame_index();
        loop {
            if !set.insert(idx) || idx == 0 {
                break;
            }
            let caller_level = self.frames[idx].saved_active;
            match self.frames[..idx]
                .iter()
                .rposition(|f| f.level == caller_level)
            {
                Some(c) => idx = c,
                None => break,
            }
        }
        set
    }

    /// Redirect the active variable frame to `level` (for `uplevel`), returning
    /// the previous active level so the caller can restore it.
    pub fn set_active_level(&mut self, level: usize) -> usize {
        let prev = self.active_level;
        self.active_level = level;
        prev
    }

    /// Move retired owners out of the frame before their native free callbacks run.
    pub(crate) fn replace_tailcall(
        &mut self,
        request: Option<PendingTailcall>,
    ) -> Result<Option<PendingTailcall>, Option<PendingTailcall>> {
        let Some(index) = self.index_of_level(self.active_level) else {
            return Err(request);
        };
        if !self.frames[index].is_proc {
            return Err(request);
        }
        Ok(std::mem::replace(&mut self.frames[index].tailcall, request))
    }

    pub(crate) fn take_tailcall(&mut self) -> Option<PendingTailcall> {
        self.frames.last_mut()?.tailcall.take()
    }

    /// Record the invoking command words on the top frame (`info level N`).
    pub(crate) fn set_words(&mut self, words: Vec<Vec<u8>>) {
        if let Some(f) = self.frames.last_mut() {
            f.words = words;
        }
    }

    /// Original top call-frame invocation and its active-variable-frame redirect.
    /// Stack identity is retained even when two TclOO frames share a level.
    pub(crate) fn error_stack_frame(&self) -> Option<(Option<usize>, &[Vec<u8>])> {
        let top_index = self.frames.len().checked_sub(1)?;
        let top = &self.frames[top_index];
        if top
            .original_error_stack_argv
            .as_ref()
            .map_or_else(|| top.words.is_empty(), Vec::is_empty)
        {
            return None;
        }
        let delta = if self.current_frame_index() != top_index {
            Some(top.level.saturating_sub(self.active_level))
        } else {
            None
        };
        Some((delta, &top.words))
    }

    /// The command words that invoked frame `level` (`info level N`); empty if
    /// the level has none (e.g. the global frame).
    pub(crate) fn words_at(&self, level: usize) -> Option<&[Vec<u8>]> {
        let i = self.index_of_level(level)?;
        Some(self.frames[i].words.as_slice())
    }

    /// The namespace frame `level` runs in (for `uplevel` ns restoration).
    pub fn frame_ns(&self, level: usize) -> NsId {
        self.index_of_level(level)
            .map_or(crate::namespace::GLOBAL, |i| self.frames[i].ns)
    }

    /// Temporarily change the existing variable frame's lookup namespace.
    pub(crate) fn replace_active_namespace(&mut self, namespace: NsId) -> NsId {
        let Some(index) = self.index_of_level(self.active_level) else {
            return crate::namespace::GLOBAL;
        };
        std::mem::replace(&mut self.frames[index].ns, namespace)
    }

    /// Bind a generated-code slot to `name`'s cell in the active frame,
    /// reserving that cell if the name has none yet.
    pub(crate) fn install_formal_cells(
        &mut self,
        names: &[Vec<u8>],
        protocol: tcl_syntax::naming::NativeCompiledVariableProtocol,
    ) {
        if !protocol.has_indexed_locals() {
            return;
        }
        let index = self.current_frame_index();
        let frame = &mut self.frames[index];
        frame.table.compiled_protocol = Some(protocol);
        for name in names {
            let cell = frame.table.declare_compiled_cell(name);
            frame.compiled_slots.push(Some(cell));
        }
    }

    pub(crate) fn store_formal_cell(
        &mut self,
        slot: usize,
        value: *mut TclObj,
    ) -> Result<(), VarError> {
        let (frame, cell) = self
            .compiled_cell(slot)
            .ok_or(VarError::NameProtocolUnavailable)?;
        unsafe {
            obj::incr_ref_count(value);
        }
        if let Some(old) = self.frames[frame].table.put_at(cell, Var::Scalar(value)) {
            old.release();
        }
        Ok(())
    }

    pub(crate) fn bind_compiled_slot(&mut self, slot: usize, name: &[u8]) {
        let Some(i) = self.index_of_level(self.active_level) else {
            return;
        };
        let frame = &mut self.frames[i];
        if frame.compiled_slots.get(slot).is_some_and(Option::is_some) {
            return;
        }
        let cell = frame
            .table
            .lookup_slot(name)
            .unwrap_or_else(|| frame.table.declare_compiled_cell(name));
        if frame.compiled_slots.len() <= slot {
            frame.compiled_slots.resize(slot + 1, None);
        }
        frame.compiled_slots[slot] = Some(cell);
    }

    /// The active frame's table cell a generated-code slot is bound to.
    fn compiled_cell(&self, slot: usize) -> Option<(usize, usize)> {
        let i = self.index_of_level(self.active_level)?;
        let cell = (*self.frames[i].compiled_slots.get(slot)?)?;
        Some((i, cell))
    }

    /// Bind this actual compiled cell rather than resolving its display name.
    pub(crate) fn bind_original_compiled_alias(
        &mut self,
        slot: usize,
        link: Link,
        traced: bool,
    ) -> Result<(), NativeCompiledAliasError> {
        let (frame, cell) = self
            .compiled_cell(slot)
            .ok_or(NativeCompiledAliasError::Unavailable)?;
        Self::bind_original_alias_cell(&self.frames[frame].table.cells[cell].contents, link, traced)
    }

    /// The automatic link addresses an admitted compiler ordinal, retaining
    /// the same physical cell and its separate TclOO enumeration origin.
    pub(crate) fn bind_tcloo_compiled_alias(
        &mut self,
        slot: usize,
        link: Link,
    ) -> Result<(), NativeCompiledAliasError> {
        self.bind_original_compiled_alias(slot, link, false)?;
        let (frame, cell) = self
            .compiled_cell(slot)
            .ok_or(NativeCompiledAliasError::Unavailable)?;
        self.frames[frame].table.cells[cell]
            .contents
            .borrow_mut()
            .link_origin = FrameLinkOrigin::TclOoInstanceCompiled;
        Ok(())
    }

    /// Bind the already-selected dynamic local cell without replacing its identity.
    pub(crate) fn bind_original_local_alias(
        &mut self,
        name: &[u8],
        link: Link,
        traced: bool,
    ) -> Result<(), NativeCompiledAliasError> {
        let frame = self
            .index_of_level(self.active_level)
            .ok_or(NativeCompiledAliasError::Unavailable)?;
        let cell = self.frames[frame]
            .table
            .lookup_slot(name)
            .ok_or(NativeCompiledAliasError::Unavailable)?;
        Self::bind_original_alias_cell(&self.frames[frame].table.cells[cell].contents, link, traced)
    }

    fn bind_original_alias_cell(
        contents: &Rc<RefCell<CellContents>>,
        link: Link,
        traced: bool,
    ) -> Result<(), NativeCompiledAliasError> {
        if link.elem.is_none()
            && link
                .native_scalar_entry
                .as_ref()
                .is_some_and(|entry| Rc::ptr_eq(&entry.contents, contents))
        {
            return Err(NativeCompiledAliasError::SelfLink);
        }
        if traced {
            return Err(NativeCompiledAliasError::Traced);
        }
        let mut contents = contents.borrow_mut();
        if matches!(contents.var, Some(Var::Scalar(_) | Var::Array(_))) {
            return Err(NativeCompiledAliasError::Exists);
        }
        if matches!(&contents.var, Some(Var::Link(previous)) if previous == &link) {
            return Ok(());
        }
        contents.undefined_shell = false;
        contents.undefined_root_home = None;
        contents.link_origin = FrameLinkOrigin::Ordinary;
        contents.binding_id.get_or_insert_with(fresh_var_id);
        if let Some(old) = contents.var.replace(Var::Link(link)) {
            old.release();
        }
        Ok(())
    }

    /// Identity of the physical compiled cell before following its alias.
    pub(crate) fn original_compiled_slot_identity(&self, slot: usize) -> Option<VarId> {
        let (frame, cell) = self.compiled_cell(slot)?;
        self.frames[frame].table.cells[cell]
            .contents
            .borrow()
            .binding_id
    }

    /// Tcl-visible name associated with a generated-code slot.
    pub(crate) fn compiled_slot_name(&self, slot: usize) -> Option<&[u8]> {
        let (frame, cell) = self.compiled_cell(slot)?;
        self.frames[frame].table.slot_name(cell)
    }

    /// The cached trace answer for the cell a generated-code slot addresses.
    pub(crate) fn compiled_slot_trace_flag(&self, slot: usize, epoch: u64) -> Option<bool> {
        let (frame, cell) = self.compiled_cell(slot)?;
        self.frames[frame].table.cached_trace_flag(cell, epoch)
    }

    /// Record the trace answer for the cell a generated-code slot addresses.
    pub(crate) fn set_compiled_slot_trace_flag(&self, slot: usize, epoch: u64, traced: bool) {
        if let Some((frame, cell)) = self.compiled_cell(slot) {
            self.frames[frame]
                .table
                .set_cached_trace_flag(cell, epoch, traced);
        }
    }

    /// The variable a generated-code slot addresses — the O(1) read behind
    /// `tcl_codegen_slot_get`. `None` when the slot is unbound or its cell is
    /// currently undefined; a [`Var::Link`] is returned as-is so the caller can
    /// decline the fast path and take the coordinator's link walk.
    pub(crate) fn compiled_slot_var(&self, slot: usize) -> Option<Ref<'_, Var>> {
        let (frame, cell) = self.compiled_cell(slot)?;
        self.frames[frame].table.cell_at(cell)
    }

    /// Defined cells before independently reported TclOO declaration names.
    pub(crate) fn local_names_without_declared_variables(&self) -> Vec<Vec<u8>> {
        self.index_of_level(self.active_level)
            .map(|index| {
                self.frames[index]
                    .table
                    .names()
                    .into_iter()
                    .map(<[u8]>::to_vec)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn local_name_pattern_inputs(
        &self,
        include_links: bool,
    ) -> Result<Vec<(Vec<u8>, tcl_syntax::native_glob::NativeNameGlobPurpose)>, VarError> {
        use tcl_syntax::native_glob::NativeNameGlobPurpose;
        let Some(index) = self.index_of_level(self.active_level) else {
            return Ok(Vec::new());
        };
        let table = &self.frames[index].table;
        let mut inputs = table
            .declared_slots
            .iter()
            .copied()
            .filter(|&slot| table.visible_cell(slot, include_links))
            .map(|slot| {
                (
                    table.cells[slot].name.clone(),
                    NativeNameGlobPurpose::InfoVariablesScan,
                )
            })
            .collect::<Vec<_>>();
        inputs.extend(table.ordered_names().into_iter().filter_map(|name| {
            table
                .lookup_slot(name)
                .filter(|&slot| table.visible_cell(slot, include_links))
                .map(|_| (name.to_vec(), NativeNameGlobPurpose::InfoVariablesSearch))
        }));
        if include_links {
            for name in self.declared_tcloo_variable_names()? {
                if !inputs.iter().any(|(existing, _)| *existing == name) {
                    inputs.push((name, NativeNameGlobPurpose::InfoVariablesScan));
                }
            }
        }
        Ok(inputs)
    }

    /// Actual defined names followed by unused declaring-provider names.
    pub(crate) fn local_names(&self) -> Vec<Vec<u8>> {
        let mut names = self.local_names_without_declared_variables();
        if let Ok(public) = self.declared_tcloo_variable_names() {
            for name in public {
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        names
    }

    /// `info locals` — the active frame's true local variables (no links).
    pub(crate) fn local_names_no_links(&self) -> Vec<Vec<u8>> {
        self.index_of_level(self.active_level)
            .map(|i| {
                self.frames[i]
                    .table
                    .non_link_names()
                    .into_iter()
                    .map(<[u8]>::to_vec)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Direct `const` bindings in the active frame (`info consts`).
    pub(crate) fn const_names(&self) -> Vec<Vec<u8>> {
        self.index_of_level(self.active_level)
            .map(|i| {
                self.frames[i]
                    .table
                    .const_names()
                    .into_iter()
                    .map(<[u8]>::to_vec)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The variable table at `level` (read).
    pub(crate) fn table(&self, level: usize) -> Option<&VarTable> {
        let i = self.index_of_level(level)?;
        self.frames[i]
            .table
            .set_hash_recipe(self.variable_hash_recipe);
        self.frames[i]
            .table
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        Some(&self.frames[i].table)
    }

    /// The variable table at `level` (mutable).
    pub(crate) fn table_mut(&mut self, level: usize) -> Option<&mut VarTable> {
        let i = self.index_of_level(level)?;
        self.frames[i]
            .table
            .set_hash_recipe(self.variable_hash_recipe);
        self.frames[i]
            .table
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        Some(&mut self.frames[i].table)
    }
}

/// Split `a(b)` into (`a`, `Some(b)`); a plain name yields (`name`, `None`).
/// The command layer uses this to route `set a(k)` / `unset a(k)` to the array
/// element ops. `TclObjLookupVarEx`'s rule comes from the shared naming owner,
/// so a zero-length array name (`(x)`) stays an element reference here and in
/// the VM alike.
pub(crate) fn split_array_ref(name: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    match tcl_syntax::naming::split_element_ref_bytes(name) {
        Some((base, elem)) => (base.to_vec(), Some(elem.to_vec())),
        None => (name.to_vec(), None),
    }
}

#[cfg(test)]
mod native_inventory_tests {
    use super::*;
    use tcl_core_types::{NativeHashBytePromotion, NativeHashRecipe, NativeHashWordWidth};
    use tcl_dialect::TclVersion;

    fn bytes(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn array_entry_ledgers_match_all_320_native_table_controls() {
        // Native proof naming.variable-table.original-array-growth-and-reinsertion:
        // docs/design/analysis/name-resolution-proofs/variable-table-original-array-growth-and-reinsertion.md
        // Native proof naming.variable-table.original-opaque-key-extents:
        // docs/design/analysis/name-resolution-proofs/variable-table-original-opaque-key-extents.md
        crate::counters::reset();
        {
            let value = obj::Owned::fresh(obj::new_wide_int_obj(7));
            let fixture = include_str!(
                "../../../rust/tcl-core-types/tests/data/native_variable_tables/array-order.tsv"
            );
            let mut controls = 0;
            for line in fixture.lines().filter(|line| !line.starts_with('#')) {
                let row: Vec<_> = line.split('\t').collect();
                let version = match row[0] {
                    "8.4.20" => TclVersion::V8_4,
                    "8.5.19" => TclVersion::V8_5,
                    "8.6.18" => TclVersion::V8_6,
                    "9.0.4" => TclVersion::V9_0,
                    "9.1.0" => TclVersion::V9_1,
                    _ => panic!("unknown native fixture"),
                };
                let abi =
                    tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(None).unwrap();
                let protocol = tcl_registry::InvocationDialect::for_version(version)
                    .native_variable_table_protocol(abi)
                    .unwrap();
                let mut table = VarTable::default();
                table.set_hash_recipe(Some(protocol.recipe()));
                for index in 0..row[1].parse::<usize>().unwrap() {
                    let original = if row[2] == "1" && index < 3 {
                        [b"k\0x".as_slice(), b"k\xff", b"k\xc0\x80"][index].to_vec()
                    } else {
                        format!("k{index:02}").into_bytes()
                    };
                    let input = protocol
                        .names()
                        .separate_variable_input(b"a", Some(&original));
                    table
                        .store_elem(b"a", input.element().unwrap().selected(), value.as_ptr())
                        .unwrap();
                }
                if row[3] == "1" {
                    table.remove_elem(b"a", b"k03");
                    table.store_elem(b"a", b"k03", value.as_ptr()).unwrap();
                }
                let observed = bytes(row[4]);
                let expected: Vec<_> = observed
                    .split(|&byte| byte == b' ')
                    .map(<[u8]>::to_vec)
                    .collect();
                assert_eq!(table.array_names(b"a").unwrap(), expected, "{line}");
                controls += 1;
            }
            assert_eq!(controls, 320);
        }
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn trace_shells_grow_table_without_becoming_defined_inventory() {
        // Native proof naming.variable-table.undefined-array-shell-growth:
        // docs/design/analysis/name-resolution-proofs/variable-table-undefined-array-shell-growth.md
        crate::counters::reset();
        {
            let value = obj::Owned::fresh(obj::new_wide_int_obj(7));
            let mut table = VarTable::default();
            table.set_hash_recipe(Some(NativeHashRecipe::Tcl {
                promotion: NativeHashBytePromotion::Unsigned,
                width: NativeHashWordWidth::Bits32,
            }));
            for index in 0..3 {
                table
                    .store_elem(b"a", format!("k{index:02}").as_bytes(), value.as_ptr())
                    .unwrap();
            }
            for index in 3..12 {
                table
                    .ensure_element_shell(b"a", format!("k{index:02}").as_bytes())
                    .unwrap();
            }
            assert_eq!(
                table
                    .contents(b"a")
                    .unwrap()
                    .borrow()
                    .member_order
                    .bucket_count(),
                Some(16)
            );
            assert_eq!(
                table.array_names(b"a").unwrap(),
                [b"k00".to_vec(), b"k01".to_vec(), b"k02".to_vec()]
            );
            let id = table.binding_id(b"a");
            table.cleanup_trace_shell(b"a", Some(b"k04"), id);
            assert_eq!(
                table
                    .contents(b"a")
                    .unwrap()
                    .borrow()
                    .member_order
                    .bucket_count(),
                Some(16)
            );
            assert_eq!(
                table
                    .contents(b"a")
                    .unwrap()
                    .borrow()
                    .member_order
                    .physical_keys()
                    .len(),
                11
            );
            table.store_elem(b"a", b"k03", value.as_ptr()).unwrap();
            assert_eq!(
                table.array_names(b"a").unwrap(),
                [
                    b"k00".to_vec(),
                    b"k01".to_vec(),
                    b"k02".to_vec(),
                    b"k03".to_vec()
                ]
            );
        }
        assert_eq!(crate::counters::finalize(), 0);
    }

    #[test]
    fn declared_undefined_namespace_cells_remain_visible_without_becoming_defined() {
        // Native proof: naming.tcloo.explicit-variable-link-counted-target
        // docs/design/analysis/name-resolution-proofs/explicit-variable-link-counted-target.md
        let mut table = VarTable::default();
        let key = b"k\0tail";
        let alias = table.retain_native_scalar_alias(key);
        assert!(table.names().is_empty());
        table.mark_namespace_declared(key);
        drop(alias);
        assert_eq!(table.names(), [key.as_slice()]);
        assert!(table.non_link_names().is_empty());
        assert!(!table.is_set(key));
        assert!(table.load_scalar(key).is_none());
        assert!(table.has_native_namespace_cell(key));
        table.remove(key);
        assert!(table.names().is_empty());
        assert!(!table.is_set(key));
    }

    #[test]
    fn trace_member_identity_selects_original_duplicate_root_allocations() {
        // Rust API coverage: naming.variable.recreated-element-independent-read-trace
        // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
        crate::counters::reset();
        {
            let mut table = VarTable::default();
            let first = table.declare_compiled_cell(b"same");
            let second = table.declare_compiled_cell(b"same");
            let value = obj::Owned::fresh(obj::new_wide_int_obj(1));
            let mut members = Vec::new();
            for slot in [first, second] {
                let receiver = table
                    .capture_receiver_at(slot, Some(b"k".to_vec()), true)
                    .unwrap()
                    .unwrap();
                receiver.store(value.as_ptr()).unwrap();
                let root = receiver.binding_id().unwrap();
                let member = table.native_trace_element_identity(root, b"k").unwrap();
                members.push((root, member));
            }
            assert_ne!(members[0].0, members[1].0);
            assert_ne!(members[0].1, members[1].1);
            for (root, member) in &members {
                assert_eq!(
                    table.native_trace_element_identity(*root, b"k"),
                    Some(*member)
                );
                assert_eq!(table.native_trace_element_identity(*root, b"absent"), None);
            }
            // A selected old member keeps its trace guard while a current
            // same-key replacement has an independent member allocation.
            let receiver = table
                .capture_receiver_at(first, None, true)
                .unwrap()
                .unwrap();
            let detached = receiver.begin_array_destruction().unwrap();
            assert_eq!(detached.trace_member_identity(b"k"), Some(members[0].1));
            receiver.ensure_array().unwrap();
            let replacement = table
                .capture_receiver_at(first, Some(b"k".to_vec()), true)
                .unwrap()
                .unwrap();
            replacement.store(value.as_ptr()).unwrap();
            let (_, replacement_id) = replacement.trace_member().unwrap();
            assert_ne!(replacement_id, members[0].1);
            assert_eq!(
                table.native_trace_element_identity(members[0].0, b"k"),
                Some(replacement_id)
            );
            let old = detached.capture_receiver(b"k".to_vec());
            assert_eq!(old.trace_member(), Some((b"k".to_vec(), members[0].1)));
            assert_eq!(old.read().unwrap(), Some(value.as_ptr()));
            detached.finish_destruction();
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }

    #[test]
    fn c_declared_cells_preserve_duplicates_before_dynamic_hash_inventory() {
        crate::counters::reset();
        {
            let mut frames = FrameStack::new();
            frames.variable_hash_recipe = Some(NativeHashRecipe::Tcl {
                promotion: NativeHashBytePromotion::Unsigned,
                width: NativeHashWordWidth::Bits32,
            });
            frames.push(crate::namespace::GLOBAL);
            frames.install_formal_cells(
                &[b"x".to_vec(), b"x".to_vec()],
                tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(
                    tcl_registry::InvocationDialect::for_version(TclVersion::V9_0)
                        .execution_point()
                        .unwrap(),
                )
                .unwrap(),
            );
            let one = obj::Owned::fresh(obj::new_wide_int_obj(1));
            let two = obj::Owned::fresh(obj::new_wide_int_obj(2));
            frames.store_formal_cell(0, one.as_ptr()).unwrap();
            frames.store_formal_cell(1, two.as_ptr()).unwrap();
            frames
                .table_mut(1)
                .unwrap()
                .store_scalar(b"extra", one.as_ptr())
                .unwrap();
            assert_eq!(
                frames.local_names(),
                [b"x".to_vec(), b"x".to_vec(), b"extra".to_vec()]
            );
            assert_eq!(
                frames.table(1).unwrap().load_scalar(b"x"),
                Some(one.as_ptr())
            );
            assert_eq!(
                frames
                    .table(1)
                    .unwrap()
                    .entry_order
                    .borrow()
                    .physical_keys(),
                [b"extra".as_slice()]
            );
            assert!(
                matches!(frames.compiled_slot_var(1).as_deref(), Some(Var::Scalar(value)) if *value == two.as_ptr())
            );
        }
        assert_eq!(crate::counters::finalize(), 0);
    }
}
