// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded original array operand stages and entered callback settlement.
use super::*;
use crate::obj::{self, Owned};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};

const ORIGINALS: &[(&str, &str)] = &[
    ("tcl8.4", include_str!("../../tests/data/native_variable_introspection_getter_order_original/505b/providers/8.4.20/execute.stdout")),
    ("tcl8.5", include_str!("../../tests/data/native_variable_introspection_getter_order_original/505b/providers/8.5.19/execute.stdout")),
    ("tcl8.6", include_str!("../../tests/data/native_variable_introspection_getter_order_original/505b/providers/8.6.18/execute.stdout")),
    ("tcl9.0", include_str!("../../tests/data/native_variable_introspection_getter_order_original/505b/providers/9.0.4/execute.stdout")),
    ("tcl9.1", include_str!("../../tests/data/native_variable_introspection_getter_order_original/505b/providers/9.1.0/execute.stdout")),
    ("jim", include_str!("../../tests/data/native_variable_introspection_getter_order_original/505b/providers/jim/execute.stdout")),
];
thread_local! {
    static EVENTS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static PAYLOADS: RefCell<Vec<(u8,Vec<u8>)>> = const { RefCell::new(Vec::new()) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static REFUSE_LABEL: Cell<u8> = const { Cell::new(0) };
    static WRITES: Cell<usize> = const { Cell::new(0) };
    static RETIRED: Cell<usize> = const { Cell::new(0) };
    static RELEASED: Cell<bool> = const { Cell::new(false) };
    static EXPECTED: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static NEW_VISIBLE: Cell<bool> = const { Cell::new(false) };
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
fn list(interp: &Interp, members: &[*mut TclObj]) -> Owned {
    Owned::fresh(crate::list::new_list_obj_native(
        members,
        interp
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap(),
    ))
}
fn reset(mode: u8, refusal: u8, payloads: &[(u8, &[u8])]) {
    EVENTS.with(|v| v.borrow_mut().clear());
    PAYLOADS.with(|v| *v.borrow_mut() = payloads.iter().map(|(k, v)| (*k, v.to_vec())).collect());
    MODE.with(|v| v.set(mode));
    REFUSE_LABEL.with(|v| v.set(refusal));
    WRITES.with(|v| v.set(0));
    RETIRED.with(|v| v.set(0));
    RELEASED.with(|v| v.set(false));
    NEW_VISIBLE.with(|v| v.set(false));
}
fn refuse(interp: &mut Interp) {
    match MODE.with(Cell::get) {
        1 => {
            interp.refuse_host_command("first original array stage");
        }
        2 => {
            let actual = interp.runtime_context();
            let mut changed = actual.clone();
            changed.packages = vec![("array-original-stage".to_owned(), "1.0".to_owned())];
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
    name: c"originalArrayOperand".as_ptr(),
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

#[test]
fn original_array_set_successful_rows_preserve_selected_header_getter_order() {
    // naming.invocation.original-variable-and-introspection-getter-order

    // Only successful505b4/6 public code/bytes and worker-phase tape. Header
    // reference counts, later array presentation and backend entry stay separate.
    for &(profile, capture) in ORIGINALS {
        for case in [4, 6] {
            let mut interp = native(profile);
            reset(
                0,
                0,
                &[
                    (b'A', b"::arr"),
                    (b'B', b"k"),
                    (b'C', if case == 4 { b"VALUE" } else { b"m" }),
                ],
            );
            let head = text(b"array");
            let sub = text(b"set");
            let name = lazy(b'A');
            let key = lazy(b'B');
            let later = lazy(b'C');
            let first = text(b"FIRST");
            let second = text(b"SECOND");
            let pair = [key.as_ptr(), later.as_ptr()];
            let pairs = [
                key.as_ptr(),
                first.as_ptr(),
                later.as_ptr(),
                second.as_ptr(),
            ];
            let values = list(&interp, if case == 4 { &pair } else { &pairs });
            let refs = unsafe { (*later.as_ptr()).ref_count };
            let current = Current::enter(&mut interp);
            let code = array_cmd(
                &mut interp,
                &[head.as_ptr(), sub.as_ptr(), name.as_ptr(), values.as_ptr()],
            );
            compare_public(&mut interp, code, capture, case);
            if case == 4 {
                assert!(!obj::has_string_rep(later.as_ptr()));
            }
            if profile == "jim" {
                assert!(!obj::has_string_rep(key.as_ptr()));
                assert!(!obj::has_string_rep(later.as_ptr()));
                assert_eq!(interp.var_get(b"arr"), Some(values.as_ptr()));
            }
            assert_eq!(
                unsafe { (*later.as_ptr()).ref_count },
                refs + if case == 4 && profile != "jim" {
                    1
                } else if case == 6
                    && interp
                        .native_c_variable_name_protocol()
                        .is_some_and(|protocol| protocol.element_table_retains_original())
                {
                    1
                } else {
                    0
                }
            );
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_array_for_successful_rows_read_member_names_only_after_array_selection() {
    // naming.invocation.original-variable-and-introspection-getter-order

    // Only C9 successful505b7 code/bytes/tape. The software identity checks
    // below use actual stored objects, without a public capture identity claim.
    for &(profile, capture) in &ORIGINALS[3..5] {
        let mut interp = native(profile);
        assert_eq!(interp.eval_str(b"array set ::arr {k VALUE}"), Code::Ok);
        reset(0, 0, &[(b'A', b"::k"), (b'B', b"::v"), (b'C', b"::arr")]);
        let head = text(b"array");
        let sub = text(b"for");
        let key_name = lazy(b'A');
        let value_name = lazy(b'B');
        let name = lazy(b'C');
        let variables = list(&interp, &[key_name.as_ptr(), value_name.as_ptr()]);
        let body = text(b"");
        let before = unsafe { (*key_name.as_ptr()).ref_count };
        let current = Current::enter(&mut interp);
        let code = array_cmd(
            &mut interp,
            &[
                head.as_ptr(),
                sub.as_ptr(),
                variables.as_ptr(),
                name.as_ptr(),
                body.as_ptr(),
            ],
        );
        compare_public(&mut interp, code, capture, 7);
        assert_eq!(
            crate::interp::obj_bytes(interp.var_get(b"k").unwrap()),
            b"k"
        );
        assert_eq!(
            crate::interp::obj_bytes(interp.var_get(b"v").unwrap()),
            b"VALUE"
        );
        assert_eq!(unsafe { (*key_name.as_ptr()).ref_count }, before);
        drop(current);
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_array_for_selected_rejections_leave_later_member_getters_unreached() {
    // naming.invocation.original-variable-and-introspection-getter-order

    // Direct selected worker source stages only: public505b error context
    // reads extra argv before VECTOR_RETURN and is intentionally not compared.
    for profile in ["tcl9.0", "tcl9.1"] {
        for malformed in [true, false] {
            let mut interp = native(profile);
            reset(0, 0, &[(b'A', b"::k"), (b'B', b"::v"), (b'C', b"::arr")]);
            let head = text(b"array");
            let sub = text(b"for");
            let key_name = lazy(b'A');
            let value_name = lazy(b'B');
            let name = lazy(b'C');
            let one = [key_name.as_ptr()];
            let two = [key_name.as_ptr(), value_name.as_ptr()];
            let variables = list(&interp, if malformed { &one } else { &two });
            let body = text(b"");
            let current = Current::enter(&mut interp);
            assert_eq!(
                array_cmd(
                    &mut interp,
                    &[
                        head.as_ptr(),
                        sub.as_ptr(),
                        variables.as_ptr(),
                        name.as_ptr(),
                        body.as_ptr()
                    ]
                ),
                Code::Error
            );
            assert_eq!(
                EVENTS.with(|v| v.borrow().clone()),
                if malformed { vec![] } else { vec![b'C'] }
            );
            assert!(!obj::has_string_rep(key_name.as_ptr()));
            assert!(!obj::has_string_rep(value_name.as_ptr()));
            assert!(!interp.host_refusal_pending());
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_array_operand_refusal_keeps_prefix_effect_and_stops_later_key() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Software-only actual extension updater and accepted pair prefix.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [1, 2] {
            let mut interp = native(profile);
            reset(
                mode,
                b'C',
                &[
                    (b'A', b"::arr"),
                    (b'B', b"k"),
                    (b'C', b"m"),
                    (b'D', b"later"),
                ],
            );
            let head = text(b"array");
            let sub = text(b"set");
            let name = lazy(b'A');
            let key = lazy(b'B');
            let refused = lazy(b'C');
            let later = lazy(b'D');
            let first = text(b"FIRST");
            let second = text(b"SECOND");
            let third = text(b"THIRD");
            let values = list(
                &interp,
                &[
                    key.as_ptr(),
                    first.as_ptr(),
                    refused.as_ptr(),
                    second.as_ptr(),
                    later.as_ptr(),
                    third.as_ptr(),
                ],
            );
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            let refs = unsafe { (*first.as_ptr()).ref_count };
            let current = Current::enter(&mut interp);
            assert_eq!(
                array_cmd(
                    &mut interp,
                    &[head.as_ptr(), sub.as_ptr(), name.as_ptr(), values.as_ptr()]
                ),
                Code::Error
            );
            assert_eq!(EVENTS.with(|v| v.borrow().clone()), b"ABC");
            assert!(!obj::has_string_rep(later.as_ptr()));
            // Inspect reached storage without issuing another name operation
            // after the original Host refusal. The guarded read is terminal.
            let first_cause = interp.native_execution_refusal().unwrap();
            assert!(interp.var_get_elem(b"arr", b"k").is_none());
            assert_eq!(
                crate::vars::get_elem(
                    &interp.frames.borrow(),
                    &interp.namespaces(),
                    interp.current_ns(),
                    b"arr",
                    b"k",
                ),
                Some(first.as_ptr()),
            );
            assert!(crate::vars::get_elem(
                &interp.frames.borrow(),
                &interp.namespaces(),
                interp.current_ns(),
                b"arr",
                b"m",
            )
            .is_none());
            assert_eq!(interp.native_execution_refusal(), Some(first_cause));
            assert_eq!(unsafe { (*first.as_ptr()).ref_count }, refs + 1);
            assert_eq!(interp.get_obj_result(), prior);
            assert!(interp.host_refusal_pending());
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

extern "C" fn retirement(_original: *mut TclObj) {
    RETIRED.with(|v| v.set(v.get() + 1));
    let mut interp = (unsafe { crate::codegen_abi::current_interp().as_ref() })
        .unwrap()
        .clone();
    let frames_released = interp.frames.try_borrow_mut().is_ok();
    // Taking this actual mutable owner fails if the reached store kept a borrow.
    drop(interp.namespaces_mut());
    RELEASED.with(|v| v.set(frames_released));
    NEW_VISIBLE
        .with(|v| v.set(interp.var_get_elem(b"arr", b"k") == Some(EXPECTED.with(Cell::get))));
    refuse(&mut interp);
}
static RETIRE: obj::TclObjType = obj::TclObjType {
    name: c"originalArrayValueRetirement".as_ptr(),
    free_int_rep_proc: Some(retirement),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
struct Write;
impl NativeVariableObserver<Interp> for Write {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _interp: &mut Interp,
        _access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        WRITES.with(|v| v.set(v.get() + 1));
        Ok(())
    }
}

#[test]
fn original_array_set_old_value_retirement_stops_before_write_observer() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual stored cell/native free hook, software callback/refcount scope.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..=2 {
            let mut interp = native(profile);
            let original = text(b"arr(k)");
            let old = Owned::fresh(obj::alloc_typed(&RETIRE, 0));
            unsafe { obj::set_native_updater_string_rep(old.as_ptr(), b"OLD", false) };
            interp
                .assign_original_named_variable(original.as_ptr(), old.as_ptr())
                .unwrap();
            drop(old);
            interp
                .add_native_variable_observer(
                    original.as_ptr(),
                    &[NativeVariableTraceOperation::Write],
                    Rc::new(Write),
                )
                .unwrap();
            let next = text(b"NEXT");
            let key = text(b"k");
            let name = text(b"arr");
            let values = list(&interp, &[key.as_ptr(), next.as_ptr()]);
            let head = text(b"array");
            let sub = text(b"set");
            reset(mode, 0, &[]);
            EXPECTED.with(|v| v.set(next.as_ptr()));
            let refs = unsafe { (*next.as_ptr()).ref_count };
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            let current = Current::enter(&mut interp);
            assert_eq!(
                array_cmd(
                    &mut interp,
                    &[head.as_ptr(), sub.as_ptr(), name.as_ptr(), values.as_ptr()]
                ),
                if mode == 0 { Code::Ok } else { Code::Error }
            );
            assert_eq!(RETIRED.with(Cell::get), 1);
            assert!(RELEASED.with(Cell::get));
            assert!(NEW_VISIBLE.with(Cell::get));
            assert_eq!(WRITES.with(Cell::get), usize::from(mode == 0));
            assert_eq!(unsafe { (*next.as_ptr()).ref_count }, refs + 1);
            if mode != 0 {
                assert_eq!(interp.get_obj_result(), prior);
                assert!(interp.host_refusal_pending());
            }
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_array_for_assigns_actual_retained_key_without_synthesizing_a_header() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Software physical identity/capability control;505b public rows do not
    // attest key identity, key sharing, or arbitrary iterator equivalence.
    for profile in ["tcl9.0", "tcl9.1"] {
        for actual_key in [true, false] {
            let mut interp = native(profile);
            let name = text(b"arr");
            let key = text(b"k");
            let value = text(b"VALUE");
            if actual_key {
                let values = list(&interp, &[key.as_ptr(), value.as_ptr()]);
                interp
                    .set_original_c_array(name.as_ptr(), values.as_ptr())
                    .unwrap();
            } else {
                // Explicit byte compatibility producer has no original key receipt.
                interp.var_set_elem(b"arr", b"k", value.as_ptr()).unwrap();
            }
            let head = text(b"array");
            let sub = text(b"for");
            let destination = text(b"key");
            let result = text(b"value");
            let variables = list(&interp, &[destination.as_ptr(), result.as_ptr()]);
            let body = text(b"");
            let before = unsafe { (*key.as_ptr()).ref_count };
            let current = Current::enter(&mut interp);
            let code = array_cmd(
                &mut interp,
                &[
                    head.as_ptr(),
                    sub.as_ptr(),
                    variables.as_ptr(),
                    name.as_ptr(),
                    body.as_ptr(),
                ],
            );
            if actual_key {
                assert_eq!(code, Code::Ok);
                assert_eq!(interp.var_get(b"key"), Some(key.as_ptr()));
                assert_eq!(interp.var_get(b"value"), Some(value.as_ptr()));
                assert_eq!(unsafe { (*key.as_ptr()).ref_count }, before + 1);
            } else {
                assert_eq!(code, Code::Error);
                assert!(interp.host_refusal_pending());
                assert!(interp.var_get(b"key").is_none());
                assert_eq!(unsafe { (*key.as_ptr()).ref_count }, before);
            }
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_jim_array_set_existing_dictionary_checks_each_reached_key_and_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual original Jim root and shared conversion/mutation worker. No new
    // external existing-root callback/order or reference-count equivalence.
    for mode in [1, 2] {
        let mut interp = native("jim");
        let name = text(b"arr");
        let first = text(b"FIRST");
        let old = text(b"OLD");
        let seed = list(&interp, &[text(b"seed").as_ptr(), old.as_ptr()]);
        interp
            .assign_original_named_variable(name.as_ptr(), seed.as_ptr())
            .unwrap();
        // Convert the exact actual borrowed variable root to Dictionary first.
        crate::dict::ensure_dict_native(
            seed.as_ptr(),
            interp
                .native_invocation_dialect()
                .native_string_protocol()
                .unwrap(),
        )
        .unwrap();
        drop(seed);
        reset(mode, b'C', &[(b'B', b"k"), (b'C', b"m"), (b'D', b"later")]);
        let head = text(b"array");
        let sub = text(b"set");
        let key = lazy(b'B');
        let refused = lazy(b'C');
        let later = lazy(b'D');
        let next = text(b"NEXT");
        let values = list(
            &interp,
            &[
                key.as_ptr(),
                first.as_ptr(),
                refused.as_ptr(),
                next.as_ptr(),
                later.as_ptr(),
                next.as_ptr(),
            ],
        );
        interp.set_result_bytes(b"PRIOR");
        let prior = interp.get_obj_result();
        let root = interp.var_get(b"arr").unwrap();
        let refs = unsafe { (*root).ref_count };
        let current = Current::enter(&mut interp);
        assert_eq!(
            array_cmd(
                &mut interp,
                &[head.as_ptr(), sub.as_ptr(), name.as_ptr(), values.as_ptr()]
            ),
            Code::Error
        );
        assert_eq!(EVENTS.with(|v| v.borrow().clone()), b"BC");
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(
            crate::dict::dict_get(root, b"k").unwrap(),
            Some(first.as_ptr())
        );
        assert_eq!(unsafe { (*root).ref_count }, refs);
        assert_eq!(interp.get_obj_result(), prior);
        assert!(interp.host_refusal_pending());
        drop(current);
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}
