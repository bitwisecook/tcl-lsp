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
use std::cell::{Cell, RefCell};
thread_local! {
 static ACTUAL: RefCell<Option<Interp>> = const { RefCell::new(None) };
 static LATER: Cell<usize> = const { Cell::new(0) };
 static FREED: Cell<usize> = const { Cell::new(0) };
 static REFS: Cell<obj::TclSize> = const { Cell::new(0) };
}
extern "C" fn refuse(original: *mut TclObj) {
    // SAFETY: real updater selected on this live independently owned header.
    REFS.set(unsafe { (*original).ref_count });
    ACTUAL.with(|actual| {
        actual
            .borrow_mut()
            .as_mut()
            .unwrap()
            .refuse_host_command("original search/sort String callback")
    });
    // SAFETY: reached updater still publishes its own counted original spelling.
    unsafe {
        obj::set_native_updater_string_rep(original, b"A", true);
    }
}
extern "C" fn later(original: *mut TclObj) {
    LATER.set(LATER.get() + 1);
    // SAFETY: actual updater owns this live header's spelling.
    unsafe {
        obj::set_native_updater_string_rep(original, b"B", true);
    }
}
extern "C" fn release(_original: *mut TclObj) {
    FREED.set(FREED.get() + 1);
    ACTUAL.with(|actual| {
        actual
            .borrow_mut()
            .as_mut()
            .unwrap()
            .refuse_host_command("later original search/sort operand release")
    });
}
static FIRST: TclObjType = TclObjType {
    name: c"actual-search-sort-first".as_ptr(),
    free_int_rep_proc: Some(release),
    dup_int_rep_proc: None,
    update_string_proc: Some(refuse),
    set_from_any_proc: None,
};
static LAST: TclObjType = TclObjType {
    name: c"actual-search-sort-later".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later),
    set_from_any_proc: None,
};
#[test]
fn checked_original_search_sort_key_and_pattern_refusals_keep_first_cause_and_real_references() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Genuine local Runtime C extension updater/refusal and operand-release controls,
    // separate from native644 empty-input outcomes. Those processes did not run
    // callbacks or prove software reference/currentness chronology.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for operation in 0..3 {
            let mut ops = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            ACTUAL.set(Some(ops.clone()));
            LATER.set(0);
            FREED.set(0);
            REFS.set(0);
            let protocol = ops
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let first = obj::Owned::fresh(obj::alloc_typed(&FIRST, 0));
            let last = obj::Owned::fresh(obj::alloc_typed(&LAST, 0));
            let plain = obj::Owned::fresh(obj::new_string_bytes(b"A"));
            let members = if operation == 2 {
                vec![last.as_ptr(), plain.as_ptr()]
            } else {
                vec![first.as_ptr(), last.as_ptr()]
            };
            let container = obj::Owned::fresh(list::new_list_obj_native(&members, protocol));
            let name = obj::Owned::fresh(obj::new_string_bytes(if operation == 0 {
                b"lsort"
            } else {
                b"lsearch"
            }));
            let pattern = if operation == 2 {
                first.as_ptr()
            } else {
                plain.as_ptr()
            };
            let mut argv = vec![name.as_ptr(), container.as_ptr()];
            if operation != 0 {
                argv.push(pattern);
            }
            let prior = obj::Owned::fresh(obj::new_string_bytes(b"PRIOR"));
            ops.set_result(prior.as_ptr());
            let code = if operation == 0 {
                super::lsort(&mut ops, &argv)
            } else {
                super::lsearch(&mut ops, &argv)
            };
            assert_eq!(code, crate::interp::Code::Error);
            let cause = ops.native_execution_refusal().unwrap();
            assert_eq!(ops.get_obj_result(), prior.as_ptr());
            assert_eq!(LATER.get(), 0);
            assert_eq!(
                REFS.get(),
                if operation == 2 {
                    1
                } else if profile == "tcl8.4" && operation == 0 {
                    3
                } else {
                    2
                }
            );
            // SAFETY: actual observerref1 plus containingList hold, no temporary Native+1.
            assert_eq!(
                unsafe { (*first.as_ptr()).ref_count },
                if operation == 2 { 1 } else { 2 }
            );
            drop(container);
            drop(first);
            assert_eq!(FREED.get(), 1);
            assert_eq!(ops.native_execution_refusal(), Some(cause));
            ACTUAL.set(None);
        }
    }
}

#[test]
fn original_numeric_primitives_preserve_first_refusal_before_keys_and_mandatory_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Real software extension + selected primitive owners; native645/647 contain
    // no callbacks. Getter counts are actual local software roles, not donated
    // EvalObjv/private provider references. Jim numeric search is unsupported.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for operation in 0..3 {
            if profile == "jim" && operation != 0 {
                continue;
            }
            let mut ops = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            ACTUAL.set(Some(ops.clone()));
            LATER.set(0);
            FREED.set(0);
            REFS.set(0);
            let protocol = ops
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let first = obj::Owned::fresh(obj::alloc_typed(&FIRST, 0));
            let last = obj::Owned::fresh(obj::alloc_typed(&LAST, 0));
            let plain = obj::Owned::fresh(obj::new_string_bytes(b"1"));
            // The first failed primitive's updater returns a String but its
            // full original Host refusal outranks any subsequent Guest parsing.
            let members = if operation == 2 {
                vec![last.as_ptr(), plain.as_ptr()]
            } else {
                vec![first.as_ptr(), last.as_ptr()]
            };
            let container = obj::Owned::fresh(list::new_list_obj_native(&members, protocol));
            let name = obj::Owned::fresh(obj::new_string_bytes(if operation == 0 {
                b"lsort"
            } else {
                b"lsearch"
            }));
            let integer = obj::Owned::fresh(obj::new_string_bytes(b"-integer"));
            let exact = obj::Owned::fresh(obj::new_string_bytes(b"-exact"));
            let mut argv = vec![name.as_ptr(), integer.as_ptr()];
            if operation != 0 {
                argv.push(exact.as_ptr());
            }
            argv.push(container.as_ptr());
            if operation != 0 {
                argv.push(if operation == 2 {
                    first.as_ptr()
                } else {
                    plain.as_ptr()
                });
            }
            let prior = obj::Owned::fresh(obj::new_string_bytes(b"PRIOR"));
            ops.set_result(prior.as_ptr());
            let code = if operation == 0 {
                super::lsort(&mut ops, &argv)
            } else {
                super::lsearch(&mut ops, &argv)
            };
            assert_eq!(code, crate::interp::Code::Error);
            let cause = ops.native_execution_refusal().unwrap();
            assert_eq!(ops.get_obj_result(), prior.as_ptr());
            assert_eq!(LATER.get(), 0);
            assert_eq!(
                REFS.get(),
                if operation == 2 {
                    1
                } else if profile == "tcl8.4" && operation == 0 {
                    3
                } else {
                    2
                }
            );
            drop(container);
            drop(first);
            assert_eq!(FREED.get(), 1);
            assert_eq!(ops.native_execution_refusal(), Some(cause));
            ACTUAL.set(None);
        }
    }
}
