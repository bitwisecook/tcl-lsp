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
use crate::interp::default_host;
use crate::interp::native_operation_currency::{
    NativeOperationScope, SavedNativeOperationCurrency,
};
#[cfg(debug_assertions)]
use crate::interp::Code;
use crate::interp::{ExceptionState, Interp, WeakInterp, GLOBAL};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[cfg(debug_assertions)]
use tcl_runtime_api::NativeExecutionError;

struct Observer {
    interpreter: WeakInterp,
    action: u8,
    getter: bool,
    arm: Rc<Cell<bool>>,
    events: Rc<RefCell<Vec<(bool, obj::TclSize)>>>,
    published: Rc<Cell<bool>>,
    nested_refused: Rc<Cell<bool>>,
    level: Rc<Cell<usize>>,
}
fn observe(state: &Observer, value: *mut TclObj, getter: bool) {
    if !state.arm.get() {
        return;
    }
    state
        .events
        .borrow_mut()
        .push((getter, unsafe { (*value).ref_count }));
    let Some(mut interpreter) = state.interpreter.upgrade() else {
        return;
    };
    state.level.set(interpreter.frames.borrow().current_level());
    // A genuine callback mutably observes the slot outside the owning
    // writer's RefCell borrow. This is software transport, not Native order.
    state
        .published
        .set(interpreter.return_options.borrow_mut().header().is_none());
    if getter != state.getter {
        return;
    }
    match state.action {
        1 => {
            interpreter.refuse_host_command("first original carried-option callback");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("original-carried-option".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            let currency = SavedNativeOperationCurrency::capture(&interpreter);
            let saved = interpreter.take_return_options();
            let cleanup =
                NativeOperationScope::enter_saved_cleanup(&currency, &interpreter).unwrap();
            interpreter
                .restore_return_options_for_cleanup(saved, &cleanup)
                .unwrap();
        }
        _ => {}
    }
    if !getter && state.action != 0 {
        // Mandatory cleanup must not issue a fresh nested entry after the
        // actual callback withdrew the inherited receipt.
        state
            .nested_refused
            .set(NativeOperationScope::enter(&interpreter).is_err());
    }
}
extern "C" fn update(value: *mut TclObj) {
    let state = unsafe { &*(obj::internal_rep(value) as usize as *const Observer) };
    unsafe { obj::set_native_updater_string_rep(value, b"-original", false) };
    observe(state, value, true);
}
extern "C" fn retire(value: *mut TclObj) {
    let state = unsafe { Box::from_raw(obj::internal_rep(value) as usize as *mut Observer) };
    observe(&state, value, false);
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalCarriedOption".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
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
    action: u8,
    getter: bool,
    arm: &Rc<Cell<bool>>,
    events: &Rc<RefCell<Vec<(bool, obj::TclSize)>>>,
    published: &Rc<Cell<bool>>,
    nested_refused: &Rc<Cell<bool>>,
) -> obj::Owned {
    let state = Box::new(Observer {
        interpreter: interpreter.downgrade(),
        action,
        getter,
        arm: arm.clone(),
        events: events.clone(),
        published: published.clone(),
        nested_refused: nested_refused.clone(),
        level: Rc::new(Cell::new(usize::MAX)),
    });
    obj::Owned::fresh(obj::alloc_typed(
        &ORIGINAL,
        Box::into_raw(state) as usize as u64,
    ))
}
fn cleanup(interpreter: &Interp, arm: &Cell<bool>) {
    arm.set(false);
    drop(
        interpreter
            .return_options
            .replace(NativeReturnOptions::default()),
    );
    drop(interpreter.exc.replace(ExceptionState::default()));
}

#[cfg(debug_assertions)]
#[test]
fn original_carried_option_retirement_stops_new_body_and_preserves_reached_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software owning-member callbacks; no external C/Jim option retirement schedule is inferred.
    for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for action in 0..3 {
            crate::counters::reset();
            {
                let mut interpreter = native(name);
                let arm = Rc::new(Cell::new(false));
                let events = Rc::new(RefCell::new(Vec::new()));
                let published = Rc::new(Cell::new(false));
                let nested = Rc::new(Cell::new(false));
                let value = original(
                    &interpreter,
                    action,
                    false,
                    &arm,
                    &events,
                    &published,
                    &nested,
                );
                interpreter
                    .set_return_option_objects_checked(vec![ReturnOptionPair {
                        key: obj::Owned::fresh(obj::new_string_bytes(b"-original")),
                        value,
                        key_bytes: b"-original".to_vec(),
                    }])
                    .unwrap();
                interpreter.set_result_bytes(b"PRIOR RESULT");
                let prior = interpreter.result.get();
                arm.set(true);
                let code = interpreter.eval_str(b"set original_later REACHED");
                assert_eq!(
                    events.borrow().as_slice(),
                    &[(false, 0)],
                    "{name} action{action}"
                );
                assert!(published.get());
                assert!(interpreter.return_options.borrow().header().is_none());
                assert_eq!(code, if action == 0 { Code::Ok } else { Code::Error });
                let stored = crate::vars::get_at(
                    &interpreter.frames.borrow(),
                    &interpreter.namespaces.borrow(),
                    b"original_later",
                    0,
                );
                if action == 0 {
                    let stored =
                        stored.expect("ordinary option retirement reaches the actual body");
                    assert_eq!(obj::bytes_of(stored), b"REACHED");
                } else {
                    assert_eq!(stored, None);
                    assert_eq!(interpreter.result.get(), prior);
                }
                if action == 0 {
                    assert!(interpreter.native_execution_refusal().is_none());
                } else {
                    let first = interpreter.native_execution_refusal().unwrap();
                    assert_eq!(
                        matches!(first, NativeExecutionError::HostCommandRefusal(_)),
                        action == 1
                    );
                    assert!(nested.get());
                    assert_eq!(
                        interpreter.clear_return_options_checked().unwrap_err(),
                        first
                    );
                }
                cleanup(&interpreter, &arm);
            }
            assert_eq!(crate::counters::finalize(), 0, "{name} action{action}");
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_carried_option_key_getter_keeps_actual_roles_and_refuses_before_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for action in 0..3 {
            crate::counters::reset();
            {
                let interpreter = native(name);
                interpreter
                    .set_return_options_checked(vec![(b"-prior".to_vec(), b"VALUE".to_vec())])
                    .unwrap();
                let header = interpreter.return_options.borrow().header();
                let arm = Rc::new(Cell::new(true));
                let events = Rc::new(RefCell::new(Vec::new()));
                let published = Rc::new(Cell::new(false));
                let nested = Rc::new(Cell::new(false));
                let key = original(
                    &interpreter,
                    action,
                    true,
                    &arm,
                    &events,
                    &published,
                    &nested,
                );
                let reached =
                    interpreter.set_return_option_objects_checked(vec![ReturnOptionPair {
                        key,
                        value: obj::Owned::fresh(obj::new_string_bytes(b"NEW VALUE")),
                        key_bytes: b"-original".to_vec(),
                    }]);
                assert_eq!(events.borrow()[0], (true, 1), "{name} action{action}");
                assert_eq!(reached.is_ok(), action == 0);
                if action == 0 {
                    assert_ne!(interpreter.return_options.borrow().header(), header);
                } else {
                    let error = match reached {
                        Err(error) => error,
                        Ok(()) => panic!("original key callback must refuse"),
                    };
                    let first = interpreter.native_execution_refusal().unwrap();
                    assert_eq!(error.native_execution_refusal(), Some(&first));
                    assert_eq!(interpreter.return_options.borrow().header(), header);
                }
                cleanup(&interpreter, &arm);
            }
            assert_eq!(crate::counters::finalize(), 0, "{name} action{action}");
        }
    }
}

#[test]
fn original_carried_option_same_header_take_restore_cannot_revive_prepared_read() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The retained Rust identity cannot wrap or be reused while held; no Native role is issued by it.
    for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        crate::counters::reset();
        {
            let interpreter = native(name);
            let arm = Rc::new(Cell::new(false));
            let events = Rc::new(RefCell::new(Vec::new()));
            let published = Rc::new(Cell::new(false));
            let nested = Rc::new(Cell::new(false));
            let key = original(&interpreter, 3, true, &arm, &events, &published, &nested);
            let original_key = key.as_ptr();
            unsafe { obj::set_string_rep(original_key, b"-original") };
            interpreter
                .set_return_option_objects_checked(vec![ReturnOptionPair {
                    key,
                    value: obj::Owned::fresh(obj::new_string_bytes(b"VALUE")),
                    key_bytes: b"-original".to_vec(),
                }])
                .unwrap();
            let (header, ticket) = {
                let options = interpreter.return_options.borrow();
                (options.header(), Rc::clone(&options.incarnation))
            };
            obj::invalidate_string(original_key);
            arm.set(true);
            let scope = NativeOperationScope::enter(&interpreter).unwrap();
            let reached = interpreter.pending_return_option_objects_checked(scope.currency());
            assert!(reached.is_err());
            assert_eq!(events.borrow().as_slice(), &[(true, 1)]);
            assert_eq!(interpreter.return_options.borrow().header(), header);
            assert!(!Rc::ptr_eq(
                &ticket,
                &interpreter.return_options.borrow().incarnation
            ));
            assert_eq!(unsafe { (*original_key).ref_count }, 1);
            let first = interpreter.native_execution_refusal().unwrap();
            assert_eq!(reached.err(), Some(first.clone()));
            assert_eq!(
                scope.currency().ensure_current_or_refuse().unwrap_err(),
                first
            );
            cleanup(&interpreter, &arm);
            drop(scope);
        }
        assert_eq!(crate::counters::finalize(), 0, "{name}");
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_saved_option_cleanup_finishes_result_restore_after_host_or_context_withdrawal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for action in 0..3 {
            crate::counters::reset();
            {
                let mut interpreter = native(name);
                interpreter
                    .set_return_options_checked(vec![(b"-saved".to_vec(), b"SAVED".to_vec())])
                    .unwrap();
                let saved_header = interpreter.return_options.borrow().header();
                interpreter.set_result_bytes(b"SAVED RESULT");
                let saved_result = interpreter.result.get();
                let arm = Rc::new(Cell::new(false));
                let events = Rc::new(RefCell::new(Vec::new()));
                let published = Rc::new(Cell::new(false));
                let nested = Rc::new(Cell::new(false));
                let reached = interpreter.with_preserved_result_checked(|interp| {
                    let value = original(interp, action, false, &arm, &events, &published, &nested);
                    interp
                        .set_return_option_objects_checked(vec![ReturnOptionPair {
                            key: obj::Owned::fresh(obj::new_string_bytes(b"-callback")),
                            value,
                            key_bytes: b"-callback".to_vec(),
                        }])
                        .unwrap();
                    interp.set_result_bytes(b"CALLBACK RESULT");
                    arm.set(true);
                    37u32
                });
                assert_eq!(events.borrow().as_slice(), &[(false, 0)]);
                assert_eq!(interpreter.result.get(), saved_result);
                assert_eq!(interpreter.return_options.borrow().header(), saved_header);
                assert_eq!(reached.is_ok(), action == 0);
                assert_eq!(interpreter.entered_native_operation_count(), 0);
                if action == 0 {
                    assert_eq!(reached.unwrap(), 37);
                } else {
                    assert!(nested.get());
                    assert_eq!(reached.err(), interpreter.native_execution_refusal());
                }
                cleanup(&interpreter, &arm);
            }
            assert_eq!(crate::counters::finalize(), 0, "{name} action{action}");
        }
    }
}

#[test]
fn original_coroutine_option_handoff_renews_installed_and_parked_incarnations() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Mandatory physical handoff software currentness only, no coroutine Native chronology grant.
    let interpreter = native("tcl9.0");
    interpreter
        .set_return_options_checked(vec![(b"-live".to_vec(), b"LIVE".to_vec())])
        .unwrap();
    let live = Rc::clone(&interpreter.return_options.borrow().incarnation);
    let header = interpreter.return_options.borrow().header();
    let mut parked = crate::interp::CoroContext::fresh(GLOBAL);
    let parked_ticket = Rc::clone(&parked.return_options.incarnation);
    interpreter.swap_coro_ctx(&mut parked);
    assert_eq!(parked.return_options.header(), header);
    assert!(!Rc::ptr_eq(&live, &parked.return_options.incarnation));
    assert!(!Rc::ptr_eq(
        &parked_ticket,
        &interpreter.return_options.borrow().incarnation
    ));
    interpreter.swap_coro_ctx(&mut parked);
    assert_eq!(interpreter.return_options.borrow().header(), header);
    assert!(!Rc::ptr_eq(
        &live,
        &interpreter.return_options.borrow().incarnation
    ));
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}

#[cfg(debug_assertions)]
#[test]
fn original_procedure_option_body_start_refusal_still_tears_down_the_real_frame() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual stored procedure/entered frame and member free; no native procedure chronology inferred.
    for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for action in 0..3 {
            crate::counters::reset();
            {
                let mut interpreter = native(name);
                assert_eq!(
                    interpreter.eval_str(
                        b"proc original_option_proc {} {set ::original_body_effect REACHED}"
                    ),
                    Code::Ok
                );
                let command = obj::Owned::fresh(obj::new_string_bytes(b"original_option_proc"));
                let (resolved, _) = interpreter
                    .resolve_original_command_at(GLOBAL, command.as_ptr())
                    .unwrap()
                    .unwrap();
                let crate::interp::Command::Proc(binding) = resolved else {
                    panic!("the actual stored original procedure must be selected");
                };
                let arm = Rc::new(Cell::new(false));
                let events = Rc::new(RefCell::new(Vec::new()));
                let published = Rc::new(Cell::new(false));
                let nested = Rc::new(Cell::new(false));
                let value = original(
                    &interpreter,
                    action,
                    false,
                    &arm,
                    &events,
                    &published,
                    &nested,
                );
                let level = unsafe {
                    Rc::clone(
                        &(*(obj::internal_rep(value.as_ptr()) as usize as *const Observer)).level,
                    )
                };
                interpreter
                    .set_return_option_objects_checked(vec![ReturnOptionPair {
                        key: obj::Owned::fresh(obj::new_string_bytes(b"-original")),
                        value,
                        key_bytes: b"-original".to_vec(),
                    }])
                    .unwrap();
                let before_level = interpreter.frames.borrow().current_level();
                let before_ns = interpreter.current_ns.get();
                let before_depth = interpreter.recursion_depth.get();
                let before_cmd_frames = interpreter.cmd_frames.borrow().len();
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                arm.set(true);
                let code = interpreter.call_bound_proc(&binding, &[command.as_ptr()]);
                assert_eq!(
                    events.borrow().as_slice(),
                    &[(false, 0)],
                    "{name} action{action}"
                );
                assert!(
                    level.get() > before_level,
                    "body-start retirement occurs in the genuine frame"
                );
                assert_eq!(interpreter.frames.borrow().current_level(), before_level);
                assert_eq!(interpreter.current_ns.get(), before_ns);
                assert_eq!(interpreter.recursion_depth.get(), before_depth);
                assert_eq!(interpreter.cmd_frames.borrow().len(), before_cmd_frames);
                let effect = crate::vars::get_at(
                    &interpreter.frames.borrow(),
                    &interpreter.namespaces.borrow(),
                    b"::original_body_effect",
                    0,
                );
                if action == 0 {
                    assert_eq!(code, Code::Ok);
                    assert_eq!(obj::bytes_of(effect.unwrap()), b"REACHED");
                    scope.currency().ensure_current_or_refuse().unwrap();
                } else {
                    assert_eq!(code, Code::Error);
                    assert_eq!(effect, None);
                    assert!(nested.get());
                    assert_eq!(
                        scope.currency().ensure_current_or_refuse().err(),
                        interpreter.native_execution_refusal()
                    );
                }
                cleanup(&interpreter, &arm);
                drop(scope);
                drop(binding);
            }
            assert_eq!(crate::counters::finalize(), 0, "{name} action{action}");
        }
    }
}
