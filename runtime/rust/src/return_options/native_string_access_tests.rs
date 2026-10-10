// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Software controls for selected original return-option String access.
//! Native processes, scalar primitive conversion and expression truth are separate.

use super::{NativeReturnOps, ReturnOptionsOps};
use crate::{
    interp::{Code, Interp},
    obj,
};

fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::ingress::resolve_environment(profile).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn original_return_option_string_access_keeps_counted_resident_key_and_header() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let interp = native(profile);
        let (mut ops, _) = NativeReturnOps::selected(&interp).unwrap();
        let original = obj::Owned::fresh(obj::new_string_bytes(b"-custom\0\xff"));
        let primary = obj::obj_type_ptr(original.as_ptr());
        assert_eq!(
            ReturnOptionsOps::bytes(&mut ops, &original).unwrap(),
            b"-custom\0\xff"
        );
        assert_eq!(obj::obj_type_ptr(original.as_ptr()), primary);
        assert!(obj::has_string_rep(original.as_ptr()));
        assert!(!interp.host_refusal_pending());
    }
}

#[cfg(have_tommath)]
fn capacity() -> tcl_syntax::raw_string::NativeValueAccessRefusal {
    tcl_syntax::raw_string::NativeValueAccessRefusal::Materialization(
        tcl_syntax::raw_string::NativeMaterializationLimitError::new(100_000_001, 100_000_000),
    )
}

#[cfg(have_tommath)]
#[test]
fn original_return_option_string_access_keeps_first_host_and_prior_result() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native("tcl9.0");
    assert_eq!(interp.eval_str(b"lseq 100000001"), Code::Ok);
    let original = obj::Owned::retain(interp.get_obj_result());
    assert!(!obj::has_string_rep(original.as_ptr()));
    let (mut ops, _) = NativeReturnOps::selected(&interp).unwrap();
    interp.set_result_bytes(b"BODY\0\xff");
    let result = interp.get_obj_result();
    interp.refuse_host_command("original return-option first cause");
    let first = interp.native_execution_refusal().unwrap();
    let error = ReturnOptionsOps::bytes(&mut ops, &original).unwrap_err();
    assert_eq!(error.native_access_refusal(), Some(capacity()));
    assert_eq!(interp.report_cmd_error(error), Code::Error);
    assert_eq!(interp.native_execution_refusal(), Some(first.clone()));
    assert_eq!(interp.error_code_bytes_checked().unwrap_err(), first);
    assert_eq!(interp.get_obj_result(), result);
    assert_eq!(interp.result_bytes(), b"BODY\0\xff");
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert!(!crate::native_arithseries::has_element_cache(
        original.as_ptr()
    ));
}

#[cfg(have_tommath)]
#[test]
fn original_return_option_key_capacity_is_host_refusal_outside_guest_catch() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_runtime_api::NativeExecutionError;
    let mut interp = native("tcl9.0");
    assert_eq!(interp.eval_str(b"set ::key [lseq 100000001]"), Code::Ok);
    let original = obj::Owned::retain(interp.var_get(b"::key").unwrap());
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert_eq!(interp.eval_str(b"set ::reached BEFORE; catch {return -options [list $::key value]} ::caught ::options; set ::after AFTER"), Code::Error);
    let expected = NativeExecutionError::ValueAccessRefusal(capacity());
    assert_eq!(interp.native_execution_refusal(), Some(expected.clone()));
    assert_eq!(interp.error_code_bytes_checked().unwrap_err(), expected);
    assert!(interp.var_get(b"::reached").is_some());
    assert!(interp.var_get(b"::caught").is_none());
    assert!(interp.var_get(b"::options").is_none());
    assert!(interp.var_get(b"::after").is_none());
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert!(!crate::native_arithseries::has_element_cache(
        original.as_ptr()
    ));
}
