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

use super::*;
use crate::interp::{default_host, ExceptionState, Interp, WeakInterp};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct RetiredMember {
    interpreter: WeakInterp,
    role: usize,
    mode: u8,
    arm: Rc<Cell<bool>>,
    events: Rc<RefCell<Vec<(usize, obj::TclSize)>>>,
    published: Rc<Cell<bool>>,
}
extern "C" fn retire(value: *mut TclObj) {
    let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut RetiredMember) };
    if !state.arm.get() {
        return;
    }
    state
        .events
        .borrow_mut()
        .push((state.role, unsafe { (*value).ref_count }));
    let Some(mut interpreter) = state.interpreter.upgrade() else {
        return;
    };
    // A real member/header free may inspect the current physical roles;
    // neither the stack slot nor innerContext can be borrowed by its caller.
    let (header, inner) = {
        let stack = interpreter.error_stack.borrow_mut();
        (stack.original_header(), stack.original_inner_context())
    };
    state.published.set(header.is_some() && inner.is_some());
    if state.role == 1 || interpreter.native_execution_refusal().is_some() {
        return;
    }
    match state.mode {
        1 => {
            interpreter.refuse_host_command("original prepared stack first cause");
        }
        2 => {
            let saved = interpreter.runtime_context();
            let mut changed = saved.clone();
            changed.packages = vec![("original-prepared-stack".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&saved).unwrap();
        }
        3 => {
            let saved = interpreter
                .save_native_variable_trace_result(false)
                .unwrap()
                .unwrap();
            interpreter.mark_error_stack_reset();
            interpreter
                .restore_native_variable_trace_result(saved)
                .unwrap();
        }
        _ => {}
    }
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalPreparedStackMember".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn native(name: &str) -> Interp {
    let environment = tcl_registry::model::ingress::resolve_known_environment(name).unwrap();
    Interp::with_native_core(
        default_host(),
        environment.unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn original(
    interpreter: &Interp,
    role: usize,
    mode: u8,
    arm: &Rc<Cell<bool>>,
    events: &Rc<RefCell<Vec<(usize, obj::TclSize)>>>,
    published: &Rc<Cell<bool>>,
) -> obj::Owned {
    let state = Box::new(RetiredMember {
        interpreter: interpreter.downgrade(),
        role,
        mode,
        arm: arm.clone(),
        events: events.clone(),
        published: published.clone(),
    });
    obj::Owned::fresh(obj::alloc_typed(
        &ORIGINAL,
        Box::into_raw(state) as usize as u64,
    ))
}
fn clear(interpreter: &Interp, arm: &Cell<bool>) {
    arm.set(false);
    let old = interpreter.error_stack.replace(NativeErrorStack::default());
    drop(old);
    let old = interpreter.exc.replace(ExceptionState::default());
    drop(old);
}

#[test]
fn original_prepared_stack_unavailable_releases_do_not_issue_or_retire_private_headers() {
    // naming.error.original-private-error-stack-header
    // docs/design/analysis/name-resolution-proofs/error-original-private-error-stack-header.md
    // These real releases lack the private stack; they cannot borrow the modern positive premise.
    for name in ["tcl8.4", "tcl8.5"] {
        crate::counters::reset();
        {
            let interpreter = native(name);
            let arm = Rc::new(Cell::new(true));
            let events = Rc::new(RefCell::new(Vec::new()));
            let published = Rc::new(Cell::new(false));
            let member = original(&interpreter, 0, 0, &arm, &events, &published);
            let scope =
                super::super::native_operation_currency::NativeOperationScope::enter(&interpreter)
                    .unwrap();
            let recipe = interpreter
                .native_invocation_dialect()
                .native_error_objects_protocol()
                .unwrap();
            assert!(!recipe.has_error_stack());
            interpreter.configure_native_error_stack_checked().unwrap();
            interpreter.error_stack_log(b"UNAVAILABLE STACK").unwrap();
            assert!(interpreter.error_stack.borrow().original_header().is_none());
            assert!(interpreter
                .error_stack
                .borrow()
                .original_inner_context()
                .is_none());
            assert!(interpreter.error_stack.borrow().protocol.is_none());
            let refused = interpreter.seed_original_error_stack(member.as_ptr());
            let cause = refused.unwrap_err().native_access_refusal();
            assert!(cause.is_some());
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 1);
            assert_eq!(
                obj::obj_type_ptr(member.as_ptr()),
                &ORIGINAL as *const obj::TclObjType
            );
            assert!(events.borrow().is_empty());
            assert!(!published.get());
            // The refusal is the operation's retained first cause.
            assert_eq!(
                scope.currency().ensure_current_or_refuse(),
                Err(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    cause.unwrap()
                ))
            );
            clear(&interpreter, &arm);
            drop(member);
            drop(scope);
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_prepared_stack_members_retire_outside_borrows_and_stop_on_episode_aba() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software member frees/tickets; no external Native stack chronology is inferred.
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for operation in 0..2 {
            for mode in 0..4 {
                crate::counters::reset();
                {
                    let mut interpreter = native(name);
                    interpreter.set_result_bytes(b"RETAINED RESULT");
                    let result = interpreter.result.get();
                    interpreter.error_line.set(37);
                    let arm = Rc::new(Cell::new(true));
                    let events = Rc::new(RefCell::new(Vec::new()));
                    let published = Rc::new(Cell::new(false));
                    let member = original(&interpreter, 0, mode, &arm, &events, &published);
                    let tag = obj::Owned::fresh(obj::new_string_bytes(b"INNER"));
                    let protocol = interpreter
                        .native_invocation_dialect()
                        .native_error_objects_protocol()
                        .unwrap()
                        .strings();
                    let replacement = obj::Owned::fresh(crate::list::new_list_obj_native(
                        &[tag.as_ptr(), member.as_ptr()],
                        protocol,
                    ));
                    let old = {
                        let mut stack = interpreter.error_stack.borrow_mut();
                        stack.renew_incarnation();
                        if operation == 0 {
                            stack.metadata.begin_inner((), ());
                            stack.header.replace(replacement)
                        } else {
                            stack.metadata.mark_reset();
                            Some(stack.inner_context.as_ref().unwrap().replace(replacement))
                        }
                    };
                    drop(old);
                    drop(member);
                    drop(tag);
                    let scope =
                        super::super::native_operation_currency::NativeOperationScope::enter(
                            &interpreter,
                        )
                        .unwrap();
                    let operand = obj::Owned::fresh(obj::new_string_bytes(b"ORIGINAL OPERAND"));
                    let reached = if operation == 0 {
                        interpreter
                            .adopt_error_stack_bytes_checked(&[b"INNER".to_vec(), b"NEW".to_vec()])
                    } else {
                        interpreter.capture_original_return_instruction_context(
                                tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                                operand.as_ptr(), operand.as_ptr())
                    };
                    assert_eq!(
                        events.borrow().as_slice(),
                        &[(0, 0)],
                        "{name} op{operation} mode{mode}"
                    );
                    assert!(published.get());
                    assert_eq!(
                        reached.is_ok(),
                        mode == 0,
                        "{name} op{operation} mode{mode}"
                    );
                    assert_eq!(interpreter.result.get(), result);
                    assert_eq!(interpreter.error_line.get(), 37);
                    if mode == 0 {
                        scope.currency().ensure_current_or_refuse().unwrap();
                        let stack = interpreter.error_stack.borrow();
                        assert!(!stack.is_reset());
                        let header = stack.original_header().unwrap();
                        let members =
                            crate::list::list_elements_native_checked(header, protocol).unwrap();
                        assert_eq!(members.len(), 2);
                        if operation == 1 {
                            let inner = stack.original_inner_context().unwrap();
                            assert_eq!(members[1], inner);
                            assert_eq!(unsafe { (*inner).ref_count }, 2);
                            let members =
                                crate::list::list_elements_native_checked(inner, protocol).unwrap();
                            assert_eq!(members.len(), 3);
                            assert_eq!(&members[1..], &[operand.as_ptr(), operand.as_ptr()]);
                            assert_eq!(unsafe { (*operand.as_ptr()).ref_count }, 3);
                        }
                    } else {
                        let first = interpreter.native_execution_refusal().unwrap();
                        assert_eq!(reached.unwrap_err(), first);
                        assert_eq!(interpreter.capture_original_return_instruction_context(
                                tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                                operand.as_ptr(), operand.as_ptr()).unwrap_err(), first);
                        assert_eq!(events.borrow().as_slice(), &[(0, 0)]);
                    }
                    clear(&interpreter, &arm);
                    drop(operand);
                    drop(scope);
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_stack_configuration_publishes_roles_before_mandatory_header_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Generic configuration software publication and cleanup, not selected Reset/Setter order.
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..4 {
            crate::counters::reset();
            {
                let mut interpreter = native(name);
                interpreter.set_result_bytes(b"PRIOR CONFIGURATION RESULT");
                let result = interpreter.result.get();
                let arm = Rc::new(Cell::new(true));
                let events = Rc::new(RefCell::new(Vec::new()));
                let published = Rc::new(Cell::new(false));
                let header = original(&interpreter, 0, mode, &arm, &events, &published);
                let inner = original(&interpreter, 1, mode, &arm, &events, &published);
                let old = interpreter.error_stack.replace(NativeErrorStack {
                    header: Some(header),
                    inner_context: Some(Rc::new(RefCell::new(inner))),
                    ..NativeErrorStack::default()
                });
                drop(old);
                let scope = super::super::native_operation_currency::NativeOperationScope::enter(
                    &interpreter,
                )
                .unwrap();
                let reached = interpreter.configure_native_error_stack_checked();
                assert_eq!(
                    events.borrow().as_slice(),
                    &[(0, 0), (1, 0)],
                    "{name} mode{mode}"
                );
                assert!(published.get());
                assert_eq!(reached.is_ok(), mode == 0);
                assert_eq!(interpreter.result.get(), result);
                assert!(interpreter.error_stack.borrow().original_header().is_some());
                assert!(interpreter
                    .error_stack
                    .borrow()
                    .original_inner_context()
                    .is_some());
                if mode == 0 {
                    scope.currency().ensure_current_or_refuse().unwrap();
                } else {
                    let first = interpreter.native_execution_refusal().unwrap();
                    assert_eq!(reached.unwrap_err(), first);
                    assert_eq!(
                        interpreter
                            .configure_native_error_stack_checked()
                            .unwrap_err(),
                        first
                    );
                }
                clear(&interpreter, &arm);
                drop(scope);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}

#[test]
fn original_stack_frame_append_keeps_existing_members_and_actual_frame_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Source-only actual List/member roles; no public getter or frame-admission grant.
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        crate::counters::reset();
        {
            let interpreter = native(name);
            let arm = Rc::new(Cell::new(true));
            let events = Rc::new(RefCell::new(Vec::new()));
            let published = Rc::new(Cell::new(false));
            let inner = original(&interpreter, 0, 0, &arm, &events, &published);
            let frame = original(&interpreter, 1, 0, &arm, &events, &published);
            let scope =
                super::super::native_operation_currency::NativeOperationScope::enter(&interpreter)
                    .unwrap();
            interpreter
                .capture_original_return_instruction_context(
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                    inner.as_ptr(),
                    inner.as_ptr(),
                )
                .unwrap();
            let header = interpreter.error_stack.borrow().original_header().unwrap();
            let protocol = interpreter
                .native_invocation_dialect()
                .native_error_objects_protocol()
                .unwrap()
                .strings();
            let before = crate::list::list_elements_native_checked(header, protocol).unwrap();
            interpreter
                .log_original_error_stack_frame_checked(ErrorStackFrame::Call(frame.clone()))
                .unwrap();
            let current = interpreter.error_stack.borrow().original_header().unwrap();
            assert_eq!(current, header);
            let after = crate::list::list_elements_native_checked(current, protocol).unwrap();
            assert_eq!(&after[..2], before.as_slice());
            assert_eq!(after.len(), 4);
            assert_eq!(after[3], frame.as_ptr());
            assert_eq!(unsafe { (*frame.as_ptr()).ref_count }, 2);
            assert!(events.borrow().is_empty());
            scope.currency().ensure_current_or_refuse().unwrap();
            clear(&interpreter, &arm);
            drop(frame);
            drop(inner);
            drop(scope);
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}
