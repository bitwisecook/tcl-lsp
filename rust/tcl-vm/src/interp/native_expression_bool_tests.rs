// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public condition evaluation retains actual completion and operation ownership.

use super::{Code, Vm, native_operation_currency::NativeOperationScope};

const ENGINES: [&str; 6] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];

fn interpreter(engine: &str) -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment(engine).analyser_profile();
    crate::native_fixture::interpreter(profile)
}

#[test]
fn original_expression_condition_retains_successful_result_and_truth() {
    for engine in ENGINES {
        let mut vm = interpreter(engine);
        for (source, truth, result) in [
            (b"0".as_slice(), false, b"0".as_slice()),
            (b"1", true, b"1"),
            (b"2 + 3", true, b"5"),
        ] {
            let (completion, actual) = vm.try_eval_expr_bool_bytes(source).unwrap();
            assert_eq!(completion.code, Code::Ok, "{engine}/{source:?}");
            assert_eq!(actual, Some(truth), "{engine}/{source:?}");
            assert_eq!(completion.result.string_bytes().as_ref(), result);
            assert_eq!(vm.host_execution_depth, 0);
            assert!(!vm.has_entered_native_operation());
        }
    }
}

#[test]
fn original_expression_condition_preserves_full_abrupt_completion() {
    for engine in ENGINES {
        for source in [
            b"1 +".as_slice(),
            b"[return -code break -level 0 PAYLOAD]",
            b"[return -code 7 -level 0 PAYLOAD]",
        ] {
            let mut reference = interpreter(engine);
            let expected = reference.try_eval_expr_bytes(source).unwrap();
            assert_ne!(expected.code, Code::Ok, "{engine}/{source:?}");
            let mut vm = interpreter(engine);
            let (completion, truth) = vm.try_eval_expr_bool_bytes(source).unwrap();
            assert_eq!(truth, None, "{engine}/{source:?}");
            assert_eq!(completion.code, expected.code, "{engine}/{source:?}");
            assert_eq!(completion.option_origin, expected.option_origin);
            assert_eq!(
                completion.result.string_bytes(),
                expected.result.string_bytes()
            );
            assert_eq!(
                completion.options.string_bytes(),
                expected.options.string_bytes()
            );
            assert_eq!(vm.host_execution_depth, 0);
            assert!(!vm.has_entered_native_operation());
        }
    }
}

#[test]
fn original_expression_condition_boolean_failure_is_guest_completion() {
    for engine in ENGINES {
        let mut vm = interpreter(engine);
        let (completion, truth) = vm.try_eval_expr_bool_bytes(b"{NOT_A_BOOLEAN}").unwrap();
        assert_eq!(completion.code, Code::Error, "{engine}: {completion:?}");
        assert_eq!(truth, None);
        assert_ne!(completion.result.string_bytes().len(), 0);
        assert!(vm.execution_refusal.is_none());
        assert_eq!(vm.host_execution_depth, 0);
        assert!(!vm.has_entered_native_operation());
    }
}

#[test]
fn original_expression_condition_keeps_first_host_failure_separate() {
    let mut vm = interpreter("tcl9.0");
    drop(vm.refuse_host_command("condition first host failure".into()));
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(vm.try_eval_expr_bool_bytes(b"1").unwrap_err(), first);
    assert!(vm.execution_refusal.is_none());
    assert_eq!(vm.host_execution_depth, 0);
    assert!(!vm.has_entered_native_operation());
    assert_eq!(vm.try_eval_expr_bool_bytes(b"1").unwrap().1, Some(true));
}

#[test]
fn original_expression_condition_does_not_retire_enclosing_host_failure() {
    let mut vm = interpreter("tcl9.0");
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    drop(vm.refuse_host_command("enclosing condition host failure".into()));
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(vm.try_eval_expr_bool_bytes(b"1").unwrap_err(), first);
    assert_eq!(vm.execution_refusal, Some(first.clone()));
    assert_eq!(vm.host_execution_depth, 0);
    assert!(vm.has_entered_native_operation());
    drop(scope);
    assert_eq!(vm.take_host_boundary_refusal(), Some(first));
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_expression_condition_does_not_install_source_compiler() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = crate::native_fixture::core(profile);
    assert!(vm.compiler.is_none());
    assert!(
        vm.try_eval_expr_bool_bytes(b"[set condition_probe 1]")
            .is_err()
    );
    assert!(vm.compiler.is_none());
    assert_eq!(vm.host_execution_depth, 0);
    assert!(!vm.has_entered_native_operation());
}
