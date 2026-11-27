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

//! Behaviour tests moved from `tcl-vm`'s `retained_activation::tests`.

use crate::prelude::*;
use std::cell::RefCell;
use tcl_runtime_api::retained_activation::{RetainedActivationRefusal, ScheduledCallbackOutcome};
use tcl_vm::{NativeCommand, RetainedActivation};

fn ok(value: Value) -> Completion<Value> {
    Completion::new(Code::Ok, value, Value::empty())
}

fn vm() -> Vm {
    let mut vm = tcl_vm::test_support::native_vm_with_output(Box::new(std::io::sink()));
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::default(),
    ));
    vm.install_irules_timer_simulation();
    vm
}

fn eval(vm: &mut Vm, source: &str) -> Completion<Value> {
    let completion = vm.try_eval_source(source).expect("actual engine execution");
    assert_eq!(completion.code, Code::Ok, "{:?}", completion.result);
    completion
}

fn root_context(vm: &mut Vm) {
    assert!(crate::fixture::call(vm, "::tmm::_timer_rule_loaded", &[Value::empty()]).is_ok());
    assert!(
        crate::fixture::call(
            vm,
            "::tmm::_timer_context_enter",
            &[Value::string("RULE_INIT"), Value::empty()]
        )
        .is_ok()
    );
}

#[test]
fn ordinary_tcl_after_keeps_its_global_callback_frame() {
    let mut vm = tcl_vm::test_support::native_vm_with_output(Box::new(std::io::sink()));
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::default(),
    ));
    eval(
        &mut vm,
        "proc schedule {} {set local 1; after 0 {set observed [info exists local]}}; schedule; update",
    );
    assert_eq!(
        vm.get_var("observed").unwrap().string_bytes().as_ref(),
        b"0"
    );
}

/// Keeps the activation of the frame that calls it.
struct Capture(Rc<RefCell<Option<RetainedActivation>>>);

impl NativeCommand for Capture {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        *self.0.borrow_mut() = Some(vm.retain_current_activation());
        ok(Value::empty())
    }
}

#[test]
fn retained_frame_rejects_reused_levels_foreign_vms_and_deleted_interpreters() {
    let mut original = vm();
    let mut foreign = vm();
    let captured = Rc::new(RefCell::new(None));
    original.register_native_command("capture", Rc::new(Capture(Rc::clone(&captured))));
    eval(
        &mut original,
        "proc first {} {capture}; proc second {} {}; first",
    );
    let capability = captured.borrow_mut().take().unwrap();
    assert!(matches!(
        foreign.eval_retained_activation(&capability, "set changed 1"),
        ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::ForeignRuntime)
    ));
    eval(&mut original, "second");
    assert!(matches!(
        original.eval_retained_activation(&capability, "set changed 1"),
        ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame)
    ));
    assert!(original.get_var("changed").is_none());
    eval(&mut original, "interp create worker");
    assert!(original.register_child_native_command(
        "worker",
        "capture",
        Rc::new(Capture(Rc::clone(&captured)))
    ));
    eval(&mut original, "interp eval worker capture");
    let child_activation = captured.borrow_mut().take().unwrap();
    eval(&mut original, "interp delete worker");
    assert!(matches!(
        original.eval_retained_activation(&child_activation, "set changed 1"),
        ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredInterpreter)
    ));
}

#[test]
fn scheduled_connection_uses_original_renamed_coroutine_and_expires_on_deletion() {
    let mut vm = vm();
    eval(
        &mut vm,
        "::tmm::_timer_rule_loaded {}; ::tmm::_timer_context_enter HTTP_REQUEST {}; proc flow {} {::tmm::_timer_anchor; set local 10; after 5 {incr local}; yield; yield}; coroutine connection flow; rename connection moved",
    );
    let reports = vm.advance_irules_time(5);
    assert_eq!(reports.len(), 1);
    let ScheduledCallbackOutcome::Guest(completion) = &reports[0].outcome else {
        panic!("{:?}", reports[0]);
    };
    assert_eq!(completion.result.string_bytes().as_ref(), b"11");
    eval(
        &mut vm,
        "coroprobe moved after 5 {set local REPLACED}; rename moved {}; coroutine moved flow",
    );
    let reports = vm.advance_irules_time(5);
    assert!(reports.iter().any(|report| matches!(
        report.outcome,
        ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame)
    )));
    assert_eq!(
        eval(&mut vm, "coroprobe moved set local")
            .result
            .string_bytes()
            .as_ref(),
        b"11"
    );
}

#[test]
fn timer_cancellation_reentrancy_and_rule_replacement_are_deterministic() {
    let mut vm = vm();
    root_context(&mut vm);
    eval(
        &mut vm,
        "set order {}; after 0 {lappend order first; after 0 {lappend order nested}}; after 0 {lappend order second}; after 2 -periodic {lappend order periodic; after cancel -current}",
    );
    assert_eq!(vm.advance_irules_time(0).len(), 2);
    assert_eq!(
        vm.get_var("order").unwrap().string_bytes().as_ref(),
        b"first second"
    );
    assert_eq!(vm.advance_irules_time(2).len(), 2);
    assert_eq!(
        vm.get_var("order").unwrap().string_bytes().as_ref(),
        b"first second nested periodic"
    );
    assert!(vm.advance_irules_time(100).is_empty());
    eval(
        &mut vm,
        "after 0 {set stale MUST_NOT_RUN}; ::tmm::_timer_rule_loaded {}",
    );
    let reports = vm.advance_irules_time(0);
    assert!(matches!(
        reports[0].outcome,
        ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredRule)
    ));
    assert!(vm.get_var("stale").is_none());
}

struct EchoOriginal;
impl NativeCommand for EchoOriginal {
    fn invoke(&self, _vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        ok(args.first().cloned().unwrap_or_else(Value::empty))
    }
}

#[test]
fn retained_callback_and_timer_execute_the_original_non_unicode_source() {
    let mut vm = vm();
    vm.register_native_command("echo_original", Rc::new(EchoOriginal));
    let source = Value::from_native_string_bytes(b"echo_original \xff\0tail".as_slice());
    let activation = vm.retain_current_activation();
    let ScheduledCallbackOutcome::Guest(completion) =
        vm.eval_retained_activation_value(&activation, &source)
    else {
        panic!("original byte callback must execute");
    };
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"\xff\0tail");
    root_context(&mut vm);
    assert!(crate::fixture::call(&mut vm, "after", &[Value::int(0), source.clone()]).is_ok());
    let reports = vm.advance_irules_time(0);
    let ScheduledCallbackOutcome::Guest(completion) = &reports[0].outcome else {
        panic!("{:?}", reports[0]);
    };
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"\xff\0tail");
}

#[test]
fn retained_activation_policy_includes_the_authored_name_provider() {
    let mut vm = vm();
    let source = tcl_dialect::DialectProfile::irules();
    let host = tcl_vm::test_support::profile("tcl9.0");
    vm.set_dialect_profile(source);
    assert!(vm.set_native_engine_profile(host));
    let activation = vm.retain_current_activation();
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(matches!(
        vm.eval_retained_activation(&activation, "MUST_NOT_RUN"),
        ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::ChangedPolicy)
    ));
}

struct Abrupt(Completion<Value>);
impl NativeCommand for Abrupt {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        self.0.clone()
    }
}

#[test]
fn scheduled_guest_options_and_host_refusal_remain_separate() {
    let mut vm = vm();
    root_context(&mut vm);
    let result = Value::from_string_bytes(b"raw \xff\0result".as_slice());
    let options = Value::list(vec![
        Value::string("-code"),
        Value::int(1),
        Value::string("-level"),
        Value::int(3),
        Value::string("-errorcode"),
        Value::from_string_bytes(b"RAW \xfe\0code".as_slice()),
        Value::string("-custom"),
        Value::string("retained"),
    ]);
    vm.register_native_command(
        "abrupt",
        Rc::new(Abrupt(Completion::new(
            Code::Other(7),
            result.clone(),
            options.clone(),
        ))),
    );
    eval(&mut vm, "after 0 abrupt");
    let reports = vm.advance_irules_time(0);
    let ScheduledCallbackOutcome::Guest(completion) = &reports[0].outcome else {
        panic!("{:?}", reports[0]);
    };
    assert_eq!(completion.code, Code::Other(7));
    assert_eq!(completion.result.string_bytes(), result.string_bytes());
    assert_eq!(completion.options.string_bytes(), options.string_bytes());
    vm.register("refuse", |vm, _| {
        vm.refuse_host_command("reached callback backend refusal".into())
    });
    eval(
        &mut vm,
        "after 0 {set before 1; catch refuse result; set continued 1}; after 0 {set later 1}",
    );
    let reports = vm.advance_irules_time(0);
    assert_eq!(reports.len(), 1);
    assert!(matches!(
        reports[0].outcome,
        ScheduledCallbackOutcome::Host(_)
    ));
    assert_eq!(vm.get_var("before").unwrap().string_bytes().as_ref(), b"1");
    assert!(vm.get_var("continued").is_none());
    assert!(vm.get_var("later").is_none());
}

#[test]
fn cancellation_cannot_remove_another_workers_timer() {
    let mut vm = vm();
    let enter = "::tmm::_timer_rule_loaded {}; ::tmm::_timer_context_enter RULE_INIT {}";
    eval(&mut vm, "interp create one; interp create two");
    let id = eval(
        &mut vm,
        &format!("interp eval one {{{enter}; after 0 {{set own 1}}}}"),
    )
    .result;
    eval(&mut vm, &format!("interp eval two {{{enter}}}"));
    let cancelled = crate::fixture::call(
        &mut vm,
        "interp",
        &[
            Value::string("eval"),
            Value::string("two"),
            Value::list(vec![Value::string("after"), Value::string("cancel"), id]),
        ],
    );
    assert!(cancelled.is_ok(), "{cancelled:?}");
    assert_eq!(vm.advance_irules_time(0).len(), 1);
    assert_eq!(
        eval(&mut vm, "interp eval one {set own}")
            .result
            .string_bytes()
            .as_ref(),
        b"1"
    );
    assert_eq!(
        eval(&mut vm, "interp eval two {info exists own}")
            .result
            .string_bytes()
            .as_ref(),
        b"0"
    );
}
