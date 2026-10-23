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
const EVIDENCE: &str =
    include_str!("../../tests/data/native_jim_namespace_text/getter-identity-20261015.tsv");
thread_local! {
    static CALLER: Cell<*mut TclObj> = const {Cell::new(std::ptr::null_mut())};
    static GETTERS: RefCell<Vec<Vec<String>>> = const {RefCell::new(Vec::new())};
    static CHANGE: RefCell<Option<Interp>> = const {RefCell::new(None)};
    static RESULT_CHANGE: RefCell<Option<Interp>> = const {RefCell::new(None)};
    static RETIRED: Cell<usize> = const {Cell::new(0)};
}
fn header(original: *mut TclObj) -> Vec<String> {
    let kind = obj::obj_type_ptr(original);
    let name = if kind.is_null() {
        "none"
    } else {
        // SAFETY: this retained object's static descriptor owns its terminated name.
        unsafe { std::ffi::CStr::from_ptr((*kind).name) }
            .to_str()
            .unwrap()
    };
    vec![
        u8::from(obj::has_string_rep(original)).to_string(),
        name.to_owned(),
    ]
}
const NAMES: &[&[u8]] = &[
    b"absent::child",
    b"absent",
    b"::absent::child",
    b"::",
    b"",
    b"::\0child",
    b"absent::x\0::tail",
    b":",
    b":::tail",
];
extern "C" fn update_name(original: *mut TclObj) {
    GETTERS.with(|rows| {
        rows.borrow_mut()
            .push(CALLER.with(|caller| header(caller.get())))
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
    // SAFETY: the original updater owns its actual String slot.
    unsafe {
        obj::set_string_rep(
            original,
            NAMES[usize::try_from(obj::internal_rep(original)).unwrap()],
        )
    };
}
static NAME: obj::TclObjType = obj::TclObjType {
    name: c"namespace-text-name-probe".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_name),
    set_from_any_proc: None,
};
extern "C" fn retire_result(_: *mut TclObj) {
    RETIRED.with(|count| count.set(count.get() + 1));
    RESULT_CHANGE.with(|state| {
        if let Some(interp) = state.borrow().as_ref() {
            interp.set_current_ns(crate::namespace::GLOBAL);
        }
    });
}
static RESULT_CALLBACK: obj::TclObjType = obj::TclObjType {
    name: c"namespace-text-result-callback".as_ptr(),
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
fn row(kind: &str, phase: &str) -> Vec<String> {
    let mut rows = EVIDENCE
        .lines()
        .map(|line| line.split('|').collect::<Vec<_>>())
        .filter(|fields| fields[0] == kind && fields[1] == phase);
    let result = rows.next().expect("actual original namespace text row")[2..]
        .iter()
        .map(|field| (*field).to_owned())
        .collect();
    assert!(rows.next().is_none());
    result
}
fn bind(interp: &Interp, originals: &[*mut TclObj]) {
    let context = interp.native_jim_object_context().unwrap();
    for &original in originals {
        crate::native_source::bind_context(original, &context).unwrap();
    }
}
fn invoke(interp: &mut Interp, caller: *mut TclObj, argv: &[*mut TclObj]) -> Code {
    let namespace = interp
        .namespaces_mut()
        .retain_unmaterialized_jim_namespace(Owned::retain(caller));
    let vector = Owned::fresh(interp.new_list_object(argv));
    interp.ns_eval_in_token(
        namespace,
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
fn check_headers(phase: &str, stage: &str, originals: [*mut TclObj; 3]) {
    for (kind, original) in ["CALLER", "ARGUMENT", "EXTRA"].into_iter().zip(originals) {
        assert_eq!(
            header(original),
            row(&format!("{kind}_{stage}"), phase),
            "{phase}/{kind}/{stage}"
        );
    }
}
fn check_result(interp: &mut Interp, phase: &str, code: Code, argument: *mut TclObj) {
    assert_eq!(
        header(interp.result_obj()),
        row("RESULT_HEADER", phase),
        "{phase}"
    );
    assert_eq!(
        u8::from(interp.result_obj() == argument).to_string(),
        row("RESULT_IS_ARGUMENT", phase)[0],
        "{phase}"
    );
    let context = interp.native_jim_object_context().unwrap();
    assert_eq!(
        u8::from(interp.result_obj() == context.empty_object().as_ptr()).to_string(),
        row("RESULT_IS_EMPTY", phase)[0],
        "{phase}"
    );
    let expected = row("RESULT", phase);
    assert_eq!(code.as_int().to_string(), expected[0], "{phase}");
    let bytes = expected[1]
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(interp.result_bytes(), bytes, "{phase}");
    assert!(interp.native_execution_refusal().is_none(), "{phase}");
}
fn run_phase(
    interp: &mut Interp,
    label: &str,
    count: usize,
    name: Option<u64>,
    reuse: bool,
    originals: [&Owned; 3],
) {
    let [caller, head, member] = originals;
    let phase = format!("{label}_{}", if reuse { "REUSE" } else { "COLD" });
    if reuse {
        obj::invalidate_string(caller.as_ptr());
    }
    let argument = Owned::fresh(name.map_or_else(
        || obj::new_wide_int_obj(84),
        |name| obj::alloc_typed(&NAME, name),
    ));
    let extra = Owned::fresh(obj::new_wide_int_obj(123));
    bind(interp, &[argument.as_ptr(), extra.as_ptr()]);
    let observed = [caller.as_ptr(), argument.as_ptr(), extra.as_ptr()];
    check_headers(&phase, "BEFORE", observed);
    let argv = [
        head.as_ptr(),
        member.as_ptr(),
        argument.as_ptr(),
        extra.as_ptr(),
    ];
    GETTERS.with(|rows| rows.borrow_mut().clear());
    let code = invoke(interp, caller.as_ptr(), &argv[..2 + count]);
    let getters = GETTERS.with(|rows| std::mem::take(&mut *rows.borrow_mut()));
    let expected = EVIDENCE
        .lines()
        .map(|line| line.split('|').collect::<Vec<_>>())
        .filter(|fields| fields[0] == "CALLER_AT_NAME_GETTER" && fields[1] == phase)
        .map(|fields| {
            fields[2..]
                .iter()
                .map(|field| (*field).to_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(getters, expected, "{phase}");
    check_headers(&phase, "AFTER", observed);
    check_result(interp, &phase, code, argument.as_ptr());
}
fn run_case(operation: &[u8], label: &str, count: usize, name: Option<u64>) {
    let mut interp = native_jim();
    let caller = Owned::fresh(obj::new_wide_int_obj(42));
    let head = Owned::fresh(obj::new_string_bytes(b"namespace"));
    let member = Owned::fresh(obj::new_string_bytes(operation));
    bind(&interp, &[caller.as_ptr(), head.as_ptr(), member.as_ptr()]);
    CALLER.with(|value| value.set(caller.as_ptr()));
    for reuse in [false, true] {
        run_phase(
            &mut interp,
            label,
            count,
            name,
            reuse,
            [&caller, &head, &member],
        );
    }
    CALLER.with(|value| value.set(std::ptr::null_mut()));
}
#[test]
fn original_jim_namespace_text_keeps_shared_empty_counted_empty_and_original_tail_identity() {
    assert_eq!(EVIDENCE.lines().count(), 516);
    for (operation, prefix) in [
        (b"qualifiers".as_slice(), "QUALIFIERS"),
        (b"tail".as_slice(), "TAIL"),
    ] {
        for (label, count, name) in [
            ("ZERO", 0, None),
            ("INTEGER", 1, None),
            ("RELATIVE", 1, Some(0)),
            ("PLAIN", 1, Some(1)),
            ("ABSOLUTE", 1, Some(2)),
            ("ROOT", 1, Some(3)),
            ("EMPTY", 1, Some(4)),
            ("NUL_ROOT", 1, Some(5)),
            ("NUL_RELATIVE", 1, Some(6)),
            ("ONE_COLON", 1, Some(7)),
            ("TRIPLE_COLON", 1, Some(8)),
            ("WRONG_ARITY", 2, Some(0)),
        ] {
            run_case(operation, &format!("{prefix}_{label}"), count, name);
        }
    }
}
fn run_withdrawal(operation: &[u8], prefix: &str, on_publication: bool) {
    let mut interp = native_jim();
    let caller = Owned::fresh(obj::new_wide_int_obj(42));
    let head = Owned::fresh(obj::new_string_bytes(b"namespace"));
    let member = Owned::fresh(obj::new_string_bytes(operation));
    bind(&interp, &[caller.as_ptr(), head.as_ptr(), member.as_ptr()]);
    CALLER.with(|state| state.set(caller.as_ptr()));
    run_phase(
        &mut interp,
        &format!("{prefix}_RELATIVE"),
        1,
        Some(0),
        false,
        [&caller, &head, &member],
    );
    obj::invalidate_string(caller.as_ptr());
    let argument = Owned::fresh(obj::alloc_typed(&NAME, 0));
    bind(&interp, &[argument.as_ptr()]);
    RETIRED.with(|count| count.set(0));
    if on_publication {
        RESULT_CHANGE.with(|state| *state.borrow_mut() = Some(interp.clone()));
    } else {
        CHANGE.with(|state| *state.borrow_mut() = Some(interp.clone()));
    }
    assert_eq!(
        invoke(
            &mut interp,
            caller.as_ptr(),
            &[head.as_ptr(), member.as_ptr(), argument.as_ptr()]
        ),
        Code::Error
    );
    CHANGE.with(|state| state.borrow_mut().take());
    RESULT_CHANGE.with(|state| state.borrow_mut().take());
    assert_eq!(RETIRED.with(Cell::get), usize::from(on_publication));
    assert!(!obj::has_string_rep(caller.as_ptr()));
    assert!(obj::has_string_rep(argument.as_ptr()));
    assert!(std::ptr::eq(obj::obj_type_ptr(argument.as_ptr()), &NAME));
    let first = interp
        .native_execution_refusal()
        .expect("actual namespace text getter/publication withdrawal");
    let _ = interp.refuse_host_command("LATER_NAMESPACE_TEXT_REFUSAL");
    assert_eq!(interp.native_execution_refusal(), Some(first));
    CALLER.with(|state| state.set(std::ptr::null_mut()));
}
#[test]
fn jim_namespace_text_refuses_after_name_getter_or_result_retirement_changes_current_namespace() {
    for (operation, prefix) in [
        (b"qualifiers".as_slice(), "QUALIFIERS"),
        (b"tail".as_slice(), "TAIL"),
    ] {
        run_withdrawal(operation, prefix, false);
        run_withdrawal(operation, prefix, true);
    }
}
