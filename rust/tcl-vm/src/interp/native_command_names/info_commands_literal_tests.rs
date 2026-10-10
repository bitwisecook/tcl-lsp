// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual compiled and generic command inventory results and retained original literals.

use crate::{Code, Value};
use tcl_syntax::value::ValueOps;

include!("../../../../tcl-registry/tests/data/native_info_commands_literal_original/cases.rs");
const WINDOWS: &str = include_str!(
    "../../../../tcl-registry/tests/data/native_info_commands_literal_original/windows.tsv"
);

fn unhex(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn assert_header(original: &Value, name: &str, present: &str, bytes: &str, context: &str) {
    assert_eq!(
        original.native_object_type_name(),
        name,
        "{context}: primary before getter"
    );
    let actual = original.resident_string_bytes();
    assert_eq!(
        actual.is_some(),
        present == "1",
        "{context}: string presence before getter"
    );
    let expected = (bytes != "-").then(|| unhex(bytes));
    assert_eq!(
        actual.as_deref(),
        expected.as_deref(),
        "{context}: counted resident bytes before getter"
    );
}

#[test]
fn compiled_and_generic_info_commands_retain_one_hundred_eighty_six_original_header_windows() {
    // naming.compiler.original-info-commands-literal-resolution
    // docs/design/analysis/name-resolution-proofs/compiler-original-info-commands-literal-resolution.md
    // windows.tsv projects exact independent original receipts; this comparison
    // grants neither native compiler admission nor command-table authority.
    let mut compared = 0;
    let mut pattern_caches = 0;
    for row in WINDOWS.lines().skip(1) {
        let fields: Vec<_> = row.split('\t').collect();
        let case = ORIGINAL_INFO_COMMANDS_CASES[fields[1].parse::<usize>().unwrap()];
        let context = format!("{}: {}", fields[0], case.0);
        // Oracle provider labels use the shared ingress aliases, including
        // `jim`; they are not the interned profile catalogue names.
        let profile = tcl_registry::model::ingress::resolve_environment(fields[0]).unit_profile();
        // The original configured Jim provider loads nshelper.tcl. Its
        // namespace-aware inventory enters that current, literal script worker;
        // a core-and-compiler fixture supplies no library binding by itself.
        let libraries = if fields[0] == "jim" {
            &[tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceInfo][..]
        } else {
            &[][..]
        };
        let mut vm = crate::native_fixture::interpreter_with_scripted_libraries(profile, libraries);
        if fields[0] == "jim" {
            assert!(
                vm.native_invocation_dialect()
                    .native_name_protocol()
                    .is_some_and(|protocol| protocol.is_jim084()),
                "{context}: actual selected Jim recipe"
            );
        }
        let prelude = vm.try_eval_source_bytes(case.1).unwrap();
        assert_eq!(
            prelude.code,
            Code::Ok,
            "{context}: genuine independent prelude"
        );
        let definition_head = Value::new_native_string_bytes(b"proc".as_slice());
        let definition = [case.3, case.4, case.2].map(Value::new_native_string_bytes);
        let defined = vm.invoke_host_original_object_vector(&definition_head, &definition);
        assert!(
            vm.execution_refusal.is_none(),
            "{context}: definition host {:?}",
            vm.execution_refusal
        );
        assert_eq!(
            defined.code,
            Code::Ok,
            "{context}: genuine original procedure binding"
        );
        let head = Value::new_native_string_bytes(case.3);
        let arguments: Vec<_> = case
            .5
            .into_iter()
            .map(Value::new_native_string_bytes)
            .collect();
        let completion = vm.invoke_host_original_object_vector(&head, &arguments);
        assert!(
            vm.execution_refusal.is_none(),
            "{context}: original invocation host {:?}",
            vm.execution_refusal
        );
        if std::env::var_os("TCL_VM_TRACE_INFO_COMMANDS_LITERAL").is_some()
            && completion.code.as_int().to_string() != fields[2]
        {
            // Diagnostic observation of this unchanged software invocation only;
            // no original-provider receipt or successful comparison is inferred.
            let resident = completion.result.resident_string_bytes();
            let checked = vm.native_name_operand_bytes(&completion.result);
            eprintln!(
                "INFO_COMMANDS_LITERAL context={context} code={} primary={} live={} resident={} checked={:?}",
                completion.code.as_int(),
                completion.result.native_object_type_name(),
                completion.result.native_object_is_live(),
                resident.is_some(),
                checked.as_deref(),
            );
        }
        assert_eq!(
            completion.code.as_int().to_string(),
            fields[2],
            "{context}: guest completion"
        );
        let original = &completion.result;
        assert_header(original, fields[4], fields[5], fields[6], &context);
        let protocol = vm
            .native_scalar_carrier_dialect()
            .native_string_protocol()
            .unwrap();
        let children = if fields[8] == "-" {
            if fields[4] == "list" {
                assert!(
                    original
                        .native_list_backing_in(protocol)
                        .unwrap()
                        .unwrap()
                        .is_empty(),
                    "{context}: original empty list"
                );
            }
            Vec::new()
        } else {
            let backing = original
                .native_list_backing_in(protocol)
                .unwrap()
                .expect("actual native result List backing");
            let children: Vec<_> = backing.iter().map(Value::downgrade_native_object).collect();
            let names: Vec<_> = fields[8].split(',').collect();
            let presence: Vec<_> = fields[9].split(',').collect();
            let bytes: Vec<_> = fields[10].split(',').collect();
            assert_eq!(
                children.len(),
                names.len(),
                "{context}: actual original child count"
            );
            for (index, child) in children.iter().enumerate() {
                let original = child.upgrade().expect("result List owns original child");
                assert_header(
                    &original,
                    names[index],
                    presence[index],
                    bytes[index],
                    &context,
                );
            }
            children
        };
        if let Some(argument) = arguments.first() {
            assert_header(argument, fields[14], fields[15], fields[16], &context);
        }
        if fields[3] == "1" {
            let procedure = vm
                .proc_def_bytes(case.3)
                .expect("genuine installed procedure");
            let artifact = procedure
                .body_src
                .native_bytecode_cache()
                .expect("genuine selected body artifact");
            assert_eq!(
                artifact.unit.asm.literals.len(),
                1,
                "{context}: native singleton object array"
            );
            let pool = artifact
                .unit
                .literal_pool
                .as_ref()
                .expect("actual original literal pool");
            pool.with_original(0, |pattern| {
                assert_header(pattern, fields[11], fields[12], fields[13], &context);
            })
            .expect("original compiled pattern");
            pattern_caches += 1;
        }
        // Physical result, child, argument and literal observations precede the
        // reached string getters, preserving the missing branch and C-version cache.
        assert_eq!(
            vm.native_string_bytes(original).unwrap().as_ref(),
            unhex(fields[7]),
            "{context}: reached result bytes"
        );
        for (child, expected) in children.iter().zip(fields[10].split(',')) {
            let original = child.upgrade().expect("result List owns original child");
            assert_eq!(
                vm.native_string_bytes(&original).unwrap().as_ref(),
                unhex(expected),
                "{context}: reached child bytes"
            );
        }
        compared += 1;
    }
    assert_eq!(compared, 186);
    assert_eq!(pattern_caches, 54);
}
