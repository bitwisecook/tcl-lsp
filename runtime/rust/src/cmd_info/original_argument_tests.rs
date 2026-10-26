// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original info operands and selected destination assignment stages.
use super::*;
use crate::obj::{self, Owned};
use std::cell::{Cell, RefCell};
use tcl_runtime_api::Procs;

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
    static REFUSAL: Cell<u8> = const { Cell::new(0) };
    static DEFAULT: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static DEFAULT_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
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
extern "C" fn getter(original: *mut TclObj) {
    let raw_label = unsafe { (*original).internal_rep } as u8;
    let label = raw_label & 0x7f;
    EVENTS.with(|events| events.borrow_mut().push(label));
    if label == b'C' {
        let default = DEFAULT.with(Cell::get);
        if !default.is_null() {
            DEFAULT_REFS.with(|value| value.set(unsafe { (*default).ref_count }));
        }
    }
    let mut interp = (unsafe { crate::codegen_abi::current_interp().as_ref() })
        .unwrap()
        .clone();
    match REFUSAL.with(Cell::get) {
        1 => {
            interp.refuse_host_command("first original info operand getter");
        }
        2 => {
            let actual = interp.runtime_context();
            let mut changed = actual.clone();
            changed.packages = vec![("original-info-getter".to_owned(), "1.0".to_owned())];
            interp.pin_context(&changed).unwrap();
            interp.pin_context(&actual).unwrap();
        }
        _ => {}
    }
    let bytes = match label {
        b'A' if raw_label & 0x80 != 0 => b"::target".as_slice(),
        b'A' => b"::p".as_slice(),
        b'B' => b"x".as_slice(),
        b'C' => b"::target".as_slice(),
        b'T' => b"::target".as_slice(),
        _ => b"::missing".as_slice(),
    };
    unsafe { obj::set_native_updater_string_rep(original, bytes, false) };
}
static LAZY: obj::TclObjType = obj::TclObjType {
    name: c"originalInfoOperand".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(getter),
    set_from_any_proc: None,
};
fn lazy(label: u8) -> Owned {
    Owned::fresh(obj::alloc_typed(&LAZY, u64::from(label)))
}
fn reset(mode: u8) {
    EVENTS.with(|events| events.borrow_mut().clear());
    REFUSAL.with(|value| value.set(mode));
    DEFAULT.with(|value| value.set(std::ptr::null_mut()));
    DEFAULT_REFS.with(|value| value.set(-1));
}
fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn row<'a>(capture: &'a str, case: usize, phase: &str) -> Vec<&'a str> {
    capture
        .lines()
        .find(|line| line.starts_with(&format!("ROW|{case}|{phase}|")))
        .unwrap()
        .split('|')
        .collect()
}
fn compare_public(interp: &mut Interp, code: Code, capture: &str, case: usize) {
    let selected = row(capture, case, "worker");
    assert_eq!(code.as_int(), selected[3].parse::<i64>().unwrap());
    assert!(!interp.host_refusal_pending());
    assert_eq!(interp.result_bytes(), decode(selected[4]));
    let labels: Vec<_> = capture
        .lines()
        .filter_map(|line| {
            let words: Vec<_> = line.split('|').collect();
            (words.first() == Some(&"EVENT")
                && words[1].parse::<usize>() == Ok(case)
                && words.last() == Some(&"phase=worker"))
            .then(|| words[3].as_bytes()[0])
        })
        .collect();
    assert_eq!(EVENTS.with(|events| events.borrow().clone()), labels);
}

#[test]
fn original_info_default_positive_rows_keep_selected_getter_order_and_borrowed_value() {

    // Only C505b successful cases2/3 public bytes/tape; native reference counts,
    // error-context getters, dispatch ownership and Jim default stay separate.
    for &(profile, capture) in &ORIGINALS[..5] {
        for case in [2, 3] {
            let mut interp = native(profile);
            assert_eq!(
                interp.eval_str(if case == 2 {
                    b"proc ::p {{x VALUE}} {}".as_slice()
                } else {
                    b"proc ::p {x} {}".as_slice()
                }),
                Code::Ok
            );
            reset(0);
            let actual = if case == 2 {
                match interp.proc_default_value_bytes(b"::p", b"x").unwrap() {
                    tcl_runtime_api::ProcDefaultValue::Declared(Some(value)) => value,
                    _ => panic!("actual declared default"),
                }
            } else {
                std::ptr::null_mut()
            };
            DEFAULT.with(|value| value.set(actual));
            let before = (!actual.is_null()).then(|| unsafe { (*actual).ref_count });
            let head = text(b"info");
            let sub = text(b"default");
            let procedure = lazy(b'A');
            let formal = lazy(b'B');
            let destination = lazy(b'C');
            let current = Current::enter(&mut interp);
            let code = info_default(
                &mut interp,
                &[
                    head.as_ptr(),
                    sub.as_ptr(),
                    procedure.as_ptr(),
                    formal.as_ptr(),
                    destination.as_ptr(),
                ],
            );
            compare_public(&mut interp, code, capture, case);
            let selected = interp.var_get(b"target").unwrap();
            assert_eq!(
                crate::interp::obj_bytes(selected),
                if case == 2 {
                    b"VALUE".as_slice()
                } else {
                    b"".as_slice()
                }
            );
            if let Some(before) = before {
                assert_eq!(DEFAULT_REFS.with(Cell::get), before);
            }
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_info_default_failed_lookup_leaves_destination_getter_unreached() {

    // Selected InfoDefaultCmd source/API stage only. Actual505b EvalObjv error
    // context reads destinationC; this direct worker does not assert that tape.
    for &(profile, _) in &ORIGINALS[..5] {
        for missing_formal in [false, true] {
            let mut interp = native(profile);
            assert_eq!(interp.eval_str(b"proc ::p {{x VALUE}} {}"), Code::Ok);
            reset(0);
            let head = text(b"info");
            let sub = text(b"default");
            let procedure = if missing_formal {
                lazy(b'A')
            } else {
                lazy(b'M')
            };
            let formal = if missing_formal {
                text(b"bad")
            } else {
                lazy(b'B')
            };
            let destination = lazy(b'C');
            let current = Current::enter(&mut interp);
            let code = info_default(
                &mut interp,
                &[
                    head.as_ptr(),
                    sub.as_ptr(),
                    procedure.as_ptr(),
                    formal.as_ptr(),
                    destination.as_ptr(),
                ],
            );
            assert_eq!(code, Code::Error);
            assert!(!interp.host_refusal_pending());
            assert!(!obj::has_string_rep(destination.as_ptr()));
            assert!(!interp.var_exists(b"target"));
            assert_eq!(
                EVENTS.with(|events| events.borrow().clone()),
                if missing_formal {
                    b"A".to_vec()
                } else {
                    b"MB".to_vec()
                }
            );
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_info_getter_first_cause_stops_later_operands_and_result_publication() {

    // Real extension updater Host/context changes; software completion transport.
    for mode in [1, 2] {
        let mut interp = native("tcl8.6");
        assert_eq!(interp.eval_str(b"proc ::p {{x VALUE}} {}"), Code::Ok);
        reset(mode);
        let head = text(b"info");
        let sub = text(b"default");
        let procedure = lazy(b'A');
        let formal = lazy(b'B');
        let destination = lazy(b'C');
        interp.set_result_bytes(b"PRIOR");
        let prior = interp.get_obj_result();
        let refs = unsafe { (*procedure.as_ptr()).ref_count };
        let current = Current::enter(&mut interp);
        assert_eq!(
            info_default(
                &mut interp,
                &[
                    head.as_ptr(),
                    sub.as_ptr(),
                    procedure.as_ptr(),
                    formal.as_ptr(),
                    destination.as_ptr()
                ]
            ),
            Code::Error
        );
        assert_eq!(EVENTS.with(|events| events.borrow().clone()), b"A");
        assert_eq!(unsafe { (*procedure.as_ptr()).ref_count }, refs);
        assert!(!obj::has_string_rep(formal.as_ptr()));
        assert!(!obj::has_string_rep(destination.as_ptr()));
        assert_eq!(interp.get_obj_result(), prior);
        assert!(!interp.var_exists(b"target"));
        let first = interp.native_execution_refusal().unwrap();
        assert_eq!(
            info_default(
                &mut interp,
                &[
                    head.as_ptr(),
                    sub.as_ptr(),
                    procedure.as_ptr(),
                    formal.as_ptr(),
                    destination.as_ptr()
                ]
            ),
            Code::Error
        );
        assert_eq!(interp.native_execution_refusal(), Some(first));
        assert_eq!(EVENTS.with(|events| events.borrow().clone()), b"A");
        drop(current);
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_info_exists_public_row_preserves_original_name_on_all_six_providers() {

    // Exact505b case17 name getter and public result only; no internal cache/ref equality.
    for &(profile, capture) in ORIGINALS {
        let mut interp = native(profile);
        assert_eq!(interp.eval_str(b"set ::target VALUE"), Code::Ok);
        reset(0);
        let head = text(b"info");
        let sub = text(b"exists");
        let name = lazy(b'A' | 0x80);
        let current = Current::enter(&mut interp);
        let code = info_exists(&mut interp, &[head.as_ptr(), sub.as_ptr(), name.as_ptr()]);
        compare_public(&mut interp, code, capture, 17);
        assert_eq!(
            crate::interp::obj_bytes(interp.var_get(b"target").unwrap()),
            b"VALUE"
        );
        drop(current);
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_info_direct_name_workers_stop_before_result_after_getter_refusal() {

    // Existing direct worker API and selected String producer. No native
    // availability, numeric frame parser or whole dispatcher equivalence.
    for mode in [1, 2] {
        for member in [
            b"script".as_slice(),
            b"frame",
            b"exists",
            b"cmdtype",
            b"constant",
        ] {
            let mut interp = native(if member == b"constant" {
                "tcl9.1"
            } else {
                "tcl8.6"
            });
            reset(mode);
            let head = text(b"info");
            let selector = text(member);
            let name = lazy(b'A');
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            let current = Current::enter(&mut interp);
            let argv = [head.as_ptr(), selector.as_ptr(), name.as_ptr()];
            let code = match member {
                b"frame" => info_frame(&mut interp, &argv),
                b"exists" => info_exists(&mut interp, &argv),
                b"cmdtype" => info_cmdtype(&mut interp, &argv),
                _ => info_cmd(&mut interp, &argv),
            };
            assert_eq!(code, Code::Error, "{member:?}/{mode}");
            assert_eq!(EVENTS.with(|events| events.borrow().clone()), b"A");
            assert!(interp.native_execution_refusal().is_some());
            assert_eq!(interp.get_obj_result(), prior);
            assert_eq!(unsafe { (*name.as_ptr()).ref_count }, 1);
            drop(current);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}
