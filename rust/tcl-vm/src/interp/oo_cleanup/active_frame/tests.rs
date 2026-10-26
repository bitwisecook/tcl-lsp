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

//! Actual physical activation and callback controls; no Native frame equivalence.
use super::*;
use crate::command::{Command, NativeCommand};
use crate::interp::{InterpId, ParkedFlow, ok};
use crate::value::Value;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::{Code, Completion};

fn provided() -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.1"))
}
fn push(vm: &mut Vm, namespace: tcl_runtime_api::NsId) {
    vm.push_ns_eval_token_frame(namespace, Vec::new());
}
fn child(vm: &mut Vm) -> InterpId {
    let name = vm.create_child(Some("active_cleanup_child".into()), false);
    vm.child_id(&name).unwrap()
}
fn first(vm: &Vm) -> tcl_runtime_api::NativeExecutionError {
    vm.execution_refusal
        .clone()
        .expect("full entered first cause")
}

#[test]
fn original_namespace_activation_birth_survives_real_hide_selection_and_flow_transport() {

    let mut vm = provided();
    let namespace = vm.definition_namespace_token("active_transport");
    push(&mut vm, namespace);
    let original = vm.resolution_stacks.observe_activation().unwrap();
    let selected = vm.select_execution_frame(0).unwrap();
    assert!(!original.original_present(&vm.resolution_stacks));
    vm.restore_execution_frame(selected);
    assert!(original.original_present(&vm.resolution_stacks));
    let mut parked = ParkedFlow::default();
    vm.swap_flow(&mut parked);
    assert!(!original.original_present(&vm.resolution_stacks));
    assert_eq!(parked.current_namespace_token(), namespace);
    vm.swap_flow(&mut parked);
    assert!(original.original_present(&vm.resolution_stacks));
    vm.pop_call_frame();
    assert!(original.original_present(&vm.resolution_stacks));
    vm.pop_ns();
    assert!(original.caller_restored(&vm.resolution_stacks));
    assert_eq!(vm.frames.len(), 1);
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_namespace_cleanup_refuses_same_token_activation_rebirth_without_popping_it() {

    let mut vm = provided();
    let namespace = vm.definition_namespace_token("active_namespace_aba");
    let mut active = OriginalOoActivation::enter(&mut vm);
    vm.push_ns_token("active_namespace_aba", namespace);
    active.capture_namespace(&mut vm).unwrap();
    vm.pop_ns();
    vm.push_ns_token("active_namespace_aba", namespace);
    let fresh = vm.resolution_stacks.observe_activation().unwrap();
    active.retire_namespace(&mut vm, false);
    let retained = first(&vm);
    assert!(fresh.original_present(&vm.resolution_stacks));
    assert_eq!(vm.resolution_stacks.ns_stack.len(), 2);
    let result = active.finish(&mut vm, ok(Value::empty()));
    assert_eq!(result.code, Code::Error);
    assert_eq!(vm.execution_refusal, Some(retained));
    vm.pop_ns();
}

#[test]
fn original_call_frame_cleanup_refuses_real_same_level_replacement_and_keeps_new_namespace() {

    let mut vm = provided();
    let namespace = vm.definition_namespace_token("active_call_aba");
    let mut active = OriginalOoActivation::enter(&mut vm);
    push(&mut vm, namespace);
    active.capture_call_and_namespace(&mut vm).unwrap();
    vm.pop_call_frame();
    vm.pop_ns();
    push(&mut vm, namespace);
    let fresh_call = Rc::downgrade(&vm.frames.last().unwrap().activation);
    let fresh_namespace = vm.resolution_stacks.observe_activation().unwrap();
    active.retire_call(&mut vm, false);
    let retained = first(&vm);
    active.retire_namespace(&mut vm, false);
    assert!(Weak::ptr_eq(
        &fresh_call,
        &Rc::downgrade(&vm.frames.last().unwrap().activation)
    ));
    assert!(fresh_namespace.original_present(&vm.resolution_stacks));
    assert_eq!(vm.frames.len(), 2);
    assert_eq!(vm.resolution_stacks.ns_stack.len(), 2);
    let result = active.finish(&mut vm, ok(Value::empty()));
    assert_eq!(result.code, Code::Error);
    assert_eq!(vm.execution_refusal, Some(retained));
    vm.pop_call_frame();
    vm.pop_ns();
}

#[test]
fn original_parked_call_cleanup_retires_original_state_and_preserves_the_child_activation() {

    let mut vm = provided();
    let original_id = vm.cur;
    let child = child(&mut vm);
    let namespace = vm.definition_namespace_token("active_original");
    let mut active = OriginalOoActivation::enter(&mut vm);
    push(&mut vm, namespace);
    active.capture_call_and_namespace(&mut vm).unwrap();
    vm.switch_to(child);
    let child_namespace = vm.definition_namespace_token("active_child");
    push(&mut vm, child_namespace);
    let child_call = Rc::downgrade(&vm.frames.last().unwrap().activation);
    let child_activation = vm.resolution_stacks.observe_activation().unwrap();
    active.retire_call(&mut vm, false);
    active.retire_namespace(&mut vm, false);
    assert_eq!(vm.cur, child);
    assert_eq!(vm.frames.len(), 2);
    assert!(Weak::ptr_eq(
        &child_call,
        &Rc::downgrade(&vm.frames.last().unwrap().activation)
    ));
    assert!(child_activation.original_present(&vm.resolution_stacks));
    let original = vm.st_of(original_id).unwrap();
    assert_eq!(original.frames.len(), 1);
    assert_eq!(original.resolution_stacks.ns_stack.len(), 1);
    let result = active.finish(&mut vm, ok(Value::empty()));
    assert_eq!(result.code, Code::Error);
    assert_eq!(vm.execution_refusal, Some(first(&vm)));
    vm.pop_call_frame();
    vm.pop_ns();
    vm.switch_to(original_id);
}

#[test]
fn original_dispatch_cleanup_restores_captured_depth_without_decrementing_the_child() {

    let mut vm = provided();
    let original_id = vm.cur;
    let child = child(&mut vm);
    let cleanup = OriginalOoCleanup::enter(&mut vm);
    let prior = cleanup.capture_dispatch_depth(&mut vm).unwrap();
    vm.enter_oo_dispatch().unwrap();
    vm.switch_to(child);
    vm.oo_dispatch_depth = 7;
    let _ = cleanup.restore_dispatch_depth(&mut vm, prior);
    assert_eq!(vm.oo_dispatch_depth, 7);
    assert_eq!(vm.st_of(original_id).unwrap().oo_dispatch_depth, prior);
    let result = cleanup.finish_completion(&mut vm, ok(Value::empty()));
    assert_eq!(result.code, Code::Error);
    assert_eq!(vm.execution_refusal, Some(first(&vm)));
    vm.switch_to(original_id);
}

#[test]
fn original_call_cleanup_keeps_full_first_host_while_retiring_the_owned_frame_and_namespace() {

    let mut vm = provided();
    let namespace = vm.definition_namespace_token("active_first_host");
    let mut active = OriginalOoActivation::enter(&mut vm);
    push(&mut vm, namespace);
    active.capture_call_and_namespace(&mut vm).unwrap();
    let header = Rc::downgrade(&vm.frames.last().unwrap().activation);
    let _ = vm.refuse_host_command("FIRST_ACTUAL_CLEANUP".into());
    let retained = first(&vm);
    active.retire_call(&mut vm, false);
    active.retire_namespace(&mut vm, false);
    assert_eq!(vm.frames.len(), 1);
    assert_eq!(vm.resolution_stacks.ns_stack.len(), 1);
    assert!(header.upgrade().is_none());
    let result = active.finish(&mut vm, ok(Value::empty()));
    assert_eq!(result.code, Code::Error);
    assert_eq!(vm.execution_refusal, Some(retained));
}

struct ForwardObserver {
    mode: u8,
    child: InterpId,
    calls: Rc<Cell<usize>>,
}
impl NativeCommand for ForwardObserver {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        assert!(!vm.oo.call_stack.is_empty());
        match self.mode {
            0 => ok(Value::string("BODY")),
            1 => vm.refuse_host_command("FIRST_FORWARD_BODY".into()),
            2 => {
                let token = vm.current_ns_id();
                let name = vm.ns_name_bytes(token);
                vm.pop_ns();
                vm.push_ns_token(name, token);
                ok(Value::string("BODY"))
            }
            3 => {
                vm.switch_to(self.child);
                let namespace = vm.definition_namespace_token("actual_forward_child");
                push(vm, namespace);
                ok(Value::string("BODY"))
            }
            _ => unreachable!(),
        }
    }
}
#[test]
fn original_forward_callback_cleanup_uses_the_real_pushed_context_and_original_namespace() {

    for mode in 0..4 {
        let mut vm = provided();
        let original_id = vm.cur;
        let child = child(&mut vm);
        let setup = vm.eval_source("oo::class create ActiveCleanupC {forward fw active_cleanup_observer}; ActiveCleanupC create activeCleanupObject").unwrap();
        assert_eq!(setup.code, Code::Ok);
        let calls = Rc::new(Cell::new(0));
        vm.register_command(
            "active_cleanup_observer",
            Command::Native(Rc::new(ForwardObserver {
                mode,
                child,
                calls: Rc::clone(&calls),
            })),
        );
        // These assertions inspect retirement while its genuine operation is
        // still entered; the outer host boundary transfers its cause afterward.
        let episode =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
        let reached = vm.try_invoke_command("activeCleanupObject", &[Value::string("fw")]);
        assert_eq!(calls.get(), 1);
        if mode == 0 {
            let result = reached.unwrap();
            assert_eq!(result.code, Code::Ok);
            assert_eq!(
                result.result.resident_string_bytes().unwrap().as_ref(),
                b"BODY"
            );
            assert!(vm.execution_refusal.is_none());
        } else {
            assert_eq!(reached.unwrap_err(), first(&vm));
            assert!(vm.execution_refusal.is_some());
        }
        assert_eq!(vm.st_of(original_id).unwrap().oo.call_stack.len(), 0);
        assert_eq!(vm.st_of(original_id).unwrap().oo_dispatch_depth, 0);
        if mode == 3 {
            assert_eq!(vm.cur, child);
            assert_eq!(vm.frames.len(), 2);
            assert_eq!(
                vm.st_of(original_id)
                    .unwrap()
                    .resolution_stacks
                    .ns_stack
                    .len(),
                1
            );
            vm.pop_call_frame();
            vm.pop_ns();
            vm.switch_to(original_id);
        } else if mode == 2 {
            assert_eq!(vm.resolution_stacks.ns_stack.len(), 2);
            vm.pop_ns();
        } else {
            assert_eq!(vm.resolution_stacks.ns_stack.len(), 1);
        }
        let retained = vm.execution_refusal.clone();
        drop(episode);
        assert_eq!(vm.take_host_boundary_refusal(), retained);
        assert!(vm.execution_refusal.is_none());
    }
}

struct SamePositionObserver {
    nested: Cell<bool>,
    original: RefCell<Option<OriginalOoCleanupFrame>>,
    first_cause: RefCell<Option<tcl_runtime_api::NativeExecutionError>>,
    calls: Cell<usize>,
}
impl NativeCommand for SamePositionObserver {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        if self.nested.get() {
            let selected = self.original.borrow().as_ref().unwrap().clone();
            let position = selected.position();
            let fresh = Rc::downgrade(&vm.oo.call_stack.last().unwrap().context_birth);
            assert_eq!(position + 1, vm.oo.call_stack.len());
            assert!(!selected.matches_birth(vm.oo.call_stack.last().unwrap()));
            let mut old = OriginalOoActivation::enter(vm);
            old.method = Some(selected);
            old.retire_method(vm);
            assert!(Weak::ptr_eq(
                &fresh,
                &Rc::downgrade(&vm.oo.call_stack.last().unwrap().context_birth)
            ));
            return old.finish(vm, ok(Value::empty()));
        }
        let selected = vm.oo.call_stack.pop().unwrap();
        *self.original.borrow_mut() = Some(OriginalOoCleanupFrame::capture(
            vm.oo.call_stack.len(),
            &selected,
        ));
        self.nested.set(true);
        let reached = vm.try_invoke_command("samePositionObject", &[Value::string("fw")]);
        self.nested.set(false);
        vm.oo.call_stack.push(selected);
        match reached {
            Ok(result) => result,
            Err(error) => {
                assert_eq!(error, first(vm));
                assert!(vm.execution_refusal.is_some());
                *self.first_cause.borrow_mut() = Some(error.clone());
                crate::command::completion_from_tcl_error(
                    vm,
                    crate::TclError::from_execution_failure(error),
                )
            }
        }
    }
}
#[test]
fn original_method_cleanup_refuses_a_genuinely_new_same_position_context_without_retiring_it() {

    let mut vm = provided();
    let setup = vm.eval_source("oo::class create SamePositionC {forward fw same_position_observer}; SamePositionC create samePositionObject").unwrap();
    assert_eq!(setup.code, Code::Ok);
    let observer = Rc::new(SamePositionObserver {
        nested: Cell::new(false),
        original: RefCell::new(None),
        first_cause: RefCell::new(None),
        calls: Cell::new(0),
    });
    vm.register_command("same_position_observer", Command::Native(observer.clone()));
    let error = vm
        .try_invoke_command("samePositionObject", &[Value::string("fw")])
        .unwrap_err();
    assert_eq!(
        error,
        observer
            .first_cause
            .borrow()
            .clone()
            .expect("nested invocation retains the complete first cause"),
    );
    assert_eq!(observer.calls.get(), 2);
    assert!(vm.execution_refusal.is_none());
    assert!(vm.oo.call_stack.is_empty());
    assert_eq!(vm.resolution_stacks.ns_stack.len(), 1);
    assert_eq!(vm.oo_dispatch_depth, 0);
}

struct ProcedureObserver {
    mode: u8,
    child: InterpId,
    calls: Rc<Cell<usize>>,
    activation: RefCell<Option<Weak<ActivationIdentity>>>,
}
impl NativeCommand for ProcedureObserver {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(vm.frames.len(), 2);
        *self.activation.borrow_mut() = Some(Rc::downgrade(&vm.frames.last().unwrap().activation));
        match self.mode {
            0 => ok(Value::string("BODY")),
            1 => vm.refuse_host_command("first actual OO procedure body".to_owned()),
            2 => {
                vm.switch_to(self.child);
                let namespace = vm.definition_namespace_token("actual_proc_child");
                push(vm, namespace);
                ok(Value::string("BODY"))
            }
            3 => {
                let namespace = vm.current_ns_id();
                vm.pop_call_frame();
                vm.pop_ns();
                push(vm, namespace);
                assert!(!Weak::ptr_eq(
                    self.activation.borrow().as_ref().unwrap(),
                    &Rc::downgrade(&vm.frames.last().unwrap().activation),
                ));
                ok(Value::string("BODY"))
            }
            _ => unreachable!(),
        }
    }
}
#[test]
fn original_procedure_epilogue_and_abort_retire_only_the_entered_oo_frame_and_namespace() {

    // Actual public method invocation owns its Frame; saved cleanup metadata adds no Native role.
    for mode in 0..4 {
        let mut vm = provided();
        let original = vm.cur;
        let child = child(&mut vm);
        let setup = vm.eval_source("oo::class create ActiveProcC {method run {} {active_proc_observer; return BODY}}; ActiveProcC create activeProcObject").unwrap();
        assert_eq!(setup.code, Code::Ok);
        let calls = Rc::new(Cell::new(0));
        let observer = Rc::new(ProcedureObserver {
            mode,
            child,
            calls: Rc::clone(&calls),
            activation: RefCell::new(None),
        });
        vm.register_command("active_proc_observer", Command::Native(observer.clone()));
        // These assertions inspect retirement while its genuine operation is
        // still entered; the outer host boundary transfers its cause afterward.
        let episode =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
        let result = vm.try_invoke_command("activeProcObject", &[Value::string("run")]);
        assert_eq!(calls.get(), 1);
        assert_eq!(
            observer
                .activation
                .borrow()
                .as_ref()
                .unwrap()
                .strong_count(),
            0
        );
        let original_state = vm.st_of(original).unwrap();
        assert!(original_state.oo.call_stack.is_empty());
        assert_eq!(original_state.oo_dispatch_depth, 0);
        if mode == 0 {
            let result = result.unwrap();
            assert_eq!(result.code, Code::Ok);
            assert_eq!(
                result.result.resident_string_bytes().unwrap().as_ref(),
                b"BODY"
            );
            assert!(vm.execution_refusal.is_none());
        } else {
            assert_eq!(result.unwrap_err(), first(&vm));
            assert!(vm.execution_refusal.is_some());
        }
        if mode == 2 {
            assert_eq!(vm.cur, child);
            assert_eq!(vm.frames.len(), 2);
            assert_eq!(vm.activation_depth, 0, "shared engine driver has unwound");
            vm.in_interp(original, |vm| assert_eq!(vm.activation_depth, 0));
            assert_eq!(vm.st_of(original).unwrap().frames.len(), 1);
            assert_eq!(
                vm.st_of(original).unwrap().resolution_stacks.ns_stack.len(),
                1
            );
            vm.pop_call_frame();
            vm.pop_ns();
            vm.switch_to(original);
        } else if mode == 3 {
            assert_eq!(vm.frames.len(), 2);
            assert_eq!(vm.resolution_stacks.ns_stack.len(), 2);
            vm.pop_call_frame();
            vm.pop_ns();
        } else {
            assert_eq!(vm.frames.len(), 1);
            assert_eq!(vm.resolution_stacks.ns_stack.len(), 1);
        }
        let retained = vm.execution_refusal.clone();
        drop(episode);
        assert_eq!(vm.take_host_boundary_refusal(), retained);
        assert!(vm.execution_refusal.is_none());
    }
}
