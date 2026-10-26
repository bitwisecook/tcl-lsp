// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for actual ValueOps coercion entry and reached effects.
//! These do not establish external C/Jim getter or expression equivalence.

use super::*;
use crate::interp::WeakInterp;
use std::cell::{Cell, RefCell};
use tcl_platform::{Capabilities, Clock, Env, Host, NumericEnvironment, StdIo};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

fn original(environment: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::resolve_environment(environment).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

fn leak_free(body: impl FnOnce()) {
    crate::counters::reset();
    body();
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

fn stale() -> NativeExecutionError {
    NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation").into()
}

fn change_and_restore(interpreter: &mut Interp) {
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("valueops-original-coercion".into(), "1.0".into())];
    interpreter.pin_context(&changed).unwrap();
    interpreter.pin_context(&original).unwrap();
}

fn coerce(interpreter: &mut Interp, value: *mut TclObj, adapter: u8) -> Result<(), ValueError> {
    match adapter {
        0 => ValueOps::as_int(interpreter, &value).map(|_| ()),
        1 => ValueOps::as_double(interpreter, &value).map(|_| ()),
        2 => ValueOps::as_bool(interpreter, &value).map(|_| ()),
        3 => ValueOps::string_compare_length(interpreter, &value).map(|_| ()),
        #[cfg(have_tommath)]
        4 => {
            let syntax = interpreter.native_invocation_dialect().numbers;
            ValueOps::integer_magnitude(interpreter, &value, Radix::Dec, syntax).map(|_| ())
        }
        _ => unreachable!("fixed original ValueOps adapter"),
    }
}

fn adapters() -> std::ops::Range<u8> {
    0..if cfg!(have_tommath) { 5 } else { 4 }
}

thread_local! {
    static UPDATER_OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static UPDATER_HOST: Cell<bool> = const { Cell::new(false) };
    static UPDATER_CALLS: Cell<usize> = const { Cell::new(0) };
}

extern "C" fn original_updater(value: *mut TclObj) {
    UPDATER_CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = UPDATER_OWNER
        .with(|owner| owner.borrow().as_ref().unwrap().upgrade())
        .expect("original updater retains its actual interpreter");
    if UPDATER_HOST.with(Cell::get) {
        interpreter.refuse_host_command("original ValueOps updater first refusal");
    } else {
        change_and_restore(&mut interpreter);
    }
    // SAFETY: this reached descriptor owns the original live header.
    unsafe { obj::set_native_updater_string_rep(value, b"17", false) };
}

static ORIGINAL_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalValueOpsCoercion".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(original_updater),
    set_from_any_proc: None,
};

#[test]
fn actual_valueops_primitives_keep_selected_cache_and_native_reference_roles() {
    leak_free(|| {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut interpreter = original(environment);
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            let integer = obj::Owned::fresh(obj::new_string_bytes(b"0x10"));
            let double = obj::Owned::fresh(obj::new_string_bytes(b"1.5"));
            assert_eq!(
                ValueOps::as_int(&mut interpreter, &integer.as_ptr()),
                Ok(16)
            );
            assert_eq!(
                ValueOps::as_double(&mut interpreter, &double.as_ptr()),
                Ok(1.5)
            );
            assert_eq!(obj::bytes_of(integer.as_ptr()), b"0x10");
            assert_eq!(obj::bytes_of(double.as_ptr()), b"1.5");
            assert!(obj::native_scalar_cache(integer.as_ptr())
                .unwrap()
                .is_some());
            assert!(obj::native_scalar_cache(double.as_ptr()).unwrap().is_some());
            for number in [17, 4_294_967_296] {
                let boolean = obj::Owned::fresh(obj::new_wide_int_obj(number));
                assert_eq!(
                    ValueOps::as_bool(&mut interpreter, &boolean.as_ptr()),
                    Ok(environment != "jim" || number == 17),
                );
                assert_eq!(unsafe { (*boolean.as_ptr()).ref_count }, 1);
                // Actual C84 SetBooleanFromAny reaches String before its
                // cached numerical shortcut. Modern C/Jim cached integers do not.
                assert_eq!(
                    obj::has_string_rep(boolean.as_ptr()),
                    environment == "tcl8.4",
                    "{environment}/{number}",
                );
                if environment == "tcl8.4" {
                    assert_eq!(
                        obj::bytes_of(boolean.as_ptr()),
                        number.to_string().as_bytes()
                    );
                }
            }
            assert_eq!(unsafe { (*integer.as_ptr()).ref_count }, 1);
            assert_eq!(unsafe { (*double.as_ptr()).ref_count }, 1);
            assert_eq!(interpreter.get_obj_result(), prior);
            assert_eq!(interpreter.result_bytes(), b"PRIOR");
            assert!(interpreter.native_execution_refusal().is_none());
        }
        // Allocation protection does not promote a live native rc0 argument.
        let mut interpreter = original("tcl8.6");
        let borrowed = obj::new_string_bytes(b"17");
        assert_eq!(unsafe { (*borrowed).ref_count }, 0);
        assert_eq!(ValueOps::as_int(&mut interpreter, &borrowed), Ok(17));
        assert_eq!(unsafe { (*borrowed).ref_count }, 0);
        crate::interp::drop_fresh(borrowed);
    });
}

#[test]
fn actual_valueops_updaters_stop_before_numeric_cache_and_later_guest_publication() {
    leak_free(|| {
        for adapter in adapters() {
            for host_first in [false, true] {
                let mut interpreter = original("tcl8.6");
                let original_context = interpreter.runtime_context();
                let original_profile = interpreter.dialect_profile();
                let value = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL_TYPE, 0));
                interpreter.set_result_bytes(b"PRIOR\0\xff");
                let prior = interpreter.get_obj_result();
                UPDATER_OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
                UPDATER_CALLS.with(|calls| calls.set(0));
                UPDATER_HOST.with(|host| host.set(host_first));
                let error = coerce(&mut interpreter, value.as_ptr(), adapter).unwrap_err();
                assert!(error.native_access_refusal().is_some());
                let first = interpreter.native_execution_refusal().unwrap();
                if host_first {
                    assert!(matches!(
                        &first,
                        NativeExecutionError::HostCommandRefusal(_)
                    ));
                } else {
                    assert_eq!(first, stale());
                }
                UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
                assert_eq!(interpreter.runtime_context(), original_context);
                assert!(std::ptr::eq(
                    interpreter.dialect_profile(),
                    original_profile
                ));
                assert_eq!(obj::bytes_of(value.as_ptr()), b"17");
                assert!(std::ptr::eq(
                    obj::obj_type_ptr(value.as_ptr()),
                    &ORIGINAL_TYPE
                ));
                assert!(obj::native_scalar_cache(value.as_ptr()).unwrap().is_none());
                assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
                // Actual command settlement observes the retained full first
                // cause, rather than publishing the ValueOps domain projection.
                assert_eq!(
                    integer_error(&mut interpreter, error),
                    crate::interp::Code::Error
                );
                assert_eq!(interpreter.native_execution_refusal(), Some(first.clone()));
                assert_eq!(interpreter.get_obj_result(), prior);
                assert_eq!(interpreter.result_bytes(), b"PRIOR\0\xff");
                crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
                let mut wide = -91;
                let mut double = -91.5;
                let mut boolean = -91;
                // SAFETY: actual caller-owned header and writable local outputs.
                unsafe {
                    assert_ne!(
                        crate::codegen_abi::tcl_value_get_wide_int(value.as_ptr(), &mut wide),
                        0
                    );
                    assert_ne!(
                        crate::codegen_abi::tcl_value_get_double(value.as_ptr(), &mut double),
                        0
                    );
                    assert_ne!(
                        crate::codegen_abi::tcl_value_get_bool(value.as_ptr(), &mut boolean),
                        0
                    );
                }
                assert_eq!((wide, double, boolean), (-91, -91.5, -91));
                assert_eq!(interpreter.native_execution_refusal(), Some(first));
                UPDATER_CALLS.with(|calls| assert_eq!(calls.get(), 1));
                crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
                UPDATER_OWNER.with(|owner| owner.borrow_mut().take());
            }
        }
    });
}

struct NumericHost {
    actual: Rc<dyn Host>,
    interpreter: RefCell<Option<WeakInterp>>,
    double_calls: Cell<usize>,
}
impl Host for NumericHost {
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
    fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
        Some(self)
    }
}
impl NumericHost {
    fn actual(&self) -> &dyn NumericEnvironment {
        self.actual
            .numeric_environment()
            .expect("actual native numeric environment")
    }
}
impl NumericEnvironment for NumericHost {
    fn c_integer_abi(
        &self,
    ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable> {
        self.actual().c_integer_abi()
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
        let mut interpreter = self
            .interpreter
            .borrow()
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap();
        change_and_restore(&mut interpreter);
        result
    }
}

#[test]
fn actual_valueops_numeric_host_change_and_restore_preserves_first_cause() {
    leak_free(|| {
        for environment in ["tcl8.4", "jim"] {
            let mut interpreter = original(environment);
            let host = Rc::new(NumericHost {
                actual: interpreter.host(),
                interpreter: RefCell::new(Some(interpreter.downgrade())),
                double_calls: Cell::new(0),
            });
            interpreter.set_host(host.clone());
            let value = obj::Owned::fresh(obj::new_string_bytes(b"1.5"));
            let before = obj::native_object_snapshot(value.as_ptr()).unwrap();
            let context = interpreter.runtime_context();
            interpreter.set_result_bytes(b"PRIOR");
            assert!(ValueOps::as_double(&mut interpreter, &value.as_ptr()).is_err());
            assert_eq!(host.double_calls.get(), 1);
            assert_eq!(interpreter.runtime_context(), context);
            assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
            assert_eq!(obj::native_object_snapshot(value.as_ptr()).unwrap(), before);
            assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
            assert_eq!(interpreter.result_bytes(), b"PRIOR");
            #[cfg(feature = "engine")]
            {
                // SAFETY: this original actual engine/header remains retained.
                assert_ne!(
                    unsafe {
                        crate::engine_abi::value_carriers::tcl_engine_bind_scalar_object(
                            &mut interpreter,
                            value.as_ptr(),
                        )
                    },
                    0
                );
                assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
                assert_eq!(host.double_calls.get(), 1);
            }
        }
    });
}

#[test]
fn actual_valueops_number_classification_and_guest_errors_keep_their_separate_owners() {
    leak_free(|| {
        let mut interpreter = original("tcl8.6");
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let integer = obj::Owned::fresh(obj::new_string_bytes(b"17"));
        assert_eq!(
            ValueOps::string_compare_length(&mut interpreter, &integer.as_ptr()),
            Ok(Some(17))
        );
        // The selected Number preparation remains independent of a primitive
        // CAPI cache producer; this modern source parse does not install one.
        assert!(obj::native_scalar_cache(integer.as_ptr())
            .unwrap()
            .is_none());
        assert_eq!(unsafe { (*integer.as_ptr()).ref_count }, 1);
        let bad = obj::Owned::fresh(obj::new_string_bytes(b"bad\0operand"));
        assert_eq!(
            ValueOps::string_compare_length(&mut interpreter, &bad.as_ptr()),
            Err(ValueError::NotIntegerBytes(b"bad\0operand".to_vec()))
        );
        assert!(matches!(
            ValueOps::as_int(&mut interpreter, &bad.as_ptr()),
            Err(ValueError::NativeScalarGetter(_))
        ));
        assert!(obj::native_scalar_cache(bad.as_ptr()).unwrap().is_none());
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(interpreter.result_bytes(), b"PRIOR");
    });
}

#[test]
#[cfg(have_tommath)]
fn actual_valueops_magnitude_cannot_reparse_rejected_original_with_caller_grammar() {
    leak_free(|| {
        let mut interpreter = original("tcl8.4");
        let modern = original("tcl9.0");
        let original = obj::Owned::fresh(obj::new_string_bytes(b"0b10"));
        let before = obj::native_object_snapshot(original.as_ptr()).unwrap();
        assert_eq!(
            ValueOps::integer_magnitude(
                &mut interpreter,
                &original.as_ptr(),
                Radix::Dec,
                modern.native_invocation_dialect().numbers
            ),
            Err(ValueError::NotIntegerBytes(b"0b10".to_vec())),
        );
        assert_eq!(
            obj::native_object_snapshot(original.as_ptr()).unwrap(),
            before
        );
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        assert!(interpreter.native_execution_refusal().is_none());
        let accepted = obj::Owned::fresh(obj::new_string_bytes(b"0x10"));
        let magnitude = ValueOps::integer_magnitude(
            &mut interpreter,
            &accepted.as_ptr(),
            Radix::Hex,
            modern.native_invocation_dialect().numbers,
        )
        .unwrap();
        assert!(!magnitude.negative);
        assert_eq!(magnitude.digits, "10");
        assert_eq!(obj::bytes_of(accepted.as_ptr()), b"0x10");
        assert!(obj::native_scalar_cache(accepted.as_ptr())
            .unwrap()
            .is_none());
        assert_eq!(unsafe { (*accepted.as_ptr()).ref_count }, 1);
    });
}

#[path = "native_unicode_getter_entry_tests.rs"]
mod native_unicode_getter_entry_tests;
