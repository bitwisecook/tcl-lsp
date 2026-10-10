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

fn native_list_selected_range(
    original: *mut TclObj,
    selection: NativeListRangeSelection,
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, ValueError> {
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native compiled List range release",
        ))?;
    if selection.is_immediate()
        && version >= TclVersion::V9_0
        && obj::has_canonical_empty_string(original)
    {
        return Ok(obj::Owned::retain(original));
    }
    if version >= TclVersion::V9_0 && crate::native_arithseries::is_series(original) {
        return Err(ValueError::CommandProtocolUnavailable(
            "native abstract List slice provider",
        ));
    }
    prepare_native_list_mutation(original, protocol)?;
    let shared = obj::is_shared(original);
    // SAFETY: the original getter installed this live header. No result hold
    // is acquired before sampling the original header's sharing.
    let list = unsafe { list_mut(original) };
    if version >= TclVersion::V9_0 && !shared {
        list.elems.collect_unreferenced();
    }
    let action = selection.action(
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
    )?;
    apply_native_list_range(original, list, action, shared, version, protocol)
}

fn apply_native_list_range(
    original: *mut TclObj,
    list: &mut TclList,
    action: NativeListRangeAction,
    shared: bool,
    version: TclVersion,
    protocol: NativeStringProtocol,
) -> Result<obj::Owned, ValueError> {
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
                *list = backing;
                obj::invalidate_string(original);
                Ok(obj::Owned::retain(original))
            }
        }
        NativeListRangeAction::FreshMembers(range) => {
            let elements = list.elems.elements()[range].to_vec();
            if version >= TclVersion::V9_0 && !shared {
                for &element in &elements {
                    unsafe { obj::incr_ref_count(element) };
                }
                list.elems = NativeListStorage::new(elements);
                list.canonical = Rc::new(Cell::new(false));
                obj::invalidate_string(original);
                Ok(obj::Owned::retain(original))
            } else {
                Ok(obj::Owned::fresh(new_list_obj_native(&elements, protocol)))
            }
        }
        NativeListRangeAction::Whole => {
            if shared {
                native_list_copy(original, protocol)
            } else {
                obj::invalidate_string(original);
                Ok(obj::Owned::retain(original))
            }
        }
        NativeListRangeAction::Window { range, span } => {
            if shared {
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
                list.elems.select_window(range, span);
                if version >= TclVersion::V9_0 && !span {
                    list.canonical.set(false);
                }
                obj::invalidate_string(original);
                Ok(obj::Owned::retain(original))
            }
        }
    }
}

impl NativeListStorage {
    pub(super) fn replace_native(
        &mut self,
        first: usize,
        delete: usize,
        insert: &[*mut TclObj],
        version: TclVersion,
    ) -> Result<bool, ValueError> {
        use tcl_cmd_core::native_list_storage::{NativeListReplaceStorage, replace_layout};
        self.checked_generation()?;
        let first = first.min(self.len());
        let delete = delete.min(self.len() - first);
        if insert.is_empty() && delete == 0 {
            return Ok(false);
        }
        if version >= TclVersion::V9_0
            && insert.is_empty()
            && (first == 0 || first + delete == self.len())
        {
            return self.delete_end_range(first, delete, version);
        }
        self.collect_unreferenced();
        let layout = if version >= TclVersion::V9_0 {
            Some(replace_layout(
                NativeListReplaceStorage {
                    length: self.len(),
                    used: self.backing.elements.borrow().0.len(),
                    allocated: self.backing.capacity.get().ok_or(
                        ValueError::CommandProtocolUnavailable("native List allocation extent"),
                    )?,
                    first_used: self.backing.first_used.get(),
                    window_start: self
                        .window
                        .as_ref()
                        .map_or(self.backing.first_used.get(), |window| window.start),
                    has_span: self.span,
                    store_shared: self.native_is_shared(),
                },
                first,
                delete,
                insert.len(),
            ))
        } else {
            None
        };
        if let Some(layout) = layout.filter(|layout| layout.shared_prepend) {
            let start = self
                .window
                .as_ref()
                .map_or(self.backing.first_used.get(), |window| window.start);
            let end = start + self.len();
            for &value in insert {
                unsafe { obj::incr_ref_count(value) };
            }
            self.backing
                .elements
                .borrow_mut()
                .0
                .splice(0..0, insert.iter().copied());
            self.backing.first_used.set(layout.first_used);
            self.window = Some(start - insert.len()..end);
            self.span = true;
            return Ok(false);
        }
        if let Some(layout) = layout.filter(|layout| layout.new_store) {
            let members = self.elements();
            let elements = members[..first]
                .iter()
                .chain(insert)
                .chain(&members[first + delete..])
                .copied()
                .collect::<Vec<_>>();
            for &value in &elements {
                unsafe { obj::incr_ref_count(value) };
            }
            drop(members);
            *self = Self::new(elements);
            self.set_capacity(layout.allocated);
            self.backing.first_used.set(layout.first_used);
            self.span = layout.span;
            return Ok(true);
        }
        if self.native_is_shared() {
            *self = self.copied_header();
        }
        self.collect_unreferenced();
        for &value in insert {
            unsafe { obj::incr_ref_count(value) };
        }
        let retired = self
            .backing
            .elements
            .borrow_mut()
            .0
            .splice(first..first + delete, insert.iter().copied())
            .collect::<Vec<_>>();
        for value in retired {
            unsafe { obj::decr_ref_count(value) };
        }
        self.note_mutation();
        if let Some(layout) = layout {
            self.set_capacity(layout.allocated);
            self.backing.first_used.set(layout.first_used);
            self.span = layout.span;
        } else {
            self.backing.capacity.set(None);
            self.backing.first_used.set(0);
            self.span = false;
        }
        self.window = None;
        Ok(version >= TclVersion::V9_0 && !(delete == 0 && first + insert.len() == self.len()))
    }

    fn delete_end_range(
        &mut self,
        first: usize,
        delete: usize,
        version: TclVersion,
    ) -> Result<bool, ValueError> {
        use tcl_cmd_core::native_list_storage::range_storage_action;
        self.collect_unreferenced();
        let range = if first == 0 {
            delete..self.len()
        } else {
            0..first
        };
        if range.is_empty() {
            *self = Self::new(Vec::new());
            return Ok(true);
        }
        let action = range_storage_action(
            version,
            range,
            NativeListRangeStorage {
                length: self.len(),
                header_shared: false,
                store_shared: self.native_is_shared(),
                has_span: self.span,
                used: self.backing.elements.borrow().0.len(),
                allocated: self.backing.capacity.get(),
                string: tcl_cmd_core::native_list_storage::NativeListStringState::Nonempty,
            },
        )?;
        match action {
            NativeListRangeAction::Window { range, span } => {
                self.select_window(range, span);
                Ok(!span)
            }
            NativeListRangeAction::FreshMembers(range) => {
                let elements = self.elements()[range].to_vec();
                for &value in &elements {
                    unsafe { obj::incr_ref_count(value) };
                }
                *self = Self::new(elements);
                Ok(true)
            }
            NativeListRangeAction::Whole => Ok(false),
            _ => unreachable!("nonempty private List range"),
        }
    }
}

/// Replace members on the physically selected header. All original index and
/// value operands must have been evaluated before this mutation door.
pub(crate) fn replace_prepared_native_elements(
    original: *mut TclObj,
    first: usize,
    delete: usize,
    insert: &[*mut TclObj],
    protocol: NativeStringProtocol,
    invalidate: bool,
) -> Result<(), ValueError> {
    let version = protocol
        .tcl_version()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native List replacement release",
        ))?;
    prepare_native_list_mutation(original, protocol)?;
    let list = unsafe { list_mut(original) };
    let previous = list.canonical.get();
    let backing = Rc::clone(&list.elems.backing);
    let reset = list.elems.replace_native(first, delete, insert, version)?;
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
    Ok(())
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
