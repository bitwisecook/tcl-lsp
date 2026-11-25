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

use super::{char_find, imm_index};
use crate::interp::Vm;
use crate::value::Value;
use tcl_bytecode::INDEX_END;
use tcl_syntax::value::ValueOps;

#[test]
fn procedure_activation_uses_the_supplied_host_without_changing_source_policy() {
    let host = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
    let source = tcl_dialect::DialectProfile::irules();
    let mut vm = crate::native_fixture::core(host);
    vm.set_dialect_profile(source);
    let policy = vm.native_compiler_policy();
    assert!(vm.native_invocation_dialect().execution_point().is_none());
    assert_eq!(
        vm.actual_native_execution_profile().cache_key(),
        host.cache_key()
    );
    let body = Value::new_native_string_bytes(b"".as_slice());
    let reached = vm.early_procedure_activation_inputs(
        &[],
        tcl_dialect::ParameterGrammar::Tcl,
        &body,
        &[],
        |_| panic!("C activation does not inspect argument usage before its body"),
    );
    assert!(matches!(reached, Ok(None)));
    assert!(vm.execution_refusal.is_none());
    assert_eq!(vm.source_profile().cache_key(), source.cache_key());
    assert_eq!(vm.native_compiler_policy(), policy);
}

#[test]
fn procedure_parse_failure_uses_the_host_but_keeps_missing_geometry_closed() {
    let host = tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile();
    let source = tcl_dialect::DialectProfile::irules();
    let mut vm = crate::native_fixture::core(host);
    vm.set_dialect_profile(source);
    let policy = vm.native_compiler_policy();
    let invoked = Value::new_native_string_bytes(b"original-procedure".as_slice());
    // A diagnostic geometry fixture grants no procedure-body execution.
    let tail = tcl_runtime_api::FatalTail {
        message: "missing close-brace".into(),
        command_text: b"set x {".to_vec(),
        compilation_command_text: Some(b"set x {".to_vec()),
        line: 1,
        compile_info: None,
        error_code: None,
    };
    let reached = vm.raise_procedure_parse_failure(tail, &invoked);
    assert_eq!(reached.code, tcl_runtime_api::Code::Error);
    assert_eq!(
        reached.result.string_bytes().as_ref(),
        b"missing close-brace"
    );
    assert!(vm.execution_refusal.is_none());
    assert_eq!(vm.source_profile().cache_key(), source.cache_key());
    assert_eq!(vm.native_compiler_policy(), policy);
    let missing = tcl_runtime_api::FatalTail::message_only("missing close-brace".into());
    let reached = vm.raise_procedure_parse_failure(missing, &invoked);
    assert_eq!(reached.code, tcl_runtime_api::Code::Error);
    assert!(matches!(
        vm.execution_refusal,
        Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_))
    ));
}

#[test]
fn procedure_activation_retains_original_invocation_references_until_exit() {
    // Source contract: naming.error.original-invocation-context-capture
    // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
    // This checks VM ownership, independently of native public stack bytes.
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
    let mut vm = crate::native_fixture::interpreter(profile);
    let invoked = Value::new_native_string_bytes(b"original-head".as_slice());
    let argument = Value::new_native_string_bytes(b"original-argument".as_slice());
    let head_loan = invoked.native_lifetime_lease();
    let argument_loan = argument.native_lifetime_lease();
    let mut frame = super::Frame::new(vm.current_placeholder_unit(), true);
    frame.procedure_invocation = Some((invoked, vec![argument]));
    let (head, arguments) = frame.procedure_invocation.as_ref().unwrap();
    assert!(head.is_same_object(head_loan.value()));
    assert!(arguments[0].is_same_object(argument_loan.value()));
    assert_eq!(head.native_object_reference_count(), 1);
    assert_eq!(arguments[0].native_object_reference_count(), 1);
    assert!(head.native_object_is_live());
    assert!(arguments[0].native_object_is_live());
    drop(frame);
    assert!(!head_loan.value().native_object_is_live());
    assert!(!argument_loan.value().native_object_is_live());
}

#[test]
fn original_trace_procedure_heads_remain_live_during_their_actual_activation() {
    // Source contract: naming.error.original-invocation-context-capture
    // docs/design/analysis/name-resolution-proofs/error-original-invocation-context-capture.md
    // The guest source is a software lifetime control, not a native fixture.
    struct Inspect(std::rc::Rc<std::cell::Cell<usize>>);
    impl crate::command::NativeCommand for Inspect {
        fn invoke(&self, vm: &mut Vm, _: &[Value]) -> tcl_runtime_api::Completion<Value> {
            let words = vm.frame_argv(vm.current_level()).unwrap();
            let head = &words[0];
            assert!(head.native_object_is_live());
            assert!(head.native_object_reference_count() > 1);
            self.0.set(self.0.get() + 1);
            crate::interp::ok(Value::empty())
        }
    }
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let count = std::rc::Rc::new(std::cell::Cell::new(0));
        vm.register_native_command("inspect", std::rc::Rc::new(Inspect(count.clone())));
        let completion = vm.try_eval_source(
                "proc observer {n1 n2 op} {inspect; error READ_FAIL}; proc work {} {set d {k 3}; trace add variable d read observer; catch {dict lappend d k Y Z} m; trace remove variable d read observer; return $m}; work",
            ).unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok, "{engine}");
        assert_eq!(count.get(), 1, "{engine}");
    }
}

#[test]
fn direct_object_vector_errors_observe_original_words_at_the_native_frontier() {
    struct Failure;
    impl crate::command::NativeCommand for Failure {
        fn invoke(&self, _: &mut Vm, _: &[Value]) -> tcl_runtime_api::Completion<Value> {
            crate::interp::err("FAIL")
        }
    }
    let observations = [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_object_vector_log/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_object_vector_log/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_object_vector_log/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_object_vector_log/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_object_vector_log/9.1.0.tsv"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_object_vector_log/Jim.tsv"),
        ),
    ];
    let mut compared = 0;
    for (engine, rows) in observations {
        for row in rows.lines().filter(|row| row.starts_with("0\t")) {
            let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
            tcl_test_support::oracle_row_progress("objectvector24", engine, fields[1], None);
            let mut vm = crate::native_fixture::core(
                tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            );
            vm.register_native_command("fail", std::rc::Rc::new(Failure));
            let original = match fields[1] {
                "0" => Value::int(17),
                "1" | "3" => Value::new_native_string_bytes(b"A\0B".as_slice()),
                "2" => Value::new_native_string_bytes(b"#hash".as_slice()),
                _ => unreachable!("native fixed controls"),
            };
            let resident =
                |value: &Value| usize::from(value.resident_string_bytes().is_some()).to_string();
            assert_eq!(
                resident(&original),
                fields[2].strip_prefix("before=").unwrap(),
                "{engine}/{row}"
            );
            let mut arguments = vec![
                Value::new_native_string_bytes(b"direct".as_slice()),
                original,
            ];
            if fields[1] == "3" {
                arguments.push(Value::new_native_string_bytes(b"TAIL".as_slice()));
            }
            let completion = vm.try_invoke_command("fail", &arguments).unwrap();
            assert_eq!(
                completion.code.as_int().to_string(),
                fields[4].strip_prefix("code=").unwrap(),
                "{engine}/{row}"
            );
            assert_eq!(
                resident(&arguments[1]),
                fields[3].strip_prefix("after=").unwrap(),
                "{engine}/{row}"
            );
            let info = vm.error_info_value().unwrap_or_default().iter().fold(
                String::new(),
                |mut text, byte| {
                    use std::fmt::Write as _;
                    write!(&mut text, "{byte:02x}").unwrap();
                    text
                },
            );
            assert_eq!(
                info,
                fields[5].strip_prefix("info=").unwrap(),
                "{engine}/{row}"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 24);
}

#[test]
fn namespace_unknown_getter_retains_the_accepted_root_object() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        let mut vm = Vm::new();
        vm.set_runtime_version(version);
        let root = Value::string("::unknown tag");
        let length = ValueOps::list_len(&mut vm, &root).unwrap();
        vm.ns_unknown_set(root.clone(), length);
        assert!(vm.ns_unknown_get().unwrap().is_same_object(&root));
        let blank = Value::string(" \t ");
        let length = ValueOps::list_len(&mut vm, &blank).unwrap();
        assert_eq!(length, 0);
        vm.ns_unknown_set(blank, length);
        assert!(!vm.ns_unknown_get().unwrap().is_same_object(&root));
    }
}

#[test]
fn unknown_handler_lookup_mutates_its_original_command_object() {
    use std::rc::Rc;
    use tcl_core_types::NameBytes;
    use tcl_runtime_api::{CommandSlot, ROOT_NS};
    struct Handler(&'static str);
    impl crate::command::NativeCommand for Handler {
        fn invoke(&self, _vm: &mut Vm, args: &[Value]) -> tcl_runtime_api::Completion<Value> {
            assert_eq!(
                args[0].resident_string_bytes().unwrap().as_ref(),
                b"missing"
            );
            crate::interp::ok(Value::string(self.0))
        }
    }
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        let mut vm = crate::native_fixture::core(
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
        );
        let slot = CommandSlot {
            namespace: ROOT_NS,
            simple: NameBytes::from(b"h\xff".as_slice()),
        };
        vm.register_command_in_slot(
            slot.clone(),
            crate::command::Command::Native(Rc::new(Handler("FIRST"))),
        );
        let head = Value::new_native_string_bytes(b"h\xff".to_vec());
        let protocol = vm
            .native_invocation_dialect()
            .native_string_materialization(None)
            .unwrap()
            .protocol();
        vm.ns_unknown_set(
            Value::native_list_constructor(vec![head.clone()], protocol),
            1,
        );
        assert!(head.native_command_name_cache().is_none());
        let missing = Value::string("missing");
        let invoke = |vm: &mut Vm| {
            vm.invoke_command_value_at(
                ROOT_NS,
                &missing,
                &[],
                &[],
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            )
        };
        let first = invoke(&mut vm);
        assert!(first.code.is_ok());
        assert_eq!(
            first.result.resident_string_bytes().unwrap().as_ref(),
            b"FIRST"
        );
        let before = head
            .native_command_name_cache()
            .expect("native unknown-prefix getter");
        assert_eq!(before.version, version);
        vm.register_command_in_slot(
            slot,
            crate::command::Command::Native(Rc::new(Handler("SECOND"))),
        );
        let second = invoke(&mut vm);
        assert!(second.code.is_ok());
        assert_eq!(
            second.result.resident_string_bytes().unwrap().as_ref(),
            b"SECOND"
        );
        assert_ne!(head.native_command_name_cache().unwrap(), before);
        assert_eq!(head.resident_string_bytes().unwrap().as_ref(), b"h\xff");
    }
}

#[test]
fn namespace_introspection_instructions_use_original_native_command_objects() {
    use std::rc::Rc;
    use tcl_bytecode::{FunctionAsm, Instruction, Op};
    use tcl_core_types::NameBytes;
    use tcl_runtime_api::{CommandSlot, ROOT_NS};
    struct EmptyCommand;
    impl crate::command::NativeCommand for EmptyCommand {
        fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> tcl_runtime_api::Completion<Value> {
            crate::interp::ok(Value::empty())
        }
    }
    fn unhex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    let fixtures = [
        include_str!("../../tests/data/native_namespace_command_names/8.4.20.tsv"),
        include_str!("../../tests/data/native_namespace_command_names/8.5.19.tsv"),
        include_str!("../../tests/data/native_namespace_command_names/8.6.18.tsv"),
        include_str!("../../tests/data/native_namespace_command_names/9.0.4.tsv"),
        include_str!("../../tests/data/native_namespace_command_names/9.1.0.tsv"),
    ];
    let mut compared = 0;
    for (version, fixture) in tcl_dialect::TclVersion::ALL.into_iter().zip(fixtures) {
        assert_eq!(fixture.lines().count(), 6);
        for row in fixture.lines() {
            let columns = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
            assert_eq!(columns.len(), 6);
            let op = match columns[0] {
                "which" => Op::RESOLVE_CMD,
                "origin" => Op::ORIGIN_CMD,
                _ => panic!("unknown native namespace observation"),
            };
            let written = unhex(columns[5]);
            let expected = unhex(columns[3]);
            // Compare the command and opcode through independent original
            // objects, so the command cannot donate a cache to the opcode.
            for compiled in [false, true] {
                let mut vm = crate::native_fixture::core(
                    tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap(),
                );
                vm.register_command_in_slot(
                    CommandSlot {
                        namespace: ROOT_NS,
                        simple: NameBytes::from(b"opaque\xff".as_slice()),
                    },
                    crate::command::Command::Native(Rc::new(EmptyCommand)),
                );
                let original = Value::new_native_string_bytes(written.clone());
                let completion = if compiled {
                    let unit = vm.compiled_unit(
                        Rc::new(FunctionAsm {
                            instructions: vec![Instruction::new(op, vec![])],
                            ..FunctionAsm::default()
                        }),
                        vm.source_namespace_path(),
                    );
                    let mut frame = super::Frame::new(unit, false);
                    frame.stack.push(original.clone());
                    match vm.tick(&mut frame) {
                        super::Tick::Continue => crate::interp::ok(frame.stack.pop().unwrap()),
                        super::Tick::Return(completion) => completion,
                        _ => panic!("namespace introspection must finish its original getter"),
                    }
                } else {
                    let mut args = vec![Value::string(columns[0])];
                    if op == Op::RESOLVE_CMD {
                        args.push(Value::string("-command"));
                    }
                    args.push(original.clone());
                    vm.try_invoke_command("namespace", &args).unwrap()
                };
                assert_eq!(
                    completion.code.as_int().to_string(),
                    columns[2],
                    "{version:?} {row} compiled={compiled}"
                );
                assert_eq!(
                    vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                    expected,
                    "{version:?} {row} compiled={compiled}"
                );
                assert_eq!(
                    original.native_command_name_cache_origin(),
                    (columns[4] == "cmdName").then_some(version)
                );
                assert_eq!(original.string_bytes().as_ref(), written);
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 60);
}

/// A dict `Value`'s top-level `(key, value-string)` pairs, mirroring the
/// production `dict_pairs` decode.
fn top_pairs(v: &Value) -> Vec<(String, String)> {
    v.as_list()
        .expect("dict value is a valid list")
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| (c[0].to_str().to_string(), c[1].to_str().to_string()))
        .collect()
}

#[test]
fn imm_index_decodes_literal_and_end_relative() {
    // A plain non-negative immediate is the literal index.
    assert_eq!(imm_index(3, 10), 3);
    assert_eq!(imm_index(0, 10), 0);
    // `INDEX_END` is `end`; `INDEX_END - k` is `end-k`.
    assert_eq!(imm_index(INDEX_END, 10), 9);
    assert_eq!(imm_index(INDEX_END - 1, 10), 8);
    // `end` of an empty list is -1 (one before the first slot).
    assert_eq!(imm_index(INDEX_END, 0), -1);
}

#[test]
fn char_find_returns_character_indices_not_byte_offsets() {
    // ASCII: first vs last occurrence, and a miss.
    assert_eq!(char_find("hello", "l", false), 2);
    assert_eq!(char_find("hello", "l", true), 3);
    assert_eq!(char_find("hello", "z", false), -1);
    // Multi-byte: `é` is two UTF-8 bytes, but the result is a *character*
    // index — `string first`/`last` operate on characters, not bytes.
    assert_eq!(char_find("héllo", "é", false), 1);
    assert_eq!(char_find("héllo", "llo", false), 2);
    assert_eq!(char_find("héllo", "l", true), 3);
    // An empty needle is a *miss*, not a match at 0 / at the end: C's
    // `TclStringFirst`/`TclStringLast` special-case `ln == 0` and leave the
    // result at -1 ("We don't find empty substrings.  Bizarre!",
    // `tclStringObj.c:3853-3858`, `:3948-3956`), where Rust's
    // `str::find`/`rfind` would answer 0 and 5.
    assert_eq!(char_find("hello", "", false), -1);
    assert_eq!(char_find("hello", "", true), -1);
    assert_eq!(char_find("", "", false), -1);
}

#[test]
fn dict_set_path_creates_updates_and_autovivifies() {
    let mut vm = Vm::new();
    let s = Value::string;
    // `dict set {} a 1` → {a 1}.
    let d = vm
        .dict_set_path(&Value::list(vec![]), &[s("a")], s("1"))
        .unwrap();
    assert_eq!(top_pairs(&d), [("a".into(), "1".into())]);

    // Updating an existing key keeps its position (order-preserving).
    let base = Value::list(vec![s("a"), s("1"), s("b"), s("2")]);
    let updated = vm.dict_set_path(&base, &[s("a")], s("9")).unwrap();
    assert_eq!(
        top_pairs(&updated),
        [("a".into(), "9".into()), ("b".into(), "2".into())]
    );

    // A new key appends at the end.
    let appended = vm.dict_set_path(&base, &[s("c")], s("3")).unwrap();
    assert_eq!(
        top_pairs(&appended),
        [
            ("a".into(), "1".into()),
            ("b".into(), "2".into()),
            ("c".into(), "3".into())
        ]
    );

    // A multi-key path auto-vivifies intermediate dicts: the inner dict's
    // list string-rep is "b 1".
    let nested = vm
        .dict_set_path(&Value::list(vec![]), &[s("a"), s("b")], s("1"))
        .unwrap();
    assert_eq!(top_pairs(&nested), [("a".into(), "b 1".into())]);
}

#[test]
fn dict_unset_path_removes_and_is_a_noop_when_absent() {
    let mut vm = Vm::new();
    let s = Value::string;
    let base = Value::list(vec![s("a"), s("1"), s("b"), s("2")]);

    // Removing a present key drops just that pair.
    let removed = vm.dict_unset_path(&base, &[s("a")]).unwrap();
    assert_eq!(top_pairs(&removed), [("b".into(), "2".into())]);

    // Removing an absent key leaves the dict unchanged (matching Tcl).
    let untouched = vm.dict_unset_path(&base, &[s("z")]).unwrap();
    assert_eq!(
        top_pairs(&untouched),
        [("a".into(), "1".into()), ("b".into(), "2".into())]
    );

    // A nested unset rewrites only the inner dict: {a {b 1 c 2}} → {a {c 2}}.
    let inner = Value::list(vec![s("b"), s("1"), s("c"), s("2")]);
    let outer = Value::list(vec![s("a"), inner]);
    let nested = vm.dict_unset_path(&outer, &[s("a"), s("b")]).unwrap();
    assert_eq!(top_pairs(&nested), [("a".into(), "c 2".into())]);
}

/// The shared `lset` core backs the compiled `INST_LSET_LIST`/`INST_LSET_FLAT`
/// opcodes; a naive implementation recursing once per index in `lset`'s
/// (possibly nested) index path has no depth cap, so a flat index path is
/// trivially inflated via `lset listVar {*}[lrepeat 100000 0] v`,
/// overflowing the native stack (SIGABRT) between depth 1800 and 2000 on
/// a 2 MiB thread (`cargo test`'s per-test default). The iterative
/// implementation has no such cap; this test checks exact correctness at
/// depth 2000, comfortably past that crash range — descending back down
/// the same index path lands on the value that was set, not merely
/// survival.
///
/// Deliberately NOT 50,000+: constructing (and, at the end of this
/// test, dropping) a `Value::list` chain nested that deep is its own,
/// unrelated native-stack risk — `Value` has no custom `Drop` impl, so
/// the compiler-generated recursive drop glue walks the same chain
/// (empirically, SIGABRT between depth 3500 and 4000 on a 2 MiB thread
/// for construction+drop alone, independent of any operation performed
/// on the value). That is a separate, genuinely unbounded-depth concern
/// in `Value`'s representation itself, not in the core's iterative
/// logic, and this test does not cover it.
#[test]
fn deeply_nested_lset_survives_and_is_correct() {
    const DEPTH: usize = 2_000;
    let mut v = Value::string("orig");
    for _ in 0..DEPTH {
        v = Value::list(vec![v]);
    }
    let path: Vec<Value> = (0..DEPTH).map(|_| Value::int(0)).collect();
    let mut vm = Vm::new();
    let result = tcl_cmd_core::list::lset(
        &mut vm,
        &v,
        &path,
        Value::string("new"),
        tcl_dialect::TclVersion::V9_0,
    )
    .expect("the lset core survives");
    let mut cur = result;
    for _ in 0..DEPTH {
        let items = cur.as_list().expect("valid list at every level");
        assert_eq!(items.len(), 1);
        cur = items[0].clone();
    }
    assert_eq!(&*cur.to_str(), "new");
}

/// A moderately nested `lset` index path (well within realistic use) is
/// byte-for-byte unaffected by the iterative rewrite.
#[test]
fn moderately_nested_lset_matches_previous_behavior() {
    let n = Value::int;
    let mut vm = Vm::new();
    let mut lset = |list: &Value, path: &[Value], value: Value| {
        tcl_cmd_core::list::lset(&mut vm, list, path, value, tcl_dialect::TclVersion::V9_0)
    };
    // Set an existing element two levels deep.
    let list = Value::list(vec![
        Value::list(vec![n(1), n(2)]),
        Value::list(vec![n(3), n(4)]),
    ]);
    let updated = lset(&list, &[n(1), n(0)], n(99)).unwrap();
    assert_eq!(&*updated.to_str(), "{1 2} {99 4}");

    // An empty path replaces the whole value (`lset x {} v` == `set x v`).
    let replaced = lset(&list, &[], Value::string("whole")).unwrap();
    assert_eq!(&*replaced.to_str(), "whole");

    // `idx == len` appends a fresh slot.
    let flat = Value::list(vec![n(1), n(2)]);
    let appended = lset(&flat, &[n(2)], n(3)).unwrap();
    assert_eq!(&*appended.to_str(), "1 2 3");

    // Out-of-range and non-numeric indices still error.
    assert!(lset(&flat, &[n(5)], n(0)).is_err());
    assert!(lset(&flat, &[Value::string("bogus")], n(0)).is_err());
}

#[test]
fn original_arithmetic_opcode_workers_keep_cached_values_and_retained_host_failure() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Calls the actual opcode workers with a placeholder frame; this does
    // not claim compiler admission or external native execution.
    use super::{Frame, bin, cmp};
    use tcl_syntax::expr::BinOp;
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let dialect = vm.actual_native_invocation_dialect();
    let value = |integer| {
        Value::from_native_scalar_cache(
            tcl_syntax::scalar_getter::NativeScalarCache::Number(tcl_syntax::number::Number::Int(
                integer,
            )),
            None,
            dialect,
        )
        .unwrap()
    };
    let left = value(6);
    let right = value(18);
    let before = [
        left.native_object_snapshot(),
        right.native_object_snapshot(),
    ];
    let mut frame = Frame::new(vm.current_placeholder_unit(), false);
    frame.stack.extend([left.clone(), right.clone()]);
    bin(&mut vm, &mut frame, BinOp::Add).unwrap();
    assert_eq!(frame.stack.pop().unwrap().as_int().unwrap(), 24);
    assert_eq!(
        [
            left.native_object_snapshot(),
            right.native_object_snapshot()
        ],
        before
    );
    frame.stack.extend([left.clone(), right.clone()]);
    cmp(&mut vm, &mut frame, BinOp::Lt).unwrap();
    assert!(frame.stack.pop().unwrap().as_bool().unwrap());
    assert_eq!(left.native_object_reference_count(), 1);
    assert_eq!(right.native_object_reference_count(), 1);
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter_native(&mut vm)
            .unwrap();
    let _ = vm.refuse_host_command("entered arithmetic opcode owner".into());
    let first = vm.execution_refusal.clone().unwrap();
    frame.stack.extend([left, right]);
    assert!(bin(&mut vm, &mut frame, BinOp::Add).is_err());
    assert!(frame.stack.is_empty());
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
}
