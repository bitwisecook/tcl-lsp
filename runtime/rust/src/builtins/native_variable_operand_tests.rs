// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded original builtin variable operands and entered callback settlement.
use super::*;
use crate::obj::{self, Owned};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const ORIGINALS: &[(&str, &str)] = &[
    (
        "tcl8.4",
        include_str!(
            "../../tests/data/native_variable_introspection_getter_order_original/505b/providers/8.4.20/execute.stdout"
        ),
    ),
    (
        "tcl8.5",
        include_str!(
            "../../tests/data/native_variable_introspection_getter_order_original/505b/providers/8.5.19/execute.stdout"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../tests/data/native_variable_introspection_getter_order_original/505b/providers/8.6.18/execute.stdout"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../tests/data/native_variable_introspection_getter_order_original/505b/providers/9.0.4/execute.stdout"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../tests/data/native_variable_introspection_getter_order_original/505b/providers/9.1.0/execute.stdout"
        ),
    ),
    (
        "jim",
        include_str!(
            "../../tests/data/native_variable_introspection_getter_order_original/505b/providers/jim/execute.stdout"
        ),
    ),
];
thread_local! {
    static EVENTS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static PAYLOADS: RefCell<Vec<(u8,Vec<u8>)>> = const { RefCell::new(Vec::new()) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static REFUSE_LABEL: Cell<u8> = const { Cell::new(0) };
}
struct Current(*mut Interp);
impl Current {
    fn enter(interp: &mut Interp) -> Self {
        let previous = crate::codegen_abi::current_interp();
        crate::codegen_abi::tcl_runtime_set_current_interp(interp);
        Self(previous)
    }
}
impl Drop for Current {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(self.0);
    }
}
fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn reset(mode: u8, refusal: u8, payloads: &[(u8, &[u8])]) {
    EVENTS.with(|v| v.borrow_mut().clear());
    PAYLOADS.with(|v| *v.borrow_mut() = payloads.iter().map(|(k, v)| (*k, v.to_vec())).collect());
    MODE.with(|v| v.set(mode));
    REFUSE_LABEL.with(|v| v.set(refusal));
}
fn refuse(interp: &mut Interp) {
    match MODE.with(Cell::get) {
        1 => {
            interp.refuse_host_command("first original builtin variable stage");
        }
        2 => {
            let actual = interp.runtime_context();
            let mut changed = actual.clone();
            changed.packages = vec![("builtin-original-stage".to_owned(), "1.0".to_owned())];
            interp.pin_context(&changed).unwrap();
            interp.pin_context(&actual).unwrap();
        }
        _ => {}
    }
}
extern "C" fn getter(original: *mut TclObj) {
    let label = unsafe { (*original).internal_rep } as u8;
    EVENTS.with(|v| v.borrow_mut().push(label));
    let bytes = PAYLOADS.with(|v| {
        v.borrow()
            .iter()
            .find(|(k, _)| *k == label)
            .unwrap()
            .1
            .clone()
    });
    if REFUSE_LABEL.with(Cell::get) == label {
        let mut interp = (unsafe { crate::codegen_abi::current_interp().as_ref() })
            .unwrap()
            .clone();
        refuse(&mut interp);
    }
    unsafe { obj::set_native_updater_string_rep(original, &bytes, false) };
}
static LAZY: obj::TclObjType = obj::TclObjType {
    name: c"originalBuiltinVariableOperand".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(getter),
    set_from_any_proc: None,
};
fn lazy(label: u8) -> Owned {
    Owned::fresh(obj::alloc_typed(&LAZY, u64::from(label)))
}
fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect()
}
fn compare_public(interp: &mut Interp, code: Code, capture: &str, case: usize) {
    let row: Vec<_> = capture
        .lines()
        .find(|line| line.starts_with(&format!("ROW|{case}|worker|")))
        .unwrap()
        .split('|')
        .collect();
    assert_eq!(code.as_int(), row[3].parse::<i64>().unwrap());
    assert!(!interp.host_refusal_pending());
    assert_eq!(interp.result_bytes(), decode(row[4]));
    let events: Vec<_> = capture
        .lines()
        .filter_map(|line| {
            let row: Vec<_> = line.split('|').collect();
            (row.first() == Some(&"EVENT")
                && row[1].parse::<usize>() == Ok(case)
                && row.last() == Some(&"phase=worker"))
            .then(|| row[3].as_bytes()[0])
        })
        .collect();
    assert_eq!(EVENTS.with(|v| v.borrow().clone()), events);
}

use crate::test_storage::global_scalar as global_storage;

fn reached_refusal(interp: &Interp, mode: u8) -> tcl_runtime_api::NativeExecutionError {
    let first = interp
        .native_execution_refusal()
        .expect("the reached original callback must retain its Host cause");
    match mode {
        1 => match &first {
            tcl_runtime_api::NativeExecutionError::HostCommandRefusal(error) => {
                assert_eq!(error.reason, "first original builtin variable stage");
            }
            _ => panic!("the actual updater's first Host cause must be retained: {first:?}"),
        },
        2 => assert_eq!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
            )
        ),
        _ => panic!("the refusal control must select a real callback mode"),
    }
    first
}

fn bind_jim(interp: &Interp, originals: &[*mut TclObj]) {
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        let context = interp.native_jim_object_context().unwrap();
        for &original in originals {
            crate::native_source::bind_context(original, &context).unwrap();
        }
    }
}

#[test]
fn original_increment_successful_rows_keep_legacy_amount_before_modern_name() {
    // Bounded successful505b12 public code/result and worker getter tape only.
    for &(profile, capture) in ORIGINALS {
        let mut interp = native(profile);
        assert_eq!(interp.eval_str(b"set ::target 1"), Code::Ok);
        reset(0, 0, &[(b'A', b"::target"), (b'B', b"2")]);
        let head = text(b"incr");
        let name = lazy(b'A');
        let amount = lazy(b'B');
        bind_jim(&interp, &[name.as_ptr(), amount.as_ptr()]);
        let current = Current::enter(&mut interp);
        let code = incr(
            &mut interp,
            &[head.as_ptr(), name.as_ptr(), amount.as_ptr()],
        );
        compare_public(&mut interp, code, capture, 12);
        assert_eq!(obj::bytes_of(interp.var_get(b"::target").unwrap()), b"3");
        drop(current);
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_unset_successful_rows_preserve_sequential_original_names() {
    // Successful505b16 comparison; API error-context getters are not worker facts.
    for &(profile, capture) in ORIGINALS {
        let mut interp = native(profile);
        assert_eq!(
            interp.eval_str(b"set ::target VALUE; set ::survivor STILL"),
            Code::Ok
        );
        reset(
            0,
            0,
            &[
                (b'A', b"::target"),
                (b'B', b"::missing"),
                (b'C', b"::survivor"),
            ],
        );
        let head = text(b"unset");
        let option = text(b"-nocomplain");
        let first = lazy(b'A');
        let missing = lazy(b'B');
        let last = lazy(b'C');
        bind_jim(&interp, &[first.as_ptr(), missing.as_ptr(), last.as_ptr()]);
        let current = Current::enter(&mut interp);
        let code = unset(
            &mut interp,
            &[
                head.as_ptr(),
                option.as_ptr(),
                first.as_ptr(),
                missing.as_ptr(),
                last.as_ptr(),
            ],
        );
        compare_public(&mut interp, code, capture, 16);
        assert!(interp.var_get(b"::target").is_none());
        assert!(interp.var_get(b"::survivor").is_none());
        drop(current);
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_increment_first_refusal_retains_selected_reached_operand_and_amount_hold() {
    // Software updater/currency control, separate from Native505b order proof.
    for &(profile, _) in ORIGINALS {
        for mode in [1, 2] {
            let mut interp = native(profile);
            assert_eq!(interp.eval_str(b"set ::target 1"), Code::Ok);
            let legacy = interp
                .native_invocation_dialect()
                .native_legacy_increment_protocol()
                .is_some();
            let reached = if legacy { b'B' } else { b'A' };
            reset(mode, reached, &[(b'A', b"::target"), (b'B', b"2")]);
            let head = text(b"incr");
            let name = lazy(b'A');
            let amount = lazy(b'B');
            bind_jim(&interp, &[name.as_ptr(), amount.as_ptr()]);
            let amount_refs = unsafe { (*amount.as_ptr()).ref_count };
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            let current = Current::enter(&mut interp);
            assert_eq!(
                incr(
                    &mut interp,
                    &[head.as_ptr(), name.as_ptr(), amount.as_ptr()]
                ),
                Code::Error
            );
            assert_eq!(EVENTS.with(|v| v.borrow().clone()), vec![reached]);
            assert!(!obj::has_string_rep(if legacy {
                name.as_ptr()
            } else {
                amount.as_ptr()
            }));
            assert_eq!(unsafe { (*amount.as_ptr()).ref_count }, amount_refs);
            assert_eq!(interp.get_obj_result(), prior);
            let first = reached_refusal(&interp, mode);
            assert_eq!(
                obj::bytes_of(global_storage(&interp, b"target").unwrap()),
                b"1"
            );
            assert!(interp.var_get(b"::target").is_none());
            assert_eq!(interp.native_execution_refusal(), Some(first));
            assert!(interp.native_execution_refusal().is_some());
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_unset_prefix_effect_survives_host_refusal_without_reading_later_name() {
    // Real reached unset effect plus callback first cause, no Native error tape claim.
    for profile in ["tcl8.4", "tcl8.6", "tcl9.1", "jim"] {
        for mode in [1, 2] {
            let mut interp = native(profile);
            assert_eq!(
                interp.eval_str(b"set ::target VALUE; set ::survivor STILL"),
                Code::Ok
            );
            reset(mode, b'B', &[(b'B', b"::missing"), (b'C', b"::survivor")]);
            let head = text(b"unset");
            let first = text(b"::target");
            let failed = lazy(b'B');
            let later = lazy(b'C');
            bind_jim(&interp, &[failed.as_ptr(), later.as_ptr()]);
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            let current = Current::enter(&mut interp);
            assert_eq!(
                unset(
                    &mut interp,
                    &[
                        head.as_ptr(),
                        first.as_ptr(),
                        failed.as_ptr(),
                        later.as_ptr()
                    ]
                ),
                Code::Error
            );
            assert_eq!(EVENTS.with(|v| v.borrow().clone()), vec![b'B']);
            let first = reached_refusal(&interp, mode);
            assert!(global_storage(&interp, b"target").is_none());
            assert_eq!(
                obj::bytes_of(global_storage(&interp, b"survivor").unwrap()),
                b"STILL"
            );
            assert!(interp.var_get(b"::target").is_none());
            assert!(interp.var_get(b"::survivor").is_none());
            assert_eq!(interp.native_execution_refusal(), Some(first));
            assert!(!obj::has_string_rep(later.as_ptr()));
            assert_eq!(interp.get_obj_result(), prior);
            assert!(interp.native_execution_refusal().is_some());
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

struct IncrementWrite;
impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Interp> for IncrementWrite {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        interp: &mut Interp,
        access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(
            access.operation,
            tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write
        );
        assert_eq!(obj::bytes_of(interp.var_get(b"::target").unwrap()), b"3");
        refuse(interp);
        Ok(())
    }
}
#[test]
fn original_increment_write_callback_keeps_reached_store_before_first_cause_settlement() {
    // Source-only reached-store/currency control; no Native505b trace observation.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [1, 2] {
            let mut interp = native(profile);
            assert_eq!(interp.eval_str(b"set ::target 1"), Code::Ok);
            let head = text(b"incr");
            let name = text(b"::target");
            let amount = text(b"2");
            interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write],
                    Rc::new(IncrementWrite),
                )
                .unwrap();
            reset(mode, 0, &[]);
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            let amount_refs = unsafe { (*amount.as_ptr()).ref_count };
            let code = incr(
                &mut interp,
                &[head.as_ptr(), name.as_ptr(), amount.as_ptr()],
            );
            assert_eq!(code, Code::Error);
            let first = reached_refusal(&interp, mode);
            assert_eq!(
                obj::bytes_of(global_storage(&interp, b"target").unwrap()),
                b"3"
            );
            assert!(interp.var_get(b"::target").is_none());
            assert_eq!(interp.native_execution_refusal(), Some(first));
            assert_eq!(interp.get_obj_result(), prior);
            assert!(interp.native_execution_refusal().is_some());
            assert_eq!(unsafe { (*amount.as_ptr()).ref_count }, amount_refs);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}
