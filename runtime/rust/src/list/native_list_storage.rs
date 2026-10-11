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

//! Original native range header and allocated backing windows.
use super::*;
use tcl_cmd_core::native_list_storage::{
    NativeListRangeAction, NativeListRangeSelection, NativeListRangeStorage,
};
use tcl_dialect::TclVersion;
use tcl_syntax::{native_compiled_index::NativeCompiledListRange, value::ValueError};

pub(crate) fn native_list_range(
    original: *mut TclObj,
    coordinates: NativeCompiledListRange,
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, ValueError> {
    native_list_selected_range(
        original,
        NativeListRangeSelection::Immediate(coordinates),
        protocol,
    )
}

pub(crate) fn native_list_command_range(
    original: *mut TclObj,
    first: i64,
    last: i64,
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, ValueError> {
    native_list_selected_range(
        original,
        NativeListRangeSelection::Command { first, last },
        protocol,
    )
}

/// Original command range with the same checked conversion, mutation and
/// returned-owner stages. The checker grants no native header reference.
pub(crate) fn native_list_command_range_using<E>(
    original: *mut TclObj,
    first: i64,
    last: i64,
    protocol: NativeStringProtocol,
    check: &impl Fn() -> Result<(), E>,
    refuse: &impl Fn(ValueError) -> E,
) -> Result<obj::Owned, E> {
    native_list_selected_range_using(
        original,
        NativeListRangeSelection::Command { first, last },
        protocol,
        Some(check),
        refuse,
    )
}

fn native_list_selected_range(
    original: *mut TclObj,
    selection: NativeListRangeSelection,
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, ValueError> {
    native_list_selected_range_using(original, selection, protocol, None, &std::convert::identity)
}

fn native_list_selected_range_using<E>(
    original: *mut TclObj,
    selection: NativeListRangeSelection,
    protocol: NativeStringProtocol,
    check: Option<&dyn Fn() -> Result<(), E>>,
    refuse: &impl Fn(ValueError) -> E,
) -> Result<obj::Owned, E> {
    let stage = || match check {
        Some(check) => check(),
        None => Ok(()),
    };
    stage()?;
    let version = protocol.tcl_version().ok_or_else(|| {
        refuse(ValueError::CommandProtocolUnavailable(
            "native compiled List range release",
        ))
    })?;
    if selection.is_immediate()
        && version >= TclVersion::V9_0
        && obj::has_canonical_empty_string(original)
    {
        return Ok(obj::Owned::retain(original));
    }
    if version >= TclVersion::V9_0 && crate::native_arithseries::is_series(original) {
        return Err(refuse(ValueError::CommandProtocolUnavailable(
            "native abstract List slice provider",
        )));
    }
    if check.is_some() {
        prepare_native_list_mutation_using(original, protocol, &stage, refuse)?;
    } else {
        prepare_native_list_mutation(original, protocol).map_err(refuse)?;
    }
    stage()?;
    let shared = obj::is_shared(original);
    if version >= TclVersion::V9_0 && !shared {
        if check.is_some() {
            collect_replacement_unreferenced(original, &stage, refuse)?;
        } else {
            unsafe { list_mut(original) }.elems.collect_unreferenced();
        }
    }
    stage()?;
    let action = {
        let list = unsafe { list_ref(original) };
        selection
            .action(
                version,
                NativeListRangeStorage {
                    length: list.elems.len(),
                    header_shared: shared,
                    store_shared: list.elems.native_is_shared(),
                    has_span: list.elems.span,
                    used: list.elems.backing.elements.borrow().0.len(),
                    allocated: list.elems.backing.capacity.get(),
                    string: tcl_cmd_core::native_list_storage::NativeListStringState::from_empty(
                        !obj::has_string_rep(original) || obj::bytes_of(original).is_empty(),
                    ),
                },
            )
            .map_err(refuse)?
    };
    let reached =
        apply_native_list_range_using(original, action, shared, version, protocol, check, refuse);
    stage()?;
    reached
}

fn apply_native_list_range_using<E>(
    original: *mut TclObj,
    action: NativeListRangeAction,
    shared: bool,
    version: TclVersion,
    protocol: NativeStringProtocol,
    check: Option<&dyn Fn() -> Result<(), E>>,
    refuse: &impl Fn(ValueError) -> E,
) -> Result<obj::Owned, E> {
    let stage = || match check {
        Some(check) => check(),
        None => Ok(()),
    };
    stage()?;
    match action {
        NativeListRangeAction::OriginalEmpty => Ok(obj::Owned::retain(original)),
        NativeListRangeAction::FreshEmpty => Ok(obj::Owned::fresh(obj::new_obj())),
        NativeListRangeAction::EmptyList => {
            let backing = TclList {
                elems: NativeListStorage::new(Vec::new()),
                canonical: Rc::new(Cell::new(false)),
                string_protocol: Cell::new(Some(protocol)),
            };
            if shared {
                Ok(obj::Owned::fresh(obj::alloc_typed(
                    &TCL_LIST_TYPE,
                    Box::into_raw(Box::new(backing)) as usize as u64,
                )))
            } else {
                if check.is_some() {
                    replace_storage(original, backing.elems, &stage, refuse)?;
                    let list = unsafe { list_mut(original) };
                    list.canonical = backing.canonical;
                    list.string_protocol.set(Some(protocol));
                } else {
                    *unsafe { list_mut(original) } = backing;
                }
                obj::invalidate_string(original);
                Ok(obj::Owned::retain(original))
            }
        }
        NativeListRangeAction::FreshMembers(range) => {
            let elements = unsafe { list_ref(original) }.elems.elements()[range].to_vec();
            if version >= TclVersion::V9_0 && !shared {
                for &element in &elements {
                    unsafe { obj::incr_ref_count(element) };
                }
                let replacement = NativeListStorage::new(elements);
                if check.is_some() {
                    replace_storage(original, replacement, &stage, refuse)?;
                } else {
                    unsafe { list_mut(original) }.elems = replacement;
                }
                unsafe { list_mut(original) }.canonical = Rc::new(Cell::new(false));
                obj::invalidate_string(original);
                Ok(obj::Owned::retain(original))
            } else {
                Ok(obj::Owned::fresh(new_list_obj_native(&elements, protocol)))
            }
        }
        NativeListRangeAction::Whole => {
            if shared {
                if check.is_some() {
                    native_list_copy_using(original, protocol, &stage, refuse)
                } else {
                    native_list_copy(original, protocol).map_err(refuse)
                }
            } else {
                obj::invalidate_string(original);
                Ok(obj::Owned::retain(original))
            }
        }
        NativeListRangeAction::Window { range, span } => {
            if shared {
                let list = unsafe { list_ref(original) };
                let backing = Box::new(TclList {
                    elems: list.elems.range_header(range),
                    canonical: Rc::clone(&list.canonical),
                    string_protocol: Cell::new(Some(protocol)),
                });
                Ok(obj::Owned::fresh(obj::alloc_typed(
                    &TCL_LIST_TYPE,
                    Box::into_raw(backing) as usize as u64,
                )))
            } else {
                if check.is_some() {
                    select_replacement_window(original, range, span, &stage, refuse)?;
                } else {
                    unsafe { list_mut(original) }
                        .elems
                        .select_window(range, span);
                }
                if version >= TclVersion::V9_0 && !span {
                    unsafe { list_mut(original) }.canonical.set(false);
                }
                obj::invalidate_string(original);
                Ok(obj::Owned::retain(original))
            }
        }
    }
}

// A replacement observer pins only allocation/backing identity. It does not
// add a native header or member reference, and is renewed only by this worker's
// own short mutation stages, before any external retirement callback.
struct ReplacementObserver {
    backing: Rc<NativeListStorageData>,
    generation: u64,
    window: Option<std::ops::Range<usize>>,
    span: bool,
}
impl ReplacementObserver {
    fn capture(original: *mut TclObj) -> Self {
        let storage = &unsafe { list_ref(original) }.elems;
        Self {
            backing: Rc::clone(&storage.backing),
            generation: storage.generation,
            window: storage.window.clone(),
            span: storage.span,
        }
    }
    fn check<E>(&self, original: *mut TclObj, refuse: &impl Fn(ValueError) -> E) -> Result<(), E> {
        obj::check_native_liveness(original).map_err(refuse)?;
        if obj::obj_type_ptr(original) != &TCL_LIST_TYPE {
            return Err(refuse(ValueError::CommandProtocolUnavailable(
                "changed native List replacement header",
            )));
        }
        let storage = &unsafe { list_ref(original) }.elems;
        if !Rc::ptr_eq(&self.backing, &storage.backing)
            || self.generation != storage.generation
            || self.generation != storage.backing.generation.get()
            || self.window != storage.window
            || self.span != storage.span
        {
            return Err(refuse(ValueError::CommandProtocolUnavailable(
                "changed native List replacement backing",
            )));
        }
        Ok(())
    }
}

// A displaced real native header is dismantled outside the live receiver
// borrow. Child retirement remains in its original order, including mandatory
// remaining releases after the first operational refusal.
fn retire_storage(storage: &mut NativeListStorage) -> Vec<*mut TclObj> {
    if !storage.header {
        return Vec::new();
    }
    storage.header = false;
    storage.backing.headers.set(
        storage
            .backing
            .headers
            .get()
            .checked_sub(1)
            .expect("native List header retirement"),
    );
    if storage.backing.headers.get() != 0 {
        return Vec::new();
    }
    let elements = storage.backing.elements.borrow();
    if !elements.1.get() {
        return Vec::new();
    }
    elements.2.replace(
        elements
            .0
            .iter()
            .map(|&value| obj::NativeObjectLifetime::retain(value))
            .collect(),
    );
    elements.1.set(false);
    elements.0.clone()
}

fn drain_replacement_members<E>(
    original: *mut TclObj,
    retired: Vec<*mut TclObj>,
    observer: &ReplacementObserver,
    check: &impl Fn() -> Result<(), E>,
    refuse: &impl Fn(ValueError) -> E,
) -> Result<(), E> {
    let pins = retired
        .iter()
        .map(|&value| obj::NativeObjectLifetime::retain(value))
        .collect::<Vec<_>>();
    let mut first = None;
    for value in retired {
        unsafe { obj::decr_ref_count(value) };
        let reached = check().and_then(|()| observer.check(original, refuse));
        if first.is_none() {
            first = reached.err();
        }
    }
    drop(pins);
    let reached = check().and_then(|()| observer.check(original, refuse));
    match first {
        Some(error) => Err(error),
        None => reached,
    }
}

pub(super) fn replace_storage<E>(
    original: *mut TclObj,
    replacement: NativeListStorage,
    check: &impl Fn() -> Result<(), E>,
    refuse: &impl Fn(ValueError) -> E,
) -> Result<(), E> {
    let mut retired = {
        let list = unsafe { list_mut(original) };
        std::mem::replace(&mut list.elems, replacement)
    };
    let observer = ReplacementObserver::capture(original);
    let values = retire_storage(&mut retired);
    let outcome = drain_replacement_members(original, values, &observer, check, refuse);
    drop(retired);
    let settled = check().and_then(|()| observer.check(original, refuse));
    outcome.and(settled)
}

fn select_replacement_window<E>(
    original: *mut TclObj,
    range: std::ops::Range<usize>,
    span: bool,
    check: &impl Fn() -> Result<(), E>,
    refuse: &impl Fn(ValueError) -> E,
) -> Result<(), E> {
    let stage = {
        let storage = &mut unsafe { list_mut(original) }.elems;
        let offset = storage
            .window
            .as_ref()
            .map_or(0, |window| window.start - storage.backing.first_used.get());
        let start = offset + range.start;
        let end = offset + range.end;
        if storage.native_is_shared() {
            storage.window = Some(
                storage.backing.first_used.get() + start..storage.backing.first_used.get() + end,
            );
            storage.span = span;
            None
        } else {
            let retired = {
                let mut elements = storage.backing.elements.borrow_mut();
                let mut retired = elements.0.drain(end..).collect::<Vec<_>>();
                retired.extend(elements.0.drain(..start));
                retired
            };
            Some((start, retired))
        }
    };
    if let Some((start, retired)) = stage {
        let observer = ReplacementObserver::capture(original);
        drain_replacement_members(original, retired, &observer, check, refuse)?;
        let storage = &mut unsafe { list_mut(original) }.elems;
        storage.backing.first_used.set(if span {
            storage.backing.first_used.get() + start
        } else {
            0
        });
        storage.window = None;
        storage.note_mutation();
        storage.span = span;
    }
    check()
}

fn collect_replacement_unreferenced<E>(
    original: *mut TclObj,
    check: &impl Fn() -> Result<(), E>,
    refuse: &impl Fn(ValueError) -> E,
) -> Result<(), E> {
    let range = {
        let storage = &unsafe { list_ref(original) }.elems;
        (!storage.native_is_shared() && storage.window.is_some())
            .then(|| (0..storage.len(), storage.span))
    };
    if let Some((range, span)) = range {
        select_replacement_window(original, range, span, check, refuse)?;
    }
    Ok(())
}

fn replace_selected_native_storage<E>(
    original: *mut TclObj,
    first: usize,
    delete: usize,
    insert: &[*mut TclObj],
    version: TclVersion,
    check: &impl Fn() -> Result<(), E>,
    refuse: &impl Fn(ValueError) -> E,
) -> Result<bool, E> {
    use tcl_cmd_core::native_list_storage::{
        NativeListReplaceStorage, range_storage_action, replace_layout,
    };
    unsafe { list_ref(original) }
        .elems
        .checked_generation()
        .map_err(refuse)?;
    let length = unsafe { list_ref(original) }.elems.len();
    let first = first.min(length);
    let delete = delete.min(length - first);
    if insert.is_empty() && delete == 0 {
        return Ok(false);
    }
    if version >= TclVersion::V9_0 && insert.is_empty() && (first == 0 || first + delete == length)
    {
        collect_replacement_unreferenced(original, check, refuse)?;
        let range = if first == 0 { delete..length } else { 0..first };
        if range.is_empty() {
            replace_storage(original, NativeListStorage::new(Vec::new()), check, refuse)?;
            return Ok(true);
        }
        let action = {
            let storage = &unsafe { list_ref(original) }.elems;
            range_storage_action(
                version,
                range,
                NativeListRangeStorage {
                    length: storage.len(),
                    header_shared: false,
                    store_shared: storage.native_is_shared(),
                    has_span: storage.span,
                    used: storage.backing.elements.borrow().0.len(),
                    allocated: storage.backing.capacity.get(),
                    string: tcl_cmd_core::native_list_storage::NativeListStringState::Nonempty,
                },
            )
            .map_err(refuse)?
        };
        return match action {
            NativeListRangeAction::Window { range, span } => {
                select_replacement_window(original, range, span, check, refuse)?;
                Ok(!span)
            }
            NativeListRangeAction::FreshMembers(range) => {
                let elements = unsafe { list_ref(original) }.elems.elements()[range].to_vec();
                for &value in &elements {
                    unsafe { obj::incr_ref_count(value) };
                }
                replace_storage(original, NativeListStorage::new(elements), check, refuse)?;
                Ok(true)
            }
            NativeListRangeAction::Whole => Ok(false),
            _ => unreachable!("nonempty private List range"),
        };
    }
    collect_replacement_unreferenced(original, check, refuse)?;
    let layout = if version >= TclVersion::V9_0 {
        let storage = &unsafe { list_ref(original) }.elems;
        Some(replace_layout(
            NativeListReplaceStorage {
                length: storage.len(),
                used: storage.backing.elements.borrow().0.len(),
                allocated: storage.backing.capacity.get().ok_or_else(|| {
                    refuse(ValueError::CommandProtocolUnavailable(
                        "native List allocation extent",
                    ))
                })?,
                first_used: storage.backing.first_used.get(),
                window_start: storage
                    .window
                    .as_ref()
                    .map_or(storage.backing.first_used.get(), |window| window.start),
                has_span: storage.span,
                store_shared: storage.native_is_shared(),
            },
            first,
            delete,
            insert.len(),
        ))
    } else {
        None
    };
    if let Some(layout) = layout.filter(|layout| layout.shared_prepend) {
        let storage = &mut unsafe { list_mut(original) }.elems;
        let start = storage
            .window
            .as_ref()
            .map_or(storage.backing.first_used.get(), |window| window.start);
        let end = start + storage.len();
        for &value in insert {
            unsafe { obj::incr_ref_count(value) };
        }
        storage
            .backing
            .elements
            .borrow_mut()
            .0
            .splice(0..0, insert.iter().copied());
        storage.backing.first_used.set(layout.first_used);
        storage.window = Some(start - insert.len()..end);
        storage.span = true;
        return Ok(false);
    }
    if let Some(layout) = layout.filter(|layout| layout.new_store) {
        let elements = {
            let members = unsafe { list_ref(original) }.elems.elements();
            members[..first]
                .iter()
                .chain(insert)
                .chain(&members[first + delete..])
                .copied()
                .collect::<Vec<_>>()
        };
        for &value in &elements {
            unsafe { obj::incr_ref_count(value) };
        }
        replace_storage(original, NativeListStorage::new(elements), check, refuse)?;
        let storage = &mut unsafe { list_mut(original) }.elems;
        storage.set_capacity(layout.allocated);
        storage.backing.first_used.set(layout.first_used);
        storage.span = layout.span;
        return Ok(true);
    }
    let copy = {
        let storage = &unsafe { list_ref(original) }.elems;
        storage.native_is_shared().then(|| storage.copied_header())
    };
    if let Some(copy) = copy {
        replace_storage(original, copy, check, refuse)?;
    }
    collect_replacement_unreferenced(original, check, refuse)?;
    let retired = {
        let storage = &mut unsafe { list_mut(original) }.elems;
        for &value in insert {
            unsafe { obj::incr_ref_count(value) };
        }
        let retired = storage
            .backing
            .elements
            .borrow_mut()
            .0
            .splice(first..first + delete, insert.iter().copied())
            .collect::<Vec<_>>();
        retired
    };
    let observer = ReplacementObserver::capture(original);
    drain_replacement_members(original, retired, &observer, check, refuse)?;
    let storage = &mut unsafe { list_mut(original) }.elems;
    storage.note_mutation();
    if let Some(layout) = layout {
        storage.set_capacity(layout.allocated);
        storage.backing.first_used.set(layout.first_used);
        storage.span = layout.span;
    } else {
        storage.backing.capacity.set(None);
        storage.backing.first_used.set(0);
        storage.span = false;
    }
    storage.window = None;
    Ok(version >= TclVersion::V9_0 && !(delete == 0 && first + insert.len() == storage.len()))
}

pub(super) fn replace_prepared_native_elements_using<E>(
    original: *mut TclObj,
    first: usize,
    delete: usize,
    insert: &[*mut TclObj],
    protocol: NativeStringProtocol,
    invalidate: bool,
    check: &impl Fn() -> Result<(), E>,
    refuse: &impl Fn(ValueError) -> E,
) -> Result<(), E> {
    check()?;
    let version = protocol.tcl_version().ok_or_else(|| {
        refuse(ValueError::CommandProtocolUnavailable(
            "native List replacement release",
        ))
    })?;
    let pins = std::iter::once(original)
        .chain(insert.iter().copied())
        .map(|value| {
            obj::check_native_liveness(value).map_err(refuse)?;
            Ok(obj::NativeObjectLifetime::retain(value))
        })
        .collect::<Result<Vec<_>, E>>()?;
    prepare_native_list_mutation_using(original, protocol, check, refuse)?;
    let observer = ReplacementObserver::capture(original);
    observer.check(original, refuse)?;
    let previous = unsafe { list_ref(original) }.canonical.get();
    let backing = Rc::clone(&unsafe { list_ref(original) }.elems.backing);
    let reset =
        replace_selected_native_storage(original, first, delete, insert, version, check, refuse)?;
    let list = unsafe { list_mut(original) };
    if !Rc::ptr_eq(&backing, &list.elems.backing) {
        list.canonical = Rc::new(Cell::new(if reset {
            false
        } else {
            protocol.copied_list_canonical(previous)
        }));
    } else if reset {
        list.canonical.set(false);
    }
    if invalidate {
        obj::invalidate_string(original);
    }
    drop(pins);
    check()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::native_compiled_index::NativeCompiledListIndex;

    #[test]
    fn command_range_empty_result_has_actual_list_store_only_in_c90() {
        // naming.list.original-range-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-range-objects-and-instructions.md
        // R3's physical header is observed before any string/list result getter.
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let original = obj::Owned::fresh(obj::new_string_bytes(b"A B C"));
            let alias = obj::Owned::retain(original.as_ptr());
            let result = native_list_command_range(
                original.as_ptr(),
                12,
                2,
                NativeStringProtocol::C(version),
            )
            .unwrap();
            assert_eq!(
                core::ptr::eq(obj::obj_type_ptr(result.as_ptr()), &TCL_LIST_TYPE),
                version == TclVersion::V9_0
            );
            assert_eq!(
                obj::has_string_rep(result.as_ptr()),
                version != TclVersion::V9_0
            );
            if version == TclVersion::V9_0 {
                let result_list = unsafe { list_ref(result.as_ptr()) };
                assert_eq!(result_list.elems.len(), 0);
                assert_eq!(result_list.elems.backing.capacity.get(), Some(1));
            }
            assert_eq!(&*obj::bytes_of(original.as_ptr()), b"A B C");
            drop(alias);
        }
    }

    #[test]
    fn runtime_list_ranges_preserve_all_five_native_shared_header_windows() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeStringProtocol::C(version);
            for (first, last, length) in [(0, -2, 3), (1, 1, 1)] {
                let original = obj::Owned::fresh(obj::new_string_bytes(b"{A} B C"));
                let alias = obj::Owned::retain(original.as_ptr());
                let result = native_list_range(
                    original.as_ptr(),
                    NativeCompiledListRange {
                        first: NativeCompiledListIndex::from_encoded(first),
                        last: NativeCompiledListIndex::from_encoded(last),
                    },
                    protocol,
                )
                .unwrap();
                assert_ne!(original.as_ptr(), result.as_ptr());
                assert_eq!(unsafe { (*result.as_ptr()).ref_count }, 1);
                assert!(!obj::has_string_rep(result.as_ptr()));
                assert_eq!(&*obj::bytes_of(original.as_ptr()), b"{A} B C");
                let source = unsafe { list_ref(original.as_ptr()) };
                let target = unsafe { list_ref(result.as_ptr()) };
                assert_eq!(target.elems.len(), length);
                assert_eq!(
                    Rc::ptr_eq(&source.elems.backing, &target.elems.backing),
                    version >= TclVersion::V9_0 && first == 0
                );
                drop(alias);
                assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
            }
        }
    }
}

#[cfg(all(test, have_tommath))]
mod original_range_retirement_controls {
    use super::*;
    use crate::interp::{Interp, native_arguments::OriginalArguments};
    use std::cell::{Cell, RefCell};
    thread_local! {
        static MODE: Cell<u8> = const { Cell::new(0) };
        static FREES: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
        static HEADER: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
        static REFERENCES: RefCell<Vec<obj::TclSize>> = const { RefCell::new(Vec::new()) };
    }
    extern "C" fn free_member(value: *mut TclObj) {
        let id = obj::internal_rep(value);
        FREES.with(|calls| calls.borrow_mut().push(id));
        REFERENCES.with(|references| {
            references
                .borrow_mut()
                .push(unsafe { (*HEADER.with(Cell::get)).ref_count })
        });
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        match MODE.with(Cell::get) {
            1 => {
                interpreter.refuse_host_command(if id == 1 {
                    "first original range retirement"
                } else {
                    "later original range retirement"
                });
            }
            2 if id == 1 => {
                let original = interpreter.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("range-retirement-context".to_owned(), "1".to_owned())];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original).unwrap();
            }
            _ => {}
        }
    }
    static MEMBER: obj::TclObjType = obj::TclObjType {
        name: c"originalRangeMemberControl".as_ptr(),
        free_int_rep_proc: Some(free_member),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    struct Current;
    impl Drop for Current {
        fn drop(&mut self) {
            crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        }
    }
    #[test]
    fn original_range_retirement_checks_first_cause_without_working_header_reference() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual software member-free callbacks exercise the shared storage
        // retirement owner. This is no C/Jim native callback chronology claim.
        for mode in [0, 1, 2] {
            let mut interpreter = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("tcl9.0"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let members = (1..=3)
                .map(|id| obj::alloc_typed(&MEMBER, id))
                .collect::<Vec<_>>();
            let original = obj::Owned::fresh(new_list_obj_native(
                &members,
                NativeStringProtocol::C(TclVersion::V9_0),
            ));
            let arguments = OriginalArguments::enter(&interpreter, &[original.as_ptr()]).unwrap();
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
            let _current = Current;
            MODE.with(|selected| selected.set(mode));
            FREES.with(|calls| calls.borrow_mut().clear());
            REFERENCES.with(|references| references.borrow_mut().clear());
            HEADER.with(|header| header.set(original.as_ptr()));
            let reached = native_list_command_range_using(
                original.as_ptr(),
                99,
                0,
                NativeStringProtocol::C(TclVersion::V9_0),
                &|| arguments.check_original_for(&interpreter, 0, original.as_ptr()),
                &|error| arguments.retain_refusal(error.into()),
            );
            if mode == 0 {
                assert_eq!(reached.unwrap().as_ptr(), original.as_ptr());
            } else {
                let first = match reached {
                    Err(first) => first,
                    Ok(_) => panic!("retirement refusal"),
                };
                assert_eq!(
                    first.native_execution_refusal(),
                    interpreter.native_execution_refusal().as_ref()
                );
            }
            assert_eq!(FREES.with(|calls| calls.borrow().clone()), [1, 2, 3]);
            assert_eq!(
                REFERENCES.with(|references| references.borrow().clone()),
                [1, 1, 1]
            );
            assert_eq!(interpreter.get_obj_result(), prior);
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
            MODE.with(|selected| selected.set(0));
        }
    }
}
