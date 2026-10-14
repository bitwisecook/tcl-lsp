// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual registration receipts and caller cleanup. Software controls only.

use super::*;
use crate::obj::{self, Owned};
use std::cell::{Cell, RefCell};

fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn noop(_: &mut Interp, _: &[*mut TclObj]) -> Code {
    Code::Ok
}
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static OWNER: RefCell<Option<super::super::WeakInterp>> = const { RefCell::new(None) };
    static REACHED: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static OLD_COUNTS: RefCell<Vec<obj::TclSize>> = const { RefCell::new(Vec::new()) };
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|value| value.borrow_mut().take());
    }
}
fn arm(interpreter: &Interp, mode: u8) -> Cleanup {
    MODE.with(|value| value.set(mode));
    CALLS.with(|value| value.set(0));
    OWNER.with(|value| *value.borrow_mut() = Some(interpreter.downgrade()));
    OLD_COUNTS.with(|value| value.borrow_mut().clear());
    Cleanup
}
fn changed_context(interpreter: &mut Interp) {
    let original = interpreter.runtime_context();
    let mut changed = original.clone();
    changed
        .packages
        .push(("stock-publication-receipt".into(), "1.0".into()));
    interpreter.pin_context(&changed).unwrap();
    interpreter.pin_context(&original).unwrap();
}
fn observe(interpreter: &mut Interp, name: &[u8]) {
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    interpreter.set_result_bytes(b"PUBLICATION_REACHED");
    REACHED.with(|value| value.set(interpreter.result_obj()));
    match MODE.with(Cell::get) {
        1 => interpreter.register_builtin(name, noop),
        2 => {
            interpreter.refuse_host_command("original stock publication observer");
        }
        3 => changed_context(interpreter),
        4 => assert_eq!(
            interpreter.rename_command(name, b"receiptMoved"),
            crate::namespace::RenameOutcome::Renamed,
        ),
        _ => {}
    }
}
extern "C" fn retire_input(original: *mut TclObj) {
    CALLS.with(|value| value.set(value.get() + 1));
    OLD_COUNTS.with(|value| value.borrow_mut().push(unsafe { (*original).ref_count }));
    let Some(mut interpreter) = OWNER.with(|value| {
        value
            .borrow()
            .as_ref()
            .and_then(super::super::WeakInterp::upgrade)
    }) else {
        return;
    };
    observe(&mut interpreter, b"receiptRoot");
}
static INPUT: obj::TclObjType = obj::TclObjType {
    name: c"originalStockPublicationInput".as_ptr(),
    free_int_rep_proc: Some(retire_input),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn delete_observer(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    CALLS.with(|value| value.set(value.get() + 1));
    observe(interpreter, b"binary");
    Code::Ok
}

#[test]
fn original_procedure_publication_receipt_survives_actual_caller_cleanup_without_rebinding() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Shared input copies are the selected C body recipe. Fixed-slot rename refusal
    // is a software continuation bound, not a Native pointer-lifetime observation.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..5 {
            crate::counters::reset();
            {
                let mut interpreter = native(profile);
                let _cleanup = arm(&interpreter, mode);
                let scope = NativeOperationScope::enter(&interpreter).unwrap();
                let original = Owned::fresh(obj::alloc_typed(&INPUT, 0));
                unsafe { obj::set_string_rep(original.as_ptr(), b"return BODY") };
                let caller = Owned::retain(original.as_ptr());
                let publication = interpreter
                    .install_proc_original_storage_publication(
                        b"receiptRoot",
                        Vec::new(),
                        None,
                        original.as_ptr(),
                        None,
                        None,
                    )
                    .unwrap();
                let generation = publication.generation;
                assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 2);
                assert_eq!(CALLS.with(Cell::get), 0);
                drop(original);
                drop(caller);
                assert_eq!(CALLS.with(Cell::get), 1);
                assert_eq!(OLD_COUNTS.with(|value| value.borrow().clone()), [0]);
                let current = scope.currency().ensure_current_or_refuse().and_then(|()| {
                    interpreter.check_original_procedure_publication(
                        publication.namespace,
                        &publication.tail,
                        generation,
                        &publication.binding,
                    )
                });
                assert_eq!(current.is_ok(), mode == 0);
                assert_eq!(interpreter.result_obj(), REACHED.with(Cell::get));
                if mode == 0 {
                    assert_eq!(
                        interpreter.resolve_cmd_token(b"receiptRoot"),
                        Some(generation)
                    );
                    assert!(interpreter.native_execution_refusal().is_none());
                } else {
                    let first = interpreter.native_execution_refusal().unwrap();
                    if mode == 2 {
                        assert!(
                            matches!(first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) if first.reason == "original stock publication observer")
                        );
                    } else if mode != 3 {
                        assert!(matches!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("original procedure publication binding changed"))));
                    }
                    if mode == 1 {
                        assert_ne!(
                            interpreter.resolve_cmd_token(b"receiptRoot"),
                            Some(generation)
                        );
                    }
                    if mode == 4 {
                        assert_eq!(
                            interpreter.resolve_cmd_token(b"receiptMoved"),
                            Some(generation)
                        );
                    }
                }
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}

#[test]
fn scripted_stock_root_consumes_actual_publication_before_metadata_after_delete_callback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A genuine C command trace exercises the software installer seam. This
    // supplied scripted body does not issue a Jim distribution or Native trace grant.
    for mode in 0..4 {
        crate::counters::reset();
        {
            let mut interpreter = native("tcl8.6");
            let family = EnsembleImplementationFamily::Binary;
            let old = interpreter.stock_ensembles.borrow().families[&family].generation;
            assert!(!interpreter.stock_ensembles.borrow().families[&family].scripted);
            let _cleanup = arm(&interpreter, mode);
            interpreter.register_builtin(b"stockPublicationObserver", delete_observer);
            assert_eq!(
                interpreter.eval_str(b"trace add command binary delete stockPublicationObserver"),
                Code::Ok
            );
            interpreter.install_stock_scripted_binary(
                tcl_registry::native_binary_value::NativeBinaryScriptedIngress {
                    name: "binary",
                    parameters: "args",
                    body: "return SOFTWARE",
                    members: &[],
                },
                &[],
            );
            assert_eq!(CALLS.with(Cell::get), 1);
            let registrations = interpreter.stock_ensembles.borrow();
            let installed = &registrations.families[&family];
            if mode == 0 {
                assert!(installed.scripted);
                assert_ne!(installed.generation, old);
                assert_eq!(
                    interpreter.resolve_cmd_token(b"binary"),
                    Some(installed.generation)
                );
                assert!(matches!(
                    interpreter.namespaces().command_in(GLOBAL, b"binary"),
                    Some(Command::Proc(_))
                ));
                assert!(interpreter.native_execution_refusal().is_none());
            } else {
                assert!(!installed.scripted);
                assert_eq!(installed.generation, old);
                assert!(interpreter.native_execution_refusal().is_some());
                if mode == 1 {
                    let replacement = interpreter.resolve_cmd_token(b"binary").unwrap();
                    assert_ne!(replacement, old);
                    assert!(!interpreter
                        .native_compilation
                        .borrow()
                        .stock
                        .contains_key(&replacement));
                    assert!(!interpreter
                        .registry_builtin_names
                        .borrow()
                        .contains_key(&replacement));
                }
                if mode == 2 {
                    assert!(
                        matches!(interpreter.native_execution_refusal().unwrap(), tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) if first.reason == "original stock publication observer")
                    );
                }
            }
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}

#[test]
fn stock_helper_publication_preserves_real_existing_slot_and_stops_prior_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        crate::counters::reset();
        {
            let mut interpreter = native(profile);
            let family = EnsembleImplementationFamily::Binary;
            let mut installed = interpreter
                .stock_ensembles
                .borrow_mut()
                .families
                .remove(&family)
                .unwrap();
            interpreter.register_builtin(b"publicationExisting", noop);
            let existing = interpreter
                .resolve_cmd_token(b"publicationExisting")
                .unwrap();
            let count = installed.members.len();
            assert!(interpreter
                .register_stock_worker(b"publicationExisting", noop, &mut installed)
                .is_none());
            assert_eq!(
                interpreter.resolve_cmd_token(b"publicationExisting"),
                Some(existing)
            );
            assert_eq!(installed.members.len(), count);
            assert!(!interpreter
                .registry_builtin_names
                .borrow()
                .contains_key(&existing));
            assert!(!interpreter
                .native_compilation
                .borrow()
                .stock
                .contains_key(&existing));
            let published = interpreter
                .register_stock_worker(b"publicationFresh", noop, &mut installed)
                .unwrap();
            assert_eq!(
                interpreter.resolve_cmd_token(b"publicationFresh"),
                Some(published)
            );
            assert!(installed
                .members
                .iter()
                .any(|(name, generation)| name == b"publicationFresh" && *generation == published));
            assert!(interpreter
                .registry_builtin_names
                .borrow()
                .contains_key(&published));
            assert_eq!(
                interpreter
                    .native_compilation
                    .borrow()
                    .stock
                    .get(&published),
                Some(&b"publicationFresh".to_vec())
            );
            interpreter.set_result_bytes(b"PRIOR_HOST");
            let reached = interpreter.result_obj();
            interpreter.refuse_host_command("prior stock helper observer");
            assert!(interpreter
                .register_stock_worker(b"publicationBlocked", noop, &mut installed)
                .is_none());
            assert!(interpreter
                .namespaces()
                .command_in(GLOBAL, b"publicationBlocked")
                .is_none());
            assert_eq!(interpreter.result_obj(), reached);
            assert!(
                matches!(interpreter.native_execution_refusal().unwrap(), tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) if first.reason == "prior stock helper observer")
            );
            interpreter
                .stock_ensembles
                .borrow_mut()
                .families
                .insert(family, installed);
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}

#[test]
fn selected_jim_scripted_root_keeps_actual_helper_generations_on_reinstallation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    crate::counters::reset();
    {
        let mut interpreter = native("jim");
        let family = EnsembleImplementationFamily::Binary;
        let ingress = interpreter
            .native_invocation_dialect()
            .binary_scripted_ingress()
            .unwrap();
        let (old, helpers) = {
            let registrations = interpreter.stock_ensembles.borrow();
            let installed = &registrations.families[&family];
            assert!(installed.scripted);
            let helpers: Vec<_> = installed
                .scripted_helpers
                .iter()
                .map(|helper| {
                    (
                        helper.declared.namespace,
                        helper.declared.simple.clone(),
                        helper.declared.slot.clone(),
                        helper.generation,
                    )
                })
                .collect();
            (installed.generation, helpers)
        };
        assert_eq!(helpers.len(), 2);
        interpreter.install_stock_scripted_binary(ingress, &[noop, noop]);
        let registrations = interpreter.stock_ensembles.borrow();
        let installed = &registrations.families[&family];
        assert!(installed.scripted);
        assert_ne!(installed.generation, old);
        let current_helpers: Vec<_> = installed
            .scripted_helpers
            .iter()
            .map(|helper| {
                (
                    helper.declared.namespace,
                    helper.declared.simple.clone(),
                    helper.declared.slot.clone(),
                    helper.generation,
                )
            })
            .collect();
        assert_eq!(current_helpers, helpers);
        assert_eq!(
            interpreter.resolve_cmd_token(ingress.name.as_bytes()),
            Some(installed.generation)
        );
        for (namespace, simple, _, generation) in &helpers {
            assert_eq!(
                interpreter
                    .namespaces()
                    .command_generation(*namespace, simple.as_bytes()),
                Some(*generation)
            );
        }
        assert!(interpreter.native_execution_refusal().is_none());
        assert!(matches!(
            interpreter.namespaces().command_in(GLOBAL, b"binary"),
            Some(Command::Proc(_))
        ));
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}
