// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual limit polling is independent of arithmetic backend configuration.
use super::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use tcl_platform::{Clock, Host};

struct ClockState {
    now: Cell<i128>,
    reads: Cell<usize>,
    numeric_queries: Cell<usize>,
}
impl Clock for ClockState {
    fn now_secs(&self) -> i64 {
        (self.now.get() / 1000) as i64
    }
    fn now_millis(&self) -> i128 {
        self.reads.set(self.reads.get() + 1);
        self.now.get()
    }
}
struct ClockHost {
    actual: Rc<dyn Host>,
    clock: Rc<ClockState>,
}
impl Host for ClockHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn Clock {
        self.clock.as_ref()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.actual.system_encoding()
    }
    fn encoding_catalogue(&self) -> Option<&dyn tcl_platform::EncodingCatalogue> {
        self.actual.encoding_catalogue()
    }
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.actual.filesystem()
    }
    fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
        self.actual.sockets()
    }
    fn process(&self) -> Option<&dyn tcl_platform::Process> {
        self.actual.process()
    }
    fn native_list_sorting_environment(
        &self,
    ) -> Option<&dyn tcl_platform::NativeListSortingEnvironment> {
        self.actual.native_list_sorting_environment()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.clock
            .numeric_queries
            .set(self.clock.numeric_queries.get() + 1);
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.actual.native_integer_formatter()
    }
}
fn native() -> (Interp, Rc<ClockState>) {
    let clock = Rc::new(ClockState {
        now: Cell::new(1000),
        reads: Cell::new(0),
        numeric_queries: Cell::new(0),
    });
    let host: Rc<dyn Host> = Rc::new(ClockHost {
        actual: default_host(),
        clock: clock.clone(),
    });
    let interpreter = Interp::with_native_core(
        host,
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    (interpreter, clock)
}

#[test]
fn original_limit_tick_unarmed_stays_query_free_in_each_backend_configuration() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut interpreter, clock) = native();
    let scope = native_operation_currency::NativeOperationScope::enter(&interpreter).unwrap();
    let reads = clock.reads.get();
    let numeric = clock.numeric_queries.get();
    for _ in 0..4096 {
        assert_eq!(interpreter.limit_check_tick(), None);
    }
    assert_eq!(clock.reads.get(), reads);
    assert_eq!(clock.numeric_queries.get(), numeric);
    assert_eq!(interpreter.limit_tick.get(), 0);
    assert_eq!(interpreter.limit_exceeded(), None);
    scope.currency().ensure_current_or_refuse().unwrap();
}

#[test]
fn original_limit_tick_retains_real_engine_budget_deadline_and_exceeded_state() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut interpreter, clock) = native();
    let scope = native_operation_currency::NativeOperationScope::enter(&interpreter).unwrap();
    let numeric = clock.numeric_queries.get();
    interpreter.set_limits(None, Some(Duration::from_millis(10)), None);
    interpreter.begin_evaluation();
    let reads = clock.reads.get();
    clock.now.set(1010);
    for _ in 0..4095 {
        assert_eq!(interpreter.limit_check_tick(), None);
    }
    assert_eq!(clock.reads.get(), reads);
    assert_eq!(interpreter.limit_check_tick(), Some(Code::Error));
    assert_eq!(clock.reads.get(), reads + 1);
    assert_eq!(
        interpreter.limit_exceeded(),
        Some(crate::budget::LimitKind::WallClock)
    );
    assert_eq!(interpreter.result_bytes(), b"time limit exceeded");
    assert_eq!(interpreter.limit_check_tick(), Some(Code::Error));
    assert_eq!(clock.reads.get(), reads + 1);
    assert_eq!(clock.numeric_queries.get(), numeric);
    scope.currency().ensure_current_or_refuse().unwrap();
}

#[test]
fn original_limit_tick_keeps_native_time_throttle_separate_from_engine_budget() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut interpreter, clock) = native();
    let scope = native_operation_currency::NativeOperationScope::enter(&interpreter).unwrap();
    let numeric = clock.numeric_queries.get();
    let reads = clock.reads.get();
    interpreter.limits.borrow_mut().time_value = Some((0, 999));
    for _ in 0..4095 {
        assert_eq!(interpreter.limit_check_tick(), None);
    }
    assert_eq!(clock.reads.get(), reads);
    assert_eq!(interpreter.limit_check_tick(), Some(Code::Error));
    assert_eq!(clock.reads.get(), reads + 1);
    assert_eq!(interpreter.result_bytes(), b"time limit exceeded");
    assert_eq!(interpreter.limit_exceeded(), None);
    assert_eq!(interpreter.limit_tick.get(), 4096);
    assert_eq!(clock.numeric_queries.get(), numeric);
    scope.currency().ensure_current_or_refuse().unwrap();
}
