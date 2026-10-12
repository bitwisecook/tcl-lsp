// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original substitution values and their independent template cache.
use super::*;
use std::rc::Rc;
use tcl_runtime_api::native_substitution::NativeSubstitutionFlags;

fn decode(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn actual(engine: &str) -> Vm {
    crate::native_fixture::interpreter(
        tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
    )
}
fn call(vm: &mut Vm, original: &Value, flags: u8) -> Completion<Value> {
    let mut arguments = Vec::new();
    for (bit, spelling) in [
        (4, b"-nobackslashes".as_slice()),
        (1, b"-nocommands"),
        (2, b"-novariables"),
    ] {
        if flags & bit == 0 {
            arguments.push(Value::new_native_string_bytes(spelling));
        }
    }
    arguments.push(original.clone());
    vm.invoke_command("subst", &arguments)
}
#[test]
fn original_compiled_substitution_matches_counted_native_flags_and_completions() {
    // Native proof: naming.substitution.counted-template-completions
    // docs/design/analysis/name-resolution-proofs/substitution-counted-template-completions.md
    // Native proof: naming.substitution.original-cache-fields
    // docs/design/analysis/name-resolution-proofs/substitution-original-cache-fields.md
    let inputs: Vec<_> =
        include_str!("../../../tcl-registry/tests/data/native_substitution_owner/inputs.json")
            .lines()
            .filter_map(|line| {
                line.trim()
                    .trim_end_matches(',')
                    .strip_prefix('"')?
                    .strip_suffix('"')
            })
            .collect();
    assert_eq!(inputs.len(), 10);
    let mut compared = 0;
    for (engine, rows) in [
        (
            "tcl8.6",
            include_str!(
                "../../../tcl-registry/tests/data/native_substitution_owner/v6/8.6.18/stdout.tsv"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../tcl-registry/tests/data/native_substitution_owner/v6/9.0.4/stdout.tsv"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../tcl-registry/tests/data/native_substitution_owner/v6/9.1.0/stdout.tsv"
            ),
        ),
    ] {
        for (source, input) in inputs.iter().enumerate() {
            let mut vm = actual(engine);
            let protocol = vm
                .actual_native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let original = Value::new_native_string_bytes(decode(input).as_slice());
            for flags in (0..=7).rev() {
                for label in ["ROOT", "ROOT_REPEAT"] {
                    for (name, value) in [
                        (b"x".as_slice(), b"X".as_slice()),
                        (b"y", b"Y"),
                        (b"a(k)", b"K"),
                        (b"k", b"k"),
                    ] {
                        assert_eq!(
                            vm.invoke_command(
                                "set",
                                &[
                                    Value::new_native_string_bytes(name),
                                    Value::new_native_string_bytes(value)
                                ]
                            )
                            .code,
                            Code::Ok
                        );
                    }
                    let fields: Vec<_> = rows
                        .lines()
                        .find(|row| row.starts_with(&format!("R|{label}|{source}|{flags}|")))
                        .unwrap()
                        .split('|')
                        .collect();
                    let result = call(&mut vm, &original, flags);
                    assert_eq!(
                        result.code.as_int(),
                        fields[4].parse::<i64>().unwrap(),
                        "{engine}/{source}/{flags}/{label}: {result:?}"
                    );
                    assert_eq!(
                        original.native_object_type_name(),
                        fields[5],
                        "{engine}/{source}/{flags}/{label}"
                    );
                    assert_eq!(
                        usize::from(original.resident_string_bytes().is_some()),
                        fields[6].parse::<usize>().unwrap()
                    );
                    assert_eq!(
                        result.result.native_object_type_name(),
                        fields[7],
                        "{engine}/{source}/{flags}/{label}"
                    );
                    assert_eq!(
                        usize::from(result.result.resident_string_bytes().is_some()),
                        fields[8].parse::<usize>().unwrap()
                    );
                    assert_eq!(
                        result
                            .result
                            .native_string_bytes(protocol)
                            .unwrap()
                            .as_ref(),
                        decode(fields[9]),
                        "{engine}/{source}/{flags}/{label}"
                    );
                    assert!(original.native_bytecode_cache().is_none());
                    let cached = original.native_substitution_cache().unwrap();
                    assert_eq!(
                        cached.context,
                        crate::value::NativeBytecodeContext::Substitution(
                            NativeSubstitutionFlags::new(
                                flags & 4 != 0,
                                flags & 1 != 0,
                                flags & 2 != 0
                            )
                        )
                    );
                    assert!(cached.procedure.borrow().upgrade().is_none());
                    assert!(cached.substitution_layout.is_none());
                    compared += 1;
                }
            }
        }
    }
    assert_eq!(compared, 480);
}
#[test]
fn original_substitution_changes_namespace_and_borrowed_local_cache_without_proc_header() {
    // Native proof: naming.substitution.namespace-and-local-context
    // docs/design/analysis/name-resolution-proofs/substitution-namespace-and-local-context.md
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(engine);
        let original = Value::new_native_string_bytes(b"$x".as_slice());
        assert_eq!(
            vm.invoke_command("set", &[Value::string("::template"), original.clone()])
                .code,
            Code::Ok
        );
        let setup = vm.eval_source("namespace eval A {variable x A}; namespace eval B {variable x B}; set x ROOT; proc P {x} {subst $::template}; proc Q {padding x} {subst $::template}").unwrap();
        assert_eq!(setup.code, Code::Ok, "{engine}: {setup:?}");
        for (source, expected) in [
            ("namespace eval A {subst $::template}", "A"),
            ("namespace eval B {subst $::template}", "B"),
            ("subst $::template", "ROOT"),
        ] {
            let result = vm.eval_source(source).unwrap();
            assert_eq!(result.code, Code::Ok, "{engine}: {result:?}");
            assert_eq!(result.result.to_str().as_ref(), expected);
        }
        let first = vm.invoke_command("P", &[Value::string("P_VALUE")]);
        assert_eq!(first.code, Code::Ok, "{engine}: {first:?}");
        assert_eq!(first.result.to_str().as_ref(), "P_VALUE");
        let p_cache = original.native_substitution_cache().unwrap();
        assert!(p_cache.substitution_layout.is_some() && p_cache.substitution_table.is_some());
        assert!(p_cache.procedure.borrow().upgrade().is_none());
        let repeated = vm.invoke_command("P", &[Value::string("P_REPEAT")]);
        assert_eq!(repeated.code, Code::Ok);
        assert_eq!(repeated.result.to_str().as_ref(), "P_REPEAT");
        assert!(Rc::ptr_eq(
            &p_cache,
            &original.native_substitution_cache().unwrap()
        ));
        let second = vm.invoke_command("Q", &[Value::string("PAD"), Value::string("Q_VALUE")]);
        assert_eq!(second.code, Code::Ok, "{engine}: {second:?}");
        assert_eq!(second.result.to_str().as_ref(), "Q_VALUE");
        let q_cache = original.native_substitution_cache().unwrap();
        assert!(!Rc::ptr_eq(&p_cache, &q_cache));
        assert_ne!(p_cache.substitution_layout, q_cache.substitution_layout);
        let duplicate = original.duplicate_native_object_in(
            vm.actual_native_invocation_dialect()
                .native_string_protocol()
                .unwrap(),
        );
        assert_eq!(duplicate.native_object_type_name(), "none");
        assert!(duplicate.native_substitution_cache().is_none());
    }
}
