// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Boxed scalars: ownership, typed reads under the host's numeric environment, cached representations and word joining.

use super::*;

#[test]
fn native_wide_int_value_is_owned_and_keeps_its_internal_rep_after_shimmering() {
    leak_free(|| unsafe {
        let value = tcl_value_new_wide_int(i64::MIN);
        assert!(!value.is_null());
        assert_eq!((*value).ref_count, 1, "generated code owns the result");
        assert!(std::ptr::eq((*value).type_ptr, &obj::TCL_INT_TYPE));
        assert!((*value).bytes.is_null(), "wide integers begin without text");
        assert_eq!(obj_bytes(value), i64::MIN.to_string().as_bytes());
        assert!(
            std::ptr::eq((*value).type_ptr, &obj::TCL_INT_TYPE),
            "string materialisation must retain the wide internal representation"
        );
        assert_eq!((*value).internal_rep as i64, i64::MIN);
        tcl_obj_release(value);
    });
}

/// Read one typed scalar through the C ABI, returning the status and the
/// value written (untouched storage on a refusal).
unsafe fn get_wide(value: *mut TclObj) -> (i32, i64) {
    let mut out: i64 = i64::MIN;
    // SAFETY: `value` is live and `out` is local aligned storage.
    let status = unsafe { tcl_value_get_wide_int(value, &mut out) };
    (status, out)
}

unsafe fn get_double(value: *mut TclObj) -> (i32, f64) {
    let mut out: f64 = f64::NAN;
    // SAFETY: `value` is live and `out` is local aligned storage.
    let status = unsafe { tcl_value_get_double(value, &mut out) };
    (status, out)
}

unsafe fn get_bool(value: *mut TclObj) -> (i32, i32) {
    let mut out: i32 = -1;
    // SAFETY: `value` is live and `out` is local aligned storage.
    let status = unsafe { tcl_value_get_bool(value, &mut out) };
    (status, out)
}

#[test]
fn boxed_primitive_reads_share_actual_environment_and_raw_boolean_projection() {
    // Software adapters: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    leak_free(|| unsafe {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut interp = scalar_interpreter(environment);
            tcl_runtime_set_current_interp(&mut interp);
            let wide = owned_word(b"0x10");
            let double = owned_word(b"1.5");
            assert_eq!(get_wide(wide), (TCL_VALUE_GET_OK, 16), "{environment}");
            assert_eq!(get_double(double), (TCL_VALUE_GET_OK, 1.5), "{environment}");
            assert_eq!(obj_bytes(wide), b"0x10");
            assert_eq!(obj_bytes(double), b"1.5");
            release_words(&[wide, double]);
            for number in [17, 4_294_967_296] {
                let primitive = obj::Owned::fresh(obj::new_wide_int_obj(number));
                let boxed = obj::Owned::fresh(obj::new_wide_int_obj(number));
                let mut raw = 777;
                assert_eq!(
                    crate::capi::Tcl_GetBooleanFromObj(&mut interp, primitive.as_ptr(), &mut raw,),
                    0
                );
                let expected = if environment == "jim" {
                    if number == 17 {
                        17
                    } else {
                        0
                    }
                } else {
                    1
                };
                assert_eq!(raw, expected, "{environment}, {number}");
                assert_eq!(
                    get_bool(boxed.as_ptr()),
                    (TCL_VALUE_GET_OK, i32::from(raw != 0))
                );
                assert!(!interp.host_refusal_pending());
            }
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

struct ScalarNumericHost {
    actual: Rc<dyn Host>,
    interpreter: RefCell<Option<Interp>>,
    double_calls: std::cell::Cell<usize>,
    abi_calls: std::cell::Cell<usize>,
    change_abi_context: std::cell::Cell<bool>,
    abi_query_fails: std::cell::Cell<bool>,
    abi_available: bool,
    change_context: bool,
}

impl Host for ScalarNumericHost {
    fn capabilities(&self) -> Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.abi_available.then_some(self)
    }
}

impl ScalarNumericHost {
    pub(super) fn actual(&self) -> &dyn tcl_platform::NumericEnvironment {
        self.actual
            .numeric_environment()
            .expect("actual native numeric environment")
    }
    fn change_and_restore_context(&self) {
        let mut interp = self.interpreter.borrow().as_ref().unwrap().clone();
        let original = interp.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("boxed-scalar-currency".to_owned(), "1.0".to_owned())];
        interp.pin_context(&changed).unwrap();
        interp.pin_context(&original).unwrap();
    }
}

impl tcl_platform::NumericEnvironment for ScalarNumericHost {
    fn c_integer_abi(
        &self,
    ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable> {
        let result = self.actual().c_integer_abi();
        self.abi_calls.set(self.abi_calls.get() + 1);
        if self.change_abi_context.get() {
            self.change_and_restore_context();
        }
        if self.abi_query_fails.get() {
            Err(tcl_platform::NumericEnvironmentUnavailable::Target)
        } else {
            result
        }
    }
    fn state(
        &self,
    ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable> {
        self.actual().state()
    }
    fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
        self.actual().reset()
    }
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.actual().unsigned_c84(input, offset, long)
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.actual().unsigned(input, offset, base)
    }
    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<tcl_platform::SignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.actual().signed_long(input, base)
    }
    fn double(
        &self,
        input: &[u8],
        reset: bool,
    ) -> Result<tcl_platform::DoubleNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        let result = self.actual().double(input, reset);
        self.double_calls.set(self.double_calls.get() + 1);
        if self.change_context {
            self.change_and_restore_context();
        }
        result
    }
}

fn scalar_host(
    interp: &Interp,
    abi_available: bool,
    change_context: bool,
) -> Rc<ScalarNumericHost> {
    let host = Rc::new(ScalarNumericHost {
        actual: interp.host(),
        interpreter: RefCell::new(None),
        double_calls: std::cell::Cell::new(0),
        abi_calls: std::cell::Cell::new(0),
        change_abi_context: std::cell::Cell::new(false),
        abi_query_fails: std::cell::Cell::new(false),
        abi_available,
        change_context,
    });
    interp.set_host(host.clone());
    host
}

#[test]
fn boxed_primitive_reads_keep_missing_abi_and_first_host_terminal() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    leak_free(|| unsafe {
        for first_host in [false, true] {
            let mut interp = scalar_interpreter("tcl8.4");
            let host = scalar_host(&interp, false, false);
            interp.set_result_bytes(b"PRIOR\0\xff");
            if first_host {
                interp.refuse_host_command("original first refusal");
            }
            let first = interp.native_execution_refusal();
            let value = owned_word(b"17");
            let before = obj::native_object_snapshot(value).unwrap();
            tcl_runtime_set_current_interp(&mut interp);
            assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
            let double = get_double(value);
            assert_eq!(double.0, TCL_VALUE_GET_ERROR);
            assert!(double.1.is_nan());
            assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
            assert_eq!(host.double_calls.get(), 0);
            assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
            assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
            assert!(interp.host_refusal_pending());
            if first_host {
                assert_eq!(interp.native_execution_refusal(), first);
            }
            release_words(&[value]);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[test]
fn boxed_primitive_reads_refuse_foreign_and_restored_engine_issuers() {
    // Software owner: naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    leak_free(|| unsafe {
        for foreign in [false, true] {
            let mut owner = scalar_interpreter("tcl8.6");
            let mut other = scalar_interpreter("tcl8.6");
            let value = owned_word(b"17");
            crate::capi::bind_scalar_getter_context(&owner, value).unwrap();
            let before = obj::native_object_snapshot(value).unwrap();
            let original = owner.runtime_context();
            if !foreign {
                let mut changed = original.clone();
                changed.packages = vec![("boxed-scalar-owner".to_owned(), "1.0".to_owned())];
                owner.pin_context(&changed).unwrap();
                owner.pin_context(&original).unwrap();
            }
            let selected = if foreign { &mut other } else { &mut owner };
            selected.set_result_bytes(b"UNCHANGED");
            tcl_runtime_set_current_interp(selected);
            assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
            assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
            assert!(selected.host_refusal_pending());
            assert_eq!(selected.result_bytes(), b"UNCHANGED");
            assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
            release_words(&[value]);
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

#[cfg(debug_assertions)]
#[test]
fn boxed_primitive_host_change_and_restore_stops_before_cache_and_output() {
    // Software currency: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|| unsafe {
        for environment in ["tcl8.4", "jim"] {
            let mut interp = scalar_interpreter(environment);
            let host = scalar_host(&interp, true, true);
            *host.interpreter.borrow_mut() = Some(interp.clone());
            let context = interp.runtime_context();
            let profile = interp.dialect_profile();
            interp.set_result_bytes(b"PRIOR\0\xff");
            let prior = interp.get_obj_result();
            let value = owned_word(b"1.5");
            let before = obj::native_object_snapshot(value).unwrap();
            tcl_runtime_set_current_interp(&mut interp);
            let outcome = get_double(value);
            assert_eq!(outcome.0, TCL_VALUE_GET_ERROR);
            assert!(outcome.1.is_nan());
            assert_eq!(host.double_calls.get(), 1, "{environment}");
            assert_eq!(interp.runtime_context(), context);
            assert!(std::ptr::eq(interp.dialect_profile(), profile));
            let first = interp.native_execution_refusal().unwrap();
            assert_eq!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation",
                    ),
                )
            );
            assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
            assert_eq!(interp.get_obj_result(), prior);
            assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
            assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
            assert_eq!(host.double_calls.get(), 1);
            assert_eq!(interp.native_execution_refusal(), Some(first));
            tcl_runtime_set_current_interp(ptr::null_mut());
            release_words(&[value]);
            host.interpreter.borrow_mut().take();
        }
    });
}

#[cfg(debug_assertions)]
#[test]
fn scalar_abi_query_change_and_restore_cannot_issue_or_refresh_a_receipt() {
    // Software currency: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|| unsafe {
        for retained in [false, true] {
            for query_fails in [false, true] {
                let mut interp = scalar_interpreter("tcl8.6");
                let host = scalar_host(&interp, true, false);
                *host.interpreter.borrow_mut() = Some(interp.clone());
                let value = owned_word(b"1.5");
                if retained {
                    crate::capi::bind_scalar_getter_context(&interp, value).unwrap();
                }
                let before = obj::native_object_snapshot(value).unwrap();
                let context = interp.runtime_context();
                let calls = host.abi_calls.get();
                host.change_abi_context.set(true);
                host.abi_query_fails.set(query_fails);
                interp.set_result_bytes(b"PRIOR");
                let error = crate::capi::probe_scalar_getter(
                    if retained { None } else { Some(&interp) },
                    value,
                    tcl_syntax::scalar_getter::NativeScalarGetterKind::Double,
                )
                .unwrap_err();
                let first = interp.native_execution_refusal().unwrap();
                assert!(
                    matches!(error, crate::capi::NativeScalarObjectAccessError::Execution(ref cause) if *cause == first)
                );
                assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "stale entered native operation",
                        ),
                    ));
                assert_eq!(host.abi_calls.get(), calls + 1);
                assert_eq!(host.double_calls.get(), 0);
                assert_eq!(interp.runtime_context(), context);
                assert_eq!(obj::native_object_snapshot(value).unwrap(), before);
                assert_eq!(interp.result_bytes(), b"PRIOR");
                if !retained {
                    assert!(obj::scalar_object_context(value).unwrap().is_none());
                }
                tcl_runtime_set_current_interp(&mut interp);
                let outcome = get_double(value);
                assert_eq!(outcome.0, TCL_VALUE_GET_ERROR);
                assert!(outcome.1.is_nan());
                assert_eq!(host.abi_calls.get(), calls + 1);
                assert_eq!(interp.native_execution_refusal(), Some(first));
                tcl_runtime_set_current_interp(ptr::null_mut());
                release_words(&[value]);
                host.interpreter.borrow_mut().take();
            }
        }
    });
}

#[cfg(debug_assertions)]
extern "C" fn scalar_context_changing_updater(value: *mut TclObj) {
    SCALAR_UPDATER_CALLS.with(|calls| calls.set(calls.get() + 1));
    // SAFETY: the actual caller retains this original interpreter for the
    // updater. The clone shares its real state without borrowing it.
    let mut interp = unsafe { current_interp().as_ref() }.unwrap().clone();
    let original = interp.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("boxed-scalar-updater".to_owned(), "1.0".to_owned())];
    interp.pin_context(&changed).unwrap();
    interp.pin_context(&original).unwrap();
    // SAFETY: this actual descriptor owns the live original header.
    unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
}

#[cfg(debug_assertions)]
static SCALAR_CONTEXT_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalScalarCurrency".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(scalar_context_changing_updater),
    set_from_any_proc: None,
};

#[cfg(debug_assertions)]
#[test]
fn boxed_primitive_updater_change_and_restore_keeps_only_reached_string_effects() {
    // Software stages: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|| unsafe {
        let mut interp = scalar_interpreter("tcl8.6");
        let original = obj::Owned::fresh(obj::alloc_typed(&SCALAR_CONTEXT_TYPE, 0));
        let value = original.as_ptr();
        let context = interp.runtime_context();
        let references = (*value).ref_count;
        interp.set_result_bytes(b"PRIOR");
        let result = interp.get_obj_result();
        SCALAR_UPDATER_CALLS.with(|calls| calls.set(0));
        tcl_runtime_set_current_interp(&mut interp);
        assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
        let first = interp.native_execution_refusal().unwrap();
        assert_eq!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation",
                ),
            )
        );
        SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(interp.runtime_context(), context);
        assert_eq!(obj::bytes_of(value), b"17");
        assert_eq!(obj::obj_type_ptr(value), &SCALAR_CONTEXT_TYPE as *const _);
        assert!(obj::native_scalar_cache(value).unwrap().is_none());
        assert_eq!((*value).ref_count, references);
        assert_eq!(interp.get_obj_result(), result);
        assert_eq!(interp.result_bytes(), b"PRIOR");
        assert_eq!(get_bool(value), (TCL_VALUE_GET_ERROR, -1));
        SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
        assert_eq!(interp.native_execution_refusal(), Some(first));
        tcl_runtime_set_current_interp(ptr::null_mut());
    });
}

#[cfg(debug_assertions)]
#[test]
fn entered_expression_number_updater_stops_before_cache_and_truth_output() {
    // Software stages: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|| unsafe {
        for production in [0, 1] {
            let mut interp = scalar_interpreter("tcl8.6");
            let original = obj::Owned::fresh(obj::alloc_typed(&SCALAR_CONTEXT_TYPE, 0));
            let value = original.as_ptr();
            let context = interp.runtime_context();
            let references = (*value).ref_count;
            interp.set_result_bytes(b"PRIOR");
            let result = interp.get_obj_result();
            SCALAR_UPDATER_CALLS.with(|calls| calls.set(0));
            tcl_runtime_set_current_interp(&mut interp);
            let mut out = -17;
            assert_eq!(
                tcl_value_get_expression_bool(value, production, &mut out),
                TCL_VALUE_GET_ERROR
            );
            assert_eq!(out, -17);
            let first = interp.native_execution_refusal().unwrap();
            assert_eq!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation",
                    ),
                )
            );
            SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
            assert_eq!(interp.runtime_context(), context);
            assert_eq!(obj::bytes_of(value), b"17");
            assert_eq!(obj::obj_type_ptr(value), &SCALAR_CONTEXT_TYPE as *const _);
            assert!(obj::native_scalar_cache(value).unwrap().is_none());
            assert_eq!((*value).ref_count, references);
            assert_eq!(interp.get_obj_result(), result);
            assert_eq!(interp.result_bytes(), b"PRIOR");
            assert_eq!(
                tcl_value_get_expression_bool(value, production, &mut out),
                TCL_VALUE_GET_ERROR
            );
            assert_eq!(out, -17);
            SCALAR_UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
            assert_eq!(interp.native_execution_refusal(), Some(first));
            tcl_runtime_set_current_interp(ptr::null_mut());
        }
    });
}

/// A successful typed read caches the parsed rep onto the object — C's
/// `TclParseNumber` write-back — and keeps the spelling, so a hot loop
/// parses once and `puts` still prints what the script wrote.
#[test]
fn typed_value_reads_cache_the_parsed_rep_and_keep_the_spelling() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        let decimal = owned_word(b"12");
        assert!((*decimal).type_ptr.is_null(), "starts as a plain string");
        assert_eq!(get_wide(decimal), (TCL_VALUE_GET_OK, 12));
        assert!(
            std::ptr::eq((*decimal).type_ptr, &obj::TCL_INT_TYPE),
            "the parsed integer rep is cached back onto the object"
        );
        assert_eq!(obj_bytes(decimal), b"12", "the spelling survives");

        // A radix spelling keeps its own text, exactly as in C Tcl.
        let hex = owned_word(b"0x10");
        assert_eq!(get_wide(hex), (TCL_VALUE_GET_OK, 16));
        assert!(std::ptr::eq((*hex).type_ptr, &obj::TCL_INT_TYPE));
        assert_eq!(obj_bytes(hex), b"0x10");

        let scientific = owned_word(b"1e3");
        assert_eq!(get_double(scientific), (TCL_VALUE_GET_OK, 1000.0));
        assert!(std::ptr::eq((*scientific).type_ptr, &obj::TCL_DOUBLE_TYPE));
        assert_eq!(obj_bytes(scientific), b"1e3");

        // Now that it carries a double rep, the integer read reports C's
        // double-branch wording and code (`tclObj.c`).
        assert_eq!(get_wide(scientific).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected integer but got \"1e3\""
        );
        assert_eq!((*interp).error_code(), b"TCL VALUE INTEGER");

        release_words(&[decimal, hex, scientific]);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// Beyond-wide integers and non-numbers report C's exact message and
/// `-errorcode` through the interpreter, leaving the out storage untouched.
#[test]
fn typed_value_reads_report_overflow_and_non_numbers_like_c() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        // `tclsh9.0`: `string repeat a [expr {2**200}]` →
        // "integer value too large to represent" /
        // ARITH IOVERFLOW {integer value too large to represent}.
        let huge = owned_word(b"99999999999999999999999");
        let (status, untouched) = get_wide(huge);
        assert_eq!(status, TCL_VALUE_GET_ERROR);
        assert_eq!(untouched, i64::MIN, "the out storage is left alone");
        assert_eq!(
            (*interp).result_bytes(),
            b"integer value too large to represent"
        );
        assert_eq!(
            (*interp).error_code(),
            b"ARITH IOVERFLOW {integer value too large to represent}"
        );
        // The same value still widens to a double.
        assert_eq!(get_double(huge), (TCL_VALUE_GET_OK, 1e23));

        let word = owned_word(b"abc");
        assert_eq!(get_wide(word).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected integer but got \"abc\""
        );
        assert_eq!((*interp).error_code(), b"TCL VALUE NUMBER");
        assert_eq!(get_double(word).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected floating-point number but got \"abc\""
        );
        assert!(
            (*word).type_ptr.is_null(),
            "a refused read caches nothing onto the object"
        );

        // C renders a well-formed multi-token value as `a list`.
        let listy = owned_word(b"a b");
        assert_eq!(get_bool(listy).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected boolean value but got a list"
        );

        release_words(&[huge, word, listy]);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The boolean read is the boolean-*context* acceptor over
/// `tcl_syntax::boolean`'s one word table: unique prefixes resolve, the
/// ambiguous `o` does not, and any number compares against zero.
#[test]
fn typed_value_boolean_read_uses_the_shared_word_table() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);

        for (spelling, expected) in [
            (&b"tru"[..], 1),
            (b"t", 1),
            (b"ye", 1),
            (b"YES", 1),
            (b"of", 0),
            (b"n", 0),
            (b"2", 1),
            (b"-1", 1),
            (b"0.0", 0),
            (b"0x0", 0),
        ] {
            let value = owned_word(spelling);
            assert_eq!(
                get_bool(value),
                (TCL_VALUE_GET_OK, expected),
                "{:?}",
                String::from_utf8_lossy(spelling)
            );
            release_words(&[value]);
        }

        // `on`/`off` share the prefix `o`, so tclsh rejects it.
        let ambiguous = owned_word(b"o");
        assert_eq!(get_bool(ambiguous).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"expected boolean value but got \"o\""
        );

        // `tclsh9.0`: `expr {NaN ? 1 : 0}` is a domain error, not truthy.
        let nan = owned_word(b"NaN");
        assert_eq!(get_bool(nan).0, TCL_VALUE_GET_ERROR);
        assert_eq!(
            (*interp).result_bytes(),
            b"floating point value is Not a Number"
        );
        assert_eq!(
            get_double(nan).0,
            TCL_VALUE_GET_ERROR,
            "the actual C9 primitive double read rejects cached NaN"
        );

        release_words(&[ambiguous, nan]);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

#[test]
fn actual_cached_getter_preserves_private_c85_code_and_exact_result_extent() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        (*interp).set_dialect_profile(crate::environment::profile_for_dialect("tcl8.5"));
        (*interp).set_error_state(b"KEEP CODE");
        let value = owned_word(b"1.5\0tail");
        assert_eq!(get_double(value), (TCL_VALUE_GET_OK, 1.5));
        assert_eq!(get_wide(value), (TCL_VALUE_GET_ERROR, i64::MIN));
        assert_eq!(
            (*interp).result_bytes(),
            b"expected integer but got \"1.5\0tail\""
        );
        assert_eq!((*interp).error_code(), b"KEEP CODE");
        assert!(core::ptr::eq(
            obj::obj_type_ptr(value),
            &obj::TCL_DOUBLE_TYPE
        ));
        assert!(!(*interp).host_refusal_pending());
        release_words(&[value]);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The native constructors produce owned, typed objects whose string rep is
/// materialised only on demand.
#[test]
fn native_double_and_boolean_values_are_owned_and_typed() {
    leak_free(|| unsafe {
        let double = tcl_value_new_double(1.5);
        assert_eq!((*double).ref_count, 1, "generated code owns the result");
        assert!(std::ptr::eq((*double).type_ptr, &obj::TCL_DOUBLE_TYPE));
        assert!((*double).bytes.is_null(), "doubles begin without text");
        assert_eq!(obj_bytes(double), b"1.5");
        assert!(
            std::ptr::eq((*double).type_ptr, &obj::TCL_DOUBLE_TYPE),
            "materialising the text keeps the double internal rep"
        );
        tcl_obj_release(double);

        // C's `Tcl_NewBooleanObj` normalises any non-zero input to 1.
        let truthy = tcl_value_new_bool(7);
        assert_eq!((*truthy).ref_count, 1);
        assert_eq!(obj_bytes(truthy), b"1");
        tcl_obj_release(truthy);

        let falsy = tcl_value_new_bool(0);
        assert_eq!(obj_bytes(falsy), b"0");
        tcl_obj_release(falsy);
    });
}

/// `tcl_codegen_word_concat` borrows its parts and returns one owned join.
#[test]
fn word_concat_borrows_its_parts() {
    leak_free(|| unsafe {
        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        let parts = [owned_word(b"hi "), owned_word(b"world")];
        let counts = parts.map(|part| (*part).ref_count);

        let joined = tcl_codegen_word_concat(parts.as_ptr(), 2);
        assert_eq!(obj_bytes(joined), b"hi world");
        assert_eq!((*joined).ref_count, 1);
        assert_eq!(parts.map(|part| (*part).ref_count), counts);
        tcl_obj_release(joined);

        let empty = tcl_codegen_word_concat(ptr::null(), 0);
        assert_eq!(obj_bytes(empty), b"");
        tcl_obj_release(empty);
        assert!(tcl_codegen_word_concat(ptr::null(), 1).is_null());

        release_words(&parts);
        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    });
}

/// The argv constructor transfers exactly one owned reference, which is
/// the reference the generated cleanup path releases after dispatch.
#[test]
fn owned_string_constructor_has_one_releasable_reference() {
    leak_free(|| unsafe {
        let word = tcl_obj_new_string_owned(b"word".as_ptr(), 4);
        assert_eq!((*word).ref_count, 1);
        assert_eq!(obj_bytes(word), b"word");
        tcl_obj_release(word);
    });
}
