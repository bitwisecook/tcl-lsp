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

use crate::interp::{Code, Command, Interp, ObjCommand};
use crate::obj::{self, Owned, TclObj};
use std::cell::{Cell, RefCell};
use tcl_syntax::value::ValueOps;

const EVIDENCE: &str =
    include_str!("../../tests/data/native_jim_namespace_delete/getter-identity-20261015.tsv");
thread_local! {
    static CALLER: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static NAME_GETTERS: RefCell<Vec<Vec<String>>> = const { RefCell::new(Vec::new()) };
    static DELETIONS: Cell<usize> = const { Cell::new(0) };
}
fn header(original: *mut TclObj) -> Vec<String> {
    let kind = obj::obj_type_ptr(original);
    let name = if kind.is_null() {
        "none"
    } else {
        // SAFETY: the retained object's static descriptor owns its terminated name.
        unsafe { std::ffi::CStr::from_ptr((*kind).name) }
            .to_str()
            .unwrap()
    };
    vec![
        u8::from(obj::has_string_rep(original)).to_string(),
        name.to_owned(),
    ]
}
extern "C" fn update_name(original: *mut TclObj) {
    NAME_GETTERS.with(|rows| {
        rows.borrow_mut()
            .push(CALLER.with(|caller| header(caller.get())))
    });
    let bytes: &[u8] = match obj::internal_rep(original) {
        0 => b"84",
        1 => b"85",
        2 => b"::",
        3 => b"",
        4 => b"::42::84",
        _ => unreachable!("fixed original deletion names"),
    };
    // SAFETY: the original updater owns its live object's String slot.
    unsafe { obj::set_string_rep(original, bytes) };
}
static NAME: obj::TclObjType = obj::TclObjType {
    name: c"delete-name-probe".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_name),
    set_from_any_proc: None,
};
unsafe extern "C" fn target(
    _: *mut std::ffi::c_void,
    interpreter: *mut Interp,
    _: std::ffi::c_int,
    _: *const *mut TclObj,
) -> std::ffi::c_int {
    // SAFETY: the actual synchronous callback receives its live interpreter.
    unsafe { (*interpreter).set_result_bytes(b"VALUE") };
    0
}
unsafe extern "C" fn delete_target(_: *mut std::ffi::c_void) {
    DELETIONS.with(|count| count.set(count.get() + 1));
}
fn native_jim() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn setup(interp: &mut Interp) {
    for name in [b"42::84::target".as_slice(), b"42::85::target"] {
        interp
            .create_obj_command(
                name,
                ObjCommand::new(target, std::ptr::null_mut(), Some(delete_target)),
            )
            .unwrap();
    }
    assert_eq!(
        interp.eval_str(b"set ::42::84::v 7; set ::42::85::v 8"),
        Code::Ok
    );
}
fn row(kind: &str, phase: &str) -> Vec<String> {
    let mut rows = EVIDENCE
        .lines()
        .map(|line| line.split('|').collect::<Vec<_>>())
        .filter(|fields| fields[0] == kind && fields[1] == phase);
    let result = rows.next().expect("actual original delete row")[2..]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert!(rows.next().is_none());
    result
}
fn expected_getters(phase: &str) -> Vec<Vec<String>> {
    EVIDENCE
        .lines()
        .map(|line| line.split('|').collect::<Vec<_>>())
        .filter(|fields| fields[1] == phase)
        .take_while(|fields| fields[0] != "CALLER_AFTER")
        .filter(|fields| fields[0] == "CALLER_AT_NAME_GETTER")
        .map(|fields| fields[2..].iter().map(|s| (*s).to_owned()).collect())
        .collect()
}
#[derive(Clone, Copy)]
struct Case<'a> {
    label: &'a str,
    direct: bool,
    count: usize,
    change: usize,
    name: usize,
    mutates: bool,
}
fn run_phase(interp: &mut Interp, case: Case<'_>, reuse: bool, originals: [&Owned; 3]) {
    let [caller, head, member] = originals;
    let phase = format!("{}_{}", case.label, if reuse { "REUSE" } else { "COLD" });
    if reuse {
        if case.mutates {
            setup(interp);
        }
        obj::invalidate_string(caller.as_ptr());
    }
    let context = interp.native_jim_object_context().unwrap();
    let first = Owned::fresh(obj::alloc_typed(
        &NAME,
        u64::try_from(case.name).expect("fixed original deletion name payload"),
    ));
    let second = Owned::fresh(obj::alloc_typed(&NAME, 1));
    for original in [first.as_ptr(), second.as_ptr()] {
        crate::native_source::bind_context(original, &context).unwrap();
    }
    let mut argv = vec![head.as_ptr()];
    if !case.direct {
        argv.push(member.as_ptr());
    }
    argv.extend(
        [first.as_ptr(), second.as_ptr()]
            .into_iter()
            .take(case.count),
    );
    let vector = Owned::fresh(interp.new_list_object(&argv));
    for (kind, original) in [
        ("CALLER_BEFORE", caller.as_ptr()),
        ("FIRST_BEFORE", first.as_ptr()),
        ("SECOND_BEFORE", second.as_ptr()),
    ] {
        assert_eq!(header(original), row(kind, &phase), "{phase}/{kind}");
    }
    let namespace = interp
        .namespaces_mut()
        .retain_unmaterialized_jim_namespace(Owned::retain(caller.as_ptr()));
    DELETIONS.with(|n| n.set(0));
    NAME_GETTERS.with(|rows| rows.borrow_mut().clear());
    let code = interp.ns_eval_in_token(
        namespace,
        &[],
        None,
        false,
        Some((
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
            vector.as_ptr(),
        )),
        None,
    );
    let getters = NAME_GETTERS.with(|rows| std::mem::take(&mut *rows.borrow_mut()));
    assert_eq!(getters, expected_getters(&phase), "{phase}");
    for (kind, original) in [
        ("CALLER_AFTER", caller.as_ptr()),
        ("FIRST_AFTER", first.as_ptr()),
        ("SECOND_AFTER", second.as_ptr()),
    ] {
        assert_eq!(header(original), row(kind, &phase), "{phase}/{kind}");
    }
    assert_eq!(
        DELETIONS.with(Cell::get).to_string(),
        row("DELETIONS", &phase)[0],
        "{phase}"
    );
    if case.change == 1 {
        let result_original = interp.result_obj();
        let result = interp.list_elements(&result_original).unwrap();
        assert_eq!(
            u8::from(result.len() == 1 && result[0] == first.as_ptr()).to_string(),
            row("REPLACEMENT_ARG_IS_ORIGINAL", &phase)[0],
            "{phase}"
        );
    }
    let expected = row("RESULT", &phase);
    assert_eq!(code.as_int().to_string(), expected[0], "{phase}");
    let bytes = expected[1]
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(interp.result_bytes(), bytes, "{phase}");
    assert!(interp.native_execution_refusal().is_none(), "{phase}");
}
fn run_case(case: Case<'_>) {
    let mut interp = native_jim();
    setup(&mut interp);
    if case.change != 0 {
        assert_eq!(
            interp.eval_str(if case.change == 1 {
                b"rename {namespace delete} saved; proc {namespace delete} {args} {return $args}"
            } else {
                b"rename {namespace delete} {}"
            }),
            Code::Ok
        );
    }
    let context = interp.native_jim_object_context().unwrap();
    let caller = Owned::fresh(obj::new_wide_int_obj(42));
    let head = Owned::fresh(obj::new_string_bytes(if case.direct {
        b"namespace delete"
    } else {
        b"namespace"
    }));
    let member = Owned::fresh(obj::new_string_bytes(b"delete"));
    for original in [caller.as_ptr(), head.as_ptr(), member.as_ptr()] {
        crate::native_source::bind_context(original, &context).unwrap();
    }
    CALLER.with(|slot| slot.set(caller.as_ptr()));
    for reuse in [false, true] {
        run_phase(&mut interp, case, reuse, [&caller, &head, &member]);
    }
    interp.set_current_ns(crate::namespace::GLOBAL);
    CALLER.with(|slot| slot.set(std::ptr::null_mut()));
}
#[test]
fn original_jim_namespace_delete_preserves_helper_getters_original_names_and_command_retirement() {
    assert_eq!(EVIDENCE.lines().count(), 333);
    for (label, count, name, mutates) in [
        ("ZERO", 0, 0, false),
        ("ROOT", 1, 2, false),
        ("EMPTY", 1, 3, false),
        ("RELATIVE", 1, 0, true),
        ("ABSOLUTE", 1, 4, true),
        ("MULTI", 2, 0, true),
    ] {
        for direct in [false, true] {
            let label = format!("{}_{}", if direct { "DIRECT" } else { "PUBLIC" }, label);
            run_case(Case {
                label: &label,
                direct,
                count,
                change: 0,
                name,
                mutates,
            });
        }
    }
}
#[test]
fn original_jim_namespace_delete_dispatches_actual_replaced_or_deleted_helper() {
    for (label, direct, change) in [
        ("PUBLIC_REPLACED", false, 1),
        ("DIRECT_REPLACED", true, 1),
        ("PUBLIC_DELETED", false, 2),
        ("DIRECT_DELETED", true, 2),
    ] {
        run_case(Case {
            label,
            direct,
            count: 1,
            change,
            name: 0,
            mutates: false,
        });
    }
    let mut interp = native_jim();
    let original = interp.resolve_cmd_token(b"namespace delete").unwrap();
    assert!(matches!(
        interp.namespaces().command_by_generation(original),
        Some((_, Command::Proc(_)))
    ));
    assert_eq!(
        interp.eval_str(
            b"rename {namespace delete} saved; proc {namespace delete} args {return REPLACED}"
        ),
        Code::Ok
    );
    let replacement = interp.resolve_cmd_token(b"namespace delete").unwrap();
    assert_ne!(replacement, original);
    interp.install_stock_jim_namespace_helpers();
    assert_eq!(
        interp.resolve_cmd_token(b"namespace delete"),
        Some(replacement)
    );
    interp.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
    interp.set_dialect_profile(crate::environment::profile_for_dialect("jimtcl"));
    assert_eq!(
        interp.resolve_cmd_token(b"namespace delete"),
        Some(replacement)
    );
    assert_eq!(interp.eval_str(b"namespace delete"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"REPLACED");
    assert!(interp.native_execution_refusal().is_none());
}
