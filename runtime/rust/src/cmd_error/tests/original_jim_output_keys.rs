// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::{
    interp::{Code, Interp},
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use tcl_syntax::native_jim_lookup::NativeJimVariableInventoryLookup;

const OBSERVATIONS: &str = include_str!(
    "../../../tests/data/native_jim_catch_try_original_output_keys/observations-20261015.tsv"
);
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
fn original_jim_catch_and_try_publish_original_output_keys_and_preserve_earlier_keys() {
    for command in ["catch", "try"] {
        for mode in 0..4 {
            let mut actual = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("jimtcl"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            if mode == 2 {
                assert_eq!(actual.eval_str(b"array set 123 {member VALUE}"), Code::Ok);
            } else if mode == 3 {
                assert_eq!(
                    actual.eval_str(b"set scalar ODD; upvar #0 scalar(member) 123"),
                    Code::Ok
                );
            }
            let message = Owned::fresh(if mode == 1 {
                obj::new_string_bytes(b"")
            } else {
                obj::new_wide_int_obj(123)
            });
            let options = Owned::fresh(obj::new_wide_int_obj(456));
            let head = Owned::fresh(obj::new_string_bytes(command.as_bytes()));
            let body = Owned::fresh(obj::new_string_bytes(b"error BOOM"));
            let mut owners = Vec::new();
            let mut argv = vec![head.as_ptr(), body.as_ptr()];
            if command == "try" {
                owners.extend([
                    Owned::fresh(obj::new_string_bytes(b"on")),
                    Owned::fresh(obj::new_string_bytes(b"error")),
                    Owned::fresh(actual.new_list_object(&[message.as_ptr(), options.as_ptr()])),
                    Owned::fresh(obj::new_string_bytes(b"list")),
                ]);
                argv.extend(owners.iter().map(Owned::as_ptr));
            } else {
                argv.extend([message.as_ptr(), options.as_ptr()]);
            }
            let code = actual.eval_original_object_vector(&argv);
            let result = format!("RESULT|{command}|{mode}|{}", code.as_int());
            assert!(
                OBSERVATIONS.lines().any(|line| line == result),
                "{result}; {:?}",
                actual.native_execution_refusal()
            );
            for (label, name, object) in [
                (
                    "MESSAGE",
                    if mode == 1 {
                        b"".as_slice()
                    } else {
                        b"123".as_slice()
                    },
                    message.as_ptr(),
                ),
                ("OPTIONS", b"456".as_slice(), options.as_ptr()),
            ] {
                let prefix = format!("KEY|{command}|{mode}|{label}|");
                let row = OBSERVATIONS
                    .lines()
                    .find(|line| line.starts_with(&prefix))
                    .unwrap()
                    .split('|')
                    .collect::<Vec<_>>();
                assert_eq!(obj::has_string_rep(object), row[4] == "1");
                assert_eq!(type_name(object), row[5].as_bytes());
                let inventory = actual
                    .namespaces()
                    .var_table(GLOBAL)
                    .capture_original_jim_inventory(
                        true,
                        NativeJimVariableInventoryLookup::Exact(name),
                    )
                    .unwrap();
                assert_eq!(!inventory.keys().is_empty(), row[6] == "1");
                let same = inventory
                    .keys()
                    .first()
                    .is_some_and(|key| key.original().unwrap() == object);
                assert_eq!(same, row[7] == "1");
            }
            assert!(actual.native_execution_refusal().is_none());
        }
    }
}
