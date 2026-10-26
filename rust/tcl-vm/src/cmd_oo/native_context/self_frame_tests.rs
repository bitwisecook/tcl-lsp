// SPDX-License-Identifier: AGPL-3.0-or-later
//! Entered self-report frame observations; no Native Method or Class lifetime claim.
use super::*;
use crate::command::{Command, NativeCommand};
use crate::interp::native_name_consumers::OriginalNameConsumerScope;
use std::{cell::Cell, rc::Rc};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

struct Probe {
    mutation: Option<u8>,
    hits: Rc<Cell<usize>>,
}
impl NativeCommand for Probe {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.hits.set(self.hits.get() + 1);
        let context = OriginalNameConsumerScope::enter(vm).unwrap();
        let frame = current_method(vm).unwrap();
        let chain = frame.chain.original_steps();
        let chain_count = chain.strong_count();
        let method = frame.chain[frame.index].method_owner.as_ref().unwrap();
        let method_count = Rc::strong_count(method);
        let observed = OriginalMethodFrame::capture(vm).unwrap();
        assert_eq!(chain.strong_count(), chain_count);
        assert_eq!(Rc::strong_count(method), method_count);
        let option = Value::new_native_string_bytes(b"method".as_slice());
        let references = option.native_object_reference_count();
        assert_eq!(
            vm.native_static_option_index_checked(
                &option,
                tcl_registry::native_tcloo_compilation::SELF_SUBCOMMANDS,
                false,
                "subcommand",
            )
            .unwrap(),
            tcl_registry::native_tcloo_compilation::SELF_SUBCOMMANDS
                .iter()
                .position(|name| *name == "method")
                .unwrap()
        );
        assert_eq!(option.native_object_reference_count(), references);
        observed.check(vm, &context).unwrap();
        if let Some(mode) = self.mutation {
            let first = if mode == 2 {
                let _ = vm.refuse_host_command("self-frame first Host".into());
                Some(vm.execution_refusal.clone().unwrap())
            } else {
                None
            };
            let original = &mut vm.oo.call_stack[observed.position];
            let saved_index = original.index;
            let saved_chain = if mode == 0 {
                Some(std::mem::replace(&mut original.chain, Vec::new().into()))
            } else {
                original.index = original.chain.len();
                None
            };
            let error = observed.check(vm, &context).unwrap_err();
            let cause = vm.execution_refusal.clone().unwrap();
            if let Some(first) = first {
                assert_eq!(cause, first);
            } else {
                assert_eq!(
                    cause,
                    NativeExecutionError::ValueAccessRefusal(
                        NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "original OO self method frame",
                        ),
                    ),
                );
            }
            let original = &mut vm.oo.call_stack[observed.position];
            original.index = saved_index;
            if let Some(chain) = saved_chain {
                let displaced = std::mem::replace(&mut original.chain, chain);
                drop(displaced);
            }
            assert_eq!(chain.strong_count(), chain_count);
            assert!(observed.is_current(vm));
            assert!(observed.check(vm, &context).is_err());
            assert_eq!(vm.execution_refusal, Some(cause));
            assert!(context.finish(vm).is_err());
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let expected = super::super::frame_object_name(vm, current_method(vm).unwrap());
        let object = super::super::cmd_self(vm, &[]);
        assert_eq!(object.code, Code::Ok);
        assert!(object.result.is_same_object(&expected));
        let method = super::super::cmd_self(vm, &[option]);
        assert_eq!(method.code, Code::Ok);
        assert_eq!(
            method.result.resident_string_bytes().unwrap().as_ref(),
            b"m"
        );
        observed.check(vm, &context).unwrap();
        assert_eq!(chain.strong_count(), chain_count);
        context.finish(vm).unwrap();
        ok(Value::string("SELF"))
    }
}
fn provided(mode: Option<u8>) -> (Vm, Rc<Cell<usize>>) {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.1"));
    let hits = Rc::new(Cell::new(0));
    vm.register_command(
        "self_frame_probe",
        Command::Native(Rc::new(Probe {
            mutation: mode,
            hits: Rc::clone(&hits),
        })),
    );
    let result = vm
        .eval_source("oo::class create C {method m {} {self_frame_probe}}; C create obj")
        .unwrap();
    assert_eq!(result.code, Code::Ok);
    (vm, hits)
}
#[test]
fn original_self_report_reacquires_the_same_entered_frame_without_method_payload_holds() {
    let (mut vm, hits) = provided(None);
    let result = vm.invoke_command("obj", &[Value::string("m")]);
    assert_eq!(result.code, Code::Ok);
    assert_eq!(
        result.result.resident_string_bytes().unwrap().as_ref(),
        b"SELF"
    );
    assert_eq!(hits.get(), 1);
    assert!(vm.oo.call_stack.is_empty());
    assert!(vm.execution_refusal.is_none());
}
#[test]
fn original_self_report_refuses_chain_or_index_withdrawal_and_keeps_the_first_host_cause() {
    // Software mutation follows an actual checked option read in a real Method frame.
    for mode in [0, 1, 2] {
        let (mut vm, hits) = provided(Some(mode));
        let result = vm.invoke_command("obj", &[Value::string("m")]);
        assert_eq!(result.code, Code::Error);
        assert_eq!(hits.get(), 1);
        assert!(vm.execution_refusal.is_some());
        assert!(vm.oo.call_stack.is_empty());
    }
}
