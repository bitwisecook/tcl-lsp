// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn actual(engine: &str) -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
    crate::native_fixture::interpreter(profile)
}

#[test]
fn original_procedure_colon_names_match_all_18_native_home_and_error_controls() {
    let rows = include_str!("../../../tcl-syntax/tests/data/native_procedure_name/rows.txt");
    let decode = |text: &str| {
        text.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    let mut count = 0;
    for row in rows.lines() {
        let fields: Vec<_> = row.split('\t').collect();
        let mut vm = actual(fields[0]);
        let result = vm
            .eval_source(&String::from_utf8(decode(fields[4])).unwrap())
            .unwrap_or_else(|error| panic!("{}/{}: {error:?}", fields[0], fields[1]));
        assert_eq!(
            result.code.as_int(),
            fields[2].parse::<i64>().unwrap(),
            "{}/{}",
            fields[0],
            fields[1]
        );
        assert_eq!(
            vm.native_name_operand_bytes(&result.result)
                .unwrap()
                .as_ref(),
            decode(fields[3]),
            "{}/{}",
            fields[0],
            fields[1]
        );
        count += 1;
    }
    assert_eq!(count, 18);
}

#[test]
fn catch_epilogue_publishes_status_in_the_actual_interpreter_result_owner() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = actual(engine);
        let body = vm
            .publish_native_interp_completion(ok(Value::new_native_string_bytes(b":".as_slice())))
            .unwrap();
        let status = vm.finish_catch(body, None, None, 0);
        assert_eq!(status.code, Code::Ok, "{engine}");
        assert_eq!(status.result.as_int().unwrap(), 0, "{engine}");
        vm.with_native_interp_result(|original| {
            assert!(original.is_same_object(&status.result), "{engine}");
            assert_eq!(original.as_int().unwrap(), 0, "{engine}");
            assert_eq!(original.native_object_reference_count(), 1, "{engine}");
        })
        .unwrap();
    }
}

#[test]
fn legacy_name_rejection_precedes_original_formal_default_and_body_access() {
    for engine in ["tcl8.4", "tcl8.5"] {
        let mut vm = actual(engine);
        let namespace = vm.activate_namespace_operand(b":").unwrap();
        vm.push_ns_eval_token_frame(namespace, Vec::new());
        let protocol = vm
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let member = Value::new_native_string_bytes(b"untouched".as_slice());
        let default = Value::native_list_constructor(vec![member.clone()], protocol);
        let formal = Value::native_list_constructor(
            vec![
                Value::new_native_string_bytes(b"x".as_slice()),
                default.clone(),
            ],
            protocol,
        );
        let parameters = Value::native_list_constructor(vec![formal], protocol);
        let body = Value::native_list_constructor(vec![member], protocol);
        for original in [&parameters, &default, &body] {
            assert_eq!(original.native_object_type_name(), "list", "{engine}");
            assert!(original.resident_string_bytes().is_none(), "{engine}");
        }
        let result = cmd_proc(
            &mut vm,
            &[
                Value::new_native_string_bytes(b":".as_slice()),
                parameters.native_lifetime_lease().into_value(),
                body.native_lifetime_lease().into_value(),
            ],
        );
        assert_eq!(result.code, tcl_runtime_api::Code::Error);
        for original in [&parameters, &default, &body] {
            assert_eq!(original.native_object_type_name(), "list", "{engine}");
            assert!(original.resident_string_bytes().is_none(), "{engine}");
        }
    }
}
