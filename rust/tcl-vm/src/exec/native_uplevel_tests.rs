// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit original-level evaluation, distinct from optional command parsing.

use std::rc::Rc;

use crate::command::NativeCommand;
use crate::{Value, Vm};
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::value::ValueOps;

#[test]
fn level_reference_reselects_current_frames_in_thirty_native_windows() {
    let mut compared = 0;
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tcl-syntax/tests/data/native_frame_reference/9.1.0.tsv"),
        ),
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::core(profile);
        let mut other = crate::native_fixture::core(profile);
        let original = Value::new_native_string_bytes(b"#1".as_slice());
        let mut records = rows.lines();
        compare_level_reference(&mut vm, &original, records.next().unwrap());
        vm.push_call_frame(None, vec![]);
        compare_level_reference(&mut vm, &original, records.next().unwrap());
        vm.pop_call_frame();
        compare_level_reference(&mut vm, &original, records.next().unwrap());
        vm.push_call_frame(None, vec![]);
        compare_level_reference(&mut vm, &original, records.next().unwrap());
        other.push_call_frame(None, vec![]);
        compare_level_reference(&mut other, &original, records.next().unwrap());
        let duplicate = original.duplicate_native_object_in(
            vm.actual_native_invocation_dialect()
                .native_string_protocol()
                .unwrap(),
        );
        drop(original);
        compare_level_reference(&mut other, &duplicate, records.next().unwrap());
        assert!(records.next().is_none());
        other.pop_call_frame();
        vm.pop_call_frame();
        compared += 6;
    }
    assert_eq!(compared, 30);
}

fn compare_level_reference(vm: &mut Vm, level: &Value, record: &str) {
    let fields: Vec<_> = record.split('\t').collect();
    let target = crate::command::runtime_explicit_frame_selection(vm, level);
    assert_eq!(
        target.as_ref().map_or(-1, |_| 1),
        fields[1].parse::<i32>().unwrap(),
        "{}",
        fields[0]
    );
    assert_eq!(
        target
            .as_ref()
            .map_or(-1, |level| i32::try_from(*level).unwrap()),
        fields[2].parse::<i32>().unwrap(),
        "{}",
        fields[0]
    );
    assert_eq!(level.native_object_type_name(), fields[3], "{}", fields[0]);
    assert_eq!(level.resident_string_bytes().is_some(), fields[4] == "1");
    assert_eq!(
        level.native_object_reference_count(),
        fields[5].parse::<usize>().unwrap()
    );
    assert!(!level.native_primary_has_free_hook());
    if fields[3] == "levelReference" {
        assert_eq!(
            level
                .native_frame_level_cache_in(vm.actual_native_invocation_dialect())
                .unwrap(),
            Some(tcl_registry::NativeFrameLevelCache::Absolute(1))
        );
    }
}

struct Explicit;
impl NativeCommand for Explicit {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let [level, script] = args else {
            panic!("fixed original native probe arity");
        };
        crate::command::compiled_uplevel(vm, level, script.clone())
    }
}

fn script(case: usize) -> Value {
    match case {
        10 => Value::new_native_string_bytes(b"error BODY".as_slice()),
        11 => Value::new_native_string_bytes(b"set prefix BEFORE; set x {".as_slice()),
        12 => Value::list(vec![
            Value::string("set"),
            Value::string("marker"),
            Value::string("VALUE"),
        ]),
        13 => Value::new_native_string_bytes(b"break".as_slice()),
        _ => Value::new_native_string_bytes(b"list [info level] [info exists local]".as_slice()),
    }
}

#[test]
fn explicit_uplevel_preserves_seventy_native_level_body_and_restore_windows() {
    let levels: [&[u8]; 14] = [
        b"#0", b"0", b"1", b"bad", b"+0", b"99", b"-1", b"1.0", b"1\0X", b"#0\0X", b"#0", b"#0",
        b"#0", b"#0",
    ];
    let mut compared = 0;
    for (engine, rows) in [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_explicit_uplevel/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_explicit_uplevel/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_explicit_uplevel/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_explicit_uplevel/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_explicit_uplevel/9.1.0.tsv"),
        ),
    ] {
        for row in rows.lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let case = fields[0].parse::<usize>().unwrap();
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let mut vm = crate::native_fixture::interpreter(profile);
            vm.register_native_command("explicit_eval", Rc::new(Explicit));
            let definition = vm.invoke_command(
                "proc",
                &[
                    Value::string("p"),
                    Value::string("level script"),
                    Value::string("set local LOCAL; explicit_eval $level $script"),
                ],
            );
            assert_eq!(definition.code, Code::Ok, "{engine}/{case}");
            drop(definition);
            let original_level = Value::new_native_string_bytes(levels[case]);
            let original_script = script(case);
            assert_eq!(
                original_level.native_object_type_name(),
                fields[1],
                "{engine}/{case}"
            );
            assert_eq!(
                usize::from(original_level.resident_string_bytes().is_some()).to_string(),
                fields[2]
            );
            let completed = vm
                .try_invoke_command("p", &[original_level.clone(), original_script.clone()])
                .unwrap_or_else(|error| {
                    panic!("{engine}/{case}: original uplevel activation: {error:?}")
                });
            assert_explicit_uplevel_result(
                &mut vm,
                &completed,
                &original_level,
                &original_script,
                &fields,
                engine,
                case,
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 70);
}

fn assert_explicit_uplevel_result(
    vm: &mut Vm,
    completed: &Completion<Value>,
    original_level: &Value,
    original_script: &Value,
    fields: &[&str],
    engine: &str,
    case: usize,
) {
    assert_eq!(
        completed.code.as_int().to_string(),
        fields[3],
        "{engine}/{case}: {}",
        completed.result.to_str()
    );
    let expected: Vec<_> = fields[4]
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(
        vm.native_string_bytes(&completed.result).unwrap().as_ref(),
        expected.as_slice(),
        "{engine}/{case}"
    );
    assert_eq!(
        original_level.native_object_type_name(),
        fields[5],
        "{engine}/{case}"
    );
    assert_eq!(
        usize::from(original_level.resident_string_bytes().is_some()).to_string(),
        fields[6],
        "{engine}/{case}"
    );
    assert_eq!(
        usize::from(original_script.resident_string_bytes().is_some()).to_string(),
        fields[7],
        "{engine}/{case}"
    );
    assert_eq!(
        vm.current_level(),
        0,
        "original caller restored: {engine}/{case}"
    );
    assert_eq!(fields[8], "1", "native restoration: {engine}/{case}");
    if case == 12 {
        assert_eq!(
            vm.get_var_bytes(b"marker").unwrap().to_str().as_ref(),
            "VALUE"
        );
    }
}

#[test]
fn uplevel_opcode_keeps_nonlevel_operand_out_of_the_original_script() {
    use super::{Frame, Tick};
    use tcl_bytecode::{FunctionAsm, Instruction, Op};
    let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    let mut vm = crate::native_fixture::interpreter(profile);
    vm.push_call_frame(Some("p".to_owned()), vec![Value::string("p")]);
    let level = Value::new_native_string_bytes(b"bad".as_slice());
    let original_script = Value::new_native_string_bytes(b"set result VALUE".as_slice());
    let asm = FunctionAsm {
        instructions: vec![Instruction::new(Op::UPLEVEL, vec![])],
        ..Default::default()
    };
    let unit = vm.compiled_unit(Rc::new(asm), vm.source_namespace_path());
    let mut frame = Frame::new(unit, false);
    frame.stack = vec![level, original_script];
    let Tick::PushEval(request) = vm.tick(&mut frame) else {
        panic!("explicit native uplevel body activation");
    };
    assert_eq!(vm.current_level(), 0);
    assert_eq!(request.label, Some("uplevel"));
    let completion = vm.run_compiled_unit(request.script);
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.to_str().as_ref(), "VALUE");
    vm.restore_execution_frame(request.selected_frame_restore);
    assert_eq!(vm.current_level(), 1);
    vm.pop_call_frame();
    assert_eq!(vm.current_level(), 0);
}
