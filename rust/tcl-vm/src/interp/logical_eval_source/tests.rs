// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::compiled::CompilerProvenance;
use std::{cell::RefCell, io::Write};
use tcl_core_types::Code;
use tcl_registry::invocation_words::LogicalSourceWordProvider;
use tcl_registry::native_eval_object::LogicalEvalObjectProvider;

struct Capture(Rc<RefCell<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Missing {
    Eval,
    Words,
    Names,
    Variables,
}

fn simulation(missing: Option<Missing>) -> (crate::Vm, Rc<RefCell<Vec<u8>>>) {
    let output = Rc::new(RefCell::new(Vec::new()));
    let host = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut vm =
        crate::native_fixture::interpreter_with_output(host, Box::new(Capture(Rc::clone(&output))));
    let logical = tcl_dialect::DialectProfile::irules();
    vm.set_dialect_profile(logical);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    vm.set_compiler(Box::new(
        tcl_compiler::compile_service::BytecodeCompileService::for_profile(logical),
    ));
    if missing != Some(Missing::Eval) {
        assert!(
            vm.set_logical_eval_object_provider(LogicalEvalObjectProvider::Tcl84CoreSimulation)
        );
    }
    if missing != Some(Missing::Words) {
        assert!(
            vm.set_logical_source_word_provider(LogicalSourceWordProvider::Tcl84CoreSimulation)
        );
    }
    if missing != Some(Missing::Names) {
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
    }
    if missing != Some(Missing::Variables) {
        assert!(vm.set_logical_compiled_variable_provider(tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation));
    }
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
    ));
    (vm, output)
}

#[test]
fn logical_eval_uses_fresh_words_without_native_pool_cache_or_layout() {
    let (mut vm, output) = simulation(None);
    let source = Value::new_native_string_bytes(
        b"puts -nonewline reached; set logical_eval_result yes".as_slice(),
    );
    let prepared = vm
        .prepare_script_commands_value_for(&source, EvalObjectPurpose::Eval)
        .unwrap();
    let unit = prepared.prefix.unwrap();
    assert!(prepared.fatal_tail.is_none());
    assert!(matches!(unit.compiler, CompilerProvenance::LogicalEval(_)));
    assert!(unit.literal_pool.is_err());
    assert!(unit.native_cache.is_none());
    assert!(unit.compiled_local_layout.is_none());
    assert!(source.native_bytecode_cache().is_none());
    let completion = vm.run_compiled_unit(unit);
    assert!(vm.execution_refusal.is_none());
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"yes");
    assert_eq!(output.borrow().as_slice(), b"reached");
    assert!(source.native_bytecode_cache().is_none());
}

#[test]
fn logical_eval_requires_each_actual_provider_and_physical_core() {
    for missing in [
        Missing::Eval,
        Missing::Words,
        Missing::Names,
        Missing::Variables,
    ] {
        let (mut vm, output) = simulation(Some(missing));
        let source = Value::new_native_string_bytes(b"puts -nonewline forbidden".as_slice());
        assert!(
            vm.prepare_script_commands_value_for(&source, EvalObjectPurpose::Eval)
                .is_err()
        );
        assert!(output.borrow().is_empty());
    }
    let (mut vm, output) = simulation(None);
    let selected = vm.eval_object_protocol().unwrap();
    let c9 = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
    assert!(
        vm.logical_eval_source_entry(c9, selected, EvalObjectPurpose::Eval)
            .unwrap()
            .is_some()
    );
    assert!(
        vm.logical_eval_source_entry(
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
            selected,
            EvalObjectPurpose::Eval,
        )
        .is_err()
    );
    let pin = vm.native_core_pin.take();
    assert!(
        vm.logical_eval_source_entry(c9, selected, EvalObjectPurpose::Eval)
            .is_err()
    );
    assert!(output.borrow().is_empty());
    vm.native_core_pin = pin;
    assert!(
        vm.logical_eval_source_entry(c9, selected, EvalObjectPurpose::Eval)
            .unwrap()
            .is_some()
    );
}

#[test]
fn logical_eval_does_not_borrow_other_purposes_or_native_dispatch() {
    let (vm, _) = simulation(None);
    let c9 = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
    let selected = vm.eval_object_protocol().unwrap();
    for purpose in [
        EvalObjectPurpose::UpLevel,
        EvalObjectPurpose::CompiledStack,
        EvalObjectPurpose::NamespaceConcat,
        EvalObjectPurpose::TraceCallback,
    ] {
        assert!(
            vm.logical_eval_source_entry(c9, selected, purpose)
                .unwrap()
                .is_none()
        );
    }
    let native = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
        .native_eval_object_protocol()
        .unwrap();
    assert!(
        vm.logical_eval_source_entry(c9, native, EvalObjectPurpose::Eval)
            .unwrap()
            .is_none()
    );
}

#[test]
fn logical_eval_policy_withdrawal_prevents_fresh_word_execution() {
    let (mut vm, output) = simulation(None);
    let source = Value::new_native_string_bytes(b"puts -nonewline stale".as_slice());
    let unit = vm
        .prepare_script_commands_value_for(&source, EvalObjectPurpose::Eval)
        .unwrap()
        .prefix
        .unwrap();
    let operands = unit.direct_source_operands.unwrap();
    assert!(operands.is_current(&vm.native_compiler_policy()));
    vm.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    assert!(!operands.is_current(&vm.native_compiler_policy()));
    let completion = vm.run_compiled_unit(unit);
    assert_eq!(completion.code, Code::Error);
    assert!(output.borrow().is_empty());
}

#[test]
fn logical_eval_keeps_data_inert_and_retains_partial_prefix_order() {
    let (mut vm, output) = simulation(None);
    let source =
        Value::new_native_string_bytes(b"set logical_eval_data {[puts forbidden]}".as_slice());
    let unit = vm
        .prepare_script_commands_value_for(&source, EvalObjectPurpose::Eval)
        .unwrap()
        .prefix
        .unwrap();
    let completion = vm.run_compiled_unit(unit);
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(
        completion.result.string_bytes().as_ref(),
        b"[puts forbidden]"
    );
    assert!(output.borrow().is_empty());
    let source =
        Value::new_native_string_bytes(b"puts -nonewline prefix; set incomplete {".as_slice());
    let prepared = vm
        .prepare_script_commands_value_for(&source, EvalObjectPurpose::Eval)
        .unwrap();
    assert!(prepared.fatal_tail.is_some());
    let completion = vm.run_compiled_unit(prepared.prefix.unwrap());
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(output.borrow().as_slice(), b"prefix");
}

#[test]
fn logical_eval_fresh_word_plan_rejects_private_allocations_and_keeps_words_distinct() {
    let (vm, _) = simulation(None);
    let c9 = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
    let selected = vm.eval_object_protocol().unwrap();
    let entry = vm
        .logical_eval_source_entry(c9, selected, EvalObjectPurpose::Eval)
        .unwrap()
        .unwrap();
    let mut asm = tcl_bytecode::FunctionAsm {
        plain_command_dispatch: true,
        ..tcl_bytecode::FunctionAsm::default()
    };
    asm.literals.intern_bytes(b"17\0suffix\xff");
    let literal = &asm.literals.entries()[0];
    let first = entry.value(literal);
    let second = entry.value(literal);
    assert!(!first.is_same_object(&second));
    assert_eq!(first.native_scalar_cache(), None);
    assert_eq!(
        first.resident_string_bytes().unwrap().as_ref(),
        literal.bytes()
    );
    let namespace = tcl_runtime_api::ByteNamespacePath::root();
    asm.plain_command_dispatch = false;
    assert!(
        vm.logical_eval_source_unit(
            &Rc::new(asm.clone()),
            &namespace,
            c9,
            selected,
            EvalObjectPurpose::Eval
        )
        .is_err()
    );
    asm.plain_command_dispatch = true;
    asm.literals.register_private_constant_list(
        &[b"x"],
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
    );
    assert!(
        vm.logical_eval_source_unit(
            &Rc::new(asm),
            &namespace,
            c9,
            selected,
            EvalObjectPurpose::Eval
        )
        .is_err()
    );
}

#[test]
fn logical_eval_refuses_a_required_compiled_local_layout() {
    let (vm, _) = simulation(None);
    let c9 = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
    let selected = vm.eval_object_protocol().unwrap();
    // This requirement supplies no real table or activation; even an otherwise
    // plain plan must not turn it into permission to borrow compiled cells.
    let asm = tcl_bytecode::FunctionAsm {
        plain_command_dispatch: true,
        required_compiled_local_layout: Some(tcl_runtime_api::native_compilation::NativeCompiledLocalLayout {
            owner: vm.native_interpreter_identity(), token: 1, epoch: 1,
            kind: tcl_runtime_api::native_compilation::NativeCompiledLocalLayoutKind::RetainedLocalCache,
            names: vec![Some(tcl_runtime_api::NameBytes::from(b"receiver".as_slice()))],
        }),
        ..tcl_bytecode::FunctionAsm::default()
    };
    assert!(crate::literal_pool::plain_source_operand_plan(&asm));
    assert!(
        vm.logical_eval_source_unit(
            &Rc::new(asm),
            &tcl_runtime_api::ByteNamespacePath::root(),
            c9,
            selected,
            EvalObjectPurpose::Eval
        )
        .is_err()
    );
}
