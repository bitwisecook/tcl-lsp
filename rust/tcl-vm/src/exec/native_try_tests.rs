// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native try compiler vectors executed through authenticated bytecode.

use super::*;

mod inputs {
    include!("../../../tcl-registry/tests/data/native_each_try_compilation/cases.rs");
}

fn bytes(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|digits| {
            let text = std::str::from_utf8(digits).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}

#[test]
fn original_try_vectors_keep_native_handler_bindings_and_finally_completion() {
    let engines = [
        (
            "tcl8.4",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tcl-registry/tests/data/native_each_try_compilation/9.1.0.tsv"),
        ),
    ];
    let mut compared = 0;
    for (name, fixture) in engines {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        for row in fixture.lines().skip(20) {
            let fields: Vec<_> = row.split('\t').collect();
            let index = fields[0].parse::<usize>().unwrap();
            let (label, original) = inputs::CASES[index];
            let mut vm = Vm::with_native_core(
                Box::new(Vec::<u8>::new()),
                Rc::new(crate::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            let defined = vm.invoke_command(
                "proc",
                &[
                    Value::new_native_string_bytes(b"p".as_slice()),
                    Value::empty(),
                    Value::new_native_string_bytes(original),
                ],
            );
            assert_eq!(defined.code, Code::Ok, "{name}/{label}: {defined:?}");
            let completion = vm.invoke_command("p", &[]);
            assert_eq!(
                completion.code,
                Code::from_int(fields[1].parse().unwrap()),
                "{name}/{label}: {completion:?}"
            );
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
                bytes(fields[2]),
                "{name}/{label}"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 105);
}
