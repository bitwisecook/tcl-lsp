// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual C84 Index result purpose is independent of a Wide operand descriptor.
use super::*;
use std::cell::Cell;
use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache, value::ValueOps};
struct CountingHost {
    actual: Rc<dyn tcl_platform::Host>,
    queries: Rc<Cell<usize>>,
    during_query: Option<fn(usize)>,
}
impl tcl_platform::Host for CountingHost {
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
        if let Some(callback) = self.during_query {
            callback(self.queries.get());
        }
        self.actual.numeric_environment()
    }
}
fn actual() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.4"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn wide(interpreter: &Interp) -> obj::Owned {
    let issuer = interpreter.native_scalar_object_issuer().unwrap();
    let original = obj::Owned::fresh(obj::new_obj());
    obj::bind_scalar_object_context(original.as_ptr(), issuer).unwrap();
    obj::adopt_native_scalar_cache(
        original.as_ptr(),
        NativeScalarCache::Number(Number::Int(42)),
        interpreter
            .native_invocation_dialect()
            .native_scalar_getter_protocol()
            .unwrap(),
    )
    .unwrap();
    obj::invalidate_string(original.as_ptr());
    original
}
#[test]
fn original_c84_index_integer_result_has_its_actual_long_descriptor_and_receiving_roles() {

    // Direct software constructors/receivers; no EvalObjv or Native count chronology.
    // A genuine Wide42 operand fits C84 search Int; the recorded Wide4294967296
    // IntWidthOverflow remains its independent unchanged primitive outcome.
    let mut interpreter = actual();
    let operation = interpreter.enter_index_operation().unwrap();
    let result = tcl_cmd_core::index::integer_result(&mut interpreter, &operation, -1).unwrap();
    let allocation = obj::NativeObjectLifetime::retain(result);
    assert!(core::ptr::eq(
        obj::obj_type_ptr(result),
        &obj::TCL_LONG84_TYPE
    ));
    assert!(!obj::has_string_rep(result));
    assert_eq!(unsafe { (*result).ref_count }, 0);
    let received = obj::Owned::fresh(ValueOps::new_list(&mut interpreter, vec![result]));
    assert_eq!(unsafe { (*result).ref_count }, 1);
    drop(received);
    assert!(obj::check_native_liveness(allocation.as_ptr()).is_err());
    let operand = wide(&interpreter);
    let list = obj::Owned::fresh(ValueOps::new_list(&mut interpreter, vec![operand.as_ptr()]));
    let exact = obj::Owned::fresh(obj::new_string_bytes(b"-exact"));
    let integer = obj::Owned::fresh(obj::new_string_bytes(b"-integer"));
    let searched = tcl_cmd_core::lsearch::lsearch::<Interp, crate::cmd_regex::AreEngine>(
        &mut interpreter,
        &[
            exact.as_ptr(),
            integer.as_ptr(),
            list.as_ptr(),
            operand.as_ptr(),
        ],
        tcl_dialect::TclVersion::V8_4,
    )
    .unwrap_or_else(|error| {
        drop(error);
        panic!("valid original C84 integer result fixture");
    });
    assert_eq!(unsafe { (*searched).ref_count }, 0);
    let searched = obj::Owned::fresh(searched);
    assert_eq!(
        obj::native_scalar_cache(searched.as_ptr()).unwrap(),
        Some(NativeScalarCache::Tcl84Long(0))
    );
    assert!(core::ptr::eq(
        obj::obj_type_ptr(searched.as_ptr()),
        &obj::TCL_LONG84_TYPE
    ));
    assert!(!obj::has_string_rep(searched.as_ptr()));
    assert!(core::ptr::eq(
        obj::obj_type_ptr(operand.as_ptr()),
        &obj::TCL_WIDE84_TYPE
    ));
    assert!(!obj::has_string_rep(operand.as_ptr()));
    assert_eq!(unsafe { (*operand.as_ptr()).ref_count }, 2);
    let all = obj::Owned::fresh(obj::new_string_bytes(b"-all"));
    let searched = obj::Owned::fresh(
        tcl_cmd_core::lsearch::lsearch::<Interp, crate::cmd_regex::AreEngine>(
            &mut interpreter,
            &[
                all.as_ptr(),
                exact.as_ptr(),
                integer.as_ptr(),
                list.as_ptr(),
                operand.as_ptr(),
            ],
            tcl_dialect::TclVersion::V8_4,
        )
        .unwrap_or_else(|error| {
            drop(error);
            panic!("valid original C84 all integer result fixture");
        }),
    );
    let members = ValueOps::list_elements(&mut interpreter, &searched.as_ptr()).unwrap();
    assert_eq!(members.len(), 1);
    assert!(core::ptr::eq(
        obj::obj_type_ptr(members[0]),
        &obj::TCL_LONG84_TYPE
    ));
    assert_eq!(unsafe { (*members[0]).ref_count }, 1);
    assert!(!obj::has_string_rep(members[0]));
}
#[test]
fn original_c84_index_result_refuses_stale_foreign_and_overwidth_before_selected_construction() {

    for mode in [0, 1, 2, 3] {
        let mut interpreter = actual();
        let queries = Rc::new(Cell::new(0));
        interpreter.set_host(Rc::new(CountingHost {
            actual: interpreter.host(),
            queries: Rc::clone(&queries),
            during_query: None,
        }));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.result_obj();
        let operation = interpreter.enter_index_operation().unwrap();
        let mut other = actual();
        if mode == 1 {
            interpreter.refuse_host_command("original Index result first Host");
        } else if mode == 2 {
            let before = interpreter.runtime_context();
            let mut changed = before.clone();
            changed
                .packages
                .push(("original-index-result".to_owned(), "1".to_owned()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&before).unwrap();
        }
        let reached = if mode == 3 {
            tcl_cmd_core::index::integer_result(&mut other, &operation, 0)
        } else {
            tcl_cmd_core::index::integer_result(
                &mut interpreter,
                &operation,
                if mode == 0 {
                    i64::from(i32::MAX) + 1
                } else {
                    0
                },
            )
        };
        assert!(reached.is_err());
        assert_eq!(queries.get(), 0);
        assert_eq!(interpreter.result_obj(), prior);
        if mode != 3 {
            assert_eq!(
                reached.unwrap_err().native_execution_refusal(),
                interpreter.native_execution_refusal().as_ref()
            );
        }
    }
}

thread_local! {
    static SECOND_QUERY_OBJECTS: Cell<i64> = const { Cell::new(-1) };
}
fn stop_second_result(query: usize) {
    if query == 2 {
        SECOND_QUERY_OBJECTS.set(crate::counters::live_objs());
        let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        actual.refuse_host_command("original Index second result provider");
    }
}
struct Current;
impl Drop for Current {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
    }
}
#[test]
fn original_c84_all_search_retires_reached_integer_prefix_after_provider_callback_refusal() {

    // Real provided ABI-entry callback; source-only software cleanup counts.
    let mut interpreter = actual();
    let queries = Rc::new(Cell::new(0));
    interpreter.set_host(Rc::new(CountingHost {
        actual: interpreter.host(),
        queries: Rc::clone(&queries),
        during_query: Some(stop_second_result),
    }));
    let member = obj::Owned::fresh(obj::new_string_bytes(b"A"));
    let list = obj::Owned::fresh(ValueOps::new_list(
        &mut interpreter,
        vec![member.as_ptr(), member.as_ptr()],
    ));
    let all = obj::Owned::fresh(obj::new_string_bytes(b"-all"));
    let exact = obj::Owned::fresh(obj::new_string_bytes(b"-exact"));
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.result_obj();
    let before = crate::counters::live_objs();
    crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
    let _current = Current;
    let error = tcl_cmd_core::lsearch::lsearch::<Interp, crate::cmd_regex::AreEngine>(
        &mut interpreter,
        &[all.as_ptr(), exact.as_ptr(), list.as_ptr(), member.as_ptr()],
        tcl_dialect::TclVersion::V8_4,
    )
    .unwrap_err();
    assert_eq!(queries.get(), 2);
    assert_eq!(SECOND_QUERY_OBJECTS.get(), before + 1);
    assert_eq!(crate::counters::live_objs(), before);
    assert_eq!(interpreter.result_obj(), prior);
    let first = interpreter.native_execution_refusal().unwrap();
    assert!(
        matches!(&first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(reason)
        if reason.reason == "original Index second result provider")
    );
    assert_eq!(
        error.command_error.unwrap().native_execution_refusal(),
        Some(&first)
    );
    assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 3);
}
