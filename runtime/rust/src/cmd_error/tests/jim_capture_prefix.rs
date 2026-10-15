// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::{
    interp::{Code, Interp},
    obj::{self, Owned, TclObj},
};
use std::cell::{Cell, RefCell};

thread_local! {
    static ACTUAL: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static SELECTOR: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static BODY_CODE: Cell<i32> = const { Cell::new(0) };
    static BODIES: Cell<usize> = const { Cell::new(0) };
    static HANDLERS: Cell<usize> = const { Cell::new(0) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
}

extern "C" fn update(original: *mut TclObj) {
    let mut actual = ACTUAL.with(|slot| slot.borrow().as_ref().unwrap().clone());
    GETTERS.with(|count| count.set(count.get() + 1));
    actual.set_result_bytes(b"GETTER");
    let bytes = match obj::internal_rep(original) {
        0 => SELECTOR.with(|word| word.borrow().clone()),
        1 => b"body_probe".to_vec(),
        2 => b"on".to_vec(),
        _ => unreachable!(),
    };
    // SAFETY: the entered actual getter owns the live extension header.
    unsafe { obj::set_native_updater_string_rep(original, &bytes, false) };
}

static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"capturePrefixStage".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};

fn body(actual: &mut Interp, _: &[*mut TclObj]) -> Code {
    BODIES.with(|count| count.set(count.get() + 1));
    actual.set_result_bytes(b"BODY");
    Code::from_int(BODY_CODE.with(Cell::get))
}

fn handler(actual: &mut Interp, _: &[*mut TclObj]) -> Code {
    HANDLERS.with(|count| count.set(count.get() + 1));
    actual.set_result_bytes(b"HANDLED");
    Code::Ok
}

fn jim() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn original_jim_try_prefix_cells_use_bounded_native_getters_and_refuse_unbound_shifts() {
    // Actual Runtime extension stages, independently of the Jim ObjType ABI.
    let cases =
        include_str!("../../../tests/data/native_jim_try_leading_switches/cases-20261015.tsv");
    let observations = include_str!(
        "../../../tests/data/native_jim_try_leading_switches/observations-20261015.tsv"
    );
    for row in observations.lines().filter(|row| row.starts_with("OBS|")) {
        let fields = row.split('|').collect::<Vec<_>>();
        let case: usize = fields[1].parse().unwrap();
        let input = cases
            .lines()
            .nth(case)
            .unwrap()
            .split('|')
            .collect::<Vec<_>>();
        let selector_bytes = decode(input[1]);
        SELECTOR.with(|word| *word.borrow_mut() = selector_bytes.clone());
        BODY_CODE.with(|code| code.set(fields[2].parse().unwrap()));
        BODIES.with(|count| count.set(0));
        HANDLERS.with(|count| count.set(0));
        GETTERS.with(|count| count.set(0));
        let mut actual = jim();
        actual.register_builtin(b"body_probe", body);
        actual.register_builtin(b"handler_probe", handler);
        let head = Owned::fresh(obj::new_string_bytes(b"try"));
        let selector = Owned::fresh(if input[2] == "1" {
            obj::alloc_typed(&ORIGINAL, 0)
        } else {
            obj::new_string_bytes(&selector_bytes)
        });
        let body = Owned::fresh(if input[3] == "1" {
            obj::alloc_typed(&ORIGINAL, 1)
        } else {
            obj::new_string_bytes(b"body_probe")
        });
        let on = Owned::fresh(if input[4] == "1" {
            obj::alloc_typed(&ORIGINAL, 2)
        } else {
            obj::new_string_bytes(b"on")
        });
        let code = Owned::fresh(obj::new_string_bytes(fields[2].as_bytes()));
        let variables = Owned::fresh(obj::new_string_bytes(b"message options"));
        let script = Owned::fresh(obj::new_string_bytes(b"handler_probe"));
        let argv = [
            head.as_ptr(),
            selector.as_ptr(),
            body.as_ptr(),
            on.as_ptr(),
            code.as_ptr(),
            variables.as_ptr(),
            script.as_ptr(),
        ];
        ACTUAL.with(|slot| *slot.borrow_mut() = Some(actual.clone()));
        let reached = actual.eval_original_object_vector(&argv);
        ACTUAL.with(|slot| *slot.borrow_mut() = None);
        if matches!(case, 2 | 3 | 4 | 5 | 15) {
            // These original-build positives remain unimplemented Native cases.
            assert!(actual.native_execution_refusal().is_some(), "{row}");
            assert_eq!(BODIES.with(Cell::get), 0, "{row}");
            assert_eq!(HANDLERS.with(Cell::get), 0, "{row}");
            continue;
        }
        assert!(
            actual.native_execution_refusal().is_none(),
            "{row}: {:?}",
            actual.native_execution_refusal()
        );
        assert_eq!(reached.as_int().to_string(), fields[3], "{row}");
        assert_eq!(BODIES.with(Cell::get).to_string(), fields[4], "{row}");
        assert_eq!(HANDLERS.with(Cell::get).to_string(), fields[5], "{row}");
        assert_eq!(GETTERS.with(Cell::get).to_string(), fields[6], "{row}");
        if fields[4] == "1" {
            assert_eq!(actual.result_bytes(), decode(fields[7]), "{row}");
        }
        assert_eq!(
            actual.var_get(b"message").is_ok(),
            fields[8] == "1",
            "{row}"
        );
        if case == 22 {
            assert!(!obj::has_string_rep(on.as_ptr()), "{row}");
        }
    }
}

#[test]
fn original_jim_catch_prefix_cannot_use_source_wrapping_as_native_execution() {
    let mut actual = jim();
    actual.register_builtin(b"body_probe", body);
    BODIES.with(|count| count.set(0));
    let words = [b"catch".as_slice(), b"-no64", b"body_probe"]
        .map(|word| Owned::fresh(obj::new_string_bytes(word)));
    let reached = actual.eval_original_object_vector(&words.each_ref().map(Owned::as_ptr));
    assert_eq!(reached, Code::Error);
    assert_eq!(BODIES.with(Cell::get), 0);
    assert!(actual.native_execution_refusal().is_some());
}
