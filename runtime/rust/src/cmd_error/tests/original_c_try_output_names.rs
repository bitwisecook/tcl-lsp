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
    // SAFETY: this retained original header has its actual static descriptor.
    unsafe { std::ffi::CStr::from_ptr((*kind).name) }
        .to_bytes()
        .to_vec()
}

#[test]
fn original_c_try_retains_selected_handler_member_names_and_validates_before_body() {
    for (dialect, observations) in [
        (
            "tcl8.6",
            include_str!("../../../tests/data/native_c_try_original_output_names/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tests/data/native_c_try_original_output_names/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tests/data/native_c_try_original_output_names/9.1.0.tsv"),
        ),
    ] {
        for mode in 0..5 {
            let mut actual = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(dialect),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            if mode == 2 {
                assert_eq!(actual.eval_str(b"array set 123 {member VALUE}"), Code::Ok);
            }
            let names = [
                Owned::fresh(if mode == 1 {
                    obj::new_string_bytes(b"")
                } else {
                    obj::new_wide_int_obj(123)
                }),
                Owned::fresh(obj::new_wide_int_obj(456)),
            ];
            let unused = [
                Owned::fresh(obj::new_wide_int_obj(777)),
                Owned::fresh(obj::new_wide_int_obj(888)),
            ];
            let protocol = actual
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let vars = Owned::fresh(if mode == 4 {
                obj::new_string_bytes(b"{")
            } else {
                crate::list::new_list_obj_native(&names.each_ref().map(Owned::as_ptr), protocol)
            });
            let unused_vars = Owned::fresh(crate::list::new_list_obj_native(
                &unused.each_ref().map(Owned::as_ptr),
                protocol,
            ));
            let head = Owned::fresh(obj::new_string_bytes(b"try"));
            let body = Owned::fresh(obj::new_string_bytes(b"set BODY RAN; error BOOM"));
            let on = Owned::fresh(obj::new_string_bytes(b"on"));
            let error = Owned::fresh(obj::new_string_bytes(b"error"));
            let dash = Owned::fresh(obj::new_string_bytes(b"-"));
            let handler = Owned::fresh(obj::new_string_bytes(b"set HANDLED YES"));
            let mut argv = vec![head.as_ptr(), body.as_ptr(), on.as_ptr(), error.as_ptr()];
            if mode == 3 {
                argv.extend_from_slice(&[
                    unused_vars.as_ptr(),
                    dash.as_ptr(),
                    on.as_ptr(),
                    error.as_ptr(),
                ]);
            }
            argv.extend_from_slice(&[vars.as_ptr(), handler.as_ptr()]);
            let code = actual.eval_original_object_vector(&argv);
            let body_ran = actual.var_get(b"BODY").is_some();
            let handler_ran = actual.var_get(b"HANDLED").is_some();
            let row = format!(
                "RESULT|{mode}|{}|{}|{}",
                code.as_int(),
                u8::from(body_ran),
                u8::from(handler_ran)
            );
            assert!(
                observations.lines().any(|line| line == row),
                "{dialect}/{row}; {:?}",
                actual.native_execution_refusal()
            );
            for (ordinal, object) in names.iter().chain(unused.iter()).enumerate() {
                let prefix = format!("NAME|{mode}|{ordinal}|");
                let row = observations
                    .lines()
                    .find(|line| line.starts_with(&prefix))
                    .unwrap()
                    .split('|')
                    .collect::<Vec<_>>();
                assert_eq!(
                    obj::has_string_rep(object.as_ptr()),
                    row[3] == "1",
                    "{dialect}/{mode}/{ordinal}"
                );
                assert_eq!(
                    type_name(object.as_ptr()),
                    row[4].as_bytes(),
                    "{dialect}/{mode}/{ordinal}"
                );
            }
            assert!(actual.native_execution_refusal().is_none());
        }
    }
}
