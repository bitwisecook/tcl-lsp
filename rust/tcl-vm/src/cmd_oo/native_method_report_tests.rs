// SPDX-License-Identifier: AGPL-3.0-or-later
//! VM value/report roles, independent of physical C header counts.
use super::*;

thread_local! {
    static REPORTS: RefCell<Vec<(String, NativeTclooMethodReportRole, usize)>> = const { RefCell::new(Vec::new()) };
}
struct Observer;
impl crate::command::NativeCommand for Observer {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        assert_eq!(args.len(), 1);
        let label = args[0].to_str().to_string();
        let frame = native_context::current_method(vm).unwrap();
        let role = frame.chain.report_role();
        let special = vm.oo.method_reports.as_ref().unwrap().report(role);
        let constructor = vm
            .oo
            .method_reports
            .as_ref()
            .unwrap()
            .constructor
            .native_object_identity();
        let report = cmd_self(vm, &[Value::new_native_string_bytes(b"method".as_slice())]);
        assert_eq!(report.code, Code::Ok);
        if let Some(special) = special {
            assert_eq!(
                report.result.native_object_identity(),
                special.native_object_identity()
            );
        } else {
            assert_eq!(role, NativeTclooMethodReportRole::Named);
            assert_eq!(
                report.result.string_bytes().as_ref(),
                if label == "named-destructor" {
                    b"<destructor>".as_slice()
                } else {
                    b"<constructor>".as_slice()
                }
            );
            assert_ne!(report.result.native_object_identity(), constructor);
        }
        REPORTS.with(|reports| {
            reports
                .borrow_mut()
                .push((label, role, report.result.native_object_identity()))
        });
        ok(Value::empty())
    }
}
fn vm(profile: &str) -> Vm {
    crate::native_fixture::interpreter(
        tcl_registry::model::ingress::resolve_environment(profile).unit_profile(),
    )
}
#[test]
fn original_foundation_value_headers_follow_constructor_destructor_next_roles() {
    // naming.tcloo.original-selected-method-role-report
    // docs/design/analysis/name-resolution-proofs/tcloo-original-selected-method-role-report.md
    // VM identity/value semantics only; this does not assert physical C refcounts.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = vm(profile);
        REPORTS.with(|reports| reports.borrow_mut().clear());
        let reports = vm.oo.method_reports.as_ref().unwrap();
        let ctor = reports.constructor.native_object_identity();
        let dtor = reports.destructor.native_object_identity();
        vm.register_command("observe_report_role", Command::Native(Rc::new(Observer)));
        let completion = vm.eval_source("oo::class create ReportBase { constructor {} {observe_report_role ctor-base}; destructor {observe_report_role dtor-base} }; oo::class create ReportDerived { superclass ReportBase; constructor {} {observe_report_role ctor-derived; next}; destructor {observe_report_role dtor-derived; next}; method <constructor> {} {observe_report_role named-literal}; method <destructor> {} {observe_report_role named-destructor}; export <constructor> <destructor> }; ReportDerived create ReportObject; ReportObject <constructor>; ReportObject <destructor>; ReportObject destroy").unwrap();
        assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
        let rows = REPORTS.with(|reports| reports.borrow().clone());
        assert_eq!(
            rows.iter().map(|row| row.0.as_str()).collect::<Vec<_>>(),
            vec![
                "ctor-derived",
                "ctor-base",
                "named-literal",
                "named-destructor",
                "dtor-derived",
                "dtor-base"
            ]
        );
        assert_eq!(rows[0].1, NativeTclooMethodReportRole::Constructor);
        assert_eq!(rows[1].1, NativeTclooMethodReportRole::Constructor);
        assert_eq!(rows[0].2, ctor);
        assert_eq!(rows[1].2, ctor);
        assert_eq!(rows[2].1, NativeTclooMethodReportRole::Named);
        assert_eq!(rows[3].1, NativeTclooMethodReportRole::Named);
        assert_eq!(rows[4].1, NativeTclooMethodReportRole::Destructor);
        assert_eq!(rows[5].1, NativeTclooMethodReportRole::Destructor);
        assert_eq!(rows[4].2, dtor);
        assert_eq!(rows[5].2, dtor);
    }
}
#[test]
fn original_foundation_value_report_refuses_missing_and_unsupported_owners() {
    // naming.tcloo.original-selected-method-role-report
    // docs/design/analysis/name-resolution-proofs/tcloo-original-selected-method-role-report.md
    for profile in ["tcl8.4", "tcl8.5", "jim"] {
        let vm = vm(profile);
        assert!(vm.oo.method_reports.is_none(), "{profile}");
    }
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = vm(profile);
        assert_eq!(
            vm.eval_source("oo::class create MissingReport {constructor {} {self method}}")
                .unwrap()
                .code,
            Code::Ok
        );
        vm.oo.method_reports = None;
        let expected = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original Foundation method report header",
            ),
        );
        let completion = vm.eval_source("MissingReport new");
        assert!(
            matches!(completion, Err(crate::error::TclError::Host(crate::error::TclHostFailure::Execution(cause))) if cause == expected)
        );
        assert_eq!(vm.execution_refusal, Some(expected));
        assert!(vm.oo.method_reports.is_none());
    }
}
