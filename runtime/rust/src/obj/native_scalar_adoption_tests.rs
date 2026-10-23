// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls of the original scalar retirement/adoption boundary.
use super::*;
use crate::interp::{native_operation_currency::NativeOperationScope, Interp};
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{
    raw_string::NativeValueAccessRefusal,
    scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FREE_CALLS: Cell<usize> = const { Cell::new(0) };
}
fn native() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
extern "C" fn free_original(value: *mut TclObj) {
    let index = FREE_CALLS.with(|calls| {
        let index = calls.get();
        calls.set(index + 1);
        index
    });
    if index != 0 {
        return;
    }
    if MODE.with(Cell::get) == 1 {
        let mut interp = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        interp.refuse_host_command("actual original scalar primary retirement");
    } else if MODE.with(Cell::get) == 2 {
        change_type(value, &TCL_INT_TYPE, 777);
    }
}
static ORIGINAL: TclObjType = TclObjType {
    name: c"originalScalarRetirement".as_ptr(),
    free_int_rep_proc: Some(free_original),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn original() -> Owned {
    let value = Owned::fresh(alloc_typed(&ORIGINAL, 0));
    unsafe { set_native_updater_string_rep(value.as_ptr(), b"2", false) };
    value
}
fn getter(
    interp: &Interp,
    value: *mut TclObj,
    scope: &NativeOperationScope,
) -> Result<NativeScalarGetterValue, ValueError> {
    let host = interp.host();
    let actual = host.numeric_environment();
    scope.currency().ensure_current_or_refuse().unwrap();
    crate::typed_value::native_scalar_getter_with_environment_and_currency(
        value,
        interp.native_invocation_dialect(),
        NativeScalarGetterKind::Int,
        actual,
        Some(scope.currency()),
    )
}
#[test]
fn original_scalar_old_primary_refusal_settles_vacancy_before_incoming_cache() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Real Runtime extension free callback; no external native free chronology claim.
    let mut interp = native();
    let value = original();
    let count = unsafe { (*value.as_ptr()).ref_count };
    let scope = NativeOperationScope::enter(&interp).unwrap();
    interp.set_result_bytes(b"PRIOR");
    let prior = interp.get_obj_result();
    MODE.with(|m| m.set(1));
    FREE_CALLS.with(|m| m.set(0));
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
    assert!(getter(&interp, value.as_ptr(), &scope).is_err());
    let first = interp.native_execution_refusal().unwrap();
    assert!(
        matches!(first,NativeExecutionError::HostCommandRefusal(ref reason) if reason.reason=="actual original scalar primary retirement")
    );
    assert_eq!(FREE_CALLS.with(Cell::get), 1);
    assert!(obj_type_ptr(value.as_ptr()).is_null());
    assert_eq!(native_scalar_cache(value.as_ptr()).unwrap(), None);
    assert_eq!(bytes_of(value.as_ptr()), b"2");
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, count);
    assert_eq!(interp.get_obj_result(), prior);
    assert_eq!(
        scope.currency().ensure_current_or_refuse().unwrap_err(),
        first
    );
    drop(value);
    assert_eq!(FREE_CALLS.with(Cell::get), 1);
    crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
}
#[test]
fn original_scalar_reentrant_publication_keeps_its_real_cache_and_refuses_outer_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut interp = native();
    let value = original();
    let scope = NativeOperationScope::enter(&interp).unwrap();
    MODE.with(|m| m.set(2));
    FREE_CALLS.with(|m| m.set(0));
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
    assert!(getter(&interp, value.as_ptr(), &scope).is_err());
    assert_eq!(
        native_scalar_cache(value.as_ptr()).unwrap(),
        Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Int(777)
        ))
    );
    let expected = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable(
            "changed native primary publication slot",
        ),
    );
    assert_eq!(interp.native_execution_refusal(), Some(expected.clone()));
    assert_eq!(
        scope.currency().ensure_current_or_refuse().unwrap_err(),
        expected
    );
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
}
#[test]
fn original_scalar_resident_cache_preserves_absent_string_and_native_role() {
    // naming.embedding.original-host-publication-and-fact-transport

    let interp = native();
    let value = Owned::fresh(new_wide_int_obj(7));
    let scope = NativeOperationScope::enter(&interp).unwrap();
    assert!(!has_string_rep(value.as_ptr()));
    let count = unsafe { (*value.as_ptr()).ref_count };
    assert_eq!(
        getter(&interp, value.as_ptr(), &scope).unwrap(),
        NativeScalarGetterValue::Wide(7)
    );
    assert!(!has_string_rep(value.as_ptr()));
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, count);
    assert!(interp.native_execution_refusal().is_none());
}
#[test]
fn original_primary_epoch_overflow_stays_unavailable_without_free_or_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    let value = original();
    auxiliary(value.as_ptr()).primary_epoch.set(Some(u64::MAX));
    MODE.with(|m| m.set(0));
    FREE_CALLS.with(|m| m.set(0));
    let mut current = || Ok::<(), ValueError>(());
    for _ in 0..2 {
        assert_eq!(
            change_type_with_stage_check(value.as_ptr(), &TCL_INT_TYPE, 2, &mut current),
            Err(ValueError::CommandProtocolUnavailable(
                "native primary publication epoch"
            ))
        );
        assert_eq!(FREE_CALLS.with(Cell::get), 0);
        assert!(core::ptr::eq(obj_type_ptr(value.as_ptr()), &ORIGINAL));
        assert_eq!(auxiliary(value.as_ptr()).primary_epoch.get(), None);
    }
    drop(value);
    assert_eq!(FREE_CALLS.with(Cell::get), 1);
}
struct RefusingAuxiliary;
impl Drop for RefusingAuxiliary {
    fn drop(&mut self) {
        let mut interp = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        interp.refuse_host_command("actual original scalar auxiliary retirement");
    }
}
#[test]
fn original_scalar_auxiliary_retirement_refuses_after_old_free_before_cache_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut interp = native();
    let value = original();
    let scope = NativeOperationScope::enter(&interp).unwrap();
    set_allocation_cache(value.as_ptr(), std::rc::Rc::new(RefusingAuxiliary));
    MODE.with(|m| m.set(0));
    FREE_CALLS.with(|m| m.set(0));
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
    assert!(getter(&interp, value.as_ptr(), &scope).is_err());
    assert_eq!(FREE_CALLS.with(Cell::get), 1);
    assert!(obj_type_ptr(value.as_ptr()).is_null());
    assert_eq!(native_scalar_cache(value.as_ptr()).unwrap(), None);
    assert!(
        matches!(interp.native_execution_refusal(),Some(NativeExecutionError::HostCommandRefusal(reason)) if reason.reason=="actual original scalar auxiliary retirement")
    );
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
}

struct JimNumericHost {
    actual: std::rc::Rc<dyn tcl_platform::Host>,
    interpreter: std::cell::RefCell<Option<Interp>>,
    withdraw: Cell<bool>,
    state_calls: Cell<usize>,
    later_calls: Cell<usize>,
}
impl tcl_platform::Host for JimNumericHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        Some(self)
    }
}
impl JimNumericHost {
    fn actual(&self) -> &dyn tcl_platform::NumericEnvironment {
        self.actual.numeric_environment().unwrap()
    }
}
impl tcl_platform::NumericEnvironment for JimNumericHost {
    fn c_integer_abi(
        &self,
    ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable> {
        self.actual().c_integer_abi()
    }
    fn state(
        &self,
    ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable> {
        self.state_calls.set(self.state_calls.get() + 1);
        if self.withdraw.get() {
            self.interpreter
                .borrow()
                .as_ref()
                .unwrap()
                .clone()
                .refuse_host_command("actual retained Jim numeric state");
        }
        self.actual().state()
    }
    fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
        self.later_calls.set(self.later_calls.get() + 1);
        self.actual().reset()
    }
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.later_calls.set(self.later_calls.get() + 1);
        self.actual().unsigned_c84(input, offset, long)
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.later_calls.set(self.later_calls.get() + 1);
        self.actual().unsigned(input, offset, base)
    }
    fn double(
        &self,
        input: &[u8],
        reset_errno: bool,
    ) -> Result<tcl_platform::DoubleNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.later_calls.set(self.later_calls.get() + 1);
        self.actual().double(input, reset_errno)
    }
}
#[test]
fn original_jim_context_numeric_workers_share_header_currency_and_stop_after_first_host_state() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual selected Jim context/Host software stages; not a native worker chronology oracle.
    for withdraw in [false, true] {
        let host = std::rc::Rc::new(JimNumericHost {
            actual: crate::interp::default_host(),
            interpreter: std::cell::RefCell::new(None),
            withdraw: Cell::new(false),
            state_calls: Cell::new(0),
            later_calls: Cell::new(0),
        });
        let interp = Interp::with_native_core(
            host.clone(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        *host.interpreter.borrow_mut() = Some(interp.clone());
        let context = interp.native_jim_object_context().unwrap();
        let value = Owned::fresh(new_string_bytes(b"17"));
        crate::native_source::bind_context(value.as_ptr(), &context).unwrap();
        let scope = NativeOperationScope::enter(&interp).unwrap();
        host.withdraw.set(withdraw);
        host.state_calls.set(0);
        host.later_calls.set(0);
        let result = crate::typed_value::native_scalar_getter_with_environment_and_currency(
            value.as_ptr(),
            interp.native_invocation_dialect(),
            NativeScalarGetterKind::Wide,
            None,
            Some(scope.currency()),
        );
        if withdraw {
            assert!(result.is_err());
            assert_eq!(host.state_calls.get(), 1);
            assert_eq!(host.later_calls.get(), 0);
            assert_eq!(native_scalar_cache(value.as_ptr()).unwrap(), None);
            assert!(
                matches!(interp.native_execution_refusal(),Some(NativeExecutionError::HostCommandRefusal(first)) if first.reason=="actual retained Jim numeric state")
            );
        } else {
            assert_eq!(result.unwrap(), NativeScalarGetterValue::Wide(17));
            assert!(host.state_calls.get() > 0);
            assert!(host.later_calls.get() > 0);
            assert!(interp.native_execution_refusal().is_none());
        }
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        host.interpreter.borrow_mut().take();
    }
}

struct ForeignNumericEnvironment(Cell<usize>);
impl tcl_platform::NumericEnvironment for ForeignNumericEnvironment {
    fn c_integer_abi(
        &self,
    ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable> {
        self.0.set(self.0.get() + 1);
        Err(tcl_platform::NumericEnvironmentUnavailable::Target)
    }
    fn state(
        &self,
    ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable> {
        self.0.set(self.0.get() + 1);
        Err(tcl_platform::NumericEnvironmentUnavailable::Target)
    }
    fn unsigned(
        &self,
        _: &[u8],
        _: usize,
        _: u32,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.0.set(self.0.get() + 1);
        Err(tcl_platform::NumericEnvironmentUnavailable::Target)
    }
    fn double(
        &self,
        _: &[u8],
        _: bool,
    ) -> Result<tcl_platform::DoubleNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.0.set(self.0.get() + 1);
        Err(tcl_platform::NumericEnvironmentUnavailable::Target)
    }
}
#[test]
fn original_jim_context_owns_all_scalar_workers_even_when_an_outer_environment_is_supplied() {
    // naming.embedding.original-host-publication-and-fact-transport

    for withdraw in [false, true] {
        let host = std::rc::Rc::new(JimNumericHost {
            actual: crate::interp::default_host(),
            interpreter: std::cell::RefCell::new(None),
            withdraw: Cell::new(false),
            state_calls: Cell::new(0),
            later_calls: Cell::new(0),
        });
        let interp = Interp::with_native_core(
            host.clone(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        *host.interpreter.borrow_mut() = Some(interp.clone());
        let context = interp.native_jim_object_context().unwrap();
        let value = Owned::fresh(new_string_bytes(b"17"));
        crate::native_source::bind_context(value.as_ptr(), &context).unwrap();
        let scope = NativeOperationScope::enter(&interp).unwrap();
        host.withdraw.set(withdraw);
        host.state_calls.set(0);
        host.later_calls.set(0);
        let foreign = ForeignNumericEnvironment(Cell::new(0));
        let result = crate::typed_value::native_scalar_getter_with_environment_and_currency(
            value.as_ptr(),
            interp.native_invocation_dialect(),
            NativeScalarGetterKind::Wide,
            Some(&foreign),
            Some(scope.currency()),
        );
        assert_eq!(foreign.0.get(), 0);
        if withdraw {
            assert!(result.is_err());
            assert_eq!(host.state_calls.get(), 1);
            assert_eq!(host.later_calls.get(), 0);
            assert_eq!(native_scalar_cache(value.as_ptr()).unwrap(), None);
            assert!(
                matches!(interp.native_execution_refusal(),Some(NativeExecutionError::HostCommandRefusal(first)) if first.reason=="actual retained Jim numeric state")
            );
        } else {
            assert_eq!(result.unwrap(), NativeScalarGetterValue::Wide(17));
            assert!(host.state_calls.get() > 0);
            assert!(host.later_calls.get() > 0);
            assert!(interp.native_execution_refusal().is_none());
        }
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        host.interpreter.borrow_mut().take();
    }
}

struct QueryCountingJimHost {
    actual: std::rc::Rc<dyn tcl_platform::Host>,
    queries: Cell<usize>,
}
impl tcl_platform::Host for QueryCountingJimHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.queries.set(self.queries.get() + 1);
        self.actual.numeric_environment()
    }
}
#[test]
fn original_jim_cache_and_boolean_word_keep_unreached_environment_unqueried() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Query counters observe software Host acquisition, not physical Jim worker chronology.
    let host = std::rc::Rc::new(QueryCountingJimHost {
        actual: crate::interp::default_host(),
        queries: Cell::new(0),
    });
    let interp = Interp::with_native_core(
        host.clone(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let context = interp.native_jim_object_context().unwrap();
    let cached = Owned::fresh(new_double_obj(1.25));
    let word = Owned::fresh(new_string_bytes(b"true"));
    for value in [cached.as_ptr(), word.as_ptr()] {
        crate::native_source::bind_context(value, &context).unwrap();
    }
    let scope = NativeOperationScope::enter(&interp).unwrap();
    host.queries.set(0);
    let cache = native_scalar_cache(cached.as_ptr()).unwrap();
    let result = crate::typed_value::native_scalar_getter_with_environment_and_currency(
        cached.as_ptr(),
        interp.native_invocation_dialect(),
        NativeScalarGetterKind::Double,
        None,
        Some(scope.currency()),
    )
    .unwrap();
    assert_eq!(result, NativeScalarGetterValue::Double(1.25));
    assert_eq!(host.queries.get(), 0);
    assert_eq!(native_scalar_cache(cached.as_ptr()).unwrap(), cache);
    let result = crate::typed_value::native_scalar_getter_with_environment_and_currency(
        word.as_ptr(),
        interp.native_invocation_dialect(),
        NativeScalarGetterKind::Boolean,
        None,
        Some(scope.currency()),
    )
    .unwrap();
    assert_eq!(
        result,
        NativeScalarGetterValue::Boolean(
            tcl_syntax::scalar_getter::NativeBooleanGetterValue::normalized(true)
        )
    );
    assert_eq!(host.queries.get(), 0);
    assert_eq!(unsafe { (*cached.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*word.as_ptr()).ref_count }, 1);
    assert!(interp.native_execution_refusal().is_none());
}
