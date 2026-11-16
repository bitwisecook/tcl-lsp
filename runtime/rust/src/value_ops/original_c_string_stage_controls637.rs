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
use std::cell::Cell;
thread_local! {
    static TARGET: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static INDEX_CALLS: Cell<usize> = const { Cell::new(0) };
}
struct Current;
impl Drop for Current {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
}
static REPLACED: TclObjType = TclObjType {
    name: c"originalStringSelectedBackingControl".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
extern "C" fn index_update(original: *mut TclObj) {
    INDEX_CALLS.with(|calls| calls.set(calls.get() + 1));
    let subject = TARGET.with(Cell::get);
    if !subject.is_null() {
        REFS.with(|refs| refs.set(unsafe { (*subject).ref_count }));
        obj::change_type(subject, &REPLACED, 0);
        unsafe { obj::set_native_updater_string_rep(subject, b"XY", false) };
    }
    unsafe { obj::set_native_updater_string_rep(original, b"0", false) };
}
static INDEX: TclObjType = TclObjType {
    name: c"originalStringIndexStageControl".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(index_update),
    set_from_any_proc: None,
};
extern "C" fn subject_update(original: *mut TclObj) {
    unsafe { obj::set_native_updater_string_rep(original, b"A\xc3\xa9Z", false) };
}
extern "C" fn subject_free(original: *mut TclObj) {
    REFS.with(|refs| refs.set(unsafe { (*original).ref_count }));
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    actual.refuse_host_command("original String primary retirement");
}
static SUBJECT: TclObjType = TclObjType {
    name: c"originalStringSubjectStageControl".as_ptr(),
    free_int_rep_proc: Some(subject_free),
    dup_int_rep_proc: None,
    update_string_proc: Some(subject_update),
    set_from_any_proc: None,
};
fn actual(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
#[test]
fn original_c_binary_pointer_and_reread_stages_use_actual_header_and_no_protective_reference() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // naming.string.original-counted-index-range-storage-and-results
    // docs/design/analysis/name-resolution-proofs/string-original-counted-index-range-storage-and-results.md
    // Real software extension updater mutates the selected subject during
    // its original index getter. Native635 public rows do not observe this
    // callback; pinned command branches bound the captured/reread schedule.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut ops = actual(profile);
        let subject = obj::Owned::fresh(crate::bytearray::new_byte_array(
            b"ABC",
            ops.native_invocation_dialect()
                .byte_array_string_recipe(None)
                .unwrap(),
        ));
        let operand = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
        TARGET.with(|target| target.set(subject.as_ptr()));
        INDEX_CALLS.with(|calls| calls.set(0));
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut ops);
        let _current = Current;
        let reached = tcl_cmd_core::string::index(
            &mut ops,
            &subject.as_ptr(),
            &operand.as_ptr(),
            tcl_syntax::number::NumberSyntax::Tcl90,
        );
        assert_eq!(INDEX_CALLS.with(Cell::get), 1);
        assert_eq!(REFS.with(Cell::get), 1);
        if profile == "tcl8.4" {
            let error = reached.unwrap_err();
            assert!(matches!(
                error.native_execution_refusal(),
                Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "retired original C binary storage"
                    )
                ))
            ));
            assert_eq!(
                error.native_execution_refusal(),
                ops.native_execution_refusal().as_ref()
            );
            assert!(core::ptr::eq(
                obj::obj_type_ptr(subject.as_ptr()),
                &REPLACED
            ));
        } else {
            let result = obj::Owned::fresh(reached.unwrap());
            assert_eq!(
                ops.index_operand_bytes(&result.as_ptr()).unwrap().as_ref(),
                b"X"
            );
            if profile == "tcl8.5" {
                assert!(crate::bytearray::native_cache_snapshot(subject.as_ptr()).is_some());
            } else {
                assert!(matches!(
                    obj::native_object_snapshot(subject.as_ptr()).unwrap().cache,
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::String {
                        num_chars: Some(2),
                        ..
                    }
                ));
            }
        }
        assert_eq!(unsafe { (*subject.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*operand.as_ptr()).ref_count }, 1);
        TARGET.with(|target| target.set(core::ptr::null_mut()));
    }
}
#[test]
fn original_count_free_callback_preserves_first_cause_before_any_index_getter_or_string_cache_adoption(
) {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual original extension free callback, not native adversarial evidence.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        let mut ops = actual(profile);
        let subject = obj::Owned::fresh(obj::alloc_typed(&SUBJECT, 0));
        let operand = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
        INDEX_CALLS.with(|calls| calls.set(0));
        TARGET.with(|target| target.set(core::ptr::null_mut()));
        ops.set_result_bytes(b"PRIOR");
        let prior = ops.get_obj_result();
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut ops);
        let _current = Current;
        let error = tcl_cmd_core::string::index(
            &mut ops,
            &subject.as_ptr(),
            &operand.as_ptr(),
            tcl_syntax::number::NumberSyntax::Tcl90,
        )
        .unwrap_err();
        assert_eq!(
            error.native_execution_refusal(),
            ops.native_execution_refusal().as_ref()
        );
        assert_eq!(REFS.with(Cell::get), 1);
        assert_eq!(INDEX_CALLS.with(Cell::get), 0);
        assert_eq!(ops.get_obj_result(), prior);
        assert!(!matches!(
            obj::native_object_snapshot(subject.as_ptr()).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
        ));
    }
}
#[test]
fn original_primary_receipt_refuses_scalar_change_and_restore() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software epoch control; no native private-epoch observation.
    let value = obj::Owned::fresh(obj::alloc_typed(&REPLACED, 7));
    let receipt = obj::NativePrimaryReceipt::capture(value.as_ptr()).unwrap();
    obj::change_type(value.as_ptr(), &REPLACED, 8);
    obj::change_type(value.as_ptr(), &REPLACED, 7);
    assert!(receipt.check(value.as_ptr()).is_err());
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
}
