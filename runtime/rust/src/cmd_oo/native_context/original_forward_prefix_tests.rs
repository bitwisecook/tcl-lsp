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

use super::*;
use crate::cmd_oo::def_forward;
use crate::cmd_oo::info_class;
use crate::cmd_oo::Method;
use crate::interp::obj_bytes;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static UPDATES: Cell<usize> = const { Cell::new(0) };
    static EXPECTED: Cell<usize> = const { Cell::new(0) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static REACHED_RESULT: Cell<usize> = const { Cell::new(0) };
    static CAPTURED: RefCell<Vec<(usize, isize, bool)>> = const { RefCell::new(Vec::new()) };
}
extern "C" fn update(original: *mut TclObj) {
    UPDATES.with(|calls| calls.set(calls.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original forward member updater");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("oo-prefix-stage".to_owned(), "1.0".to_owned()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
    unsafe { obj::set_native_updater_string_rep(original, b"prefix-data", false) };
}
static MEMBER: obj::TclObjType = obj::TclObjType {
    name: c"originalForwardMember".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn capture(interpreter: &mut Interp, argv: &[*mut TclObj]) -> Code {
    assert_eq!(argv.len(), 2);
    assert_eq!(argv[1] as usize, EXPECTED.with(Cell::get));
    let before = (
        UPDATES.with(Cell::get),
        unsafe { (*argv[1]).ref_count },
        obj::has_string_rep(argv[1]),
    );
    CAPTURED.with(|rows| rows.borrow_mut().push(before));
    if MODE.with(Cell::get) != 0 {
        interpreter.set_result_bytes(b"REACHED WORKER RESULT");
        REACHED_RESULT.with(|result| result.set(interpreter.get_obj_result() as usize));
    }
    let bytes = match interpreter.native_object_string_bytes(argv[1]) {
        Ok(bytes) => bytes,
        Err(error) => return interpreter.report_cmd_error(error.into()),
    };
    assert_eq!(bytes.as_ref(), b"prefix-data");
    let after = (
        UPDATES.with(Cell::get),
        unsafe { (*argv[1]).ref_count },
        obj::has_string_rep(argv[1]),
    );
    CAPTURED.with(|rows| rows.borrow_mut().push(after));
    interpreter.set_result_bytes(b"CAPTURE");
    Code::Ok
}
fn native(version: &str) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::class create PrefixClass; PrefixClass create PrefixObject"),
        Code::Ok
    );
    interpreter.register_builtin(b"capture", capture);
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    UPDATES.with(|calls| calls.set(0));
    MODE.with(|mode| mode.set(0));
    CAPTURED.with(|rows| rows.borrow_mut().clear());
    interpreter
}
fn define(interpreter: &mut Interp, member: *mut TclObj) -> OoId {
    let target = interpreter.oo_resolve_object(b"PrefixClass");
    let context = [word(b"oo::define"), word(b"PrefixClass"), word(b"")];
    let original: Vec<_> = context.iter().map(obj::Owned::as_ptr).collect();
    let caller = interpreter
        .oo_enter_definition(&DefTarget::Class(target), &original)
        .unwrap();
    let argv = [word(b"forward"), word(b"go"), word(b"capture")];
    let mut pointers: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
    pointers.push(member);
    assert_eq!(def_forward(interpreter, &pointers), Code::Ok);
    interpreter.oo_leave_definition(caller);
    target
}
fn stream(version: &str) -> &'static str {
    match version {
            "tcl8.6" => include_str!("../../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/8.6.18/execute.stdout"),
            "tcl9.0" => include_str!("../../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/9.0.4/execute.stdout"),
            "tcl9.1" => include_str!("../../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/9.1.0/execute.stdout"),
            _ => unreachable!(),
        }
}
fn fields(version: &str, case: usize, stage: &str) -> Vec<(&'static str, &'static str)> {
    stream(version)
        .lines()
        .filter(|line| line.starts_with("ROW\t"))
        .map(|line| {
            line.split('\t')
                .skip(1)
                .filter_map(|field| field.split_once('='))
                .collect::<Vec<_>>()
        })
        .find(|fields| {
            fields
                .iter()
                .any(|&(key, value)| key == "case" && value == case.to_string())
                && fields
                    .iter()
                    .any(|&(key, value)| key == "stage" && value == stage)
        })
        .expect("one exact complete original ROW")
}
fn original_snapshot(version: &str, case: usize, stage: &str) -> (usize, isize, bool) {
    let fields = fields(version, case, stage);
    let get = |wanted| {
        fields
            .iter()
            .find_map(|&(key, value)| (key == wanted).then_some(value))
            .unwrap()
    };
    assert_eq!(get("status"), "0");
    (
        get("r0_update").parse().unwrap(),
        get("r0_refs").parse().unwrap(),
        get("r0_string") == "1",
    )
}
fn snapshot(original: *mut TclObj) -> (usize, isize, bool) {
    (
        UPDATES.with(Cell::get),
        unsafe { (*original).ref_count },
        obj::has_string_rep(original),
    )
}
fn prefix(interpreter: &Interp, target: OoId) -> obj::Owned {
    let Some(Method::Forward { prefix }) =
        interpreter.oo.borrow().classes[&target].methods.get(b"go")
    else {
        panic!("retained original forward");
    };
    prefix
}

#[test]
fn original_forward_definition_and_raw_report_match_original_member_stages() {
    // naming.tcloo.original-operand-string-and-prefix-stages
    // docs/design/analysis/name-resolution-proofs/tcloo-original-operand-string-and-prefix-stages.md
    // Fixed C86/C90/C91 ROW comparisons; presentation is an explicit later read.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version);
        let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
        assert_eq!(
            snapshot(member.as_ptr()),
            original_snapshot(version, 0, "initial")
        );
        let target = define(&mut interpreter, member.as_ptr());
        assert_eq!(
            snapshot(member.as_ptr()),
            original_snapshot(version, 0, "forward-defined")
        );
        let stored = prefix(&interpreter, target);
        let argv: Vec<_> = [
            b"info".as_slice(),
            b"class",
            b"forward",
            b"PrefixClass",
            b"go",
        ]
        .iter()
        .map(|bytes| word(bytes))
        .collect();
        let pointers: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
        assert_eq!(info_class(&mut interpreter, &pointers), Code::Ok);
        assert_eq!(interpreter.get_obj_result(), stored.as_ptr());
        assert_eq!(
            snapshot(member.as_ptr()),
            original_snapshot(version, 0, "info-forward-returned")
        );
        let members = interpreter.list_elements(&stored.as_ptr()).unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[1], member.as_ptr());
        assert_eq!(
            snapshot(member.as_ptr()),
            original_snapshot(version, 0, "info-forward-list-accessed")
        );
        assert!(stream(version)
            .lines()
            .any(|line| line == "PREFIX\tcase=0\tstatus=0\tcount=2\toriginal_member=1"));
        let bytes = interpreter
            .native_object_string_bytes(stored.as_ptr())
            .unwrap();
        assert_eq!(bytes.as_ref(), b"capture prefix-data");
        assert_eq!(
            snapshot(member.as_ptr()),
            original_snapshot(version, 0, "info-forward-presented")
        );
    }
}

#[test]
fn original_forward_invocation_passes_exact_member_before_its_worker_getter() {
    // naming.tcloo.original-operand-string-and-prefix-stages
    // docs/design/analysis/name-resolution-proofs/tcloo-original-operand-string-and-prefix-stages.md
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version);
        let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
        define(&mut interpreter, member.as_ptr());
        EXPECTED.with(|expected| expected.set(member.as_ptr() as usize));
        let argv = [word(b"PrefixObject"), word(b"go")];
        assert_eq!(
            interpreter.dispatch(&argv.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()),
            Code::Ok
        );
        assert_eq!(
            CAPTURED.with(|rows| rows.borrow().clone()),
            vec![
                original_snapshot(version, 1, "capture-before-read"),
                original_snapshot(version, 1, "capture-after-read")
            ]
        );
        assert_eq!(
            snapshot(member.as_ptr()),
            original_snapshot(version, 1, "forward-invoked-without-report")
        );
        assert!(stream(version)
            .lines()
            .any(|line| line == "CALL\tcase=1\tkind=forward\targc=2\toriginal_member=1"));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn forward_declaration_clone_shares_the_retained_list_without_member_reference_donation() {
    // naming.tcloo.original-operand-string-and-prefix-stages
    // docs/design/analysis/name-resolution-proofs/tcloo-original-operand-string-and-prefix-stages.md
    // Software sharing control for original CloneForwardMethod's source recipe;
    // original nine-case rows contain no independent native clone observation.
    let mut interpreter = native("tcl8.6");
    let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
    let target = define(&mut interpreter, member.as_ptr());
    let stored = prefix(&interpreter, target);
    let copied = interpreter.oo.borrow().classes[&target]
        .methods
        .duplicate_declarations()
        .unwrap();
    let Some(Method::Forward { prefix: cloned }) = copied.get(b"go") else {
        panic!("forward clone");
    };
    assert_eq!(cloned.as_ptr(), stored.as_ptr());
    assert_eq!(snapshot(member.as_ptr()), (0, 2, false));
    let removed = interpreter
        .oo
        .borrow_mut()
        .classes
        .get_mut(&target)
        .unwrap()
        .methods
        .remove(b"go");
    drop(removed);
    assert_eq!(snapshot(member.as_ptr()), (0, 2, false));
    drop(copied);
    drop(stored);
    assert_eq!(snapshot(member.as_ptr()), (0, 2, false));
    drop(cloned);
    assert_eq!(snapshot(member.as_ptr()), (0, 1, false));
}

#[cfg(debug_assertions)]
#[test]
fn forward_worker_member_refusal_keeps_reached_bytes_and_stops_result_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2] {
        let mut interpreter = native("tcl8.6");
        let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
        define(&mut interpreter, member.as_ptr());
        EXPECTED.with(|expected| expected.set(member.as_ptr() as usize));
        interpreter.set_result_bytes(b"PRIOR");
        MODE.with(|selected| selected.set(mode));
        let argv = [word(b"PrefixObject"), word(b"go")];
        assert_eq!(
            interpreter.dispatch(&argv.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()),
            Code::Error
        );
        assert!(interpreter.native_execution_refusal().is_some());
        assert_eq!(
            interpreter.get_obj_result() as usize,
            REACHED_RESULT.with(Cell::get)
        );
        assert_eq!(
            obj_bytes(interpreter.get_obj_result()),
            b"REACHED WORKER RESULT"
        );
        assert_eq!(snapshot(member.as_ptr()), (1, 2, true));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
