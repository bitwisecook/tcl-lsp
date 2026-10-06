// Copyright (C) 2026 tcl-lsp contributors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::rc::Rc;

use super::{Frame, Tick};
use crate::interp::{Vm, ok};
use crate::value::Value;
use tcl_bytecode::{FunctionAsm, Instruction, Op, Operand};
use tcl_runtime_api::Completion;
use tcl_runtime_api::native_compilation::NativeMathFunctionResolution;

fn fixed_call(vm: &Vm, name: &str) -> FunctionAsm {
    let prerequisite = vm
        .native_fixed_math_prerequisite()
        .expect("actual C8.4 fixed table");
    let NativeMathFunctionResolution::Present(binding) = prerequisite.table.lookup(name) else {
        panic!("actual fixed registration for {name}");
    };
    let argc = i32::try_from(binding.arity.unwrap()).unwrap();
    let mut instruction = Instruction::new(Op::CALL_FUNC1, vec![Operand::Imm(argc)]);
    instruction.native_fixed_math_call = Some(binding.clone());
    FunctionAsm {
        instructions: vec![instruction],
        native_math_table_prerequisite: Some(prerequisite),
        ..Default::default()
    }
}

fn vm84() -> Vm {
    crate::native_fixture::core(tcl_dialect::DialectProfile::find("tcl8.4").unwrap())
}

fn tick_call(vm: &mut Vm, asm: FunctionAsm, stack: Vec<Value>) -> (Tick, Frame) {
    let unit = vm.compiled_unit(Rc::new(asm), vm.source_namespace_path());
    let mut frame = Frame::new(unit, false);
    frame.stack = stack;
    (vm.tick(&mut frame), frame)
}

#[test]
fn original_fixed_math_opcode_consumes_only_actual_value_arguments() {
    for (name, args, expected) in [
        ("abs", vec![Value::string("-4")], "4"),
        ("pow", vec![Value::string("2"), Value::string("3")], "8.0"),
    ] {
        let mut vm = vm84();
        let asm = fixed_call(&vm, name);
        assert_eq!(asm.validate_native_compilation_entry(), Ok(()));
        let sentinel = Value::string("original lower stack object");
        let mut stack = vec![sentinel.clone()];
        stack.extend(args);
        let (tick, frame) = tick_call(&mut vm, asm, stack);
        assert!(matches!(tick, Tick::Continue));
        assert_eq!(frame.stack.len(), 2);
        assert!(frame.stack[0].is_same_object(&sentinel));
        assert_eq!(frame.stack[1].to_str().as_ref(), expected);
    }
    let mut vm = vm84();
    let asm = fixed_call(&vm, "rand");
    let sentinel = Value::string("retained");
    let (tick, frame) = tick_call(&mut vm, asm, vec![sentinel.clone()]);
    assert!(matches!(tick, Tick::Continue));
    assert!(frame.stack[0].is_same_object(&sentinel));
    let result = frame.stack[1].to_str().parse::<f64>().unwrap();
    assert!((0.0..1.0).contains(&result));
}

fn shadow(_: &mut Vm, _: &[Value]) -> Completion<Value> {
    ok(Value::string("COMMAND_SHADOW"))
}

#[test]
fn original_fixed_math_opcode_is_independent_of_same_spelled_commands() {
    let mut vm = vm84();
    let asm = fixed_call(&vm, "abs");
    vm.register("::tcl::mathfunc::abs", shadow);
    assert!(vm.native_math_table_prerequisite_matches(&asm));
    let (tick, frame) = tick_call(&mut vm, asm, vec![Value::string("-4")]);
    assert!(matches!(tick, Tick::Continue));
    assert_eq!(frame.stack[0].to_str().as_ref(), "4");
}

#[test]
fn original_fixed_math_opcode_requires_exact_registration_and_arity() {
    let mut vm = vm84();
    let asm = fixed_call(&vm, "abs");
    for mutation in 0..5 {
        let mut invalid = asm.clone();
        match mutation {
            0 => invalid.instructions[0].native_fixed_math_call = None,
            1 => invalid.native_math_table_prerequisite = None,
            2 => invalid.instructions[0].operands = vec![Operand::Imm(2)],
            3 => invalid.instructions[0].operands.clear(),
            4 => {
                invalid.instructions[0]
                    .native_fixed_math_call
                    .as_mut()
                    .unwrap()
                    .implementation_generation += 1;
            }
            _ => unreachable!(),
        }
        assert_eq!(
            invalid.validate_native_compilation_entry(),
            Err(tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired)
        );
    }
    vm.register_stock_builtin("tcl::mathfunc::abs", shadow);
    assert!(!vm.native_math_table_prerequisite_matches(&asm));
    assert!(matches!(
        vm.try_run_function(&asm),
        Err(tcl_runtime_api::NativeExecutionError::CompilationAdmission(
            tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired
        ))
    ));
    let mut original = vm84();
    let original_asm = fixed_call(&original, "abs");
    let unit = original.compiled_unit(Rc::new(original_asm), original.source_namespace_path());
    let selected = unit.fixed_math_calls.unwrap().get(&0).unwrap().clone();
    let mut foreign = vm84();
    assert!(!foreign.native_math_table_prerequisite_matches(&asm));
    foreign.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl9.0").unwrap());
    let completion = foreign.invoke_native_fixed_math_call(&selected, &[Value::string("-4")]);
    assert!(foreign.finish_host_execution(completion).is_err());
}

#[test]
fn original_fixed_math_opcode_keeps_its_entered_handler_after_operand_replacement() {
    let mut vm = vm84();
    let asm = fixed_call(&vm, "abs");
    let unit = vm.compiled_unit(Rc::new(asm.clone()), vm.source_namespace_path());
    let mut entered = Frame::new(unit, false);
    // The native C8.4 control replaces abs during its substituted argument.
    // The entered builtin still returns 4; the next compilation selects 999.
    vm.register_stock_builtin("tcl::mathfunc::abs", shadow);
    assert!(!vm.native_math_table_prerequisite_matches(&asm));
    entered.stack.push(Value::string("-4"));
    assert!(matches!(vm.tick(&mut entered), Tick::Continue));
    assert_eq!(entered.stack[0].to_str().as_ref(), "4");
    let replacement = fixed_call(&vm, "abs");
    let (tick, frame) = tick_call(&mut vm, replacement, vec![Value::string("-4")]);
    assert!(matches!(tick, Tick::Continue));
    assert_eq!(frame.stack[0].to_str().as_ref(), "COMMAND_SHADOW");
}

#[test]
fn original_fixed_math_opcode_preserves_native_numeric_failure() {
    let mut vm = vm84();
    let asm = fixed_call(&vm, "abs");
    let (tick, _) = tick_call(&mut vm, asm, vec![Value::string("not-a-number")]);
    let Tick::Return(completion) = tick else {
        panic!("native math conversion must fail");
    };
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    assert!(vm.finish_host_execution(completion).is_ok());
}
