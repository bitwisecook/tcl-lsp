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

//! `apply` arguments, lambda namespaces and coroutine dispatch, compared with the tclsh and jimsh captures.

use crate::prelude::*;

fn unhex(value: &str) -> Vec<u8> {
    crate::fixture::unhex(value)
}
fn row<'a>(rows: &'a str, label: &str) -> Vec<&'a str> {
    rows.lines()
        .find(|line| line.split('|').next() == Some(label))
        .unwrap()
        .split('|')
        .collect()
}

fn call(vm: &mut Vm, head: &[u8], args: &[Value]) -> Completion<Value> {
    crate::fixture::call(vm, std::str::from_utf8(head).unwrap(), args)
}
fn assert_row(vm: &Vm, completion: &Completion<Value>, rows: &str, label: &str) {
    let native = row(rows, label);
    assert_eq!(
        completion.code.as_int(),
        native[1].parse::<i64>().unwrap(),
        "{label}"
    );
    assert_eq!(
        crate::fixture::bytes(vm, &completion.result),
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
                "../../../../tcl-registry/tests/data/native_apply_original_argv/tcl8.5/stdout.tsv"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../tcl-registry/tests/data/native_apply_original_argv/tcl8.6/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../tcl-registry/tests/data/native_apply_original_argv/tcl9.0/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../tcl-registry/tests/data/native_apply_original_argv/tcl9.1/stdout.tsv"
            ),
        ),
        (
            "jimtcl",
            include_str!(
                "../../../../tcl-registry/tests/data/native_apply_original_argv/jim/stdout.tsv"
            ),
        ),
    ] {
        for input in rows.lines().filter(|line| line.starts_with("INPUT_")) {
            let fields: Vec<_> = input.split('|').collect();
            let case = fields[0].strip_prefix("INPUT_").unwrap();
            let argument = Value::new_native_string_bytes(unhex(fields[2]));
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
            let lambda = Value::new_native_string_bytes(b"{x} {return $x}".as_slice());
            let completion = call(&mut vm, b"apply", &[lambda, argument]);
            assert_row(&vm, &completion, rows, &format!("APPLY_{case}"));
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
                "../../../../tcl-registry/tests/data/native_apply_original_argv/tcl8.6/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../tcl-registry/tests/data/native_apply_original_argv/tcl9.0/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../tcl-registry/tests/data/native_apply_original_argv/tcl9.1/stdout.tsv"
            ),
        ),
    ] {
        for input in rows.lines().filter(|line| line.starts_with("INPUT_")) {
            let fields: Vec<_> = input.split('|').collect();
            let case = fields[0].strip_prefix("INPUT_").unwrap();
            let argument = Value::new_native_string_bytes(unhex(fields[2]));
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
            let mut args = [b"c".as_slice(), b"apply", b"{x} {yield READY; return $x}"]
                .map(Value::new_native_string_bytes)
                .to_vec();
            args.push(argument);
            let start = call(&mut vm, b"coroutine", &args);
            assert_row(&vm, &start, rows, &format!("COROUTINE_START_{case}"));
            let finish = call(&mut vm, b"c", &[]);
            assert_row(&vm, &finish, rows, &format!("COROUTINE_RETURN_{case}"));
            comparisons += 1;
        }
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let shadow = vm
            .eval_source("proc apply args {return SHADOW}; coroutine c apply {} VALUE")
            .unwrap();
        assert_row(&vm, &shadow, rows, "SHADOWED_APPLY_COROUTINE");
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
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl8.6"));
        let pending = crate::fixture::eval(
            &mut vm,
            &["coroutine c apply {{x} {", body, "}} VALUE"].concat(),
        );
        assert!(pending.code.is_ok(), "parked={pending:?}");

        let public = crate::fixture::eval(&mut vm, "info commands ::tcl::apply::lambda*");
        assert!(public.code.is_ok());
        assert!(public.result.as_list().unwrap().is_empty());

        let completion = crate::fixture::eval(&mut vm, finish);
        assert_eq!(completion.code.as_int(), code, "finish={completion:?}");
        let left = crate::fixture::eval(&mut vm, "info commands c");
        assert!(left.result.as_list().unwrap().is_empty());
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
            "../../../../tcl-registry/tests/data/native_lambda_object_getter/tcl8.5/stdout.tsv"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../tcl-registry/tests/data/native_lambda_object_getter/tcl8.6/stdout.tsv"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../tcl-registry/tests/data/native_lambda_object_getter/tcl9.0/stdout.tsv"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../tcl-registry/tests/data/native_lambda_object_getter/tcl9.1/stdout.tsv"
        ),
    ),
    (
        "jimtcl",
        include_str!(
            "../../../../tcl-registry/tests/data/native_lambda_object_getter/jim/stdout.tsv"
        ),
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
                let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
                let setup = vm.eval_source("namespace eval ns {}; namespace eval :ns {}; namespace eval \\u00e9 {}; namespace eval :\\u00e9 {}").unwrap();
                assert_eq!(setup.code, tcl_runtime_api::Code::Ok);
                let namespace = if byte_array {
                    Value::byte_array(original)
                } else {
                    Value::new_native_string_bytes(original)
                };
                let protocol = crate::fixture::strings(&vm);
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
                    crate::fixture::bytes(&vm, &completion.result),
                    unhex(native_apply[2]),
                    "{engine}/{apply_label}"
                );
                assert_eq!(
                    crate::fixture::bytes(&vm, &namespace),
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
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("jimtcl"));
        let setup = crate::fixture::eval(
            &mut vm,
            "namespace eval Caller {proc launch {lambda} {apply $lambda}}",
        );
        assert_eq!(setup.code, tcl_runtime_api::Code::Ok);
        let protocol = crate::fixture::strings(&vm);
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
            "namespace={original:?}"
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            [b"::".as_slice(), original].concat(),
        );
        assert_eq!(crate::fixture::bytes(&vm, &namespace), original);
        assert_eq!(
            crate::fixture::run(&mut vm, "namespace", &["current"]).1,
            b"::"
        );
    }
}

const MISSING_LAMBDA_ROWS: [(&str, &str); 4] = [
    (
        "tcl8.5",
        include_str!(
            "../../../../tcl-registry/tests/data/native_lambda_missing_namespace_report/tcl8.5/stdout.tsv"
        ),
    ),
    (
        "tcl8.6",
        include_str!(
            "../../../../tcl-registry/tests/data/native_lambda_missing_namespace_report/tcl8.6/stdout.tsv"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../tcl-registry/tests/data/native_lambda_missing_namespace_report/tcl9.0/stdout.tsv"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../tcl-registry/tests/data/native_lambda_missing_namespace_report/tcl9.1/stdout.tsv"
        ),
    ),
];

#[test]
fn original_lambda_missing_namespaces_match_native_result_and_error_code() {
    let mut comparisons = 0;
    for (engine, rows) in MISSING_LAMBDA_ROWS {
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
                let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
                let namespace = if byte_array {
                    Value::byte_array(original)
                } else {
                    Value::new_native_string_bytes(original)
                };
                let protocol = crate::fixture::strings(&vm);
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
                    crate::fixture::bytes(&vm, &completion.result),
                    unhex(apply[3]),
                    "{engine}/{label}"
                );
                if completion.code == tcl_runtime_api::Code::Error {
                    let tuple =
                        crate::fixture::call(&mut vm, "set", &[Value::string("::errorCode")])
                            .result;
                    assert_eq!(
                        crate::fixture::bytes(&vm, &tuple),
                        unhex(oracle("ERROR_CODE")[3]),
                        "{engine}/{label}/errorCode"
                    );
                }
                assert_eq!(
                    crate::fixture::bytes(&vm, &namespace),
                    unhex(oracle("GETTER")[3]),
                    "{engine}/{label}/getter"
                );
                comparisons += 1;
            }
        }
    }
    assert_eq!(comparisons, 72);
}

#[test]
fn jim_lambda_preflight_leaves_unentered_namespace_objects_stringless() {
    let rows = include_str!(
        "../../../../tcl-registry/tests/data/native_jim_lambda_namespace_preflight/jim/stdout.tsv"
    );
    for (label, body, required) in [
        ("EMPTY_BODY", "", false),
        ("RETURN_BODY", "return BODY", false),
        ("NAMESPACE_BODY", "namespace current", false),
        ("EMPTY_REQUIRED", "", true),
        ("NONEMPTY_REQUIRED", "return BODY", true),
    ] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("jimtcl"));
        let protocol = crate::fixture::strings(&vm);
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
        let completion = call(&mut vm, b"apply", &[lambda]);
        let result = fields("RESULT");
        assert_eq!(
            completion.code.as_int(),
            result[2].parse::<i64>().unwrap(),
            "{label}/code"
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            unhex(result[3]),
            "{label}/result"
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &namespace),
            unhex(fields("GETTER")[3]),
            "{label}/getter"
        );
        assert_eq!(
            crate::fixture::run(&mut vm, "namespace", &["current"]).1,
            b"::"
        );
    }
}

#[test]
fn jim_lambda_unreached_command_lookup_keeps_original_namespace_unmaterialized() {
    let rows = include_str!(
        "../../../../tcl-registry/tests/data/native_jim_lambda_deferred_context/jim/stdout.tsv"
    );
    for (label, source) in [("COMMENT_BODY", "# only a comment"), ("INVALID_BODY", "{")] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("jimtcl"));
        let protocol = crate::fixture::strings(&vm);
        let namespace = Value::int(42);
        let body = Value::new_native_string_bytes(source.as_bytes());
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
            completion.code.as_int(),
            expected("RESULT")[2].parse::<i64>().unwrap(),
            "{label}/code"
        );
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            unhex(expected("RESULT")[3]),
            "{label}/result"
        );
        assert_eq!(
            crate::fixture::run(&mut vm, "namespace", &["current"]).1,
            b"::"
        );
    }
}

#[test]
fn jim_lambda_temporary_publication_keeps_the_original_execution_namespace_separate() {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("jimtcl"));
    let protocol = crate::fixture::strings(&vm);
    let lambda = Value::native_list_constructor(
        vec![
            Value::native_list_constructor(Vec::new(), protocol),
            Value::string("return BODY"),
            Value::int(42),
        ],
        protocol,
    );
    let before = crate::fixture::run(&mut vm, "info", &["commands"]);
    let completion = call(&mut vm, b"apply", &[lambda]);
    assert_eq!(crate::fixture::bytes(&vm, &completion.result), b"BODY");
    assert_eq!(crate::fixture::run(&mut vm, "info", &["commands"]), before);
    assert_eq!(
        crate::fixture::run(&mut vm, "namespace", &["current"]).1,
        b"::"
    );
}
