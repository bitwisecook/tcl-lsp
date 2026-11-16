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
#[cfg(debug_assertions)]
use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation;
#[cfg(debug_assertions)]
use tcl_runtime_api::native_variable_trace::{NativeVariableObserver, NativeVariableTraceAccess};

thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static ELEMENT: Cell<bool> = const { Cell::new(false) };
    static EXPECTED: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static SAW_NEW: Cell<bool> = const { Cell::new(false) };
    static SAW_OLD: Cell<bool> = const { Cell::new(false) };
    static INCOMING_REFS: Cell<isize> = const { Cell::new(0) };
    static RELEASED: Cell<bool> = const { Cell::new(false) };
    static WRITES: Cell<usize> = const { Cell::new(0) };
}
#[cfg(debug_assertions)]
fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
#[cfg(debug_assertions)]
struct Current(*mut Interp);
#[cfg(debug_assertions)]
impl Current {
    #[cfg(debug_assertions)]
    fn enter(interp: &mut Interp) -> Self {
        let old = crate::codegen_abi::current_interp();
        crate::codegen_abi::tcl_runtime_set_current_interp(interp);
        Self(old)
    }
}
#[cfg(debug_assertions)]
impl Drop for Current {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(self.0);
    }
}
#[cfg(debug_assertions)]
extern "C" fn retire(original: *mut TclObj) {
    CALLS.with(|value| value.set(value.get() + 1));
    let Some(mut interp) = (unsafe { crate::codegen_abi::current_interp().as_ref() }).cloned()
    else {
        return;
    };
    let available =
        interp.frames.try_borrow_mut().is_ok() && interp.namespaces.try_borrow_mut().is_ok();
    RELEASED.with(|value| value.set(available));
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        let selected = interp
            .namespaces()
            .var_table(crate::namespace::GLOBAL)
            .published_scalar_header(b"target");
        SAW_OLD.with(|value| value.set(selected == Some(original)));
        assert_eq!(unsafe { (*original).ref_count }, 0);
        INCOMING_REFS.with(|value| {
            value.set(unsafe { (*EXPECTED.with(Cell::get)).ref_count });
        });
    } else {
        let selected = if ELEMENT.with(Cell::get) {
            interp.var_get_elem(b"arr", b"k")
        } else {
            interp.var_get(b"target")
        };
        SAW_NEW.with(|value| value.set(selected == Some(EXPECTED.with(Cell::get))));
    }
    match MODE.with(Cell::get) {
        1 => {
            interp.refuse_host_command("first original assignment retirement");
        }
        2 => {
            let actual = interp.runtime_context();
            let mut changed = actual.clone();
            changed.packages = vec![("assignment-retirement".to_owned(), "1.0".to_owned())];
            interp.pin_context(&changed).unwrap();
            interp.pin_context(&actual).unwrap();
        }
        _ => {}
    }
}
#[cfg(debug_assertions)]
static RETIRE: obj::TclObjType = obj::TclObjType {
    name: c"originalAssignmentRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
#[cfg(debug_assertions)]
struct Write;
#[cfg(debug_assertions)]
impl NativeVariableObserver<Interp> for Write {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _interp: &mut Interp,
        _access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        WRITES.with(|value| value.set(value.get() + 1));
        Ok(())
    }
}
#[cfg(debug_assertions)]
fn reset(mode: u8, element: bool, value: *mut TclObj) {
    MODE.with(|selected| selected.set(mode));
    ELEMENT.with(|selected| selected.set(element));
    EXPECTED.with(|selected| selected.set(value));
    CALLS.with(|selected| selected.set(0));
    SAW_NEW.with(|selected| selected.set(false));
    SAW_OLD.with(|selected| selected.set(false));
    INCOMING_REFS.with(|selected| selected.set(0));
    RELEASED.with(|selected| selected.set(false));
    WRITES.with(|selected| selected.set(0));
}

#[cfg(debug_assertions)]
#[test]
fn original_c_assignment_releases_old_value_before_trace_with_cell_borrows_released() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual receiver callbacks and ownership, without external refcount/order equivalence.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for element in [false, true] {
            for mode in 0..=2 {
                let mut interp = native(profile);
                let name = Owned::fresh(obj::new_string_bytes(if element {
                    b"arr(k)"
                } else {
                    b"target"
                }));
                let old = Owned::fresh(obj::alloc_typed(&RETIRE, 0));
                unsafe { obj::set_native_updater_string_rep(old.as_ptr(), b"OLD", false) };
                interp
                    .assign_original_named_variable(name.as_ptr(), old.as_ptr())
                    .unwrap();
                drop(old);
                interp
                    .add_native_variable_observer(
                        name.as_ptr(),
                        &[NativeVariableTraceOperation::Write],
                        Rc::new(Write),
                    )
                    .unwrap();
                let next = Owned::fresh(obj::new_string_bytes(b"NEXT\0VALUE"));
                interp.set_result_bytes(b"PRIOR");
                let prior = interp.get_obj_result();
                let refs = unsafe { (*next.as_ptr()).ref_count };
                reset(mode, element, next.as_ptr());
                let current = Current::enter(&mut interp);
                let assigned = interp.assign_original_named_variable(name.as_ptr(), next.as_ptr());
                assert_eq!(assigned.is_ok(), mode == 0, "{profile}/{element}/{mode}");
                assert_eq!(CALLS.with(Cell::get), 1);
                assert!(RELEASED.with(Cell::get));
                assert!(SAW_NEW.with(Cell::get));
                assert_eq!(WRITES.with(Cell::get), usize::from(mode == 0));
                assert_eq!(unsafe { (*next.as_ptr()).ref_count }, refs + 1);
                assert_eq!(interp.get_obj_result(), prior);
                if mode == 0 {
                    assert!(!interp.host_refusal_pending());
                } else {
                    let cause = interp.native_execution_refusal().unwrap();
                    if mode == 1 {
                        let tcl_runtime_api::NativeExecutionError::HostCommandRefusal(host) = cause
                        else {
                            panic!("actual original Host command cause");
                        };
                        assert_eq!(host.reason, "first original assignment retirement");
                    }
                    assert_eq!(assigned, Err(Code::Error));
                }
                drop(current);
                assert_eq!(interp.entered_native_operation_count(), 0);
            }
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_jim_assignment_updates_actual_cached_cell_before_retirement_callback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Jim_SetVariable keeps the old VarVal pointer during its decrement.
    // A refused software continuation cannot perform the later assignment.
    for fresh_name in [false, true] {
        for mode in 0..=2 {
            let mut interp = native("jim");
            let original = Owned::fresh(obj::new_string_bytes(b"target"));
            let old = Owned::fresh(obj::alloc_typed(&RETIRE, 0));
            unsafe { obj::set_native_updater_string_rep(old.as_ptr(), b"OLD", false) };
            interp
                .assign_original_named_variable(original.as_ptr(), old.as_ptr())
                .unwrap();
            drop(old);
            let selected = if fresh_name {
                Owned::fresh(obj::new_string_bytes(b"target"))
            } else {
                Owned::retain(original.as_ptr())
            };
            let next = Owned::fresh(obj::new_string_bytes(b"NEXT"));
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            reset(mode, false, next.as_ptr());
            let current = Current::enter(&mut interp);
            let assigned = interp.assign_original_named_variable(selected.as_ptr(), next.as_ptr());
            assert_eq!(assigned.is_ok(), mode == 0, "{fresh_name}/{mode}");
            assert_eq!(CALLS.with(Cell::get), 1);
            assert!(SAW_OLD.with(Cell::get));
            assert_eq!(INCOMING_REFS.with(Cell::get), 2);
            assert!(RELEASED.with(Cell::get));
            let published = interp
                .namespaces()
                .var_table(crate::namespace::GLOBAL)
                .published_scalar_header(b"target");
            assert_eq!(published, (mode == 0).then_some(next.as_ptr()));
            assert_eq!(interp.get_obj_result(), prior);
            assert_eq!(
                unsafe { (*next.as_ptr()).ref_count },
                if mode == 0 { 2 } else { 1 }
            );
            assert_eq!(interp.host_refusal_pending(), mode != 0);
            if mode == 0 {
                assert_eq!(interp.var_get(b"target"), Some(next.as_ptr()));
            } else {
                assert_eq!(assigned, Err(Code::Error));
                let cause = interp.native_execution_refusal().unwrap();
                if mode == 1 {
                    assert!(
                        matches!(cause, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(host)
                            if host.reason == "first original assignment retirement")
                    );
                }
            }
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}
