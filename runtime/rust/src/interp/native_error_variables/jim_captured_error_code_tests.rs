// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::obj::Owned;

fn jim() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn original_jim_catch_try_options_reuse_the_once_captured_global_header() {
    let observations = include_str!(
        "../../../tests/data/native_jim_captured_error_code_options/observations-20261015.tsv"
    );
    for mode in 0..4 {
        let mut actual = jim();
        let head = Owned::fresh(new_string(if mode % 2 == 0 { b"catch" } else { b"try" }));
        let body = Owned::fresh(new_string(if mode < 2 {
            include_bytes!("../../../tests/data/native_jim_captured_error_code_options/body.tcl")
                .as_slice()
        } else {
            include_bytes!("../../../tests/data/native_jim_captured_error_code_options/ok-body.tcl")
                .as_slice()
        }));
        let name = Owned::fresh(new_string(b"::errorCode"));
        let opts = Owned::fresh(new_string(b"opts"));
        let on = Owned::fresh(new_string(b"on"));
        let error = Owned::fresh(new_string(if mode < 2 { b"error" } else { b"ok" }));
        let vars = Owned::fresh(new_string(b"::errorCode opts"));
        let handler = Owned::fresh(new_string(b"set HANDLED YES"));
        let argv = if mode % 2 == 0 {
            vec![head.as_ptr(), body.as_ptr(), name.as_ptr(), opts.as_ptr()]
        } else {
            vec![
                head.as_ptr(),
                body.as_ptr(),
                on.as_ptr(),
                error.as_ptr(),
                vars.as_ptr(),
                handler.as_ptr(),
            ]
        };
        let code = actual.eval_original_object_vector(&argv);
        let result = actual
            .result_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert!(
            observations
                .lines()
                .any(|line| line == format!("RESULT|{mode}|{}|{result}", code.as_int())),
            "{:?}",
            actual.native_execution_refusal()
        );
        let kept = actual.var_get(b"keep");
        let current = actual.var_get(b"errorCode").unwrap();
        let options = actual.var_get(b"opts").unwrap();
        let members = crate::list::list_elements_native_checked(
            options,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap();
        let captured = members
            .chunks_exact(2)
            .find(|pair| crate::interp::obj_bytes(pair[0]) == b"-errorcode")
            .map(|pair| pair[1]);
        let value = captured
            .map(|value| {
                crate::interp::obj_bytes(value)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            })
            .unwrap_or_else(|| "-".into());
        let row = format!(
            "CAPTURE|{mode}|{}|{}|{}|{value}",
            u8::from(captured.is_some()),
            u8::from(captured.is_some_and(|value| Some(value) == kept)),
            u8::from(captured == Some(current))
        );
        assert!(observations.lines().any(|line| line == row), "{row}");
        let current = crate::interp::obj_bytes(current)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert!(observations
            .lines()
            .any(|line| line == format!("CURRENT|{mode}|{current}")));
        assert!(actual.native_execution_refusal().is_none());
    }
}

#[test]
fn captured_error_code_has_no_early_native_hold_and_cannot_revive_a_retired_header() {
    let mut actual = jim();
    let value = Owned::fresh(obj::new_wide_int_obj(777));
    actual
        .assign_original_jim_global_error_code(value.as_ptr())
        .unwrap();
    // SAFETY: the actual Owned value and global cell both retain this header.
    let before = unsafe { (*value.as_ptr()).ref_count };
    let captured = actual.capture_original_jim_error_code().unwrap();
    assert_eq!(captured.original(&actual).unwrap(), Some(value.as_ptr()));
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, before);
    drop(value);
    let replacement = Owned::fresh(obj::new_wide_int_obj(888));
    actual
        .assign_original_jim_global_error_code(replacement.as_ptr())
        .unwrap();
    assert!(captured.original(&actual).is_err());
    assert!(actual
        .jim_return_receipt_with_captured_error_code(&captured)
        .is_err());
    assert!(actual.native_execution_refusal().is_some());
}

#[test]
fn unused_captured_error_code_can_retire_before_non_error_options() {
    let mut actual = jim();
    let value = Owned::fresh(obj::new_wide_int_obj(777));
    actual
        .assign_original_jim_global_error_code(value.as_ptr())
        .unwrap();
    let captured = actual.capture_original_jim_error_code().unwrap();
    drop(value);
    let replacement = Owned::fresh(obj::new_wide_int_obj(888));
    actual
        .assign_original_jim_global_error_code(replacement.as_ptr())
        .unwrap();
    let options = actual
        .jim_option_pairs_with_captured_error_code(&captured, crate::interp::Code::Ok)
        .unwrap();
    assert_eq!(options.len(), 2);
    assert!(actual.native_execution_refusal().is_none());
}

#[test]
fn captured_missing_value_does_not_requery_a_later_global_or_cross_interpreters() {
    let mut actual = jim();
    let captured = actual.capture_original_jim_error_code().unwrap();
    assert!(captured.original(&actual).unwrap().is_none());
    let value = Owned::fresh(obj::new_wide_int_obj(777));
    actual
        .assign_original_jim_global_error_code(value.as_ptr())
        .unwrap();
    assert!(actual
        .jim_return_receipt_with_captured_error_code(&captured)
        .unwrap()
        .error_code
        .is_none());
    let foreign = jim();
    assert!(captured.original(&foreign).is_err());
    assert!(foreign.native_execution_refusal().is_some());
}
