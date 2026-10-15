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
use std::cell::RefCell;
use tcl_syntax::value::ValueOps;

const EVIDENCE: &str =
    include_str!("../../tests/data/native_jim_namespace_upvar/helper-identity-20261015.tsv");
type Row = (String, Vec<String>);
#[derive(Default)]
struct Invocation {
    pointers: [*mut TclObj; 6],
    rows: Vec<Row>,
}
thread_local! {static INVOCATION: RefCell<Invocation> = RefCell::new(Invocation::default());}
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
fn record(kind: &str, fields: Vec<String>) {
    INVOCATION.with(|slot| slot.borrow_mut().rows.push((kind.to_owned(), fields)));
}
extern "C" fn update_name(original: *mut TclObj) {
    let caller = INVOCATION.with(|slot| slot.borrow().pointers[0]);
    record("CALLER_AT_NAME_GETTER", header(caller));
    let bytes: &[u8] = match obj::internal_rep(original) {
        0 => b"v",
        1 => b"other",
        2 => b"::84",
        _ => unreachable!("fixed original upvar names"),
    };
    // SAFETY: the original updater owns its live object's String slot.
    unsafe { obj::set_string_rep(original, bytes) };
}
static NAME: obj::TclObjType = obj::TclObjType {
    name: c"upvar-name-probe".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_name),
    set_from_any_proc: None,
};
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    for byte in bytes {
        write!(&mut output, "{byte:02x}").unwrap();
    }
    output
}
unsafe extern "C" fn observe(
    _: *mut std::ffi::c_void,
    interpreter: *mut Interp,
    count: std::ffi::c_int,
    words: *const *mut TclObj,
) -> std::ffi::c_int {
    // SAFETY: the synchronous actual command callback receives live arguments and interpreter.
    let (interp, argv) = unsafe {
        (
            &mut *interpreter,
            std::slice::from_raw_parts(words, usize::try_from(count).unwrap()),
        )
    };
    let pointers = INVOCATION.with(|slot| slot.borrow().pointers);
    let [caller, _, _, local, _, last] = pointers;
    let current = interp
        .namespaces()
        .jim_namespace_original_owner(interp.current_ns())
        .unwrap()
        .as_ptr();
    let context = interp.native_jim_object_context().unwrap();
    let empty = context.empty_object().as_ptr();
    record(
        "INVOKE",
        vec![
            argv.len().to_string(),
            u8::from(current == caller).to_string(),
            u8::from(argv.get(3) == Some(&local)).to_string(),
            u8::from(argv.get(3) == Some(&empty)).to_string(),
            u8::from(argv.get(5) == Some(&last)).to_string(),
        ],
    );
    for (kind, original) in ["CALLER", "SPACE", "OTHER", "LOCAL", "EXTRA", "LAST"]
        .into_iter()
        .zip(pointers)
    {
        record(&format!("{kind}_INVOKE"), header(original));
    }
    for (index, kind) in [(2, "TARGET"), (4, "TARGET_SECOND")] {
        if let Some(original) = argv.get(index) {
            let bytes = interp.native_string_bytes(original).unwrap();
            record(kind, vec!["0".to_owned(), hex(&bytes)]);
        }
    }
    interp.set_result(argv.get(3).copied().unwrap_or(empty));
    0
}
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
    let result = rows.next().expect("actual original upvar row")[2..]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert!(rows.next().is_none());
    result
}
fn expected_events(phase: &str) -> Vec<Row> {
    EVIDENCE
        .lines()
        .map(|line| line.split('|').collect::<Vec<_>>())
        .filter(|fields| fields[1] == phase)
        .take_while(|fields| fields[0] != "CALLER_AFTER")
        .filter(|fields| {
            fields[0] == "INVOKE"
                || fields[0].ends_with("_INVOKE")
                || fields[0] == "CALLER_AT_NAME_GETTER"
                || fields[0].starts_with("TARGET")
        })
        .map(|fields| {
            (
                fields[0].to_owned(),
                fields[2..].iter().map(|s| (*s).to_owned()).collect(),
            )
        })
        .collect()
}
fn check_headers(phase: &str, stage: &str, pointers: [*mut TclObj; 6]) {
    for (kind, original) in ["CALLER", "SPACE", "OTHER", "LOCAL", "EXTRA", "LAST"]
        .into_iter()
        .zip(pointers)
    {
        assert_eq!(
            header(original),
            row(&format!("{kind}_{stage}"), phase),
            "{phase}/{kind}/{stage}"
        );
    }
}
fn check_result(interp: &mut Interp, phase: &str, code: Code, kind: &str) {
    let expected = row(kind, phase);
    assert_eq!(code.as_int().to_string(), expected[0], "{phase}");
    assert_eq!(hex(&interp.result_bytes()), expected[1], "{phase}");
    assert!(interp.native_execution_refusal().is_none(), "{phase}");
}
#[derive(Clone, Copy)]
struct Case<'a> {
    label: &'a str,
    direct: bool,
    count: usize,
    change: usize,
    absolute: bool,
}
fn run_phase(interp: &mut Interp, case: Case<'_>, reuse: bool, originals: [&Owned; 3]) {
    let [caller, head, member] = originals;
    let phase = format!("{}_{}", case.label, if reuse { "REUSE" } else { "COLD" });
    if reuse {
        obj::invalidate_string(caller.as_ptr());
    }
    let context = interp.native_jim_object_context().unwrap();
    let space = Owned::fresh(if case.absolute {
        obj::alloc_typed(&NAME, 2)
    } else {
        obj::new_wide_int_obj(84)
    });
    let other = Owned::fresh(obj::alloc_typed(&NAME, 0));
    let local = Owned::fresh(obj::new_wide_int_obj(123));
    let extra = Owned::fresh(obj::alloc_typed(&NAME, 1));
    let last = Owned::fresh(obj::new_wide_int_obj(456));
    let pointers = [
        caller.as_ptr(),
        space.as_ptr(),
        other.as_ptr(),
        local.as_ptr(),
        extra.as_ptr(),
        last.as_ptr(),
    ];
    for original in &pointers[1..] {
        crate::native_source::bind_context(*original, &context).unwrap();
    }
    let mut argv = vec![head.as_ptr()];
    if !case.direct {
        argv.push(member.as_ptr());
    }
    argv.extend(pointers[1..].iter().copied().take(case.count));
    let vector = Owned::fresh(interp.new_list_object(&argv));
    check_headers(&phase, "BEFORE", pointers);
    let namespace = interp
        .namespaces_mut()
        .retain_unmaterialized_jim_namespace(Owned::retain(caller.as_ptr()));
    INVOCATION.with(|slot| {
        *slot.borrow_mut() = Invocation {
            pointers,
            rows: Vec::new(),
        }
    });
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
    let events = INVOCATION.with(|slot| std::mem::take(&mut slot.borrow_mut().rows));
    assert_eq!(events, expected_events(&phase), "{phase}");
    check_headers(&phase, "AFTER", pointers);
    assert_eq!(
        u8::from(interp.result_obj() == local.as_ptr()).to_string(),
        row("RESULT_IS_LOCAL", &phase)[0],
        "{phase}"
    );
    if case.change == 1 {
        let result_original = interp.result_obj();
        let result = interp.list_elements(&result_original).unwrap();
        assert_eq!(
            u8::from(result.len() == 2 && result[0] == other.as_ptr()).to_string(),
            row("REPLACEMENT_OTHER_IS_ORIGINAL", &phase)[0],
            "{phase}"
        );
    }
    check_result(interp, &phase, code, "RESULT");
}
fn run_case(case: Case<'_>) {
    let mut interp = native_jim();
    interp
        .create_obj_command(
            b"upvar",
            ObjCommand::new(observe, std::ptr::null_mut(), None),
        )
        .unwrap();
    if case.change != 0 {
        assert_eq!(
            interp.eval_str(if case.change == 1 {
                b"rename {namespace upvar} saved; proc {namespace upvar} {ns args} {return $args}"
            } else {
                b"rename {namespace upvar} {}"
            }),
            Code::Ok
        );
    }
    let context = interp.native_jim_object_context().unwrap();
    let caller = Owned::fresh(obj::new_wide_int_obj(42));
    let head = Owned::fresh(obj::new_string_bytes(if case.direct {
        b"namespace upvar"
    } else {
        b"namespace"
    }));
    let member = Owned::fresh(obj::new_string_bytes(b"upvar"));
    for original in [caller.as_ptr(), head.as_ptr(), member.as_ptr()] {
        crate::native_source::bind_context(original, &context).unwrap();
    }
    for reuse in [false, true] {
        run_phase(&mut interp, case, reuse, [&caller, &head, &member]);
    }
    interp.set_current_ns(crate::namespace::GLOBAL);
    INVOCATION.with(|slot| *slot.borrow_mut() = Invocation::default());
}
fn real_worker_cases() {
    let scripts: [&[u8];4]=[
        b"namespace eval 42::84 {variable v ORIGINAL}; namespace eval 42 {namespace upvar 84 v linked; set linked CHANGED}; set ::42::84::v",
        b"namespace eval 42::84 {variable v ORIGINAL}; namespace eval 42 {namespace upvar 84 v; set {} CHANGED}; set ::42::84::v",
        b"namespace eval 42 {namespace upvar 84}",b"namespace upvar",
    ];
    for (index, script) in scripts.into_iter().enumerate() {
        let mut interp = native_jim();
        let code = interp.eval_str(script);
        check_result(
            &mut interp,
            &format!("REAL_WORKER_{index}"),
            code,
            "REAL_RESULT",
        );
    }
}
#[test]
fn original_jim_namespace_upvar_preserves_selected_arguments_empty_padding_and_real_links() {
    assert_eq!(EVIDENCE.lines().count(), 645);
    for (name, count, absolute) in [
        ("ZERO", 0, false),
        ("NAMESPACE_ONLY", 1, false),
        ("PAIR", 3, false),
        ("ABSOLUTE", 3, true),
        ("ODD", 2, false),
        ("MULTI", 5, false),
    ] {
        for direct in [false, true] {
            let label = format!("{}_{}", if direct { "DIRECT" } else { "PUBLIC" }, name);
            run_case(Case {
                label: &label,
                direct,
                count,
                change: 0,
                absolute,
            });
        }
    }
    real_worker_cases();
}
#[test]
fn original_jim_namespace_upvar_dispatches_actual_replaced_or_deleted_helper() {
    for (label, direct, change) in [
        ("PUBLIC_REPLACED", false, 1),
        ("DIRECT_REPLACED", true, 1),
        ("PUBLIC_DELETED", false, 2),
        ("DIRECT_DELETED", true, 2),
    ] {
        run_case(Case {
            label,
            direct,
            count: 3,
            change,
            absolute: false,
        });
    }
    let mut interp = native_jim();
    let original = interp.resolve_cmd_token(b"namespace upvar").unwrap();
    assert!(matches!(
        interp.namespaces().command_by_generation(original),
        Some((_, Command::Proc(_)))
    ));
    assert_eq!(
        interp.eval_str(
            b"rename {namespace upvar} saved; proc {namespace upvar} {ns args} {return REPLACED}"
        ),
        Code::Ok
    );
    let replacement = interp.resolve_cmd_token(b"namespace upvar").unwrap();
    assert_ne!(replacement, original);
    interp.install_stock_jim_namespace_helpers();
    assert_eq!(
        interp.resolve_cmd_token(b"namespace upvar"),
        Some(replacement)
    );
    interp.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
    interp.set_dialect_profile(crate::environment::profile_for_dialect("jimtcl"));
    assert_eq!(
        interp.resolve_cmd_token(b"namespace upvar"),
        Some(replacement)
    );
    assert_eq!(interp.eval_str(b"namespace upvar 84"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"REPLACED");
    assert!(interp.native_execution_refusal().is_none());
}
