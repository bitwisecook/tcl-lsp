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
    static RESULT_CHANGE: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static RESULT_RETIRED: Cell<usize> = const { Cell::new(0) };
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
    let result_owner = RESULT_CHANGE.with(|state| state.borrow().clone());
    if let Some(mut interp) = result_owner {
        let pending = Owned::fresh(obj::alloc_typed(&RESULT_CALLBACK, 0));
        interp.set_result(pending.as_ptr());
    }
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
extern "C" fn retire_result(_: *mut TclObj) {
    RESULT_RETIRED.with(|state| state.set(state.get() + 1));
    RESULT_CHANGE.with(|state| {
        if let Some(interp) = state.borrow().as_ref() {
            interp.set_current_ns(crate::namespace::GLOBAL);
        }
    });
}
static RESULT_CALLBACK: obj::TclObjType = obj::TclObjType {
    name: c"inventory-result-callback".as_ptr(),
    free_int_rep_proc: Some(retire_result),
    dup_int_rep_proc: None,
    update_string_proc: None,
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
fn evidence_row<'a>(evidence: &'a str, kind: &str, phase: &str) -> Vec<&'a str> {
    let mut rows = evidence
        .lines()
        .map(|line| line.split('|').collect::<Vec<_>>())
        .filter(|row| row.first() == Some(&kind) && row.get(1) == Some(&phase));
    let row = rows.next().expect("actual original inventory row");
    assert!(rows.next().is_none());
    row
}
fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn original_jim_variable_inventories_keep_raw_and_helper_namespace_frame_stages() {
    let evidence =
        include_str!("../../../tests/data/native_jim_variable_inventory/getter-order-20261015.tsv");
    assert_eq!(evidence.lines().count(), 133);
    for member in ["vars", "globals", "locals"] {
        for (mode, nons, count) in [
            ("RAW_PATTERN", true, 1),
            ("AWARE_PATTERN", false, 1),
            ("RAW_ARITY", true, 2),
            ("AWARE_ARITY", false, 2),
        ] {
            let label = format!("{}_{}", member.to_ascii_uppercase(), mode);
            let mut interp = native_jim();
            assert_eq!(
                interp.eval_str(b"set 42::target VALUE; set target ROOT"),
                Code::Ok
            );
            let context = interp.native_jim_object_context().unwrap();
            let namespace = Owned::fresh(obj::new_wide_int_obj(42));
            let head = Owned::fresh(obj::new_string_bytes(b"info"));
            let nons_word = Owned::fresh(obj::new_string_bytes(b"-nons"));
            let member = Owned::fresh(obj::new_string_bytes(member.as_bytes()));
            let other = Owned::fresh(obj::new_string_bytes(b"other"));
            for original in [
                namespace.as_ptr(),
                head.as_ptr(),
                nons_word.as_ptr(),
                member.as_ptr(),
                other.as_ptr(),
            ] {
                crate::native_source::bind_context(original, &context).unwrap();
            }
            NAMESPACE.with(|state| state.set(namespace.as_ptr()));
            for hot in [false, true] {
                let phase = format!("{label}_{}", if hot { "HOT" } else { "COLD" });
                if hot {
                    obj::invalidate_string(namespace.as_ptr());
                }
                let pattern = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
                crate::native_source::bind_context(pattern.as_ptr(), &context).unwrap();
                let mut words = vec![head.as_ptr()];
                if nons {
                    words.push(nons_word.as_ptr());
                }
                words.extend([member.as_ptr(), pattern.as_ptr()]);
                if count == 2 {
                    words.push(other.as_ptr());
                }
                for (kind, object) in [("NAMESPACE", namespace.as_ptr()), ("ARG", pattern.as_ptr())]
                {
                    let row = evidence_row(evidence, &format!("{kind}_BEFORE"), &phase);
                    assert_eq!(obj::has_string_rep(object), row[2] == "1", "{phase}/{kind}");
                    assert_eq!(type_name(object), row[3].as_bytes(), "{phase}/{kind}");
                }
                OBSERVED.with(|state| state.borrow_mut().clear());
                let code = invoke(&mut interp, namespace.as_ptr(), &words);
                let observed = evidence
                    .lines()
                    .map(|line| line.split('|').collect::<Vec<_>>())
                    .filter(|row| {
                        row.first() == Some(&"NAMESPACE_AT_ARG_GETTER")
                            && row.get(1) == Some(&phase.as_str())
                    })
                    .map(|row| row[2] == "1")
                    .collect::<Vec<_>>();
                assert_eq!(
                    OBSERVED.with(|state| state.borrow().clone()),
                    observed,
                    "{phase}"
                );
                for (kind, object) in [("NAMESPACE", namespace.as_ptr()), ("ARG", pattern.as_ptr())]
                {
                    let row = evidence_row(evidence, &format!("{kind}_AFTER"), &phase);
                    assert_eq!(obj::has_string_rep(object), row[2] == "1", "{phase}/{kind}");
                    assert_eq!(type_name(object), row[3].as_bytes(), "{phase}/{kind}");
                }
                let result = evidence_row(evidence, "RESULT", &phase);
                assert_eq!(
                    code.as_int().to_string(),
                    result[2],
                    "{phase}: {:?}",
                    interp.native_execution_refusal()
                );
                assert_eq!(interp.result_bytes(), decode(result[3]), "{phase}");
                assert!(interp.native_execution_refusal().is_none(), "{phase}");
            }
            interp.set_current_ns(crate::namespace::GLOBAL);
            NAMESPACE.with(|state| state.set(std::ptr::null_mut()));
        }
    }
}
#[test]
fn original_jim_variable_inventory_returns_same_keys_and_preserves_top_locals_empty() {
    use crate::interp::native_operation_currency::NativeOperationScope;
    let evidence = include_str!(
        "../../../tests/data/native_jim_variable_inventory/original-keys-20261015.tsv"
    );
    let locals =
        include_str!("../../../tests/data/native_jim_variable_inventory/top-locals-20261015.tsv");
    assert_eq!(evidence.lines().count(), 17);
    assert_eq!(locals.lines().count(), 9);
    let mut interp = native_jim();
    let context = interp.native_jim_object_context().unwrap();
    let _scope = NativeOperationScope::enter(&interp).unwrap();
    let set = Owned::fresh(obj::new_string_bytes(b"set"));
    let name = Owned::fresh(obj::new_wide_int_obj(123));
    let value = Owned::fresh(obj::new_wide_int_obj(789));
    let head = Owned::fresh(obj::new_string_bytes(b"info"));
    let nons = Owned::fresh(obj::new_string_bytes(b"-nons"));
    let globals = Owned::fresh(obj::new_string_bytes(b"globals"));
    let local = Owned::fresh(obj::new_string_bytes(b"locals"));
    for original in [
        set.as_ptr(),
        name.as_ptr(),
        value.as_ptr(),
        head.as_ptr(),
        nons.as_ptr(),
        globals.as_ptr(),
        local.as_ptr(),
    ] {
        crate::native_source::bind_context(original, &context).unwrap();
    }
    assert_eq!(
        interp.dispatch(&[set.as_ptr(), name.as_ptr(), value.as_ptr()]),
        Code::Ok
    );
    for filtered in [false, true] {
        let phase = if filtered { "FILTERED" } else { "UNFILTERED" };
        assert_eq!(interp.eval_str(b""), Code::Ok);
        assert_eq!(
            crate::typed_value::native_wide_int(name.as_ptr(), interp.native_invocation_dialect()),
            Ok(123)
        );
        obj::invalidate_string(name.as_ptr());
        let pattern = Owned::fresh(obj::new_wide_int_obj(123));
        crate::native_source::bind_context(pattern.as_ptr(), &context).unwrap();
        for (kind, original) in [("KEY", name.as_ptr()), ("PATTERN", pattern.as_ptr())] {
            let row = evidence_row(evidence, &format!("{kind}_BEFORE"), phase);
            assert_eq!(obj::has_string_rep(original), row[2] == "1");
            assert_eq!(type_name(original), row[3].as_bytes());
        }
        // SAFETY: the actual probe and table roles retain this original key.
        assert_eq!(
            unsafe { (*name.as_ptr()).ref_count }.to_string(),
            evidence_row(evidence, "KEY_REFS_BEFORE", phase)[2]
        );
        let mut words = vec![head.as_ptr(), nons.as_ptr(), globals.as_ptr()];
        if filtered {
            words.push(pattern.as_ptr());
        }
        assert_eq!(
            interp.dispatch(&words),
            Code::Ok,
            "{phase}: {:?}",
            interp.native_execution_refusal()
        );
        for (kind, original) in [("KEY", name.as_ptr()), ("PATTERN", pattern.as_ptr())] {
            let row = evidence_row(evidence, &format!("{kind}_AFTER"), phase);
            assert_eq!(obj::has_string_rep(original), row[2] == "1");
            assert_eq!(type_name(original), row[3].as_bytes());
        }
        // SAFETY: the same original key is now additionally owned by the result List.
        assert_eq!(
            unsafe { (*name.as_ptr()).ref_count }.to_string(),
            evidence_row(evidence, "KEY_REFS_AFTER", phase)[2]
        );
        let result = interp.get_obj_result();
        let members = interp.list_elements(&result).unwrap();
        assert_eq!(
            members
                .iter()
                .filter(|&&member| member == name.as_ptr())
                .count()
                .to_string(),
            evidence_row(evidence, "KEY_IS_ORIGINAL", phase)[2]
        );
        if filtered {
            assert_eq!(
                interp.result_bytes(),
                decode(evidence_row(evidence, "RESULT", phase)[3])
            );
        }
    }
    for hot in [false, true] {
        let phase = if hot {
            "TOP_LOCALS_HOT"
        } else {
            "TOP_LOCALS_COLD"
        };
        let pattern = Owned::fresh(obj::new_wide_int_obj(123));
        crate::native_source::bind_context(pattern.as_ptr(), &context).unwrap();
        assert_eq!(
            obj::has_string_rep(pattern.as_ptr()),
            evidence_row(locals, "PATTERN_BEFORE", phase)[2] == "1"
        );
        assert_eq!(
            interp.dispatch(&[
                head.as_ptr(),
                nons.as_ptr(),
                local.as_ptr(),
                pattern.as_ptr()
            ]),
            Code::Ok
        );
        assert_eq!(
            obj::has_string_rep(pattern.as_ptr()),
            evidence_row(locals, "PATTERN_AFTER", phase)[2] == "1"
        );
        assert_eq!(
            type_name(pattern.as_ptr()),
            evidence_row(locals, "PATTERN_AFTER", phase)[3].as_bytes()
        );
        assert_eq!(interp.get_obj_result(), context.empty_object().as_ptr());
        assert_eq!(
            evidence_row(locals, "RESULT_IS_CONTEXT_EMPTY", phase)[2],
            "1"
        );
        assert_eq!(
            interp.result_bytes(),
            decode(evidence_row(locals, "RESULT", phase)[3])
        );
    }
    assert!(interp.native_execution_refusal().is_none());
}
#[test]
fn original_jim_inventory_receipt_does_not_retain_key_or_follow_recreated_slot() {
    use tcl_syntax::native_jim_lookup::NativeJimVariableInventoryLookup;
    let mut interp = native_jim();
    let context = interp.native_jim_object_context().unwrap();
    let set = Owned::fresh(obj::new_string_bytes(b"set"));
    let name = Owned::fresh(obj::new_string_bytes(b"original_inventory_key"));
    let value = Owned::fresh(obj::new_wide_int_obj(7));
    for original in [set.as_ptr(), name.as_ptr(), value.as_ptr()] {
        crate::native_source::bind_context(original, &context).unwrap();
    }
    assert_eq!(
        interp.dispatch(&[set.as_ptr(), name.as_ptr(), value.as_ptr()]),
        Code::Ok
    );
    // SAFETY: actual table and probe roles own the exact live header.
    let count = unsafe { (*name.as_ptr()).ref_count };
    let inventory = interp
        .namespaces()
        .var_table(crate::namespace::GLOBAL)
        .capture_original_jim_inventory(
            true,
            NativeJimVariableInventoryLookup::Exact(b"original_inventory_key"),
        )
        .unwrap();
    assert_eq!(unsafe { (*name.as_ptr()).ref_count }, count);
    assert_eq!(inventory.keys().len(), 1);
    assert_eq!(inventory.keys()[0].original().unwrap(), name.as_ptr());
    assert_eq!(
        interp.eval_str(b"unset original_inventory_key; set original_inventory_key REPLACEMENT"),
        Code::Ok
    );
    assert!(inventory.check().is_err());
    assert!(inventory.keys()[0].original().is_err());
    let pointer = name.as_ptr();
    drop(name);
    assert!(obj::check_native_liveness(pointer).is_err());
    assert!(inventory.keys()[0].original().is_err());

    let mut entered = native_jim();
    let context = entered.native_jim_object_context().unwrap();
    let namespace = Owned::fresh(obj::new_wide_int_obj(42));
    let head = Owned::fresh(obj::new_string_bytes(b"info"));
    let nons = Owned::fresh(obj::new_string_bytes(b"-nons"));
    let vars = Owned::fresh(obj::new_string_bytes(b"vars"));
    for original in [
        namespace.as_ptr(),
        head.as_ptr(),
        nons.as_ptr(),
        vars.as_ptr(),
    ] {
        crate::native_source::bind_context(original, &context).unwrap();
    }
    NAMESPACE.with(|state| state.set(namespace.as_ptr()));
    let cold = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
    crate::native_source::bind_context(cold.as_ptr(), &context).unwrap();
    assert_eq!(
        invoke(
            &mut entered,
            namespace.as_ptr(),
            &[head.as_ptr(), nons.as_ptr(), vars.as_ptr(), cold.as_ptr()]
        ),
        Code::Ok
    );
    obj::invalidate_string(namespace.as_ptr());
    let pattern = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
    crate::native_source::bind_context(pattern.as_ptr(), &context).unwrap();
    CHANGE.with(|state| *state.borrow_mut() = Some(entered.clone()));
    assert_eq!(
        invoke(
            &mut entered,
            namespace.as_ptr(),
            &[
                head.as_ptr(),
                nons.as_ptr(),
                vars.as_ptr(),
                pattern.as_ptr()
            ]
        ),
        Code::Error
    );
    CHANGE.with(|state| state.borrow_mut().take());
    let first = entered
        .native_execution_refusal()
        .expect("actual variable inventory withdrawal");
    assert!(!obj::has_string_rep(namespace.as_ptr()));
    assert!(obj::has_string_rep(pattern.as_ptr()));
    let _ = entered.refuse_host_command("LATER_VARIABLE_INVENTORY_REFUSAL");
    assert_eq!(entered.native_execution_refusal(), Some(first));
    NAMESPACE.with(|state| state.set(std::ptr::null_mut()));

    let mut publishing = native_jim();
    let context = publishing.native_jim_object_context().unwrap();
    let namespace = Owned::fresh(obj::new_wide_int_obj(42));
    let head = Owned::fresh(obj::new_string_bytes(b"info"));
    let nons = Owned::fresh(obj::new_string_bytes(b"-nons"));
    let vars = Owned::fresh(obj::new_string_bytes(b"vars"));
    for original in [
        namespace.as_ptr(),
        head.as_ptr(),
        nons.as_ptr(),
        vars.as_ptr(),
    ] {
        crate::native_source::bind_context(original, &context).unwrap();
    }
    NAMESPACE.with(|state| state.set(namespace.as_ptr()));
    let cold = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
    crate::native_source::bind_context(cold.as_ptr(), &context).unwrap();
    assert_eq!(
        invoke(
            &mut publishing,
            namespace.as_ptr(),
            &[head.as_ptr(), nons.as_ptr(), vars.as_ptr(), cold.as_ptr()]
        ),
        Code::Ok
    );
    obj::invalidate_string(namespace.as_ptr());
    let pattern = Owned::fresh(obj::alloc_typed(&PATTERN, 0));
    crate::native_source::bind_context(pattern.as_ptr(), &context).unwrap();
    RESULT_RETIRED.with(|state| state.set(0));
    RESULT_CHANGE.with(|state| *state.borrow_mut() = Some(publishing.clone()));
    assert_eq!(
        invoke(
            &mut publishing,
            namespace.as_ptr(),
            &[
                head.as_ptr(),
                nons.as_ptr(),
                vars.as_ptr(),
                pattern.as_ptr()
            ]
        ),
        Code::Error
    );
    RESULT_CHANGE.with(|state| state.borrow_mut().take());
    assert_eq!(RESULT_RETIRED.with(Cell::get), 1);
    let first = publishing
        .native_execution_refusal()
        .expect("actual result-retirement withdrawal");
    assert!(!obj::has_string_rep(namespace.as_ptr()));
    assert!(obj::has_string_rep(pattern.as_ptr()));
    let _ = publishing.refuse_host_command("LATER_RESULT_PUBLICATION_REFUSAL");
    assert_eq!(publishing.native_execution_refusal(), Some(first));
    NAMESPACE.with(|state| state.set(std::ptr::null_mut()));
}
