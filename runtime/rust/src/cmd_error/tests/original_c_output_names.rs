// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::{
    interp::{Code, Interp},
    obj::{self, Owned, TclObj},
};
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

#[test]
fn original_c_catch_uses_original_name_headers_and_keeps_unassigned_options_original() {
    for (dialect, observations) in [
        (
            "tcl8.4",
            include_str!("../../../tests/data/native_c_catch_original_output_names/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tests/data/native_c_catch_original_output_names/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tests/data/native_c_catch_original_output_names/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tests/data/native_c_catch_original_output_names/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tests/data/native_c_catch_original_output_names/9.1.0.tsv"),
        ),
    ] {
        for mode in 0..3 {
            let mut actual = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(dialect),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            if mode == 2 {
                assert_eq!(actual.eval_str(b"array set 123 {member VALUE}"), Code::Ok);
            }
            let head = Owned::fresh(obj::new_string_bytes(b"catch"));
            let body = Owned::fresh(obj::new_string_bytes(b"error BOOM"));
            let name = Owned::fresh(if mode == 1 {
                obj::new_string_bytes(b"")
            } else {
                obj::new_wide_int_obj(123)
            });
            let options = Owned::fresh(obj::new_wide_int_obj(456));
            let mut argv = vec![head.as_ptr(), body.as_ptr(), name.as_ptr()];
            if dialect != "tcl8.4" {
                argv.push(options.as_ptr());
            }
            let code = actual.eval_original_object_vector(&argv);
            let row = format!("RESULT|{mode}|{}", code.as_int());
            assert!(
                observations.lines().any(|line| line == row),
                "{dialect}/{row}; {:?}",
                actual.native_execution_refusal()
            );
            for (ordinal, object) in [name.as_ptr(), options.as_ptr()].into_iter().enumerate() {
                let prefix = format!("NAME|{mode}|{ordinal}|");
                let row = observations
                    .lines()
                    .find(|line| line.starts_with(&prefix))
                    .unwrap()
                    .split('|')
                    .collect::<Vec<_>>();
                assert_eq!(
                    obj::has_string_rep(object),
                    row[3] == "1",
                    "{dialect}/{mode}/{ordinal}"
                );
                assert_eq!(
                    type_name(object),
                    row[4].as_bytes(),
                    "{dialect}/{mode}/{ordinal}"
                );
            }
            assert!(actual.native_execution_refusal().is_none());
        }
    }
}
