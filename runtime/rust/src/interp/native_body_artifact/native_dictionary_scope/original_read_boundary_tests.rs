// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Real compiled scope read observer and stored-role software controls.

use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};

struct ReadRole {
    selected: Rc<crate::interp::native_variable_names::OriginalCVariableCapture>,
    pending: bool,
    calls: Cell<usize>,
    published: Cell<*mut TclObj>,
    cleanup: RefCell<Option<Box<dyn FnOnce()>>>,
    currency: crate::interp::native_operation_currency::NativeOperationCurrency,
}

impl NativeVariableObserver<Interp> for ReadRole {
    type Error = tcl_cmd_core::CmdError;

    fn observe(
        &self,
        _actual: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Read);
        self.calls.set(self.calls.get() + 1);
        let incoming = obj::new_string_bytes(b"K REACHED");
        self.published.set(incoming);
        if self.pending {
            let current = self.selected.receiver.read_initial().unwrap().unwrap();
            let (prepared, retired) = self
                .selected
                .receiver
                .publish_c84_pending_scalar(current, incoming)
                .unwrap();
            retired
                .finish(|| self.currency.ensure_current_or_refuse())
                .unwrap();
            let selected = self.selected.clone();
            let currency = self.currency.clone();
            *self.cleanup.borrow_mut() = Some(Box::new(move || {
                let _ = selected
                    .receiver
                    .discard_c84_pending_scalar(&prepared)
                    .finish(|| currency.ensure_current_or_refuse());
            }));
        } else {
            self.selected.receiver.store(incoming).unwrap();
        }
        Ok(())
    }
}

#[test]
fn original_compiled_scope_read_refuses_pending_role_before_missing_metadata_and_keeps_live_replacement(
) {

    // Pending publication here is an explicit Rust callback transport control.
    // It grants no modern Native scalar/setter schedule or C84 teardown parity.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for with in [false, true] {
            for pending in [false, true] {
                let mut interp = fixture::native(profile);
                let script = if with {
                    b"proc p {} {set d {K A}; dict with d {set K B}; return $d}".as_slice()
                } else {
                    b"proc p {} {set d {K A}; dict update d K v {set v B}; return $d}".as_slice()
                };
                assert_eq!(interp.eval_str(script), Code::Ok);
                assert_eq!(interp.eval_str(b"p"), Code::Ok);
                let procedure = interp.proc_def(b"p").unwrap();
                let artifact = cache(procedure.body.as_ptr()).unwrap();
                let instruction = artifact
                    .scripts
                    .values()
                    .flat_map(|script| &script.commands)
                    .find_map(|command| match &command.operation {
                        Operation::DictionaryScope(operation) => Some(operation),
                        _ => None,
                    })
                    .expect("actual original Dictionary Update/With");
                assert_eq!(matches!(instruction.recipe.kind, Kind::With { .. }), with);
                assert!(instruction.root_slot.is_some());
                let protocol = interp
                    .native_invocation_dialect()
                    .native_compiled_variable_protocol()
                    .filter(|protocol| protocol.has_indexed_locals())
                    .expect("actual native indexed-local protocol");
                {
                    let mut frames = interp.frames.borrow_mut();
                    frames.push(crate::namespace::GLOBAL);
                    frames.install_formal_cells(&[], protocol);
                    frames
                        .install_native_compiled_local_layout(
                            artifact.compiled_local_layout().unwrap(),
                        )
                        .unwrap();
                }
                let _current = fixture::install(&mut interp, 0);
                let initial = fixture::string(b"K A");
                let target = local_target(instruction.root_name.as_deref().unwrap());
                interp
                    .body_store_value(&target, instruction.root_slot, &initial)
                    .unwrap();
                let selected = Rc::new(
                    interp
                        .body_capture_target(&target, instruction.root_slot, with)
                        .unwrap(),
                );
                let original = BodyObjectOperation::capture(&interp).unwrap();
                let currency =
                    crate::interp::native_operation_currency::NativeOperationCurrency::issue(
                        &interp,
                    )
                    .unwrap();
                let observer = Rc::new(ReadRole {
                    selected: selected.clone(),
                    pending,
                    calls: Cell::new(0),
                    published: Cell::new(core::ptr::null_mut()),
                    cleanup: Default::default(),
                    currency,
                });
                let name = fixture::string(&target.root);
                let token = interp
                    .add_native_variable_observer(
                        name.as_ptr(),
                        &[NativeVariableTraceOperation::Read],
                        observer.clone(),
                    )
                    .unwrap();
                interp.set_result_bytes(b"PRIOR");
                interp.set_c_error_code(b"PRIOR CODE");
                let prior_result = interp.get_obj_result();
                let prior_code = {
                    let exception = interp.exc.borrow();
                    (
                        exception.code.clone(),
                        exception.native.code.as_ref().unwrap().as_ptr(),
                    )
                };
                let reached = interp.compiled_dictionary_read(
                    &original,
                    &target,
                    instruction.root_slot,
                    with,
                );
                assert_eq!(observer.calls.get(), 1);
                assert_eq!(interp.get_obj_result(), prior_result);
                {
                    let exception = interp.exc.borrow();
                    assert_eq!(exception.code, prior_code.0);
                    assert_eq!(
                        exception.native.code.as_ref().unwrap().as_ptr(),
                        prior_code.1
                    );
                }
                assert_eq!(unsafe { (*initial.as_ptr()).ref_count }, 1);
                if pending {
                    assert!(matches!(reached, Err(Code::Error)));
                    assert!(original.check(&interp).is_err());
                    assert_eq!(
                        selected.receiver.read_initial(),
                        Err(crate::frame::VarError::NameProtocolUnavailable)
                    );
                    assert_eq!(unsafe { (*observer.published.get()).ref_count }, 0);
                    assert!(matches!(
                        interp.native_execution_refusal(),
                        Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_))
                    ));
                    let cleanup = observer.cleanup.borrow_mut().take().unwrap();
                    cleanup();
                    assert!(!obj::allocation_is_live(observer.published.get()));
                } else {
                    let (_, current) = match reached {
                        Ok(Some(selected)) => selected,
                        _ => panic!("current selected read must keep its real replacement"),
                    };
                    assert_eq!(current, observer.published.get());
                    assert_eq!(unsafe { (*current).ref_count }, 1);
                    original.check(&interp).unwrap();
                    assert!(interp.native_execution_refusal().is_none());
                    assert_eq!(
                        original.bytes(&interp, current).unwrap().as_ref(),
                        b"K REACHED"
                    );
                }
                interp.remove_native_variable_observer(&token);
            }
        }
    }
}

struct ReadFrame {
    withdraw: bool,
    calls: Cell<usize>,
}
impl NativeVariableObserver<Interp> for ReadFrame {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        actual: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Read);
        self.calls.set(self.calls.get() + 1);
        if self.withdraw {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        Ok(())
    }
}
#[test]
fn original_compiled_scope_read_checks_original_frame_before_selected_read_and_missing_metadata() {

    // Genuine reached Rust Read callback; no Native retirement chronology claim.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for withdraw in [false, true] {
            let mut interp = fixture::native(profile);
            assert_eq!(
                interp.eval_str(b"proc p {} {set d {K A}; dict update d K v {set v B}; return $d}"),
                Code::Ok
            );
            assert_eq!(interp.eval_str(b"p"), Code::Ok);
            let procedure = interp.proc_def(b"p").unwrap();
            let artifact = cache(procedure.body.as_ptr()).unwrap();
            let instruction = artifact
                .scripts
                .values()
                .flat_map(|script| &script.commands)
                .find_map(|command| match &command.operation {
                    Operation::DictionaryScope(operation) => Some(operation),
                    _ => None,
                })
                .expect("actual original Dictionary Update");
            let protocol = interp
                .native_invocation_dialect()
                .native_compiled_variable_protocol()
                .filter(|protocol| protocol.has_indexed_locals())
                .expect("actual native indexed-local protocol");
            {
                let mut frames = interp.frames.borrow_mut();
                frames.push(crate::namespace::GLOBAL);
                frames.install_formal_cells(&[], protocol);
                frames
                    .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
                    .unwrap();
            }
            let _current = fixture::install(&mut interp, 0);
            let initial = fixture::string(b"K A");
            let target = local_target(instruction.root_name.as_deref().unwrap());
            interp
                .body_store_value(&target, instruction.root_slot, &initial)
                .unwrap();
            let original = BodyObjectOperation::capture(&interp).unwrap();
            let observer = Rc::new(ReadFrame {
                withdraw,
                calls: Cell::new(0),
            });
            let name = fixture::string(&target.root);
            let token = interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[NativeVariableTraceOperation::Read],
                    observer.clone(),
                )
                .unwrap();
            interp.set_result_bytes(b"PRIOR");
            interp.set_c_error_code(b"PRIOR CODE");
            let prior_result = interp.get_obj_result();
            let prior_code = {
                let exception = interp.exc.borrow();
                (
                    exception.code.clone(),
                    exception.native.code.as_ref().unwrap().as_ptr(),
                )
            };
            let reached =
                interp.compiled_dictionary_read(&original, &target, instruction.root_slot, false);
            assert_eq!(observer.calls.get(), 1);
            assert_eq!(interp.get_obj_result(), prior_result);
            {
                let exception = interp.exc.borrow();
                assert_eq!(exception.code, prior_code.0);
                assert_eq!(
                    exception.native.code.as_ref().unwrap().as_ptr(),
                    prior_code.1
                );
            }
            if withdraw {
                assert!(matches!(reached, Err(Code::Error)));
                assert!(matches!(
                    interp.native_execution_refusal(),
                    Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_))
                ));
                assert_eq!(unsafe { (*initial.as_ptr()).ref_count }, 1);
            } else {
                let (_, current) = reached.unwrap().unwrap();
                assert_eq!(current, initial.as_ptr());
                assert_eq!(unsafe { (*initial.as_ptr()).ref_count }, 2);
                original.check(&interp).unwrap();
            }
            interp.remove_native_variable_observer(&token);
        }
    }
}
