// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{Code, Vm};
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_syntax::value::ValueOps;

fn jim() -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    vm
}

fn unhex(input: &str) -> Vec<u8> {
    input
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn original_jim_dictionary_library_bodies_and_formals_are_real_procedures() {
    let mut vm = jim();
    let bodies: &[(&str, &[u8], &[u8])] = &[
        (
            "update",
            include_bytes!("../../tests/data/native_jim_dictionary/update-body.bin"),
            include_bytes!("../../tests/data/native_jim_dictionary/update-args.bin"),
        ),
        (
            "replace",
            include_bytes!("../../tests/data/native_jim_dictionary/replace-body.bin"),
            include_bytes!("../../tests/data/native_jim_dictionary/replace-args.bin"),
        ),
        (
            "lappend",
            include_bytes!("../../tests/data/native_jim_dictionary/lappend-body.bin"),
            include_bytes!("../../tests/data/native_jim_dictionary/lappend-args.bin"),
        ),
        (
            "append",
            include_bytes!("../../tests/data/native_jim_dictionary/append-body.bin"),
            include_bytes!("../../tests/data/native_jim_dictionary/append-args.bin"),
        ),
        (
            "incr",
            include_bytes!("../../tests/data/native_jim_dictionary/incr-body.bin"),
            include_bytes!("../../tests/data/native_jim_dictionary/incr-args.bin"),
        ),
        (
            "remove",
            include_bytes!("../../tests/data/native_jim_dictionary/remove-body.bin"),
            include_bytes!("../../tests/data/native_jim_dictionary/remove-args.bin"),
        ),
        (
            "for",
            include_bytes!("../../tests/data/native_jim_dictionary/for-body.bin"),
            include_bytes!("../../tests/data/native_jim_dictionary/for-args.bin"),
        ),
    ];
    assert_eq!(
        tcl_registry::dictionary_scope::stock_scripted_wrappers(vm.native_invocation_dialect())
            .len(),
        bodies.len()
    );
    for (name, expected_body, expected_args) in bodies {
        for (operation, expected) in [("body", *expected_body), ("args", *expected_args)] {
            let completion = vm
                .try_eval_source(&format!("info {operation} {{dict {name}}}"))
                .unwrap();
            assert_eq!(completion.code, Code::Ok, "{name}/{operation}");
            let actual = vm.native_string_bytes(&completion.result).unwrap();
            assert_eq!(actual.as_ref(), expected, "{name}/{operation}");
        }
    }
}

#[test]
fn native_jim_scripted_dictionary_completion_and_forwarding_observations() {
    let rows = include_str!("../../tests/data/native_jim_dictionary/observations.tsv");
    let mut compared = 0;
    for row in rows.lines() {
        let fields: Vec<_> = row.split('\t').collect();
        let [name, code, result, source] = fields.as_slice() else {
            panic!("native observation columns")
        };
        let mut vm = jim();
        let completion = vm
            .try_eval_source_bytes(&unhex(source))
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(
            completion.code.as_int(),
            code.parse::<i64>().unwrap(),
            "{name}"
        );
        let actual = vm.native_string_bytes(&completion.result).unwrap();
        assert_eq!(actual.as_ref(), unhex(result), "{name}");
        compared += 1;
    }
    assert_eq!(compared, 13);
}

#[test]
fn repinning_keeps_replacement_and_deletion_of_actual_library_generations() {
    let mut vm = jim();
    let replaced = vm
        .try_eval_source("proc {dict update} {args} {return REPLACED}")
        .unwrap();
    assert_eq!(replaced.code, Code::Ok);
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    vm.set_dialect_profile(profile);
    let result = vm.try_eval_source("dict update d k x {}").unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(
        vm.native_string_bytes(&result.result).unwrap().as_ref(),
        b"REPLACED"
    );
    assert_eq!(
        vm.try_eval_source("rename {dict update} {}").unwrap().code,
        Code::Ok
    );
    vm.set_dialect_profile(profile);
    let result = vm.try_eval_source("dict update d k x {}").unwrap();
    assert_eq!(result.code, Code::Error);
    assert_eq!(
        vm.native_string_bytes(&result.result).unwrap().as_ref(),
        b"invalid command name \"dict update\""
    );
}
