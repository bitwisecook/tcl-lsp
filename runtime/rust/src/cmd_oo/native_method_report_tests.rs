// SPDX-License-Identifier: AGPL-3.0-or-later
//! Entered role reports: source/API header ownership, separate from CLI values.
use super::*;

thread_local! {
    static REPORTS: RefCell<Vec<(Vec<u8>, NativeTclooMethodReportRole, usize)>> = const { RefCell::new(Vec::new()) };
}
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn observer(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    assert_eq!(argv.len(), 2);
    let label = interp.native_object_string_bytes(argv[1]).unwrap().to_vec();
    let (role, expected, constructor, destructor) = {
        let state = interp.oo.borrow();
        let frame = state.call_stack.last().unwrap();
        let role = frame.chain.report_role();
        let expected = if role == NativeTclooMethodReportRole::Named {
            frame.chain[frame.index]
                .method_owner
                .as_ref()
                .unwrap()
                .original_name()
                .unwrap()
        } else {
            state.method_reports.as_ref().unwrap().report(role).unwrap()
        };
        (
            role,
            expected,
            state.method_reports.as_ref().unwrap().constructor.as_ptr(),
            state.method_reports.as_ref().unwrap().destructor.as_ptr(),
        )
    };
    let head = word(b"self");
    let method = word(b"method");
    assert_eq!(
        self_cmd(interp, &[head.as_ptr(), method.as_ptr()]),
        Code::Ok
    );
    assert_eq!(interp.get_obj_result(), expected);
    if role == NativeTclooMethodReportRole::Named {
        assert_ne!(
            expected, constructor,
            "literal text does not issue a constructor role"
        );
        assert_ne!(
            expected, destructor,
            "literal text does not issue a destructor role"
        );
    } else {
        // Source/API role count only: the real Foundation +1 and reached result +1.
        assert_eq!(unsafe { (*expected).ref_count }, 2);
    }
    REPORTS.with(|reports| reports.borrow_mut().push((label, role, expected as usize)));
    interp.set_result_bytes(b"");
    Code::Ok
}
fn missing_report_observer(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"PRIOR_MISSING_REPORT");
    let original_result = interp.get_obj_result();
    let original_stamp = interp.snapshot_error().code_bytes().to_vec();
    let head = word(b"self");
    let method = word(b"method");
    let code = self_cmd(interp, &[head.as_ptr(), method.as_ptr()]);
    assert_eq!(code, Code::Error);
    assert_eq!(interp.get_obj_result(), original_result);
    assert_eq!(
        interp.snapshot_error().code_bytes(),
        original_stamp.as_slice()
    );
    assert_eq!(
        interp.native_execution_refusal(),
        Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original Foundation method report header"
            )
        ))
    );
    code
}
fn native(version: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
#[test]
fn original_foundation_headers_and_inherited_next_keep_genuine_report_roles() {
    // naming.tcloo.original-selected-method-role-report
    // docs/design/analysis/name-resolution-proofs/tcloo-original-selected-method-role-report.md
    // Source/API header and reached-role control; ordinary CLI value proof is separate.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = native(version);
        REPORTS.with(|reports| reports.borrow_mut().clear());
        let (ctor, dtor) = {
            let state = interp.oo.borrow();
            let reports = state.method_reports.as_ref().unwrap();
            assert_eq!(unsafe { (*reports.constructor.as_ptr()).ref_count }, 1);
            assert_eq!(unsafe { (*reports.destructor.as_ptr()).ref_count }, 1);
            (reports.constructor.as_ptr(), reports.destructor.as_ptr())
        };
        interp.register_builtin(b"observe_report_role", observer);
        assert_eq!(interp.eval_str(b"oo::class create ReportBase { constructor {} {observe_report_role ctor-base}; destructor {observe_report_role dtor-base} }; oo::class create ReportDerived { superclass ReportBase; constructor {} {observe_report_role ctor-derived; next}; destructor {observe_report_role dtor-derived; next}; method <constructor> {} {observe_report_role named-literal}; method <destructor> {} {observe_report_role named-destructor}; export <constructor> <destructor> }; ReportDerived create ReportObject; ReportObject <constructor>; ReportObject <destructor>; ReportObject destroy"), Code::Ok);
        let rows = REPORTS.with(|reports| reports.borrow().clone());
        assert_eq!(
            rows.iter().map(|row| row.0.as_slice()).collect::<Vec<_>>(),
            vec![
                b"ctor-derived".as_slice(),
                b"ctor-base",
                b"named-literal",
                b"named-destructor",
                b"dtor-derived",
                b"dtor-base"
            ]
        );
        assert_eq!(rows[0].1, NativeTclooMethodReportRole::Constructor);
        assert_eq!(rows[1].1, NativeTclooMethodReportRole::Constructor);
        assert_eq!(rows[0].2, ctor as usize);
        assert_eq!(rows[1].2, ctor as usize);
        assert_eq!(rows[2].1, NativeTclooMethodReportRole::Named);
        assert_eq!(rows[3].1, NativeTclooMethodReportRole::Named);
        assert_eq!(rows[4].1, NativeTclooMethodReportRole::Destructor);
        assert_eq!(rows[5].1, NativeTclooMethodReportRole::Destructor);
        assert_eq!(rows[4].2, dtor as usize);
        assert_eq!(rows[5].2, dtor as usize);
        assert_eq!(unsafe { (*ctor).ref_count }, 1);
        assert_eq!(unsafe { (*dtor).ref_count }, 1);
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}
#[test]
fn original_foundation_report_refuses_missing_headers_and_unsupported_selection() {
    // naming.tcloo.original-selected-method-role-report
    // docs/design/analysis/name-resolution-proofs/tcloo-original-selected-method-role-report.md
    // Unsupported original entry never receives a Foundation report header.
    for version in ["tcl8.4", "tcl8.5", "jim"] {
        let interp = native(version);
        assert!(interp.oo.borrow().method_reports.is_none(), "{version}");
    }
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = native(version);
        interp.register_builtin(b"missing_report_observer", missing_report_observer);
        assert_eq!(
            interp.eval_str(
                b"oo::class create MissingReport {constructor {} {missing_report_observer}}"
            ),
            Code::Ok
        );
        let old_reports = interp.oo.borrow_mut().method_reports.take();
        drop(old_reports);
        assert_eq!(interp.eval_str(b"MissingReport new"), Code::Error);
        assert!(interp.oo.borrow().method_reports.is_none());
        assert_eq!(
            interp.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original Foundation method report header"
                )
            ))
        );
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}
