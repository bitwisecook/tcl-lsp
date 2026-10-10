// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{Code, Vm};
use tcl_syntax::value::ValueOps;

fn jim() -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    crate::native_fixture::interpreter_with_dictionary_library(profile)
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
    // Native proof: naming.jim.scripted-dictionary-worker-body-and-dispatch
    // docs/design/analysis/name-resolution-proofs/jim-scripted-dictionary-worker-body-and-dispatch.md
    // The fixture proves public body/formal bytes. Header equality below checks
    // this backend's retained declaration ownership, independently of that proof.
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
            if operation == "args" {
                let declaration = vm
                    .proc_def_bytes(format!("dict {name}").as_bytes())
                    .unwrap();
                let original = declaration.native_parameters.as_ref().unwrap();
                assert_eq!(
                    completion.result.native_object_identity(),
                    original.native_object_identity(),
                    "{name}: info args must return the retained original formal list"
                );
            }
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

#[test]
fn native_jim_dictionary_worker_requires_explicit_distribution_initialisation() {
    // Native proof: naming.jim.dictionary-core-versus-stdlib-bootstrap
    // docs/design/analysis/name-resolution-proofs/jim-dictionary-core-versus-stdlib-bootstrap.md
    // The actual same interpreter's invocation/caller-value fields are tested;
    // the independent enumeration field grants no worker-availability evidence.
    let profile = crate::environment::profile_for_dialect("jim");
    let mut vm = crate::native_fixture::interpreter(profile);
    let source = b"set d {first NEW};set ok BEFORE;set entered 0;set c [catch {dict update d first ok {set entered 1}} r];list [info commands {dict update}] $c $r $ok $entered $d";
    let observe = |vm: &mut Vm| {
        let completion = vm
            .try_eval_source_bytes(source)
            .expect("original native Jim source");
        assert_eq!(completion.code, Code::Ok);
        let result = vm.native_name_operand_bytes(&completion.result).unwrap();
        let fields = tcl_syntax::list::split_native_list_bytes(
            &result,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap();
        assert_eq!(fields.len(), 6);
        fields[1..]
            .iter()
            .map(|field| field.to_vec())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        observe(&mut vm),
        [
            b"1".to_vec(),
            b"invalid command name \"dict update\"".to_vec(),
            b"BEFORE".to_vec(),
            b"0".to_vec(),
            b"first NEW".to_vec(),
        ]
    );
    vm.install_scripted_library(
        tcl_registry::native_scripted_distribution::NativeScriptedLibrary::Dictionary,
    );
    assert_eq!(
        observe(&mut vm),
        [
            b"0".to_vec(),
            b"1".to_vec(),
            b"NEW".to_vec(),
            b"1".to_vec(),
            b"first NEW".to_vec(),
        ]
    );
}
