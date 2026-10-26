// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for original C API collection ownership and reached stages.

use super::*;
use crate::interp::WeakInterp;
use core::{mem::size_of, ptr};
use std::{cell::Cell, rc::Rc};
use tcl_dialect::TclVersion;
use tcl_runtime_api::NativeExecutionError;

fn leak_free(body: impl FnOnce()) {
    crate::counters::reset();
    body();
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}
fn native(version: TclVersion) -> Interp {
    let environment = match version {
        TclVersion::V8_4 => "tcl8.4",
        TclVersion::V8_6 => "tcl8.6",
        TclVersion::V9_0 => "tcl9.0",
        _ => panic!("collection controls require their measured C8.4, C8.6 or C9.0 core"),
    };
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(environment),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn stale() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "stale entered native operation",
        ),
    )
}
struct Callback {
    interpreter: WeakInterp,
    updates: Rc<Cell<usize>>,
    frees: Rc<Cell<usize>>,
    action: u8,
    on_free: bool,
    bytes: &'static [u8],
}
fn reached(state: &Callback) {
    let mut interpreter = state.interpreter.upgrade().unwrap();
    match state.action {
        1 => {
            interpreter.refuse_host_command("first original collection callback");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("collection-callback".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}
extern "C" fn update(value: *mut TclObj) {
    // SAFETY: the actual extension descriptor owns this record until retirement.
    let state = unsafe { &*(obj::internal_rep(value) as usize as *const Callback) };
    state.updates.set(state.updates.get() + 1);
    if !state.on_free {
        reached(state);
    }
    // SAFETY: preserve the reached original updater's exact resident String.
    unsafe { obj::set_native_updater_string_rep(value, state.bytes, false) };
}
extern "C" fn free(value: *mut TclObj) {
    // SAFETY: this original extension owns one boxed record.
    let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut Callback) };
    state.frees.set(state.frees.get() + 1);
    if state.on_free {
        reached(&state);
    }
}
static CALLBACK: TclObjType = TclObjType {
    name: c"originalCollectionCallback".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn callback(
    interpreter: &Interp,
    action: u8,
    on_free: bool,
    resident: bool,
    bytes: &'static [u8],
) -> (obj::Owned, Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let updates = Rc::new(Cell::new(0));
    let frees = Rc::new(Cell::new(0));
    let state = Box::new(Callback {
        interpreter: interpreter.downgrade(),
        updates: updates.clone(),
        frees: frees.clone(),
        action,
        on_free,
        bytes,
    });
    let value = obj::alloc_typed(&CALLBACK, Box::into_raw(state) as usize as u64);
    if resident {
        unsafe { obj::set_string_rep(value, bytes) };
    }
    (obj::Owned::fresh(value), updates, frees)
}
fn assert_first(interpreter: &Interp, action: u8) {
    match action {
        1 => {
            let Some(NativeExecutionError::HostCommandRefusal(first)) =
                interpreter.native_execution_refusal()
            else {
                panic!("the original Host cause was not retained");
            };
            assert_eq!(first.reason, "first original collection callback");
        }
        2 => assert_eq!(interpreter.native_execution_refusal(), Some(stale())),
        _ => assert!(interpreter.native_execution_refusal().is_none()),
    }
}
fn table() -> [*const c_char; 3] {
    [c"alpha".as_ptr(), c"beta".as_ptr(), ptr::null()]
}

#[test]
fn original_collection_rc0_views_keep_header_and_backing_copy_on_write_separate() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual software header and backing ownership; no external callback timing claim.
    leak_free(|| {
        let mut interpreter = native(TclVersion::V8_6);
        let raw = obj::new_string_bytes(b"alpha beta");
        assert_eq!(unsafe { (*raw).ref_count }, 0);
        let (mut count, mut window) = (-19, ptr::null_mut());
        assert_eq!(
            unsafe { get_elements(&mut interpreter, raw, &raw mut count, &raw mut window) },
            TCL_OK
        );
        assert_eq!(unsafe { (*raw).ref_count }, 0);
        assert_eq!(count, 2);
        assert!(!window.is_null());
        assert_eq!(unsafe { obj::bytes_of(*window) }, b"alpha");
        let original = obj::Owned::fresh(raw);
        let copy = obj::Owned::fresh(obj::duplicate(raw));
        let member = obj::Owned::fresh(obj::new_string_bytes(b"gamma"));
        assert_eq!(
            unsafe { append(&mut interpreter, copy.as_ptr(), member.as_ptr()) },
            TCL_OK
        );
        assert_eq!(unsafe { (*copy.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
        assert_eq!(unsafe { obj::bytes_of(*window.add(1)) }, b"beta");
        assert_eq!(list::list_elements(raw).unwrap().len(), 2);
        assert_eq!(list::list_elements(copy.as_ptr()).unwrap().len(), 3);
        drop(copy);
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 1);
        drop(original);
        drop(member);
    });
}

fn error_code_keeps_unreached_original_updater(action: u8) {
    leak_free(|| {
        let mut interpreter = native(TclVersion::V9_0);
        interpreter.set_result_bytes(b"SEEDED RESULT");
        interpreter.set_c_error_code(b"STAMP BEFORE");
        let result = interpreter.get_obj_result();
        let context = interpreter.runtime_context();
        let (value, updates, frees) = callback(&interpreter, action, false, false, b"alpha beta");
        // Tcl_SetObjErrorCode retains the header without requesting its String.
        // C8.6.18 tclResult.c:1103 and C9.0.4 tclResult.c:535.
        unsafe { set_error_code(&mut interpreter, value.as_ptr()) };
        assert_eq!(updates.get(), 0);
        assert_eq!(frees.get(), 0);
        assert!(!obj::has_string_rep(value.as_ptr()));
        assert!(ptr::eq(obj::obj_type_ptr(value.as_ptr()), &CALLBACK));
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
        assert_eq!(interpreter.get_obj_result(), result);
        assert_eq!(interpreter.runtime_context(), context);
        assert!(interpreter.native_execution_refusal().is_none());
        interpreter.set_c_error_code(b"REPLACED");
        assert_eq!(updates.get(), 0);
        assert_eq!(frees.get(), 0);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        drop(value);
        assert_eq!(frees.get(), 1);
    });
}

#[test]
fn original_collection_updaters_keep_reached_string_and_stop_before_cache_or_outputs() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Each real updater mutates/restores the original context or raises the first Host cause.
    for action in [1, 2] {
        for worker in 0..5 {
            leak_free(|| {
                let mut interpreter = native(TclVersion::V9_0);
                interpreter.set_result_bytes(b"SEEDED RESULT");
                interpreter.set_c_error_code(b"STAMP BEFORE");
                let result = interpreter.get_obj_result();
                let context = interpreter.runtime_context();
                let (value, updates, frees) =
                    callback(&interpreter, action, false, false, b"alpha beta");
                let member = obj::Owned::fresh(obj::new_string_bytes(b"member"));
                let names = table();
                let mut count: TclSize = -71;
                let mut window = ptr::dangling_mut::<*mut TclObj>();
                let mut index = -73_i32;
                let status = unsafe {
                    match worker {
                        0 => get_elements(
                            &mut interpreter,
                            value.as_ptr(),
                            &raw mut count,
                            &raw mut window,
                        ),
                        1 => length(&mut interpreter, value.as_ptr(), &raw mut count),
                        2 => append(&mut interpreter, value.as_ptr(), member.as_ptr()),
                        3 => get_index(
                            &mut interpreter,
                            value.as_ptr(),
                            names.as_ptr().cast(),
                            size_of::<*const c_char>() as TclSize,
                            c"option".as_ptr(),
                            0,
                            (&raw mut index).cast(),
                        ),
                        4 => {
                            wrong_arguments(
                                &mut interpreter,
                                1,
                                [value.as_ptr()].as_ptr(),
                                c"tail".as_ptr(),
                            );
                            TCL_ERROR
                        }
                        _ => unreachable!("five getter workers"),
                    }
                };
                assert_eq!(status, TCL_ERROR);
                assert_eq!(updates.get(), 1);
                assert_eq!(frees.get(), 0);
                assert!(ptr::eq(obj::obj_type_ptr(value.as_ptr()), &CALLBACK));
                assert_eq!(obj::bytes_of(value.as_ptr()), b"alpha beta");
                assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
                assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 1);
                assert_eq!(count, -71);
                assert_eq!(window, ptr::dangling_mut());
                assert_eq!(index, -73);
                assert_eq!(interpreter.get_obj_result(), result);
                assert_eq!(interpreter.snapshot_error().code_bytes(), b"STAMP BEFORE");
                assert_eq!(interpreter.runtime_context(), context);
                assert_first(&interpreter, action);
                drop(value);
                assert_eq!(frees.get(), 1);
            });
        }
        error_code_keeps_unreached_original_updater(action);
    }
}

#[test]
fn original_collection_retirement_runs_once_before_later_cache_publication() {
    // naming.embedding.original-host-publication-and-fact-transport

    // A real old freeIntRep runs outside interpreter borrows. Reached retirement is retained.
    for action in [1, 2] {
        for index_worker in [false, true] {
            leak_free(|| {
                let mut interpreter = native(TclVersion::V9_0);
                let (value, updates, frees) = callback(&interpreter, action, true, true, b"alpha");
                let mut count: TclSize = -31;
                let mut index = -37_i32;
                let names = table();
                let status = if index_worker {
                    unsafe {
                        get_index(
                            &mut interpreter,
                            value.as_ptr(),
                            names.as_ptr().cast(),
                            size_of::<*const c_char>() as TclSize,
                            c"option".as_ptr(),
                            0,
                            (&raw mut index).cast(),
                        )
                    }
                } else {
                    unsafe { length(&mut interpreter, value.as_ptr(), &raw mut count) }
                };
                assert_eq!(status, TCL_ERROR);
                assert_eq!(updates.get(), 0);
                assert_eq!(frees.get(), 1);
                assert!(obj::obj_type_ptr(value.as_ptr()).is_null());
                assert_eq!(obj::bytes_of(value.as_ptr()), b"alpha");
                assert_eq!(count, -31);
                assert_eq!(index, -37);
                assert_first(&interpreter, action);
                drop(value);
                assert_eq!(frees.get(), 1);
            });
        }
    }
}

#[test]
fn original_collection_empty_and_abstract_length_keep_their_actual_cache_purposes() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Selected software native protocol and genuine constructor, independent of execution admission.
    leak_free(|| {
        for version in [TclVersion::V8_4, TclVersion::V8_6, TclVersion::V9_0] {
            let mut interpreter = native(version);
            let value = obj::Owned::fresh(obj::new_string_bytes(b""));
            let mut count: TclSize = -1;
            let mut window = ptr::dangling_mut::<*mut TclObj>();
            assert_eq!(
                unsafe {
                    get_elements(
                        &mut interpreter,
                        value.as_ptr(),
                        &raw mut count,
                        &raw mut window,
                    )
                },
                TCL_OK
            );
            assert_eq!(count, 0);
            assert_eq!(
                ptr::eq(obj::obj_type_ptr(value.as_ptr()), &list::TCL_LIST_TYPE),
                version == TclVersion::V8_4
            );
            if version != TclVersion::V8_4 {
                assert!(window.is_null());
            }
            assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        }
        let mut interpreter = native(TclVersion::V9_0);
        let protocol = interpreter
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let plan = match tcl_cmd_core::lseq::decode(&[b"1", b"3", b"1"]) {
            Ok(plan) => plan,
            Err(_) => panic!("original arithmetic-series operands must decode"),
        };
        let series = tcl_cmd_core::lseq::prepare_series(&plan).unwrap().unwrap();
        let value =
            obj::Owned::fresh(crate::native_arithseries::new_series(series, protocol).unwrap());
        let mut count: TclSize = -1;
        assert_eq!(
            unsafe { length(&mut interpreter, value.as_ptr(), &raw mut count) },
            TCL_OK
        );
        assert_eq!(count, 3);
        assert!(!crate::native_arithseries::has_element_cache(
            value.as_ptr()
        ));
        assert!(!obj::has_string_rep(value.as_ptr()));
        let mut window = ptr::null_mut();
        assert_eq!(
            unsafe {
                get_elements(
                    &mut interpreter,
                    value.as_ptr(),
                    &raw mut count,
                    &raw mut window,
                )
            },
            TCL_OK
        );
        assert!(crate::native_arithseries::has_element_cache(value.as_ptr()));
        assert!(!obj::has_string_rep(value.as_ptr()));
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        assert_eq!(
            window,
            crate::native_arithseries::cached_elements_raw(value.as_ptr(), protocol)
                .unwrap()
                .0
        );
        for position in 0..3 {
            assert_eq!(unsafe { (*(*window.add(position))).ref_count }, 1);
        }
    });
}

#[test]
fn original_collection_null_missing_and_foreign_issuers_cannot_supply_a_view() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Header/backing protocol alone does not supply an interpreter/current-operation issuer.
    leak_free(|| {
        let value = obj::Owned::fresh(list::new_list_obj(&[]));
        let refusal = unsafe { probe_list_elements(None, value.as_ptr()) }.unwrap_err();
        assert_eq!(
            refusal.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "missing original C API collection interpreter issuer"
                )
            )
        );
        let mut count: TclSize = -29;
        let mut window = ptr::dangling_mut::<*mut TclObj>();
        assert_eq!(
            unsafe {
                get_elements(
                    ptr::null_mut(),
                    value.as_ptr(),
                    &raw mut count,
                    &raw mut window,
                )
            },
            TCL_ERROR
        );
        assert_eq!(count, -29);
        assert_eq!(window, ptr::dangling_mut());
        let mut interpreter = native(TclVersion::V8_6);
        assert_eq!(
            unsafe {
                get_elements(
                    &mut interpreter,
                    value.as_ptr(),
                    &raw mut count,
                    &raw mut window,
                )
            },
            TCL_OK
        );
        interpreter.set_runtime_version(TclVersion::V9_0);
        count = -29;
        assert_eq!(
            unsafe { length(&mut interpreter, value.as_ptr(), &raw mut count) },
            TCL_ERROR
        );
        assert_eq!(count, -29);
        assert!(interpreter.native_execution_refusal().is_some());
    });
}

#[test]
fn original_index_cache_keeps_table_stride_protocol_and_selected_output_width() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Original table contract and source spelling; no NativeStaticIndexTable capability fabricated.
    leak_free(|| {
        let mut interpreter = native(TclVersion::V9_0);
        let value = obj::Owned::fresh(obj::new_string_bytes(b"al"));
        let names = table();
        let mut tiny = 91_u8;
        assert_eq!(
            unsafe {
                get_index(
                    &mut interpreter,
                    value.as_ptr(),
                    names.as_ptr().cast(),
                    size_of::<*const c_char>() as TclSize,
                    c"option".as_ptr(),
                    2,
                    (&raw mut tiny).cast(),
                )
            },
            TCL_OK
        );
        assert_eq!(tiny, 0);
        assert!(ptr::eq(obj::obj_type_ptr(value.as_ptr()), &TCL_INDEX_TYPE));
        let mut index = -1_i32;
        assert_eq!(
            unsafe {
                get_index(
                    &mut interpreter,
                    value.as_ptr(),
                    names.as_ptr().cast(),
                    size_of::<*const c_char>() as TclSize,
                    c"option".as_ptr(),
                    TCL_EXACT,
                    (&raw mut index).cast(),
                )
            },
            TCL_OK
        );
        assert_eq!(index, 0);
        assert_eq!(
            index_spelling(
                &Operation::enter(Some(&interpreter), &[value.as_ptr()]).unwrap(),
                value.as_ptr()
            )
            .unwrap()
            .unwrap(),
            b"alpha"
        );
        let other = [c"allow".as_ptr(), ptr::null()];
        assert_eq!(
            unsafe {
                get_index(
                    &mut interpreter,
                    value.as_ptr(),
                    other.as_ptr().cast(),
                    size_of::<*const c_char>() as TclSize,
                    c"option".as_ptr(),
                    0,
                    (&raw mut index).cast(),
                )
            },
            TCL_OK
        );
        assert_eq!(index_rep(value.as_ptr()).table, other.as_ptr().cast());
        assert_eq!(index_rep(value.as_ptr()).entry, c"allow".as_ptr());
        let temporary = obj::Owned::fresh(obj::new_string_bytes(b"be"));
        assert_eq!(
            unsafe {
                get_index(
                    &mut interpreter,
                    temporary.as_ptr(),
                    names.as_ptr().cast(),
                    size_of::<*const c_char>() as TclSize,
                    c"option".as_ptr(),
                    TCL_INDEX_TEMP_TABLE,
                    (&raw mut index).cast(),
                )
            },
            TCL_OK
        );
        assert_eq!(index, 1);
        assert!(obj::obj_type_ptr(temporary.as_ptr()).is_null());
        interpreter.set_runtime_version(TclVersion::V8_6);
        index = -1;
        assert_eq!(
            unsafe {
                get_index(
                    &mut interpreter,
                    value.as_ptr(),
                    other.as_ptr().cast(),
                    size_of::<*const c_char>() as TclSize,
                    c"option".as_ptr(),
                    0,
                    (&raw mut index).cast(),
                )
            },
            TCL_ERROR
        );
        assert_eq!(index, -1);
    });
}

#[test]
fn original_collection_parse_failure_keeps_original_primary_and_owned_children() {
    // naming.embedding.original-host-publication-and-fact-transport

    // The genuine selected List parser fails before original retirement; no native failure row is inferred.
    leak_free(|| {
        let mut interpreter = native(TclVersion::V8_6);
        let (value, updates, frees) = callback(&interpreter, 0, true, true, b"{");
        let member = obj::Owned::fresh(obj::new_string_bytes(b"member"));
        let mut count: TclSize = -43;
        let mut window = ptr::dangling_mut::<*mut TclObj>();
        assert_eq!(
            unsafe {
                get_elements(
                    &mut interpreter,
                    value.as_ptr(),
                    &raw mut count,
                    &raw mut window,
                )
            },
            TCL_ERROR
        );
        assert_eq!(count, -43);
        assert_eq!(window, ptr::dangling_mut());
        assert!(ptr::eq(obj::obj_type_ptr(value.as_ptr()), &CALLBACK));
        assert_eq!(frees.get(), 0);
        assert_eq!(updates.get(), 0);
        assert!(interpreter.native_execution_refusal().is_none());
        assert!(interpreter
            .result_bytes()
            .starts_with(b"unmatched open brace"));
        assert_eq!(
            unsafe { append(&mut interpreter, value.as_ptr(), member.as_ptr()) },
            TCL_ERROR
        );
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 1);
        assert_eq!(frees.get(), 0);
        drop(value);
        assert_eq!(frees.get(), 1);
    });
}

#[test]
fn original_collection_dictionary_members_require_their_retained_protocol() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Pure original Dictionary member conversion validates its actual backing recipe without a String detour.
    for foreign in [false, true] {
        leak_free(|| {
            let mut interpreter = native(TclVersion::V8_6);
            let protocol = interpreter
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let key = obj::Owned::fresh(obj::new_string_bytes(b"original-key"));
            let member = obj::Owned::fresh(obj::new_string_bytes(b"original-member"));
            let value = obj::Owned::fresh(
                crate::dict::new_dict_obj_native(
                    &[(key.as_ptr(), member.as_ptr())],
                    None,
                    protocol,
                )
                .unwrap(),
            );
            assert!(!obj::has_string_rep(value.as_ptr()));
            if foreign {
                interpreter.set_runtime_version(TclVersion::V9_0);
            }
            let mut count: TclSize = -41;
            let mut window = ptr::dangling_mut::<*mut TclObj>();
            let status = unsafe {
                get_elements(
                    &mut interpreter,
                    value.as_ptr(),
                    &raw mut count,
                    &raw mut window,
                )
            };
            if foreign {
                assert_eq!(status, TCL_ERROR);
                assert_eq!(count, -41);
                assert_eq!(window, ptr::dangling_mut());
                assert!(ptr::eq(
                    obj::obj_type_ptr(value.as_ptr()),
                    &crate::dict::TCL_DICT_TYPE
                ));
                assert_eq!(
                    crate::dict::native_string_protocol(value.as_ptr()),
                    Some(protocol)
                );
                assert!(interpreter.native_execution_refusal().is_some());
            } else {
                assert_eq!(status, TCL_OK);
                assert_eq!(count, 2);
                assert!(ptr::eq(
                    obj::obj_type_ptr(value.as_ptr()),
                    &list::TCL_LIST_TYPE
                ));
                assert_eq!(unsafe { *window }, key.as_ptr());
                assert_eq!(unsafe { *window.add(1) }, member.as_ptr());
                assert!(interpreter.native_execution_refusal().is_none());
            }
            assert!(!obj::has_string_rep(value.as_ptr()));
            assert_eq!(unsafe { (*key.as_ptr()).ref_count }, 2);
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
        });
    }
}

#[test]
fn generic_collection_preserves_first_refusal_before_original_updater() {
    leak_free(|| {
        let mut interpreter = Interp::new();
        interpreter.set_runtime_version(TclVersion::V9_0);
        let initial_refusal = interpreter.native_execution_refusal().is_some();
        interpreter.set_result_bytes(b"SEEDED RESULT");
        interpreter.set_c_error_code(b"STAMP BEFORE");
        let seeded_refusal = interpreter.native_execution_refusal().is_some();
        if !seeded_refusal {
            interpreter.refuse_host_command("existing collection refusal");
        }
        let first = interpreter.native_execution_refusal().unwrap();
        if std::env::var("TCL_LSP_RUNTIME_BASELINE_DIAGNOSTIC").as_deref() == Ok("1") {
            eprintln!("COLLECTION_GENERIC_ENTRY initial_refusal={initial_refusal} seeded_refusal={seeded_refusal}");
        }
        let (value, updates, frees) = callback(&interpreter, 0, false, false, b"alpha beta");
        let mut count: TclSize = -71;
        let mut window = ptr::dangling_mut::<*mut TclObj>();
        assert_eq!(
            unsafe {
                get_elements(
                    &mut interpreter,
                    value.as_ptr(),
                    &raw mut count,
                    &raw mut window,
                )
            },
            TCL_ERROR
        );
        assert_eq!(updates.get(), 0);
        assert_eq!(frees.get(), 0);
        assert_eq!(count, -71);
        assert_eq!(window, ptr::dangling_mut());
        assert_eq!(interpreter.native_execution_refusal(), Some(first));
        assert!(ptr::eq(obj::obj_type_ptr(value.as_ptr()), &CALLBACK));
        drop(value);
        assert_eq!(frees.get(), 1);
    });
}

#[test]
fn generic_actual_c_profile_without_refusal_enters_collection_updater() {
    leak_free(|| {
        let mut interpreter = Interp::new();
        interpreter.set_runtime_version(TclVersion::V9_0);
        assert!(interpreter.native_execution_refusal().is_none());
        let (value, updates, frees) = callback(&interpreter, 0, false, false, b"alpha beta");
        let mut count: TclSize = -71;
        assert_eq!(
            unsafe { length(&mut interpreter, value.as_ptr(), &raw mut count) },
            TCL_OK,
        );
        assert_eq!(count, 2);
        assert_eq!(updates.get(), 1);
        assert_eq!(frees.get(), 1);
        assert!(ptr::eq(
            obj::obj_type_ptr(value.as_ptr()),
            &list::TCL_LIST_TYPE
        ));
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        assert!(interpreter.native_execution_refusal().is_none());
        drop(value);
        assert_eq!(frees.get(), 1);
    });
}
