// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected software precision stages and original context barriers. These
//! controls do not measure external Tcl callback chronology or grant Tcl32.

use super::*;
use crate::interp::{default_host, WeakInterp};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_host_c_abi::NativeNumericEnvironment;
use tcl_platform::{
    DoubleNumericConversion, Host, NativeCIntegerAbi, NumericEnvironment,
    NumericEnvironmentUnavailable, NumericErrorState, UnsignedNumericConversion,
};

type OriginalLongCall = (Vec<u8>, usize, u32, UnsignedNumericConversion);

struct ObservedNumeric {
    queries: Cell<usize>,
    abi_queries: Cell<usize>,
    resets: Cell<usize>,
    long_calls: RefCell<Vec<OriginalLongCall>>,
    owner: RefCell<Option<WeakInterp>>,
    change_after_long: Cell<bool>,
}

impl ObservedNumeric {
    fn new() -> Self {
        Self {
            queries: Cell::new(0),
            abi_queries: Cell::new(0),
            resets: Cell::new(0),
            long_calls: RefCell::new(Vec::new()),
            owner: RefCell::new(None),
            change_after_long: Cell::new(false),
        }
    }
    fn clear(&self) {
        self.queries.set(0);
        self.abi_queries.set(0);
        self.resets.set(0);
        self.long_calls.borrow_mut().clear();
    }
}

impl NumericEnvironment for ObservedNumeric {
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        self.abi_queries.set(self.abi_queries.get() + 1);
        NativeNumericEnvironment.c_integer_abi()
    }
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        NativeNumericEnvironment.state()
    }
    fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
        self.resets.set(self.resets.get() + 1);
        NativeNumericEnvironment.reset()
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        NativeNumericEnvironment.unsigned(input, offset, base)
    }
    fn unsigned_long(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        let actual = NativeNumericEnvironment.unsigned_long(input, offset, base)?;
        self.long_calls
            .borrow_mut()
            .push((input.to_vec(), offset, base, actual));
        if self.change_after_long.replace(false) {
            let owner = self.owner.borrow().clone().unwrap();
            change_context(&mut owner.upgrade().unwrap());
        }
        Ok(actual)
    }
    fn double(
        &self,
        input: &[u8],
        reset: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
        NativeNumericEnvironment.double(input, reset)
    }
}

struct NumericHost {
    base: Rc<dyn Host>,
    numeric: ObservedNumeric,
    available: Cell<bool>,
}
impl NumericHost {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            base: default_host(),
            numeric: ObservedNumeric::new(),
            available: Cell::new(true),
        })
    }
}
impl Host for NumericHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.base.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.base.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.base.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.base.env()
    }
    fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
        self.numeric.queries.set(self.numeric.queries.get() + 1);
        self.available
            .get()
            .then_some(&self.numeric as &dyn NumericEnvironment)
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.base.native_integer_formatter()
    }
}

fn native(engine: &str, host: Rc<NumericHost>) -> Interp {
    let environment = tcl_registry::model::ingress::resolve_known_environment(engine).unwrap();
    let interpreter = Interp::with_native_core(
        host.clone(),
        environment.unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    *host.numeric.owner.borrow_mut() = Some(interpreter.downgrade());
    host.numeric.clear();
    interpreter
}
fn policy(interpreter: &Interp) -> tcl_dialect::DoubleStringPolicy {
    interpreter
        .native_invocation_dialect()
        .double_string_policy()
        .unwrap()
}
fn name(interpreter: &Interp) -> &'static [u8] {
    policy(interpreter).precision_variable().unwrap().as_bytes()
}
fn root(interpreter: &Interp) -> Option<*mut obj::TclObj> {
    crate::vars::get_at(
        &interpreter.frames.borrow(),
        &interpreter.namespaces.borrow(),
        name(interpreter),
        0,
    )
}
fn put(interpreter: &mut Interp, value: *mut obj::TclObj) {
    interpreter.var_set_at(name(interpreter), value, 0).unwrap();
}
fn hidden(
    interpreter: &mut Interp,
    operation: &[u8],
) -> Result<VariableTraceGuestOutcome, NativeExecutionError> {
    let name = name(interpreter);
    let home = interpreter.trace_identity(name);
    assert!(interpreter.native_precision_trace_at(&home));
    let access = interpreter.trace_access(name, name, None, &home, false);
    interpreter.native_precision_trace(&home, &access, operation)
}
fn stale() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "stale entered native operation",
    ))
}
fn unavailable() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::ScalarNumericInputUnavailable,
    )
}
fn change_context(interpreter: &mut Interp) {
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("precision-stage-context".into(), "1.0".into())];
    interpreter.pin_context(&changed).unwrap();
    interpreter.pin_context(&original).unwrap();
}

struct Original {
    owner: WeakInterp,
    updates: Rc<Cell<usize>>,
    frees: Rc<Cell<usize>>,
    update_changes_context: bool,
    retirement_refuses: bool,
    getter_refs: Rc<Cell<Option<isize>>>,
}
extern "C" fn update(value: *mut obj::TclObj) {
    // SAFETY: this original extension descriptor owns the boxed state.
    let original = unsafe { &*(obj::internal_rep(value) as usize as *const Original) };
    original.updates.set(original.updates.get() + 1);
    original
        .getter_refs
        .set(Some(unsafe { (*value).ref_count }));
    unsafe { obj::set_native_updater_string_rep(value, b"5", false) };
    if original.update_changes_context {
        change_context(&mut original.owner.upgrade().unwrap());
    }
}
extern "C" fn retire(value: *mut obj::TclObj) {
    // SAFETY: the one original free callback retires this boxed state once.
    let original = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut Original) };
    original.frees.set(original.frees.get() + 1);
    if original.retirement_refuses {
        original
            .owner
            .upgrade()
            .unwrap()
            .refuse_host_command("first precision retirement");
    }
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalPrecisionStage".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn original(
    interpreter: &Interp,
    update_changes_context: bool,
    retirement_refuses: bool,
) -> (obj::Owned, Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let updates = Rc::new(Cell::new(0));
    let frees = Rc::new(Cell::new(0));
    let state = Box::new(Original {
        owner: interpreter.downgrade(),
        updates: updates.clone(),
        frees: frees.clone(),
        update_changes_context,
        retirement_refuses,
        getter_refs: Rc::new(Cell::new(None)),
    });
    let value = obj::alloc_typed(&ORIGINAL, Box::into_raw(state) as usize as u64);
    (obj::Owned::fresh(value), updates, frees)
}

#[test]
fn original_precision_safe_write_precedes_string_and_numeric_stages() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        let host = NumericHost::new();
        let mut interpreter = native(engine, host.clone());
        interpreter.make_safe();
        let selected = policy(&interpreter);
        let prior_precision = obj::double_precision(selected);
        let (value, updates, _) = original(&interpreter, true, false);
        put(&mut interpreter, value.as_ptr());
        interpreter.set_result_bytes(b"BEFORE RESULT");
        interpreter.set_c_error_code(b"BEFORE CODE");
        let result = interpreter.get_obj_result();
        let code = interpreter.snapshot_error();
        host.numeric.clear();
        assert_eq!(
            hidden(&mut interpreter, b"write"),
            Ok(VariableTraceGuestOutcome::GuestError)
        );
        assert_eq!(updates.get(), 0);
        assert_eq!(host.numeric.queries.get(), 0);
        assert_eq!(host.numeric.abi_queries.get(), 0);
        assert!(host.numeric.long_calls.borrow().is_empty());
        assert_eq!(obj::double_precision(selected), prior_precision);
        assert_eq!(interpreter.get_obj_result(), result);
        assert_eq!(interpreter.snapshot_error().code_bytes(), code.code_bytes());
        assert_eq!(
            interpreter.traces.borrow().pending_err.as_deref(),
            Some(b"can't modify precision from a safe interpreter".as_slice())
        );
        if engine == "tcl8.4" {
            assert_ne!(root(&interpreter), Some(value.as_ptr()));
        } else {
            assert_eq!(root(&interpreter), Some(value.as_ptr()));
        }
        assert!(interpreter.native_execution_refusal().is_none());
    }
}

#[test]
fn original_precision_modern_int_uses_cache_and_rejects_double_without_string() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for engine in ["tcl8.5", "tcl8.6"] {
        let host = NumericHost::new();
        let mut interpreter = native(engine, host.clone());
        let selected = policy(&interpreter);
        let prior_precision = obj::double_precision(selected);
        let value = obj::Owned::fresh(obj::new_wide_int_obj(5));
        unsafe { obj::set_native_updater_string_rep(value.as_ptr(), b"17", false) };
        put(&mut interpreter, value.as_ptr());
        interpreter.set_result_bytes(b"CACHE RESULT");
        interpreter.set_c_error_code(b"CACHE CODE");
        let result = interpreter.get_obj_result();
        let code = interpreter.snapshot_error();
        let before = obj::native_object_snapshot(value.as_ptr()).unwrap();
        host.numeric.clear();
        assert_eq!(
            hidden(&mut interpreter, b"write"),
            Ok(VariableTraceGuestOutcome::Complete)
        );
        assert_eq!(obj::double_precision(selected), 5);
        assert_eq!(obj::native_object_snapshot(value.as_ptr()).unwrap(), before);
        assert!(host.numeric.abi_queries.get() > 0);
        assert_eq!(host.numeric.resets.get(), 0);
        assert!(host.numeric.long_calls.borrow().is_empty());
        let double = obj::Owned::fresh(obj::new_double_obj(17.0));
        put(&mut interpreter, double.as_ptr());
        let before_double = obj::native_object_snapshot(double.as_ptr()).unwrap();
        assert!(!obj::has_string_rep(double.as_ptr()));
        assert_eq!(
            hidden(&mut interpreter, b"write"),
            Ok(VariableTraceGuestOutcome::GuestError)
        );
        assert_eq!(
            obj::native_object_snapshot(double.as_ptr()).unwrap(),
            before_double
        );
        assert_eq!(root(&interpreter), Some(double.as_ptr()));
        assert_eq!(obj::double_precision(selected), 5);
        assert_eq!(
            interpreter.traces.borrow().pending_err.as_deref(),
            Some(b"improper value for precision".as_slice())
        );
        assert_eq!(interpreter.get_obj_result(), result);
        assert_eq!(interpreter.snapshot_error().code_bytes(), code.code_bytes());
        obj::set_double_precision(selected, prior_precision);
    }
}

#[test]
fn original_precision_c84_decimal_worker_keeps_c_int_narrowing_and_no_reset() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual linked libc/ABI, software callback purpose; no external Tcl32 recipe.
    let host = NumericHost::new();
    let mut interpreter = native("tcl8.4", host.clone());
    let selected = policy(&interpreter);
    let prior_precision = obj::double_precision(selected);
    for (bytes, accepted) in [
        (b"010".as_slice(), Some(10)),
        (b"0x10".as_slice(), None),
        (b"4294967297".as_slice(), Some(1)),
        (b"4294967296".as_slice(), None),
        (b"-4294967295".as_slice(), Some(1)),
        (b"-4294967296".as_slice(), None),
        (b"17\0ignored".as_slice(), Some(17)),
        (b"18446744073709551616".as_slice(), None),
    ] {
        let value = obj::Owned::fresh(crate::interp::new_string(bytes));
        put(&mut interpreter, value.as_ptr());
        host.numeric.clear();
        let state_before = NativeNumericEnvironment.state().unwrap();
        let reached = hidden(&mut interpreter, b"write");
        assert_eq!(
            reached,
            Ok(if accepted.is_some() {
                VariableTraceGuestOutcome::Complete
            } else {
                VariableTraceGuestOutcome::GuestError
            }),
            "{bytes:?}"
        );
        let calls = host.numeric.long_calls.borrow();
        assert_eq!(calls.len(), 1);
        let (input, offset, radix, actual) = &calls[0];
        assert_eq!(input, bytes);
        assert_eq!((*offset, *radix), (0, 10));
        assert_eq!(actual.before, state_before);
        assert_eq!(host.numeric.resets.get(), 0);
        if bytes == b"18446744073709551616" {
            assert!(actual.after.range_error);
        }
        if let Some(precision) = accepted {
            assert_eq!(obj::double_precision(selected), precision);
        }
        assert!(interpreter.native_execution_refusal().is_none());
    }
    obj::set_double_precision(selected, prior_precision);
}

#[test]
fn original_precision_string_and_numeric_callbacks_stop_before_publication_on_aba() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for at_string in [true, false] {
        let host = NumericHost::new();
        let mut interpreter = native("tcl8.4", host.clone());
        let selected = policy(&interpreter);
        let prior_precision = obj::double_precision(selected);
        let (value, updates, _) = original(&interpreter, at_string, false);
        put(&mut interpreter, value.as_ptr());
        interpreter.set_result_bytes(b"BEFORE ABA");
        let result = interpreter.get_obj_result();
        host.numeric.clear();
        host.numeric.change_after_long.set(!at_string);
        assert_eq!(hidden(&mut interpreter, b"write"), Err(stale()));
        assert_eq!(updates.get(), 1);
        assert!(obj::has_string_rep(value.as_ptr()));
        assert_eq!(
            host.numeric.long_calls.borrow().len(),
            usize::from(!at_string)
        );
        assert_eq!(host.numeric.queries.get(), usize::from(!at_string));
        assert_eq!(obj::double_precision(selected), prior_precision);
        assert!(interpreter.traces.borrow().pending_err.is_none());
        assert_eq!(interpreter.get_obj_result(), result);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
    }
}

#[test]
fn original_precision_c84_restore_keeps_reached_cell_and_first_retirement_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let host = NumericHost::new();
    let mut interpreter = native("tcl8.4", host.clone());
    interpreter.make_safe();
    let selected = policy(&interpreter);
    let prior_precision = obj::double_precision(selected);
    let (value, updates, frees) = original(&interpreter, false, true);
    put(&mut interpreter, value.as_ptr());
    let old_pointer = value.as_ptr();
    drop(value);
    interpreter.set_result_bytes(b"BEFORE RESTORE");
    let result = interpreter.get_obj_result();
    host.numeric.clear();
    let reached = hidden(&mut interpreter, b"write");
    let first = interpreter.native_execution_refusal().unwrap();
    let NativeExecutionError::HostCommandRefusal(ref cause) = first else {
        panic!("wrong first cause: {first:?}")
    };
    assert_eq!(cause.reason, "first precision retirement");
    assert_eq!(reached, Err(first));
    assert_eq!(updates.get(), 0);
    assert_eq!(frees.get(), 1);
    assert_eq!(host.numeric.queries.get(), 0);
    let stored = root(&interpreter).unwrap();
    assert_ne!(stored, old_pointer);
    assert_eq!(
        obj::native_scalar_cache(stored).unwrap(),
        Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Int(i64::from(prior_precision))
        ))
    );
    assert_eq!(obj::double_precision(selected), prior_precision);
    assert!(interpreter.traces.borrow().pending_err.is_none());
    assert_eq!(interpreter.get_obj_result(), result);
}

#[test]
fn original_precision_missing_environment_and_prior_cause_remain_outer_refusals() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        let host = NumericHost::new();
        let mut interpreter = native(engine, host.clone());
        let selected = policy(&interpreter);
        let prior_precision = obj::double_precision(selected);
        let value = obj::Owned::fresh(if engine == "tcl8.4" {
            crate::interp::new_string(b"5")
        } else {
            obj::new_wide_int_obj(5)
        });
        put(&mut interpreter, value.as_ptr());
        host.available.set(false);
        host.numeric.clear();
        if engine == "tcl8.4" {
            assert_eq!(hidden(&mut interpreter, b"write"), Err(unavailable()));
            assert_eq!(interpreter.native_execution_refusal(), Some(unavailable()));
            assert_eq!(obj::double_precision(selected), prior_precision);
        } else {
            // The selected known cached Int worker needs no optional libc
            // conversion stage. Absence must not invent a failed getter.
            assert_eq!(
                hidden(&mut interpreter, b"write"),
                Ok(VariableTraceGuestOutcome::Complete)
            );
            assert!(interpreter.native_execution_refusal().is_none());
            assert_eq!(obj::double_precision(selected), 5);
            obj::set_double_precision(selected, prior_precision);
        }
        assert_eq!(root(&interpreter), Some(value.as_ptr()));
        assert!(interpreter.traces.borrow().pending_err.is_none());
    }
    let host = NumericHost::new();
    let mut interpreter = native("tcl8.4", host.clone());
    let (value, updates, _) = original(&interpreter, true, false);
    put(&mut interpreter, value.as_ptr());
    let marker = interpreter.native_precision_cell.get();
    interpreter.refuse_host_command("prior precision cause");
    let first = interpreter.native_execution_refusal().unwrap();
    host.numeric.clear();
    assert_eq!(hidden(&mut interpreter, b"write"), Err(first.clone()));
    assert_eq!(
        interpreter.install_native_precision_trace_checked(),
        Err(first.clone())
    );
    assert_eq!(interpreter.native_precision_cell.get(), marker);
    assert_eq!(updates.get(), 0);
    assert_eq!(host.numeric.queries.get(), 0);
    assert_eq!(interpreter.native_execution_refusal(), Some(first));
}

#[test]
fn original_precision_read_ignores_guest_array_store_without_promoting_host_failure() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        let host = NumericHost::new();
        let mut interpreter = native(engine, host.clone());
        interpreter.ensure_array(name(&interpreter)).unwrap();
        interpreter.set_result_bytes(b"ARRAY RESULT");
        let result = interpreter.get_obj_result();
        host.numeric.clear();
        assert_eq!(
            hidden(&mut interpreter, b"read"),
            Ok(VariableTraceGuestOutcome::Complete)
        );
        assert_eq!(interpreter.get_obj_result(), result);
        assert_eq!(host.numeric.queries.get(), 0);
        assert!(interpreter.traces.borrow().pending_err.is_none());
        assert!(interpreter.native_execution_refusal().is_none());
    }
}

#[test]
fn original_precision_unsigned_long_default_absence_cannot_borrow_another_worker() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The environment supplies its genuine target and other actual workers,
    // while omitting precisely the newly selected optional unsigned-long stage.
    struct MissingLong {
        alternatives: Cell<usize>,
        resets: Cell<usize>,
    }
    impl NumericEnvironment for MissingLong {
        fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
            NativeNumericEnvironment.c_integer_abi()
        }
        fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
            NativeNumericEnvironment.state()
        }
        fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
            self.resets.set(self.resets.get() + 1);
            NativeNumericEnvironment.reset()
        }
        fn unsigned_c84(
            &self,
            input: &[u8],
            offset: usize,
            long: bool,
        ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
            self.alternatives.set(self.alternatives.get() + 1);
            NativeNumericEnvironment.unsigned_c84(input, offset, long)
        }
        fn unsigned(
            &self,
            input: &[u8],
            offset: usize,
            radix: u32,
        ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
            self.alternatives.set(self.alternatives.get() + 1);
            NativeNumericEnvironment.unsigned(input, offset, radix)
        }
        fn double(
            &self,
            input: &[u8],
            reset: bool,
        ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
            self.alternatives.set(self.alternatives.get() + 1);
            NativeNumericEnvironment.double(input, reset)
        }
    }
    let environment = MissingLong {
        alternatives: Cell::new(0),
        resets: Cell::new(0),
    };
    let target = tcl_cmd_core::native_numeric::scalar_getter_target(&environment).unwrap();
    assert_eq!(
        tcl_cmd_core::native_numeric::c84_precision_from_host(b"010", &environment),
        Err(ValueError::ScalarNumericInputUnavailable)
    );
    assert_eq!(environment.alternatives.get(), 0);
    assert_eq!(environment.resets.get(), 0);
    assert_eq!(
        tcl_syntax::number::c84_double_precision_from_unsigned_long(b"010", 10, 3, target),
        Some(10)
    );
    assert_eq!(
        tcl_syntax::number::double_precision_from_native_integer(
            tcl_dialect::DoubleStringPolicy::Shortest,
            5
        ),
        None
    );
}

#[test]
fn original_precision_absent_value_keeps_the_selected_null_getter_worker() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6"] {
        let host = NumericHost::new();
        let mut interpreter = native(engine, host.clone());
        assert_eq!(root(&interpreter), None);
        host.numeric.clear();
        assert_eq!(
            hidden(&mut interpreter, b"write"),
            Ok(VariableTraceGuestOutcome::GuestError)
        );
        assert_eq!(
            interpreter.traces.borrow().pending_err.as_deref(),
            Some(b"improper value for precision".as_slice())
        );
        if engine == "tcl8.4" {
            let calls = host.numeric.long_calls.borrow();
            assert_eq!(calls.len(), 1);
            let (bytes, offset, radix, reached) = &calls[0];
            assert!(bytes.is_empty());
            assert_eq!((*offset, *radix, reached.end), (0, 10, 0));
            assert_eq!(reached.value, 0);
            assert!(root(&interpreter).is_some());
        } else {
            assert!(host.numeric.long_calls.borrow().is_empty());
            assert_eq!(host.numeric.queries.get(), 0);
            assert_eq!(root(&interpreter), None);
        }
        assert_eq!(host.numeric.resets.get(), 0);
        assert!(interpreter.native_execution_refusal().is_none());
    }
}

#[test]
fn original_precision_string_getter_keeps_the_borrowed_cell_reference_role() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software extension observes the original native header. The allocation
    // lease changes no public reference or sharing state at the getter.
    let host = NumericHost::new();
    let mut interpreter = native("tcl8.4", host);
    let selected = policy(&interpreter);
    let prior_precision = obj::double_precision(selected);
    let (value, updates, _) = original(&interpreter, false, false);
    let pointer = value.as_ptr();
    let seen = unsafe { &*(obj::internal_rep(pointer) as usize as *const Original) }
        .getter_refs
        .clone();
    put(&mut interpreter, pointer);
    drop(value);
    assert_eq!(unsafe { (*pointer).ref_count }, 1);
    assert_eq!(
        hidden(&mut interpreter, b"write"),
        Ok(VariableTraceGuestOutcome::Complete)
    );
    assert_eq!(updates.get(), 1);
    assert_eq!(seen.get(), Some(1));
    assert_eq!(root(&interpreter), Some(pointer));
    assert_eq!(unsafe { (*pointer).ref_count }, 1);
    assert_eq!(obj::double_precision(selected), 5);
    assert!(interpreter.native_execution_refusal().is_none());
    obj::set_double_precision(selected, prior_precision);
}
