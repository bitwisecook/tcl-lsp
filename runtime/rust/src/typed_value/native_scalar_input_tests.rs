// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Software controls for checked original expression-input materialisation.
//! External provider observations and native pointer behaviour are independent.

use super::scalar_number;
use crate::{interp::Interp, obj};
use tcl_syntax::number::Number;

fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::ingress::resolve_environment(profile).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn original_resident_scalar_input_keeps_selected_parsing_and_cached_no_string_path() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for (profile, integer) in [
        ("tcl8.4", 8),
        ("tcl8.5", 8),
        ("tcl8.6", 8),
        ("tcl9.0", 10),
        ("tcl9.1", 10),
        ("jim", 8),
    ] {
        let interp = native(profile);
        let dialect = interp.native_invocation_dialect();
        let original = obj::Owned::fresh(obj::new_string_bytes(b"010"));
        assert!(obj::obj_type_ptr(original.as_ptr()).is_null());
        assert_eq!(
            scalar_number(original.as_ptr(), dialect, true).unwrap(),
            Some(Number::Int(integer)),
            "{profile}",
        );
        assert_eq!(obj::bytes_of(original.as_ptr()), b"010", "{profile}");
        let cached = obj::Owned::fresh(obj::new_wide_int_obj(42));
        assert!(!obj::has_string_rep(cached.as_ptr()));
        assert_eq!(
            scalar_number(cached.as_ptr(), dialect, true).unwrap(),
            Some(Number::Int(42)),
            "{profile}",
        );
        assert!(!obj::has_string_rep(cached.as_ptr()), "{profile}");
        assert!(!interp.host_refusal_pending(), "{profile}");
    }
}

#[cfg(have_tommath)]
fn original_series(interp: &mut Interp) -> obj::Owned {
    assert_eq!(interp.eval_str(b"lseq 100000001"), crate::interp::Code::Ok);
    let original = obj::Owned::retain(interp.get_obj_result());
    assert!(crate::native_arithseries::is_series(original.as_ptr()));
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert!(!crate::native_arithseries::has_element_cache(
        original.as_ptr()
    ));
    interp.set_result_bytes(b"BODY\0\xff");
    original
}

#[cfg(have_tommath)]
fn capacity() -> tcl_syntax::raw_string::NativeValueAccessRefusal {
    tcl_syntax::raw_string::NativeValueAccessRefusal::Materialization(
        tcl_syntax::raw_string::NativeMaterializationLimitError::new(100_000_001, 100_000_000),
    )
}

#[cfg(have_tommath)]
#[test]
fn original_lazy_scalar_input_refuses_capacity_before_integer_guest_conversion() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::value::ValueOps;
    let mut interp = native("tcl9.0");
    let original = original_series(&mut interp);
    let result = interp.get_obj_result();
    let increment = obj::Owned::fresh(obj::new_wide_int_obj(1));
    let error =
        ValueOps::int_add(&mut interp, Some(&original.as_ptr()), &increment.as_ptr()).unwrap_err();
    assert_eq!(error.native_access_refusal(), Some(capacity()));
    assert_eq!(
        crate::value_ops::integer_error(&mut interp, error),
        crate::interp::Code::Error,
    );
    let cause = NativeExecutionError::ValueAccessRefusal(capacity());
    assert_eq!(interp.native_execution_refusal(), Some(cause.clone()));
    assert_eq!(interp.error_code_bytes_checked().unwrap_err(), cause);
    assert_eq!(interp.get_obj_result(), result);
    assert_eq!(interp.result_bytes(), b"BODY\0\xff");
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert!(!crate::native_arithseries::has_element_cache(
        original.as_ptr()
    ));
}

#[cfg(have_tommath)]
#[test]
fn original_lazy_boolean_input_keeps_capacity_and_existing_first_host_metadata() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native("tcl9.0");
    let original = original_series(&mut interp);
    let result = interp.get_obj_result();
    interp.refuse_host_command("original scalar first cause");
    let first = interp.native_execution_refusal().unwrap();
    let error = match super::boolean_in(original.as_ptr(), interp.native_invocation_dialect()) {
        Err(error) => error,
        Ok(_) => panic!("original lazy Boolean input must retain the materialisation refusal"),
    };
    assert_eq!(error, capacity());
    assert_eq!(
        crate::value_ops::integer_error(&mut interp, error.into()),
        crate::interp::Code::Error,
    );
    assert_eq!(interp.native_execution_refusal(), Some(first.clone()));
    assert_eq!(interp.error_code_bytes_checked().unwrap_err(), first);
    assert_eq!(interp.get_obj_result(), result);
    assert_eq!(interp.result_bytes(), b"BODY\0\xff");
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert!(!crate::native_arithseries::has_element_cache(
        original.as_ptr()
    ));
}
