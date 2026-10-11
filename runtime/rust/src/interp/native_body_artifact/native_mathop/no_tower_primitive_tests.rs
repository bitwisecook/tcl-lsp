// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual existing primitive stages under the absent expression backend.
use super::*;

fn native() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn original_no_tower_string_equality_uses_its_existing_independent_worker() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The shared body primitive's independent String branch remains reachable.
    let mut interpreter = native();
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
        .unwrap();
    let left = obj::Owned::fresh(obj::new_string_bytes(b"EXACT"));
    let right = obj::Owned::fresh(obj::new_string_bytes(b"EXACT"));
    let result = interpreter
        .compiled_mathop_primitive(
            M::StringEqual,
            vec![left, right],
            tcl_dialect::TclVersion::V8_6,
        )
        .unwrap();
    assert_eq!(obj::bytes_of(result.as_ptr()), b"1");
    assert!(interpreter.native_execution_refusal().is_none());
    scope.currency().ensure_current_or_refuse().unwrap();
}

#[test]
fn original_no_tower_numeric_primitive_refuses_before_result_or_guest_numeric_error() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Original reached operands are cleaned up; no unavailable arithmetic worker or Guest parse runs.
    let mut interpreter = native();
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
        .unwrap();
    interpreter.set_result_bytes(b"BEFORE_NUMERIC_STAGE");
    let before = interpreter.result_obj();
    let result = interpreter.compiled_mathop_primitive(
        M::Add,
        vec![
            obj::Owned::fresh(obj::new_string_bytes(b"not-a-number")),
            obj::Owned::fresh(obj::new_string_bytes(b"later-number")),
        ],
        tcl_dialect::TclVersion::V8_6,
    );
    assert!(matches!(result, Err(Code::Error)));
    assert_eq!(interpreter.result_obj(), before);
    let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable,
    );
    assert_eq!(interpreter.native_execution_refusal(), Some(first.clone()));
    assert_eq!(scope.currency().ensure_current_or_refuse(), Err(first));
}
