// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual C interpreter result roles and completion transport ownership.

use super::{Vm, ok};
use crate::Value;
use tcl_dialect::TclVersion;

const RELEASES: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];

#[test]
fn completion_transport_borrows_the_original_result_role() {
    for release in RELEASES {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(release.dialect_name()).unwrap(),
        );
        let original = Value::new_native_string_bytes(b"original\xff\0tail".as_slice());
        let identity = original.native_object_identity();
        let completion = vm.publish_native_interp_completion(ok(original)).unwrap();
        assert_eq!(completion.result.native_object_identity(), identity);
        assert_eq!(completion.result.native_object_reference_count(), 1);
        vm.with_native_interp_result(|result| {
            assert_eq!(result.native_object_identity(), identity);
            assert_eq!(result.native_object_reference_count(), 1);
        })
        .unwrap();
        let retained = completion.result.clone();
        assert_eq!(retained.native_object_reference_count(), 2);
        drop(retained);
        let list = Value::list(vec![completion.result]);
        vm.with_native_interp_result(|result| {
            assert_eq!(result.native_object_reference_count(), 2)
        })
        .unwrap();
        drop(list);
        vm.with_native_interp_result(|result| {
            assert_eq!(result.native_object_reference_count(), 1)
        })
        .unwrap();
    }
}

#[test]
fn reset_reuses_only_the_unshared_original_result_header() {
    for release in RELEASES {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(release.dialect_name()).unwrap(),
        );
        let completion = vm
            .publish_native_interp_completion(ok(Value::int(42)))
            .unwrap();
        let identity = completion.result.native_object_identity();
        vm.reset_native_jim_result().unwrap();
        assert_eq!(completion.result.native_object_identity(), identity);
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            b""
        );
        assert_eq!(completion.result.native_object_type_name(), "none");
        assert_eq!(completion.result.native_object_reference_count(), 1);
        let retained = completion.result.clone();
        vm.reset_native_jim_result().unwrap();
        vm.with_native_interp_result(|result| {
            assert_ne!(result.native_object_identity(), identity);
            assert_eq!(result.native_object_reference_count(), 1);
        })
        .unwrap();
        assert_eq!(retained.native_object_reference_count(), 1);
    }
}

#[test]
fn trace_chain_saves_the_selected_native_result_reference() {
    for release in RELEASES {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(release.dialect_name()).unwrap(),
        );
        let before = vm
            .publish_native_interp_completion(ok(Value::string("BEFORE")))
            .unwrap();
        let script_saved = vm.save_native_script_trace_result().unwrap();
        assert_eq!(script_saved.is_some(), release == TclVersion::V8_4);
        if let Some(script_saved) = script_saved {
            assert_eq!(before.result.native_object_reference_count(), 1);
            vm.with_native_interp_result(|current| {
                assert!(!current.is_same_object(&before.result));
                assert_eq!(current.native_object_reference_count(), 1);
                assert_eq!(current.resident_string_bytes().unwrap().as_ref(), b"");
            })
            .unwrap();
            vm.restore_native_interp_trace_result(script_saved);
        }
        let saved = vm.save_native_interp_trace_result().unwrap();
        if release == TclVersion::V8_4 {
            assert!(saved.is_none());
            assert_eq!(before.result.native_object_reference_count(), 1);
            continue;
        }
        let saved = saved.expect("modern C saves one real trace-chain reference");
        assert_eq!(before.result.native_object_reference_count(), 2);
        let callback = vm
            .publish_native_interp_completion(ok(Value::string("CALLBACK")))
            .unwrap();
        assert_eq!(before.result.native_object_reference_count(), 1);
        vm.restore_native_interp_trace_result(saved);
        vm.with_native_interp_result(|result| {
            assert!(result.is_same_object(&before.result));
            assert_eq!(result.native_object_reference_count(), 1);
        })
        .unwrap();
        assert_eq!(callback.result.native_object_reference_count(), 0);
    }
}

#[test]
fn modern_trace_state_restores_return_and_error_metadata_without_child_copies() {
    for release in RELEASES.into_iter().skip(1) {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(release.dialect_name()).unwrap(),
        );
        vm.set_native_c_return_state(7, 3);
        let info = Value::string("ORIGINAL INFO");
        let code = Value::string("ORIGINAL CODE");
        let options = Value::list(vec![Value::string("-custom"), Value::string("ORIGINAL")]);
        vm.native_errors.native_error_info = Some(info.clone());
        vm.native_errors.native_error_info_len = 13;
        vm.native_errors.primitive_error_code = Some(code.clone());
        vm.native_errors.native_return_options = Some(options.clone());
        vm.native_errors.error_logged = true;
        let context = Value::string("ORIGINAL CONTEXT");
        if release.has_error_stack() {
            vm.native_errors
                .error_stack
                .begin_inner(Value::string("INNER"), context.clone());
            vm.native_errors.error_stack.mark_reset();
        }
        let saved = vm.save_native_interp_trace_result().unwrap().unwrap();
        assert_eq!(info.native_object_reference_count(), 3);
        assert_eq!(code.native_object_reference_count(), 3);
        assert_eq!(options.native_object_reference_count(), 3);
        assert_eq!(
            context.native_object_reference_count(),
            usize::from(release.has_error_stack()) + 1
        );
        vm.set_native_c_return_state(1, 0);
        vm.native_errors.native_error_info = None;
        vm.native_errors.native_error_info_len = 0;
        vm.native_errors.primitive_error_code = None;
        vm.native_errors.native_return_options = None;
        vm.native_errors.error_logged = false;
        if release.has_error_stack() {
            vm.native_errors
                .error_stack
                .begin_inner(Value::string("INNER"), Value::string("CALLBACK"));
        }
        vm.restore_native_interp_trace_result(saved);
        assert_eq!(vm.native_errors.native_c_return_state.code, 7);
        assert_eq!(vm.native_errors.native_c_return_state.level, 3);
        assert!(
            vm.native_errors
                .native_error_info
                .as_ref()
                .unwrap()
                .is_same_object(&info)
        );
        assert_eq!(vm.native_errors.native_error_info_len, 13);
        assert!(
            vm.native_errors
                .primitive_error_code
                .as_ref()
                .unwrap()
                .is_same_object(&code)
        );
        assert!(
            vm.native_errors
                .native_return_options
                .as_ref()
                .unwrap()
                .is_same_object(&options)
        );
        assert!(vm.native_errors.error_logged);
        if release.has_error_stack() {
            assert!(vm.native_errors.error_stack.is_reset());
            assert!(vm.native_errors.error_stack.entries()[1].is_same_object(&context));
        }
    }
}

fn vector_failure(_vm: &mut Vm, arguments: &[Value]) -> tcl_runtime_api::Completion<Value> {
    assert_eq!(arguments[0].native_object_reference_count(), 2);
    super::err("VECTOR FAILURE")
}

fn vector_success(_vm: &mut Vm, arguments: &[Value]) -> tcl_runtime_api::Completion<Value> {
    assert_eq!(arguments[0].native_object_reference_count(), 2);
    ok(Value::string("DONE"))
}

#[test]
fn jim_original_vector_captures_error_arguments_before_releasing_argv() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = crate::native_fixture::core(profile);
    vm.register("vector_failure", vector_failure);
    let arguments = [Value::new_native_string_bytes(
        b"original\xff\0tail".as_slice(),
    )];
    let completion = vm.try_invoke_command("vector_failure", &arguments).unwrap();
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    assert_eq!(arguments[0].native_object_reference_count(), 2);
    let trace = vm.jim_stacktrace();
    let fields = trace.as_list().unwrap();
    let captured = fields[3].as_list().unwrap();
    assert!(captured[1].is_same_object(&arguments[0]));
    assert!(vm.jim_errors.frames.is_empty());
    assert!(vm.jim_errors.vectors.is_empty());
}

#[test]
fn jim_successful_vector_releases_all_temporary_argument_owners() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = crate::native_fixture::core(profile);
    vm.register("vector_success", vector_success);
    let arguments = [Value::new_native_string_bytes(
        b"original\xff\0tail".as_slice(),
    )];
    let completion = vm.try_invoke_command("vector_success", &arguments).unwrap();
    assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
    assert_eq!(arguments[0].native_object_reference_count(), 1);
    assert!(vm.jim_errors.frames.is_empty());
    assert!(vm.jim_errors.vectors.is_empty());
}
