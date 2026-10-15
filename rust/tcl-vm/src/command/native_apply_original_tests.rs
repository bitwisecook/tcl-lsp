// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original apply arguments and selected coroutine dispatch, without source serialization.
use crate::{Vm, value::Value};
use tcl_runtime_api::Completion;
fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn row<'a>(rows: &'a str, label: &str) -> Vec<&'a str> {
    rows.lines()
        .find(|line| line.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect()
}

fn call(vm: &mut Vm, head: &[u8], args: &[Value]) -> Completion<Value> {
    vm.invoke_host_original_object_vector(&Value::new_native_string_bytes(head), args)
}
fn assert_row(
    vm: &Vm,
    completion: &Completion<Value>,
    argument: Option<&Value>,
    rows: &str,
    label: &str,
) {
    let native = row(rows, label);
    // Inspect identity before the only selected native result getter.
    let same = argument.is_some_and(|argument| completion.result.is_same_object(argument));
    assert_eq!(
        completion.code.as_int(),
        native[1].parse::<i64>().unwrap(),
        "{label}"
    );
    assert_eq!(same, native[2] == "1", "{label}/identity");
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    assert_eq!(
        completion
            .result
            .native_string_bytes(protocol)
            .unwrap()
            .as_ref(),
        unhex(native[3]),
        "{label}/bytes"
    );
}

#[test]
fn apply_returns_original_counted_argument_objects() {
    // Native proof: naming.lambda.original-argument-object-return
    // docs/design/analysis/name-resolution-proofs/lambda-original-argument-object-return.md
    let mut comparisons = 0;
    for (engine, rows) in [
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_apply_original_argv/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_apply_original_argv/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_apply_original_argv/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_apply_original_argv/9.1.0/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../tcl-registry/tests/data/native_apply_original_argv/jim/stdout.tsv"
            ),
        ),
    ] {
        for input in rows.lines().filter(|line| line.starts_with("INPUT_")) {
            let fields: Vec<_> = input.split('|').collect();
            let case = fields[0].strip_prefix("INPUT_").unwrap();
            let argument = Value::new_native_string_bytes(unhex(fields[2]));
            let mut vm =
                crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
            let lambda = Value::new_native_string_bytes(b"{x} {return $x}".as_slice());
            let completion = call(&mut vm, b"apply", &[lambda, argument.clone()]);
            assert_row(
                &vm,
                &completion,
                Some(&argument),
                rows,
                &format!("APPLY_{case}"),
            );
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 35);
}
#[test]
fn apply_coroutine_keeps_arguments_and_selects_the_actual_shadowed_handler() {
    // Native proof: naming.lambda.original-coroutine-call-and-shadowing
    // docs/design/analysis/name-resolution-proofs/lambda-original-coroutine-call-and-shadowing.md
    let mut comparisons = 0;
    for (engine, rows) in [
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_apply_original_argv/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_apply_original_argv/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_apply_original_argv/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        for input in rows.lines().filter(|line| line.starts_with("INPUT_")) {
            let fields: Vec<_> = input.split('|').collect();
            let case = fields[0].strip_prefix("INPUT_").unwrap();
            let argument = Value::new_native_string_bytes(unhex(fields[2]));
            let mut vm =
                crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
            let mut args = [b"c".as_slice(), b"apply", b"{x} {yield READY; return $x}"]
                .map(Value::new_native_string_bytes)
                .to_vec();
            args.push(argument.clone());
            let start = call(&mut vm, b"coroutine", &args);
            assert_row(
                &vm,
                &start,
                Some(&argument),
                rows,
                &format!("COROUTINE_START_{case}"),
            );
            let finish = call(&mut vm, b"c", &[]);
            assert_row(
                &vm,
                &finish,
                Some(&argument),
                rows,
                &format!("COROUTINE_RETURN_{case}"),
            );
            comparisons += 1;
        }
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(engine));
        let shadow = vm
            .eval_source("proc apply args {return SHADOW}; coroutine c apply {} VALUE")
            .unwrap();
        assert_row(&vm, &shadow, None, rows, "SHADOWED_APPLY_COROUTINE");
    }
    assert_eq!(comparisons, 21);
}
#[test]
fn parked_lambda_registration_retires_after_coroutine_deletion() {
    // Implementation proof: naming.lambda.suspended-registration-lifetime
    // docs/design/analysis/name-resolution-proofs/lambda-suspended-registration-lifetime.md
    for (body, finish, code) in [
        ("yield READY; return $x", "rename c {}", 0),
        ("yield READY; return $x", "c", 0),
        ("yield READY; error FINISH", "c", 1),
    ] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl8.6"));
        let pending = vm
            .eval_source(&["coroutine c apply {{x} {", body, "}} VALUE"].concat())
            .unwrap();
        assert!(pending.code.is_ok(), "parked={pending:?}");
        let registrations = vm.coro.parked_lambda_registrations();
        assert_eq!(registrations.len(), 1, "parked={pending:?}");
        let registration = &registrations[0];
        assert!(registration.is_attached());

        // This is an internal activation registration, not a guest command
        // in a declared namespace. Observe its actual handle for cleanup.
        let public = vm
            .eval_source("info commands ::tcl::apply::lambda*")
            .unwrap();
        assert!(public.code.is_ok());
        assert!(public.result.as_list().unwrap().is_empty());
        assert!(registration.is_attached());

        let completion = vm.eval_source(finish).unwrap();
        assert_eq!(completion.code.as_int(), code, "finish={completion:?}");
        assert!(!registration.is_attached());
        assert!(vm.coro.parked_lambda_registrations().is_empty());
    }
}

const LAMBDA_NAMESPACE_INPUTS: [(&str, &[u8]); 9] = [
    ("PLAIN", b"ns"),
    ("COLON", b":ns"),
    ("ABSOLUTE", b"::ns"),
    ("ABSOLUTE_COLON", b":::ns"),
    ("EMPTY", b""),
    ("RELATIVE_ZERO", b":ns\0suffix"),
    ("ABSOLUTE_ZERO", b"::ns\0suffix"),
    ("LEADING_ZERO", b"\0suffix"),
    ("UNICODE_COLON", b":\xc3\xa9"),
];

const LAMBDA_NAMESPACE_ROWS: [(&str, &str); 5] = [
    (
        "tcl8.5",
        include_str!(
            "../../../tcl-registry/tests/data/native_lambda_object_getter/8.5.19/stdout.tsv"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../tcl-registry/tests/data/native_lambda_object_getter/8.6.18/stdout.tsv"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../tcl-registry/tests/data/native_lambda_object_getter/9.0.4/stdout.tsv"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../tcl-registry/tests/data/native_lambda_object_getter/9.1.0/stdout.tsv"
        ),
    ),
    (
        "jimtcl",
        include_str!("../../../tcl-registry/tests/data/native_lambda_object_getter/jim/stdout.tsv"),
    ),
];

#[test]
fn original_lambda_namespace_objects_match_constructor_and_getter_controls() {
    // naming.lambda.original-namespace-constructor-and-getter
    // docs/design/analysis/name-resolution-proofs/lambda-original-namespace-constructor-and-getter.md
    // Compare actual original object-vector completion/result and the retained
    // namespace object's subsequent getter. BEFORE/AFTER header rows are not
    // compared; Jim has no tested native ByteArray constructor.
    let mut comparisons = 0;
    for (engine, rows) in LAMBDA_NAMESPACE_ROWS {
        for (name, original) in LAMBDA_NAMESPACE_INPUTS {
            for byte_array in [false, true] {
                if byte_array && engine == "jimtcl" {
                    continue;
                }
                let constructor = if byte_array {
                    "BYTE_ARRAY"
                } else {
                    "RAW_STRING"
                };
                let apply_label = format!("{constructor}_{name}_APPLY");
                let getter_label = format!("{constructor}_{name}_GETTER");
                let native_apply = row(rows, &apply_label);
                let native_getter = row(rows, &getter_label);
                let mut vm = crate::native_fixture::interpreter(
                    crate::environment::profile_for_dialect(engine),
                );
                let setup = vm.eval_source("namespace eval ns {}; namespace eval :ns {}; namespace eval \\u00e9 {}; namespace eval :\\u00e9 {}").unwrap();
                assert_eq!(setup.code, tcl_runtime_api::Code::Ok);
                let namespace = if byte_array {
                    Value::byte_array(original)
                } else {
                    Value::new_native_string_bytes(original)
                };
                let protocol = vm
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                let lambda = Value::native_list_constructor(
                    vec![
                        Value::new_native_string_bytes(b"".as_slice()),
                        Value::new_native_string_bytes(b"namespace current".as_slice()),
                        namespace.clone(),
                    ],
                    protocol,
                );
                let completion = call(&mut vm, b"apply", &[lambda]);
                assert_eq!(
                    completion.code.as_int(),
                    native_apply[1].parse::<i64>().unwrap(),
                    "{engine}/{apply_label}"
                );
                assert_eq!(
                    completion
                        .result
                        .native_string_bytes(protocol)
                        .unwrap()
                        .as_ref(),
                    unhex(native_apply[2]),
                    "{engine}/{apply_label}"
                );
                assert_eq!(
                    vm.native_name_operand_bytes(&namespace).unwrap().as_ref(),
                    unhex(native_getter[2]),
                    "{engine}/{getter_label}"
                );
                comparisons += 1;
            }
        }
    }
    assert_eq!(comparisons, 81);
}

#[test]
fn jim_lambda_keeps_absent_original_namespace_objects_without_c_lookup() {
    for original in [
        b"AbsentLambda".as_slice(),
        b"::AbsentLambda",
        b"::::AbsentLambda",
        b":AbsentLambda",
        b"Absent:Lambda",
        b"Absent\0Tail",
        b"",
        b"Absent::Child",
    ] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"));
        let setup = vm
            .eval_source("namespace eval Caller {proc launch {lambda} {apply $lambda}}")
            .unwrap();
        assert_eq!(setup.code, tcl_runtime_api::Code::Ok);
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let namespace = Value::new_native_string_bytes(original);
        let lambda = Value::native_list_constructor(
            vec![
                Value::new_native_string_bytes(b"".as_slice()),
                Value::new_native_string_bytes(b"namespace current".as_slice()),
                namespace.clone(),
            ],
            protocol,
        );
        let completion = call(&mut vm, b"::Caller::launch", &[lambda]);
        assert_eq!(
            completion.code,
            tcl_runtime_api::Code::Ok,
            "namespace={original:?}, host refusal={:?}",
            vm.execution_refusal
        );
        assert_eq!(
            completion
                .result
                .native_string_bytes(protocol)
                .unwrap()
                .as_ref(),
            [b"::".as_slice(), original].concat(),
        );
        assert_eq!(
            vm.native_name_operand_bytes(&namespace).unwrap().as_ref(),
            original
        );
        assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_lambda_missing_namespaces_match_native_result_and_error_code() {
    let mut comparisons = 0;
    for (engine, rows) in [
        (
            "tcl8.5",
            include_str!(
                "../../../tcl-registry/tests/data/native_lambda_missing_namespace_report/8.5.19/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_lambda_missing_namespace_report/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_lambda_missing_namespace_report/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_lambda_missing_namespace_report/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        for (name, original) in [
            ("PLAIN", b"Missing".as_slice()),
            ("ABSOLUTE", b"::Missing"),
            ("REPEATED", b"::::Missing"),
            ("LITERAL_COLON", b":Missing"),
            ("EMBEDDED_COLON", b"Missing:Tail"),
            ("SPACE", b"Missing Name"),
            ("RAW_NUL", b"Missing\0Tail"),
            ("LEADING_NUL", b"\0Tail"),
            ("EMPTY", b""),
        ] {
            for byte_array in [false, true] {
                let constructor = if byte_array {
                    "BYTE_ARRAY"
                } else {
                    "RAW_STRING"
                };
                let label = format!("{constructor}_{name}");
                let oracle = |kind: &str| {
                    rows.lines()
                        .find(|line| line.starts_with(&format!("{kind}|{label}|")))
                        .unwrap()
                        .split('|')
                        .collect::<Vec<_>>()
                };
                let apply = oracle("APPLY");
                let mut vm = crate::native_fixture::interpreter(
                    crate::environment::profile_for_dialect(engine),
                );
                let namespace = if byte_array {
                    Value::byte_array(original)
                } else {
                    Value::new_native_string_bytes(original)
                };
                let protocol = vm
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                let lambda = Value::native_list_constructor(
                    vec![
                        Value::new_native_string_bytes(b"".as_slice()),
                        Value::new_native_string_bytes(b"namespace current".as_slice()),
                        namespace.clone(),
                    ],
                    protocol,
                );
                let completion = call(&mut vm, b"apply", &[lambda]);
                assert_eq!(
                    completion.code.as_int(),
                    apply[2].parse::<i64>().unwrap(),
                    "{engine}/{label}"
                );
                assert_eq!(
                    completion
                        .result
                        .native_string_bytes(protocol)
                        .unwrap()
                        .as_ref(),
                    unhex(apply[3]),
                    "{engine}/{label}"
                );
                if completion.code == tcl_runtime_api::Code::Error {
                    let tuple = vm.get_var_bytes(b"::errorCode").unwrap();
                    assert_eq!(
                        tuple.native_string_bytes(protocol).unwrap().as_ref(),
                        unhex(oracle("ERROR_CODE")[3]),
                        "{engine}/{label}/errorCode"
                    );
                }
                assert_eq!(
                    vm.native_name_operand_bytes(&namespace).unwrap().as_ref(),
                    unhex(oracle("GETTER")[3]),
                    "{engine}/{label}/getter"
                );
                assert!(vm.execution_refusal.is_none(), "{engine}/{label}");
                comparisons += 1;
            }
        }
    }
    assert_eq!(comparisons, 72);
}

#[test]
fn jim_lambda_preflight_leaves_unentered_namespace_objects_stringless() {
    let rows = include_str!(
        "../../../tcl-registry/tests/data/native_jim_lambda_namespace_preflight/jim/stdout.tsv"
    );
    for (label, body, required) in [
        ("EMPTY_BODY", "", false),
        ("RETURN_BODY", "return BODY", false),
        ("NAMESPACE_BODY", "namespace current", false),
        ("EMPTY_REQUIRED", "", true),
        ("NONEMPTY_REQUIRED", "return BODY", true),
    ] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"));
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let namespace = Value::int(42);
        let parameters = if required {
            Value::new_native_string_bytes(b"required".as_slice())
        } else {
            Value::native_list_constructor(vec![], protocol)
        };
        let lambda = Value::native_list_constructor(
            vec![
                parameters,
                Value::new_native_string_bytes(body.as_bytes()),
                namespace.clone(),
            ],
            protocol,
        );
        let fields = |kind: &str| {
            rows.lines()
                .find(|line| line.starts_with(&format!("{kind}|{label}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>()
        };
        assert_eq!(
            namespace.resident_string_bytes().is_some(),
            fields("BEFORE")[2] == "1",
            "{label}/before"
        );
        let completion = call(&mut vm, b"apply", &[lambda]);
        assert_eq!(
            namespace.resident_string_bytes().is_some(),
            fields("AFTER")[2] == "1",
            "{label}/after"
        );
        let result = fields("RESULT");
        assert_eq!(
            completion.code.as_int(),
            result[2].parse::<i64>().unwrap(),
            "{label}/code"
        );
        assert_eq!(
            completion
                .result
                .native_string_bytes(protocol)
                .unwrap()
                .as_ref(),
            unhex(result[3]),
            "{label}/result"
        );
        assert_eq!(
            vm.native_name_operand_bytes(&namespace).unwrap().as_ref(),
            unhex(fields("GETTER")[3]),
            "{label}/getter"
        );
        assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn jim_lambda_unreached_command_lookup_keeps_original_namespace_unmaterialized() {
    let rows = include_str!(
        "../../../tcl-registry/tests/data/native_jim_lambda_deferred_context/jim/stdout.tsv"
    );
    for (label, source) in [("COMMENT_BODY", "# only a comment"), ("INVALID_BODY", "{")] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"));
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let namespace = Value::int(42);
        let context =
            crate::interp::OriginalJimNamespaceContext::for_lambda(&mut vm, &namespace).unwrap();
        let execution = crate::interp::NamespaceExecutionContext::OriginalJim(context);
        let body = Value::new_native_string_bytes(source.as_bytes());
        let prepared = vm
            .prepare_native_jim_script_unit(&body, execution.clone())
            .unwrap();
        assert!(
            namespace.resident_string_bytes().is_none(),
            "{label}/preparation getter"
        );
        assert!(
            execution.resolved_token().is_none(),
            "{label}/preparation coordinate"
        );
        assert!(
            prepared.source_namespace.compiled_path().is_none(),
            "{label}/compiled path"
        );
        let lambda = Value::native_list_constructor(
            vec![
                Value::native_list_constructor(vec![], protocol),
                body,
                namespace.clone(),
            ],
            protocol,
        );
        let completion = call(&mut vm, b"apply", &[lambda]);
        let expected = |kind: &str| {
            rows.lines()
                .find(|line| line.starts_with(&format!("{kind}|{label}|")))
                .unwrap()
                .split('|')
                .collect::<Vec<_>>()
        };
        assert_eq!(
            namespace.resident_string_bytes().is_some(),
            expected("AFTER")[2] == "1",
            "{label}/getter"
        );
        assert_eq!(
            completion.code.as_int(),
            expected("RESULT")[2].parse::<i64>().unwrap(),
            "{label}/code"
        );
        assert_eq!(
            completion
                .result
                .native_string_bytes(protocol)
                .unwrap()
                .as_ref(),
            unhex(expected("RESULT")[3]),
            "{label}/result"
        );
        assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
        assert!(vm.execution_refusal.is_none(), "{label}/host refusal");
    }
}

#[test]
fn jim_lambda_temporary_publication_keeps_the_original_execution_namespace_separate() {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"));
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    let namespace = Value::int(42);
    let lambda = Value::native_list_constructor(
        vec![
            Value::native_list_constructor(Vec::new(), protocol),
            Value::new_native_string_bytes(b"return BODY".as_slice()),
            namespace.clone(),
        ],
        protocol,
    );
    let definition = super::build_lambda_proc(
        &mut vm,
        &lambda,
        vec![
            Value::new_native_string_bytes(b"apply".as_slice()),
            lambda.clone(),
        ],
    )
    .unwrap_or_else(|completion| {
        panic!(
            "actual lambda definition failed: {:?}; refusal={:?}",
            completion.code, vm.execution_refusal
        )
    });
    let declaration = definition.binding.declaration();
    assert_eq!(declaration.command_ns_id, tcl_core_types::ROOT_NS);
    assert_eq!(
        declaration.simple_name.as_bytes(),
        declaration.name.as_bytes()
    );
    assert!(matches!(
        &declaration.execution_namespace,
        crate::interp::ProcedureNamespace::OriginalJim(_)
    ));
    assert!(
        declaration
            .execution_namespace
            .context()
            .resolved_token()
            .is_none()
    );
    assert!(
        declaration
            .native_jim_namespace
            .as_ref()
            .unwrap()
            .is_same_object(&namespace)
    );
    assert!(namespace.resident_string_bytes().is_none());
    let (storage, selected) = vm
        .lookup_command_bytes_checked(tcl_core_types::ROOT_NS, declaration.simple_name.as_bytes())
        .expect("actual Jim flat command query")
        .expect("published temporary binding");
    assert_eq!(
        definition.registration.key(),
        Some(crate::interp::CommandSidecarKey::visible(storage))
    );
    let crate::command::Command::Proc(selected) = selected else {
        panic!("actual procedure publication");
    };
    assert!(std::rc::Rc::ptr_eq(&selected.declaration(), &declaration));
    assert!(namespace.resident_string_bytes().is_none());
    assert!(
        vm.retire_command_lifecycle_key(&definition.registration.key().unwrap())
            .is_some()
    );
    assert!(!definition.registration.is_attached());
    assert!(namespace.resident_string_bytes().is_none());
    assert!(vm.execution_refusal.is_none());
}
