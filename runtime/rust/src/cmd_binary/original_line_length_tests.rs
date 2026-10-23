// SPDX-License-Identifier: AGPL-3.0-or-later
//! Primitive line-length command controls with authentic Runtime ingress.
use super::*;
use std::cell::Cell;
fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn call(interp: &mut Interp, words: &[&[u8]]) -> Code {
    let owned: Vec<_> = words
        .iter()
        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
        .collect();
    let argv: Vec<_> = owned.iter().map(obj::Owned::as_ptr).collect();
    binary_cmd(interp, &argv)
}
#[test]
fn original_binary_maxlen_uses_selected_primitive_errors_and_never_index_grammar() {
    // naming.numeric.io-debug-public-conversion-and-side-effect-order

    // Native628 measures stock final results; these exercise the actual Runtime adapter.
    for (profile, primitive) in [
        ("tcl8.6", b"TCL VALUE INTEGER".as_slice()),
        ("tcl9.0", b"TCL VALUE NUMBER".as_slice()),
        ("tcl9.1", b"TCL VALUE NUMBER".as_slice()),
    ] {
        for input in [b"BAD".as_slice(), b"end", b"1+2"] {
            let mut interp = native(profile);
            assert_eq!(
                call(
                    &mut interp,
                    &[b"binary", b"encode", b"base64", b"-maxlen", input, b"A"]
                ),
                Code::Error
            );
            assert_eq!(interp.error_code(), primitive, "{profile}/{input:?}");
            assert!(interp.result_bytes().starts_with(b"expected integer"));
            assert!(interp.native_execution_refusal().is_none());
        }
        let mut interp = native(profile);
        assert_eq!(
            call(
                &mut interp,
                &[b"binary", b"encode", b"base64", b"-maxlen", b"-1", b"A"]
            ),
            Code::Error
        );
        assert_eq!(interp.error_code(), b"TCL BINARY ENCODE LINE_LENGTH");
        assert_eq!(interp.result_bytes(), b"line length out of range");
        assert!(interp.native_execution_refusal().is_none());
    }
}
#[test]
fn original_binary_base64_wide_and_uuencode_int_remain_distinct() {
    // naming.numeric.io-debug-public-conversion-and-side-effect-order

    for profile in ["tcl9.0", "tcl9.1"] {
        let mut interp = native(profile);
        assert_eq!(
            call(
                &mut interp,
                &[
                    b"binary",
                    b"encode",
                    b"base64",
                    b"-maxlen",
                    b"2147483648",
                    b"A"
                ]
            ),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"QQ==");
        assert!(interp.native_execution_refusal().is_none());
        let mut interp = native(profile);
        assert_eq!(
            call(
                &mut interp,
                &[
                    b"binary",
                    b"encode",
                    b"uuencode",
                    b"-maxlen",
                    b"2147483648",
                    b"A"
                ]
            ),
            Code::Error
        );
        assert_ne!(interp.error_code(), b"TCL BINARY ENCODE LINE_LENGTH");
        assert!(interp.native_execution_refusal().is_none());
    }
}
thread_local! { static CALLS:Cell<usize>=const{Cell::new(0)}; }
extern "C" fn updater(value: *mut TclObj) {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut interp = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    let original = interp.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("original-binary-getter".to_owned(), "1.0".to_owned())];
    interp.pin_context(&changed).unwrap();
    interp.pin_context(&original).unwrap();
    unsafe { obj::set_native_updater_string_rep(value, b"2", false) };
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalBinaryNumeric".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(updater),
    set_from_any_proc: None,
};
#[test]
fn original_binary_numeric_updater_withdrawal_stops_before_data_and_guest_publication() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual extension updater ABA; no external Native updater chronology claim.
    let mut interp = native("tcl9.0");
    let fixed: Vec<_> = [b"binary".as_slice(), b"encode", b"base64", b"-maxlen"]
        .into_iter()
        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
        .collect();
    let numeric = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
    let data = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
    let mut argv: Vec<_> = fixed.iter().map(obj::Owned::as_ptr).collect();
    argv.extend([numeric.as_ptr(), data.as_ptr()]);
    CALLS.with(|calls| calls.set(0));
    interp.set_result_bytes(b"PRIOR");
    let prior = interp.get_obj_result();
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
    assert_eq!(binary_cmd(&mut interp, &argv), Code::Error);
    assert_eq!(CALLS.with(Cell::get), 1);
    assert!(!obj::has_string_rep(data.as_ptr()));
    assert_eq!(interp.get_obj_result(), prior);
    assert_eq!(obj::native_scalar_cache(numeric.as_ptr()).unwrap(), None);
    assert_eq!(unsafe { (*numeric.as_ptr()).ref_count }, 1);
    assert!(interp.native_execution_refusal().is_some());
    crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
}
