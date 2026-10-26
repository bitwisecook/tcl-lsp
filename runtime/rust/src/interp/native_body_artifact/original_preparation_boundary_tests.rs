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
//! Software callback boundaries in the actual original C preparation workers.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj::TclObjType;
use std::cell::Cell;
use tcl_runtime_api::native_literal::{NativeLiteralKey, NativeLocalNameTable};
use tcl_runtime_api::native_substitution::NativeSubstitutionFlags;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static PROCEDURE_FRAME: Cell<bool> = const { Cell::new(false) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
    static LATER: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
}
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.set(0);
    }
}
fn reset(mode: u8, procedure: bool) -> Reset {
    MODE.set(mode);
    PROCEDURE_FRAME.set(procedure);
    GETTERS.set(0);
    LATER.set(0);
    FREES.set(0);
    REFS.set(-1);
    Reset
}
fn change() {
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.get() {
        1 => {
            actual.refuse_host_command("original preparation callback");
        }
        2 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            if PROCEDURE_FRAME.get() {
                frames.push(GLOBAL);
            } else {
                frames.push_namespace(GLOBAL);
            }
        }
        4 => {
            let before = actual.runtime_context();
            let mut changed = before.clone();
            changed.packages = vec![("original-preparation".to_owned(), "1".to_owned())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&before).unwrap();
        }
        _ => {}
    }
}
extern "C" fn update_source(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    change();
    unsafe {
        obj::set_native_updater_string_rep(original, b"", false);
    }
}
extern "C" fn update_first_name(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    change();
    unsafe {
        obj::set_native_updater_string_rep(original, b"prep_first_name", false);
    }
}
extern "C" fn update_later_name(original: *mut TclObj) {
    LATER.set(LATER.get() + 1);
    unsafe {
        obj::set_native_updater_string_rep(original, b"prep_later_name", false);
    }
}
extern "C" fn retire(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    if MODE.get() == 3 {
        // Real same-header type/rep ABA while the original free is reached.
        MODE.set(0);
        obj::change_type(original, &MARKER, 77);
        obj::change_type(original, &RETIRE, 0);
        MODE.set(3);
    } else {
        change();
    }
}
static SOURCE: TclObjType = TclObjType {
    name: c"originalPreparationSource".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_source),
    set_from_any_proc: None,
};
static FIRST_NAME: TclObjType = TclObjType {
    name: c"originalPreparationFirstName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_first_name),
    set_from_any_proc: None,
};
static LATER_NAME: TclObjType = TclObjType {
    name: c"originalPreparationLaterName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_later_name),
    set_from_any_proc: None,
};
static RETIRE: TclObjType = TclObjType {
    name: c"originalPreparationRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static MARKER: TclObjType = TclObjType {
    name: c"originalPreparationReplacement".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn prepare(actual: &mut Interp, original: *mut TclObj, substitution: bool) -> bool {
    if substitution {
        actual.substitute_original_c_template(
            original,
            NativeSubstitutionFlags::new(true, true, true),
        ) == Code::Ok
    } else {
        matches!(
            actual.prepare_original_c_body(original, GLOBAL, None),
            Ok(Some(_))
        )
    }
}
fn cached(original: *mut TclObj, substitution: bool) -> bool {
    if substitution {
        let descriptor = obj::obj_type_ptr(original);
        !descriptor.is_null()
            && unsafe { std::ffi::CStr::from_ptr((*descriptor).name) } == c"substcode"
    } else {
        cache(original).is_some()
    }
}
fn assert_first(actual: &Interp, mode: u8) {
    let first = actual
        .native_execution_refusal()
        .expect("full original preparation cause");
    if mode == 1 {
        assert!(
            matches!(first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref reason)
            if reason.reason == "original preparation callback")
        );
    } else {
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
        if mode == 3 {
            assert!(matches!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "changed native primary publication slot"
                    )
                )
            ));
        }
    }
}
fn profiles(substitution: bool) -> &'static [&'static str] {
    if substitution {
        &["tcl8.6", "tcl9.0", "tcl9.1"]
    } else {
        &["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
    }
}
#[test]
fn original_preparation_source_string_keeps_frame_and_currency_before_cache_publication() {
    // Actual C issuers; updater observations are software controls, not Native chronology.
    for substitution in [false, true] {
        for profile in profiles(substitution) {
            for mode in [0, 1, 2, 4] {
                let mut actual = tests::interpreter(profile);
                actual.frames.borrow_mut().push_namespace(GLOBAL);
                let _current = fixture::install(&mut actual, 0);
                let _reset = reset(mode, false);
                let source = obj::Owned::fresh(obj::alloc_typed(&SOURCE, 0));
                let prior = fixture::string(b"PRIOR");
                actual.set_result(prior.as_ptr());
                let scope =
                    native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
                let reached = prepare(&mut actual, source.as_ptr(), substitution);
                assert_eq!(GETTERS.get(), 1, "{profile}/{substitution}/{mode}");
                assert_eq!(REFS.get(), 1);
                assert_eq!(FREES.get(), 0);
                assert_eq!(unsafe { (*source.as_ptr()).ref_count }, 1);
                assert!(obj::has_string_rep(source.as_ptr()));
                assert!(obj::bytes_of(source.as_ptr()).is_empty());
                if mode == 0 {
                    assert!(reached);
                    assert!(cached(source.as_ptr(), substitution));
                    scope.currency().ensure_current_or_refuse().unwrap();
                    if substitution {
                        assert!(actual.result_bytes().is_empty());
                    } else {
                        assert_eq!(actual.result_obj(), prior.as_ptr());
                    }
                } else {
                    assert!(!reached);
                    assert!(!cached(source.as_ptr(), substitution));
                    assert_first(&actual, mode);
                    assert_eq!(actual.result_obj(), prior.as_ptr());
                    assert!(scope.currency().ensure_current_or_refuse().is_err());
                }
            }
        }
    }
}
#[test]
fn original_preparation_old_primary_retirement_checks_real_frame_and_primary_aba_before_adoption() {
    for substitution in [false, true] {
        for profile in profiles(substitution) {
            for mode in [0, 1, 2, 3] {
                let mut actual = tests::interpreter(profile);
                actual.frames.borrow_mut().push_namespace(GLOBAL);
                let _current = fixture::install(&mut actual, 0);
                let _reset = reset(mode, false);
                let source = obj::Owned::fresh(obj::alloc_typed(&RETIRE, 0));
                unsafe {
                    obj::set_native_updater_string_rep(source.as_ptr(), b"", false);
                }
                let prior = fixture::string(b"PRIOR");
                actual.set_result(prior.as_ptr());
                let scope =
                    native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
                let reached = prepare(&mut actual, source.as_ptr(), substitution);
                assert_eq!(GETTERS.get(), 0);
                assert_eq!(FREES.get(), if mode == 3 { 2 } else { 1 });
                assert_eq!(REFS.get(), 1);
                assert_eq!(unsafe { (*source.as_ptr()).ref_count }, 1);
                if mode == 0 {
                    assert!(reached);
                    assert!(cached(source.as_ptr(), substitution));
                } else {
                    assert!(!reached);
                    assert!(!cached(source.as_ptr(), substitution));
                    assert_first(&actual, mode);
                    assert_eq!(actual.result_obj(), prior.as_ptr());
                    assert!(scope.currency().ensure_current_or_refuse().is_err());
                    if mode == 3 {
                        assert!(core::ptr::eq(obj::obj_type_ptr(source.as_ptr()), &RETIRE));
                    }
                }
                MODE.set(0);
            }
        }
    }
}
#[test]
fn original_preparation_borrowed_name_table_stops_at_actual_first_updater_without_extra_native_hold(
) {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 4] {
            let mut actual = tests::interpreter(profile);
            let protocol = actual.source_string_protocol().unwrap();
            let names = [
                Some(tcl_runtime_api::NameBytes::from(
                    b"prep_first_name".as_slice(),
                )),
                Some(tcl_runtime_api::NameBytes::from(
                    b"prep_later_name".as_slice(),
                )),
            ];
            for name in names.iter().flatten() {
                let key = NativeLiteralKey {
                    protocol,
                    namespace: None,
                    original: name.as_bytes().to_vec(),
                };
                assert_eq!(
                    actual
                        .native_literal_world
                        .borrow()
                        .key_values(&key)
                        .count(),
                    0
                );
            }
            let table = NativeLocalNameTable::create(
                &actual.native_literal_world,
                &names,
                protocol,
                |owner: &obj::Owned| Ok::<_, ValueError>(obj::bytes_of(owner.as_ptr())),
                |bytes| {
                    Ok::<_, ValueError>(obj::Owned::fresh(obj::alloc_typed(
                        if bytes == b"prep_first_name" {
                            &FIRST_NAME
                        } else {
                            &LATER_NAME
                        },
                        0,
                    )))
                },
            )
            .unwrap();
            {
                let mut frames = actual.frames.borrow_mut();
                frames.push(GLOBAL);
                frames.install_native_local_name_table(Rc::clone(&table));
            }
            let _current = fixture::install(&mut actual, 0);
            let _reset = reset(mode, true);
            let source = fixture::string(b"");
            let prior = fixture::string(b"PRIOR");
            actual.set_result(prior.as_ptr());
            let scope = native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
            let reached = actual.prepare_original_c_body(source.as_ptr(), GLOBAL, None);
            assert_eq!(GETTERS.get(), 1);
            assert_eq!(REFS.get(), 2);
            assert_eq!(actual.result_obj(), prior.as_ptr());
            let original_name = table.names[0].as_ref().unwrap();
            assert_eq!(unsafe { (*original_name.as_ptr()).ref_count }, 2);
            assert_eq!(obj::bytes_of(original_name.as_ptr()), b"prep_first_name");
            if mode == 0 {
                let artifact = reached.unwrap().expect("actual empty compiled body");
                let retained = artifact
                    ._borrowed_table
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .unwrap();
                assert!(Rc::ptr_eq(&retained, &table));
                assert_eq!(LATER.get(), 1);
                scope.currency().ensure_current_or_refuse().unwrap();
            } else {
                assert_eq!(reached.err(), Some(Code::Error));
                assert_eq!(LATER.get(), 0);
                assert!(cache(source.as_ptr()).is_none());
                assert_first(&actual, mode);
                assert!(scope.currency().ensure_current_or_refuse().is_err());
            }
        }
    }
}
