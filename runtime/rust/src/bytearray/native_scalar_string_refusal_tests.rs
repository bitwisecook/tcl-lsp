// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Software controls for the actual primitive String-access boundary.
//! External provider observations and expression truth remain independent.

use crate::{interp::Interp, obj};
use tcl_syntax::{scalar_getter::NativeScalarGetterKind as Kind, value::ValueError};

fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::ingress::resolve_environment(profile).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[cfg(have_tommath)]
#[test]
fn original_primitive_scalar_string_access_keeps_capacity_and_first_host_cause() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use crate::interp::Code;
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::raw_string::{NativeMaterializationLimitError, NativeValueAccessRefusal};
    let refusal = NativeValueAccessRefusal::Materialization(NativeMaterializationLimitError::new(
        100_000_001,
        100_000_000,
    ));
    for (kind, earlier_host) in [
        (Kind::Wide, false),
        (Kind::Double, false),
        (Kind::Boolean, false),
        (Kind::Wide, true),
    ] {
        let mut interp = native("tcl9.0");
        assert_eq!(interp.eval_str(b"lseq 100000001"), Code::Ok);
        let original = obj::Owned::retain(interp.get_obj_result());
        assert!(crate::native_arithseries::is_series(original.as_ptr()));
        assert!(!obj::has_string_rep(original.as_ptr()));
        assert!(!crate::native_arithseries::has_element_cache(
            original.as_ptr()
        ));
        interp.set_result_bytes(b"BODY\0\xff");
        let result = interp.get_obj_result();
        if earlier_host {
            interp.refuse_host_command("original primitive first cause");
        }
        let first = interp.native_execution_refusal();
        let dialect = interp.native_invocation_dialect();
        let error =
            crate::typed_value::native_scalar_probe(original.as_ptr(), dialect, kind).unwrap_err();
        assert_eq!(error.native_access_refusal(), Some(refusal));
        let rendered =
            crate::typed_value::native_scalar_getter(original.as_ptr(), dialect, kind).unwrap_err();
        assert_eq!(rendered.native_access_refusal(), Some(refusal));
        assert!(!obj::has_string_rep(original.as_ptr()));
        assert!(!crate::native_arithseries::has_element_cache(
            original.as_ptr()
        ));
        assert_eq!(interp.get_obj_result(), result);
        assert_eq!(interp.report_cmd_error(rendered.into()), Code::Error);
        let expected = first.unwrap_or(NativeExecutionError::ValueAccessRefusal(refusal));
        assert_eq!(interp.native_execution_refusal(), Some(expected.clone()));
        assert_eq!(interp.error_code_bytes_checked().unwrap_err(), expected);
        assert_eq!(interp.get_obj_result(), result);
        assert_eq!(interp.result_bytes(), b"BODY\0\xff");
    }
}

#[test]
fn original_primitive_scalar_storage_admission_precedes_bytearray_materialization() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let producer = native("tcl9.0");
    let recipe = producer
        .native_invocation_dialect()
        .byte_array_string_recipe(None)
        .unwrap();
    let consumer = native("jim");
    for kind in [Kind::Wide, Kind::Double, Kind::Boolean] {
        let original = obj::Owned::fresh(super::new_byte_array(b"42", recipe));
        assert!(!obj::has_string_rep(original.as_ptr()));
        let error = crate::typed_value::native_scalar_probe(
            original.as_ptr(),
            consumer.native_invocation_dialect(),
            kind,
        )
        .unwrap_err();
        assert_eq!(error, ValueError::ScalarNumericInputUnavailable);
        assert!(!obj::has_string_rep(original.as_ptr()));
        assert_eq!(
            super::native_cache_snapshot(original.as_ptr())
                .unwrap()
                .0
                .as_ref(),
            b"42"
        );
        assert!(!consumer.host_refusal_pending());
    }
}
