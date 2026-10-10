// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Software controls for the shared integer command error boundary.
//! These controls do not establish external C Tcl/Jim/BIG-IP pointer behaviour.

use super::{ValueError, ValueOps, integer_error};
use crate::{
    interp::{Code, Interp},
    obj,
};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

fn native_tcl84() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

fn original_overflow(interp: &mut Interp) -> ValueError {
    let current = obj::Owned::fresh(obj::new_string_bytes(b"18446744073709551616"));
    let increment = obj::Owned::fresh(obj::new_wide_int_obj(1));
    ValueOps::int_add(interp, Some(&current.as_ptr()), &increment.as_ptr()).unwrap_err()
}

#[test]
fn retired_integer_operand_retains_host_cause_before_guest_error_publication() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native_tcl84();
    interp.set_result_bytes(b"BODY\0\xff");
    let result = interp.get_obj_result();
    let original = obj::Owned::fresh(obj::new_wide_int_obj(42));
    // The lease retains allocation memory only; dropping the last native
    // owner still retires the header. No header or string getter follows.
    let lease = obj::NativeObjectLifetime::retain(original.as_ptr());
    drop(original);
    let retired = lease.as_ptr();
    assert!(!obj::allocation_is_live(retired));
    let increment = obj::Owned::fresh(obj::new_wide_int_obj(1));
    let error = ValueOps::int_add(&mut interp, Some(&retired), &increment.as_ptr()).unwrap_err();
    assert_eq!(
        error,
        ValueError::CommandProtocolUnavailable("retired native object")
    );
    let cause = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable("retired native object"),
    );
    assert_eq!(integer_error(&mut interp, error), Code::Error);
    assert_eq!(interp.native_execution_refusal(), Some(cause.clone()));
    assert_eq!(interp.error_code_bytes_checked().unwrap_err(), cause);
    assert_eq!(interp.get_obj_result(), result);
    assert_eq!(interp.result_bytes(), b"BODY\0\xff");
    assert!(!obj::allocation_is_live(retired));

    // A later genuine Guest overflow cannot replace the first Host cause.
    let overflow = original_overflow(&mut interp);
    assert_eq!(overflow, ValueError::IntegerOverflow);
    assert_eq!(integer_error(&mut interp, overflow), Code::Error);
    assert_eq!(interp.native_execution_refusal(), Some(cause));
    assert_eq!(interp.get_obj_result(), result);
    assert_eq!(interp.result_bytes(), b"BODY\0\xff");
}

#[test]
fn original_tcl84_integer_overflow_keeps_its_guest_diagnostic() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native_tcl84();
    let error = original_overflow(&mut interp);
    assert_eq!(error, ValueError::IntegerOverflow);
    assert_eq!(integer_error(&mut interp, error), Code::Error);
    assert_eq!(
        interp.result_bytes(),
        b"integer value too large to represent"
    );
    assert_eq!(
        interp.error_code_bytes_checked().unwrap(),
        b"ARITH IOVERFLOW {integer value too large to represent}",
    );
    assert!(interp.native_execution_refusal().is_none());
}
