// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual activation callbacks and original physical cleanup; software controls only.
use super::*;
use crate::command::NativeCommand;
use crate::frame::ActivationIdentity;
use crate::interp::InterpId;
use std::cell::{Cell, RefCell};
use std::rc::Weak;

fn native() -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl8.6"))
}
fn select(vm: &mut Vm, id: InterpId) {
    crate::interp::oo_cleanup::select_cleanup_fixture_interpreter(vm, id);
}
fn child(vm: &mut Vm) -> InterpId {
    let name = vm.create_child(Some("activation_cleanup_child".into()), false);
    let id = vm.child_id(&name).unwrap();
    vm.in_interp(id, |vm| {
        let ns = vm.definition_namespace_token("original_child_namespace");
        vm.push_ns_eval_token_frame(ns, Vec::new());
    });
    id
}
struct Later(Rc<Cell<usize>>);
impl NativeCommand for Later {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.0.set(self.0.get() + 1);
        ok(Value::string("LATER"))
    }
}
struct LeaveChild {
    child: InterpId,
    calls: Rc<Cell<usize>>,
    original_call: Rc<RefCell<Option<Weak<ActivationIdentity>>>>,
    prior_host: bool,
}
impl NativeCommand for LeaveChild {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        NativeOperationScope::current_entered(vm).unwrap().unwrap();
        self.calls.set(self.calls.get() + 1);
        *self.original_call.borrow_mut() = Some(Rc::downgrade(&vm.retained_frame_identity(false)));
        if self.prior_host {
            let refused = vm.refuse_host_command("FIRST_ORIGINAL_ACTIVATION".into());
            drop(refused);
        }
        select(vm, self.child);
        ok(Value::string("UNPUBLISHED"))
    }
}
fn install(
    vm: &mut Vm,
    child: InterpId,
    prior_host: bool,
) -> (
    Rc<Cell<usize>>,
    Rc<Cell<usize>>,
    Rc<RefCell<Option<Weak<ActivationIdentity>>>>,
) {
    let calls = Rc::new(Cell::new(0));
    let later = Rc::new(Cell::new(0));
    let original = Rc::new(RefCell::new(None));
    vm.register_native_command(
        "leave_child",
        Rc::new(LeaveChild {
            child,
            calls: calls.clone(),
            original_call: original.clone(),
            prior_host,
        }),
    );
    vm.register_native_command("later", Rc::new(Later(later.clone())));
    (calls, later, original)
}
fn preserved(vm: &mut Vm, original: InterpId, child: InterpId, birth: &Weak<ActivationIdentity>) {
    assert_eq!(vm.cur_interp(), child);
    assert_eq!(vm.current_level(), 1);
    assert!(Weak::ptr_eq(
        birth,
        &Rc::downgrade(&vm.retained_frame_identity(false))
    ));
    let current_ns = vm.current_ns_id();
    assert_ne!(current_ns, tcl_core_types::ROOT_NS);
    vm.in_interp(original, |vm| {
        assert_eq!(vm.current_level(), 0);
        assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
        assert_eq!(vm.recursion_depth(), 0);
        assert!(vm.pending_exec_leave.is_none());
        assert!(vm.exec_step_scopes.is_empty());
    });
    assert_eq!(vm.cur_interp(), child);
    assert_eq!(vm.current_ns_id(), current_ns);
}
#[test]
fn original_activation_abort_retires_userproc_in_parent_and_preserves_child_and_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for prior_host in [false, true] {
        let mut vm = native();
        let original = vm.cur_interp();
        let child = child(&mut vm);
        let birth = vm.in_interp(child, |vm| {
            Rc::downgrade(&vm.retained_frame_identity(false))
        });
        let (calls, later, frame) = install(&mut vm, child, prior_host);
        assert_eq!(
            vm.try_eval_source("proc original_cleanup {owned} {leave_child; later}")
                .unwrap()
                .code,
            Code::Ok
        );
        let operand = Value::new_native_string_bytes(b"OWNED_LOCAL".as_slice());
        let baseline = operand.native_object_reference_count();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let error = vm
            .try_invoke_command("original_cleanup", std::slice::from_ref(&operand))
            .unwrap_err();
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(error, first);
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
        assert_eq!(calls.get(), 1);
        assert_eq!(later.get(), 0);
        assert_eq!(frame.borrow().as_ref().unwrap().strong_count(), 0);
        assert_eq!(operand.native_object_reference_count(), baseline);
        preserved(&mut vm, original, child, &birth);
        assert_eq!(vm.execution_refusal, Some(first));
        select(&mut vm, original);
    }
}
#[test]
fn original_activation_abort_keeps_apply_publication_and_oo_cleanup_on_original_target() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for oo in [false, true] {
        let mut vm = native();
        let original = vm.cur_interp();
        let child = child(&mut vm);
        let birth = vm.in_interp(child, |vm| {
            Rc::downgrade(&vm.retained_frame_identity(false))
        });
        let (calls, later, frame) = install(&mut vm, child, true);
        if oo {
            assert_eq!(vm.try_eval_source("oo::class create ActivationCleanupClass {method run {} {leave_child; later}}; ActivationCleanupClass create activationCleanupObject").unwrap().code, Code::Ok);
        }
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let reached = if oo {
            vm.try_invoke_command("activationCleanupObject", &[Value::string("run")])
        } else {
            vm.try_eval_source("apply {{} {leave_child; later}}")
        };
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(reached.unwrap_err(), first);
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
        assert_eq!(calls.get(), 1);
        assert_eq!(later.get(), 0);
        assert_eq!(frame.borrow().as_ref().unwrap().strong_count(), 0);
        preserved(&mut vm, original, child, &birth);
        let parent = vm.st_of(original).unwrap();
        assert_eq!(parent.oo.call_stack.len(), 0);
        assert!(parent.pending.procedure_call.is_none());
        select(&mut vm, original);
    }
}
struct Reborn {
    calls: Rc<Cell<usize>>,
    fresh: Rc<RefCell<Option<Weak<ActivationIdentity>>>>,
}
impl NativeCommand for Reborn {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        let ns = vm.current_ns_id();
        vm.pop_call_frame();
        vm.pop_ns();
        vm.push_ns_eval_token_frame(ns, Vec::new());
        *self.fresh.borrow_mut() = Some(Rc::downgrade(&vm.retained_frame_identity(false)));
        let refused = vm.refuse_host_command("FIRST_REBORN_ACTIVATION".into());
        drop(refused);
        ok(Value::string("REPLACEMENT"))
    }
}
#[test]
fn original_activation_abort_preserves_callback_reborn_call_and_namespace_in_parent() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = native();
    let calls = Rc::new(Cell::new(0));
    let fresh = Rc::new(RefCell::new(None));
    vm.register_native_command(
        "reborn",
        Rc::new(Reborn {
            calls: calls.clone(),
            fresh: fresh.clone(),
        }),
    );
    assert_eq!(
        vm.try_eval_source("proc original_rebirth {} {reborn}")
            .unwrap()
            .code,
        Code::Ok
    );
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let reached = vm.try_invoke_command("original_rebirth", &[]);
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(reached.unwrap_err(), first);
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(calls.get(), 1);
    assert_eq!(vm.current_level(), 1);
    assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
    assert!(Weak::ptr_eq(
        fresh.borrow().as_ref().unwrap(),
        &Rc::downgrade(&vm.retained_frame_identity(false))
    ));
    assert!(vm.pending_exec_leave.is_none());
    vm.pop_call_frame();
    vm.pop_ns();
    assert_eq!(vm.execution_refusal, Some(first));
}
#[test]
fn original_activation_unwind_keeps_saved_target_after_last_frame_before_leave_callback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = native();
    let original = vm.cur_interp();
    let child = child(&mut vm);
    let birth = vm.in_interp(child, |vm| {
        Rc::downgrade(&vm.retained_frame_identity(false))
    });
    let (calls, later, _) = install(&mut vm, child, false);
    assert_eq!(vm.try_eval_source("proc original_leave {} {return FINISHED}; trace add execution original_leave leave leave_child").unwrap().code, Code::Ok);
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let reached = vm.try_invoke_command("original_leave", &[]);
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(reached.unwrap_err(), first);
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(calls.get(), 1);
    assert_eq!(later.get(), 0);
    preserved(&mut vm, original, child, &birth);
    select(&mut vm, original);
}
#[test]
fn original_activation_empty_tailcall_cleanup_uses_issued_parent_and_preserves_child_pending() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = native();
    let original = vm.cur_interp();
    let child = child(&mut vm);
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    vm.pending_exec_leave = Some(Vm::original_scope_context(scope.clone()));
    let request = TailcallReq {
        namespace: Value::string("::"),
        words: vec![Value::string("later")],
        original_list: None,
        original_scope: OriginalExecutionOwner::Entered(scope.clone()),
    };
    select(&mut vm, child);
    let child_scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    vm.pending_exec_leave = Some(Vm::original_scope_context(child_scope));
    let birth = Rc::downgrade(&vm.retained_frame_identity(false));
    let mut acts = Vec::new();
    let reached = vm.dispatch_tailcall(&mut acts, request).unwrap();
    assert_eq!(reached.code, Code::Error);
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert!(acts.is_empty());
    assert_eq!(vm.cur_interp(), child);
    assert!(vm.pending_exec_leave.is_some());
    assert!(vm.st_of(original).unwrap().pending_exec_leave.is_none());
    assert!(Weak::ptr_eq(
        &birth,
        &Rc::downgrade(&vm.retained_frame_identity(false))
    ));
    select(&mut vm, original);
}
#[test]
fn original_activation_positive_procedure_keeps_existing_owned_roles_and_no_saved_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = native();
    assert_eq!(
        vm.try_eval_source("proc original_positive {owned} {return $owned}")
            .unwrap()
            .code,
        Code::Ok
    );
    let value = Value::new_native_string_bytes(b"POSITIVE".as_slice());
    let baseline = value.native_object_reference_count();
    let result = vm
        .try_invoke_command("original_positive", std::slice::from_ref(&value))
        .unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(
        result.result.resident_string_bytes().unwrap().as_ref(),
        b"POSITIVE"
    );
    assert_eq!(vm.current_level(), 0);
    assert_eq!(vm.current_ns_id(), tcl_core_types::ROOT_NS);
    assert!(vm.pending_exec_leave.is_none());
    assert!(vm.exec_step_scopes.is_empty());
    assert!(vm.execution_refusal.is_none());
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    drop(result);
    assert_eq!(
        vm.try_eval_source("set cleanup_result CLEAR").unwrap().code,
        Code::Ok
    );
    assert_eq!(value.native_object_reference_count(), baseline);
}
