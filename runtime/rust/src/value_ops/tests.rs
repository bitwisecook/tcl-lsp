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
use std::cell::{Cell, RefCell};
#[cfg(have_tommath)]
use tcl_cmd_core::regex::NativeRegexSource;
thread_local! {
    static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static REFUSE: Cell<u8> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
}
fn actual() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
struct Current;
impl Drop for Current {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
}
fn refuse_reached(stage: u8, original: *mut TclObj) {
    REFS.with(|refs| refs.set(unsafe { (*original).ref_count }));
    let selected = REFUSE.with(Cell::get);
    if selected == stage || selected == stage + 10 {
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        if selected == stage {
            interpreter.refuse_host_command("original Index reached callback");
        } else {
            let original_context = interpreter.runtime_context();
            let mut changed = original_context.clone();
            changed.packages = vec![(
                "original-index-context-control".to_owned(),
                "1.0".to_owned(),
            )];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original_context).unwrap();
        }
    }
}
extern "C" fn index_updater(original: *mut TclObj) {
    EVENTS.with(|events| events.borrow_mut().push("index-getter"));
    refuse_reached(1, original);
    unsafe { obj::set_native_updater_string_rep(original, b"end", false) };
}
extern "C" fn index_free(original: *mut TclObj) {
    EVENTS.with(|events| events.borrow_mut().push("index-free"));
    refuse_reached(2, original);
}
extern "C" fn list_updater(original: *mut TclObj) {
    EVENTS.with(|events| events.borrow_mut().push("container-getter"));
    unsafe { obj::set_native_updater_string_rep(original, b"a b c", false) };
}
static INDEX: TclObjType = TclObjType {
    name: c"indexOwnerControl".as_ptr(),
    free_int_rep_proc: Some(index_free),
    dup_int_rep_proc: None,
    update_string_proc: Some(index_updater),
    set_from_any_proc: None,
};
static CONTAINER: TclObjType = TclObjType {
    name: c"indexContainerControl".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(list_updater),
    set_from_any_proc: None,
};
#[cfg(have_tommath)]
#[test]
fn original_jim_container_and_regex_indices_share_one_original_expression_cache() {
    // naming.jim.original-index-conversion-and-result-object
    // docs/design/analysis/name-resolution-proofs/jim-original-index-conversion-and-result-object.md
    let mut interp = actual();
    let original = obj::Owned::fresh(obj::new_string_bytes(b"2*3"));
    assert_eq!(
        tcl_cmd_core::index::resolve_value(&mut interp, &original.as_ptr(), 10).unwrap(),
        6
    );
    assert!(core::ptr::eq(
        obj::obj_type_ptr(original.as_ptr()),
        &JIM_INDEX_TYPE
    ));
    assert_eq!(interp.jim_regex_index(&original.as_ptr()).unwrap(), 6);
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
    let relative = obj::Owned::fresh(obj::new_string_bytes(b"end-2\0tail"));
    assert_eq!(
        tcl_cmd_core::index::resolve_value(&mut interp, &relative.as_ptr(), 10).unwrap(),
        7
    );
    assert_eq!(
        tcl_cmd_core::index::resolve_value(&mut interp, &relative.as_ptr(), 4).unwrap(),
        1
    );
    assert_eq!(obj::bytes_of(relative.as_ptr()), b"end-2\0tail");
    assert_eq!(unsafe { (*relative.as_ptr()).ref_count }, 1);
}
#[test]
fn original_jim_index_integer_bypass_and_bad_string_preserve_primary_and_error_code_stage() {
    // naming.jim.original-index-conversion-and-result-object
    // docs/design/analysis/name-resolution-proofs/jim-original-index-conversion-and-result-object.md
    // Native case 8 observes failed String->Int; second call clamp is source/API control.
    let mut interp = actual();
    let original = obj::Owned::fresh(obj::new_string_bytes(b"2147483648"));
    let error = native_jim_index::original_index(&mut interp, &original.as_ptr()).unwrap_err();
    assert_eq!(
        error.error_code_update(),
        &tcl_cmd_core::CmdErrorCodeUpdate::Unchanged
    );
    assert!(error.native_execution_refusal().is_none());
    assert!(core::ptr::eq(
        obj::obj_type_ptr(original.as_ptr()),
        &obj::TCL_INT_TYPE
    ));
    assert_eq!(
        native_jim_index::original_index(&mut interp, &original.as_ptr()).unwrap(),
        i32::MAX
    );
    assert!(core::ptr::eq(
        obj::obj_type_ptr(original.as_ptr()),
        &obj::TCL_INT_TYPE
    ));
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
}
#[test]
fn original_jim_index_getter_and_free_callbacks_preserve_full_cause_without_protective_references()
{
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual extension callbacks are software controls; no native hostile-callback chronology claim.
    for stage in [1, 2] {
        let mut interp = actual();
        let original = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
        interp.set_result_bytes(b"PRIOR");
        let prior = interp.get_obj_result();
        EVENTS.with(|events| events.borrow_mut().clear());
        REFUSE.with(|selected| selected.set(stage));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
        let _current = Current;
        let error = native_jim_index::original_index(&mut interp, &original.as_ptr()).unwrap_err();
        let first = interp.native_execution_refusal().unwrap();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(REFS.with(Cell::get), 1);
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        assert!(!core::ptr::eq(
            obj::obj_type_ptr(original.as_ptr()),
            &JIM_INDEX_TYPE
        ));
        assert_eq!(interp.get_obj_result(), prior);
        let expected = if stage == 1 {
            vec!["index-getter"]
        } else {
            vec!["index-getter", "index-free"]
        };
        assert_eq!(EVENTS.with(|events| events.borrow().clone()), expected);
        REFUSE.with(|selected| selected.set(0));
    }
}
#[test]
fn original_jim_list_indices_precede_list_getters_but_string_range_gets_its_subject_first() {
    // naming.jim.original-index-conversion-and-result-object
    // docs/design/analysis/name-resolution-proofs/jim-original-index-conversion-and-result-object.md
    // Pinned-source order is checked with actual Runtime extension headers.
    // The original 17-case native probe measures conversion, not this callback program.
    for mode in 0..4 {
        let mut interp = actual();
        let index = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
        let container = obj::Owned::fresh(obj::alloc_typed(&CONTAINER, 0));
        EVENTS.with(|events| events.borrow_mut().clear());
        REFUSE.with(|selected| selected.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
        let _current = Current;
        let result = match mode {
            0 => {
                tcl_cmd_core::list::lindex_flat(&mut interp, &container.as_ptr(), &[index.as_ptr()])
            }
            1 => tcl_cmd_core::list::lrange(
                &mut interp,
                &container.as_ptr(),
                &index.as_ptr(),
                &index.as_ptr(),
            ),
            2 => tcl_cmd_core::string::index(
                &mut interp,
                &container.as_ptr(),
                &index.as_ptr(),
                tcl_syntax::number::NumberSyntax::Tcl90,
            ),
            _ => tcl_cmd_core::string::range(
                &mut interp,
                &container.as_ptr(),
                &index.as_ptr(),
                &index.as_ptr(),
                tcl_syntax::number::NumberSyntax::Tcl90,
            ),
        }
        .unwrap();
        interp.set_result(result);
        let expected = if mode == 3 {
            vec!["container-getter", "index-getter", "index-free"]
        } else {
            vec!["index-getter", "index-free", "container-getter"]
        };
        assert_eq!(
            EVENTS.with(|events| events.borrow().clone()),
            expected,
            "mode {mode}"
        );
        assert_eq!(REFS.with(Cell::get), 1);
        assert_eq!(unsafe { (*index.as_ptr()).ref_count }, 1);
    }
}
#[test]
fn original_jim_index_lifetime_lease_keeps_allocation_without_reviving_a_native_header() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = actual();
    let original = obj::Owned::fresh(obj::new_string_bytes(b"end"));
    let pointer = original.as_ptr();
    assert_eq!(
        native_jim_index::original_index(&mut interp, &pointer).unwrap(),
        -1
    );
    let allocation = obj::NativeObjectLifetime::retain(pointer);
    drop(original);
    assert!(!obj::allocation_is_live(pointer));
    let error = native_jim_index::original_index(&mut interp, &pointer).unwrap_err();
    assert!(error.native_execution_refusal().is_some());
    drop(allocation);
}
#[cfg(debug_assertions)]
#[test]
fn original_indexed_string_count_refuses_before_later_indices_and_cache_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine Runtime extension callbacks test this checked API. Native
    // Jim command/cache observations remain the separate pinned17 rows.
    for stage in [1, 2, 11, 12] {
        let mut interp = actual();
        let subject = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
        let first = obj::Owned::fresh(obj::new_string_bytes(b"0"));
        let last = obj::Owned::fresh(obj::new_string_bytes(b"end"));
        interp.set_result_bytes(b"PRIOR");
        let prior = interp.get_obj_result();
        EVENTS.with(|events| events.borrow_mut().clear());
        REFUSE.with(|selected| selected.set(stage));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
        let _current = Current;
        let failure = tcl_cmd_core::string::range(
            &mut interp,
            &subject.as_ptr(),
            &first.as_ptr(),
            &last.as_ptr(),
            tcl_syntax::number::NumberSyntax::Tcl90,
        )
        .unwrap_err();
        let original = interp.native_execution_refusal().unwrap();
        assert_eq!(failure.native_execution_refusal(), Some(&original));
        assert_eq!(interp.get_obj_result(), prior);
        assert!(!core::ptr::eq(
            obj::obj_type_ptr(first.as_ptr()),
            &JIM_INDEX_TYPE
        ));
        assert!(!core::ptr::eq(
            obj::obj_type_ptr(last.as_ptr()),
            &JIM_INDEX_TYPE
        ));
        assert!(!matches!(
            obj::native_object_snapshot(subject.as_ptr()).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
        ));
        assert_eq!(unsafe { (*subject.as_ptr()).ref_count }, 1);
        assert_eq!(REFS.with(Cell::get), 1);
        REFUSE.with(|selected| selected.set(0));
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_indexed_list_conversion_refuses_before_backing_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Direct shared List API, original foreign updater/free, same actual
    // entered currency. No native adversarial-callback source claim.
    for stage in [1, 2, 11, 12] {
        let mut interp = actual();
        let subject = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
        let index = obj::Owned::fresh(obj::new_string_bytes(b"end"));
        interp.set_result_bytes(b"PRIOR");
        let prior = interp.get_obj_result();
        EVENTS.with(|events| events.borrow_mut().clear());
        REFUSE.with(|selected| selected.set(stage));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
        let _current = Current;
        let failure =
            tcl_cmd_core::list::lindex_flat(&mut interp, &subject.as_ptr(), &[index.as_ptr()])
                .unwrap_err();
        let original = interp.native_execution_refusal().unwrap();
        assert_eq!(failure.native_execution_refusal(), Some(&original));
        assert_eq!(interp.get_obj_result(), prior);
        assert!(crate::list::native_list_backing(subject.as_ptr()).is_none());
        assert!(core::ptr::eq(
            obj::obj_type_ptr(index.as_ptr()),
            &JIM_INDEX_TYPE
        ));
        assert_eq!(unsafe { (*subject.as_ptr()).ref_count }, 1);
        REFUSE.with(|selected| selected.set(0));
    }
}
