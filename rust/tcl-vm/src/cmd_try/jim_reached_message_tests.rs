// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use tcl_syntax::native_string::NativeStringProtocol;

const ORIGINAL: &str = include_str!(
    "../../../tcl-registry/tests/data/source_contract_vectors/results/jim-try-reached-message-getters/capture07/execution.stdout"
);
fn jim() -> Vm {
    let mut vm = crate::native_fixture::interpreter_with_scripted_libraries(
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers],
    );
    vm.register("body_error", body_error);
    vm
}
fn body_error(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
    let code = Value::native_list_constructor(
        vec![Value::from_native_string_bytes(b"777".as_slice())],
        NativeStringProtocol::Jim084,
    );
    if let Err(error) = vm.store_original_jim_global_error_code(code) {
        return error;
    }
    err("BODY")
}
fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn observed(mode: usize) -> Vec<&'static str> {
    ORIGINAL
        .lines()
        .find(|row| row.starts_with(&format!("OBS|{mode}|")))
        .unwrap()
        .split('|')
        .collect()
}
fn list(values: Vec<Value>) -> Value {
    Value::native_list_constructor(values, NativeStringProtocol::Jim084)
}
fn word(value: &str) -> Value {
    Value::from_native_string_bytes(value.as_bytes())
}

#[test]
fn original_jim_unselected_script_and_later_trap_keep_reached_integer_getters_lazy() {
    for mode in [11, 12, 13] {
        let mut vm = jim();
        let integer = Value::int(777);
        let original = integer.native_lifetime_lease();
        let head = word("try");
        let variables = || list(vec![word("message"), word("options")]);
        let args = match mode {
            11 => vec![
                word("body_error"),
                word("on"),
                word("error"),
                variables(),
                word("set ::handled $::message"),
                word("on"),
                word("error"),
                list(vec![word("other"), word("options2")]),
                integer,
            ],
            12 => vec![
                word("body_error"),
                word("trap"),
                list(vec![integer, word("888")]),
                variables(),
                word("set ::handled $::message"),
            ],
            13 => vec![
                word("body_error"),
                word("on"),
                word("error"),
                variables(),
                word("set ::handled $::message"),
                word("trap"),
                list(vec![integer]),
                list(vec![word("other"), word("options2")]),
                word("set ::handled WRONG"),
            ],
            _ => unreachable!(),
        };
        assert!(original.value().resident_string_bytes().is_none());
        let completion = vm.invoke_host_original_object_vector(&head, &args);
        let row = observed(mode);
        assert_eq!(completion.code.as_int(), row[2].parse::<i64>().unwrap());
        assert_eq!(
            ValueOps::native_string_bytes(&mut vm, &completion.result)
                .unwrap()
                .as_ref(),
            unhex(row[4])
        );
        for (name, expected) in [
            (b"message".as_slice(), row[5]),
            (b"handled".as_slice(), row[6]),
            (b"other".as_slice(), row[7]),
        ] {
            let value = vm.get_var_bytes(name);
            assert_eq!(value.is_some(), expected != "-");
            if let Some(value) = value {
                assert_eq!(
                    ValueOps::native_string_bytes(&mut vm, &value)
                        .unwrap()
                        .as_ref(),
                    unhex(expected)
                );
            }
        }
        let prefix = match mode {
            11 => "UNSELECTED_SCRIPT",
            12 => "UNREACHED_MEMBER",
            13 => "REACHED_MEMBER",
            _ => unreachable!(),
        };
        let observation = ORIGINAL
            .lines()
            .find(|row| row.starts_with(&format!("{prefix}|{mode}|")))
            .unwrap()
            .split('|')
            .collect::<Vec<_>>();
        assert_eq!(
            original.value().resident_string_bytes().is_some(),
            observation[2] == "1"
        );
        assert_eq!(original.value().native_object_type_name(), observation[3]);
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn software_stage_message_binding_observes_the_actual_result_after_clause_work() {
    // Actual original custom ObjType callbacks are not a Rust capability. This
    // control publishes the observed callback result through the real result
    // owner, then uses the genuine private clause issuer and reached binder.
    for mode in [2, 5, 9] {
        let mut vm = jim();
        let body = vm.try_eval_source_bytes(b"body_error").unwrap();
        assert_eq!(body.code, Code::Error);
        let capture = vm.capture_original_jim_error_code().unwrap();
        let clauses = vec![
            word("on"),
            word("error"),
            list(vec![word("message"), word("options")]),
            word("set ::handled $::message"),
        ];
        let (handlers, finally) =
            parse_clauses(&mut vm, &clauses, Some(body.code), Some(&capture)).unwrap();
        assert!(finally.is_none());
        let handler = handlers
            .iter()
            .find(|handler| handler.jim_matches == Some(true))
            .unwrap();
        let row = observed(mode);
        let published = Value::from_native_string_bytes(unhex(row[5]));
        let actual = published.native_lifetime_lease();
        vm.adopt_native_interp_result(published).unwrap();
        assert!(!actual.value().is_same_object(&body.result));
        bind_original_jim_handler_vars(
            &mut vm,
            handler.jim_variables.as_ref().unwrap(),
            body.code,
            &capture,
        )
        .unwrap();
        let message = vm.get_var_bytes(b"message").unwrap();
        assert!(message.is_same_object(actual.value()));
        assert_eq!(
            ValueOps::native_string_bytes(&mut vm, &message)
                .unwrap()
                .as_ref(),
            unhex(row[5])
        );
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn software_stage_selected_names_keep_their_actual_headers_after_parent_shimmer() {
    let mut vm = jim();
    let body = vm.try_eval_source_bytes(b"body_error").unwrap();
    let capture = vm.capture_original_jim_error_code().unwrap();
    let result_name = word("message");
    // An independent native holder keeps the original selected child live when
    // the actual parent List releases its member storage during String shimmer.
    let original_name = result_name.clone();
    let vars = list(vec![result_name]);
    let clauses = vec![word("on"), word("error"), vars.clone(), word("list")];
    let (handlers, _) = parse_clauses(&mut vm, &clauses, Some(body.code), Some(&capture)).unwrap();
    ValueOps::native_char_len(&mut vm, &vars).unwrap();
    assert!(
        vars.native_list_backing_in(NativeStringProtocol::Jim084)
            .unwrap()
            .is_none()
    );
    assert!(original_name.native_object_is_live());
    bind_original_jim_handler_vars(
        &mut vm,
        handlers[0].jim_variables.as_ref().unwrap(),
        body.code,
        &capture,
    )
    .unwrap();
    assert!(matches!(
        original_name.native_object_snapshot().cache,
        tcl_syntax::native_object::NativeObjectCacheSnapshot::JimVariable { .. }
    ));
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn software_stage_empty_selected_names_still_require_the_original_caller() {
    let mut vm = jim();
    let body = vm.try_eval_source_bytes(b"body_error").unwrap();
    let capture = vm.capture_original_jim_error_code().unwrap();
    let clauses = vec![word("on"), word("error"), list(Vec::new()), word("list")];
    let (handlers, _) = parse_clauses(&mut vm, &clauses, Some(body.code), Some(&capture)).unwrap();
    let names = handlers[0].jim_variables.as_ref().unwrap();
    assert!(names.result.is_none() && names.options.is_none());
    bind_original_jim_handler_vars(&mut vm, names, body.code, &capture).unwrap();
    let mut foreign = jim();
    assert!(bind_original_jim_handler_vars(&mut foreign, names, body.code, &capture).is_err());
    assert!(foreign.execution_refusal.is_some());
    capture.check(&mut vm).unwrap();
}
