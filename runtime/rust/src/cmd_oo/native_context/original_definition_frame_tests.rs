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
use std::cell::Cell;
use tcl_runtime_api::Introspect;

thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static UPDATES: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
}
extern "C" fn update(original: *mut TclObj) {
    UPDATES.with(|calls| calls.set(calls.get() + 1));
    let interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    unsafe { obj::set_native_updater_string_rep(original, b"frame-data", false) };
}
extern "C" fn free(_original: *mut TclObj) {
    FREES.with(|calls| calls.set(calls.get() + 1));
}
static MEMBER: obj::TclObjType = obj::TclObjType {
    name: c"originalDefinitionFrameMember".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn native(version: &str) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::class create FrameClass"),
        Code::Ok
    );
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    UPDATES.with(|calls| calls.set(0));
    FREES.with(|calls| calls.set(0));
    interpreter
}
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn stream(version: &str) -> &'static str {
    match version {
            "tcl8.6" => include_str!("../../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/tcl8.6/execute.stdout"),
            "tcl9.0" => include_str!("../../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/tcl9.0/execute.stdout"),
            "tcl9.1" => include_str!("../../../../../rust/tcl-registry/tests/data/native_oo_operand_stages_original/capture/tcl9.1/execute.stdout"),
            _ => unreachable!(),
        }
}
fn row(version: &str, case: usize, stage: &str) -> (usize, isize, bool) {
    let fields = stream(version)
        .lines()
        .filter(|line| line.starts_with("ROW\t"))
        .map(|line| {
            tcl_test_support::fixture::tsv_fields(line)
                .skip(1)
                .filter_map(|field| field.split_once('='))
                .collect::<Vec<_>>()
        })
        .find(|fields| {
            fields.contains(&("case", case.to_string().as_str()))
                && fields.contains(&("stage", stage))
        })
        .expect("one exact complete native ROW");
    let field = |name| {
        fields
            .iter()
            .find_map(|&(key, value)| (key == name).then_some(value))
            .unwrap()
    };
    assert_eq!(field("status"), "0");
    (
        field("r0_update").parse().unwrap(),
        field("r0_refs").parse().unwrap(),
        field("r0_string") == "1",
    )
}
fn snapshot(original: *mut TclObj) -> (usize, isize, bool) {
    (
        UPDATES.with(Cell::get),
        unsafe { (*original).ref_count },
        obj::has_string_rep(original),
    )
}

#[test]
fn original_definition_entry_waits_for_a_report_and_preserves_raw_list_members() {
    // naming.tcloo.original-operand-string-and-prefix-stages
    // docs/design/analysis/name-resolution-proofs/tcloo-original-operand-string-and-prefix-stages.md
    // Match String/update phases and the report's native member +1 delta.
    // The direct software frame fixture has no public EvalObjv argv +1 role.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for case in [2, 3, 4] {
            let mut interpreter = native(version);
            let target = interpreter.oo_resolve_object(b"FrameClass");
            let argv = [
                word(b"oo::define"),
                word(b"FrameClass"),
                word(b"observer"),
                obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0)),
            ];
            let original: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
            let member = original[3];
            let initial = snapshot(member);
            let operation = OriginalArguments::enter(&interpreter, &original).unwrap();
            let caller = interpreter
                .oo_enter_definition(&DefTarget::Class(target), &original)
                .unwrap();
            let entered = snapshot(member);
            let expected_entered = row(version, case, "definition-entered");
            assert_eq!(
                (entered.0, entered.2),
                (expected_entered.0, expected_entered.2)
            );
            assert_eq!(
                entered.1, initial.1,
                "frame tape owns allocation safety, not a native reference"
            );
            assert!(interpreter.frames.borrow().words_at(1).unwrap().is_empty());
            if case == 2 {
                interpreter.oo_leave_definition(caller);
                let returned = row(version, case, "definition-returned");
                assert_eq!(
                    (snapshot(member).0, snapshot(member).2),
                    (returned.0, returned.2)
                );
                assert_eq!(snapshot(member).1, initial.1);
            } else {
                let report =
                    obj::Owned::fresh(Introspect::level_argv(&interpreter, 1).unwrap().unwrap());
                let expected_report = row(version, case, "definition-info-level-returned");
                let reported = snapshot(member);
                assert_eq!(
                    (reported.0, reported.2),
                    (expected_report.0, expected_report.2)
                );
                assert_eq!(
                    reported.1 - entered.1,
                    expected_report.1 - expected_entered.1
                );
                let members = interpreter.list_elements(&report.as_ptr()).unwrap();
                assert_eq!(members, original);
                assert!(stream(version).lines().any(|line| line
                    == format!("REPORT\tcase={case}\tstatus=0\tcount=4\toriginal_member=1")));
                if case == 4 {
                    let bytes = interpreter
                        .native_object_string_bytes(report.as_ptr())
                        .unwrap();
                    assert_eq!(bytes.as_ref(), b"oo::define FrameClass observer frame-data");
                    let expected = row(version, case, "definition-info-presented");
                    assert_eq!(
                        (snapshot(member).0, snapshot(member).2),
                        (expected.0, expected.2)
                    );
                }
                interpreter.oo_leave_definition(caller);
                assert_eq!(
                    snapshot(member).1,
                    initial.1 + 1,
                    "only the returned List owns a native member reference"
                );
                drop(report);
                assert_eq!(snapshot(member).1, initial.1);
            }
            assert!(operation.check().is_ok());
            drop(operation);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_frame_allocation_lease_cannot_revive_a_retired_member() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Allocation-only frame retention is independent of native header liveness.
    let interpreter = native("tcl8.6");
    let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
    let pointer = member.as_ptr();
    interpreter
        .frames
        .borrow_mut()
        .push(crate::namespace::GLOBAL);
    interpreter
        .frames
        .borrow_mut()
        .install_original_error_stack_argv(&[pointer])
        .unwrap();
    assert_eq!(snapshot(pointer), (0, 1, false));
    drop(member);
    assert_eq!(FREES.with(Cell::get), 1);
    assert!(obj::check_native_liveness(pointer).is_err());
    let error = match Introspect::level_argv(&interpreter, 1) {
        Err(error) => error,
        Ok(_) => panic!("a lease cannot revive the retired original header"),
    };
    assert_eq!(
        error,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object"
            )
        )
    );
    interpreter.frames.borrow_mut().pop();
    assert_eq!(FREES.with(Cell::get), 1);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}

#[test]
fn shared_info_level_report_refusal_stops_before_bad_level_string_conversion() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The genuine shared command receives the role's outer failure; only
    // an absent inner frame may reach bad-level String formatting.
    let mut interpreter = native("tcl8.6");
    let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
    interpreter
        .frames
        .borrow_mut()
        .push(crate::namespace::GLOBAL);
    interpreter
        .frames
        .borrow_mut()
        .install_original_error_stack_argv(&[member.as_ptr()])
        .unwrap();
    drop(member);
    let index = obj::Owned::fresh(obj::new_wide_int_obj(0));
    assert!(!obj::has_string_rep(index.as_ptr()));
    interpreter.set_result_bytes(b"PRIOR");
    let prior = interpreter.get_obj_result();
    let error = match tcl_cmd_core::info::level(&mut interpreter, Some(&index.as_ptr())) {
        Err(error) => error,
        Ok(_) => panic!("retired original report must refuse"),
    };
    let expected = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "retired native object",
        ),
    );
    assert_eq!(error.native_execution_refusal(), Some(&expected));
    assert!(
        !obj::has_string_rep(index.as_ptr()),
        "outer report failure must not format the level argument"
    );
    assert_eq!(interpreter.report_cmd_error(error), Code::Error);
    assert_eq!(interpreter.native_execution_refusal(), Some(expected));
    assert_eq!(interpreter.get_obj_result(), prior);
    interpreter.frames.borrow_mut().pop();
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
