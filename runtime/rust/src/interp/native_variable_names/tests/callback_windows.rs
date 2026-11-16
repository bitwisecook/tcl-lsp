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

//! What a callback observes mid-operation: element tables and entries, search slots, scalar aliases and parsed name headers.

use super::*;

#[test]
fn element_table_and_var_roles_match_all_25_actual_callback_windows() {
    // Native proof: naming.variable.original-element-unset-callback-roles
    // docs/design/analysis/name-resolution-proofs/variable-original-element-unset-callback-roles.md
    use super::super::*;
    use crate::frame::NativeElementEntryObserver;
    use crate::namespace::GLOBAL;
    use std::cell::RefCell;
    use tcl_runtime_api::native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    };
    struct Windows {
        version: &'static str,
        index_k: *mut TclObj,
        index_j: *mut TclObj,
        old_k: NativeElementEntryObserver,
        old_j: NativeElementEntryObserver,
        object_table: bool,
        rows: Rc<RefCell<Vec<String>>>,
        sequence: Rc<RefCell<usize>>,
    }
    impl Windows {
        fn record(&self, interp: &Interp, phase: &str) {
            let current = interp
                .namespaces
                .borrow()
                .var_table(GLOBAL)
                .capture_array_cell(b"arr");
            let (present_k, _, defined_k, dead_k, refs_k) =
                self.old_k.observe(current.as_ref(), self.object_table);
            let (present_j, _, defined_j, dead_j, refs_j) =
                self.old_j.observe(current.as_ref(), self.object_table);
            // SAFETY: the probe's two original index owners outlive every callback.
            let (key_k, key_j) = unsafe { ((*self.index_k).ref_count, (*self.index_j).ref_count) };
            self.rows.borrow_mut().push(format!("{}|window|{phase}|{key_k}|{key_j}|{dead_k}|{dead_j}|{defined_k}|{defined_j}|{refs_k}|{refs_j}|{present_k}|{present_j}", self.version));
        }
    }
    impl NativeVariableObserver<Interp> for Windows {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            interp: &mut Interp,
            access: NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            let phase = if access.name2.is_empty() {
                "root".to_owned()
            } else {
                *self.sequence.borrow_mut() += 1;
                format!(
                    "element{}-{}",
                    *self.sequence.borrow(),
                    std::str::from_utf8(access.name2).unwrap()
                )
            };
            self.record(interp, &phase);
            Ok(())
        }
    }
    let rows = Rc::new(RefCell::new(Vec::new()));
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        let root = Owned::fresh(obj::new_string_bytes(b"arr"));
        let index_k = Owned::fresh(obj::new_string_bytes(b"k"));
        let index_j = Owned::fresh(obj::new_string_bytes(b"j"));
        let one = Owned::fresh(obj::new_string_bytes(b"ONE"));
        for index in [&index_k, &index_j] {
            let capture = interp
                .capture_original_c_parts_report(
                    root.as_ptr(),
                    Some(index.as_ptr()),
                    NativeVariableNameLookupPurpose::Write,
                )
                .unwrap()
                .unwrap();
            capture.receiver.store(one.as_ptr()).unwrap();
        }
        let array = interp
            .namespaces
            .borrow()
            .var_table(GLOBAL)
            .capture_array_cell(b"arr")
            .unwrap();
        let names = [
            Owned::fresh(obj::new_string_bytes(b"arr(k)")),
            Owned::fresh(obj::new_string_bytes(b"arr(j)")),
        ];
        let mut aliases = Vec::new();
        for name in &names {
            let mut target = interp
                .prepare_original_c_link_target(name.as_ptr(), 0)
                .unwrap()
                .unwrap();
            crate::vars::prepare_upvar_target(
                &mut interp.frames.borrow_mut(),
                &mut interp.namespaces.borrow_mut(),
                &mut target,
            )
            .unwrap();
            aliases.push(target);
        }
        let windows = Rc::new(Windows {
            version,
            index_k: index_k.as_ptr(),
            index_j: index_j.as_ptr(),
            old_k: array.observe_element_entry(b"k").unwrap(),
            old_j: array.observe_element_entry(b"j").unwrap(),
            object_table: interp
                .native_c_variable_name_protocol()
                .unwrap()
                .element_table_retains_original(),
            rows: Rc::clone(&rows),
            sequence: Rc::new(RefCell::new(0)),
        });
        for name in std::iter::once(&root).chain(names.iter()) {
            interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[NativeVariableTraceOperation::Unset],
                    windows.clone(),
                )
                .unwrap();
        }
        windows.record(&interp, "before");
        assert!(interp.var_unset(b"arr"));
        windows.record(&interp, "after");
        drop(aliases);
    }
    let expected = include_str!("../../../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.txt").lines().filter(|line| line.contains("|window|")).collect::<Vec<_>>();
    assert_eq!(rows.borrow().len(), 25);
    assert_eq!(*rows.borrow(), expected);
}

#[test]
fn element_entry_lifecycle_matches_all_70_actual_c_windows() {
    // Native proof: naming.variable.original-element-alias-recreation
    // docs/design/analysis/name-resolution-proofs/variable-original-element-alias-recreation.md
    // Native proof: naming.variable.original-element-alias-header-roles
    // docs/design/analysis/name-resolution-proofs/variable-original-element-alias-header-roles.md
    use super::super::*;
    use crate::namespace::GLOBAL;
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        for whole in [false, true] {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            let protocol = interp.native_c_variable_name_protocol().unwrap();
            let original = Owned::fresh(obj::new_string_bytes(b"arr(k)"));
            let fresh = Owned::fresh(obj::new_string_bytes(b"arr(k)"));
            let one = Owned::fresh(obj::new_string_bytes(b"ONE"));
            let two = Owned::fresh(obj::new_string_bytes(b"TWO"));
            interp
                .assign_original_named_variable(original.as_ptr(), one.as_ptr())
                .unwrap();
            let array = interp
                .namespaces
                .borrow()
                .var_table(GLOBAL)
                .capture_array_cell(b"arr")
                .unwrap();
            let observer = array.observe_element_entry(b"k").unwrap();
            let root = obj::native_variable_name::with_parsed(original.as_ptr(), |cache| {
                cache.array.as_ref().unwrap().0.as_ptr()
            })
            .unwrap();
            let key = observer.key();
            // This is the native probe's declared external key observer,
            // needed only for the separately allocated C85/86 table key.
            let _key_pin = (protocol.version() >= tcl_dialect::TclVersion::V8_5
                && protocol.version() < tcl_dialect::TclVersion::V9_0)
                .then(|| Owned::retain(key.unwrap()));
            let observe = |interp: &Interp, phase: &str, rows: &mut Vec<String>| {
                let current = interp
                    .namespaces
                    .borrow()
                    .var_table(GLOBAL)
                    .capture_array_cell(b"arr");
                let (present, same, defined, dead, refs) =
                    observer.observe(current.as_ref(), protocol.element_table_retains_original());
                // SAFETY: original parser storage or the declared observer pin owns each header.
                let (name_refs, root_refs, key_refs) = unsafe {
                    (
                        (*original.as_ptr()).ref_count,
                        (*root).ref_count,
                        key.map_or(-1, |key| (*key).ref_count),
                    )
                };
                rows.push(format!("{version}|window|{}|{phase}|{name_refs}|{root_refs}|{key_refs}|{present}|{same}|{defined}|{dead}|{refs}", if whole { "whole" } else { "element" }));
            };
            observe(&interp, "created", &mut rows);
            let mut first = interp
                .prepare_original_c_link_target(original.as_ptr(), 0)
                .unwrap()
                .unwrap();
            crate::vars::prepare_upvar_target(
                &mut interp.frames.borrow_mut(),
                &mut interp.namespaces.borrow_mut(),
                &mut first,
            )
            .unwrap();
            let mut second = interp
                .prepare_original_c_link_target(original.as_ptr(), 0)
                .unwrap()
                .unwrap();
            crate::vars::prepare_upvar_target(
                &mut interp.frames.borrow_mut(),
                &mut interp.namespaces.borrow_mut(),
                &mut second,
            )
            .unwrap();
            observe(&interp, "linked", &mut rows);
            let target: &[u8] = if whole { b"arr" } else { b"arr(k)" };
            assert!(interp.var_unset(target));
            observe(&interp, "unset-target", &mut rows);
            interp
                .assign_original_named_variable(fresh.as_ptr(), two.as_ptr())
                .unwrap();
            observe(&interp, "recreated", &mut rows);
            assert!(interp.var_unset(target));
            observe(&interp, "unset-recreated", &mut rows);
            drop(second);
            observe(&interp, "one-alias", &mut rows);
            drop(first);
            observe(&interp, "no-alias", &mut rows);
        }
    }
    let expected = include_str!("../../../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_lifecycle.txt").lines().filter(|line| line.contains("|window|") && !line.starts_with("Jim|")).collect::<Vec<_>>();
    assert_eq!(rows.len(), 70);
    assert_eq!(rows, expected);
}

#[test]
fn search_free_slots_match_all_20_actual_c_windows() {
    // Native proof: naming.variable.array-search-free-slot-retirement
    // docs/design/analysis/name-resolution-proofs/variable-array-search-free-slot-retirement.md
    use super::super::*;
    use tcl_cmd_core::native_array_search::NativeArraySearchBackend;
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        let protocol = interp.array_search_protocol().unwrap();
        let name = Owned::fresh(obj::new_string_bytes(b"a(k)"));
        let member = Owned::fresh(obj::new_string_bytes(b"v"));
        interp
            .assign_original_named_variable(name.as_ptr(), member.as_ptr())
            .unwrap();
        let array = Owned::fresh(obj::new_string_bytes(b"a"));
        let command = Owned::fresh(obj::new_string_bytes(b"array"));
        let start = Owned::fresh(obj::new_string_bytes(b"startsearch"));
        assert_eq!(
            interp.dispatch(&[command.as_ptr(), start.as_ptr(), array.as_ptr()]),
            Code::Ok
        );
        let handle = Owned::retain(interp.get_obj_result());
        let anymore = Owned::fresh(obj::new_string_bytes(b"anymore"));
        assert_eq!(
            interp.dispatch(&[
                command.as_ptr(),
                anymore.as_ptr(),
                array.as_ptr(),
                handle.as_ptr()
            ]),
            Code::Ok
        );
        let observe = |phase: &str, original: &Owned| {
            let kind = obj::obj_type_ptr(original.as_ptr());
            // SAFETY: the test owns this original header and live descriptor.
            let free = !kind.is_null() && unsafe { (*kind).free_int_rep_proc.is_some() };
            format!(
                "{version}|{phase}|{}|{}|{}",
                if obj::native_array_search_cache_in(original.as_ptr(), protocol)
                    .unwrap()
                    .is_some()
                {
                    "array search"
                } else if matches!(
                    obj::native_object_snapshot(original.as_ptr())
                        .unwrap()
                        .cache,
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
                ) {
                    "string"
                } else {
                    "none"
                },
                usize::from(free),
                usize::from(obj::has_string_rep(original.as_ptr()))
            )
        };
        rows.push(observe("converted", &handle));
        let copy = Owned::fresh(obj::duplicate(handle.as_ptr()));
        rows.push(observe("duplicate", &copy));
        assert!(interp
            .read_original_named_variable(handle.as_ptr())
            .is_err());
        rows.push(observe("missing", &handle));
        assert!(interp.read_original_named_variable(copy.as_ptr()).is_err());
        rows.push(observe("duplicate-missing", &copy));
    }
    let expected = include_str!(
        "../../../../../../rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt"
    )
    .lines()
    .filter(|line| !line.is_empty())
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 20);
    assert_eq!(rows, expected);
}

#[test]
fn scalar_alias_entries_and_array_parts_match_all_95_native_windows() {
    // Native proof: naming.variable.original-scalar-alias-entry-lifetime
    // docs/design/analysis/name-resolution-proofs/variable-original-scalar-alias-entry-lifetime.md
    // Native proof: naming.variable.original-parsed-array-child-references
    // docs/design/analysis/name-resolution-proofs/variable-original-parsed-array-child-references.md
    use crate::namespace::GLOBAL;
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        for mode in 0..2 {
            let name = Owned::fresh(obj::new_string_bytes(b"k"));
            let value = Owned::fresh(obj::new_string_bytes(b"ONE"));
            interp
                .assign_original_named_variable(name.as_ptr(), value.as_ptr())
                .unwrap();
            let before = interp
                .namespaces
                .borrow()
                .var_table(GLOBAL)
                .native_name_cell_identity(b"k")
                .unwrap();
            let observe = |interp: &Interp, phase: &str, fresh: Option<&Owned>| {
                let namespaces = interp.namespaces.borrow();
                let table = namespaces.var_table(GLOBAL);
                let found = table.native_name_cell_identity(b"k");
                assert!(matches!(
                    obj::native_object_snapshot(name.as_ptr()).unwrap().cache,
                    Cache::ParsedVariableName { .. }
                ));
                // SAFETY: these original object headers are owned throughout each window.
                let (references, fresh_references) = unsafe {
                    (
                        (*name.as_ptr()).ref_count,
                        fresh.map_or(-1, |fresh| (*fresh.as_ptr()).ref_count),
                    )
                };
                format!(
                        "{version}|alias|{mode}|{phase}|parsedVarName|{references}|{fresh_references}|{}|{}|{}",
                        usize::from(found.is_some()),
                        usize::from(found == Some(before)),
                        usize::from(table.original_native_key_is(b"k", name.as_ptr()))
                    )
            };
            rows.push(observe(&interp, "created", None));
            interp.frames.borrow_mut().push(GLOBAL);
            let target = crate::vars::link_target_at(
                &interp.frames.borrow(),
                &interp.namespaces.borrow(),
                b"k",
                None,
                0,
            )
            .unwrap();
            interp.make_upvar(target, b"a");
            rows.push(observe(&interp, "linked", None));
            assert!(interp.var_unset(b"::k"));
            rows.push(observe(&interp, "unset-target", None));
            let fresh = Owned::fresh(obj::new_string_bytes(b"k"));
            let written =
                Owned::fresh(obj::new_string_bytes(if mode == 0 { b"::k" } else { b"a" }));
            interp
                .assign_original_named_variable(written.as_ptr(), value.as_ptr())
                .unwrap();
            rows.push(observe(&interp, "recreated", Some(&fresh)));
            assert!(interp.var_unset(b"::k"));
            rows.push(observe(&interp, "unset-recreated-target", Some(&fresh)));
            assert!(!interp.var_unset(b"a"));
            rows.push(observe(&interp, "unset-alias", Some(&fresh)));
            interp.frames.borrow_mut().pop();
            rows.push(observe(&interp, "frame-popped", Some(&fresh)));
            assert!(interp.read_original_named_variable(fresh.as_ptr()).is_err());
            rows.push(observe(&interp, "fresh-read", Some(&fresh)));
        }
        let name = Owned::fresh(obj::new_string_bytes(b"arr(k)"));
        let value = Owned::fresh(obj::new_string_bytes(b"VALUE"));
        interp
            .assign_original_named_variable(name.as_ptr(), value.as_ptr())
            .unwrap();
        let observe = |phase: &str| {
            obj::native_variable_name::with_parsed(name.as_ptr(), |cache| {
                let (root, element) = cache.array.as_ref().unwrap();
                // SAFETY: cache children remain owned by the original parsed name.
                let (root_refs, element_refs) = unsafe {
                    (
                        (*root.as_ptr()).ref_count,
                        match element {
                            NativeParsedVariableElement::Object(element) => {
                                (*element.as_ptr()).ref_count
                            }
                            NativeParsedVariableElement::Bytes(_) => -1,
                        },
                    )
                };
                format!("{version}|parts|{phase}|{root_refs}|{element_refs}")
            })
            .unwrap()
        };
        rows.push(observe("stored"));
        let copy = Owned::fresh(obj::duplicate(name.as_ptr()));
        rows.push(observe("duplicated"));
        drop(copy);
        assert!(interp.var_unset(b"arr"));
        rows.push(observe("unset"));
    }
    let expected = include_str!(
        "../../../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt"
    )
    .lines()
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 95);
    assert_eq!(rows, expected);
}

pub(super) fn observe(version: &str, case: &str, phase: &str, original: &Owned) -> String {
    let snapshot = obj::native_object_snapshot(original.as_ptr()).unwrap();
    let kind = match snapshot.cache {
        Cache::None => "none",
        Cache::Numeric(_) => "int",
        Cache::List { .. } => "list",
        Cache::ByteArray { .. } => "bytearray",
        Cache::ParsedVariableName { .. } => "parsedVarName",
        other => panic!("unexpected actual original primary: {other:?}"),
    };
    let descriptor = obj::obj_type_ptr(original.as_ptr());
    // SAFETY: both the retained original header and its descriptor are live.
    let (references, free_hook) = unsafe {
        (
            (*original.as_ptr()).ref_count,
            !descriptor.is_null() && (*descriptor).free_int_rep_proc.is_some(),
        )
    };
    let hex = snapshot
        .resident
        .as_deref()
        .map_or_else(String::new, |bytes| {
            bytes.iter().map(|byte| format!("{byte:02x}")).collect()
        });
    format!(
        "{version}|{case}|{phase}|{kind}|{references}|{}|{}|{hex}",
        usize::from(free_hook),
        usize::from(snapshot.resident.is_some())
    )
}

#[test]
fn original_parsed_headers_match_all_150_actual_c_windows() {
    // Native proof: naming.variable.original-scalar-parsed-header
    // docs/design/analysis/name-resolution-proofs/variable-original-scalar-parsed-header.md
    // Native proof: naming.variable.original-counted-array-parsed-header
    // docs/design/analysis/name-resolution-proofs/variable-original-counted-array-parsed-header.md
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        let dialect = interp.native_invocation_dialect();
        let protocol = dialect.native_string_protocol().unwrap();
        for case in ["string", "int", "list", "bytearray", "array"] {
            let original = match case {
                "string" => Owned::fresh(obj::new_string_bytes(b"missing")),
                "int" => {
                    let original = Owned::fresh(obj::new_wide_int_obj(42));
                    if version == "8.4.20" {
                        obj::adopt_native_scalar_cache(
                            original.as_ptr(),
                            tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(42),
                            dialect.native_scalar_getter_protocol().unwrap(),
                        )
                        .unwrap();
                    }
                    original
                }
                "list" => {
                    let member = Owned::fresh(obj::new_string_bytes(b"missing"));
                    Owned::fresh(crate::list::new_list_obj_native(
                        &[member.as_ptr()],
                        protocol,
                    ))
                }
                "bytearray" => Owned::fresh(interp.new_native_byte_array(b"missing").unwrap()),
                "array" => Owned::fresh(obj::new_string_bytes(b"arr(k\0z)")),
                _ => unreachable!(),
            };
            rows.push(observe(version, case, "before", &original));
            assert!(interp
                .read_original_named_variable(original.as_ptr())
                .is_err());
            rows.push(observe(version, case, "missing", &original));
            let value = Owned::fresh(obj::new_string_bytes(b"VALUE"));
            interp
                .store_original_named_variable(original.as_ptr(), value.as_ptr())
                .unwrap();
            rows.push(observe(version, case, "stored", &original));
            assert_eq!(
                interp
                    .read_original_named_variable(original.as_ptr())
                    .unwrap(),
                value.as_ptr()
            );
            rows.push(observe(version, case, "read", &original));
            let copy = Owned::fresh(obj::duplicate(original.as_ptr()));
            rows.push(observe(version, case, "duplicate", &copy));
            drop(copy);
            for name in [b"missing".as_slice(), b"42".as_slice(), b"arr".as_slice()] {
                interp.var_unset(name);
            }
            assert!(interp
                .read_original_named_variable(original.as_ptr())
                .is_err());
            rows.push(observe(version, case, "after-unset-missing", &original));
        }
    }
    let expected = include_str!(
        "../../../../../../rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.txt"
    )
    .lines()
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 150);
    assert_eq!(rows, expected);
}
