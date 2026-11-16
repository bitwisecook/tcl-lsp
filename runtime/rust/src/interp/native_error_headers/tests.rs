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

use super::*;
use tcl_dialect::TclVersion;
#[test]
fn private_headers_save_without_duplicate_member_references() {
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let recipe = tcl_registry::InvocationDialect::for_version(version)
            .native_error_objects_protocol()
            .unwrap();
        let key = obj::Owned::fresh(obj::new_string_bytes(b"-custom"));
        let value = obj::Owned::fresh(obj::new_string_bytes(b"original\0\xff"));
        let key_ptr = key.as_ptr();
        let value_ptr = value.as_ptr();
        let options = NativeReturnOptions::new(
            vec![ReturnOptionPair {
                key,
                value,
                key_bytes: b"-custom".to_vec(),
            }],
            Some(recipe),
        )
        .unwrap();
        let header = options.header().unwrap();
        // SAFETY: these objects are borrowed from the live private Dict owner.
        unsafe {
            assert_eq!((*header).ref_count, 1);
            assert_eq!((*key_ptr).ref_count, 1);
            assert_eq!((*value_ptr).ref_count, 1);
        }
        let saved = options.clone();
        unsafe {
            assert_eq!((*header).ref_count, 2);
            assert_eq!((*value_ptr).ref_count, 1);
        }
        drop(options);
        unsafe {
            assert_eq!((*header).ref_count, 1);
            assert_eq!((*value_ptr).ref_count, 1);
        }
        let snapshot = saved.pairs();
        assert_eq!(snapshot[0].key.as_ptr(), key_ptr);
        assert_eq!(snapshot[0].value.as_ptr(), value_ptr);
        drop(snapshot);
        if !recipe.has_error_stack() {
            continue;
        }
        let mut stack = NativeErrorStack::default();
        stack.configure(Some(recipe));
        let header = stack.header.as_ref().unwrap().as_ptr();
        assert!(obj::obj_type_ptr(header).is_null());
        stack.begin_inner(b"INNER".to_vec(), b"original".to_vec());
        assert_eq!(stack.header.as_ref().unwrap().as_ptr(), header);
        let parts = crate::list::list_elements_native_checked(header, recipe.strings()).unwrap();
        let child = parts[1];
        let saved = stack.clone();
        unsafe {
            assert_eq!((*header).ref_count, 2);
            assert_eq!((*child).ref_count, 1);
        }
        stack.mark_reset();
        stack.begin_inner(b"INNER".to_vec(), b"callback".to_vec());
        assert_ne!(stack.header.as_ref().unwrap().as_ptr(), header);
        unsafe {
            assert_eq!((*header).ref_count, 1);
            assert_eq!((*child).ref_count, 1);
        }
        stack = saved;
        assert_eq!(stack.header.as_ref().unwrap().as_ptr(), header);
        assert_eq!(stack.entries()[1], b"original");
    }
}
#[test]
fn explicit_stack_preserves_original_member_primary_without_string_getters() {
    let recipe = tcl_registry::InvocationDialect::for_version(TclVersion::V9_0)
        .native_error_objects_protocol()
        .unwrap();
    let member = obj::Owned::fresh(crate::list::new_list_obj_native(
        &[obj::new_string_bytes(b"original")],
        recipe.strings(),
    ));
    let tag = obj::Owned::fresh(obj::new_string_bytes(b"INNER"));
    let supplied = obj::Owned::fresh(crate::list::new_list_obj_native(
        &[tag.as_ptr(), member.as_ptr()],
        recipe.strings(),
    ));
    let mut stack = NativeErrorStack::default();
    stack.configure(Some(recipe));
    let identity = stack.header.as_ref().unwrap().as_ptr();
    stack.adopt_original(supplied.as_ptr()).unwrap();
    assert_eq!(stack.header.as_ref().unwrap().as_ptr(), identity);
    assert!(!obj::has_string_rep(member.as_ptr()));
    let parts = crate::list::list_elements_native_checked(identity, recipe.strings()).unwrap();
    assert_eq!(parts[1], member.as_ptr());
    let getter = stack.value();
    assert_eq!(getter.as_ptr(), identity);
    stack
        .adopt(vec![b"INNER".to_vec(), b"second".to_vec()])
        .unwrap();
    assert_ne!(stack.header.as_ref().unwrap().as_ptr(), identity);
    let parts =
        crate::list::list_elements_native_checked(getter.as_ptr(), recipe.strings()).unwrap();
    assert_eq!(parts[1], member.as_ptr());
}
