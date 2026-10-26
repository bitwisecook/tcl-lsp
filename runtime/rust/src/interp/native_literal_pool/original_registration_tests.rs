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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual literal-table getters and reached registration retirement.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::Cell;
thread_local! {
    static HELD: RefCell<Option<Rc<NativeRuntimeLiteralArray>>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
}
fn actual() -> Interp {
    unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone()
}
extern "C" fn retire(original: *mut obj::TclObj) {
    FREES.set(FREES.get() + 1);
    assert_eq!(unsafe { (*original).ref_count }, 0);
    assert!(actual().native_literal_world.try_borrow_mut().is_ok());
}
extern "C" fn update(original: *mut obj::TclObj) {
    CALLS.set(CALLS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    assert!(actual().native_literal_world.try_borrow_mut().is_ok());
    match MODE.get() {
        1 => {
            let retired = HELD.with(|held| held.borrow_mut().take());
            drop(retired);
            assert!(!obj::allocation_is_live(original));
            return;
        }
        2 => {
            let source = obj::Owned::fresh(obj::new_string_bytes(b"nested source"));
            let nested = actual()
                .create_native_literal_array(
                    source.as_ptr(),
                    &[NativeRuntimeLiteral::RegisteredBytes {
                        bytes: b"nested independent key".to_vec(),
                        namespace: None,
                    }],
                )
                .unwrap();
            drop(nested);
        }
        3 => {
            actual().refuse_host_command("original literal getter");
        }
        4 => {
            let interpreter = actual();
            let mut frames = interpreter.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push_namespace(crate::namespace::GLOBAL);
        }
        _ => {}
    }
    unsafe {
        obj::set_native_updater_string_rep(original, b"registered original", false);
    }
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalLiteralRegistration".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
#[test]
fn original_literal_registration_reentry_preserves_header_lifetime_and_native_roles() {
    // Real updater/free callbacks are software controls, not external C chronology.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..=4 {
            let mut interpreter = fixture::native(profile);
            interpreter
                .frames
                .borrow_mut()
                .push_namespace(crate::namespace::GLOBAL);
            let _installed = fixture::install(&mut interpreter, 0);
            let scope =
                super::super::native_operation_currency::NativeOperationScope::enter(&interpreter)
                    .unwrap();
            MODE.set(0);
            CALLS.set(0);
            FREES.set(0);
            REFS.set(-1);
            let source = obj::Owned::fresh(obj::new_string_bytes(b"source"));
            let declarations = [NativeRuntimeLiteral::RegisteredBytes {
                bytes: b"registered original".to_vec(),
                namespace: None,
            }];
            let first = interpreter
                .create_native_literal_array(source.as_ptr(), &declarations)
                .unwrap();
            let registration = first.registrations[0].unwrap();
            let original = first.original(0).unwrap();
            let lease = obj::NativeObjectLifetime::retain(original);
            assert_eq!(unsafe { (*original).ref_count }, 2);
            obj::change_type(original, &ORIGINAL, 0);
            obj::invalidate_string(original);
            HELD.with(|held| *held.borrow_mut() = Some(first));
            MODE.set(mode);
            let reached = interpreter.create_native_literal_array(source.as_ptr(), &declarations);
            assert_eq!(CALLS.get(), 1, "{profile}/{mode}");
            assert_eq!(REFS.get(), 2);
            if mode == 0 {
                let second = reached.unwrap();
                assert_eq!(second.original(0), Some(original));
                assert_eq!(unsafe { (*original).ref_count }, 3);
                assert_eq!(
                    interpreter
                        .native_literal_world
                        .borrow()
                        .registration_owners(registration),
                    Some(2)
                );
                drop(second);
                assert_eq!(unsafe { (*original).ref_count }, 2);
                scope.currency().ensure_current_or_refuse().unwrap();
            } else {
                assert!(reached.is_err());
                let first = interpreter
                    .native_execution_refusal()
                    .expect("full original registration failure");
                if mode == 3 {
                    assert!(
                        matches!(first,tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref reason) if reason.reason=="original literal getter")
                    );
                } else {
                    assert!(matches!(
                        first,
                        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
                    ));
                }
                assert!(scope.currency().ensure_current_or_refuse().is_err());
                if mode == 1 {
                    assert_eq!(FREES.get(), 1);
                    assert!(!obj::allocation_is_live(lease.as_ptr()));
                    assert_eq!(
                        interpreter
                            .native_literal_world
                            .borrow()
                            .registration_owners(registration),
                        None
                    );
                } else {
                    assert_eq!(FREES.get(), 0);
                    assert!(obj::allocation_is_live(lease.as_ptr()));
                    assert_eq!(unsafe { (*original).ref_count }, 2);
                    assert_eq!(
                        interpreter
                            .native_literal_world
                            .borrow()
                            .registration_owners(registration),
                        Some(1)
                    );
                }
            }
            MODE.set(0);
            let retired = HELD.with(|held| held.borrow_mut().take());
            drop(retired);
            assert_eq!(FREES.get(), 1);
            assert!(!obj::allocation_is_live(lease.as_ptr()));
            drop(lease);
            interpreter.frames.borrow_mut().pop();
        }
    }
}

thread_local! { static LOCAL_CALLS: Cell<usize> = const { Cell::new(0) }; }
extern "C" fn local_update(original: *mut obj::TclObj) {
    LOCAL_CALLS.set(LOCAL_CALLS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    let interpreter = actual();
    let procedure = interpreter
        .frames
        .borrow()
        .current_c_procedure()
        .cloned()
        .unwrap();
    assert!(procedure.native_local_names.try_borrow_mut().is_ok());
    assert!(interpreter.native_literal_world.try_borrow_mut().is_ok());
    if MODE.get() == 3 {
        interpreter
            .clone()
            .refuse_host_command("original local-name getter");
    }
    unsafe {
        obj::set_native_updater_string_rep(original, b"literal_local_first", false);
    }
}
static LOCAL: obj::TclObjType = obj::TclObjType {
    name: c"originalLiteralLocalName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(local_update),
    set_from_any_proc: None,
};
#[test]
fn original_local_name_factory_getter_runs_outside_procedure_and_world_borrows() {
    // Authentic compiled layout; callback/reference checks remain software-only.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 3] {
            let mut interpreter = fixture::native(profile);
            assert_eq!(interpreter.eval_str(b"proc literal_factory {} {set literal_local_first 1; set literal_local_later 2}"), super::super::Code::Ok);
            assert_eq!(
                interpreter.eval_str(b"literal_factory"),
                super::super::Code::Ok
            );
            let procedure = interpreter.proc_def(b"literal_factory").unwrap();
            let artifact = interpreter
                .prepare_original_c_body(
                    procedure.body.as_ptr(),
                    procedure.namespace(),
                    Some(&procedure),
                )
                .unwrap()
                .unwrap();
            let layout = artifact.compiled_local_layout().unwrap();
            let retired = procedure.native_local_names.borrow_mut().take();
            drop(retired);
            let protocol = interpreter.source_string_protocol().unwrap();
            let table = tcl_runtime_api::native_literal::NativeLocalNameTable::create(
                &interpreter.native_literal_world,
                &layout.names,
                protocol,
                |owner: &obj::Owned| Ok::<_, ValueError>(obj::bytes_of(owner.as_ptr())),
                |bytes| Ok::<_, ValueError>(obj::Owned::fresh(obj::new_string_bytes(bytes))),
            )
            .unwrap();
            let original = table
                .names
                .iter()
                .flatten()
                .find(|name| obj::bytes_of(name.as_ptr()) == b"literal_local_first")
                .unwrap()
                .as_ptr();
            obj::change_type(original, &LOCAL, 0);
            obj::invalidate_string(original);
            let refs = unsafe { (*original).ref_count };
            {
                let mut frames = interpreter.frames.borrow_mut();
                frames.push(crate::namespace::GLOBAL);
                frames.retain_c_procedure(Rc::clone(&procedure));
                frames.install_native_compiled_local_layout(layout).unwrap();
            }
            let _installed = fixture::install(&mut interpreter, 0);
            let scope =
                super::super::native_operation_currency::NativeOperationScope::enter(&interpreter)
                    .unwrap();
            LOCAL_CALLS.set(0);
            REFS.set(-1);
            MODE.set(mode);
            let reached = interpreter.refresh_native_local_name_table();
            assert_eq!(LOCAL_CALLS.get(), 1);
            assert_eq!(REFS.get(), refs);
            if mode == 0 {
                reached.unwrap();
                assert!(procedure.native_local_names.borrow().is_some());
                assert!(interpreter
                    .frames
                    .borrow()
                    .native_local_name_table()
                    .is_some());
                scope.currency().ensure_current_or_refuse().unwrap();
            } else {
                assert!(reached.is_err());
                assert!(procedure.native_local_names.borrow().is_none());
                assert!(interpreter
                    .frames
                    .borrow()
                    .native_local_name_table()
                    .is_none());
                assert!(
                    matches!(interpreter.native_execution_refusal().unwrap(),tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref reason) if reason.reason=="original local-name getter")
                );
                assert_eq!(unsafe { (*original).ref_count }, refs);
            }
            MODE.set(0);
            interpreter.frames.borrow_mut().pop();
        }
    }
}
