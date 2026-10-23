// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::interp::{Code, TclObj};
thread_local! {
    static MODE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static EXPECTED: std::cell::Cell<*mut TclObj> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}
const OBSERVATIONS: &str = include_str!(
    "../../../tests/data/native_jim_original_global_error_code_getter/observations-20261015.tsv"
);
fn jim() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn type_name(object: *mut TclObj) -> Vec<u8> {
    let kind = obj::obj_type_ptr(object);
    if kind.is_null() {
        return b"none".to_vec();
    }
    // SAFETY: the actual retained header has a static native descriptor.
    unsafe { std::ffi::CStr::from_ptr((*kind).name) }
        .to_bytes()
        .to_vec()
}
fn read_in_caller(actual: &mut Interp, _: &[*mut TclObj]) -> Code {
    let frame = super::super::native_arguments::OriginalArgumentFrame::capture(actual);
    let namespace = actual.current_ns();
    assert_ne!(namespace, GLOBAL);
    assert_ne!(actual.frames.borrow().current_frame_index(), 0);
    let local = obj::NativeObjectLifetime::retain(actual.var_get(b"errorCode").unwrap());
    actual.set_result_bytes(b"MARKER");
    let value = actual.read_original_jim_global_error_code().unwrap();
    let mode = MODE.with(std::cell::Cell::get);
    let row = OBSERVATIONS
        .lines()
        .find(|line| line.starts_with(&format!("GLOBAL_READ|{mode}|")))
        .unwrap()
        .split('|')
        .collect::<Vec<_>>();
    assert_eq!(value.is_some(), row[2] == "1");
    assert_eq!(
        value.is_some_and(|value| EXPECTED.with(|expected| expected.get()) == value),
        row[3] == "1"
    );
    frame.check_frame(actual).unwrap();
    assert_eq!(actual.current_ns(), namespace);
    assert_eq!(
        actual.var_get(b"errorCode"),
        Some(local.checked_ptr().unwrap())
    );
    if let Some(value) = value {
        assert_eq!(obj::has_string_rep(value), row[7] == "1");
        assert_eq!(type_name(value), row[8].as_bytes());
    }
    let result = actual
        .result_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(result, row[9]);
    assert!(actual.native_execution_refusal().is_none());
    Code::Ok
}

#[test]
fn original_jim_quiet_global_error_code_getter_keeps_actual_headers_and_caller_context() {
    for line in
        include_str!("../../../tests/data/native_jim_original_global_error_code_getter/setups.tsv")
            .lines()
    {
        let (mode, source) = line.split_once('|').unwrap();
        let mode: usize = mode.parse().unwrap();
        MODE.with(|selected| selected.set(mode));
        let mut actual = jim();
        let value = obj::Owned::fresh(obj::new_wide_int_obj(777));
        EXPECTED.with(|expected| expected.set(std::ptr::null_mut()));
        if mode == 1 {
            actual
                .assign_original_jim_global_error_code(value.as_ptr())
                .unwrap();
            EXPECTED.with(|expected| expected.set(value.as_ptr()));
        } else if mode == 2 {
            let target = obj::Owned::fresh(new_string(b"target"));
            actual
                .assign_original_named_variable(target.as_ptr(), value.as_ptr())
                .unwrap();
            EXPECTED.with(|expected| expected.set(value.as_ptr()));
        }
        assert_eq!(actual.eval_str(source.as_bytes()), Code::Ok);
        actual.register_builtin(b"read_selected_error_code", read_in_caller);
        assert_eq!(
            actual.eval_str(include_bytes!(
                "../../../tests/data/native_jim_original_global_error_code_getter/call.tcl"
            )),
            Code::Ok
        );
        assert_eq!(actual.current_ns(), GLOBAL);
        assert_eq!(actual.frames.borrow().current_frame_index(), 0);
        EXPECTED.with(|expected| expected.set(std::ptr::null_mut()));
    }
}

#[test]
fn original_jim_return_receipt_keeps_the_actual_global_value_without_a_value_string_getter() {
    let mut actual = jim();
    let value = obj::Owned::fresh(obj::new_wide_int_obj(777));
    actual
        .assign_original_jim_global_error_code(value.as_ptr())
        .unwrap();
    let receipt = actual.jim_return_receipt().unwrap();
    assert_eq!(receipt.error_code.unwrap().as_ptr(), value.as_ptr());
    assert!(!obj::has_string_rep(value.as_ptr()));
    assert!(actual.native_execution_refusal().is_none());
}
