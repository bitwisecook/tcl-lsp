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

//! Tcl List values own a contiguous array of original object pointers.
//! Retained views pin allocation memory without keeping retired native child
//! caches alive. Mutation copies shared native headers before changing them.
//!
//! Each backing retains the selected native string protocol. Native rendering
//! uses the shared list serialization owner and materializes original members
//! under that protocol. Resident strings remain authoritative. A no-context
//! updater cannot select a protocol for an unselected backing.
//!
//! Pointer arguments must identify live objects owned by the caller or backing.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    rc::Rc,
};
use tcl_syntax::native_string::NativeStringProtocol;

use crate::obj::{self, TclObj, TclObjType};
use crate::parse::{self, ListError};

/// The list backing: the contiguous, growable element array. Each element is a
/// `*mut TclObj` the list owns a `+1` of.
struct TclList {
    elems: NativeListStorage,
    /// Actual native backing flag. Pure List eligibility also admits a missing
    /// resident string; it does not require this flag to be set.
    canonical: Rc<Cell<bool>>,
    string_protocol: Cell<Option<NativeStringProtocol>>,
}

#[path = "list/native_list_storage.rs"]
mod native_list_storage;
pub(crate) use native_list_storage::{
    native_list_command_range, native_list_range, replace_prepared_native_elements,
};

struct NativeListElements(
    Vec<*mut TclObj>,
    Cell<bool>,
    RefCell<Vec<obj::NativeObjectLifetime>>,
);

struct NativeListStorage {
    backing: Rc<NativeListStorageData>,
    header: bool,
    window: Option<std::ops::Range<usize>>,
    span: bool,
    generation: u64,
}
struct NativeListStorageData {
    elements: RefCell<NativeListElements>,
    headers: Cell<usize>,
    capacity: Cell<Option<usize>>,
    first_used: Cell<usize>,
    generation: Cell<u64>,
}
impl NativeListStorage {
    fn new(elements: Vec<*mut TclObj>) -> Self {
        let capacity = elements.len().max(1);
        Self {
            backing: Rc::new(NativeListStorageData {
                elements: RefCell::new(NativeListElements(
                    elements,
                    Cell::new(true),
                    RefCell::new(Vec::new()),
                )),
                headers: Cell::new(1),
                capacity: Cell::new(Some(capacity)),
                first_used: Cell::new(0),
                generation: Cell::new(0),
            }),
            header: true,
            window: None,
            span: false,
            generation: 0,
        }
    }
    fn elements(&self) -> Ref<'_, [*mut TclObj]> {
        Ref::map(self.backing.elements.borrow(), |elements| {
            match &self.window {
                Some(window) => {
                    &elements.0[window.start - self.backing.first_used.get()
                        ..window.end - self.backing.first_used.get()]
                }
                None => elements.0.as_slice(),
            }
        })
    }
    fn len(&self) -> usize {
        self.window.as_ref().map_or_else(
            || self.backing.elements.borrow().0.len(),
            std::ops::Range::len,
        )
    }
    fn get(&self, index: usize) -> Option<*mut TclObj> {
        self.elements().get(index).copied()
    }
    fn lifetime_view(&self) -> Self {
        Self {
            backing: Rc::clone(&self.backing),
            header: false,
            window: self.window.clone(),
            span: self.span,
            generation: self.generation,
        }
    }
    fn copied_header(&self) -> Self {
        let elements = self.elements().to_vec();
        for &value in &elements {
            unsafe { obj::incr_ref_count(value) };
        }
        Self::new(elements)
    }
    fn set_capacity(&self, capacity: usize) {
        assert!(capacity >= self.backing.elements.borrow().0.len());
        let mut members = self.backing.elements.borrow_mut();
        let length = members.0.len();
        members.0.reserve_exact(capacity - length);
        self.backing.capacity.set(Some(capacity));
    }
    fn checked_generation(&self) -> Result<(), tcl_syntax::value::ValueError> {
        if self.generation != self.backing.generation.get() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "changed native List member vector",
            ));
        }
        Ok(())
    }
    fn range_header(&self, range: std::ops::Range<usize>) -> Self {
        let mut header = self.clone();
        let start = self
            .window
            .as_ref()
            .map_or(self.backing.first_used.get(), |window| window.start);
        header.window = Some(start + range.start..start + range.end);
        header.span = self.span || range.start != 0 || range.end != self.len();
        header
    }
    fn select_window(&mut self, range: std::ops::Range<usize>, span: bool) {
        assert!(self.header && range.start <= range.end && range.end <= self.len());
        let offset = self
            .window
            .as_ref()
            .map_or(0, |window| window.start - self.backing.first_used.get());
        let start = offset + range.start;
        let end = offset + range.end;
        if self.native_is_shared() {
            self.window =
                Some(self.backing.first_used.get() + start..self.backing.first_used.get() + end);
        } else {
            let retired = {
                let mut elements = self.backing.elements.borrow_mut();
                let mut retired = elements.0.drain(end..).collect::<Vec<_>>();
                retired.extend(elements.0.drain(..start));
                retired
            };
            for value in retired {
                unsafe { obj::decr_ref_count(value) };
            }
            self.backing.first_used.set(if span {
                self.backing.first_used.get() + start
            } else {
                0
            });
            self.window = None;
            self.note_mutation();
        }
        self.span = span;
    }
    fn collect_unreferenced(&mut self) {
        if !self.native_is_shared() && self.window.is_some() {
            self.select_window(0..self.len(), self.span);
        }
    }
    fn note_mutation(&mut self) {
        let generation = self
            .backing
            .generation
            .get()
            .checked_add(1)
            .expect("native List generation");
        self.backing.generation.set(generation);
        self.generation = generation;
    }
    fn native_is_shared(&self) -> bool {
        self.backing.headers.get() > 1
    }
    fn make_mut(&mut self) -> RefMut<'_, Vec<*mut TclObj>> {
        assert!(self.header, "a lifetime view cannot mutate a native List");
        if self.native_is_shared() {
            *self = self.copied_header();
        }
        self.collect_unreferenced();
        self.note_mutation();
        self.backing.capacity.set(None);
        RefMut::map(self.backing.elements.borrow_mut(), |elements| {
            &mut elements.0
        })
    }
}
impl Clone for NativeListStorage {
    fn clone(&self) -> Self {
        if self.header {
            self.backing.headers.set(
                self.backing
                    .headers
                    .get()
                    .checked_add(1)
                    .expect("native List header count"),
            );
        }
        Self {
            backing: Rc::clone(&self.backing),
            header: self.header,
            window: self.window.clone(),
            span: self.span,
            generation: self.generation,
        }
    }
}
impl Drop for NativeListStorage {
    fn drop(&mut self) {
        if self.header {
            self.backing.headers.set(
                self.backing
                    .headers
                    .get()
                    .checked_sub(1)
                    .expect("native List header retirement"),
            );
            if self.backing.headers.get() == 0 {
                let values = {
                    let elements = self.backing.elements.borrow();
                    if !elements.1.get() {
                        return;
                    }
                    // Pin memory before releasing real List member references.
                    // These pins grant no permission to read a retired cache.
                    elements.2.replace(
                        elements
                            .0
                            .iter()
                            .map(|&value| obj::NativeObjectLifetime::retain(value))
                            .collect(),
                    );
                    elements.1.set(false);
                    elements.0.clone()
                };
                for value in values {
                    // SAFETY: the last genuine header releases its actual
                    // member owner now, outside the backing RefCell borrow.
                    unsafe { obj::decr_ref_count(value) };
                }
            }
        }
    }
}

impl Clone for NativeListElements {
    fn clone(&self) -> Self {
        assert!(
            self.1.get(),
            "cannot duplicate a retired List member inventory"
        );
        for &value in &self.0 {
            // SAFETY: the existing backing owns each live original object.
            unsafe { obj::incr_ref_count(value) };
        }
        Self(self.0.clone(), Cell::new(true), RefCell::new(Vec::new()))
    }
}

impl Drop for NativeListElements {
    fn drop(&mut self) {
        for &value in self.0.iter().filter(|_| self.1.get()) {
            // SAFETY: release the original backing's one owning reference.
            unsafe { obj::decr_ref_count(value) };
        }
    }
}

impl std::ops::Deref for NativeListElements {
    type Target = Vec<*mut TclObj>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Stable original List backing; retaining it adds no element references.
#[derive(Clone)]
pub(crate) struct NativeListBacking {
    elements: NativeListStorage,
    canonical: Rc<Cell<bool>>,
}

impl NativeListBacking {
    pub(crate) fn len(&self) -> usize {
        self.elements.len()
    }
    /// Return live original members. A retired sole child remains allocated for
    /// pointer safety but its native cache cannot be read through this view.
    pub(crate) fn elements(&self) -> Result<Ref<'_, [*mut TclObj]>, tcl_syntax::value::ValueError> {
        self.elements.checked_generation()?;
        let elements = self.elements.elements();
        if elements
            .iter()
            .any(|&value| !obj::allocation_is_live(value))
        {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired native List member",
            ));
        }
        Ok(elements)
    }

    /// Original List canonical flag, independent of resident string contents.
    pub(crate) fn canonical(&self) -> bool {
        self.canonical.get()
    }
}

/// Retain an existing native List backing without conversion or child clones.
pub(crate) fn native_list_backing(value: *mut TclObj) -> Option<NativeListBacking> {
    if obj::obj_type_ptr(value) != &TCL_LIST_TYPE {
        return None;
    }
    // SAFETY: the exact descriptor owns this live backing.
    let list = unsafe { list_ref(value) };
    Some(NativeListBacking {
        elements: list.elems.lifetime_view(),
        canonical: Rc::clone(&list.canonical),
    })
}

/// Actual retained native headers, excluding cursor/inspection lifetime views.
#[cfg(test)]
pub(crate) fn native_header_reference_count(
    value: *mut TclObj,
) -> Result<Option<usize>, tcl_syntax::value::ValueError> {
    if obj::obj_type_ptr(value) != &TCL_LIST_TYPE {
        return Ok(None);
    }
    // SAFETY: exact descriptor owns this header and its actual shared backing.
    Ok(Some(unsafe { list_ref(value) }.elems.backing.headers.get()))
}

/// Apply native C shared List backing or Jim's copied member backing to an
/// already duplicated object. This requires independently selected protocol.
pub(crate) fn duplicate_native_backing(
    original: *mut TclObj,
    duplicate: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) {
    if obj::obj_type_ptr(original) != &TCL_LIST_TYPE || protocol.is_jim084() {
        return;
    }
    // SAFETY: both objects are live and the original exact List owns its backing.
    let original = unsafe { list_ref(original) };
    let backing = Box::new(TclList {
        elems: original.elems.clone(),
        canonical: Rc::clone(&original.canonical),
        string_protocol: original.string_protocol.clone(),
    });
    obj::change_type(
        duplicate,
        &TCL_LIST_TYPE,
        Box::into_raw(backing) as usize as u64,
    );
}

/// The `list` type descriptor — free/dup/update-string procs wired to the list
/// backing (`update_string` regenerates the canonical list string form).
pub static TCL_LIST_TYPE: TclObjType = TclObjType {
    name: c"list".as_ptr(),
    free_int_rep_proc: Some(list_free),
    dup_int_rep_proc: Some(list_dup),
    update_string_proc: Some(list_update_string),
    set_from_any_proc: None,
};

// internalRep accessors

unsafe fn list_ref<'a>(obj: *mut TclObj) -> &'a TclList {
    // SAFETY: `obj` has the list type, so its internalRep is a live `TclList *`.
    unsafe { &*(obj::internal_rep(obj) as usize as *const TclList) }
}

unsafe fn list_mut<'a>(obj: *mut TclObj) -> &'a mut TclList {
    // SAFETY: as `list_ref`, and the caller holds the only reference while mutating.
    unsafe { &mut *(obj::internal_rep(obj) as usize as *mut TclList) }
}

// type procs

extern "C" fn list_free(obj: *mut TclObj) {
    // SAFETY: `obj` is a live list obj being freed; reclaim the backing box and
    // release the +1 the list held on each element.
    unsafe {
        let p = obj::internal_rep(obj) as usize as *mut TclList;
        if p.is_null() {
            return;
        }
        drop(Box::from_raw(p));
    }
}

extern "C" fn list_dup(src: *mut TclObj, dup: *mut TclObj) {
    // SAFETY: deep-copy the element vector and retain each element for the copy.
    unsafe {
        let src_ref = list_ref(src);
        let elems = if src_ref
            .string_protocol
            .get()
            .is_some_and(|protocol| protocol.tcl_version().is_some())
        {
            src_ref.elems.clone()
        } else {
            src_ref.elems.copied_header()
        };
        let canonical = if src_ref
            .string_protocol
            .get()
            .is_some_and(|protocol| protocol.tcl_version().is_some())
        {
            Rc::clone(&src_ref.canonical)
        } else {
            Rc::new(Cell::new(false))
        };
        let boxed = Box::new(TclList {
            elems,
            canonical,
            string_protocol: src_ref.string_protocol.clone(),
        });
        obj::change_type(dup, &TCL_LIST_TYPE, Box::into_raw(boxed) as usize as u64);
    }
}

extern "C" fn list_update_string(value: *mut TclObj) {
    if let Some(protocol) = native_string_protocol(value) {
        let _ = crate::dict::native_object_bytes(value, protocol);
    }
}

/// Selected recipe retained by this actual List backing, without materialization.
pub(crate) fn native_string_protocol(value: *mut TclObj) -> Option<NativeStringProtocol> {
    (obj::obj_type_ptr(value) == &TCL_LIST_TYPE)
        .then(|| unsafe { list_ref(value) }.string_protocol.get())
        .flatten()
}

/// Seal a recipe at an independently selected native boundary.
pub(crate) fn seal_string_protocol(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<(), tcl_syntax::value::ValueError> {
    if obj::obj_type_ptr(value) != &TCL_LIST_TYPE {
        return Ok(());
    }
    let retained = &unsafe { list_ref(value) }.string_protocol;
    if retained.get().is_some_and(|existing| existing != protocol) {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native List string recipe origin",
        ));
    }
    retained.set(Some(protocol));
    Ok(())
}

/// Install a reached selected updater's string and actual canonical flag.
pub(crate) fn install_native_string(
    value: *mut TclObj,
    bytes: &[u8],
    protocol: NativeStringProtocol,
) {
    // SAFETY: the selected original List backing owns this live object.
    unsafe {
        obj::set_native_updater_string_rep(
            value,
            bytes,
            protocol.compound_updater_storage()
                == tcl_syntax::native_string::NativeStringStorageIdentity::CanonicalEmpty,
        )
    };
    let backing = unsafe { list_ref(value) };
    backing.canonical.set(
        protocol.updated_list_canonical(backing.canonical.get(), backing.elems.native_is_shared()),
    );
}

// shimmer

/// Ensure `obj` carries the list internal rep, parsing its string rep into
/// elements if it does not (string → list shimmer). The string rep is kept.
fn ensure_list(obj: *mut TclObj) -> Result<(), ListError> {
    if obj::obj_type_ptr(obj) == &TCL_LIST_TYPE {
        return Ok(());
    }
    let bytes = obj::bytes_of(obj);
    let elem_bytes = parse::split_list(&bytes)?;
    let mut elems = Vec::with_capacity(elem_bytes.len());
    for eb in &elem_bytes {
        let eo = obj::new_string_bytes(eb);
        // SAFETY: `eo` is fresh; the list takes the owning +1.
        unsafe { obj::incr_ref_count(eo) };
        elems.push(eo);
    }
    // Shimmered from a string: the kept string rep is the source of truth, so
    // the value is *not* canonical (its string may not match the list form).
    let boxed = Box::new(TclList {
        elems: NativeListStorage::new(elems),
        canonical: Rc::new(Cell::new(false)),
        string_protocol: Cell::new(None),
    });
    obj::change_type(obj, &TCL_LIST_TYPE, Box::into_raw(boxed) as usize as u64);
    Ok(())
}

// public ops

/// `Tcl_NewListObj` — a fresh (`rc 0`) list of the given elements (each retained).
pub fn new_list_obj(elems: &[*mut TclObj]) -> *mut TclObj {
    let v: Vec<*mut TclObj> = elems.to_vec();
    for &e in &v {
        // SAFETY: each element is live; the list takes a +1.
        unsafe { obj::incr_ref_count(e) };
    }
    let boxed = Box::new(TclList {
        elems: NativeListStorage::new(v),
        canonical: Rc::new(Cell::new(false)),
        string_protocol: Cell::new(None),
    });
    obj::alloc_typed(&TCL_LIST_TYPE, Box::into_raw(boxed) as usize as u64)
}

/// Create a List with the selected physical updater recipe.
pub(crate) fn new_list_obj_native(
    elems: &[*mut TclObj],
    protocol: NativeStringProtocol,
) -> *mut TclObj {
    if elems.is_empty() && protocol.tcl_version().is_some() {
        return obj::new_obj();
    }
    let value = new_list_obj(elems);
    seal_string_protocol(value, protocol).expect("fresh native List recipe");
    value
}

/// Read an authenticated ordinary list backing without converting storage.
pub(crate) fn cached_length(value: *mut TclObj) -> Option<usize> {
    (obj::obj_type_ptr(value) == &TCL_LIST_TYPE).then(|| unsafe { list_ref(value) }.elems.len())
}

/// Inspect the ordinary list's primary length and canonical flag.
pub(crate) fn native_cache_snapshot(value: *mut TclObj) -> Option<(usize, bool)> {
    if obj::obj_type_ptr(value) != &TCL_LIST_TYPE {
        return None;
    }
    // SAFETY: the exact descriptor owns this live backing.
    let backing = unsafe { list_ref(value) };
    Some((backing.elems.len(), backing.canonical.get()))
}

/// `Tcl_ListObjLength`. Shimmers a string to a list if needed.
pub fn list_length(obj: *mut TclObj) -> Result<usize, ListError> {
    if let Some(length) = crate::native_arithseries::length(obj) {
        return Ok(length);
    }
    ensure_list(obj)?;
    // SAFETY: `ensure_list` guarantees the list rep.
    Ok(unsafe { list_ref(obj) }.elems.len())
}

/// `Tcl_ListObjIndex` — the element at `i` (borrowed), or `None` if out of range.
pub fn list_index(obj: *mut TclObj, i: usize) -> Result<Option<*mut TclObj>, ListError> {
    if crate::native_arithseries::is_series(obj) {
        return Ok(crate::native_arithseries::index(obj, i));
    }
    ensure_list(obj)?;
    // SAFETY: list rep guaranteed.
    Ok(unsafe { list_ref(obj) }.elems.get(i))
}

/// Every element (borrowed pointers), e.g. for `Tcl_ListObjGetElements` /
/// `foreach`. The returned objects are owned by the list.
/// Whether `obj` is a **pure** list — it has the list internal rep and no
/// string rep, so its canonical form is its elements (C's
/// `TclListObjIsCanonical`). Such a value can be evaluated as a single command
/// by element *identity* (preserving each element obj's TIP 280 source
/// location), instead of being stringified and re-parsed.
#[must_use]
pub fn is_pure_list(obj: *mut TclObj) -> bool {
    obj::obj_type_ptr(obj) == &TCL_LIST_TYPE
        && (!obj::has_string_rep(obj) || unsafe { list_ref(obj) }.canonical.get())
}

pub fn list_elements(obj: *mut TclObj) -> Result<Vec<*mut TclObj>, tcl_syntax::value::ValueError> {
    if crate::native_arithseries::is_series(obj) {
        return crate::native_arithseries::elements_retained(obj);
    }
    ensure_list(obj).map_err(|error| tcl_syntax::value::ValueError::ListParse {
        error: error.shared(),
        source: obj::bytes_of(obj),
    })?;
    // SAFETY: list rep guaranteed.
    Ok(unsafe { list_ref(obj) }.elems.elements().to_vec())
}

/// Selected native byte-list conversion, retaining current element objects
/// when a list representation already exists. Conversion withdraws any prior
/// cached Jim string count through the central object type-change owner.
pub(crate) fn list_elements_in(
    value: *mut TclObj,
    syntax: tcl_dialect::ListParse,
    escapes: tcl_dialect::EscapeSyntax,
) -> Result<Vec<*mut TclObj>, ListError> {
    list_elements_using(value, None, syntax, escapes, None)
}

/// Convert original native object-list storage while preserving host refusals.
pub(crate) fn list_elements_native_checked(
    value: *mut TclObj,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<Vec<*mut TclObj>, tcl_syntax::value::ValueError> {
    if crate::native_arithseries::is_series(value) {
        return crate::native_arithseries::elements(value, protocol);
    }
    seal_string_protocol(value, protocol)?;
    let direct = obj::obj_type_ptr(value) == &TCL_LIST_TYPE
        || (obj::obj_type_ptr(value) == &crate::dict::TCL_DICT_TYPE && !obj::has_string_rep(value));
    let source = if protocol.is_jim084() && !direct {
        let context = crate::native_source::context(value)?;
        let info = crate::native_source::pin_source_info(value, &context)?;
        Some((context, info))
    } else {
        None
    };
    if !direct {
        drop(crate::dict::native_object_bytes(value, protocol)?);
    }
    list_elements_using(
        value,
        Some(protocol),
        tcl_dialect::ListParse::Strict,
        protocol.escape_syntax(),
        source,
    )
    .map_err(|error| tcl_syntax::value::ValueError::NativeListParse {
        error: error.shared(),
        source: obj::bytes_of(value),
        protocol,
    })
}

/// Enter ordinary `SetListFromAny`; abstract Index hooks create fresh members
/// before the original primary is retired. Resident string storage is retained.
pub(crate) fn ordinary_list_elements_native_checked(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<Vec<*mut TclObj>, tcl_syntax::value::ValueError> {
    if !crate::native_arithseries::is_series(value) {
        return list_elements_native_checked(value, protocol);
    }
    let members = crate::native_arithseries::ordinary_members(value, protocol)?;
    let elements: Vec<_> = members.iter().map(obj::Owned::as_ptr).collect();
    for &element in &elements {
        // SAFETY: each fresh member remains owned until the List takes its ref.
        unsafe { obj::incr_ref_count(element) };
    }
    let backing = Box::new(TclList {
        elems: NativeListStorage::new(elements),
        canonical: Rc::new(Cell::new(false)),
        string_protocol: Cell::new(Some(protocol)),
    });
    obj::change_type(
        value,
        &TCL_LIST_TYPE,
        Box::into_raw(backing) as usize as u64,
    );
    drop(members);
    list_elements_native_checked(value, protocol)
}

/// Reach C8.5+ TclListObjCopy: ordinary Lists retain their member backing in a
/// fresh absent-string header; abstract length-hook objects duplicate directly.
pub(crate) fn native_list_copy(
    original: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
    if !protocol
        .tcl_version()
        .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_5)
    {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "TclListObjCopy",
        ));
    }
    if crate::native_arithseries::is_series(original) {
        crate::native_arithseries::length_in(original, protocol)?;
        // Native lengthProc objects duplicate their genuine abstract header
        // before any GetElements call, retaining the original backing/cache.
        return Ok(obj::Owned::fresh(obj::duplicate(original)));
    }
    let _ = list_elements_native_checked(original, protocol)?;
    // SAFETY: selected conversion installed the actual ordinary List primary.
    // Rc backing ownership creates a native header without retaining children.
    let list = unsafe { list_ref(original) };
    let backing = Box::new(TclList {
        elems: list.elems.clone(),
        canonical: Rc::clone(&list.canonical),
        string_protocol: Cell::new(Some(protocol)),
    });
    Ok(obj::Owned::fresh(obj::alloc_typed(
        &TCL_LIST_TYPE,
        Box::into_raw(backing) as usize as u64,
    )))
}

/// Reach a concrete mutable List only when a mutator requires it. Abstract
/// GetElements itself preserves the arithmetic primary and its element cache.
fn prepare_native_list_mutation(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<(), tcl_syntax::value::ValueError> {
    let elements = list_elements_native_checked(value, protocol)?;
    if crate::native_arithseries::is_series(value) {
        for &element in &elements {
            // SAFETY: actual new List backing owns each original cached member.
            unsafe { obj::incr_ref_count(element) };
        }
        let backing = Box::new(TclList {
            elems: NativeListStorage::new(elements),
            canonical: Rc::new(Cell::new(false)),
            string_protocol: Cell::new(Some(protocol)),
        });
        obj::change_type(
            value,
            &TCL_LIST_TYPE,
            Box::into_raw(backing) as usize as u64,
        );
    }
    Ok(())
}

/// Native dictionary List append observes member sharing before a working hold.
/// Zero operands avoid conversion while retaining native member-copy semantics.
pub(crate) fn append_native_elements(
    original: Option<*mut TclObj>,
    elements: &[*mut TclObj],
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Result<crate::obj::Owned, tcl_syntax::value::ValueError> {
    let value = match original {
        None => {
            return Ok(crate::obj::Owned::fresh(
                if elements.is_empty() && protocol.tcl_version().is_some() {
                    obj::new_string_bytes(b"")
                } else {
                    new_list_obj_native(elements, protocol)
                },
            ));
        }
        Some(value) if obj::is_shared(value) => {
            let duplicate = obj::duplicate(value);
            duplicate_native_backing(value, duplicate, protocol);
            crate::obj::Owned::fresh(duplicate)
        }
        Some(value) => {
            if !elements.is_empty() {
                prepare_native_list_mutation(value, protocol)?;
            }
            crate::obj::Owned::retain(value)
        }
    };
    if elements.is_empty() {
        return Ok(value);
    }
    append_prepared_native_elements(value.as_ptr(), elements, protocol)?;
    Ok(value)
}

/// Mutate the already selected unshared native header. The caller has observed
/// sharing and, where required, installed the duplicate in its actual variable.
pub(crate) fn append_prepared_native_elements(
    value: *mut TclObj,
    elements: &[*mut TclObj],
    protocol: NativeStringProtocol,
) -> Result<(), tcl_syntax::value::ValueError> {
    prepare_native_list_mutation(value, protocol)?;
    if elements.is_empty() {
        let preserve = protocol.tcl_version().is_some_and(|version| {
            version >= tcl_dialect::TclVersion::V9_0
                && native_list_backing(value).is_some_and(|backing| backing.canonical())
        });
        if !preserve {
            obj::invalidate_string(value);
        }
        return Ok(());
    }
    // SAFETY: preparation selected the actual unshared receiver before its
    // working owner; each appended original object becomes a list member.
    unsafe {
        let list = list_mut(value);
        if list.elems.native_is_shared() {
            list.canonical = Rc::new(Cell::new(
                protocol.copied_list_canonical(list.canonical.get()),
            ));
        }
        if protocol.tcl_version().is_some() {
            let length = list.elems.len();
            replace_prepared_native_elements(value, length, 0, elements, protocol, false)?;
        } else {
            for &element in elements {
                obj::incr_ref_count(element);
                list.elems.make_mut().push(element);
            }
        }
    }
    obj::invalidate_string(value);
    Ok(())
}

/// LAPPEND_LIST reaches original receiver conversion before its header COW.
pub(crate) fn append_native_list_elements(
    original: *mut TclObj,
    elements: &[*mut TclObj],
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
    prepare_native_list_mutation(original, protocol)?;
    let value = if obj::is_shared(original) {
        let value = obj::duplicate(original);
        duplicate_native_backing(original, value, protocol);
        obj::Owned::fresh(value)
    } else {
        obj::Owned::retain(original)
    };
    append_prepared_native_elements(value.as_ptr(), elements, protocol)?;
    Ok(value)
}

/// LIST_CONCAT duplicates its target before reaching source GetElements.
pub(crate) fn concatenate_native_lists(
    original: *mut TclObj,
    source: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
    let value = if obj::is_shared(original) {
        let value = obj::duplicate(original);
        duplicate_native_backing(original, value, protocol);
        obj::Owned::fresh(value)
    } else {
        obj::Owned::retain(original)
    };
    let elements = list_elements_native_checked(source, protocol)?;
    append_prepared_native_elements(value.as_ptr(), &elements, protocol)?;
    Ok(value)
}

/// The C9 full-range opcode used by a single expanded lappend operand.
pub(crate) fn native_full_list_range(
    original: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
    if !protocol
        .tcl_version()
        .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
    {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "C9 full List range",
        ));
    }
    let elements = list_elements_native_checked(original, protocol)?;
    if elements.is_empty() {
        return Ok(
            if !obj::has_string_rep(original) || obj::bytes_of(original).is_empty() {
                obj::Owned::retain(original)
            } else {
                obj::Owned::fresh(obj::new_string_bytes(b""))
            },
        );
    }
    let value = if obj::is_shared(original) {
        native_list_copy(original, protocol)?
    } else {
        obj::Owned::retain(original)
    };
    obj::invalidate_string(value.as_ptr());
    Ok(value)
}

fn list_elements_using(
    value: *mut TclObj,
    native: Option<tcl_syntax::native_string::NativeStringProtocol>,
    syntax: tcl_dialect::ListParse,
    escapes: tcl_dialect::EscapeSyntax,
    source: Option<(
        Rc<crate::native_source::NativeJimObjectContext>,
        crate::native_source::NativeJimSourceInfo,
    )>,
) -> Result<Vec<*mut TclObj>, ListError> {
    if obj::obj_type_ptr(value) == &crate::dict::TCL_DICT_TYPE && !obj::has_string_rep(value) {
        // Native SetListFromAny reads dictionary members directly. Retain the
        // original member objects before releasing the dictionary backing;
        // materializing its string would change purity and operand identity.
        let pairs = crate::dict::dict_pairs(value).expect("exact dictionary backing");
        let elems: Vec<_> = pairs
            .into_iter()
            .flat_map(|(key, value)| [key, value])
            .collect();
        for &element in &elems {
            unsafe { obj::incr_ref_count(element) };
        }
        let backing = Box::new(TclList {
            elems: NativeListStorage::new(elems),
            canonical: Rc::new(Cell::new(false)),
            string_protocol: Cell::new(native),
        });
        obj::change_type(
            value,
            &TCL_LIST_TYPE,
            Box::into_raw(backing) as usize as u64,
        );
    }
    if obj::obj_type_ptr(value) != &TCL_LIST_TYPE {
        let bytes = obj::bytes_of(value);
        let parsed: Vec<_> = match native {
            Some(protocol) => tcl_syntax::list::split_native_list_elements(&bytes, protocol)?
                .into_iter()
                .map(|element| (element.value, element.line_delta))
                .collect(),
            None => tcl_syntax::list::split_list_bytes_in(&bytes, syntax, escapes)?
                .into_iter()
                .map(|value| (value, 0))
                .collect(),
        };
        if source.is_some() {
            let backing = Box::new(TclList {
                elems: NativeListStorage::new(Vec::new()),
                canonical: Rc::new(Cell::new(false)),
                string_protocol: Cell::new(native),
            });
            obj::change_type(
                value,
                &TCL_LIST_TYPE,
                Box::into_raw(backing) as usize as u64,
            );
        }
        let elems = parsed
            .iter()
            .map(|(value, line_delta)| {
                let member = obj::new_string_bytes(value);
                if let Some((context, info)) = &source {
                    crate::native_source::install_source(
                        member,
                        crate::native_source::NativeJimSourceInfo {
                            filename: info.filename.clone(),
                            line: info.line.wrapping_add_unsigned(*line_delta),
                        },
                        context,
                    )
                    .expect("fresh native List member admits its original Source");
                }
                // SAFETY: the new List backing owns this fresh member.
                unsafe { obj::incr_ref_count(member) };
                member
            })
            .collect();
        let storage = NativeListStorage::new(elems);
        if native.is_some_and(|protocol| {
            protocol
                .tcl_version()
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
        }) {
            storage.set_capacity(tcl_syntax::list::max_list_length_bytes(&bytes).max(1));
        }
        let backing = Box::new(TclList {
            elems: storage,
            canonical: Rc::new(Cell::new(false)),
            string_protocol: Cell::new(native),
        });
        obj::change_type(
            value,
            &TCL_LIST_TYPE,
            Box::into_raw(backing) as usize as u64,
        );
    }
    // SAFETY: successful selected conversion or the prior live rep owns them.
    Ok(unsafe { list_ref(value) }.elems.elements().to_vec())
}

/// Replace a private List header's complete contents, duplicating only a shared header.
pub(crate) fn replace_elements_native(
    original: *mut TclObj,
    elements: &[*mut TclObj],
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
    let value = if obj::is_shared(original) {
        obj::Owned::fresh(obj::duplicate(original))
    } else {
        obj::Owned::retain(original)
    };
    prepare_native_list_mutation(value.as_ptr(), protocol)?;
    // SAFETY: the checked receiver owns a List header. Retain all replacement
    // members before releasing the old backing, including overlapping inputs.
    unsafe {
        let list = list_mut(value.as_ptr());
        if protocol.tcl_version().is_some() {
            let length = list.elems.len();
            replace_prepared_native_elements(value.as_ptr(), 0, length, elements, protocol, false)?;
        } else {
            for &element in elements {
                obj::incr_ref_count(element);
            }
            list.elems = NativeListStorage::new(elements.to_vec());
            list.canonical = Rc::new(Cell::new(false));
        }
    }
    obj::invalidate_string(value.as_ptr());
    Ok(value)
}

/// `Tcl_ListObjGetElements`'s view: the list's own element array and its
/// length, after shimmering a string to a list. The array belongs to the list
/// and is good until the list changes or is freed.
pub(crate) fn elements_raw(obj: *mut TclObj) -> Result<(*mut *mut TclObj, usize), ListError> {
    ensure_list(obj)?;
    // SAFETY: list rep guaranteed; the pointer stays the list's own.
    let list = unsafe { list_mut(obj) };
    Ok((list.elems.as_mut_ptr(), list.elems.len()))
}

/// `Tcl_ListObjAppendElement` — append `elem` (retained) in place and invalidate
/// the string rep.
///
/// In-place mutation is correct only when `obj` is **unshared** (`refCount <= 1`)
/// — exactly Tcl's contract for this call. The `lappend` command is
/// responsible for copy-on-write when the value is shared.
pub fn list_append(obj: *mut TclObj, elem: *mut TclObj) -> Result<(), ListError> {
    ensure_list(obj)?;
    // SAFETY: list rep guaranteed; the list takes a +1 on `elem`.
    unsafe {
        obj::incr_ref_count(elem);
        let list = list_mut(obj);
        if list.elems.native_is_shared() {
            if let Some(protocol) = list.string_protocol.get() {
                list.canonical = Rc::new(Cell::new(
                    protocol.copied_list_canonical(list.canonical.get()),
                ));
            }
        }
        list.elems.make_mut().push(elem);
    }
    obj::invalidate_string(obj); // regenerate the string form on next read
    Ok(())
}

// list-element string quoting

/// Append `elem` to `buf` in canonical Tcl list-element form — a thin binding
/// of the shared `tcl_syntax::list` codec's **byte** entry point
/// (`TclScanElement` / `TclConvertElement`, tclUtil.c:1056 / :1422).
///
/// `quote_hash` (the first element of a list) forces a leading `#` to be quoted
/// so the rendered list cannot be misread as starting a comment
/// (`TCL_DONT_QUOTE_HASH` inverted). Shared with `dict` (key/value quoting).
///
/// A separate runtime port of the same four `CONVERT_*` modes would risk
/// exactly this kind of drift: disjoint parity tables with no drift gate
/// between them, and no guarantee that a flag setting on the trailing-`\`
/// and `\<newline>` arms stays aligned with C. One implementation, one
/// parity table.
pub(crate) fn append_list_element(buf: &mut Vec<u8>, elem: &[u8], quote_hash: bool) {
    tcl_syntax::list::append_list_element(buf, elem, quote_hash);
}

#[cfg(test)]
mod tests {
    #[test]
    fn selected_double_updaters_match_all_native_nonfinite_payloads() {
        let fixtures = [
            include_str!("../../../rust/tcl-syntax/testdata/native_nonfinite_double/8.4.20.tsv"),
            include_str!("../../../rust/tcl-syntax/testdata/native_nonfinite_double/8.5.19.tsv"),
            include_str!("../../../rust/tcl-syntax/testdata/native_nonfinite_double/8.6.18.tsv"),
            include_str!("../../../rust/tcl-syntax/testdata/native_nonfinite_double/9.0.4.tsv"),
            include_str!("../../../rust/tcl-syntax/testdata/native_nonfinite_double/9.1.0.tsv"),
            include_str!("../../../rust/tcl-syntax/testdata/native_nonfinite_double/jim.tsv"),
        ];
        let protocols = [
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ];
        let bits = [
            0x7ff8_0000_0000_0000,
            0xfff8_0000_0000_0000,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0,
            0x8000_0000_0000_0000,
        ];
        let mut observations = 0;
        for (protocol, fixture) in protocols.into_iter().zip(fixtures) {
            for row in fixture.lines() {
                let (case, expected) = row.split_once('\t').unwrap();
                let case: usize = case.parse().unwrap();
                let value = f64::from_bits(bits[case]);
                let object = obj::Owned::fresh(obj::new_double_obj(value));
                assert!(!obj::has_string_rep(object.as_ptr()));
                let bytes = crate::dict::native_object_bytes(object.as_ptr(), protocol).unwrap();
                assert_eq!(obj::double_of(object.as_ptr()).to_bits(), bits[case]);
                let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                assert_eq!(hex, expected, "{protocol:?}/{case}");
                observations += 1;
            }
        }
        assert_eq!(observations, 36);
    }

    #[test]
    fn list_backing_canonical_flags_match_original_native_operations() {
        let fixtures = [
            include_str!("../../../rust/tcl-syntax/testdata/native_list_canonical/8.4.20.tsv"),
            include_str!("../../../rust/tcl-syntax/testdata/native_list_canonical/8.5.19.tsv"),
            include_str!("../../../rust/tcl-syntax/testdata/native_list_canonical/8.6.18.tsv"),
            include_str!("../../../rust/tcl-syntax/testdata/native_list_canonical/9.0.4.tsv"),
            include_str!("../../../rust/tcl-syntax/testdata/native_list_canonical/9.1.0.tsv"),
        ];
        let mut operations = 0;
        for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(fixtures) {
            let protocol = NativeStringProtocol::C(version);
            let rows: Vec<_> = fixture.lines().collect();
            for pair in rows.chunks_exact(2) {
                let before_fields: Vec<_> = pair[0].split('\t').collect();
                let after_fields: Vec<_> = pair[1].split('\t').collect();
                let mode: usize = before_fields[0].parse().unwrap();
                leak_free(|| {
                    let first = obj::Owned::fresh(obj::new_string_bytes(b"a"));
                    let second = obj::Owned::fresh(obj::new_string_bytes(b"b"));
                    let third = obj::Owned::fresh(obj::new_string_bytes(b"c"));
                    let root = obj::Owned::fresh(super::new_list_obj_native(
                        &[first.as_ptr(), second.as_ptr()],
                        protocol,
                    ));
                    let root = if mode == 4 {
                        let parsed = obj::Owned::fresh(obj::new_string_bytes(b"a   b"));
                        drop(
                            super::list_elements_native_checked(parsed.as_ptr(), protocol).unwrap(),
                        );
                        parsed
                    } else {
                        root
                    };
                    if (2..5).contains(&mode) {
                        crate::dict::native_object_bytes(root.as_ptr(), protocol).unwrap();
                    }
                    let duplicate = (mode == 3 || mode == 5)
                        .then(|| obj::Owned::fresh(obj::duplicate(root.as_ptr())));
                    let selected = duplicate.as_ref().unwrap_or(&root);
                    let before = super::native_cache_snapshot(selected.as_ptr()).unwrap().1;
                    assert_eq!(
                        obj::has_string_rep(selected.as_ptr()),
                        before_fields[3] == "1"
                    );
                    if version != tcl_dialect::TclVersion::V8_4 {
                        assert_eq!(before, before_fields[4] == "1");
                    }
                    let result = if mode == 0 || mode == 5 {
                        crate::dict::native_object_bytes(selected.as_ptr(), protocol).unwrap();
                        obj::Owned::retain(selected.as_ptr())
                    } else {
                        super::append_native_elements(
                            Some(selected.as_ptr()),
                            &[third.as_ptr()],
                            protocol,
                        )
                        .unwrap()
                    };
                    assert_eq!(obj::has_string_rep(result.as_ptr()), after_fields[3] == "1");
                    if version != tcl_dialect::TclVersion::V8_4 {
                        assert_eq!(
                            super::native_cache_snapshot(result.as_ptr()).unwrap().1,
                            after_fields[4] == "1"
                        );
                    }
                });
                operations += 1;
            }
        }
        assert_eq!(operations, 30);
    }

    #[test]
    fn selected_empty_compound_updaters_install_native_allocation_identity() {
        use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
        for protocol in [
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
            NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ] {
            let list = crate::obj::Owned::fresh(super::new_list_obj(&[]));
            super::seal_string_protocol(list.as_ptr(), protocol).unwrap();
            assert!(!crate::obj::has_string_rep(list.as_ptr()));
            assert_eq!(
                crate::dict::native_object_bytes(list.as_ptr(), protocol).unwrap(),
                b""
            );
            assert_eq!(
                crate::obj::has_canonical_empty_string(list.as_ptr()),
                protocol.compound_updater_storage() == NativeStringStorageIdentity::CanonicalEmpty
            );
            assert_eq!(
                super::native_list_backing(list.as_ptr())
                    .unwrap()
                    .canonical(),
                protocol.updated_list_canonical(false, false)
            );
            if protocol == NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4) {
                continue;
            }
            let dict = crate::obj::Owned::fresh(crate::dict::new_dict_obj(&[]));
            crate::dict::seal_string_protocol(dict.as_ptr(), protocol).unwrap();
            assert_eq!(
                crate::dict::native_object_bytes(dict.as_ptr(), protocol).unwrap(),
                b""
            );
            assert_eq!(
                crate::obj::has_canonical_empty_string(dict.as_ptr()),
                protocol.compound_updater_storage() == NativeStringStorageIdentity::CanonicalEmpty
            );
        }
    }

    #[test]
    fn compound_updater_matches_all_six_native_original_object_fixtures() {
        use tcl_dialect::TclVersion;
        const FIXTURES: [&str; 6] = [
            include_str!(
                "../../../rust/tcl-syntax/testdata/native_compound_string_updaters/8.4.20.tsv"
            ),
            include_str!(
                "../../../rust/tcl-syntax/testdata/native_compound_string_updaters/8.5.19.tsv"
            ),
            include_str!(
                "../../../rust/tcl-syntax/testdata/native_compound_string_updaters/8.6.18.tsv"
            ),
            include_str!(
                "../../../rust/tcl-syntax/testdata/native_compound_string_updaters/9.0.4.tsv"
            ),
            include_str!(
                "../../../rust/tcl-syntax/testdata/native_compound_string_updaters/9.1.0.tsv"
            ),
            include_str!(
                "../../../rust/tcl-syntax/testdata/native_compound_string_updaters/jim.tsv"
            ),
        ];
        let protocols = [
            NativeStringProtocol::C(TclVersion::V8_4),
            NativeStringProtocol::C(TclVersion::V8_5),
            NativeStringProtocol::C(TclVersion::V8_6),
            NativeStringProtocol::C(TclVersion::V9_0),
            NativeStringProtocol::C(TclVersion::V9_1),
            NativeStringProtocol::Jim084,
        ];
        let mut rows = 0;
        let mut unavailable = 0;
        for (protocol, fixture) in protocols.into_iter().zip(FIXTURES) {
            let source_context = protocol.is_jim084().then(|| {
                crate::native_source::NativeJimObjectContext::new(
                    tcl_registry::InvocationDialect::of_profile(
                        crate::environment::profile_for_dialect("jim"),
                    ),
                )
                .unwrap()
            });

            for row in fixture.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                rows += 1;
                if fields[1] == "unavailable" {
                    assert_eq!(protocol, protocols[0]);
                    unavailable += 1;
                    continue;
                }
                let case: u8 = fields[0].parse().unwrap();
                let list = |items: &[*mut TclObj]| super::new_list_obj_native(items, protocol);
                let dict = || {
                    let root = crate::dict::new_dict_obj(&[(
                        obj::new_string_bytes(b"#key"),
                        obj::new_string_bytes(b"a\"b"),
                    )]);
                    crate::dict::seal_string_protocol(root, protocol).unwrap();
                    root
                };
                let root = obj::Owned::fresh(match case {
                    0 => list(&[
                        obj::new_string_bytes(b"#first"),
                        obj::new_string_bytes(b"#later"),
                    ]),
                    1 => list(&[obj::new_string_bytes(b"a\"b"), obj::new_string_bytes(b"]")]),
                    2 => list(&[
                        obj::new_string_bytes(b"A\0\xff"),
                        obj::new_string_bytes(b"\\\n"),
                    ]),
                    3 => list(&[
                        list(&[obj::new_wide_int_obj(17), obj::new_string_bytes(b"a\"b")]),
                        obj::new_string_bytes(b"#later"),
                    ]),
                    4 => dict(),
                    5 => list(&[dict(), obj::new_string_bytes(b"#later")]),
                    6 => {
                        let value = obj::new_string_bytes(b"ORIGINAL");
                        if let Some(context) = &source_context {
                            crate::native_source::bind_context(value, context).unwrap();
                        }
                        drop(super::list_elements_native_checked(value, protocol).unwrap());
                        value
                    }
                    _ => unreachable!(),
                });
                let type_name = |value| {
                    let descriptor = obj::obj_type_ptr(value);
                    if descriptor.is_null() {
                        "none"
                    } else {
                        unsafe { core::ffi::CStr::from_ptr((*descriptor).name) }
                            .to_str()
                            .unwrap()
                    }
                };
                let state = || {
                    let child = if let Some(backing) = super::native_list_backing(root.as_ptr()) {
                        backing.elements().unwrap()[0]
                    } else {
                        crate::dict::dict_pairs(root.as_ptr()).unwrap()[0].1
                    };
                    (
                        type_name(root.as_ptr()),
                        obj::has_string_rep(root.as_ptr()),
                        type_name(child),
                        obj::has_string_rep(child),
                        unsafe { (*child).ref_count },
                    )
                };
                let before = state();
                assert_eq!(before.0, fields[1]);
                assert_eq!(before.1, fields[2] == "1");
                assert_eq!(before.2, fields[3]);
                assert_eq!(before.3, fields[4] == "1");
                assert_eq!(before.4, fields[10].parse::<isize>().unwrap());
                let bytes = crate::dict::native_object_bytes(root.as_ptr(), protocol).unwrap();
                let hex = bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                assert_eq!(hex, fields[5], "{protocol:?}: {row}");
                let after = state();
                assert_eq!(after.0, fields[6]);
                assert_eq!(after.1, fields[7] == "1");
                assert_eq!(after.2, fields[8]);
                assert_eq!(after.2, before.2);
                assert_eq!(after.3, fields[9] == "1");
                assert_eq!(after.4, fields[11].parse::<isize>().unwrap());
            }
        }
        assert_eq!(rows, 42);
        assert_eq!(unavailable, 2);
    }

    // These ABI backing tests choose the audited C8.5 string updater explicitly.
    fn test_list(elems: &[*mut TclObj]) -> *mut TclObj {
        super::new_list_obj_native(
            elems,
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
        )
    }
    use super::*;
    use crate::counters;
    use crate::obj::{TclObj, new_string_bytes};

    fn leak_free(body: impl FnOnce()) {
        counters::reset();
        body();
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    fn release(obj: *mut TclObj) {
        unsafe {
            obj::incr_ref_count(obj);
            obj::decr_ref_count(obj);
        }
    }

    #[test]
    fn native_dictionary_list_conversion_keeps_members_and_resident_spelling() {
        use tcl_syntax::native_string::NativeStringProtocol;
        leak_free(|| {
            for protocol in [
                NativeStringProtocol::C(tcl_dialect::TclVersion::V8_5),
                NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
                NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
                NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
                NativeStringProtocol::Jim084,
            ] {
                let key = obj::Owned::fresh(obj::new_string_bytes(b"k\0\xff"));
                let child = obj::Owned::fresh(obj::new_wide_int_obj(7));
                let parent =
                    obj::Owned::fresh(crate::dict::new_dict_obj(&[(key.as_ptr(), child.as_ptr())]));
                let alias = parent.clone();
                let members = list_elements_native_checked(parent.as_ptr(), protocol).unwrap();
                assert_eq!(members, [key.as_ptr(), child.as_ptr()]);
                assert!(!obj::has_string_rep(alias.as_ptr()));
                assert!(!obj::has_string_rep(child.as_ptr()));
                assert!(!native_list_backing(alias.as_ptr()).unwrap().canonical());
                assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);

                let resident =
                    obj::Owned::fresh(crate::dict::new_dict_obj(&[(key.as_ptr(), child.as_ptr())]));
                unsafe { obj::set_string_rep(resident.as_ptr(), b"k FIRST k LAST") };
                let source_context = protocol.is_jim084().then(|| {
                    crate::native_source::NativeJimObjectContext::new(
                        tcl_registry::InvocationDialect::of_profile(
                            crate::environment::profile_for_dialect("jim"),
                        ),
                    )
                    .unwrap()
                });
                if let Some(context) = &source_context {
                    crate::native_source::bind_context(resident.as_ptr(), context).unwrap();
                }
                let members = list_elements_native_checked(resident.as_ptr(), protocol).unwrap();
                assert_eq!(members.len(), 4);
                assert_eq!(obj::bytes_of(members[1]), b"FIRST");
                assert_eq!(obj::bytes_of(members[3]), b"LAST");
                assert_ne!(members[1], child.as_ptr());
            }
        });
    }

    #[test]
    fn lifetime_views_do_not_create_native_headers_or_force_member_cow() {
        leak_free(|| {
            let child = obj::Owned::fresh(obj::new_string_bytes(b"MEMBER"));
            let parent = obj::Owned::fresh(test_list(&[child.as_ptr()]));
            let view = native_list_backing(parent.as_ptr()).unwrap();
            let another_view = view.clone();
            assert_eq!(&*view.elements().unwrap(), &[child.as_ptr()]);
            assert_eq!(&*another_view.elements().unwrap(), &[child.as_ptr()]);
            assert_eq!(
                native_header_reference_count(parent.as_ptr()).unwrap(),
                Some(1)
            );
            let extra = obj::Owned::fresh(obj::new_string_bytes(b"EXTRA"));
            list_append(parent.as_ptr(), extra.as_ptr()).unwrap();
            assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);
            // Lifetime views keep allocation safety without authorising a new
            // member-vector generation or adding native List header owners.
            let changed = tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "changed native List member vector",
            );
            assert_eq!(view.elements().unwrap_err(), changed);
            assert_eq!(another_view.elements().unwrap_err(), changed);
            let current = native_list_backing(parent.as_ptr()).unwrap();
            assert_eq!(
                &*current.elements().unwrap(),
                &[child.as_ptr(), extra.as_ptr()]
            );
            let copy = obj::Owned::fresh(obj::duplicate(parent.as_ptr()));
            assert_eq!(
                native_header_reference_count(parent.as_ptr()).unwrap(),
                Some(2)
            );
            assert_eq!(
                native_header_reference_count(copy.as_ptr()).unwrap(),
                Some(2)
            );
            list_append(copy.as_ptr(), extra.as_ptr()).unwrap();
            assert_eq!(
                native_header_reference_count(parent.as_ptr()).unwrap(),
                Some(1)
            );
            assert_eq!(
                native_header_reference_count(copy.as_ptr()).unwrap(),
                Some(1)
            );
            assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 3);
            // Mutating the copied header detaches its backing; the original
            // current vector remains readable without forcing member COW.
            assert_eq!(current.elements().unwrap().len(), 2);
            assert_eq!(view.elements().unwrap_err(), changed);
        });
    }

    #[test]
    fn original_backing_survives_shimmer_without_extra_child_owners() {
        leak_free(|| {
            let child = obj::Owned::fresh(obj::new_string_bytes(b"CHILD\xff"));
            let parent = obj::Owned::fresh(test_list(&[child.as_ptr()]));
            assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);
            let backing = native_list_backing(parent.as_ptr()).unwrap();
            assert!(!backing.canonical());
            assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);
            obj::change_type(parent.as_ptr(), &obj::TCL_INT_TYPE, 7);
            assert_eq!(&*backing.elements().unwrap(), &[child.as_ptr()]);
            assert_eq!(obj::bytes_of(backing.elements().unwrap()[0]), b"CHILD\xff");
            assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 1);
            drop(backing);
            assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 1);
        });
    }

    #[test]
    fn native_c_list_copy_shares_backing_until_first_mutation() {
        leak_free(|| {
            let protocol = tcl_syntax::native_string::NativeStringProtocol::for_tcl_version(
                tcl_dialect::TclVersion::V9_0,
            );
            let child = obj::Owned::fresh(obj::new_wide_int_obj(7));
            let parent = obj::Owned::fresh(test_list(&[child.as_ptr()]));
            let copy = obj::Owned::fresh(obj::duplicate(parent.as_ptr()));
            duplicate_native_backing(parent.as_ptr(), copy.as_ptr(), protocol);
            assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 2);
            let pinned = native_list_backing(parent.as_ptr()).unwrap();
            let extra = obj::Owned::fresh(obj::new_string_bytes(b"EXTRA"));
            list_append(copy.as_ptr(), extra.as_ptr()).unwrap();
            assert_eq!(&*pinned.elements().unwrap(), &[child.as_ptr()]);
            assert_eq!(
                list_elements(copy.as_ptr()).unwrap(),
                vec![child.as_ptr(), extra.as_ptr()]
            );
            assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 3);
        });
    }

    #[test]
    fn build_index_length() {
        leak_free(|| {
            let a = new_string_bytes(b"a");
            let b = new_string_bytes(b"b");
            let c = new_string_bytes(b"c");
            let list = test_list(&[a, b, c]);
            unsafe { obj::incr_ref_count(list) };
            assert_eq!(list_length(list).unwrap(), 3);
            assert_eq!(obj::bytes_of(list_index(list, 1).unwrap().unwrap()), b"b");
            assert!(list_index(list, 9).unwrap().is_none());
            // the bare element objs (a,b,c) are owned by the list; free our
            // construction refs (they were rc 0, list took them to rc 1)
            unsafe { obj::decr_ref_count(list) }; // frees list + its 3 elements
        });
    }

    #[test]
    fn list_to_string_quotes() {
        leak_free(|| {
            let plain = new_string_bytes(b"a");
            let spaced = new_string_bytes(b"b c");
            let empty = new_string_bytes(b"");
            let list = test_list(&[plain, spaced, empty]);
            unsafe { obj::incr_ref_count(list) };
            // a {b c} {}
            assert_eq!(obj::bytes_of(list), b"a {b c} {}");
            unsafe { obj::decr_ref_count(list) };
        });
    }

    #[test]
    fn string_to_list_shimmer() {
        leak_free(|| {
            let s = new_string_bytes(b"x {y z} w");
            unsafe { obj::incr_ref_count(s) };
            // reading length shimmers the string into a list
            assert_eq!(list_length(s).unwrap(), 3);
            assert_eq!(obj::bytes_of(list_index(s, 1).unwrap().unwrap()), b"y z");
            unsafe { obj::decr_ref_count(s) };
        });
    }

    #[test]
    fn append_invalidates_string_rep() {
        leak_free(|| {
            let list = test_list(&[new_string_bytes(b"a")]);
            unsafe { obj::incr_ref_count(list) };
            assert_eq!(obj::bytes_of(list), b"a");
            list_append(list, new_string_bytes(b"b")).unwrap();
            // string rep regenerated to include the new element
            assert_eq!(obj::bytes_of(list), b"a b");
            assert_eq!(list_length(list).unwrap(), 2);
            unsafe { obj::decr_ref_count(list) };
        });
    }

    #[test]
    fn list_element_quoting_modes() {
        let q = |e: &[u8]| {
            let mut buf = Vec::new();
            append_list_element(&mut buf, e, true);
            buf
        };
        // escape mode (unbalanced open brace) — braces escaped too.
        assert_eq!(q(b"a{b"), b"a\\{b");
        // bare — balanced braces need no quoting.
        assert_eq!(q(b"a{b}c"), b"a{b}c");
        // brace mode — `[`/`$`/`;` force quoting but braces round-trip.
        assert_eq!(q(b"[append"), b"{[append}");
        assert_eq!(q(b"a$b"), b"{a$b}");
        assert_eq!(q(b"a;b"), b"{a;b}");
        assert_eq!(q(b"a[b]c"), b"{a[b]c}");
        // mask mode — a lone `]`/`"` escapes (braces left literal).
        assert_eq!(q(b".b]"), b".b\\]");
        assert_eq!(q(b"x]y"), b"x\\]y");
        // leading `#` (first element) → brace.
        assert_eq!(q(b"#c"), b"{#c}");
        // whitespace → brace.
        assert_eq!(q(b"a b"), b"{a b}");
        let _ = release; // silence unused in this pure test
    }
}
