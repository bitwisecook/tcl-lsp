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

//! Genuine software callback/currentness controls; not native636 chronology.
use crate::{
    interp::Interp,
    list,
    obj::{self, TclObj, TclObjType},
};
use std::cell::Cell;
use tcl_cmd_core::{index::NativeIndexOps, native_option_index::OptionCommand};
thread_local! {
    static PATH: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static LATER: Cell<usize> = const { Cell::new(0) };
    static CALLER_REFS: Cell<obj::TclSize> = const { Cell::new(0) };
}
extern "C" fn replace_path(original: *mut TclObj) {
    // SAFETY: actual extension callback on the selected live original operand.
    CALLER_REFS.set(unsafe { (*original).ref_count });
    PATH.with(|path| obj::change_type(path.get(), &obj::TCL_INT_TYPE, 7));
    // SAFETY: this original updater publishes its own resident result.
    unsafe {
        obj::set_native_updater_string_rep(original, b"0", true);
    }
}
extern "C" fn later(original: *mut TclObj) {
    LATER.with(|count| count.set(count.get() + 1));
    // SAFETY: the actual selected extension owns this live header's spelling.
    unsafe {
        obj::set_native_updater_string_rep(original, b"1", true);
    }
}
static FIRST: TclObjType = TclObjType {
    name: c"actual-option-first".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(replace_path),
    set_from_any_proc: None,
};
static LAST: TclObjType = TclObjType {
    name: c"actual-option-later".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later),
    set_from_any_proc: None,
};
#[test]
fn original_option_backing_withdrawal_stops_later_coordinate_without_protective_native_hold() {


    // Real local extension updater withdraws the original path while both old
    // children retain independent observer refs. The native636 processes did
    // not execute this callback; they establish original argument/cache roles.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut ops = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(profile),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let protocol = ops
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let first = obj::Owned::fresh(obj::alloc_typed(&FIRST, 0));
        let last = obj::Owned::fresh(obj::alloc_typed(&LAST, 0));
        let path = obj::Owned::fresh(list::new_list_obj_native(
            &[first.as_ptr(), last.as_ptr()],
            protocol,
        ));
        PATH.set(path.as_ptr());
        LATER.set(0);
        CALLER_REFS.set(0);
        let prior = obj::Owned::fresh(obj::new_string_bytes(b"PRIOR"));
        ops.set_result(prior.as_ptr());
        let failure = ops
            .original_c_option_path(&path.as_ptr(), OptionCommand::Sort, true)
            .unwrap_err();
        assert!(failure.native_execution_refusal().is_some());
        assert_eq!(
            ops.native_execution_refusal().as_ref(),
            failure.native_execution_refusal()
        );
        assert_eq!(LATER.get(), 0);
        assert_eq!(CALLER_REFS.get(), 2);
        assert_eq!(ops.get_obj_result(), prior.as_ptr());
        assert!(core::ptr::eq(
            obj::obj_type_ptr(path.as_ptr()),
            &obj::TCL_INT_TYPE
        ));
        assert_eq!(obj::internal_rep(path.as_ptr()), 7);
        // SAFETY: explicit observer owners and actual path constructor holds.
        assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*last.as_ptr()).ref_count }, 1);
        PATH.set(core::ptr::null_mut());
    }
}
