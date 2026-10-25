// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual VM trace callbacks, typed refusals and original cleanup owners.

use super::super::{Vm, ok};
use crate::Value;
use std::{cell::RefCell, rc::Rc};
use tcl_cmd_core::CmdError;
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};
use tcl_runtime_api::{NativeExecutionError, Traces};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

const RELEASES: [&str; 5] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];

#[derive(Clone)]
enum Action {
    Complete,
    Guest,
    Refuse(NativeExecutionError),
    ChangeHost,
    NestedBoundary(Option<super::super::InterpId>),
}
struct Observer {
    reached: Rc<RefCell<Vec<u8>>>,
    label: u8,
    action: Action,
}
impl NativeVariableObserver<Vm> for Observer {
    type Error = CmdError;
    fn observe(&self, vm: &mut Vm, _access: NativeVariableTraceAccess<'_>) -> Result<(), CmdError> {
        self.reached.borrow_mut().push(self.label);
        match &self.action {
            Action::Complete => Ok(()),
            Action::Guest => Err(CmdError::new("actual callback Guest error")),
            Action::Refuse(first) => Err(CmdError::from_execution_refusal(first.clone())),
            Action::ChangeHost => {
                let original = vm.host_rc();
                vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                vm.set_host(original);
                Ok(())
            }
            Action::NestedBoundary(child) => {
                let _ = vm.refuse_host_command("actual nested trace boundary".into());
                let first = vm.execution_refusal.clone().unwrap();
                let run = |vm: &mut Vm| {
                    assert_eq!(
                        vm.finish_host_execution(ok(Value::empty())).unwrap_err(),
                        first
                    );
                    assert_eq!(vm.execution_refusal, Some(first.clone()));
                };
                if let Some(child) = child {
                    vm.in_interp(*child, run);
                } else {
                    run(vm);
                }
                assert_eq!(vm.execution_refusal, Some(first));
                Ok(())
            }
        }
    }
}
fn native(profile: &str) -> Vm {
    crate::native_fixture::core(crate::environment::profile_for_dialect(profile))
}
fn first() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "original VM trace observer cause",
    ))
}
fn add(
    vm: &mut Vm,
    name: &[u8],
    operation: NativeVariableTraceOperation,
    reached: &Rc<RefCell<Vec<u8>>>,
    label: u8,
    action: Action,
) {
    vm.add_native_variable_observer(
        &Value::new_native_string_bytes(name),
        &[operation],
        Rc::new(Observer {
            reached: Rc::clone(reached),
            label,
            action,
        }),
    )
    .unwrap();
}

#[test]
fn original_callback_host_cause_stops_later_rows_and_restores_the_real_result() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual VM callback/owner control; this is not an external Tcl execution.
    for profile in RELEASES {
        let mut vm = native(profile);
        vm.set_var_bytes(b"x", Value::int(1)).unwrap();
        let reached = Rc::new(RefCell::new(Vec::new()));
        add(
            &mut vm,
            b"x",
            NativeVariableTraceOperation::Read,
            &reached,
            1,
            Action::Complete,
        );
        add(
            &mut vm,
            b"x",
            NativeVariableTraceOperation::Read,
            &reached,
            2,
            Action::Refuse(first()),
        );
        let cell = vm.trace_cell_bytes(b"x").unwrap().id.unwrap();
        let before = vm
            .publish_native_interp_completion(ok(Value::string("ORIGINAL RESULT")))
            .unwrap();
        let identity = before.result.native_object_identity();
        assert_eq!(
            Traces::fire_bytes(&mut vm, b"x", "read").unwrap_err(),
            first(),
            "{profile}"
        );
        assert_eq!(*reached.borrow(), [2], "{profile}");
        assert_eq!(vm.execution_refusal, Some(first()));
        assert!(!vm.var_arena.has_operation_refs(cell));
        assert!(vm.active_traces.is_empty());
        assert!(!vm.has_entered_native_operation());
        vm.with_native_interp_result(|result| {
            assert_eq!(result.native_object_identity(), identity);
        })
        .unwrap();
        assert_eq!(Traces::fire(&mut vm, "x", "read").unwrap_err(), first());
        assert_eq!(*reached.borrow(), [2]);
    }
}

#[test]
fn original_callback_host_aba_keeps_reached_effects_without_a_later_observer() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in RELEASES {
        let mut vm = native(profile);
        vm.set_var_bytes(b"x", Value::int(1)).unwrap();
        let reached = Rc::new(RefCell::new(Vec::new()));
        add(
            &mut vm,
            b"x",
            NativeVariableTraceOperation::Read,
            &reached,
            1,
            Action::Complete,
        );
        add(
            &mut vm,
            b"x",
            NativeVariableTraceOperation::Read,
            &reached,
            2,
            Action::ChangeHost,
        );
        let before = vm
            .publish_native_interp_completion(ok(Value::string("ORIGINAL RESULT")))
            .unwrap();
        let identity = before.result.native_object_identity();
        let refusal = Traces::fire_bytes(&mut vm, b"x", "read").unwrap_err();
        assert_eq!(
            refusal,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "entered VM original execution context"
                )
            )
        );
        assert_eq!(*reached.borrow(), [2]);
        assert_eq!(vm.execution_refusal, Some(refusal));
        vm.with_native_interp_result(|result| {
            assert_eq!(result.native_object_identity(), identity);
        })
        .unwrap();
        assert!(vm.active_traces.is_empty());
    }
}

#[test]
fn original_quiet_scalar_element_and_c_exists_ignore_guest_errors_only() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in RELEASES {
        for element in [false, true] {
            let mut vm = native(profile);
            let name: &[u8] = if element { b"a(k)" } else { b"x" };
            vm.set_var_bytes(name, Value::int(1)).unwrap();
            let reached = Rc::new(RefCell::new(Vec::new()));
            add(
                &mut vm,
                name,
                NativeVariableTraceOperation::Read,
                &reached,
                1,
                Action::Guest,
            );
            let found = if element {
                vm.exists_elem_traced_bytes(b"a", b"k")
            } else {
                vm.exists_var_traced_bytes(name)
            };
            assert_eq!(found, Ok(true), "{profile}, element={element}");
            assert!(vm.execution_refusal.is_none());
            assert!(
                vm.exists_original_c_parts(&Value::new_native_string_bytes(name), None)
                    .unwrap()
            );
            assert_eq!(*reached.borrow(), [1, 1]);
            assert!(vm.active_traces.is_empty());
        }
    }
}

#[test]
fn original_scalar_element_and_c_exists_refuse_before_boolean_publication() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in RELEASES {
        for route in 0..3 {
            let mut vm = native(profile);
            let name: &[u8] = if route == 1 { b"a(k)" } else { b"x" };
            vm.set_var_bytes(name, Value::int(1)).unwrap();
            let reached = Rc::new(RefCell::new(Vec::new()));
            add(
                &mut vm,
                name,
                NativeVariableTraceOperation::Read,
                &reached,
                1,
                Action::Refuse(first()),
            );
            if route == 2 {
                assert!(
                    vm.exists_original_c_parts(&Value::new_native_string_bytes(name), None)
                        .is_err()
                );
            } else {
                let result = if route == 1 {
                    vm.exists_elem_traced_bytes(b"a", b"k")
                } else {
                    vm.exists_var_traced_bytes(name)
                };
                assert_eq!(result, Err(first()));
            }
            assert_eq!(vm.execution_refusal, Some(first()));
            assert_eq!(*reached.borrow(), [1]);
            assert!(vm.active_traces.is_empty());
        }
    }
}

#[test]
fn original_taken_unset_stops_host_callbacks_but_finishes_removed_cell_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in RELEASES {
        for host in [false, true] {
            let mut vm = native(profile);
            vm.set_var_bytes(b"x", Value::int(1)).unwrap();
            let reached = Rc::new(RefCell::new(Vec::new()));
            add(
                &mut vm,
                b"x",
                NativeVariableTraceOperation::Unset,
                &reached,
                1,
                Action::Complete,
            );
            add(
                &mut vm,
                b"x",
                NativeVariableTraceOperation::Unset,
                &reached,
                2,
                if host {
                    Action::Refuse(first())
                } else {
                    Action::Guest
                },
            );
            let cell = vm.trace_cell_bytes(b"x").unwrap().id.unwrap();
            assert!(vm.unset_var_bytes(b"x"));
            assert_eq!(*reached.borrow(), if host { vec![2] } else { vec![2, 1] });
            assert_eq!(vm.execution_refusal, host.then(first));
            assert!(!vm.variable_observers.script_traces.contains_key(&cell));
            assert!(!vm.var_arena.has_operation_refs(cell));
            assert!(vm.active_traces.is_empty());
            assert!(vm.get_var_bytes(b"x").is_none());
        }
    }
}

#[test]
fn original_parent_scope_keeps_the_exact_cause_across_current_and_parked_host_boundaries() {
    // naming.embedding.original-host-publication-and-fact-transport

    for parked in [false, true] {
        let mut vm = native("tcl8.6");
        let child_name = vm.create_child(Some("child".into()), false);
        let child = vm.child_id(&child_name).unwrap();
        vm.set_var_bytes(b"x", Value::int(1)).unwrap();
        let reached = Rc::new(RefCell::new(Vec::new()));
        add(
            &mut vm,
            b"x",
            NativeVariableTraceOperation::Read,
            &reached,
            1,
            Action::Complete,
        );
        add(
            &mut vm,
            b"x",
            NativeVariableTraceOperation::Read,
            &reached,
            2,
            Action::NestedBoundary(parked.then_some(child)),
        );
        let refusal = Traces::fire_bytes(&mut vm, b"x", "read").unwrap_err();
        let NativeExecutionError::HostCommandRefusal(payload) = &refusal else {
            panic!("original Host payload");
        };
        assert_eq!(payload.reason, "actual nested trace boundary");
        assert_eq!(*reached.borrow(), [2]);
        assert_eq!(vm.execution_refusal, Some(refusal.clone()));
        assert!(!vm.has_entered_native_operation());
        assert_eq!(vm.take_host_boundary_refusal(), Some(refusal));
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_saved_trace_currency_is_descriptive_until_same_target_mandatory_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in RELEASES {
        let mut vm = native(profile);
        let parent = vm.cur;
        let before = vm
            .publish_native_interp_completion(ok(Value::string("PARENT ORIGINAL")))
            .unwrap();
        let identity = before.result.native_object_identity();
        let saved = if profile == "tcl8.4" {
            vm.save_native_script_trace_result()
        } else {
            vm.save_native_interp_trace_result()
        }
        .unwrap()
        .unwrap();
        assert!(
            !vm.has_entered_native_operation(),
            "a saved receipt is not an active scope"
        );
        let child_name = vm.create_child(Some("child".into()), false);
        let child = vm.child_id(&child_name).unwrap();
        vm.in_interp(child, |vm| {
            let before_child = vm
                .publish_native_interp_completion(ok(Value::string("CHILD ORIGINAL")))
                .unwrap();
            let child_identity = before_child.result.native_object_identity();
            vm.restore_native_interp_trace_result(saved).unwrap();
            vm.with_native_interp_result(|result| {
                assert_eq!(result.native_object_identity(), child_identity);
            })
            .unwrap();
            let original = vm.st_of(parent).unwrap();
            assert_eq!(
                original
                    .native_c_result
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .native_object_identity(),
                identity
            );
            assert!(!vm.has_entered_native_operation());
        });
        vm.with_native_interp_result(|result| {
            assert_eq!(result.native_object_identity(), identity);
        })
        .unwrap();
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_saved_trace_cleanup_keeps_changed_currency_and_refuses_foreign_target() {
    // naming.embedding.original-host-publication-and-fact-transport

    for foreign in [false, true] {
        let mut vm = native("tcl8.6");
        let before = vm
            .publish_native_interp_completion(ok(Value::string("ORIGINAL")))
            .unwrap();
        let saved = vm.save_native_interp_trace_result().unwrap().unwrap();
        assert!(!vm.has_entered_native_operation());
        let original = vm.host_rc();
        vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
        vm.set_host(original);
        if foreign {
            let mut other = native("tcl8.6");
            let untouched = other
                .publish_native_interp_completion(ok(Value::string("OTHER")))
                .unwrap();
            assert!(other.restore_native_interp_trace_result(saved).is_err());
            other
                .with_native_interp_result(|result| {
                    assert!(result.is_same_object(&untouched.result));
                })
                .unwrap();
            assert!(vm.execution_refusal.is_none());
        } else {
            let refusal = vm.restore_native_interp_trace_result(saved).unwrap_err();
            assert_eq!(vm.execution_refusal, Some(refusal.clone()));
            assert_eq!(
                refusal,
                NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "entered VM original execution context"
                    )
                )
            );
            vm.with_native_interp_result(|result| assert!(result.is_same_object(&before.result)))
                .unwrap();
            assert!(!vm.has_entered_native_operation());
        }
    }
}

#[test]
fn original_native_observer_guest_initializes_error_info_before_its_trace_frame() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Real observer/VM owner stages; no Native callback chronology/count equivalence.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for operation in [
            NativeVariableTraceOperation::Read,
            NativeVariableTraceOperation::Write,
        ] {
            let mut vm = native(profile);
            vm.set_var_bytes(b"x", Value::int(1)).unwrap();
            let reached = Rc::new(RefCell::new(Vec::new()));
            add(&mut vm, b"x", operation, &reached, 1, Action::Complete);
            add(&mut vm, b"x", operation, &reached, 2, Action::Guest);
            vm.adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
                .unwrap();
            assert!(vm.native_errors.error_info.is_none());
            assert!(vm.native_errors.native_error_info.is_none());
            let op = if operation == NativeVariableTraceOperation::Read {
                "read"
            } else {
                "write"
            };
            let failure = vm
                .fire_var_traces_from_cell_bytes(b"x", op, None, None, None)
                .unwrap()
                .unwrap_err();
            let expected = format!(
                "can't {} \"x\": actual callback Guest error",
                if op == "read" { "read" } else { "set" }
            );
            let info = format!("actual callback Guest error\n    ({op} trace on \"x\")");
            assert_eq!(failure.code, crate::Code::Error, "{profile}/{op}");
            assert_eq!(failure.result.string_bytes().as_ref(), expected.as_bytes());
            assert_eq!(
                vm.native_errors.error_info.as_deref(),
                Some(info.as_bytes())
            );
            vm.with_native_interp_result(|result| {
                assert!(result.is_same_object(&failure.result));
            })
            .unwrap();
            assert_eq!(*reached.borrow(), [2]);
            assert!(
                vm.execution_refusal.is_none(),
                "{profile}/{op}: {:?}",
                vm.execution_refusal
            );
            assert!(vm.active_traces.is_empty());
            assert!(!vm.has_entered_native_operation());
        }
    }
}

#[test]
fn original_script_trace_guest_keeps_its_existing_error_episode_through_presentation() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = native(profile);
        let completion = vm.try_eval_source(
            "proc original_trace_stop {args} {error STOP SCRIPT_MARKER}; trace add variable x write original_trace_stop; set x changed"
        ).unwrap();
        assert_eq!(completion.code, crate::Code::Error, "{profile}");
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"can't set \"x\": STOP"
        );
        let info = vm.native_errors.error_info.as_ref().unwrap();
        assert!(info.starts_with(b"SCRIPT_MARKER"));
        assert!(
            info.windows(b"(write trace on \"x\")".len())
                .any(|part| part == b"(write trace on \"x\")")
        );
        assert!(vm.execution_refusal.is_none());
        assert!(vm.active_traces.is_empty());
        assert!(!vm.has_entered_native_operation());
    }
}

#[test]
fn original_quiet_trace_guest_restores_the_selected_result_without_new_error_info() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in RELEASES {
        let mut vm = native(profile);
        let name = Value::new_native_string_bytes(b"x".as_slice());
        vm.set_var_bytes(b"x", Value::int(1)).unwrap();
        let reached = Rc::new(RefCell::new(Vec::new()));
        add(
            &mut vm,
            b"x",
            NativeVariableTraceOperation::Write,
            &reached,
            1,
            Action::Guest,
        );
        let before = vm
            .adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
            .unwrap();
        let captured = vm.capture_update_cell(b"x", None).unwrap();
        let failure = vm
            .with_variable_operation(&captured.cell, |vm| {
                vm.store_captured_update_with_original_and_errors(
                    b"x",
                    None,
                    &captured,
                    Value::new_native_string_bytes(b"changed".as_slice()),
                    Some(&name),
                    false,
                )
            })
            .unwrap_err();
        assert_eq!(failure.code, crate::Code::Error);
        assert!(failure.result.string_bytes().is_empty());
        vm.with_native_interp_result(|result| assert!(result.is_same_object(before.value())))
            .unwrap();
        assert!(vm.native_errors.error_info.is_none());
        assert!(vm.native_errors.native_error_info.is_none());
        assert_eq!(*reached.borrow(), [1]);
        assert_eq!(
            vm.get_var_bytes(b"x").unwrap().string_bytes().as_ref(),
            b"changed"
        );
        assert!(vm.execution_refusal.is_none());
        assert!(vm.active_traces.is_empty());
        assert!(!vm.has_entered_native_operation());
    }
}
