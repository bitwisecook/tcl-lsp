// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native control and expression compiler windows.
use super::*;
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::value::ValueOps;

fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn profile(version: &str) -> &'static tcl_dialect::DialectProfile {
    tcl_dialect::DialectProfile::find(match version {
        "8.4.20" => "tcl8.4",
        "8.5.19" => "tcl8.5",
        "8.6.18" => "tcl8.6",
        "9.0.4" => "tcl9.0",
        "9.1.0" => "tcl9.1",
        _ => panic!("native version"),
    })
    .unwrap()
}
fn interpreter(version: &str) -> Vm {
    let profile = profile(version);
    crate::native_fixture::interpreter(profile)
}
fn define(vm: &mut Vm, bytes: &[u8]) -> (Value, Vec<Value>) {
    let head = Value::new_native_string_bytes(b"proc".as_slice());
    let arguments = vec![
        Value::new_native_string_bytes(b"p".as_slice()),
        Value::new_native_string_bytes(b"x".as_slice()),
        Value::new_native_string_bytes(bytes),
    ];
    assert_eq!(
        vm.invoke_host_original_object_vector(&head, &arguments)
            .code,
        Code::Ok
    );
    (head, arguments)
}
fn run_outputs(table: &str, expected: usize) {
    let mut compared = 0;
    for row in table.lines() {
        let f = row.split('\t').collect::<Vec<_>>();
        let mut vm = interpreter(f[0]);
        let _definition = define(&mut vm, &decode(f[2]));
        if f[0] != "8.4.20" {
            let Some(Command::Proc(command)) = vm.lookup_command("p") else {
                panic!("original procedure");
            };
            // The original CLI controls invoke disassemble before the call.
            // Both successful and rejected preparation leave their real visits.
            let _prepared = vm.ensure_proc_traced(command);
        }
        let head = Value::new_native_string_bytes(b"p".as_slice());
        let args = [Value::new_native_string_bytes(f[5].as_bytes())];
        let completion = vm.invoke_host_original_object_vector(&head, &args);
        assert_eq!(
            completion.code,
            Code::from_int(f[3].parse().unwrap()),
            "{}/{}: {completion:?}",
            f[0],
            f[1]
        );
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            decode(f[4]),
            "{}/{}",
            f[0],
            f[1]
        );
        compared += 1;
    }
    assert_eq!(compared, expected);
}
#[test]
fn registered_control_instructions_match_75_native_controls() {
    run_outputs(
        include_str!("../../../tcl-registry/tests/data/native_control_expression/control75.tsv"),
        75,
    );
}
#[test]
fn compound_expression_programs_match_40_native_controls() {
    run_outputs(
        include_str!("../../../tcl-registry/tests/data/native_control_expression/compound40.tsv"),
        40,
    );
}
fn storage(value: &Value, version: &str) -> String {
    use tcl_syntax::scalar_getter::NativeScalarCache;
    let original = value.native_object_snapshot();
    let class = match &original.cache {
        Cache::None => "none",
        Cache::String { .. } => "string",
        Cache::Numeric(NativeScalarCache::Tcl84Long(_)) => "int",
        Cache::Numeric(NativeScalarCache::Number(tcl_syntax::number::Number::Int(_))) => {
            if version == "8.4.20" {
                "wideInt"
            } else {
                "int"
            }
        }
        Cache::Numeric(NativeScalarCache::Number(tcl_syntax::number::Number::Double(_))) => {
            "double"
        }
        Cache::WordBoolean { .. } => "boolean",
        Cache::CommandName { .. } => "cmdName",
        Cache::Dictionary { .. } => "dict",
        _ => panic!(
            "unhandled original primary: {:?}; descriptor {}; release {version}; resident {:?}",
            original.cache,
            value.native_object_type_name(),
            original.resident
        ),
    };
    let mut result = format!(
        "{class},{},{}",
        usize::from(original.resident.is_some()),
        value.native_object_reference_count()
    );
    if let Some(bytes) = original.resident {
        result.push(',');
        for byte in bytes.iter() {
            use std::fmt::Write;
            write!(result, "{byte:02x}").unwrap();
        }
    }
    result
}
fn run_expression_storage(table: &str, expected_count: usize) {
    let mut compared = 0;
    for row in table.lines() {
        let f = row.split('\t').collect::<Vec<_>>();
        let mut vm = interpreter(f[0]);
        let _definition = define(&mut vm, &decode(f[2]));
        assert_eq!(f[3], "0", "native definition");
        let head = Value::new_native_string_bytes(b"p".as_slice());
        let args = [Value::new_native_string_bytes(b"3".as_slice())];
        let completion = vm.invoke_host_original_object_vector(&head, &args);
        let code = completion.code;
        drop(completion);
        assert_eq!(
            code,
            Code::from_int(f[4].parse().unwrap()),
            "{}/{}",
            f[0],
            f[1]
        );
        let observed = vm
            .with_native_interp_result(|result| storage(result, f[0]))
            .unwrap();
        let Some(Command::Proc(command)) = vm.lookup_command("p") else {
            panic!("original procedure");
        };
        let declaration = command.declaration();
        let cache = declaration
            .body_src
            .native_bytecode_cache()
            .expect("original Bytecode primary");
        let pool = cache
            .unit
            .literal_pool
            .as_ref()
            .expect("actual literal array");
        assert_eq!(
            observed,
            f[5],
            "{}/{} original result before GetString; actual profile {:?}; arithmetic {:?}; scalar getter {:?}; retained originals {:?}; instructions {:?}",
            f[0],
            f[1],
            vm.actual_native_execution_profile().name,
            vm.numeric_context().arithmetic(),
            vm.numeric_context().native_scalar_getter_protocol(),
            (0..cache.unit.asm.literals.len())
                .map(|index| { pool.with_original(index, |original| storage(original, f[0])) })
                .collect::<Vec<_>>(),
            cache
                .unit
                .asm
                .instructions
                .iter()
                .map(|instruction| instruction.op)
                .collect::<Vec<_>>()
        );
        let expected = f[6].split('|').collect::<Vec<_>>();
        assert_eq!(
            cache.unit.asm.literals.len(),
            expected.len(),
            "{}/{} literal count; original pools {:?}; instructions {:?}",
            f[0],
            f[1],
            (0..cache.unit.asm.literals.len())
                .map(|index| pool.with_original(index, |original| storage(original, f[0])))
                .collect::<Vec<_>>(),
            cache
                .unit
                .asm
                .instructions
                .iter()
                .map(|instruction| instruction.op)
                .collect::<Vec<_>>()
        );
        for (index, expected) in expected.into_iter().enumerate() {
            assert_eq!(
                pool.with_original(index, |original| storage(original, f[0]))
                    .unwrap(),
                expected,
                "{}/{} original literal {index} before GetString",
                f[0],
                f[1]
            );
        }
        compared += 1;
    }
    assert_eq!(compared, expected_count);
}

#[test]
fn compiled_expression_storage_matches_40_original_native_windows() {
    run_expression_storage(
        include_str!("../../../tcl-registry/tests/data/native_control_expression/storage40.tsv"),
        40,
    );
}

const SYNTAX_CONTEXT_FIXTURES: [(&str, &str); 3] = [
    (
        "8.6.18",
        include_str!(
            "../../../tcl-registry/tests/data/native_control_expression/syntax-context-8.6.18.tsv"
        ),
    ),
    (
        "9.0.4",
        include_str!(
            "../../../tcl-registry/tests/data/native_control_expression/syntax-context-9.0.4.tsv"
        ),
    ),
    (
        "9.1.0",
        include_str!(
            "../../../tcl-registry/tests/data/native_control_expression/syntax-context-9.1.0.tsv"
        ),
    ),
];

#[test]
fn compiled_syntax_context_retains_same_native_message_and_options() {
    for (version, table) in SYNTAX_CONTEXT_FIXTURES {
        let mut vm = interpreter(version);
        let _definition = define(&mut vm, b"expr {$x ? (1/0) : (1+2)}");
        let head = Value::new_native_string_bytes(b"p".as_slice());
        let arguments = [Value::new_native_string_bytes(b"3".as_slice())];
        let completion = vm.invoke_host_original_object_vector(&head, &arguments);
        assert_eq!(completion.code, Code::Error);
        drop(completion);
        let Some(Command::Proc(command)) = vm.lookup_command("p") else {
            panic!("original procedure");
        };
        let declaration = command.declaration();
        let cache = declaration
            .body_src
            .native_bytecode_cache()
            .expect("original Bytecode");
        let pool = cache
            .unit
            .literal_pool
            .as_ref()
            .expect("original literal array");
        let context = vm
            .native_errors
            .error_stack
            .original_inner_context()
            .expect("interpreter innerContext");
        let (members, _) = context
            .cached_list_representation()
            .expect("actual innerContext List");
        assert_eq!(members.len(), 3);
        if version == "9.1.0" {
            let same_message = members[2]
                .with_cached_dictionary_member(b"-errorinfo", |value| {
                    value.is_some_and(|value| value.is_same_object(&members[1]))
                })
                .expect("original compiled options Dictionary");
            assert!(
                same_message,
                "C91 options retain the original Syntax message"
            );
        }
        for line in table.lines().filter(|line| line.starts_with("CTX\t5\t")) {
            let fields = line.split('\t').collect::<Vec<_>>();
            let index = fields[2].parse::<usize>().unwrap();
            let original = &members[index];
            if index == 0 {
                let name = original
                    .native_instruction_name()
                    .expect("actual instname primary");
                let opcodes = table
                    .lines()
                    .find(|line| line.starts_with("OPCODES\t"))
                    .unwrap()
                    .split('\t')
                    .collect::<Vec<_>>();
                assert_eq!(name.opcode(), opcodes[1].parse::<u8>().unwrap());
                assert_eq!(original.native_object_type_name(), "instname");
                assert!(original.resident_string_bytes().is_none());
                assert_eq!(original.native_object_reference_count(), 1);
                assert!(!original.native_primary_has_free_hook());
                let duplicate =
                    original.duplicate_native_object_in(NativeStringProtocol::C(name.version()));
                assert_eq!(duplicate.native_instruction_name(), Some(name));
                assert!(duplicate.resident_string_bytes().is_none());
                assert_eq!(
                    duplicate
                        .native_string_bytes(NativeStringProtocol::C(name.version()))
                        .unwrap()
                        .as_ref(),
                    b"syntax"
                );
                assert!(original.resident_string_bytes().is_none());
                assert!(
                    original
                        .native_string_bytes(NativeStringProtocol::Jim084)
                        .is_err()
                );
            } else {
                assert_eq!(storage(original, version), fields[5], "{version}: {line}");
                let slot = fields[3].parse::<usize>().unwrap();
                assert!(
                    pool.with_original(slot, |literal| literal.is_same_object(original))
                        .unwrap()
                );
                let is_result = vm
                    .with_native_interp_result(|result| result.is_same_object(original))
                    .unwrap();
                assert_eq!(is_result, fields[4] == "1");
            }
        }
    }
}

#[test]
fn folded_logical_literal_ownership_matches_eight_native_collisions() {
    run_expression_storage(
        include_str!(
            "../../../tcl-registry/tests/data/native_control_expression/folded-boolean8.tsv"
        ),
        8,
    );
}

const RETURN_CONTEXT_FIXTURES: [(&str, &str); 3] = [
    (
        "8.6.18",
        include_str!(
            "../../../tcl-registry/tests/data/native_control_expression/return-context-8.6.18.tsv"
        ),
    ),
    (
        "9.0.4",
        include_str!(
            "../../../tcl-registry/tests/data/native_control_expression/return-context-9.0.4.tsv"
        ),
    ),
    (
        "9.1.0",
        include_str!(
            "../../../tcl-registry/tests/data/native_control_expression/return-context-9.1.0.tsv"
        ),
    ),
];

#[test]
fn compiled_return_context_retains_original_operands_without_annotation() {
    let sources = [
        b"return -level 0 -code error BODY".as_slice(),
        b"return -level 0 -code error [set y BODY]",
        b"error BODY",
    ];
    for (version, table) in RETURN_CONTEXT_FIXTURES {
        for (case, source) in sources.iter().enumerate() {
            let mut vm = interpreter(version);
            let _definition = define(&mut vm, source);
            let head = Value::new_native_string_bytes(b"p".as_slice());
            let arguments = [Value::new_native_string_bytes(b"3".as_slice())];
            let completion = vm.invoke_host_original_object_vector(&head, &arguments);
            assert_eq!(completion.code, Code::Error, "{version}/{case}");
            drop(completion);
            let Some(Command::Proc(command)) = vm.lookup_command("p") else {
                panic!("original procedure");
            };
            let declaration = command.declaration();
            let cache = declaration
                .body_src
                .native_bytecode_cache()
                .expect("original Bytecode");
            let pool = cache
                .unit
                .literal_pool
                .as_ref()
                .expect("original literal array");
            if case < 2 {
                let instruction = cache
                    .unit
                    .asm
                    .instructions
                    .iter()
                    .find(|instruction| instruction.op == tcl_bytecode::Op::RETURN_IMM)
                    .expect("actual RETURN_IMM");
                assert!(
                    instruction.error_stack_context.is_none(),
                    "original Return has no Error annotation"
                );
            }
            let native = table
                .lines()
                .find(|line| line.starts_with(&format!("CASE\t{case}\t")))
                .unwrap()
                .split('\t')
                .collect::<Vec<_>>();
            assert_eq!(
                vm.with_native_interp_result(|result| storage(result, version))
                    .unwrap(),
                native[4],
                "{version}/{case}"
            );
            let context = vm
                .native_errors
                .error_stack
                .original_inner_context()
                .expect("original innerContext");
            let (members, _) = context.cached_list_representation().unwrap();
            assert_eq!(members.len(), 3);
            for line in table
                .lines()
                .filter(|line| line.starts_with(&format!("CTX\t{case}\t")))
            {
                let fields = line.split('\t').collect::<Vec<_>>();
                let index = fields[2].parse::<usize>().unwrap();
                let original = &members[index];
                if index == 0 {
                    assert_eq!(original.native_object_type_name(), "instname");
                    assert!(original.resident_string_bytes().is_none());
                    assert_eq!(original.native_object_reference_count(), 1);
                    let name = original.native_instruction_name().unwrap();
                    let native = table
                        .lines()
                        .find(|line| line.starts_with(&format!("INST\t{case}\t")))
                        .unwrap()
                        .split('\t')
                        .collect::<Vec<_>>();
                    assert_eq!(name.opcode(), native[2].parse::<u8>().unwrap());
                    assert_eq!(name.string_bytes(), decode(native[3]));
                } else {
                    assert_eq!(
                        storage(original, version),
                        fields[5],
                        "{version}/{case}: {line}"
                    );
                    assert!(
                        pool.with_original(fields[3].parse().unwrap(), |literal| literal
                            .is_same_object(original))
                            .unwrap()
                    );
                    assert_eq!(
                        vm.with_native_interp_result(|result| result.is_same_object(original))
                            .unwrap(),
                        fields[4] == "1"
                    );
                }
            }
        }
    }
}

#[test]
fn c84_logical_storage_and_short_circuit_match_eight_native_controls() {
    run_expression_storage(
        include_str!("../../../tcl-registry/tests/data/native_control_expression/logical84.tsv"),
        8,
    );
}

#[test]
fn compiled_catch_store_order_matches_four_native_alias_controls() {
    let mut compared = 0;
    for row in include_str!("../../../tcl-registry/tests/data/native_control_expression/catch4.tsv")
        .lines()
    {
        let fields = row.split('\t').collect::<Vec<_>>();
        let mut vm = interpreter(fields[0]);
        let head = Value::new_native_string_bytes(b"proc".as_slice());
        let definition = [
            Value::new_native_string_bytes(b"p".as_slice()),
            Value::new_native_string_bytes(b"".as_slice()),
            Value::new_native_string_bytes(decode(fields[1])),
        ];
        assert_eq!(
            vm.invoke_host_original_object_vector(&head, &definition)
                .code,
            Code::Ok
        );
        let call_head = Value::new_native_string_bytes(b"p".as_slice());
        let completion = vm.invoke_host_original_object_vector(&call_head, &[]);
        assert_eq!(
            completion.code,
            Code::from_int(fields[2].parse().unwrap()),
            "{}: {completion:?}",
            fields[0]
        );
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            decode(fields[3]),
            "{}",
            fields[0]
        );
        compared += 1;
    }
    assert_eq!(compared, 4);
}

#[test]
fn catch_publication_and_store_order_match_six_native_engines() {
    let mut compared = 0;
    for row in include_str!(
        "../../../tcl-registry/tests/data/native_control_expression/catch-publication6.tsv"
    )
    .lines()
    {
        let fields = row.split('\t').collect::<Vec<_>>();
        let selected = if fields[0] == "jim" {
            tcl_registry::model::ingress::resolve_environment("jim").unit_profile()
        } else {
            profile(fields[0])
        };
        let mut vm = crate::native_fixture::interpreter(selected);
        let completion = vm.try_eval_source_bytes(&decode(fields[1])).unwrap();
        assert_eq!(
            completion.code,
            Code::from_int(fields[2].parse().unwrap()),
            "{}: {completion:?}",
            fields[0]
        );
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            decode(fields[3]),
            "{} original publication callbacks and variable stores",
            fields[0]
        );
        compared += 1;
    }
    assert_eq!(compared, 6);
}
