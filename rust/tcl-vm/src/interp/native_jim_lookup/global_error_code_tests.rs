// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use tcl_runtime_api::Code;
use tcl_syntax::value::ValueOps;

const OBSERVATIONS: &str = include_str!(
    "../../../../../runtime/rust/tests/data/native_jim_original_global_error_code_getter/observations-20261015.tsv"
);
const SETUPS: &str = include_str!(
    "../../../../../runtime/rust/tests/data/native_jim_original_global_error_code_getter/setups.tsv"
);
const CALL: &[u8] = include_bytes!(
    "../../../../../runtime/rust/tests/data/native_jim_original_global_error_code_getter/call.tcl"
);
thread_local! {
    static MODE: Cell<usize> = const { Cell::new(0) };
    static EXPECTED: Cell<Option<usize>> = const { Cell::new(None) };
}

fn jim() -> Vm {
    crate::native_fixture::interpreter_with_scripted_libraries(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers],
    )
}

fn read_in_caller(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
    let caller = super::super::native_name_consumers::OriginalNameConsumerScope::enter(vm).unwrap();
    assert_ne!(vm.current_ns_id(), ROOT_NS);
    assert!(vm.frames.len() > 1);
    let namespace = vm.current_ns_id();
    let local = vm
        .get_var_bytes(b"errorCode")
        .unwrap()
        .native_lifetime_lease();
    vm.adopt_native_interp_result(Value::from_native_string_bytes(b"MARKER".as_slice()))
        .unwrap();
    let value = vm.read_original_jim_global_error_code().unwrap();
    let mode = MODE.with(Cell::get);
    let row = OBSERVATIONS
        .lines()
        .find(|line| line.starts_with(&format!("GLOBAL_READ|{mode}|")))
        .unwrap()
        .split('|')
        .collect::<Vec<_>>();
    assert_eq!(value.is_some(), row[2] == "1");
    assert_eq!(
        value
            .as_ref()
            .is_some_and(|value| EXPECTED.with(Cell::get) == Some(value.native_object_identity())),
        row[3] == "1"
    );
    caller.check(vm).unwrap();
    assert_eq!(vm.current_ns_id(), namespace);
    assert!(
        vm.get_var_bytes(b"errorCode")
            .unwrap()
            .is_same_object(local.value())
    );
    if let Some(value) = &value {
        assert_eq!(value.resident_string_bytes().is_some(), row[7] == "1");
        assert_eq!(value.native_object_type_name(), row[8]);
    }
    let bytes = vm
        .with_native_interp_result(|result| result.resident_string_bytes().unwrap().to_vec())
        .unwrap();
    let actual = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(actual, row[9]);
    assert!(vm.execution_refusal.is_none());
    let result = vm
        .with_native_interp_result(|result| result.native_lifetime_lease())
        .unwrap();
    super::super::ok(result.into_value())
}

#[test]
fn original_jim_quiet_global_error_code_getter_keeps_actual_headers_and_caller_context() {
    for line in SETUPS.lines() {
        let (mode, source) = line.split_once('|').unwrap();
        let mode: usize = mode.parse().unwrap();
        MODE.with(|value| value.set(mode));
        EXPECTED.with(|value| value.set(None));
        let mut vm = jim();
        let value = Value::int(777);
        if mode == 1 || mode == 2 {
            let name = Value::from_native_string_bytes(if mode == 1 {
                &b"errorCode"[..]
            } else {
                &b"target"[..]
            });
            vm.store_original_named_variable(&name, value.clone())
                .unwrap();
            EXPECTED.with(|expected| expected.set(Some(value.native_object_identity())));
        }
        assert_eq!(
            vm.try_eval_source_bytes(source.as_bytes()).unwrap().code,
            Code::Ok
        );
        vm.register("read_selected_error_code", read_in_caller);
        assert_eq!(vm.try_eval_source_bytes(CALL).unwrap().code, Code::Ok);
        assert_eq!(vm.current_ns_id(), ROOT_NS);
        assert_eq!(vm.frames.len(), 1);
        EXPECTED.with(|value| value.set(None));
    }
}

#[test]
fn original_jim_return_receipt_keeps_the_global_header_without_a_value_getter() {
    let mut vm = jim();
    let value = Value::int(777);
    let name = Value::from_native_string_bytes(b"errorCode".as_slice());
    vm.store_original_named_variable(&name, value.clone())
        .unwrap();
    let receipt = vm.jim_return_receipt().unwrap();
    assert!(receipt.error_code.unwrap().is_same_object(&value));
    assert!(value.resident_string_bytes().is_none());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn captured_error_code_options_do_not_reach_a_second_global_getter() {
    let mut vm = jim();
    let original = Value::int(777);
    let name = Value::from_native_string_bytes(b"errorCode".as_slice());
    vm.store_original_named_variable(&name, original.clone())
        .unwrap();
    let native_references = original.native_object_reference_count();
    let captured = vm.capture_original_jim_error_code().unwrap();
    assert_eq!(original.native_object_reference_count(), native_references);
    let setup = SETUPS
        .lines()
        .find_map(|line| line.strip_prefix("3|"))
        .unwrap();
    assert_eq!(
        vm.try_eval_source_bytes(setup.as_bytes()).unwrap().code,
        Code::Ok
    );
    vm.adopt_native_interp_result(Value::from_native_string_bytes(b"MARKER".as_slice()))
        .unwrap();
    let options = vm
        .current_jim_options_for_exit_code_with_error_code(Code::Error, &captured)
        .unwrap();
    assert!(
        crate::command::opt_get(&options, "-errorcode")
            .unwrap()
            .is_same_object(&original)
    );
    assert_eq!(
        vm.with_native_interp_result(|result| result.resident_string_bytes().unwrap().to_vec())
            .unwrap(),
        b"MARKER"
    );
    assert!(original.resident_string_bytes().is_none());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn a_retired_captured_error_code_refuses_instead_of_becoming_a_quiet_miss() {
    let mut vm = jim();
    let original = Value::int(777);
    let name = Value::from_native_string_bytes(b"errorCode".as_slice());
    vm.store_original_named_variable(&name, original.clone())
        .unwrap();
    let native_references = original.native_object_reference_count();
    let captured = vm.capture_original_jim_error_code().unwrap();
    assert_eq!(original.native_object_reference_count(), native_references);
    let lease = original.native_lifetime_lease();
    drop(original);
    assert_eq!(
        vm.try_eval_source_bytes(b"unset errorCode").unwrap().code,
        Code::Ok
    );
    assert!(!lease.value().native_object_is_live());
    assert!(
        vm.current_jim_options_for_exit_code_with_error_code(Code::Error, &captured)
            .is_err()
    );
    let first = vm.execution_refusal.clone().expect("actual header refusal");
    assert!(vm.read_original_jim_global_error_code().is_err());
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
}

#[test]
fn unused_retired_capture_does_not_add_an_error_header_read_to_ok_options() {
    let mut vm = jim();
    let original = Value::int(777);
    let name = Value::from_native_string_bytes(b"errorCode".as_slice());
    vm.store_original_named_variable(&name, original.clone())
        .unwrap();
    let native_references = original.native_object_reference_count();
    let captured = vm.capture_original_jim_error_code().unwrap();
    assert_eq!(original.native_object_reference_count(), native_references);
    let lease = original.native_lifetime_lease();
    drop(original);
    assert_eq!(
        vm.try_eval_source_bytes(b"unset errorCode").unwrap().code,
        Code::Ok
    );
    assert!(!lease.value().native_object_is_live());
    let options = vm
        .current_jim_options_for_exit_code_with_error_code(Code::Ok, &captured)
        .unwrap();
    assert!(crate::command::opt_get(&options, "-errorcode").is_none());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn catch_try_quiet_global_capture_matches_original_reached_message_and_options() {
    const CAPTURE: &str = include_str!(
        "../../../../tcl-registry/tests/data/source_contract_vectors/results/jim-catch-try-quiet-global-capture/capture03/execution.stdout"
    );
    fn unhex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    for (input, observed) in CAPTURE
        .lines()
        .collect::<Vec<_>>()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|rows| (rows[0], rows[1]))
    {
        let input = input.split('|').collect::<Vec<_>>();
        let observed = observed.split('|').collect::<Vec<_>>();
        let mut vm = jim();
        assert_eq!(
            vm.try_eval_source_bytes(b"set keep SAVED; set handler NONE; set finally NONE")
                .unwrap()
                .code,
            Code::Ok
        );
        let saved = vm.get_var_bytes(b"keep").unwrap().native_lifetime_lease();
        let completion = vm.try_eval_source_bytes(&unhex(input[3])).unwrap();
        assert_eq!(
            completion.code.as_int(),
            observed[3].parse::<i64>().unwrap()
        );
        assert_eq!(
            ValueOps::native_string_bytes(&mut vm, &completion.result)
                .unwrap()
                .as_ref(),
            unhex(observed[4])
        );
        let message = vm.get_var_bytes(b"message");
        assert_eq!(message.is_some(), observed[5] != "-");
        if let Some(message) = message {
            assert_eq!(
                ValueOps::native_string_bytes(&mut vm, &message)
                    .unwrap()
                    .as_ref(),
                unhex(observed[5])
            );
        }
        let options = vm.get_var_bytes(b"options");
        assert_eq!(options.is_some(), observed[6] != "-");
        let errorcode = options
            .as_ref()
            .and_then(|options| crate::command::opt_get(options, "-errorcode"));
        assert_eq!(
            errorcode
                .as_ref()
                .is_some_and(|value| value.is_same_object(saved.value())),
            observed[7] == "1"
        );
        assert_eq!(errorcode.is_some(), observed[8] != "-");
        for (name, expected) in [
            (b"handler".as_slice(), observed[9]),
            (b"finally".as_slice(), observed[10]),
        ] {
            let value = vm.get_var_bytes(name).unwrap();
            assert_eq!(
                ValueOps::native_string_bytes(&mut vm, &value)
                    .unwrap()
                    .as_ref(),
                unhex(expected)
            );
        }
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_error_code_capture_does_not_cross_interpreter_ownership() {
    let mut original = jim();
    let capture = original.capture_original_jim_error_code().unwrap();
    let mut foreign = jim();
    assert!(capture.check(&mut foreign).is_err());
    assert!(foreign.execution_refusal.is_some());
    assert!(original.execution_refusal.is_none());
    capture.check(&mut original).unwrap();
}
