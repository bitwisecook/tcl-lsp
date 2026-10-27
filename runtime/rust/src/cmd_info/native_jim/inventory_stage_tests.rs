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

use crate::interp::{Code, Interp};
use crate::obj::{self, Owned, TclObj};
use std::cell::{Cell, RefCell};
use tcl_syntax::value::ValueOps;
thread_local! {
    static NAMESPACE: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static OBSERVED: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
    static CHANGE: RefCell<Option<Interp>> = const { RefCell::new(None) };
}
extern "C" fn update_pattern(original: *mut TclObj) {
    OBSERVED.with(|state| {
        state
            .borrow_mut()
            .push(NAMESPACE.with(|object| obj::has_string_rep(object.get())))
    });
    CHANGE.with(|state| {
        if let Some(interp) = state.borrow().as_ref() {
            interp.set_current_ns(crate::namespace::GLOBAL);
        }
    });
    // SAFETY: the actual updater owns this original object's absent String slot.
    unsafe {
        obj::set_string_rep(original, b"::42::target");
    }
}
static PATTERN: obj::TclObjType = obj::TclObjType {
    name: c"inventory-argument-probe".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_pattern),
    set_from_any_proc: None,
};
fn native_jim() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn type_name(object: *mut TclObj) -> Vec<u8> {
    let kind = obj::obj_type_ptr(object);
    if kind.is_null() {
        return b"none".to_vec();
    }
    // SAFETY: the retained actual object has its original static descriptor.
    unsafe { std::ffi::CStr::from_ptr((*kind).name) }
        .to_bytes()
        .to_vec()
}
fn invoke(interp: &mut Interp, namespace: *mut TclObj, words: &[*mut TclObj]) -> Code {
    let vector = Owned::fresh(interp.new_list_object(words));
    let current = interp
        .namespaces_mut()
        .retain_unmaterialized_jim_namespace(Owned::retain(namespace));
    interp.ns_eval_in_token(
        current,
        &[],
        None,
        false,
        Some((
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
            vector.as_ptr(),
        )),
        None,
    )
}
fn exercise_inventory_cases(
    cases: &[(&str, bool, usize, bool, bool)],
    evidence: &str,
    replacement: bool,
) {
    let row = |kind: &str, phase: &str| {
        let mut rows = evidence
            .lines()
            .map(|line| line.split('|').collect::<Vec<_>>())
            .filter(|fields| fields.first() == Some(&kind) && fields.get(1) == Some(&phase));
        let selected = rows.next().expect("original inventory stage row");
        assert!(rows.next().is_none());
        selected
    };
    for &(label, nons, count, all, rooted) in cases {
        let mut interp = native_jim();
        assert_eq!(interp.eval_str(b"proc 42::target {} {}"), Code::Ok);
        let context = interp.native_jim_object_context().unwrap();
        let namespace = Owned::fresh(obj::new_wide_int_obj(42));
        let head = Owned::fresh(obj::new_string_bytes(if rooted {
            b"::info"
        } else {
            b"info"
        }));
        let nons_word = Owned::fresh(obj::new_string_bytes(b"-nons"));
        let member = Owned::fresh(obj::new_string_bytes(b"commands"));
        let all_word = Owned::fresh(obj::new_string_bytes(b"-all"));
        let other = Owned::fresh(obj::new_string_bytes(b"other"));
        for object in [
            namespace.as_ptr(),
            head.as_ptr(),
            nons_word.as_ptr(),
            member.as_ptr(),
            all_word.as_ptr(),
            other.as_ptr(),
        ] {
            crate::native_source::bind_context(object, &context).unwrap();
        }
        NAMESPACE.with(|state| state.set(namespace.as_ptr()));
        for index in 0..if replacement { 3 } else { 2 } {
            let hot = index != 0;
            if replacement && index == 1 {
                assert_eq!(interp.eval_str(b"rename {namespace info} {namespace info original}; proc {namespace info} {cmd {pattern *}} {return REPLACED}"), Code::Ok);
            }
            if replacement && index == 2 {
                assert_eq!(interp.eval_str(b"rename {namespace info} {}"), Code::Ok);
            }
            let suffix = if replacement {
                ["DEFAULT", "REPLACED", "DELETED"][index]
            } else if hot {
                "HOT"
            } else {
                "COLD"
            };
            let phase = format!("{label}_{suffix}");
            if hot {
                obj::invalidate_string(namespace.as_ptr());
            }
            let pattern = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
            crate::native_source::bind_context(pattern.as_ptr(), &context).unwrap();
            let mut words = vec![head.as_ptr()];
            if nons {
                words.push(nons_word.as_ptr());
            }
            words.push(member.as_ptr());
            if count > 0 {
                words.push(if all {
                    all_word.as_ptr()
                } else {
                    pattern.as_ptr()
                });
            }
            if count > 1 {
                words.push(if all {
                    pattern.as_ptr()
                } else {
                    other.as_ptr()
                });
            }
            for (kind, object) in [("NAMESPACE", namespace.as_ptr()), ("ARG", pattern.as_ptr())] {
                assert_eq!(
                    obj::has_string_rep(object),
                    row(&format!("{kind}_BEFORE"), &phase)[2] == "1",
                    "{phase}/{kind}"
                );
                assert_eq!(
                    type_name(object),
                    row(&format!("{kind}_BEFORE"), &phase)[3].as_bytes()
                );
            }
            OBSERVED.with(|state| state.borrow_mut().clear());
            let code = invoke(&mut interp, namespace.as_ptr(), &words);
            let expected = evidence
                .lines()
                .map(|line| line.split('|').collect::<Vec<_>>())
                .filter(|fields| {
                    fields.first() == Some(&"NAMESPACE_AT_ARG_GETTER")
                        && fields.get(1) == Some(&phase.as_str())
                })
                .map(|fields| fields[2] == "1")
                .collect::<Vec<_>>();
            assert_eq!(
                OBSERVED.with(|state| state.borrow().clone()),
                expected,
                "{phase}"
            );
            for (kind, object) in [("NAMESPACE", namespace.as_ptr()), ("ARG", pattern.as_ptr())] {
                assert_eq!(
                    obj::has_string_rep(object),
                    row(&format!("{kind}_AFTER"), &phase)[2] == "1",
                    "{phase}/{kind}"
                );
                assert_eq!(
                    type_name(object),
                    row(&format!("{kind}_AFTER"), &phase)[3].as_bytes()
                );
            }
            let result = row("RESULT", &phase);
            assert_eq!(code.as_int().to_string(), result[2], "{phase}");
            if count == 0 {
                // SDK distributions have independent complete command rosters;
                // this header-stage control observes the real selected target.
                let result = interp.result_obj();
                let names = interp.list_elements(&result).unwrap();
                assert!(names.into_iter().any(|name| interp
                    .native_string_bytes(&name)
                    .unwrap()
                    .as_ref()
                    == b"42::target"));
            } else {
                let expected = result[3]
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(interp.result_bytes(), expected, "{phase}");
            }
            assert!(interp.native_execution_refusal().is_none(), "{phase}");
        }
        interp.set_current_ns(crate::namespace::GLOBAL);
        NAMESPACE.with(|state| state.set(std::ptr::null_mut()));
    }
}
#[test]
fn original_jim_inventory_preflight_preserves_nons_and_worker_arity_namespace_getter_order() {
    let evidence =
        include_str!("../../../tests/data/native_jim_inventory_stage/getter-order-20261015.tsv");
    assert_eq!(evidence.lines().count(), 105);
    exercise_inventory_cases(
        &[
            ("RAW_PATTERN", true, 1, false, false),
            ("RAW_NONE", true, 0, false, false),
            ("RAW_ALL", true, 2, true, false),
            ("RAW_ARITY", true, 2, false, false),
            ("AWARE_ARITY", false, 2, false, false),
            ("ROOTED_RAW_PATTERN", true, 1, false, true),
            ("ROOTED_AWARE_ARITY", false, 2, false, true),
        ],
        evidence,
        false,
    );
}
#[test]
fn original_jim_inventory_default_helper_matches_original_namespace_aware_pattern_stages() {
    let evidence =
        include_str!("../../../tests/data/native_jim_inventory_stage/getter-order-20261015.tsv");
    assert_eq!(evidence.lines().count(), 105);
    exercise_inventory_cases(
        &[
            ("AWARE_PATTERN", false, 1, false, false),
            ("AWARE_ALL", false, 2, true, false),
        ],
        evidence,
        false,
    );
}
#[test]
fn original_jim_inventory_reaches_actual_replaced_and_deleted_info_helper() {
    let evidence = include_str!(
        "../../../tests/data/native_jim_inventory_stage/helper-replacement-20261015.tsv"
    );
    assert_eq!(evidence.lines().count(), 19);
    exercise_inventory_cases(
        &[("AWARE_REPLACEMENT", false, 1, false, false)],
        evidence,
        true,
    );
}
#[test]
fn original_jim_inventory_stops_after_actual_pattern_getter_withdraws_current_namespace() {
    let mut interp = native_jim();
    assert_eq!(interp.eval_str(b"proc 42::target {} {}"), Code::Ok);
    let context = interp.native_jim_object_context().unwrap();
    let namespace = Owned::fresh(obj::new_wide_int_obj(42));
    let head = Owned::fresh(obj::new_string_bytes(b"info"));
    let nons = Owned::fresh(obj::new_string_bytes(b"-nons"));
    let member = Owned::fresh(obj::new_string_bytes(b"commands"));
    for object in [
        namespace.as_ptr(),
        head.as_ptr(),
        nons.as_ptr(),
        member.as_ptr(),
    ] {
        crate::native_source::bind_context(object, &context).unwrap();
    }
    NAMESPACE.with(|state| state.set(namespace.as_ptr()));
    let cold = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
    crate::native_source::bind_context(cold.as_ptr(), &context).unwrap();
    assert_eq!(
        invoke(
            &mut interp,
            namespace.as_ptr(),
            &[head.as_ptr(), nons.as_ptr(), member.as_ptr(), cold.as_ptr()]
        ),
        Code::Ok
    );
    obj::invalidate_string(namespace.as_ptr());
    let pattern = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
    crate::native_source::bind_context(pattern.as_ptr(), &context).unwrap();
    OBSERVED.with(|state| state.borrow_mut().clear());
    CHANGE.with(|state| *state.borrow_mut() = Some(interp.clone()));
    let code = invoke(
        &mut interp,
        namespace.as_ptr(),
        &[
            head.as_ptr(),
            nons.as_ptr(),
            member.as_ptr(),
            pattern.as_ptr(),
        ],
    );
    CHANGE.with(|state| *state.borrow_mut() = None);
    assert_eq!(code, Code::Error);
    assert_eq!(OBSERVED.with(|state| state.borrow().clone()), [false]);
    assert!(!obj::has_string_rep(namespace.as_ptr()));
    assert!(obj::has_string_rep(pattern.as_ptr()));
    let Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(first),
    )) = interp.native_execution_refusal()
    else {
        panic!("actual namespace withdrawal must retain its first Host refusal")
    };
    interp.refuse_native_execution(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "later inventory refusal",
        ),
    ));
    assert!(
        matches!(interp.native_execution_refusal(),Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(current))) if current==first)
    );
    assert_eq!(interp.current_ns(), crate::namespace::GLOBAL);
    NAMESPACE.with(|state| state.set(std::ptr::null_mut()));
}

#[test]
fn zz_probe_jim_pattern_conversion() {
    let mut interp = native_jim();
    let context = interp.native_jim_object_context().unwrap();
    assert_eq!(interp.eval_str(b"proc 42::target {} {}; set 42::v 1"), Code::Ok);
    let anchor = Owned::fresh(obj::new_string_bytes(b"anchor"));
    NAMESPACE.with(|state| state.set(anchor.as_ptr()));
    for script in [&b"string first"[..], b"string length", b"string match"] {
        let pattern = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
        crate::native_source::bind_context(pattern.as_ptr(), &context).unwrap();
        let mut words: Vec<Owned> = script
            .split(|b| *b == b' ')
            .map(|w| Owned::fresh(obj::new_string_bytes(w)))
            .collect();
        if script != b"string length" {
            words.push(Owned::fresh(obj::new_string_bytes(b"::")));
        }
        for w in &words {
            crate::native_source::bind_context(w.as_ptr(), &context).unwrap();
        }
        let mut argv: Vec<*mut TclObj> = words.iter().map(Owned::as_ptr).collect();
        argv.push(pattern.as_ptr());
        let code = interp.eval_original_object_vector(&argv);
        eprintln!(
            "ZZJIM {:?} code={code:?} type={:?}",
            String::from_utf8_lossy(script),
            String::from_utf8_lossy(&type_name(pattern.as_ptr()))
        );
    }
    for member in [&b"commands"[..], b"vars"] {
        let namespace = Owned::fresh(obj::new_wide_int_obj(42));
        let head = Owned::fresh(obj::new_string_bytes(b"info"));
        let member = Owned::fresh(obj::new_string_bytes(member));
        let pattern = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
        for object in [namespace.as_ptr(), head.as_ptr(), member.as_ptr(), pattern.as_ptr()] {
            crate::native_source::bind_context(object, &context).unwrap();
        }
        NAMESPACE.with(|state| state.set(namespace.as_ptr()));
        OBSERVED.with(|state| state.borrow_mut().clear());
        let code = invoke(&mut interp, namespace.as_ptr(), &[head.as_ptr(), member.as_ptr(), pattern.as_ptr()]);
        let result = interp.result_obj();
        eprintln!(
            "ZZJIM info {:?} code={code:?} observed={:?} type={:?} result={:?} refusal={:?}",
            obj::bytes_of(member.as_ptr()),
            OBSERVED.with(|state| state.borrow().clone()),
            String::from_utf8_lossy(&type_name(pattern.as_ptr())),
            interp.native_string_bytes(&result).map(|b| String::from_utf8_lossy(&b).into_owned()),
            interp.native_access_refusal(),
        );
    }
}
