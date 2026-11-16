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

use std::cell::{Cell as PolicyCell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use tcl_runtime_api::{FrameLinkOrigin, VarId};

use crate::namespace::NsId;
use crate::obj::{self, TclObj};

mod jim_alias_publication;
mod jim_link_store;
mod jim_lookup;
pub(crate) use jim_alias_publication::PreparedJimAliasCell;
pub(crate) use jim_link_store::{CapturedJimLinkRoot, JimLinkStoreError};
pub(crate) use jim_lookup::{CapturedJimDictionaryCell, JimVariableRead, WeakJimVariableCell};
mod container;
pub(crate) use container::CapturedContainerModel;
mod destruction;
mod materialisation;
#[cfg(test)]
mod materialisation_tests;
#[cfg(test)]
mod member_materialisation_tests;
mod retired_variable;
mod trace_shell;
pub(crate) use trace_shell::TraceShellState;
#[cfg(test)]
mod stored_link_role_tests;
#[cfg(test)]
mod trace_creation_tests;
pub(crate) use destruction::NativeArraySearchInput;
pub use destruction::{RetainedArrayCell, WeakArraySearchCell};
pub(crate) use materialisation::{
    OriginalVariableLinkObservation, PreparedArrayMember, VariablePublicationObservation,
};
pub(crate) use retired_variable::RetiredVariableParts;
mod native_element_entry;
pub(crate) use native_element_entry::NativeElementAliasEntry;
#[cfg(test)]
pub(crate) use native_element_entry::NativeElementEntryObserver;
mod pending_scalar;
pub use pending_scalar::PendingScalar;
mod rmw;
pub(crate) use rmw::VariableReceiver;
mod native_array_target;
mod native_scalar_alias;
pub(crate) use native_array_target::NativeArrayTargetCell;
mod native_variable_names;
pub(crate) use native_scalar_alias::NativeScalarAliasEntry;
pub(crate) use native_variable_names::{
    OriginalJimVariableInventory, OriginalJimVariableInventoryKey,
};

mod activation_records;
mod call_levels;
mod compiled_slots;
mod table_cells;
mod table_storage;

/// Extracted ownership of an actual array default's two native references.
/// The storage worker publishes the replacement while borrowed, then its caller
/// retires this value after releasing the cell and enclosing owner borrows.
pub(crate) struct RetiredArrayDefault(*mut TclObj);
impl RetiredArrayDefault {
    pub(super) fn capture(original: *mut TclObj) -> Self {
        Self(original)
    }
}
impl Drop for RetiredArrayDefault {
    fn drop(&mut self) {
        // SAFETY: this token carries the old actual table's +2, exactly once.
        unsafe {
            obj::decr_ref_count(self.0);
            obj::decr_ref_count(self.0);
        }
    }
}

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
    /// A selected setter has published this allocation before taking cell +1.
    /// Checked scalar consumers refuse until its exact role ticket is settled.
    PendingScalar(PendingScalar),
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
    original: crate::obj::NativeObjectLifetime,
    stored_name: RefCell<Option<crate::obj::Owned>>,
    pub(crate) frame: tcl_runtime_api::jim_call_frame::JimCallFrameStorageReference,
}
impl OriginalJimLinkTarget {
    pub(crate) fn new(
        name: crate::obj::Owned,
        frame: tcl_runtime_api::jim_call_frame::JimCallFrameStorageReference,
    ) -> Self {
        Self {
            original: crate::obj::NativeObjectLifetime::retain(name.as_ptr()),
            stored_name: RefCell::new(Some(name)),
            frame,
        }
    }
    pub(crate) fn original_name(&self) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        if self.stored_name.borrow().is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "withdrawn original Jim alias role",
            ));
        }
        crate::obj::check_native_liveness(self.original.as_ptr())?;
        Ok(self.original.as_ptr())
    }
    pub(super) fn take_stored_name(
        &self,
    ) -> Result<crate::obj::Owned, tcl_syntax::value::ValueError> {
        self.stored_name.borrow_mut().take().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "withdrawn original Jim alias role",
            ),
        )
    }
    pub(crate) fn withdraw_stored_role(&self) -> RetiredVariableParts {
        let name = self.stored_name.borrow_mut().take();
        let mut retired = RetiredVariableParts::default();
        retired.key(name);
        retired
    }
}
impl std::fmt::Debug for OriginalJimLinkTarget {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("OriginalJimLinkTarget")
            .field("original", &self.original.as_ptr())
            .finish_non_exhaustive()
    }
}
impl PartialEq for OriginalJimLinkTarget {
    fn eq(&self, other: &Self) -> bool {
        self.original.as_ptr() == other.original.as_ptr() && self.frame == other.frame
    }
}
impl Eq for OriginalJimLinkTarget {}
impl Drop for OriginalJimLinkTarget {
    fn drop(&mut self) {
        drop(self.withdraw_stored_role());
    }
}
impl Link {
    pub(crate) fn check_stored_roles(&self) -> Result<(), tcl_syntax::value::ValueError> {
        if let Some(role) = &self.native_scalar_entry {
            role.check_stored_role()?;
        }
        if let Some(role) = &self.native_element_entry {
            role.check_stored_role()?;
        }
        if let Some(role) = &self.original_jim_target {
            role.original_name()?;
        }
        Ok(())
    }
    pub(crate) fn check_selected_target(&self) -> Result<(), tcl_syntax::value::ValueError> {
        if let Some(entry) = &self.native_scalar_entry {
            entry.check_selected_target()?;
        }
        if let Some(entry) = &self.native_element_entry {
            entry.check_selected_target()?;
        }
        if let Some(array) = &self.array_cell {
            if !array.is_live() {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original alias array generation",
                ));
            }
        }
        if let Some(target) = &self.original_jim_target {
            target.original_name()?;
        }
        Ok(())
    }
    pub(crate) fn check_target_identity(&self) -> Result<(), tcl_syntax::value::ValueError> {
        if let Some(entry) = &self.native_scalar_entry {
            entry.check_target_identity()?;
        }
        if let Some(entry) = &self.native_element_entry {
            entry.check_target_identity()?;
        }
        if let Some(array) = &self.array_cell {
            if !array.is_live() {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original alias array generation",
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn acquire_selected_roles(&self) -> Result<(), tcl_syntax::value::ValueError> {
        self.check_selected_target()?;
        if let Some(entry) = &self.native_scalar_entry {
            entry.acquire_selected_role()?;
        }
        if let Some(entry) = &self.native_element_entry {
            entry.acquire_selected_role()?;
        }
        self.check_stored_roles()
    }
    /// An incoming metadata clone does not own the stored role it borrows from
    /// the original local link. A distinct incoming issuer still retires once.
    pub(crate) fn withdraw_unshared_roles(&self, original: Option<&Self>) -> RetiredVariableParts {
        let mut retired = RetiredVariableParts::default();
        if let Some(role) = &self.native_scalar_entry {
            if !original
                .and_then(|old| old.native_scalar_entry.as_ref())
                .is_some_and(|old| Rc::ptr_eq(old, role))
            {
                retired.extend(role.withdraw_stored_role());
            }
        }
        if let Some(role) = &self.native_element_entry {
            if !original
                .and_then(|old| old.native_element_entry.as_ref())
                .is_some_and(|old| Rc::ptr_eq(old, role))
            {
                retired.extend(role.withdraw_stored_role());
            }
        }
        if let Some(role) = &self.original_jim_target {
            if !original
                .and_then(|old| old.original_jim_target.as_ref())
                .is_some_and(|old| Rc::ptr_eq(old, role))
            {
                retired.extend(role.withdraw_stored_role());
            }
        }
        retired
    }
    pub(crate) fn withdraw_stored_roles(&self) -> RetiredVariableParts {
        let mut retired = RetiredVariableParts::default();
        if let Some(role) = &self.native_scalar_entry {
            retired.extend(role.withdraw_stored_role());
        }
        if let Some(role) = &self.native_element_entry {
            retired.extend(role.withdraw_stored_role());
        }
        if let Some(role) = &self.original_jim_target {
            retired.extend(role.withdraw_stored_role());
        }
        retired
    }
}

impl Var {
    /// Release every object reference this var owns. Consumes `self`; call
    /// exactly once when the cell leaves its table.
    fn release(self) {
        match self {
            // SAFETY: a stored cell owns a +1 on each object; releasing balances
            // the retain taken when it was stored.
            Var::Scalar(p) => unsafe { obj::decr_ref_count(p) },
            Var::PendingScalar(pending) => drop(pending),
            Var::Array(map) => {
                for (_, p) in map {
                    unsafe { obj::decr_ref_count(p) }
                }
            }
            Var::Link(link) => drop(link.withdraw_stored_roles()),
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
    /// A store that would land outside the running procedure's own frame while
    /// stores are confined (`can't set "name": stores are confined to the
    /// activation`; `Interp::confine_stores`).
    Confined,
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
    /// Non-owning old scalar pointer during the selected Jim decrement.
    jim_scalar_retirement: Option<Rc<jim_alias_publication::JimScalarRetirement>>,
    var: Option<Var>,
    namespace_declared: bool,
    undefined_shell: bool,
    /// Actual selected home which issued an undefined physical self-link marker.
    undefined_root_home: Option<VarHome>,
    /// The default belongs to this physical array, with two owning references.
    array_default: Option<*mut TclObj>,
    /// Detached array generations whose members retire after the root callback.
    detached_arrays: std::collections::HashMap<(VarId, u64), destruction::DetachedArray>,
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

/// Previous genuine stored ownership extracted before releasing owner borrows.
pub(crate) enum RetiredVariableContents {
    Value(Option<Var>),
    Dictionary(Option<container::RetiredRootContents>),
}
impl Drop for RetiredVariableContents {
    fn drop(&mut self) {
        match self {
            Self::Value(previous) => {
                if let Some(previous) = previous.take() {
                    previous.release();
                }
            }
            Self::Dictionary(previous) => {
                drop(previous.take());
            }
        }
    }
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
        if source.check_owned_value_role().is_err() {
            return false;
        }
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
            Some(Var::PendingScalar(_) | Var::Link(_)) | None => return false,
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
                jim_scalar_retirement: None,
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
    fn check_owned_value_role(&self) -> Result<(), tcl_syntax::value::ValueError> {
        if self.jim_scalar_retirement.is_some() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retiring Jim scalar stored role",
            ));
        }
        if matches!(self.var.as_ref(), Some(Var::PendingScalar(_))) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unsettled scalar native reference role",
            ));
        }
        Ok(())
    }
    fn take_value(&mut self) -> Option<Var> {
        let previous = self.var.take();
        if self
            .jim_scalar_retirement
            .take()
            .is_some_and(|ticket| ticket.displace(&previous))
        {
            return None;
        }
        if let Some(Var::PendingScalar(pending)) = &previous {
            pending.mark_displaced();
        }
        previous
    }
    fn replace_value(&mut self, incoming: Var) -> Option<Var> {
        let previous = self.take_value();
        self.var = Some(incoming);
        previous
    }
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

    fn publish_selected_array(&mut self, root: Option<&[u8]>) -> Result<Option<Var>, VarError> {
        let mut retired = None;
        if self.array_materialisation_needed(root)? {
            retired = self.replace_value(Var::Array(Default::default()));
            self.undefined_shell = false;
            self.undefined_root_home = None;
            self.rmw_shell_entry = None;
        }
        Ok(retired)
    }

    fn insert_member_entry(&mut self, key: &[u8]) {
        if !self.member_order.contains_key(key) {
            self.array_search_epoch = self.array_search_epoch.wrapping_add(1);
            self.array_searches.clear();
        }
        self.adopt_member_entry(key);
    }

    /// Metadata adoption after the caller has retired the actual search chain.
    /// This stage contains no Native header release or String getter.
    fn adopt_member_entry(&mut self, key: &[u8]) {
        self.member_order.insert(key);
        let id = *self
            .element_ids
            .entry(key.to_vec())
            .or_insert_with(fresh_var_id);
        self.native_member_keys.ensure(id, key, None);
    }

    fn retire_rmw_shell(&mut self) -> Option<native_variable_names::NativeHashKey> {
        if self.var.is_some()
            || self.operation_refs != 0
            || self.rmw_refs != 0
            || self.native_alias_refs != 0
            || self.namespace_declared
            || self.undefined_shell
            || self.rmw_retirement.is_some()
        {
            return None;
        }
        if let Some(entry) = self.rmw_shell_entry.take() {
            return entry.retire(self.binding_id);
        }
        None
    }
}

impl Drop for CellContents {
    fn drop(&mut self) {
        if let Some(var) = self.take_value() {
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
    container_transition_pending: Rc<PolicyCell<bool>>,
    container_transition_busy: Rc<PolicyCell<bool>>,
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
    /// Whether `name` is an array variable here (the `set a` array-vs-scalar
    /// diagnostic; `array exists`).
    #[cfg(test)]
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
            Some(Var::Scalar(_) | Var::PendingScalar(_) | Var::Array(_))
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

    #[cfg(test)]
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
        let mut old = RetiredVariableParts::default();
        for cell in &self.cells {
            let mut contents = cell.contents.borrow_mut();
            if contents.rmw_refs != 0
                || contents.native_alias_refs != 0
                || contents.native_member_keys.any_aliases()
            {
                contents.rmw_retirement = Some(VarError::DeletedNamespace);
                old.elements(std::mem::take(&mut contents.native_member_keys));
                old.value(contents.take_value());
            }
        }
        drop(old);
    }
}

// FrameStack — the proc call frames (level 0 is the global context).

/// A selected replacement retains its actual native transport through frame cleanup.
pub(crate) enum PendingTailcall {
    OriginalNamespaceList(crate::obj::Owned),
    Jim(crate::interp::native_jim_tailcall::JimSelectedTailcall),
}

struct Frame {
    jim_link_birth: Rc<()>,
    jim_storage: tcl_runtime_api::jim_call_frame::JimCallFrameStorageSlot,
    c_procedure: Option<Rc<crate::interp::ProcDef>>,
    original_jim_procedure: Option<std::rc::Weak<crate::interp::ProcDef>>,
    original_jim_variadic: Option<OriginalJimVariadic>,
    original_jim_import: Option<Rc<crate::interp::OriginalJimImportTraversal>>,
    original_namespace_info: Option<Rc<crate::interp::OriginalNamespaceInfoFrame>>,
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
    original_error_stack_argv: Option<Vec<obj::NativeObjectLifetime>>,
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
    /// The procedure body this frame activates, whose compiled locals a
    /// script the release compiles in the frame can reach (C's
    /// `localCachePtr`).
    unit: Option<Rc<crate::compiled_locals::ProcUnit>>,
}

pub(crate) struct OriginalJimVariadic {
    original: obj::NativeObjectLifetime,
    backing: crate::list::NativeListBacking,
}

impl OriginalJimVariadic {
    pub(crate) fn capture(
        original: *mut obj::TclObj,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        obj::check_native_liveness(original)?;
        let backing = crate::list::native_list_backing(original).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "actual Jim variadic List constructor",
            ),
        )?;
        Ok(Self {
            original: obj::NativeObjectLifetime::retain(original),
            backing,
        })
    }

    fn original(&self) -> Result<*mut obj::TclObj, tcl_syntax::value::ValueError> {
        let original = self.original.checked_ptr()?;
        self.backing.check_original_header(original)?;
        Ok(original)
    }
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
            original_jim_procedure: None,
            original_jim_variadic: None,
            original_jim_import: None,
            original_namespace_info: None,
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
            unit: None,
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
    pub(crate) fn cached_increment_is_root(&self) -> bool {
        self.active_level == 0
            && self.frames.len() == 1
            && self
                .frames
                .first()
                .is_some_and(|frame| !frame.is_proc && frame.oo_variable_resolver.is_none())
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

    pub(crate) fn original_variable_table_at(&self, level: usize) -> Option<&VarTable> {
        Some(&self.frames[self.index_of_level(level)?].table)
    }

    pub(crate) fn capture_container_model_at(
        &self,
        level: usize,
    ) -> Option<(
        std::rc::Weak<std::cell::RefCell<tcl_core_types::NativeEntryLedger>>,
        Option<container::CapturedContainerModel>,
    )> {
        let i = self.index_of_level(level)?;
        let table = &self.frames[i].table;
        table.set_hash_recipe(self.variable_hash_recipe);
        Some((
            table.original_table_ledger(),
            table.capture_container_model(
                self.variable_container_model,
                self.variable_string_protocol,
            ),
        ))
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
mod native_inventory_tests;
