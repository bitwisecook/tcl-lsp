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
use crate::counters;

struct ExposureRetirement {
    interpreter: std::rc::Weak<InterpState>,
    calls: Rc<std::cell::Cell<usize>>,
    mode: u8,
}

unsafe extern "C" fn exposure_retire(data: *mut c_void) {
    // SAFETY: this exact command owns the client record until its once-only drop.
    let state = unsafe { Box::from_raw(data.cast::<ExposureRetirement>()) };
    state.calls.set(state.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().map(Interp).unwrap();
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    assert!(interpreter.hidden.try_borrow_mut().is_ok());
    assert!(interpreter.traces.try_borrow_mut().is_ok());
    match state.mode {
        1 => {
            interpreter.refuse_host_command("original exposure retirement refusal");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("exposure-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        4 => {
            interpreter.refuse_host_command("later unpublished binding retirement refusal");
        }
        3 => {
            assert_eq!(
                interpreter.rename_command(b"destination", b""),
                crate::namespace::RenameOutcome::Deleted,
            );
            interpreter.register_builtin(b"destination", guarded_builtin);
        }
        _ => {
            assert_eq!(
                interpreter.rename_command(b"destination", b"other::moved"),
                crate::namespace::RenameOutcome::Renamed,
            );
        }
    }
}

fn queue_exposure_retirement(interpreter: &mut Interp, mode: u8) -> Rc<std::cell::Cell<usize>> {
    let calls = Rc::new(std::cell::Cell::new(0));
    let state = Box::new(ExposureRetirement {
        interpreter: Rc::downgrade(&interpreter.0),
        calls: calls.clone(),
        mode,
    });
    interpreter
        .create_obj_command(
            b"exposure_cleanup",
            ObjCommand::new(
                publication_noop,
                Box::into_raw(state).cast(),
                Some(exposure_retire),
            ),
        )
        .unwrap();
    // Use the real extracted slot's deferred cleanup door, before expose enters.
    interpreter.namespaces.borrow_mut().bind(
        GLOBAL,
        b"exposure_cleanup",
        Command::Builtin(guarded_builtin),
    );
    assert_eq!(calls.get(), 0);
    calls
}

#[test]
fn original_exposure_after_cleanup_follows_actual_generation_for_traces_and_imports() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real software delete callback moves the token; no Native callback chronology is asserted.
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        ok(interpreter, b"proc kept {} {return VALUE}; namespace export kept; namespace eval imports {namespace import ::kept}; trace add command kept rename {list}");
        let original = interpreter
            .namespaces()
            .resolve_generation(GLOBAL, b"kept")
            .unwrap();
        assert_eq!(
            interpreter.hide_command(b"kept", b"hidden"),
            CommandVisibilityOutcome::Moved
        );
        let calls = queue_exposure_retirement(interpreter, 0);
        assert_eq!(
            interpreter.expose_command(b"hidden", b"destination"),
            CommandVisibilityOutcome::Moved
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(
            interpreter
                .namespaces()
                .command_fqn_by_generation(original)
                .as_deref(),
            Some(b"::other::moved".as_slice()),
        );
        assert!(interpreter
            .traces
            .borrow()
            .cmd_traces
            .iter()
            .any(|trace| { trace.token == Some(original) && trace.name == b"::other::moved" }));
        let imported = interpreter
            .namespaces()
            .resolve(GLOBAL, b"imports::kept")
            .unwrap();
        assert!(
            matches!(imported, Command::Imported { source, source_generation, .. }
                if source == b"::other::moved" && source_generation == original)
        );
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(ok(interpreter, b"imports::kept"), b"VALUE");
    });
}

#[cfg(debug_assertions)]
#[test]
fn original_exposure_cleanup_preserves_host_and_context_first_causes_before_retargeting() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Fixed genuine callback controls check software currency, not external provider outcomes.
    for mode in [1, 2] {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            ok(
                interpreter,
                b"proc kept {} {return VALUE}; trace add command kept rename {list}",
            );
            let original = interpreter
                .namespaces()
                .resolve_generation(GLOBAL, b"kept")
                .unwrap();
            assert_eq!(
                interpreter.hide_command(b"kept", b"hidden"),
                CommandVisibilityOutcome::Moved
            );
            let calls = queue_exposure_retirement(interpreter, mode);
            assert_eq!(
                interpreter.expose_command(b"hidden", b"destination"),
                CommandVisibilityOutcome::Missing
            );
            assert_eq!(calls.get(), 1);
            assert!(interpreter
                .traces
                .borrow()
                .cmd_traces
                .iter()
                .any(|trace| { trace.token == Some(original) && trace.name == b"::hidden" }));
            if mode == 1 {
                assert_eq!(
                    interpreter.host_command_refusal().as_deref(),
                    Some("original exposure retirement refusal")
                );
            } else {
                assert_eq!(interpreter.native_execution_refusal(), Some(
                        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                                "stale entered native operation",
                            ),
                        ),
                    ));
            }
        });
    }
}

#[test]
fn original_exposure_cleanup_declines_a_replacement_generation_at_the_requested_name() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A real software delete callback recreates the requested slot with another token.
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        ok(interpreter, b"proc kept {} {return VALUE}");
        let original = interpreter
            .namespaces()
            .resolve_generation(GLOBAL, b"kept")
            .unwrap();
        assert_eq!(
            interpreter.hide_command(b"kept", b"hidden"),
            CommandVisibilityOutcome::Moved
        );
        let calls = queue_exposure_retirement(interpreter, 3);
        assert_eq!(
            interpreter.expose_command(b"hidden", b"destination"),
            CommandVisibilityOutcome::Missing
        );
        assert_eq!(calls.get(), 1);
        assert!(interpreter
            .namespaces()
            .command_fqn_by_generation(original)
            .is_none());
        let replacement = interpreter
            .namespaces()
            .resolve_generation(GLOBAL, b"destination")
            .unwrap();
        assert_ne!(replacement, original);
        assert_eq!(
            interpreter.host_command_refusal().as_deref(),
            Some("exposed original command generation was retired")
        );
    });
}

thread_local! {
    static EXPOSURE_CONFIG_OWNER: std::cell::RefCell<Option<WeakInterp>> = const { std::cell::RefCell::new(None) };
    static EXPOSURE_CONFIG_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static EXPOSURE_CONFIG_FIRST: std::cell::RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { std::cell::RefCell::new(None) };
}

extern "C" fn exposure_configuration_retire(original: *mut TclObj) {
    EXPOSURE_CONFIG_CALLS.with(|calls| calls.set(calls.get() + 1));
    // SAFETY: the owning role passes its live original during final release.
    assert_eq!(unsafe { (*original).ref_count }, 0);
    let mut interpreter = EXPOSURE_CONFIG_OWNER.with(|owner| {
        owner
            .borrow()
            .as_ref()
            .and_then(WeakInterp::upgrade)
            .unwrap()
    });
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    assert!(interpreter.hidden.try_borrow_mut().is_ok());
    EXPOSURE_CONFIG_FIRST
        .with(|first| *first.borrow_mut() = interpreter.native_execution_refusal());
    interpreter.refuse_host_command("later unpublished ensemble configuration refusal");
}

static EXPOSURE_CONFIGURATION: obj::TclObjType = obj::TclObjType {
    name: c"originalExposureConfiguration".as_ptr(),
    free_int_rep_proc: Some(exposure_configuration_retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

#[test]
fn original_failed_exposure_retires_incoming_binding_outside_table_borrow_after_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real extension client-data release on this software unavailable-slot route only.
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let calls = Rc::new(std::cell::Cell::new(0));
        let state = Box::new(ExposureRetirement {
            interpreter: Rc::downgrade(&interpreter.0),
            calls: calls.clone(),
            mode: 4,
        });
        interpreter
            .create_obj_command(
                b"kept",
                ObjCommand::new(
                    publication_noop,
                    Box::into_raw(state).cast(),
                    Some(exposure_retire),
                ),
            )
            .unwrap();
        assert_eq!(
            interpreter.hide_command(b"kept", b"hidden"),
            CommandVisibilityOutcome::Moved
        );
        assert_eq!(
            interpreter.expose_command(b"hidden", b"::unavailable"),
            CommandVisibilityOutcome::Missing
        );
        assert_eq!(calls.get(), 1);
        assert!(interpreter.hidden_names().is_empty());
        assert_eq!(
            interpreter.host_command_refusal().as_deref(),
            Some("exposed command has no actual publication slot")
        );
    });
}

#[test]
fn original_failed_exposure_retires_actual_ensemble_configuration_after_publication_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A genuine owning configured role supplies its existing +1; no protective Native hold.
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        EXPOSURE_CONFIG_OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
        EXPOSURE_CONFIG_CALLS.with(|calls| calls.set(0));
        EXPOSURE_CONFIG_FIRST.with(|first| first.borrow_mut().take());
        let original = obj::Owned::fresh(obj::alloc_typed(&EXPOSURE_CONFIGURATION, 0));
        // SAFETY: this fresh owning role retains the live original.
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        let default_originals = crate::ensemble::NativeEnsembleObjects::default();
        let unknown = Some(tcl_cmd_core::ensemble::EnsembleObjectRole::new(
            crate::ensemble::NativeEnsembleRoot::owned(original),
        ));
        let originals = crate::ensemble::NativeEnsembleObjects {
            unknown,
            ..default_originals
        };
        let config = crate::ensemble::EnsembleConfig {
            originals,
            ns: GLOBAL,
            map: None,
            subcommands: None,
            prefixes: true,
            parameters: Vec::new(),
            unknown: Vec::new(),
        };
        let token = crate::ensemble::EnsembleToken::with_configuration_retirement(
            config,
            b"::kept".to_vec(),
            tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
            crate::ensemble::retire_configuration,
        );
        interpreter.bind_command_replacement(GLOBAL, b"kept", Command::Ensemble(Rc::new(token)));
        assert_eq!(
            interpreter.hide_command(b"kept", b"hidden"),
            CommandVisibilityOutcome::Moved
        );
        assert_eq!(
            interpreter.expose_command(b"hidden", b"::unavailable"),
            CommandVisibilityOutcome::Missing
        );
        assert_eq!(EXPOSURE_CONFIG_CALLS.with(std::cell::Cell::get), 1);
        let first = EXPOSURE_CONFIG_FIRST.with(|first| first.borrow().clone());
        assert!(first.is_some());
        assert_eq!(interpreter.native_execution_refusal(), first);
        assert_eq!(
            interpreter.host_command_refusal().as_deref(),
            Some("exposed command has no actual publication slot")
        );
        EXPOSURE_CONFIG_OWNER.with(|owner| owner.borrow_mut().take());
    });
}

struct PublicationRetirement {
    interpreter: std::rc::Weak<InterpState>,
    calls: Rc<std::cell::Cell<usize>>,
    visible: Rc<std::cell::Cell<Option<u64>>>,
    active_calls: Rc<std::cell::Cell<usize>>,
    mode: u8,
}

unsafe extern "C" fn publication_noop(
    _: *mut c_void,
    _: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    0
}

unsafe extern "C" fn publication_retire(data: *mut c_void) {
    // SAFETY: this command registered one boxed client record, consumed
    // exactly once by its delete procedure.
    let state = unsafe { Box::from_raw(data.cast::<PublicationRetirement>()) };
    state.calls.set(state.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().map(Interp).unwrap();
    state
        .visible
        .set(interpreter.resolve_cmd_token(b"retiring"));
    match state.mode {
        1 => {
            interpreter.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original delete procedure refusal",
                ),
            );
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("publication-retirement".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}

unsafe extern "C" fn publication_active(
    data: *mut c_void,
    interpreter: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    // SAFETY: the client record is live at callback entry. Replacement may
    // consume it, so retain only the observers before reentry.
    let state = unsafe { &*data.cast::<PublicationRetirement>() };
    let active_calls = state.active_calls.clone();
    let calls = state.calls.clone();
    let interpreter = unsafe { &mut *interpreter };
    let created = interpreter.create_obj_command(
        b"retiring",
        ObjCommand::new(publication_noop, core::ptr::null_mut(), None),
    );
    active_calls.set(calls.get());
    c_int::from(created.is_none())
}

fn retirement_state(interpreter: &Interp, mode: u8) -> Box<PublicationRetirement> {
    Box::new(PublicationRetirement {
        interpreter: Rc::downgrade(&interpreter.0),
        calls: Rc::new(std::cell::Cell::new(0)),
        visible: Rc::new(std::cell::Cell::new(None)),
        active_calls: Rc::new(std::cell::Cell::new(usize::MAX)),
        mode,
    })
}

struct PublicationNamespaceRetirement {
    interpreter: std::rc::Weak<InterpState>,
    namespace: NsId,
    visible: Rc<std::cell::Cell<Option<u64>>>,
    recreated: Rc<std::cell::Cell<Option<NsId>>>,
    calls: Rc<std::cell::Cell<usize>>,
    currency_current: Rc<std::cell::Cell<bool>>,
}

unsafe extern "C" fn publication_retire_namespace(data: *mut c_void) {
    // SAFETY: the original command owns this record until its sole final
    // retirement; the actual deletion callback consumes that ownership.
    let state = unsafe { Box::from_raw(data.cast::<PublicationNamespaceRetirement>()) };
    let mut interpreter = state.interpreter.upgrade().map(Interp).unwrap();
    state.calls.set(state.calls.get() + 1);
    let currency = native_operation_currency::NativeOperationCurrency::issue(&interpreter).unwrap();
    state.visible.set(
        interpreter
            .namespaces()
            .command_generation(state.namespace, b"retiring"),
    );
    interpreter.delete_namespace_by_id(state.namespace);
    state
        .recreated
        .set(Some(interpreter.ensure_global_namespace(b"::retirement")));
    state
        .currency_current
        .set(currency.ensure_current().is_ok());
}

fn publication_builtin(_: &mut Interp, _: &[*mut TclObj]) -> Code {
    Code::Ok
}

#[test]
fn original_publication_holder_refuses_delete_and_recreated_namespace() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software namespace retirement without an interpreter epoch
    // change; no new native-provider timing or discard observation.
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let namespace = interpreter.ensure_global_namespace(b"::retirement");
        let calls = Rc::new(std::cell::Cell::new(0));
        let recreated = Rc::new(std::cell::Cell::new(None));
        let currency_current = Rc::new(std::cell::Cell::new(false));
        let visible = Rc::new(std::cell::Cell::new(None));
        let state = Box::new(PublicationNamespaceRetirement {
            interpreter: Rc::downgrade(&interpreter.0),
            namespace,
            visible: visible.clone(),
            recreated: recreated.clone(),
            calls: calls.clone(),
            currency_current: currency_current.clone(),
        });
        let original = interpreter
            .create_obj_command(
                b"::retirement::retiring",
                ObjCommand::new(
                    publication_noop,
                    Box::into_raw(state).cast(),
                    Some(publication_retire_namespace),
                ),
            )
            .unwrap();
        let currency =
            native_operation_currency::NativeOperationCurrency::issue(interpreter).unwrap();
        let original_context = interpreter.runtime_context();
        interpreter.set_result_bytes(b"SEEDED RESULT");
        let result = interpreter.get_obj_result();
        let replacement = interpreter.create_obj_command(
            b"::retirement::retiring",
            ObjCommand::new(publication_noop, core::ptr::null_mut(), None),
        );
        assert!(replacement.is_none());
        assert_eq!(calls.get(), 1);
        assert_eq!(visible.get(), Some(original));
        assert!(
            currency_current.get(),
            "namespace retirement did not change interpreter currency"
        );
        let fresh = recreated.get().unwrap();
        assert_ne!(fresh, namespace);
        assert!(interpreter.namespaces().namespace_is_live(fresh));
        assert!(interpreter
            .namespaces()
            .command_in(namespace, b"retiring")
            .is_none());
        assert!(interpreter
            .namespaces()
            .command_in(fresh, b"retiring")
            .is_none());
        assert_eq!(interpreter.runtime_context(), original_context);
        assert_eq!(interpreter.get_obj_result(), result);
        let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original command publication namespace retired",
            ),
        );
        assert_eq!(interpreter.native_execution_refusal(), Some(first.clone()));
        assert_eq!(currency.ensure_current(), Err(first));
    });
}

#[test]
fn original_publication_holder_refuses_initial_dead_or_absent_namespace() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for absent in [false, true] {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            let namespace = interpreter.ensure_global_namespace(b"::retired");
            interpreter.delete_namespace_by_id(namespace);
            let selected = if absent { usize::MAX } else { namespace };
            let root_children = interpreter.namespaces().children(GLOBAL);
            interpreter.bind_command_replacement(
                selected,
                b"untouched",
                Command::Builtin(publication_builtin),
            );
            assert!(interpreter
                .namespaces()
                .command_in(namespace, b"untouched")
                .is_none());
            assert_eq!(interpreter.namespaces().children(GLOBAL), root_children);
            assert_eq!(
                interpreter.native_execution_refusal(),
                Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original command publication namespace retired",
                    ),
                ),)
            );
        });
    }
}

#[test]
fn original_publication_holder_preserves_retained_dying_and_permanent_root_tokens() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The actual retained arena holder exercises the shared publication
    // seam. Independent original TclOO source comparisons retain OO scope.
    leak_free(|interpreter| {
        use tcl_syntax::native_namespace_name::NativeNamespaceLifecycle;
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let namespace = interpreter.ensure_global_namespace(b"::dying");
        let retained = interpreter
            .namespaces()
            .namespace_name_token(interpreter.native_command_interpreter, namespace)
            .unwrap();
        interpreter
            .namespaces_mut()
            .begin_namespace_teardown(namespace);
        assert_eq!(retained.lifecycle(), NativeNamespaceLifecycle::Dying);
        interpreter.bind_command_replacement(
            namespace,
            b"retained",
            Command::Builtin(publication_builtin),
        );
        assert!(interpreter
            .namespaces()
            .command_in(namespace, b"retained")
            .is_some());
        assert!(interpreter
            .namespaces()
            .owns_namespace_name_token(&retained));
        assert_eq!(retained.lifecycle(), NativeNamespaceLifecycle::Dying);
        assert!(interpreter.native_execution_refusal().is_none());
        interpreter
            .namespaces_mut()
            .delete_namespace_by_id(namespace);
        assert_eq!(retained.lifecycle(), NativeNamespaceLifecycle::Dead);
        let root = interpreter
            .namespaces()
            .namespace_name_token(interpreter.native_command_interpreter, GLOBAL)
            .unwrap();
        interpreter
            .namespaces_mut()
            .begin_namespace_teardown(GLOBAL);
        assert_eq!(root.lifecycle(), NativeNamespaceLifecycle::Dying);
        interpreter.namespaces_mut().delete_namespace_by_id(GLOBAL);
        interpreter
            .namespaces_mut()
            .finish_namespace_teardown(&[GLOBAL]);
        assert_eq!(root.lifecycle(), NativeNamespaceLifecycle::Live);
        interpreter.bind_command_replacement(
            GLOBAL,
            b"root",
            Command::Builtin(publication_builtin),
        );
        assert!(interpreter
            .namespaces()
            .command_in(GLOBAL, b"root")
            .is_some());
        assert!(interpreter.namespaces().owns_namespace_name_token(&root));
        assert!(interpreter.native_execution_refusal().is_none());
    });
}

#[cfg(debug_assertions)]
#[test]
fn idle_original_delete_procedure_runs_before_publication_and_preserves_first_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software object-command retirement, not a new native-provider
    // observation or an assertion about callback-recreated-slot discard.
    for mode in 0..3 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            let state = retirement_state(interpreter, mode);
            let calls = state.calls.clone();
            let visible = state.visible.clone();
            let original = interpreter
                .create_obj_command(
                    b"retiring",
                    ObjCommand::new(
                        publication_noop,
                        Box::into_raw(state).cast(),
                        Some(publication_retire),
                    ),
                )
                .unwrap();
            interpreter.set_result_bytes(b"SEEDED RESULT");
            let result = interpreter.get_obj_result();
            let replacement = interpreter.create_obj_command(
                b"retiring",
                ObjCommand::new(publication_noop, core::ptr::null_mut(), None),
            );
            assert_eq!(calls.get(), 1);
            assert_eq!(
                visible.get(),
                Some(original),
                "the exact original slot remains visible through deleteProc"
            );
            assert_eq!(interpreter.get_obj_result(), result);
            match mode {
                0 => {
                    assert_ne!(replacement.unwrap(), original);
                    assert!(interpreter.native_execution_refusal().is_none());
                }
                _ => {
                    assert!(replacement.is_none());
                    assert!(interpreter.resolve_cmd_token(b"retiring").is_none());
                    let reason = if mode == 1 {
                        "original delete procedure refusal"
                    } else {
                        "stale entered native operation"
                    };
                    assert_eq!(interpreter.native_execution_refusal(), Some(
                            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(reason))));
                }
            }
        });
    }
}

#[test]
fn active_public_command_replacement_retires_client_data_before_dispatch_releases_its_handle() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Public creation calls token deletion before replacement returns:
    // tclBasic.c C8.4.20:1737-1869/2550-2696, C8.5.19:1998-2153/2774-2939,
    // C8.6.18:2334-2502/3147-3320, C9.0.4:2924-3077/3727-3902,
    // C9.1.0:2879-3033/3697-3872. Active handles retain allocation only.
    // Witness671 measures active explicit deletion with stack client records;
    // self-replacement's path comes from source, not that explicit-deletion witness.
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let state = retirement_state(interpreter, 0);
        let calls = state.calls.clone();
        let active_calls = state.active_calls.clone();
        let original = interpreter
            .create_obj_command(
                b"retiring",
                ObjCommand::new(
                    publication_active,
                    Box::into_raw(state).cast(),
                    Some(publication_retire),
                ),
            )
            .unwrap();
        let word = obj::Owned::fresh(new_string(b"retiring"));
        let refs = unsafe { (*word.as_ptr()).ref_count };
        assert_eq!(interpreter.dispatch(&[word.as_ptr()]), Code::Ok);
        assert_eq!(
            active_calls.get(),
            1,
            "public replacement retired clientData inside the actual call"
        );
        assert_eq!(
            calls.get(),
            1,
            "the original delete procedure ran exactly once"
        );
        assert_ne!(
            interpreter.resolve_cmd_token(b"retiring").unwrap(),
            original
        );
        assert_eq!(unsafe { (*word.as_ptr()).ref_count }, refs);
        assert!(interpreter.native_execution_refusal().is_none());
    });
}

#[test]
fn c_command_publication_preserves_pending_host_before_namespace_or_output_changes() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        interpreter
            .create_obj_command(
                b"kept",
                ObjCommand::new(publication_noop, core::ptr::null_mut(), None),
            )
            .unwrap();
        let kept = interpreter.resolve_cmd_token(b"kept");
        let children = interpreter.namespaces().children(crate::namespace::GLOBAL);
        interpreter.set_result_bytes(b"SEEDED RESULT");
        let result = interpreter.get_obj_result();
        interpreter.refuse_host_command("original C publication refusal");
        let first = interpreter.native_execution_refusal();
        assert!(unsafe {
            crate::capi::Tcl_CreateObjCommand(
                interpreter,
                c"untouched::command".as_ptr(),
                Some(publication_noop),
                core::ptr::null_mut(),
                None,
            )
        }
        .is_null());
        assert_eq!(
            unsafe { crate::capi::Tcl_DeleteCommand(interpreter, c"kept".as_ptr()) },
            -1
        );
        assert_eq!(interpreter.resolve_cmd_token(b"kept"), kept);
        assert_eq!(
            interpreter.namespaces().children(crate::namespace::GLOBAL),
            children
        );
        assert_eq!(interpreter.get_obj_result(), result);
        assert_eq!(interpreter.native_execution_refusal(), first);
    });
}

/// Seed the actual selected error-code storage used by the native probes.
/// ExceptionState bytes alone do not publish C8.4's global errorCode cell.
fn seed_original_getter_code(interp: &mut Interp, bytes: &[u8]) {
    interp.set_error_state(bytes);
    let code = crate::obj::Owned::fresh(new_string(bytes));
    // SAFETY: the actual interpreter and borrowed original code header are live.
    unsafe { crate::capi::Tcl_SetObjErrorCode(interp, code.as_ptr()) };
    assert!(interp.native_execution_refusal().is_none());
    assert_eq!(interp.error_code(), bytes);
}

#[test]
fn c84_primitive_invalid_code_is_preserved_until_actual_script_propagation() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // naming.numeric.seeded-wide-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-seeded-wide-frontier.md
    // Software publication at the actual Runtime boundary, independently
    // of the public primitive fields and the original callback captures.
    use tcl_syntax::scalar_getter::{
        NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterProtocol,
    };
    leak_free(|interp| {
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_4);
        for kind in [
            NativeScalarGetterKind::Int,
            NativeScalarGetterKind::Long,
            NativeScalarGetterKind::Wide,
            NativeScalarGetterKind::Double,
            NativeScalarGetterKind::Boolean,
        ] {
            seed_original_getter_code(interp, b"SEEDED CODE");
            let record = protocol
                .failure_presentation(kind, NativeScalarGetterFailure::Invalid, b"bad")
                .unwrap();
            let message = record.message_bytes().to_vec();
            let error = tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record));
            assert_eq!(interp.report_cmd_error(error.into()), Code::Error);
            assert_eq!(interp.error_code(), b"SEEDED CODE", "{kind:?} direct");
            assert_eq!(interp.result_bytes(), message);
            interp.log_command_bytes(1, b"original_getter");
            assert_eq!(interp.error_code(), b"NONE", "{kind:?} Eval");
            assert!(interp.exc.borrow().primitive_getter.is_none());
        }
        seed_original_getter_code(interp, b"SEEDED CODE");
        let record = protocol
            .failure_presentation(
                NativeScalarGetterKind::Long,
                NativeScalarGetterFailure::IntegerOverflow,
                b"18446744073709551616",
            )
            .unwrap();
        assert_eq!(
            interp.report_cmd_error(
                tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record)).into(),
            ),
            Code::Error
        );
        let code = interp.error_code();
        assert!(code.starts_with(b"ARITH IOVERFLOW"));
        interp.log_command_bytes(1, b"original_getter");
        assert_eq!(interp.error_code(), code);
    });
}

#[test]
fn primitive_getter_preserves_seeded_code_until_script_propagation() {
    use tcl_syntax::scalar_getter::{
        NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterProtocol,
    };
    leak_free(|interp| {
        interp.set_error_state(b"PROBE BEFORE");
        interp.mark_error_code_explicit();
        let protocol = NativeScalarGetterProtocol::for_tcl_version(tcl_dialect::TclVersion::V8_5);
        let record = protocol
            .failure_presentation(
                NativeScalarGetterKind::Wide,
                NativeScalarGetterFailure::CachedNonInteger,
                b"1.5\0X",
            )
            .expect("audited cached C8.5 getter failure");
        let primitive_message = record.message_bytes().to_vec();
        let propagated_message = record.eval_message_bytes().to_vec();
        assert_ne!(primitive_message, propagated_message);
        let error = tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record));
        assert_eq!(interp.report_cmd_error(error.into()), Code::Error);
        assert_eq!(interp.result_bytes(), primitive_message);
        assert_eq!(interp.error_code(), b"PROBE BEFORE");
        assert!(interp.exc.borrow().code_explicit);

        let snapshot = interp.snapshot_error();
        interp.error_with_code(b"other", b"OTHER");
        interp.set_result_bytes(&primitive_message);
        let _ = interp.restore_error(snapshot);
        interp.log_command_bytes(1, b"getter");
        assert_eq!(interp.result_bytes(), propagated_message);
        assert_eq!(interp.error_code(), b"PROBE BEFORE");
        assert!(interp.exc.borrow().primitive_getter.is_none());

        let record = protocol
            .failure_presentation(
                NativeScalarGetterKind::Wide,
                NativeScalarGetterFailure::CachedNonInteger,
                b"1.5\0X",
            )
            .expect("same primitive stage inside contextual command error");
        let error = tcl_cmd_core::CmdError::from(
            tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record)),
        );
        let mut details = error.into_byte_details();
        details.message.splice(0..0, b"context: ".iter().copied());
        interp.report_cmd_error(tcl_cmd_core::CmdError::from_byte_details(details));
        interp.log_command_bytes(1, b"getter");
        let mut wrapped = b"context: ".to_vec();
        wrapped.extend_from_slice(&propagated_message);
        assert_eq!(interp.result_bytes(), wrapped);
    });
}

#[test]
#[cfg(have_tommath)]
fn c84_direct_invalid_expression_preserves_code_until_eval() {
    // Native proof: naming.expression.original-direct-arithmetic-error-stage
    // docs/design/analysis/name-resolution-proofs/expression-original-direct-arithmetic-error-stage.md

    leak_free(|interp| {
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        for source in [b"\"bad\" + 1".as_slice(), b"\"bad\" ? 1 : 0".as_slice()] {
            seed_original_getter_code(interp, b"PROBE BEFORE");
            assert_eq!(
                crate::builtins::eval_expr_obj(interp, source),
                Err(Code::Error)
            );
            assert_eq!(interp.error_code(), b"PROBE BEFORE");
            assert!(interp.exc.borrow().expression_error_stage.is_some());
            interp.log_command_bytes(1, b"expr");
            assert_eq!(interp.error_code(), b"NONE");
        }
        seed_original_getter_code(interp, b"PROBE BEFORE");
        assert_eq!(
            crate::builtins::eval_expr_obj(interp, b"NaN + 1"),
            Err(Code::Error)
        );
        assert_ne!(interp.error_code(), b"PROBE BEFORE");
        assert!(interp.exc.borrow().expression_error_stage.is_none());
        let domain = interp.error_code();
        interp.log_command_bytes(1, b"expr");
        assert_eq!(interp.error_code(), domain);
    });
}

#[test]
#[cfg(have_tommath)]
fn counted_syntax_failures_keep_direct_code_and_apply_eval_update() {
    for &(engine, case, source, message, direct, propagated) in
        tcl_test_support::expressions::NATIVE_EXPRESSION_SYNTAX_STATES
    {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect(engine));
            interp.set_error_state(b"PROBE BEFORE");
            assert_eq!(
                crate::builtins::eval_expr_obj(interp, source),
                Err(Code::Error),
                "{engine}/{case}"
            );
            assert_eq!(interp.result_bytes(), message, "{engine}/{case}");
            assert_eq!(interp.error_code(), direct, "{engine}/{case} Direct");
            let snapshot = interp.snapshot_error();
            interp.error_with_code(b"other failure", b"OTHER");
            interp.set_result_bytes(message);
            let _ = interp.restore_error(snapshot);
            interp.log_command_bytes(1, b"expr $probeSource");
            assert_eq!(interp.error_code(), propagated, "{engine}/{case} Eval");
            assert_eq!(interp.result_bytes(), message, "{engine}/{case}");
            assert!(interp.exc.borrow().expression_error_stage.is_none());
        });
    }
}

#[test]
fn native_wrong_arguments_string_result_requires_the_structured_presenter() {
    // naming.variable.original-upvar-and-exists-completion-and-name-windows
    // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
    // naming.list.original-assign-objects-and-instructions
    // docs/design/analysis/name-resolution-proofs/list.original-assign-objects-and-instructions.md
    // The original tables independently observe C84 upvar case11 and
    // C85 lassign case28 String results. This tests the shared presenter.
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let error = tcl_cmd_core::CmdError::wrong_args_bytes(b"upvar ?level? otherVar localVar");
        let message = error.message_bytes().to_vec();
        assert_eq!(interp.report_cmd_error(error), Code::Error);
        assert!(!interp.host_refusal_pending(), "{engine}");
        assert!(
            matches!(
                obj::native_object_snapshot(interp.result_obj())
                    .unwrap()
                    .cache,
                Cache::String { .. }
            ),
            "{engine}"
        );
        assert_eq!(interp.result_bytes(), message, "{engine}");
        assert_eq!(
            interp.report_cmd_error(tcl_cmd_core::CmdError::new_bytes(message.as_slice())),
            Code::Error
        );
        assert!(
            matches!(
                obj::native_object_snapshot(interp.result_obj())
                    .unwrap()
                    .cache,
                Cache::None
            ),
            "{engine}: unrelated message"
        );
    }
}

#[test]
fn unchanged_primitive_projection_preserves_the_original_string_result() {
    // naming.compiler.introspection-source-and-effect-frontiers
    // docs/design/analysis/name-resolution-proofs/compiler-introspection-source-and-effect-frontiers.md
    // Native case13 observes the selected integer-getter String result.
    // This separate owner control checks unchanged propagation identity.
    use tcl_syntax::scalar_getter::{
        NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterProtocol,
    };
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let version = interp.native_invocation_dialect().tcl_version.unwrap();
        let record = NativeScalarGetterProtocol::for_tcl_version(version)
            .failure_presentation(
                NativeScalarGetterKind::Int,
                NativeScalarGetterFailure::Invalid,
                b"invalid",
            )
            .expect("retained native integer failure");
        let error = tcl_cmd_core::CmdError::from(
            tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record)),
        )
        .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::C(version));
        assert_eq!(interp.report_cmd_error(error), Code::Error);
        let original = obj::Owned::retain(interp.result_obj());
        let before = obj::native_object_snapshot(original.as_ptr()).unwrap();
        assert!(matches!(
            before.cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
        ));
        interp.propagate_error_stage().unwrap();
        assert_eq!(interp.result_obj(), original.as_ptr(), "{engine}");
        assert_eq!(
            obj::native_object_snapshot(interp.result_obj())
                .unwrap()
                .cache,
            before.cache,
            "{engine}"
        );
        assert!(interp.exc.borrow().primitive_getter.is_none());
    }
}

#[test]
fn command_error_default_and_explicit_updates_replace_seeded_code() {
    for (environment, expected) in [
        ("tcl8.4", b"NONE".as_slice()),
        ("tcl8.5", b"NONE".as_slice()),
        ("tcl8.6", b"TCL WRONGARGS".as_slice()),
        ("tcl9.0", b"TCL WRONGARGS".as_slice()),
        ("tcl9.1", b"TCL WRONGARGS".as_slice()),
        ("jim", b"NONE".as_slice()),
    ] {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            interp.set_error_state(b"PROBE BEFORE");
            interp.mark_error_code_explicit();
            let authentic = tcl_cmd_core::CmdError::wrong_args_bytes(b"n\0\xc0\x80\xff arg");
            let original = authentic.message_bytes().to_vec();
            interp.report_cmd_error(tcl_cmd_core::CmdError::new_bytes(original.as_slice()));
            assert_eq!(interp.result_bytes(), original);
            assert_eq!(interp.error_code(), b"NONE", "{environment}");
            assert!(!interp.exc.borrow().code_explicit);
            interp.report_cmd_error(authentic);
            assert_eq!(interp.result_bytes(), original);
            assert_eq!(interp.error_code(), expected, "{environment}");
            interp.report_cmd_error(tcl_cmd_core::CmdError::with_error_code("bad", "EXACT CODE"));
            assert_eq!(interp.error_code(), b"EXACT CODE");
        });
    }
}

#[test]
fn missing_arity_protocol_refuses_before_guest_state_changes() {
    leak_free(|interp| {
        let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )));
        interp.set_dialect_profile(unknown);
        // Bootstrap release globals use their owned slots; installing an
        // unsupported profile does not issue an unrelated variable lookup.
        assert_eq!(interp.native_access_refusal(), None);
        assert!(interp
                .native_invocation_dialect()
                .wrong_arguments_protocol(Some(tcl_registry::native_wrong_arguments::LogicalWrongArgumentsProvider::Tcl84CoreSimulation))
                .is_none());
        interp.set_result_bytes(b"RESULT BEFORE");
        interp.set_error_state(b"CODE BEFORE");
        assert_eq!(
            interp.report_cmd_error(tcl_cmd_core::CmdError::wrong_args("native arg")),
            Code::Error
        );
        assert_eq!(interp.result_bytes(), b"RESULT BEFORE");
        assert_eq!(interp.exc.borrow().code, b"CODE BEFORE");
        assert_eq!(
            interp.error_code_bytes_checked().unwrap_err(),
            interp.native_execution_refusal().unwrap()
        );
        assert_eq!(
            interp.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "wrong arguments",
                ),
            )
        );
    });
}

#[test]
fn jim_diagnostic_argv_borrows_preserve_sharing_and_lazy_string_representation() {
    fn probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        // SAFETY: invocation owns the argument through this callback.
        assert_eq!(unsafe { (*argv[1]).ref_count }, 1);
        assert!(unsafe { (*argv[1]).bytes.is_null() });
        // SAFETY: argv is live; set_obj_result takes its own result hold.
        unsafe { interp.set_obj_result(argv[1]) };
        Code::Ok
    }
    fn fail(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        assert_eq!(unsafe { (*argv[1]).ref_count }, 1);
        assert!(unsafe { (*argv[1]).bytes.is_null() });
        interp.error(b"BOOM")
    }
    let mut interp = Interp::new();
    interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
    interp
        .jim_evaluation_frames
        .borrow_mut()
        .push(OriginalJimEvaluationFrame {
            procedure_level: 0,
            command: None,
            is_procedure: false,
            script: None,
            invocation: Vec::new(),
        });
    for (callback, expected) in [
        (probe as BuiltinFn, Code::Ok),
        (fail as BuiltinFn, Code::Error),
    ] {
        let argv = [new_string(b"probe"), obj::new_wide_int_obj(42)];
        for word in argv {
            unsafe { obj::incr_ref_count(word) };
        }
        assert_eq!(
            interp.invoke_bound(Command::Builtin(callback), None, &argv),
            expected
        );
        assert!(interp.jim_invocation_borrows.borrow().is_empty());
        if expected == Code::Ok {
            assert!(unsafe { (*argv[1]).bytes.is_null() });
        } else {
            assert_eq!(interp.jim_stacktrace().unwrap(), b"{} {} 1 {probe 42}");
        }
        release_all(&argv);
        interp.set_result_bytes(b"");
    }
    interp.jim_evaluation_frames.borrow_mut().pop();
}

struct SyntheticHost {
    clock: SyntheticClock,
    stdio: SyntheticStdIo,
    env: SyntheticEnv,
}

impl SyntheticHost {
    fn new(entries: &[(&str, &str)]) -> Self {
        Self {
            clock: SyntheticClock,
            stdio: SyntheticStdIo,
            env: SyntheticEnv(RefCell::new(
                entries
                    .iter()
                    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                    .collect(),
            )),
        }
    }
}

struct SyntheticClock;

impl tcl_platform::Clock for SyntheticClock {
    fn now_secs(&self) -> i64 {
        0
    }

    fn now_millis(&self) -> i128 {
        0
    }
}

struct SyntheticStdIo;

impl tcl_platform::StdIo for SyntheticStdIo {
    fn write_stdout(&self, _bytes: &[u8]) {}

    fn write_stderr(&self, _bytes: &[u8]) {}
}

struct SyntheticEnv(RefCell<std::collections::BTreeMap<String, String>>);

impl tcl_platform::Env for SyntheticEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.0.borrow().get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) {
        self.0
            .borrow_mut()
            .insert(key.to_string(), value.to_string());
    }
    fn unset(&self, key: &str) {
        self.0.borrow_mut().remove(key);
    }

    fn vars(&self) -> Vec<(String, String)> {
        self.0
            .borrow()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    fn cwd(&self) -> Result<String, tcl_platform::HostError> {
        Ok("/synthetic".to_string())
    }

    fn chdir(&self, _path: &str) -> Result<(), tcl_platform::HostError> {
        Ok(())
    }
}

impl tcl_platform::Host for SyntheticHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        tcl_platform::Capabilities::empty()
    }

    fn clock(&self) -> &dyn tcl_platform::Clock {
        &self.clock
    }

    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        &self.stdio
    }

    fn env(&self) -> &dyn tcl_platform::Env {
        &self.env
    }
}

const GUARDED_IDENTITY: GuardIdentity = GuardIdentity::new(1, 71);

fn guarded_builtin(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"");
    Code::Ok
}

fn resultless_builtin(_interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    Code::Ok
}

fn prepare_interpreter_guard(interp: &mut Interp) -> GuardToken {
    interp.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
    interp
        .prepare_command_guard(
            b"guarded",
            GUARDED_IDENTITY,
            GuardDomains::one(GuardDomain::Interpreter),
        )
        .expect("interpreter domain should be guardable")
}

fn assert_interpreter_guard_stale(interp: &mut Interp, token: GuardToken) {
    // The command stays attested through every mutation below, so a failed
    // check proves the Interpreter epoch changed.
    assert!(interp.attested_identities(b"guarded").is_some());
    assert!(!interp.check_command_guard(token, b"guarded"));
}

fn registration_host_worker(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    interpreter.set_result_bytes(b"HOST");
    Code::Ok
}

#[test]
fn namespace_child_birth_and_export_clear_use_the_shared_lookup_revision() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        let namespace = interpreter
            .namespaces_mut()
            .ensure_namespace(GLOBAL, b"::RegistrationExisting");
        interpreter.register_guarded_builtin(
            b"registrationGuarded",
            registration_host_worker,
            GUARDED_IDENTITY,
        );
        let domains = [
            GuardDomain::CommandEnvironment,
            GuardDomain::Namespace,
            GuardDomain::UnknownHandling,
        ];
        let prepare = |interpreter: &mut Interp| {
            domains.map(|domain| {
                interpreter
                    .prepare_command_guard(
                        b"registrationGuarded",
                        GUARDED_IDENTITY,
                        GuardDomains::one(domain),
                    )
                    .unwrap()
            })
        };
        let tokens = prepare(interpreter);
        assert_eq!(
            interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"::RegistrationExisting"),
            namespace
        );
        interpreter.namespaces_mut().clear_exports(namespace);
        interpreter.register_builtin(
            b"::RegistrationExisting::unrelated",
            registration_host_worker,
        );
        assert!(tokens
            .iter()
            .all(|token| interpreter.check_command_guard(*token, b"registrationGuarded")));
        // This is the actual command-publication child birth, rather than
        // an Interp facade adding an invalidation after table mutation.
        interpreter.register_builtin(b"::RegistrationBorn::unrelated", registration_host_worker);
        assert!(tokens.iter().zip(domains).all(|(token, domain)| {
            interpreter.check_command_guard(*token, b"registrationGuarded")
                == (domain == GuardDomain::CommandEnvironment)
        }));
        for token in tokens {
            assert!(interpreter.release_command_guard(token));
        }
        interpreter.namespaces_mut().export(namespace, b"unrelated");
        let tokens = prepare(interpreter);
        interpreter.namespaces_mut().clear_exports(namespace);
        assert!(tokens
            .iter()
            .all(|token| !interpreter.check_command_guard(*token, b"registrationGuarded")));
        for token in tokens {
            assert!(interpreter.release_command_guard(token));
        }
    });
}

#[cfg(debug_assertions)]
unsafe extern "C" fn registration_ensemble_retirement(data: *mut c_void) {
    // The actual old ObjCommand transfers its sole client record here.
    let state = unsafe { Box::from_raw(data.cast::<PublicationRetirement>()) };
    state.calls.set(state.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().map(Interp).unwrap();
    match state.mode {
        0 => interpreter.register_builtin(b"retiring", registration_host_worker),
        1 => {
            interpreter.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original ensemble delete refusal",
                ),
            );
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("ensemble-publication-retirement".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
}

#[cfg(debug_assertions)]
#[test]
fn stock_root_installation_keeps_the_actual_publication_receipt_after_retirement() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Runtime callbacks check custody/refusal, not native ensemble chronology.
    for mode in 0..4 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            let state = retirement_state(interpreter, mode);
            let calls = state.calls.clone();
            let old = interpreter
                .create_obj_command(
                    b"retiring",
                    ObjCommand::new(
                        publication_noop,
                        Box::into_raw(state).cast(),
                        Some(registration_ensemble_retirement),
                    ),
                )
                .unwrap();
            interpreter.register_stock_ensemble(
                tcl_registry::invocation_words::EnsembleImplementationFamily::String,
                b"retiring",
                registration_host_worker,
                &[(b"length", registration_host_worker)],
                &[b"length".as_slice()],
            );
            assert_eq!(calls.get(), 1);
            let current = interpreter.resolve_cmd_token(b"retiring");
            if mode == 3 {
                let generation = current.unwrap();
                assert_ne!(generation, old);
                assert!(interpreter.native_execution_refusal().is_none());
                assert_eq!(
                    interpreter
                        .native_compilation
                        .borrow()
                        .stock
                        .get(&generation),
                    Some(&b"retiring".to_vec())
                );
                assert!(matches!(
                    interpreter.raw_command_by_generation(generation),
                    Some(Command::Ensemble(_))
                ));
            } else {
                assert!(interpreter.native_execution_refusal().is_some());
                if let Some(generation) = current {
                    assert!(!interpreter
                        .native_compilation
                        .borrow()
                        .stock
                        .contains_key(&generation));
                    assert!(!interpreter
                        .registry_builtin_names
                        .borrow()
                        .contains_key(&generation));
                }
                if mode == 0 {
                    assert!(matches!(
                        interpreter.raw_command_by_generation(current.unwrap()),
                        Some(Command::Builtin(_))
                    ));
                }
            }
        });
    }
}

std::thread_local! {
    static REGISTRATION_NESTED_RETIREMENT_MODE: std::cell::Cell<u8> = const { std::cell::Cell::new(3) };
    static REGISTRATION_NESTED_RETIREMENT_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn registration_nested_retirement(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
    REGISTRATION_NESTED_RETIREMENT_CALLS.with(|calls| calls.set(calls.get() + 1));
    match REGISTRATION_NESTED_RETIREMENT_MODE.with(std::cell::Cell::get) {
        0 => interpreter.register_builtin(b"::tcl::binary::encode", registration_host_worker),
        1 => {
            interpreter.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original nested ensemble delete refusal",
                ),
            );
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("nested-ensemble-retirement".into(), "1.0".into())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        _ => {}
    }
    Code::Ok
}

#[test]
fn stock_nested_installation_stops_before_attesting_a_refused_or_recreated_slot() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The actual command delete trace, not an invented native delete role,
    // supplies this software callback on the installed nested ensemble.
    for mode in 0..4 {
        leak_free(|interpreter| {
            interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            REGISTRATION_NESTED_RETIREMENT_MODE.with(|value| value.set(mode));
            REGISTRATION_NESTED_RETIREMENT_CALLS.with(|value| value.set(0));
            interpreter.register_builtin(
                b"registrationNestedRetirement",
                registration_nested_retirement,
            );
            let name = b"::tcl::binary::encode";
            let old = interpreter.resolve_cmd_token(name).unwrap();
            assert_eq!(
                interpreter.eval_str(
                    b"trace add command ::tcl::binary::encode delete registrationNestedRetirement"
                ),
                Code::Ok
            );
            interpreter.register_stock_nested_ensemble(
                tcl_registry::invocation_words::EnsembleImplementationFamily::Binary,
                name,
                registration_host_worker,
                &[(b"hex", registration_host_worker)],
            );
            assert_eq!(
                REGISTRATION_NESTED_RETIREMENT_CALLS.with(std::cell::Cell::get),
                1
            );
            let current = interpreter.resolve_cmd_token(name);
            if mode == 3 {
                let generation = current.unwrap();
                assert_ne!(generation, old);
                assert!(interpreter.native_execution_refusal().is_none());
                assert_eq!(
                    interpreter
                        .native_compilation
                        .borrow()
                        .stock
                        .get(&generation),
                    Some(&name.to_vec())
                );
                assert!(matches!(
                    interpreter.raw_command_by_generation(generation),
                    Some(Command::Ensemble(_))
                ));
            } else {
                assert!(interpreter.native_execution_refusal().is_some());
                if let Some(generation) = current {
                    assert!(!interpreter
                        .native_compilation
                        .borrow()
                        .stock
                        .contains_key(&generation));
                }
                if mode == 0 {
                    assert!(matches!(
                        interpreter.raw_command_by_generation(current.unwrap()),
                        Some(Command::Builtin(_))
                    ));
                }
            }
            REGISTRATION_NESTED_RETIREMENT_MODE.with(|value| value.set(3));
            REGISTRATION_NESTED_RETIREMENT_CALLS.with(|value| value.set(0));
        });
    }
}

#[test]
fn dynamic_builtin_registration_preserves_native_namespace_fallback_without_stock_identity() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // These are Runtime registration/lookup controls, not external execution proofs.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        leak_free(|interpreter| {
            *interpreter = Interp::with_native_core(
                default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interpreter.register_builtin(b"registrationHostWorker", registration_host_worker);
            let generation = interpreter
                .resolve_cmd_token(b"registrationHostWorker")
                .unwrap();
            assert!(!interpreter
                .registry_builtin_names
                .borrow()
                .contains_key(&generation));
            assert!(!interpreter
                .native_compilation
                .borrow()
                .stock
                .contains_key(&generation));
            assert!(interpreter
                .guarded_commands
                .borrow()
                .get(&generation)
                .is_none());
            assert_eq!(
                interpreter.eval_str(b"namespace eval RegistrationHost {registrationHostWorker}"),
                Code::Ok,
                "{profile}: result={:?}, refusal={:?}",
                interpreter.result_bytes(),
                interpreter.native_execution_refusal(),
            );
            assert_eq!(interpreter.result_bytes(), b"HOST", "{profile}");
            assert!(interpreter.native_execution_refusal().is_none());
        });
    }
}

#[test]
fn explicitly_guarded_host_replacement_has_no_stock_release_floor() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        interpreter.register_guarded_builtin(
            b"lassign",
            registration_host_worker,
            GUARDED_IDENTITY,
        );
        let generation = interpreter.resolve_cmd_token(b"lassign").unwrap();
        let token = interpreter
            .prepare_command_guard(
                b"lassign",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::CommandEnvironment),
            )
            .unwrap();
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert_eq!(interpreter.eval_str(b"lassign"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"HOST");
        assert!(interpreter.check_command_guard_identity(token, b"lassign", GUARDED_IDENTITY));
        assert!(!interpreter
            .registry_builtin_names
            .borrow()
            .contains_key(&generation));
        assert!(!interpreter
            .native_compilation
            .borrow()
            .stock
            .contains_key(&generation));
        assert!(interpreter.release_command_guard(token));
    });
}

#[test]
fn stock_ensemble_attestation_requires_the_original_worker_at_its_actual_map_slot() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for move_original in [false, true] {
        leak_free(|interpreter| {
            let intrinsic = tcl_registry::IntrinsicId::StringLength;
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                intrinsic.stable_id(),
                intrinsic.guard_semantics_key(interpreter.runtime_version()),
            );
            let token = interpreter
                .prepare_command_guard(
                    b"string",
                    identity,
                    GuardDomains::one(GuardDomain::CommandEnvironment),
                )
                .unwrap();
            let worker = interpreter
                .resolve_cmd_token(b"::tcl::string::length")
                .unwrap();
            if move_original {
                assert_eq!(
                    interpreter
                        .eval_str(b"rename ::tcl::string::length ::registrationOriginalLength"),
                    Code::Ok
                );
                assert!(interpreter
                    .raw_command_location_by_generation(worker)
                    .is_some());
            }
            interpreter.register_builtin(b"::tcl::string::length", registration_host_worker);
            assert!(!interpreter.check_command_guard_identity(token, b"string", identity));
            assert!(interpreter.attested_identities(b"string").is_none());
            assert_eq!(interpreter.eval_str(b"string length abc"), Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"HOST");
            assert!(interpreter.release_command_guard(token));
        });
    }
}

#[test]
fn stock_ensemble_attestation_refuses_a_changed_actual_mapping() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        let intrinsic = tcl_registry::IntrinsicId::StringLength;
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key(interpreter.runtime_version()),
        );
        let token = interpreter
            .prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::CommandEnvironment),
            )
            .unwrap();
        interpreter.register_builtin(b"registrationMappedLength", registration_host_worker);
        assert_eq!(
            interpreter.eval_str(
                b"namespace ensemble configure string -map {length ::registrationMappedLength}",
            ),
            Code::Ok
        );
        assert!(!interpreter.check_command_guard_identity(token, b"string", identity));
        assert_eq!(interpreter.eval_str(b"string length abc"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"HOST");
        assert!(interpreter.release_command_guard(token));
    });
}

#[test]
fn namespace_mutation_owner_synchronises_all_lookup_guard_domains() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mutation in 0..10 {
        leak_free(|interpreter| {
            assert_eq!(interpreter.eval_str(
                    b"namespace eval RegistrationSource {proc target {} {}; namespace export target};
                      namespace eval RegistrationImports {namespace import ::RegistrationSource::target}",
                ), Code::Ok);
            let namespace = interpreter
                .namespaces()
                .find_namespace(GLOBAL, b"RegistrationImports")
                .unwrap();
            if mutation == 4 {
                interpreter.namespaces_mut().export(namespace, b"target");
            } else if mutation == 5 {
                interpreter
                    .namespaces_mut()
                    .set_path(namespace, vec![GLOBAL]);
            } else if mutation == 9 {
                interpreter
                    .namespaces_mut()
                    .begin_namespace_teardown(namespace);
            }
            interpreter.register_guarded_builtin(
                b"registrationGuarded",
                registration_host_worker,
                GUARDED_IDENTITY,
            );
            let tokens = [
                GuardDomain::CommandEnvironment,
                GuardDomain::Namespace,
                GuardDomain::UnknownHandling,
            ]
            .map(|domain| {
                interpreter
                    .prepare_command_guard(
                        b"registrationGuarded",
                        GUARDED_IDENTITY,
                        GuardDomains::one(domain),
                    )
                    .unwrap()
            });
            interpreter.register_builtin(b"registrationUnrelated", registration_host_worker);
            assert!(tokens
                .iter()
                .all(|token| interpreter.check_command_guard(*token, b"registrationGuarded")));
            match mutation {
                0 => interpreter.namespaces_mut().set_path(GLOBAL, vec![GLOBAL]),
                1 => interpreter
                    .namespaces_mut()
                    .export(GLOBAL, b"registrationNewExport"),
                2 => {
                    let handler = obj::Owned::fresh(new_string(b"registrationUnknown"));
                    let previous = interpreter
                        .namespaces_mut()
                        .set_unknown_handler(GLOBAL, Some(handler));
                    drop(previous);
                }
                3 => assert!(interpreter.namespaces_mut().remove_in(namespace, b"target")),
                4 => interpreter.namespaces_mut().clear_exports(namespace),
                5 => interpreter
                    .namespaces_mut()
                    .finish_native_namespace_command_path(namespace),
                6 => interpreter
                    .namespaces_mut()
                    .delete_namespace_by_id(namespace),
                7 => {
                    let retired = interpreter
                        .namespaces_mut()
                        .clear_namespace_ids(&[namespace]);
                    drop(retired);
                }
                8 => interpreter
                    .namespaces_mut()
                    .begin_namespace_variable_teardown(namespace),
                _ => interpreter
                    .namespaces_mut()
                    .finish_namespace_teardown(&[namespace]),
            }
            assert!(tokens
                .iter()
                .zip([
                    GuardDomain::CommandEnvironment,
                    GuardDomain::Namespace,
                    GuardDomain::UnknownHandling,
                ])
                .all(|(token, domain)| {
                    interpreter.check_command_guard(*token, b"registrationGuarded")
                        == (domain == GuardDomain::CommandEnvironment && mutation >= 6)
                }));
            for token in tokens {
                assert!(interpreter.release_command_guard(token));
            }
        });
    }
}

#[test]
fn exhausted_namespace_lookup_revision_permanently_poisons_its_guard_domains() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        interpreter.register_guarded_builtin(
            b"registrationGuarded",
            registration_host_worker,
            GUARDED_IDENTITY,
        );
        let domains = [
            GuardDomain::CommandEnvironment,
            GuardDomain::Namespace,
            GuardDomain::UnknownHandling,
        ];
        let tokens = domains.map(|domain| {
            interpreter
                .prepare_command_guard(
                    b"registrationGuarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(domain),
                )
                .unwrap()
        });
        let independent = interpreter
            .prepare_command_guard(
                b"registrationGuarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::Interpreter),
            )
            .unwrap();
        interpreter
            .namespaces_mut()
            .exhaust_guard_lookup_environment_revision_for_test();
        assert!(tokens
            .iter()
            .all(|token| !interpreter.check_command_guard(*token, b"registrationGuarded")));
        assert!(interpreter.check_command_guard(independent, b"registrationGuarded"));
        interpreter.namespaces_mut().set_path(GLOBAL, vec![GLOBAL]);
        for domain in domains {
            assert_eq!(
                interpreter.prepare_command_guard(
                    b"registrationGuarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(domain),
                ),
                Err(GuardError::Poisoned)
            );
        }
        for token in tokens {
            assert!(interpreter.release_command_guard(token));
        }
        assert!(interpreter.release_command_guard(independent));
    });
}

unsafe extern "C" fn registration_recreate_on_retirement(data: *mut c_void) {
    // SAFETY: the actual retiring command transfers this sole client record.
    let state = unsafe { Box::from_raw(data.cast::<PublicationRetirement>()) };
    state.calls.set(state.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().map(Interp).unwrap();
    interpreter.register_builtin(b"retiring", registration_host_worker);
}

#[test]
fn refused_guarded_registration_cannot_attest_a_callback_created_generation() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // This checks the Runtime receipt; it claims no native discard chronology.
    leak_free(|interpreter| {
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let state = retirement_state(interpreter, 0);
        let calls = state.calls.clone();
        interpreter
            .create_obj_command(
                b"retiring",
                ObjCommand::new(
                    publication_noop,
                    Box::into_raw(state).cast(),
                    Some(registration_recreate_on_retirement),
                ),
            )
            .unwrap();
        interpreter.register_guarded_builtin(b"retiring", resultless_builtin, GUARDED_IDENTITY);
        assert_eq!(calls.get(), 1);
        assert!(interpreter.native_execution_refusal().is_some());
        let replacement = interpreter.resolve_cmd_token(b"retiring").unwrap();
        assert!(interpreter
            .guarded_commands
            .borrow()
            .get(&replacement)
            .is_none());
        assert!(interpreter.attested_identities(b"retiring").is_none());
        assert!(!interpreter
            .registry_builtin_names
            .borrow()
            .contains_key(&replacement));
    });
}

#[test]
fn renamed_stock_ensemble_keeps_its_release_identity_and_dynamic_replacement_does_not() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interpreter| {
        assert_eq!(
            interpreter.eval_str(b"rename dict registrationMovedDict"),
            Code::Ok
        );
        let stock = interpreter
            .resolve_cmd_token(b"registrationMovedDict")
            .unwrap();
        assert!(matches!(
            interpreter.raw_command_by_generation(stock),
            Some(Command::Ensemble(_))
        ));
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert!(interpreter
            .resolve_dispatchable(GLOBAL, b"registrationMovedDict")
            .is_none());
        interpreter.register_builtin(b"registrationMovedDict", registration_host_worker);
        let replacement = interpreter
            .resolve_cmd_token(b"registrationMovedDict")
            .unwrap();
        assert_ne!(replacement, stock);
        assert_eq!(interpreter.eval_str(b"registrationMovedDict"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"HOST");
    });
}

#[test]
fn release_visibility_does_not_donate_a_foreign_list_backing_origin() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The explicit origin refusal is separate from command-table visibility.
    use tcl_syntax::value::ValueOps;
    leak_free(|interpreter| {
        let member = obj::Owned::fresh(new_string(b"A"));
        let list = obj::Owned::fresh(interpreter.new_list(vec![member.as_ptr()]));
        assert_eq!(
            interpreter.list_elements(&list.as_ptr()).unwrap(),
            vec![member.as_ptr()]
        );
        let before = obj::native_object_snapshot(list.as_ptr()).unwrap();
        interpreter.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert!(matches!(
            interpreter.list_elements(&list.as_ptr()),
            Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native List string recipe origin"
            ))
        ));
        assert_eq!(obj::native_object_snapshot(list.as_ptr()).unwrap(), before);
    });
}

#[cfg(have_tommath)]
#[test]
fn native_math_function_commands_follow_constructor_surface() {
    for (environment, expected) in [
        ("jim", false),
        ("tcl8.4", false),
        ("tcl8.5", true),
        ("tcl8.6", true),
        ("tcl9.0", true),
        ("tcl9.1", true),
    ] {
        let interpreter = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(environment),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert!(
            interpreter.native_execution_refusal().is_none(),
            "{environment}"
        );
        assert_eq!(
            interpreter
                .find_command_id(GLOBAL, b"::tcl::mathfunc::abs")
                .is_some(),
            expected,
            "{environment}",
        );
    }
    let generic = Interp::new();
    assert!(generic
        .find_command_id(GLOBAL, b"::tcl::mathfunc::abs")
        .is_some());
}

#[test]
fn native_jim_core_keeps_only_constructor_empty_roles() {
    let interp = Interp::with_native_core(
        default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let context = interp.native_jim_object_context().unwrap();
    let empty = context.empty_object();
    assert_eq!(interp.result_obj(), empty.as_ptr());
    // These are the actual empty, result, errorProc and top-frame namespace
    // roles in Jim_CreateInterp, before a distribution extension is loaded.
    assert_eq!(unsafe { (*empty.as_ptr()).ref_count }, 4);
    assert!(interp.find_command_id(GLOBAL, b"binary").is_none());
    assert!(interp.find_command_id(GLOBAL, b"set").is_some());
}

#[test]
fn native_core_root_cells_match_six_original_constructor_inventories() {
    use tcl_registry::special_vars::NativeBootstrapInputs;
    let fixture =
        include_str!("../../../../rust/tcl-registry/tests/data/native_bootstrap/core-roots.tsv");
    for (profile_name, version) in [
        ("tcl8.4", "8.4"),
        ("tcl8.5", "8.5"),
        ("tcl8.6", "8.6"),
        ("tcl9.0", "9.0"),
        ("tcl9.1", "9.1"),
        ("jim", "jim"),
    ] {
        counters::reset();
        {
            let interp = Interp::with_native_core(
                default_host(),
                crate::environment::profile_for_dialect(profile_name),
                NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: Some(b"/native/build/library".to_vec()),
                },
            )
            .unwrap();
            assert_eq!(
                interp.find_command_id(GLOBAL, b"binary").is_some(),
                version != "jim",
                "{profile_name}"
            );
            assert_eq!(
                interp.find_command_id(GLOBAL, b"oo::class").is_some(),
                matches!(version, "8.6" | "9.0" | "9.1"),
                "{profile_name}"
            );
            assert_eq!(
                interp.find_command_id(GLOBAL, b"try").is_some(),
                matches!(version, "8.6" | "9.0" | "9.1" | "jim"),
                "{profile_name}"
            );
            assert_eq!(
                interp.find_command_id(GLOBAL, b"throw").is_some(),
                matches!(version, "8.6" | "9.0" | "9.1"),
                "{profile_name}"
            );
            assert!(
                interp.find_command_id(GLOBAL, b"auto_load").is_none(),
                "{profile_name}"
            );
            assert!(
                interp.find_command_id(GLOBAL, b"auto_import").is_none(),
                "{profile_name}"
            );
            let expected: Vec<_> = fixture
                .lines()
                .filter_map(|line| {
                    let fields: Vec<_> = line.split('\t').collect();
                    (fields[0] == version && fields[2] == "1")
                        .then_some(fields[1].as_bytes().to_vec())
                })
                .collect();
            assert!(!expected.is_empty());
            let namespaces = interp.namespaces.borrow();
            let table = namespaces.var_table(GLOBAL);
            assert_eq!(
                table
                    .names()
                    .into_iter()
                    .map(<[u8]>::to_vec)
                    .collect::<Vec<_>>(),
                expected,
                "{profile_name}"
            );
            for line in fixture
                .lines()
                .filter(|line| line.starts_with(&format!("{version}\t")))
            {
                let fields: Vec<_> = line.split('\t').collect();
                assert!(
                    table.has_native_namespace_cell(fields[1].as_bytes()),
                    "{profile_name}: {}",
                    fields[1]
                );
            }
            for name in [b"argv".as_slice(), b"argc", b"argv0", b"tcl_library"] {
                assert!(
                    !table.has_native_namespace_cell(name),
                    "{profile_name}: {name:?}"
                );
            }
        }
        assert_eq!(
            counters::finalize(),
            0,
            "{profile_name}: {} objects, {} buffers",
            counters::live_objs(),
            counters::live_bufs()
        );
    }
}

fn leak_free(body: impl FnOnce(&mut Interp)) {
    counters::reset();
    {
        let mut interp = Interp::new();
        body(&mut interp);
    }
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0);
}

/// Evaluate `src`, requiring success, and return the result bytes.
fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
    assert_eq!(
        i.eval_str(src),
        Code::Ok,
        "eval {:?} -> {:?}",
        String::from_utf8_lossy(src),
        String::from_utf8_lossy(&i.result_bytes())
    );
    i.result_bytes()
}

#[test]
fn fresh_interp_installs_the_shared_platform_schema_before_init() {
    leak_free(|i| {
        let mut expected = tcl_platform::bootstrap::entries()
            .iter()
            .map(|entry| entry.name())
            .collect::<Vec<_>>();
        expected.sort_unstable();
        assert_eq!(
            ok(i, b"lsort [array names ::tcl_platform]"),
            expected.join(" ").as_bytes()
        );
        assert_eq!(ok(i, b"set ::tcl_platform(osVersion)"), b"");
        assert!(!ok(i, b"set ::tcl_platform(machine)").is_empty());
        assert_eq!(
            ok(
                i,
                b"list [info exists ::env] [info exists ::argv] \
                      [info exists ::argv0] [info exists ::argc] \
                      [info exists ::auto_path] [info exists ::tcl_library]"
            ),
            b"1 1 1 1 1 1"
        );
    });
}

#[test]
fn selected_host_bootstrap_and_rebind_replace_all_host_globals() {
    counters::reset();
    {
        let first = Rc::new(SyntheticHost::new(&[
            ("USER", "first-user"),
            ("TCL_LIBRARY", "/first/lib"),
            ("TCL_WASM_SPEC", "first-wasm"),
            ("FIRST_ONLY", "stale"),
        ]));
        let mut interp = Interp::with_host(first);
        assert_eq!(
            interp.host().capabilities(),
            tcl_platform::Capabilities::empty()
        );
        assert_eq!(
            ok(
                &mut interp,
                b"list $::tcl_platform(user) $::tcl_platform(wasm) \
                      $::tcl_library $::env(FIRST_ONLY)"
            ),
            b"first-user first-wasm /first/lib stale"
        );
        ok(
            &mut interp,
            b"set ::env(EMBEDDER_STALE) old; \
                  set ::tcl_platform(user) old; \
                  set ::auto_path /old/auto; \
                  set ::tclDefaultLibrary /old/default; \
                  set ::tcl_pkgPath /old/pkg",
        );

        interp.set_host(Rc::new(SyntheticHost::new(&[
            ("USER", "second-user"),
            ("TCL_LIBRARY", "/second/lib"),
            ("TCL_WASM_SPEC", "second-wasm"),
            ("SECOND_ONLY", "fresh"),
        ])));
        assert_eq!(
            ok(
                &mut interp,
                b"list $::tcl_platform(user) $::tcl_platform(wasm) \
                      $::tcl_library $::env(SECOND_ONLY) \
                      [info exists ::env(FIRST_ONLY)] \
                      [info exists ::env(EMBEDDER_STALE)] \
                      [info exists ::tclDefaultLibrary] \
                      [info exists ::tcl_pkgPath] [llength $::auto_path]"
            ),
            b"second-user second-wasm /second/lib fresh 0 0 0 0 0"
        );
        assert_eq!(
            ok(
                &mut interp,
                b"interp create child; child eval {list $::tcl_platform(user) \
                      $::tcl_library $::env(SECOND_ONLY)}"
            ),
            b"second-user /second/lib fresh"
        );
    }
    assert_eq!(counters::finalize(), 0);
    assert_eq!(counters::double_free_count(), 0);
}

#[test]
fn child_and_safe_platform_schemas_come_from_the_shared_owner() {
    leak_free(|i| {
        let mut child_keys = tcl_platform::bootstrap::entries()
            .iter()
            .map(|entry| entry.name())
            .collect::<Vec<_>>();
        child_keys.sort_unstable();
        assert_eq!(
            ok(
                i,
                b"interp create child; child eval {lsort [array names ::tcl_platform]}"
            ),
            child_keys.join(" ").as_bytes()
        );

        let scrubbed = tcl_platform::bootstrap::safe_scrub_keys().collect::<Vec<_>>();
        let mut safe_keys = tcl_platform::bootstrap::entries()
            .iter()
            .map(|entry| entry.name())
            .filter(|name| !scrubbed.contains(name))
            .collect::<Vec<_>>();
        safe_keys.sort_unstable();
        assert_eq!(
            ok(
                i,
                b"interp create -safe safe; safe eval {lsort [array names ::tcl_platform]}"
            ),
            safe_keys.join(" ").as_bytes()
        );
        assert!(safe_keys.contains(&"threaded"));
        assert_eq!(
            ok(
                i,
                b"safe eval {list [info exists ::env] \
                      [info exists ::tcl_library] [info exists ::auto_path] \
                      [info exists ::tclDefaultLibrary] [info exists ::tcl_pkgPath]}"
            ),
            b"0 0 0 0 0"
        );
    });
}

/// A safe parent's children are safe with or without `-safe`, at every
/// level, including a child addressed by path; a `marktrusted` parent's are
/// not (tclsh 8.6.18 / 9.0.4 agree).
#[test]
fn children_of_a_safe_interp_inherit_safety() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                b"interp create -safe s; interp create n; list \
                      [s eval {interp issafe [interp create]}] \
                      [s eval {interp issafe [interp create -safe]}] \
                      [s eval {interp create t; t eval {interp issafe [interp create]}}] \
                      [n eval {interp issafe [interp create]}] \
                      [s eval {interp issafe t}] \
                      [interp issafe [interp create {s p}]]"
            ),
            b"1 1 1 0 1 1"
        );
        assert_eq!(
            ok(
                i,
                b"interp marktrusted s; s eval {interp issafe [interp create]}"
            ),
            b"0"
        );
    });
}

/// Each lookup event moves the domain that covers it, whatever else it
/// leaves alone (#2292): namespace membership moves `Namespace`, and the
/// unknown handler or the command it names moves `UnknownHandling`.
#[test]
fn lookup_events_move_the_namespace_and_unknown_handling_domains() {
    let setup = b"namespace eval src {proc p {} {}; namespace export p}";
    let rows: [(GuardDomain, &[u8], &[u8]); 10] = [
        (GuardDomain::Namespace, b"", b"namespace eval created {}"),
        (GuardDomain::Namespace, b"", b"namespace delete src"),
        (GuardDomain::Namespace, b"", b"namespace export guarded"),
        (GuardDomain::Namespace, b"", b"namespace path ::src"),
        (GuardDomain::Namespace, b"", b"namespace import src::p"),
        (
            GuardDomain::Namespace,
            b"namespace import src::p",
            b"namespace forget src::p",
        ),
        (
            GuardDomain::UnknownHandling,
            b"",
            b"namespace unknown handler",
        ),
        (
            GuardDomain::UnknownHandling,
            b"",
            b"proc unknown args {return 0}",
        ),
        (
            GuardDomain::UnknownHandling,
            b"proc unknown args {}",
            b"rename unknown {}",
        ),
        (
            GuardDomain::UnknownHandling,
            b"proc unknown args {}",
            b"rename unknown moved",
        ),
    ];
    for (domain, prepare, event) in rows {
        leak_free(|i| {
            i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            ok(i, setup);
            ok(i, prepare);
            let token = i
                .prepare_command_guard(b"guarded", GUARDED_IDENTITY, GuardDomains::one(domain))
                .unwrap();
            assert!(i.check_command_guard(token, b"guarded"));
            ok(i, event);
            assert!(
                !i.check_command_guard(token, b"guarded"),
                "{domain:?} kept across {}",
                String::from_utf8_lossy(event)
            );
        });
    }
}

#[test]
fn command_guard_checks_identity_and_exact_lifecycle() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        assert_eq!(
            i.prepare_command_guard(b"guarded", GuardIdentity::new(1, 72), domains),
            Err(GuardError::IdentityMismatch)
        );
        let token = i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .unwrap();
        assert!(i.check_command_guard(token, b"guarded"));
        assert!(i.release_command_guard(token));
        assert!(!i.release_command_guard(token));
        assert!(!i.check_command_guard(token, b"guarded"));
    });
}

/// A request for a Family-B intrinsic covers the variable-trace domain or
/// is refused, whatever the caller asked for, and a Value member needs no
/// such domain. Covered, the guard is refused while a variable trace
/// exists and goes stale when one is added.
#[test]
fn a_family_b_guard_request_must_cover_the_variable_trace_domain() {
    use tcl_registry::IntrinsicId;
    leak_free(|i| {
        let version = i.runtime_version();
        let identity = |member: IntrinsicId| {
            GuardIdentity::registry_intrinsic_with_semantics(
                member.stable_id(),
                member.guard_semantics_key(version),
            )
        };
        let (stores, value) = (
            identity(IntrinsicId::DictSet),
            identity(IntrinsicId::ListLength),
        );
        i.register_guarded_builtin(b"stores", guarded_builtin, stores);
        i.register_guarded_builtin(b"pure", guarded_builtin, value);
        let command = GuardDomains::one(GuardDomain::CommandEnvironment);
        let traced = command.with(GuardDomain::VariableTrace);

        assert_eq!(
            i.prepare_command_guard(b"stores", stores, command),
            Err(GuardError::DomainsInsufficient)
        );
        let token = i
            .prepare_command_guard(b"pure", value, command)
            .expect("a Value member requires no variable-trace domain");
        assert!(i.release_command_guard(token));

        let token = i
            .prepare_command_guard(b"stores", stores, traced)
            .expect("a request covering the family's domain");
        assert!(i.check_command_guard_identity(token, b"stores", stores));
        assert_eq!(
            i.eval_str(b"trace add variable watched write callback"),
            Code::Ok
        );
        assert!(!i.check_command_guard_identity(token, b"stores", stores));
        assert_eq!(
            i.prepare_command_guard(b"stores", stores, traced),
            Err(GuardError::PrerequisiteUnsatisfied)
        );
        assert!(i.prepare_command_guard(b"pure", value, command).is_ok());
    });
}

/// A guard is bound to its command's token. A definition, rename or alias
/// of another command leaves it and its attestation alone; replacing,
/// renaming away or hiding the command drops it, and restoring the same
/// token brings it back; a profile pin keeps the attestation.
#[test]
fn an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        let token = i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .unwrap();

        i.register_builtin(b"unrelated", guarded_builtin);
        ok(i, b"proc foo {} {return 1}");
        ok(i, b"rename foo bar");
        ok(i, b"interp alias {} baz {} bar");
        ok(i, b"rename baz {}");
        assert!(i.check_command_guard(token, b"guarded"));
        assert!(i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .is_ok());

        // A change to the lookup environment itself stales the token over
        // it, and the attestation stays.
        ok(i, b"namespace eval other {namespace path ::}");
        assert!(!i.check_command_guard(token, b"guarded"));
        let token = i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .expect("still attested");

        // The profile pin keeps the attestation and a token that does not
        // depend on interpreter policy.
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert!(
            i.check_command_guard(token, b"guarded"),
            "guard phase/generation/broad(before,after)/policy(before,after)/kernel: {:?}",
            super::command_guard_lookup::diagnostic(),
        );
        assert!(i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .is_ok());

        // Renaming the command away drops the guard at its name, and the
        // attestation goes with the command; restoring the name restores it.
        ok(i, b"rename guarded moved");
        assert!(!i.check_command_guard(token, b"guarded"));
        assert_eq!(
            i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
            Err(GuardError::IdentityUnavailable)
        );
        assert!(i
            .prepare_command_guard(b"moved", GUARDED_IDENTITY, domains)
            .is_ok());
        ok(i, b"rename moved guarded");
        assert!(i.check_command_guard(token, b"guarded"));

        // Hiding it drops the guard, exposing it restores it.
        assert_eq!(
            i.hide_command(b"guarded", b"guarded"),
            CommandVisibilityOutcome::Moved
        );
        assert!(!i.check_command_guard(token, b"guarded"));
        assert_eq!(
            i.expose_command(b"guarded", b"guarded"),
            CommandVisibilityOutcome::Moved
        );
        assert!(i.check_command_guard(token, b"guarded"));

        // A different command at the name is never attested.
        ok(i, b"proc guarded {} {return 1}");
        assert!(!i.check_command_guard(token, b"guarded"));
        assert_eq!(
            i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
            Err(GuardError::IdentityUnavailable)
        );
    });
}

#[test]
fn command_currency_retains_original_lookup_but_refuses_path_and_scope_changes() {
    leak_free(|interpreter| {
        let caller = interpreter
            .namespaces_mut()
            .ensure_namespace(GLOBAL, b"caller");
        interpreter.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        interpreter.current_ns.set(caller);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        let token = interpreter
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .unwrap();
        interpreter.register_builtin(b"::unrelated::worker", guarded_builtin);
        assert!(interpreter.check_command_guard(token, b"guarded"));
        interpreter
            .namespaces_mut()
            .delete_namespace(GLOBAL, b"unrelated");
        assert!(interpreter.check_command_guard(token, b"guarded"));
        interpreter.current_ns.set(GLOBAL);
        assert!(!interpreter.check_command_guard(token, b"guarded"));
        interpreter.current_ns.set(caller);
        assert!(interpreter.check_command_guard(token, b"guarded"));
        interpreter.namespaces_mut().set_path(caller, vec![GLOBAL]);
        assert!(!interpreter.check_command_guard(token, b"guarded"));
        interpreter.namespaces_mut().set_path(caller, Vec::new());
        assert!(!interpreter.check_command_guard(token, b"guarded"));
        assert!(interpreter.release_command_guard(token));
        interpreter.current_ns.set(GLOBAL);
    });
}

/// The attestation belongs to the token the name reaches, so a definition
/// in a namespace that shadows the command drops the guard for calls made
/// from that namespace and only from it.
#[test]
fn a_shadowing_definition_drops_the_guard_only_for_calls_from_its_namespace() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        let token = i
            .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
            .unwrap();

        ok(i, b"namespace eval ns {proc guarded {} {return shadow}}");
        let ns = i
            .namespaces
            .borrow()
            .find_namespace(GLOBAL, b"ns")
            .expect("the namespace exists");
        assert!(i.check_command_guard(token, b"guarded"));
        i.current_ns.set(ns);
        assert!(!i.check_command_guard(token, b"guarded"));
        assert_eq!(
            i.prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains),
            Err(GuardError::IdentityUnavailable)
        );
        i.current_ns.set(GLOBAL);
        assert!(i.check_command_guard(token, b"guarded"));
    });
}

/// The three lookup domains move together on a hidden invocation into a
/// namespace, whether the namespace is named, is the global one, or does not
/// exist yet, and on `oo::copy`: events the guard domain's docs list beside
/// `namespace path` and the rest, which a command-table mutation is not.
#[test]
fn the_lookup_domains_move_on_a_hidden_invocation_into_a_namespace_and_on_oo_copy() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::UnknownHandling);
        let moves = |i: &mut Interp, script: &[u8]| {
            let token = i
                .prepare_command_guard(b"guarded", GUARDED_IDENTITY, domains)
                .expect("the lookup domains are guardable");
            ok(i, script);
            assert!(i.attested_identities(b"guarded").is_some());
            !i.check_command_guard(token, b"guarded")
        };

        ok(i, b"interp hide {} lindex");
        assert!(
            moves(i, b"interp invokehidden {} -namespace fresh lindex {a b} 0"),
            "a namespace it creates"
        );
        assert!(
            moves(i, b"interp invokehidden {} -namespace fresh lindex {a b} 0"),
            "a namespace that exists"
        );
        assert!(
            moves(i, b"interp invokehidden {} -global lindex {a b} 0"),
            "the global namespace"
        );
        assert!(
            !moves(i, b"interp invokehidden {} lindex {a b} 0"),
            "a hidden invocation that names no namespace moves nothing"
        );

        ok(
            i,
            b"oo::class create C {variable v; constructor {} {set v 1}}",
        );
        ok(i, b"set o [C new]");
        assert!(moves(i, b"oo::copy $o"), "oo::copy");
    });
}

/// The sweep reads the generation the interpreter is pinned to and nothing
/// an overlay installed: a pack's command is no runtime implementation, and
/// only the runtime may attest.
#[test]
fn identities_come_from_the_pinned_generation_never_an_overlay() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_registry::{CommandSpec, IntrinsicId, SemanticOperationId};
    leak_free(|i| {
        // A generation an overlay installed under this interpreter's own
        // profile, holding a spec that declares an intrinsic, and a builtin
        // of that name in the interpreter.
        let overlaid = tcl_registry::registry_for_profile_with_overlay(
            i.dialect_profile(),
            0xC0DE,
            |registry| {
                registry.insert(CommandSpec {
                    name: "overlay_length",
                    semantic_operation: Some(SemanticOperationId::Intrinsic(
                        IntrinsicId::StringLength,
                    )),
                    ..CommandSpec::DEFAULT
                });
            },
        );
        i.register_builtin(b"overlay_length", guarded_builtin);

        i.attach_identities();
        assert!(
            i.attested_identities(b"overlay_length").is_none(),
            "the sweep of the pinned generation attests nothing an overlay declares"
        );
        assert!(
            i.attested_identities(b"string").is_some(),
            "and still attests the shipped command"
        );

        // A store cannot attest an ordinary host handler by its spelling.
        i.attach_identities_from(&overlaid);
        assert!(i.attested_identities(b"overlay_length").is_none());
        // Explicit spec registration supplies the independent semantic claim.
        i.register_spec_builtin(
            overlaid.get_exact("overlay_length").unwrap(),
            guarded_builtin,
        );
        assert!(i.attested_identities(b"overlay_length").is_some());
    });
}

/// A name is attested only while the command bound at it is the builtin
/// the runtime registered there: a procedure defined over it, or another
/// builtin renamed into it, gains nothing, and the original, put back,
/// has what it had.
#[test]
fn the_sweep_attests_only_the_builtin_it_registered() {
    leak_free(|i| {
        use tcl_registry::IntrinsicId;
        let length = GuardIdentity::registry_intrinsic_with_semantics(
            IntrinsicId::StringLength.stable_id(),
            IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let attested = |i: &Interp| {
            i.attested_identities(b"string")
                .is_some_and(|identities| identities.contains(&length))
        };
        assert!(attested(i));

        ok(i, b"rename string original");
        assert!(!attested(i), "moved away, the name reaches nothing");
        ok(i, b"proc string args {return x}");
        i.attach_identities();
        assert!(!attested(i), "a procedure at the name gains no attestation");

        ok(i, b"rename string {}");
        ok(i, b"rename puts string");
        i.attach_identities();
        assert!(
            !attested(i),
            "another builtin renamed into the name gains none"
        );

        ok(i, b"rename string puts");
        ok(i, b"rename original string");
        assert!(attested(i), "the original, restored, is attested as it was");
    });
}

/// Every builtin this runtime registers at the name of a spec that
/// declares an intrinsic or definition contract is attested for every
/// corresponding identity, and no other command is attested.
#[test]
fn engine_owned_ensemble_attestation_requires_original_workers_and_configuration() {
    for change_configuration in [false, true] {
        leak_free(|interpreter| {
            let generation = interpreter
                .shipped_builtin_generation(b"info")
                .expect("actual engine-installed Info ensemble");
            let Some(Command::Ensemble(ensemble)) =
                interpreter.raw_command_by_generation(generation)
            else {
                panic!("original Info ensemble token");
            };
            assert!(interpreter.stock_ensemble_implementation_is_current(generation));
            assert!(interpreter.attested_identities(b"info").is_some());
            let mut configuration = ensemble.config();
            if change_configuration {
                configuration.prefixes = !configuration.prefixes;
                ensemble.configure(configuration);
            } else {
                let worker = configuration
                    .map
                    .as_ref()
                    .expect("actual Info worker map")
                    .iter()
                    .find(|(member, _)| member == b"exists")
                    .expect("actual Info Exists worker")
                    .1[0]
                    .clone();
                interpreter.register_builtin(&worker, guarded_builtin);
            }
            assert!(interpreter
                .stock_ensemble_registry_identity(generation)
                .is_some());
            assert!(!interpreter.stock_ensemble_implementation_is_current(generation));
            assert!(interpreter.shipped_builtin_generation(b"info").is_none());
            interpreter.attach_identities();
            assert!(interpreter.attested_identities(b"info").is_none());
        });
    }
}

#[test]
fn a_public_ensemble_copy_cannot_acquire_engine_intrinsic_attestation() {
    leak_free(|interpreter| {
        let original = interpreter
            .shipped_builtin_generation(b"info")
            .expect("actual engine-installed Info ensemble");
        let Some(Command::Ensemble(ensemble)) = interpreter.raw_command_by_generation(original)
        else {
            panic!("original Info ensemble token");
        };
        let replacement = interpreter
            .create_ensemble(b"info", ensemble.config())
            .expect("actual public replacement ensemble");
        assert_ne!(replacement, original);
        assert!(interpreter
            .stock_ensemble_registry_identity(replacement)
            .is_none());
        assert!(interpreter.shipped_builtin_generation(b"info").is_none());
        interpreter.attach_identities();
        assert!(interpreter.attested_identities(b"info").is_none());
    });
}

#[test]
fn every_registered_builtin_with_an_intrinsic_is_attested_for_all_of_them() {
    leak_free(|i| {
        let registry = crate::environment::store_for_profile(i.dialect_profile());
        let report = tcl_runtime_api::BackingReport::from_entries(i.backing_report());
        let mut attested_names = 0;
        for name in registry.command_names() {
            let spec = registry.get_exact(name).expect("a named spec");
            let expected: std::collections::BTreeSet<GuardIdentity> = spec
                .intrinsic_ids()
                .into_iter()
                .flat_map(|id| {
                    id.guard_semantics_variants().iter().map(move |semantics| {
                        GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                    })
                })
                .chain(
                    spec.native_lowering_at(
                        i.native_invocation_dialect().authoring_query().as_ref(),
                    )
                    .definition_guard_identity(i.native_invocation_dialect()),
                )
                .collect();
            let actual = i.attested_identities(name.as_bytes());
            if expected.is_empty() || report.of(name) != RegisteredBacking::Builtin {
                assert!(actual.is_none(), "{name} has nothing to attest");
            } else {
                assert_eq!(actual.as_ref(), Some(&expected), "{name}");
                attested_names += 1;
            }
        }
        assert!(attested_names >= 10, "{attested_names}");
    });
}

#[test]
fn unicode_backing_catalogue_omits_opaque_names_without_changing_command_identity() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|interp| {
        interp.register_builtin(b"opaque_\xff", guarded_builtin);
        interp.register_builtin("café".as_bytes(), guarded_builtin);
        assert!(interp
            .command_names()
            .iter()
            .any(|name| name == b"opaque_\xff"));
        let head = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"opaque_\xff"));
        assert_eq!(interp.dispatch_invoke(&[head.as_ptr()]), Code::Ok);
        assert!(interp.native_execution_refusal().is_none());
        let report = interp.backing_report();
        assert!(report
            .iter()
            .any(|(name, backing)| name == "café" && *backing == RegisteredBacking::Builtin));
        assert!(!report.iter().any(|(name, _)| name == "opaque_�"));
        assert!(!report.iter().any(|(name, _)| name.starts_with("opaque_")));
    });
}

#[test]
fn backing_report_preserves_colon_components_from_actual_placed_slots() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software table placement and diagnostic projection only; no Native dispatch or inverse-name claim.
    leak_free(|interp| {
        let holder = {
            let mut namespaces = interp.namespaces.borrow_mut();
            for name in [b"run".as_slice(), b":run", b"::run"] {
                namespaces.bind(GLOBAL, name, Command::Builtin(guarded_builtin));
            }
            let holder = namespaces.ensure_namespace(GLOBAL, b":");
            namespaces.bind(holder, b"run", Command::Builtin(guarded_builtin));
            namespaces.bind(holder, b":run", Command::Builtin(guarded_builtin));
            assert_eq!(namespaces.command_fqn_at(GLOBAL, b":run"), b":::run");
            assert_eq!(namespaces.command_fqn_at(GLOBAL, b"::run"), b"::::run");
            assert_eq!(namespaces.command_fqn_at(holder, b"run"), b":::::run");
            holder
        };
        let original = interp
            .namespaces
            .borrow()
            .command_generation(holder, b"run");
        assert!(original.is_some());
        let entries = interp.backing_report();
        for expected in ["run", ":run", "::run", ":::run", "::::run"] {
            assert!(
                    entries
                        .iter()
                        .any(|(key, backing)| key == expected
                            && *backing == RegisteredBacking::Builtin),
                    "{expected}"
                );
        }
        let report = tcl_runtime_api::BackingReport::from_unrooted_entries(entries);
        for expected in ["run", ":run", "::run", ":::run", "::::run"] {
            assert_eq!(report.of_unrooted(expected), RegisteredBacking::Builtin);
        }
        assert_eq!(report.of_unrooted(":::::run"), RegisteredBacking::Absent);
        assert_eq!(
            interp
                .namespaces
                .borrow()
                .command_generation(holder, b"run"),
            original
        );
    });
}

/// The backing report says what the handler table holds: handlers, the
/// engine's `TclOO` roots and the commands the object system binds in every
/// object, handlers that only refuse, and absences; and nothing a script
/// defines.
#[test]
fn the_backing_report_says_what_the_handler_table_holds() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|i| {
        let report = |i: &Interp| tcl_runtime_api::BackingReport::from_entries(i.backing_report());
        let before = report(i);
        assert_eq!(before.of("set"), RegisteredBacking::Builtin);
        assert_eq!(
            before.of("::tcl::string::insert"),
            RegisteredBacking::Builtin
        );
        assert_eq!(before.of("exec"), RegisteredBacking::Unsupported);
        assert_eq!(before.of("oo::class"), RegisteredBacking::Object);
        assert_eq!(before.of("oo::object"), RegisteredBacking::Object);
        assert_eq!(before.of("my"), RegisteredBacking::Object);
        assert_eq!(before.of("zipfs"), RegisteredBacking::Absent);
        // The selected native ensemble owns this real private worker.
        assert_eq!(before.of("tcl::dict::get"), RegisteredBacking::Builtin);
        #[cfg(have_tommath)]
        assert_eq!(before.of("expr"), RegisteredBacking::Builtin);
        #[cfg(not(have_tommath))]
        assert_eq!(before.of("expr"), RegisteredBacking::NeedsNumericTower);

        ok(i, b"proc mine {} {}");
        ok(i, b"oo::class create Mine");
        ok(i, b"interp alias {} aliased {} set");
        let after = report(i);
        for name in ["mine", "Mine", "aliased"] {
            assert_eq!(after.of(name), RegisteredBacking::Absent, "{name}");
        }
        assert_eq!(after, before, "a script adds nothing to the report");
    });
}

/// The commands a build without the numeric tower reports as needing it
/// are the ones a build with it registers as handlers.
#[cfg(have_tommath)]
#[test]
fn the_tower_commands_a_build_without_it_names_are_registered_with_it() {
    leak_free(|i| {
        let report = tcl_runtime_api::BackingReport::from_entries(i.backing_report());
        let names = crate::builtins::tower_command_names();
        assert!(names.len() > 60, "{}", names.len());
        for name in names {
            assert_eq!(report.of(&name), RegisteredBacking::Builtin, "{name}");
        }
    });
}

/// A build without the tower reports each of those commands as needing it,
/// and registers none of them.
#[cfg(not(have_tommath))]
#[test]
fn a_build_without_the_tower_reports_the_commands_it_lacks_as_needing_it() {
    leak_free(|i| {
        let report = tcl_runtime_api::BackingReport::from_entries(i.backing_report());
        for name in crate::builtins::tower_command_names() {
            assert_eq!(
                report.of(&name),
                RegisteredBacking::NeedsNumericTower,
                "{name}"
            );
        }
        assert_eq!(ok(i, b"info commands expr"), b"");
    });
}

#[test]
fn trace_registration_invalidates_matching_guard_domains() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        let command_token = i
            .prepare_command_guard(
                b"guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::CommandTrace),
            )
            .unwrap();
        assert_eq!(
            i.eval_str(b"trace add command guarded rename callback"),
            Code::Ok
        );
        assert!(!i.check_command_guard(command_token, b"guarded"));

        let variable_token = i
            .prepare_command_guard(
                b"guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::VariableTrace),
            )
            .unwrap();
        assert_eq!(
            i.eval_str(b"trace add variable watched write callback"),
            Code::Ok
        );
        assert!(!i.check_command_guard(variable_token, b"guarded"));
    });
}

#[test]
fn object_dispatch_remains_poisoned_but_interpreter_is_guardable() {
    leak_free(|i| {
        i.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
        assert_eq!(
            i.prepare_command_guard(
                b"guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::ObjectDispatch)
            ),
            Err(GuardError::Poisoned)
        );
        let token = prepare_interpreter_guard(i);
        assert!(i.check_command_guard(token, b"guarded"));
    });
}

#[test]
fn command_identity_survives_unrelated_mutation_and_follows_its_actual_generation() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|i| {
        let token = prepare_interpreter_guard(i);
        i.register_builtin(b"unrelated", resultless_builtin);
        assert!(i.check_command_guard(token, b"guarded"));
        let fresh = i
            .prepare_command_guard(
                b"guarded",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::CommandEnvironment),
            )
            .unwrap();
        assert_eq!(
            i.rename_command(b"guarded", b"moved"),
            RenameOutcome::Renamed
        );
        // A rename preserves the actual command token and both guards.
        assert!(i.check_command_guard(fresh, b"moved"));
        assert!(i.check_command_guard(token, b"moved"));
        let moved = i
            .prepare_command_guard(
                b"moved",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::Interpreter),
            )
            .unwrap();
        i.register_guarded_builtin(b"moved", guarded_builtin, GUARDED_IDENTITY);
        assert!(!i.check_command_guard(token, b"moved"));
        assert!(!i.check_command_guard(moved, b"moved"));
        let replacement = i
            .prepare_command_guard(
                b"moved",
                GUARDED_IDENTITY,
                GuardDomains::one(GuardDomain::Interpreter),
            )
            .unwrap();
        assert!(i.check_command_guard(replacement, b"moved"));
        assert!(i.release_command_guard(token));
        assert!(i.release_command_guard(fresh));
        assert!(i.release_command_guard(moved));
        assert!(i.release_command_guard(replacement));
    });
}

#[test]
fn string_length_guard_accepts_irreducible_base_domains() {
    leak_free(|i| {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::CommandTrace)
            .with(GuardDomain::Interpreter);
        let token = i
            .prepare_command_guard(b"string", identity, domains)
            .expect("the registry BASE domains should be live");
        assert!(i.check_command_guard_identity(token, b"string", identity));
        assert!(i.release_command_guard(token));
    });
}

#[test]
fn visibility_safety_and_topology_mutations_stale_interpreter_guards() {
    leak_free(|i| {
        let token = prepare_interpreter_guard(i);
        assert_eq!(
            i.hide_command(b"set", b"set"),
            CommandVisibilityOutcome::Moved
        );
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        assert_eq!(
            i.expose_command(b"set", b"set"),
            CommandVisibilityOutcome::Moved
        );
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        assert_eq!(i.create_child(Some(b"child".to_vec())), b"child");
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        assert!(i.delete_child(b"child"));
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        i.make_safe();
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        i.mark_trusted();
        assert_interpreter_guard_stale(i, token);
    });
}

/// A child is another interpreter of the same build, so it states the
/// world its parent is pinned to, the overlay included.
#[test]
fn a_child_states_the_context_its_parent_is_pinned_to() {
    const OVERLAY: u64 = 0x0C0_1705;
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
    tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
    leak_free(|parent| {
        let mut context = tcl_registry::model::runtime_context_for_profile(profile);
        context.overlay_generation = OVERLAY;
        context.packages = vec![("vendor".to_owned(), "2.1".to_owned())];
        parent.pin_context(&context).expect("installed, so it pins");
        parent.create_child(Some(b"child".to_vec()));
        let child = parent
            .children
            .borrow()
            .get(b"child".as_slice())
            .expect("child")
            .clone();
        assert_eq!(child.runtime_context(), context);
        assert_eq!(child.held_identity(), parent.held_identity());
    });
}

#[test]
fn child_parent_association_stales_interpreter_guard() {
    leak_free(|parent| {
        parent.create_child(Some(b"child".to_vec()));
        let mut child = parent
            .children
            .borrow()
            .get(b"child".as_slice())
            .expect("child")
            .clone();
        let token = prepare_interpreter_guard(&mut child);
        assert!(child.check_command_guard(token, b"guarded"));

        parent.with_child(b"child", |_| ());
        assert_interpreter_guard_stale(&mut child, token);
    });
}

#[test]
fn execution_policy_and_runtime_version_mutations_stale_interpreter_guards() {
    leak_free(|i| {
        let token = prepare_interpreter_guard(i);
        i.set_host(i.host());
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        i.set_bgerror_handler(b"handler");
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        let frame = new_string(b"-frame");
        let enabled = new_string(b"1");
        let result = i.debug_apply(&[frame, enabled]).expect("debug policy");
        drop_fresh(frame);
        drop_fresh(enabled);
        drop_fresh(result);
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        let limit = new_string(b"250");
        assert_eq!(i.recursion_limit_apply(Some(limit)), Ok(250));
        drop_fresh(limit);
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        let option = new_string(b"-value");
        let value = new_string(b"100");
        let result = i
            .limit_apply(b"commands", &[option, value])
            .expect("command limit policy");
        drop_fresh(option);
        drop_fresh(value);
        drop_fresh(result);
        assert_interpreter_guard_stale(i, token);

        let token = prepare_interpreter_guard(i);
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert_interpreter_guard_stale(i, token);
    });
}

#[test]
fn expression_policy_keeps_native_dispatch_separate_from_authored_parser() {
    use tcl_registry::invocation_words::LogicalExpressionParseProvider;
    use tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin;
    leak_free(|interp| {
        interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        let native = interp.expression_evaluation_policy().unwrap();
        assert!(matches!(
            native.origin,
            ExpressionEvaluationOrigin::Native(_)
        ));
        assert_eq!(
            tcl_registry::native_expression_program::expression_function_dispatch(
                Some(&native),
                interp.native_invocation_dialect()
            ),
            Some(tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable)
        );
        interp.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(interp.expression_evaluation_policy().is_none());
        let host = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        assert!(interp.set_logical_expression_parse_provider(
            LogicalExpressionParseProvider::Tcl84CoreSimulation,
            host
        ));
        let authored = interp.expression_evaluation_policy().unwrap();
        assert_eq!(
            authored.origin,
            ExpressionEvaluationOrigin::AuthoredTcl84Parser
        );
        assert!(authored.numeric_simulation.is_none());
        assert_eq!(
            tcl_registry::native_expression_program::expression_function_dispatch(
                Some(&authored),
                tcl_registry::InvocationDialect::of_profile(host)
            ),
            None
        );
    });
}

// These helpers exercise epoch policy against an original live registration.
// They do not restore a named lookup, native instruction or hosted engine.
fn retained_fixture_identity(interp: &Interp, generation: u64) -> Option<GuardIdentity> {
    interp.raw_command_location_by_generation(generation)?;
    interp
        .guarded_commands
        .borrow()
        .get(&generation)?
        .contains(&GUARDED_IDENTITY)
        .then_some(GUARDED_IDENTITY)
}

fn prepare_retained_fixture_guard(
    interp: &Interp,
    generation: u64,
    domain: GuardDomain,
) -> GuardToken {
    let observed = retained_fixture_identity(interp, generation);
    interp
        .guards
        .borrow_mut()
        .prepare(
            GUARDED_IDENTITY,
            observed,
            GuardDomains::one(domain),
            generation,
        )
        .unwrap()
}

fn check_retained_fixture_guard(interp: &Interp, token: GuardToken, generation: u64) -> bool {
    interp.guards.borrow().check(
        token,
        retained_fixture_identity(interp, generation),
        &generation,
    )
}

#[test]
fn logical_provider_changes_stale_only_interpreter_policy_guards() {
    fn install(
        interp: &mut Interp,
        host: &'static tcl_dialect::DialectProfile,
        provider: u8,
    ) -> bool {
        match provider {
            0 => interp.set_logical_eval_object_provider(
                tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation,
                host,
            ),
            1 => interp.set_logical_source_word_provider(
                tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation,
                host,
            ),
            2 => interp.set_logical_expression_parse_provider(
                tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
                host,
            ),
            _ => unreachable!(),
        }
    }
    for provider in [0, 1, 2] {
        leak_free(|interp| {
            let source = tcl_dialect::DialectProfile::irules();
            let host = crate::environment::profile_for_dialect("tcl9.0");
            let replacement = crate::environment::profile_for_dialect("tcl9.1");
            // Retain an actual registration before selecting a hosted
            // source profile that has no command naming capability.
            interp.set_dialect_profile(host);
            interp.register_guarded_builtin(b"guarded", guarded_builtin, GUARDED_IDENTITY);
            let generation = interp
                .namespaces
                .borrow()
                .resolve_generation(GLOBAL, b"guarded")
                .unwrap();
            interp.set_dialect_profile(source);
            assert!(matches!(
                interp.prepare_command_guard(
                    b"guarded",
                    GUARDED_IDENTITY,
                    GuardDomains::one(GuardDomain::Interpreter)
                ),
                Err(GuardError::IdentityUnavailable)
            ));
            let command =
                prepare_retained_fixture_guard(interp, generation, GuardDomain::CommandEnvironment);
            let policy =
                prepare_retained_fixture_guard(interp, generation, GuardDomain::Interpreter);
            assert!(interp.native_compiler_cache_epochs(GLOBAL).is_none());
            assert!(!install(interp, source, provider));
            assert!(check_retained_fixture_guard(interp, policy, generation));
            assert!(install(interp, host, provider));
            assert!(!check_retained_fixture_guard(interp, policy, generation));
            assert!(interp.release_command_guard(policy));

            let policy =
                prepare_retained_fixture_guard(interp, generation, GuardDomain::Interpreter);
            assert!(install(interp, host, provider));
            assert!(check_retained_fixture_guard(interp, policy, generation));
            assert!(!install(interp, source, provider));
            assert!(check_retained_fixture_guard(interp, policy, generation));
            assert!(install(interp, replacement, provider));
            assert!(!check_retained_fixture_guard(interp, policy, generation));
            assert!(check_retained_fixture_guard(interp, command, generation));
            assert!(interp.native_compiler_cache_epochs(GLOBAL).is_none());
            assert!(interp
                .native_invocation_dialect()
                .execution_point()
                .is_none());
            assert!(interp.release_command_guard(policy));
            assert!(interp.release_command_guard(command));
        });
    }
}

#[test]
fn string_length_intrinsic_uses_the_selected_runtime_character_model() {
    leak_free(|i| {
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
        for intrinsic in [
            tcl_registry::IntrinsicId::StringLength,
            tcl_registry::IntrinsicId::StringIndex,
        ] {
            let identity = GuardIdentity::registry_intrinsic_with_semantics(
                intrinsic.stable_id(),
                intrinsic.guard_semantics_key(i.runtime_version()),
            );
            let token = i
                .prepare_command_guard(b"string", identity, domains)
                .unwrap();
            assert!(i.check_command_guard(token, b"string"));
        }

        let value = new_string("é🙂".as_bytes());
        assert_eq!(
            i.execute_intrinsic(tcl_registry::IntrinsicId::StringLength, &[value]),
            Some(Code::Ok)
        );
        assert_eq!(i.result_bytes(), b"2");

        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let live_domains = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::CommandTrace)
            .with(GuardDomain::Interpreter);
        let token = i
            .prepare_command_guard(b"string", identity, live_domains)
            .expect("registry BASE domains are guardable");
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert!(!i.check_command_guard_identity(token, b"string", identity));
        assert!(i.release_command_guard(token));
        assert_eq!(
            i.execute_intrinsic(tcl_registry::IntrinsicId::StringLength, &[value]),
            Some(Code::Error)
        );
        assert_eq!(
            i.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native string count cache origin",
                ),
            )
        );
        i.reset_native_compilation_admission();
        let c86_value = new_string("é🙂".as_bytes());
        assert_eq!(
            i.execute_intrinsic(tcl_registry::IntrinsicId::StringLength, &[c86_value]),
            Some(Code::Ok)
        );
        assert_eq!(i.result_bytes(), b"3");
        assert_eq!(i.eval_str("string length é🙂".as_bytes()), Code::Ok);
        assert_eq!(i.result_bytes(), b"3");

        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        assert_eq!(i.eval_str("string length é🙂".as_bytes()), Code::Ok);
        assert_eq!(i.result_bytes(), b"2");
        drop_fresh(value);
        drop_fresh(c86_value);
        assert_eq!(
            i.execute_intrinsic(tcl_registry::IntrinsicId::ListLength, &[]),
            None
        );
    });
}

#[test]
fn renaming_spec_registered_string_stales_its_intrinsic_guard() {
    leak_free(|i| {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let token = i
            .prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::CommandEnvironment),
            )
            .unwrap();
        assert_eq!(i.eval_str(b"rename string moved"), Code::Ok);
        assert!(!i.check_command_guard(token, b"string"));
        assert_eq!(
            i.prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::CommandEnvironment)
            ),
            Err(GuardError::IdentityUnavailable)
        );
    });
}

/// The real runtime's `string length` guard, over the registry's base
/// domains, survives the definition, rename and alias of unrelated commands
/// and falls back once `string` itself is rebound.
#[test]
fn an_unrelated_mutation_keeps_the_string_length_guard() {
    leak_free(|i| {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::CommandTrace)
            .with(GuardDomain::Interpreter);
        let token = i
            .prepare_command_guard(b"string", identity, domains)
            .expect("the registry BASE domains are guardable");

        ok(i, b"proc foo {x} {return $x}");
        ok(i, b"rename foo bar");
        ok(i, b"interp alias {} baz {} bar");
        assert!(i.check_command_guard_identity(token, b"string", identity));

        ok(i, b"rename string moved");
        assert!(!i.check_command_guard_identity(token, b"string", identity));
        ok(i, b"rename moved string");
        assert!(i.check_command_guard_identity(token, b"string", identity));
        ok(i, b"proc string args {return shadow}");
        assert!(!i.check_command_guard_identity(token, b"string", identity));
    });
}

/// The pin keeps every attestation and the pinned surface decides which of
/// them a guard reaches: a command the release lacks has none, and answers
/// again under a release that has it.
#[test]
fn a_pin_to_a_release_without_the_command_leaves_it_unattested() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    leak_free(|i| {
        // This is the actual shipped allocation, not a same-name host handler.
        let intrinsic = tcl_registry::IntrinsicId::ListAssign;
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key(i.runtime_version()),
        );
        let domains =
            GuardDomains::one(GuardDomain::CommandEnvironment).with(GuardDomain::VariableTrace);
        let token = i
            .prepare_command_guard(b"lassign", identity, domains)
            .expect("attested in the default release");
        assert!(i.check_command_guard(token, b"lassign"));

        i.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert!(!i.check_command_guard(token, b"lassign"));
        assert_eq!(
            i.prepare_command_guard(b"lassign", identity, domains),
            Err(GuardError::IdentityUnavailable)
        );

        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        assert!(i
            .prepare_command_guard(b"lassign", identity, domains)
            .is_ok());
    });
}

#[test]
fn command_trace_stales_the_string_intrinsic_guard() {
    leak_free(|i| {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            tcl_registry::IntrinsicId::StringLength.stable_id(),
            tcl_registry::IntrinsicId::StringLength.guard_semantics_key(i.runtime_version()),
        );
        let token = i
            .prepare_command_guard(
                b"string",
                identity,
                GuardDomains::one(GuardDomain::CommandTrace),
            )
            .unwrap();
        assert_eq!(
            i.eval_str(b"trace add command string rename callback"),
            Code::Ok
        );
        assert!(!i.check_command_guard(token, b"string"));
    });
}

/// The native Runtime returns its explicit host evaluation-limit error
/// before synchronously nested generic callbacks exhaust the host stack.
/// This host boundary is separate from the original native completions.
#[test]
fn deeply_nested_foreach_reaches_the_explicit_host_evaluation_limit() {
    // Native proof: naming.runtime.original-nested-foreach-source-completion
    // docs/design/analysis/name-resolution-proofs/runtime-original-nested-foreach-source-completion.md
    // The identical 300-body source completes in all six native providers.
    // These assertions exercise this Runtime's separately bounded host path.
    leak_free(|i| {
        let mut src = String::new();
        for _ in 0..300 {
            src.push_str("foreach x {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..300 {
            src.push_str("}\n");
        }
        assert_eq!(i.eval_str(src.as_bytes()), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"too many nested evaluations (infinite loop?)"
        );
    });
}

/// Defence in depth for the alias trampoline. `interp alias`
/// and `rename` both refuse a cycle at definition time, so plant one
/// straight into the command table — bypassing that gate the way only a
/// bug could — and confirm the dispatch bound turns what would otherwise be
/// unbounded native recursion (`invoke` → `dispatch_alias` → `invoke`, i.e.
/// stack exhaustion / a WASM trap) into a catchable Tcl error.
#[test]
fn a_planted_alias_cycle_errors_instead_of_exhausting_the_stack() {
    leak_free(|i| {
        let alias = |target: &[u8]| Command::Alias {
            original_prefix: Rc::new(NativeAliasHeader::new(
                i.generated_alias_prefix(target, &[]),
            )),
            registration_name: Some(Rc::new(NativeAliasHeader::new(obj::Owned::fresh(
                new_string(target),
            )))),
            publication_name: target.to_vec(),
            identity: Rc::new(()),
        };
        {
            let mut namespaces = i.namespaces.borrow_mut();
            namespaces.register(b"loop_a", alias(b"loop_b"));
            namespaces.register(b"loop_b", alias(b"loop_a"));
        }
        assert_eq!(i.eval_str(b"loop_a"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"too many nested alias invocations (infinite loop?)"
        );
        // The counter unwinds with the failed call, so a later alias call
        // is unaffected.
        assert_eq!(i.eval_str(b"interp alias {} = {} set"), Code::Ok);
        assert_eq!(i.eval_str(b"= v 1"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
    });
}

/// A moderately nested `foreach` (well under `NATIVE_EVAL_DEPTH_LIMIT`)
/// still runs to completion — the safety net must not fire on realistic
/// nesting depths.
#[test]
fn moderately_nested_foreach_still_runs() {
    leak_free(|i| {
        let mut src = String::new();
        for _ in 0..50 {
            src.push_str("foreach x {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..50 {
            src.push_str("}\n");
        }
        assert_eq!(i.eval_str(src.as_bytes()), Code::Ok);
        assert_eq!(i.eval_str(b"set done"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
    });
}

/// A recursive proc with no base case relies purely on a recursion cap
/// to terminate. Before `NATIVE_EVAL_DEPTH_LIMIT`, this overflowed the
/// native stack well before the pre-existing `RECURSION_LIMIT` (1000)
/// was ever reached — a real, unguarded crash on ordinary recursive Tcl
/// code, not just pathological control-flow nesting.
#[test]
fn unbounded_proc_recursion_errors_instead_of_crashing() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"proc r {} { r }"), Code::Ok);
        assert_eq!(i.eval_str(b"r"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"too many nested evaluations (infinite loop?)"
        );
    });
}

#[test]
fn set_and_read_back() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set x 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
    });
}

#[test]
fn command_substitution_closes_the_seam() {
    // [set y 42] evaluates the inner command; x becomes its result.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set x [set y 42]"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        assert_eq!(i.eval_str(b"set x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
    });
}

#[test]
fn incr_arithmetic() {
    leak_free(|i| {
        i.eval_str(b"set n 10");
        assert_eq!(i.eval_str(b"incr n"), Code::Ok);
        assert_eq!(i.result_bytes(), b"11");
        assert_eq!(i.eval_str(b"incr n 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"16");
        // incr of an unset var starts from 0
        assert_eq!(i.eval_str(b"incr fresh"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
    });
}

#[test]
fn undefined_variable_is_an_error() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set y $nope"), Code::Error);
        assert_eq!(i.result_bytes(), b"can't read \"nope\": no such variable");
    });
}

#[test]
fn unknown_command_is_an_error() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"frobnicate a b"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"frobnicate\"");
    });
}

#[test]
fn missing_alias_target_uses_custom_unknown_with_target_and_args() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(b"proc unknown {cmd args} {list unknown $cmd $args}"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"interp alias {} la {} nosuch"), Code::Ok);
        assert_eq!(i.eval_str(b"la a b"), Code::Ok);
        assert_eq!(i.result_bytes(), b"unknown nosuch {a b}");
    });
}

#[test]
fn missing_alias_target_without_unknown_preserves_target_error_identity() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp alias {} la {} nosuch"), Code::Ok);
        assert_eq!(i.eval_str(b"la a b"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"nosuch\"");
    });
}

#[test]
fn missing_alias_target_preserves_prefix_arguments_for_unknown() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(b"proc unknown {cmd args} {list $cmd $args}"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"interp alias {} la {} nosuch prefix"), Code::Ok);
        assert_eq!(i.eval_str(b"la one two"), Code::Ok);
        assert_eq!(i.result_bytes(), b"nosuch {prefix one two}");
    });
}

#[test]
fn missing_alias_target_selects_the_native_handler_context() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(
                i.eval_str(b"proc unknown {cmd args} {list global $cmd $args}"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace eval n {proc u {cmd args} {list ns $cmd $args}; namespace unknown ::n::u; interp alias {} la {} nosuch; la x}"), Code::Ok);
            let expected: &[u8] = if version == tcl_dialect::TclVersion::V8_5 {
                b"global nosuch x"
            } else {
                b"ns nosuch x"
            };
            assert_eq!(i.result_bytes(), expected);
        });
    }
}

#[test]
fn original_command_getter_uses_the_retained_lookup_namespace() {
    for version in tcl_dialect::TclVersion::ALL {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(i.eval_str(b"namespace eval A {proc p {} {return A}}; namespace eval B {proc p {} {return B}}"), Code::Ok);
            let head = crate::obj::Owned::fresh(new_string(b"p"));
            for name in [b"::A".as_slice(), b"::B", b"::A"] {
                let namespace = i.namespaces.borrow().find_namespace(GLOBAL, name).unwrap();
                let (command, generation) = i
                    .resolve_original_command_at(namespace, head.as_ptr())
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    i.invoke_bound(command, generation, &[head.as_ptr()]),
                    Code::Ok
                );
                assert_eq!(i.result_bytes(), &name[2..]);
            }
        });
    }
}

#[test]
fn tailcall_retired_namespace_relookup_observes_deletion_and_recreation() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_6)
    {
        for recreate in [false, true] {
            leak_free(|i| {
                i.set_runtime_version(version);
                let source: &[u8] = if recreate {
                    b"namespace eval N {proc issue {} {tailcall target}; proc target {} {return OLD}}; proc leave args {namespace delete N; namespace eval N {proc target {} {return NEW}}}; trace add execution N::issue leave leave; N::issue"
                } else {
                    b"namespace eval N {proc issue {} {tailcall target}; proc target {} {return OLD}}; proc leave args {namespace delete N}; trace add execution N::issue leave leave; N::issue"
                };
                assert_eq!(
                    i.eval_str(source),
                    if recreate { Code::Ok } else { Code::Error }
                );
                assert_eq!(
                    i.result_bytes(),
                    if recreate {
                        b"NEW".as_slice()
                    } else {
                        b"namespace \"::N\" not found"
                    }
                );
            });
        }
    }
}

#[test]
fn tailcall_selected_miss_does_not_relookup_the_callers_command() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_6)
    {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(i.eval_str(b"namespace eval Iss {proc p {} {tailcall missing}}; namespace eval Caller {proc missing {} {return WRONG}; proc u {cmd args} {return HANDLER}; namespace unknown ::Caller::u; ::Iss::p}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"HANDLER");
        });
    }
}

#[test]
fn missing_alias_unknown_cycle_is_bounded() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"interp alias {} la {} nosuch"), Code::Ok);
        assert_eq!(i.eval_str(b"proc unknown {cmd args} {la}"), Code::Ok);
        assert_eq!(i.eval_str(b"la"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"too many nested evaluations (infinite loop?)"
        );
    });
}

#[test]
fn release_hidden_alias_target_uses_unknown_under_tcl84() {
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(
            i.eval_str(b"proc unknown {cmd args} {list hidden $cmd $args}"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"interp alias {} la {} lassign"), Code::Ok);
        assert_eq!(i.eval_str(b"la {a b} x y"), Code::Ok);
        assert_eq!(i.result_bytes(), b"hidden lassign {{a b} x y}");
    });
}

#[test]
fn absolute_qualified_command_resolves() {
    leak_free(|i| {
        // `::set` is the global `set`, reached via the namespace resolver.
        assert_eq!(i.eval_str(b"::set x 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        // an unknown namespace qualifier is an error
        assert_eq!(i.eval_str(b"::nosuch::cmd a"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"::nosuch::cmd\"");
    });
}

#[cfg(have_tommath)]
#[test]
fn expr_command_end_to_end() {
    leak_free(|i| {
        // braced arithmetic with precedence
        assert_eq!(i.eval_str(b"expr {2 + 3 * 4}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"14");
        // a variable resolved through the frame store (object-preserving)
        assert_eq!(i.eval_str(b"set x 20"), Code::Ok);
        assert_eq!(i.eval_str(b"expr {$x * 2 + 2}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"42");
        // overflow promotes to a bignum, then a command substitution feeds back in
        assert_eq!(i.eval_str(b"expr {2 ** 64}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"18446744073709551616");
        assert_eq!(i.eval_str(b"expr {[set x] < 100}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        // divide by zero surfaces the verbatim error
        assert_eq!(i.eval_str(b"expr {1 / 0}"), Code::Error);
        assert_eq!(i.result_bytes(), b"divide by zero");
    });
}

#[cfg(have_tommath)]
#[test]
fn incr_promotes_to_bignum() {
    leak_free(|i| {
        // incr starts at 0 for an unset var
        assert_eq!(i.eval_str(b"incr n"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        // incr past a wide promotes to a bignum (never wraps)
        assert_eq!(i.eval_str(b"set big 9223372036854775807"), Code::Ok); // i64::MAX
        assert_eq!(i.eval_str(b"incr big"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9223372036854775808");
        // incrementing a bignum cell keeps working, and demotes when it fits
        assert_eq!(i.eval_str(b"incr big -1"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9223372036854775807");
        // a non-integer value is rejected verbatim
        assert_eq!(i.eval_str(b"set f 1.5"), Code::Ok);
        assert_eq!(i.eval_str(b"incr f"), Code::Error);
        assert_eq!(i.result_bytes(), b"expected integer but got \"1.5\"");
    });
}

/// The `incr` adapter's element + const paths, exercised through the shared
/// `ValueOps::int_add` seam: array elements increment (and widen) correctly,
/// an unset element starts at 0, and a `const` scalar is rejected before the
/// read-modify-write (the const check stays runtime-side).
#[cfg(have_tommath)]
#[test]
fn incr_element_and_const_via_seam() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set a(k) 10"), Code::Ok);
        assert_eq!(i.eval_str(b"incr a(k) 5"), Code::Ok);
        assert_eq!(i.result_bytes(), b"15");
        // an unset element starts at 0
        assert_eq!(i.eval_str(b"incr a(fresh)"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        // an element past a wide promotes to a bignum (never wraps)
        assert_eq!(i.eval_str(b"set a(big) 9223372036854775807"), Code::Ok);
        assert_eq!(i.eval_str(b"incr a(big)"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9223372036854775808");
        // a const scalar cannot be incremented
        assert_eq!(i.eval_str(b"const c 7"), Code::Ok);
        assert_eq!(i.eval_str(b"incr c"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't incr \"c\": variable is a constant"
        );
    });
}

#[test]
fn qualified_global_aliases_plain_at_top_level() {
    // `::pinged` and `pinged` resolve to the SAME global.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set ::pinged 1"), Code::Ok);
        assert_eq!(i.eval_str(b"set pinged"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(i.eval_str(b"set pinged 2"), Code::Ok);
        assert_eq!(i.eval_str(b"set ::pinged"), Code::Ok);
        assert_eq!(i.result_bytes(), b"2");
        assert_eq!(i.eval_str(b"unset ::pinged"), Code::Ok);
    });
}

#[test]
fn qualified_var_resolves_through_namespace_table() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"namespace eval a {}"), Code::Ok);
        // qualified write lands in ::a's var table …
        assert_eq!(i.eval_str(b"set ::a::x 5"), Code::Ok);
        // … visible as the unqualified `x` from inside ::a …
        assert_eq!(i.eval_str(b"namespace eval a { set x }"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        // … and through `$::a::x` substitution.
        assert_eq!(i.eval_str(b"set y $::a::x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"5");
        assert_eq!(i.eval_str(b"unset ::a::x"), Code::Ok);
    });
}

#[test]
fn qualified_set_into_missing_namespace_errors() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set ::nosuch::x 1"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't set \"::nosuch::x\": parent namespace doesn't exist"
        );
        // a read of the same name reports the ordinary no-such-variable.
        assert_eq!(i.eval_str(b"set ::nosuch::x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't read \"::nosuch::x\": no such variable"
        );
    });
}

#[test]
fn namespace_eval_body_set_is_a_namespace_var() {
    // `set x` inside `namespace eval` (a non-proc context) creates a ns var,
    // not a global.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"namespace eval b { set v 10 }"), Code::Ok);
        assert_eq!(i.eval_str(b"set ::b::v"), Code::Ok);
        assert_eq!(i.result_bytes(), b"10");
        assert_eq!(i.eval_str(b"set v"), Code::Error); // not a global
        assert_eq!(i.eval_str(b"unset ::b::v"), Code::Ok);
    });
}

#[test]
fn array_element_via_set_and_subst() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set a(k) hello"), Code::Ok);
        assert_eq!(i.eval_str(b"set out $a(k)"), Code::Ok);
        assert_eq!(i.result_bytes(), b"hello");
    });
}

#[test]
fn literal_element_references_match_the_selected_native_interpreter() {
    let mut engines: Vec<_> = tcl_test_support::available_tclshs()
        .into_iter()
        .map(|engine| (engine.path, Some(engine.version)))
        .collect();
    if let Some(engine) = tcl_test_support::locate_jimsh().expect("Jim oracle discovery") {
        engines.push((engine.path, None));
    }
    let scripts = [
            "set arr(k) ELEMENT; list ${arr(k)} $arr(k)",
            "set i k; set arr(k) ELEMENT; set {arr($i)} LITERAL; list ${arr($i)} $arr($i)",
            "set arr(k) ELEMENT; list prefix${arr(k)}suffix [subst {${arr(k)}}]",
            "set arr(k) ELEMENT; catch {set result ${arr(missing)}} result; set result",
            "proc probe {} {set arr(k) ELEMENT; set {arr($i)} LITERAL; set i k; list ${arr(k)} ${arr($i)} $arr($i)}; probe",
            "set code [catch {proc probe {{arr(k)}} {list [info exists arr] ${arr(k)} $arr(k) [set arr]}; probe FORMAL} result]; list $code $result",
        ];
    for (path, version) in engines {
        for script in scripts {
            let expected = tcl_test_support::run_script(
                &path,
                format!("set c [catch {{{script}}} r]; puts [list $c $r]\n").as_bytes(),
            )
            .expect("native variable reference execution");
            assert!(
                expected.success() && expected.stderr.is_empty(),
                "{expected:?}"
            );
            counters::reset();
            {
                let profile = version.map_or("jim", tcl_dialect::TclVersion::dialect_profile_name);
                let mut interp = Interp::with_native_core(
                    default_host(),
                    crate::environment::profile_for_dialect(profile),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .expect("selected original native interpreter constructor");
                let code = interp.eval_str(script.as_bytes());
                assert!(
                    !interp.host_refusal_pending(),
                    "{path:?}: {script}: {:?}",
                    interp.native_access_refusal(),
                );
                let observed = jim_list_bytes([
                    code.as_int().to_string().into_bytes(),
                    interp.result_bytes(),
                ]);
                assert_eq!(
                    observed,
                    expected
                        .stdout
                        .strip_suffix(b"\n")
                        .unwrap_or(&expected.stdout),
                    "{path:?}: {script}"
                );
                assert!(!interp.host_refusal_pending(), "{path:?}: {script}");
            }
            assert_eq!(
                counters::finalize(),
                0,
                "{path:?}: native source owner leaks"
            );
            assert_eq!(counters::double_free_count(), 0);
        }
    }
}

#[test]
#[cfg(not(have_tommath))]
fn an_unavailable_sugar_engine_cannot_be_caught_as_a_guest_error() {
    leak_free(|interp| {
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        let code =
            interp.eval_str(b"set prior 1; catch {set destination $(1+2)} captured; set after 1");
        assert_eq!(code, Code::Error);
        assert_eq!(
            interp.native_access_refusal(),
            Some(tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable)
        );
        assert_eq!(interp.var_get(b"prior").map(obj_bytes), Some(b"1".to_vec()));
        assert!(interp.var_get(b"captured").is_none());
        assert!(interp.var_get(b"destination").is_none());
        assert!(interp.var_get(b"after").is_none());
    });
}

#[test]
#[cfg(not(have_tommath))]
fn unavailable_safe_integer_engine_stops_after_the_reached_operand() {
    leak_free(|interp| {
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        let code =
            interp.eval_str(b"set prior 1; catch {string repeat x {1+1}} captured; set after 1");
        assert_eq!(code, Code::Error);
        assert_eq!(
            interp.native_access_refusal(),
            Some(tcl_syntax::raw_string::NativeValueAccessRefusal::ExpressionEngineUnavailable)
        );
        assert_eq!(interp.var_get(b"prior").map(obj_bytes), Some(b"1".to_vec()));
        assert!(interp.var_get(b"captured").is_none());
        assert!(interp.var_get(b"after").is_none());
    });
}

#[test]
#[cfg(have_tommath)]
fn jim_expression_sugar_bypasses_command_lookup_and_keeps_original_expression() {
    let jim = tcl_test_support::require_jimsh().expect("Jim expression sugar oracle");
    let scripts = [
            "rename expr oldexpr; proc expr args {return WRONG}; set x 3; list $(1+2) \"pre$($x+1)post\" [subst {$(1+2)}] [subst -novariables {$(1+2)}] [subst -nocommands {$(1+2)}]",
            "set seen 0; proc once {} {incr ::seen; return 4}; list $([once]+1) $seen",
            "set prior 1; set code [catch {set destination $(k)} result]; list $code $result $prior [info exists destination]",
            "set value $(1+(2*3)); list $value $()",
        ];
    for script in scripts {
        let expected = tcl_test_support::run_script(
            &jim.path,
            format!("set c [catch {{{script}}} r]; puts [list $c $r]\n").as_bytes(),
        )
        .expect("native sugar execution");
        assert!(
            expected.success() && expected.stderr.is_empty(),
            "{expected:?}"
        );
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            let code = interp.eval_str(script.as_bytes());
            let observed = jim_list_bytes([
                code.as_int().to_string().into_bytes(),
                interp.result_bytes(),
            ]);
            assert_eq!(
                observed,
                expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout),
                "{script}"
            );
            assert!(!interp.host_refusal_pending(), "{script}");
        });
    }
}

#[test]
#[cfg(have_tommath)]
fn jim_raw_expression_comparisons_errors_and_numeric_nul_match_native() {
    use tcl_test_support::expressions::{
        JIM_RAW_EXPRESSION_ERROR_SCRIPTS, JIM_RAW_EXPRESSION_VALUE_SCRIPTS,
        NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS,
    };
    let jim = tcl_test_support::require_jimsh().expect("required Jim expression oracle");
    for script in JIM_RAW_EXPRESSION_VALUE_SCRIPTS
        .iter()
        .chain(JIM_RAW_EXPRESSION_ERROR_SCRIPTS)
        .chain(NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS)
    {
        let observation = format!(
                "set ::errorCode NONE; set c [catch {{{script}}} r]; binary scan $r H* hx; list $c $hx $::errorCode"
            );
        let expected =
            tcl_test_support::run_script(&jim.path, format!("puts [{observation}]\n").as_bytes())
                .expect("native raw expression observation");
        assert!(
            expected.success() && expected.stderr.is_empty(),
            "{script}: {expected:?}"
        );
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
            assert_eq!(
                interp.eval_str(observation.as_bytes()),
                Code::Ok,
                "{script}"
            );
            assert!(!interp.host_refusal_pending(), "{script}");
            assert_eq!(
                interp.result_bytes(),
                expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout),
                "{script}"
            );
        });
    }
}

#[test]
#[cfg(have_tommath)]
fn numeric_nul_expression_observations_match_every_selected_native_engine() {
    let scripts = tcl_test_support::expressions::NUMERIC_NUL_EXPRESSION_OBSERVATION_SCRIPTS;
    assert_eq!(scripts.len(), 18);
    let mut engines: Vec<_> = tcl_test_support::available_tclshs()
        .into_iter()
        .map(|engine| (engine.path, Some(engine.version)))
        .collect();
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(tcl_test_support::require_jimsh().expect("required Jim numeric oracle"))
    } else {
        tcl_test_support::locate_jimsh().expect("Jim numeric oracle discovery")
    };
    if let Some(jim) = jim {
        engines.push((jim.path, None));
    }
    let mut failures = Vec::new();
    for (path, version) in engines {
        for (index, script) in scripts.iter().enumerate() {
            let expected =
                tcl_test_support::run_script(&path, format!("puts [{script}]\n").as_bytes())
                    .expect("native numeric NUL observation");
            assert!(
                expected.success() && expected.stderr.is_empty(),
                "{script}: {expected:?}"
            );
            leak_free(|interp| {
                match version {
                    Some(version) => interp.set_runtime_version(version),
                    None => {
                        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"))
                    }
                }
                let code = interp.eval_str(script.as_bytes());
                let observed = interp.result_bytes();
                let wanted = expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout);
                if code != Code::Ok || interp.host_refusal_pending() || observed != wanted {
                    failures.push(format!("{version:?} case {index}: {script}\ncode {code:?}, observed {observed:?}, expected {wanted:?}"));
                }
            });
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn original_variable_word_vectors_match_the_selected_native_interpreter() {
    use tcl_syntax::execution_conformance::{vectors, ExecutionDomain};
    let cases: Vec<_> = vectors(ExecutionDomain::CommandBinding)
        .into_iter()
        .filter(|case| case.id.starts_with("variable_word_"))
        .collect();
    assert_eq!(
        cases.len(),
        12,
        "retain every shared original-variable control"
    );
    let mut engines: Vec<_> = tcl_test_support::available_tclshs()
        .into_iter()
        .map(|engine| (engine.path, Some(engine.version)))
        .collect();
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(tcl_test_support::require_jimsh().expect("required Jim variable-word oracle"))
    } else {
        tcl_test_support::locate_jimsh().expect("Jim variable-word oracle discovery")
    };
    if let Some(engine) = jim {
        engines.push((engine.path, None));
    }
    let mut failures = Vec::new();
    for (path, version) in engines {
        for case in &cases {
            let expected = tcl_test_support::run_script(&path, case.script().as_bytes())
                .expect("native original-variable execution");
            assert!(
                expected.success() && expected.stderr.is_empty(),
                "{expected:?}"
            );
            leak_free(|interp| {
                match version {
                    Some(version) => interp.set_runtime_version(version),
                    None => interp.set_dialect_profile(
                        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
                    ),
                }
                let code = interp.eval_str(case.source.as_bytes());
                let observed = jim_list_bytes([
                    code.as_int().to_string().into_bytes(),
                    interp.result_bytes(),
                ]);
                let wanted = expected
                    .stdout
                    .strip_suffix(b"\n")
                    .unwrap_or(&expected.stdout);
                if observed != wanted || interp.host_refusal_pending() {
                    failures.push(format!(
                        "{path:?} {}: observed {observed:?}, expected {wanted:?}, host refusal {}",
                        case.id,
                        interp.host_refusal_pending(),
                    ));
                }
            });
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn expand_marker_splits_arguments() {
    // A recording builtin proves {*} produced the right argv.
    leak_free(|i| {
        fn record(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
            let joined: Vec<Vec<u8>> = argv[1..].iter().map(|&o| obj_bytes(o)).collect();
            interp.set_result_bytes(&joined.join(&b'|'));
            Code::Ok
        }
        i.register_builtin(b"record", record);
        i.eval_str(b"set lst {a b c}");
        assert_eq!(i.eval_str(b"record {*}$lst tail"), Code::Ok);
        assert_eq!(i.result_bytes(), b"a|b|c|tail");
    });
}

#[test]
fn return_sets_code() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"return done"), Code::Return);
        assert_eq!(i.result_bytes(), b"done");
    });
}

#[test]
fn multi_command_script() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set a 1; set b 2\nset c $a$b"), Code::Ok);
        assert_eq!(i.result_bytes(), b"12");
    });
}

#[test]
fn direct_dispatch_resets_the_prior_result_at_the_central_boundary() {
    leak_free(|i| {
        i.register_builtin(b"resultless", resultless_builtin);
        assert_eq!(i.eval_str(b"set stale prior"), Code::Ok);
        assert_eq!(i.result_bytes(), b"prior");

        let command = new_string(b"resultless");
        // `dispatch` borrows argv whose owners retain each object.
        unsafe { obj::incr_ref_count(command) };
        assert_eq!(i.dispatch(&[command]), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
        unsafe { obj::decr_ref_count(command) };
    });
}

#[test]
fn command_substitution_propagates_non_error_codes() {
    // A non-OK code other than `Error` (here `[return]`) propagates out of
    // `[...]` rather than being treated as an ordinary value (C Tcl). The
    // path is uniform (`code != Ok → Err(code)`), so this also covers
    // `break`/`continue` once those commands land.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set x [return foo]"), Code::Return);
        assert_eq!(i.result_bytes(), b"foo");
    });
}

#[test]
fn scalar_read_of_array_reports_variable_is_array() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set a(k) v"), Code::Ok);
        assert_eq!(i.eval_str(b"set a"), Code::Error);
        assert_eq!(i.result_bytes(), b"can't read \"a\": variable is array");
    });
}

#[test]
fn expand_split_error_names_the_right_failure() {
    // `{*}` over a value whose list form has an unmatched quote reports the
    // quote failure, not a hardcoded brace message.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"set s {\"abc}"), Code::Ok); // s = "abc
        assert_eq!(i.eval_str(b"list {*}$s"), Code::Error);
        assert_eq!(i.result_bytes(), b"unmatched open quote in list");
    });
}

/// Selecting the release selects the *numeral grammar* the whole runtime
/// reads with, because every numeral goes through one facility
/// (`tcl_syntax::number`) whose ambient dialect `set_runtime_version`
/// installs. Tcl 9.0 defines `KILL_OCTAL` in `tclStrToD.c`, so a bare
/// leading zero is decimal there (`0755` == 755) while 8.4/8.6 read it as
/// octal (`0755` == 493); `0b`/`0o` only exist from 8.5 and `0d` from 9.0.
/// Exercised through `expr`, which reads operands with
/// `ParseFlags::default()` — i.e. exactly the ambient this installs.
#[cfg(have_tommath)]
#[test]
fn expr_numerals_follow_the_release_selected_number_grammar() {
    use tcl_dialect::TclVersion;

    // TP: octal-by-leading-zero holds up to 8.6 …
    for version in [TclVersion::V8_4, TclVersion::V8_6] {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(ok(i, b"expr {0755}"), b"493", "{version:?}");
            assert_eq!(ok(i, b"expr {010 + 1}"), b"9", "{version:?}");
            // TN: without a leading zero the release cannot matter.
            assert_eq!(ok(i, b"expr {755}"), b"755", "{version:?}");
            // A leading-zero run before `.`/`e` stays a decimal float in
            // every release (C backtracks out of its octal state).
            assert_eq!(ok(i, b"expr {07.5}"), b"7.5", "{version:?}");
        });
    }

    // FN: 9.0 must not keep reading it as octal — the direction of this
    // change is easy to invert, so pin both sides.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"expr {0755}"), b"755");
        assert_eq!(ok(i, b"expr {010 + 1}"), b"11");
        assert_eq!(ok(i, b"expr {755}"), b"755");
        assert_eq!(ok(i, b"expr {07.5}"), b"7.5");
    });

    // FP: a prefix its release does not have is not a numeral at all, so
    // the word is a bareword rather than a silently-different number.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"expr {0o17}"), Code::Error);
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(ok(i, b"expr {0o17}"), b"15");
        assert_eq!(i.eval_str(b"expr {1_0}"), Code::Error);
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"expr {0o17}"), b"15");
        assert_eq!(ok(i, b"expr {1_0}"), b"10");
    });
}

/// Selecting the release selects the *lexing grammar* scripts parse
/// under and the builtin command surface: under an 8.4 pin `{*}` does
/// not expand and the first-close `${…}`
/// rule applies, and `lassign` (8.5+) resolves to `invalid command
/// name` — while a user-defined proc of the same name stays callable
/// (the compat-polyfill pattern).
#[test]
fn grammar_and_command_surface_follow_the_selected_release() {
    use tcl_dialect::TclVersion;

    // The `{*}` expansion grammar (TIP 157, 8.5+).
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"llength [list {*}{a b}]"), b"2");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        // No expansion under 8.4 (TIP 157 is 8.5+), so `{*}{a b}` is a
        // braced word `{*}` with `{a b}` welded onto its close-brace —
        // which C rejects. Measured on tclsh8.4.20: `llength [list
        // {*}{a b}]` reports `extra characters after close-brace`, where
        // 8.5.19/8.6.16/9.0.4/9.1b0 all answer `2`. The boundary owner's
        // `welded_after_close` enforces this: a braced word directly
        // followed by another brace-delimited word is rejected rather
        // than merged into one word.
        assert_eq!(i.eval_str(b"llength [list {*}{a b}]"), Code::Error);
        assert_eq!(i.result_bytes(), b"extra characters after close-brace");
    });

    // The `${…}` delimiting rule: 8.x stops at the first `}`
    // (its `Tcl_ParseVarName` counts no braces), 9.x nests. Verified
    // against tclsh8.4.20 / tclsh9.0.4.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"set \"a{b\" 5"), Code::Ok);
        assert_eq!(i.eval_str(b"set \"a{b}c\" 9"), Code::Ok);
        assert_eq!(ok(i, b"set r ${a{b}c}"), b"5c}");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"set \"a{b\" 5"), Code::Ok);
        assert_eq!(i.eval_str(b"set \"a{b}c\" 9"), Code::Ok);
        assert_eq!(ok(i, b"set r ${a{b}c}"), b"9");
    });

    // The builtin surface: lassign is 8.5+, lpop is 9.0+.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"lassign {a b} x"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
        assert_eq!(ok(i, b"llength [info commands lassign]"), b"0");
        // The polyfill pattern: a user proc wins over the hidden builtin.
        assert_eq!(
            i.eval_str(b"proc lassign {l args} { return polyfill }"),
            Code::Ok
        );
        assert_eq!(ok(i, b"lassign {a b} x"), b"polyfill");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(ok(i, b"lassign {a b} x"), b"b");
        assert_eq!(i.eval_str(b"set l {a b c}"), Code::Ok);
        assert_eq!(i.eval_str(b"lpop l"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lpop\"");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"set l {a b c}"), Code::Ok);
        assert_eq!(ok(i, b"lpop l"), b"c");
    });
}

/// The availability gate is a property of the **final resolved
/// builtin**, not of the spelling the caller wrote. Without that, the
/// two resolve-then-`invoke` shapes — the alias trampoline and the
/// `namespace import` redirect — could reach the builtin through a
/// second, ungated resolution and make an 8.4-hidden `lassign` callable
/// as `interp alias {} la {} lassign; la {a b} x y`.
///
/// Error identity is oracled against real tclsh 8.6.16 / 9.0.4:
/// `interp alias {} la {} nosuchcmd; la a b` reports `invalid command
/// name "nosuchcmd"` — the *target* name, not the alias — so a
/// release-hidden target reports the same way a deleted one does.
#[test]
fn the_release_command_surface_gates_alias_and_import_dispatch() {
    use tcl_dialect::TclVersion;

    // The reviewer's reproducer: an 8.4 alias to the 8.5+ `lassign`.
    // (Creating the alias still succeeds — real Tcl binds alias targets
    // lazily, so an alias to a nonexistent command is legal.)
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"interp alias {} la {} lassign"), Code::Ok);
        assert_eq!(i.eval_str(b"la {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
        // …and nothing was assigned through the alias.
        assert_eq!(i.eval_str(b"set x"), Code::Error);
        // An alias *chain* is gated at the final builtin too, not merely
        // at the first hop.
        assert_eq!(i.eval_str(b"interp alias {} lb {} la"), Code::Ok);
        assert_eq!(i.eval_str(b"lb {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
    });
    // The same alias is a working `lassign` on a release that carries it.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"interp alias {} la {} lassign"), Code::Ok);
        assert_eq!(ok(i, b"la {a b} x y; list $x $y"), b"a b");
    });

    // The `namespace import` redirect.  A leading `::` with no separator
    // before the command (`::lassign`) must retain its absolute-global
    // meaning while resolving the import source (the public `namespace
    // qualifiers` result intentionally returns an empty string here).
    // First pin the exact issue reproducer, then explicitly export the
    // builtin so the redirect and release-hidden dispatch halves remain
    // observable in the same test.
    const IMPORT: &[u8] = b"namespace eval n {namespace import ::lassign}";
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(IMPORT), Code::Ok);
        // This reproducer succeeds without an explicit export.  Global
        // builtins are not exported by default, so this
        // records no alias; the important contract is that it does not
        // resolve the absolute root as the destination namespace.
        assert_eq!(i.eval_str(b"namespace export lassign"), Code::Ok);
        assert_eq!(i.eval_str(IMPORT), Code::Ok);
        assert_eq!(ok(i, b"n::lassign {a b} x y; list $x $y"), b"a b");
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"n::lassign {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"::lassign\"");
    });

    // Direct dispatch is unchanged by moving the gate into the shared
    // resolver — both the hidden and the carried release.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"lassign {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lassign\"");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"lassign {a b} x y; list $x $y"), b"a b");
    });
}

/// An import keeps the final builtin's registry identity when that source
/// moves into the hidden table. Repinning the runtime surface must gate the
/// visible import, while an explicit `invokehidden` remains the privileged
/// path that deliberately bypasses ordinary command visibility.
#[test]
fn an_imported_hidden_builtin_keeps_its_release_gate() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(
            i.eval_str(
                b"namespace export lpop
                      namespace eval n {namespace import ::lpop}
                      set l {a b c}
                      interp hide {} lpop hp
                      n::lpop l",
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"c");

        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(i.eval_str(b"n::lpop l"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"::hp\"");
        assert_eq!(ok(i, b"set l"), b"a b");

        // Release visibility grants no cross-provider List backing. Supply
        // fresh current-release input after observing the earlier effect.
        assert_eq!(ok(i, b"set l {a b}"), b"a b");
        assert_eq!(
            i.eval_str(b"interp invokehidden {} hp l"),
            Code::Ok,
            "result={:?}, refusal={:?}",
            i.result_bytes(),
            i.native_execution_refusal(),
        );
        assert_eq!(i.result_bytes(), b"b");

        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"interp expose {} hp lz"), Code::Ok);
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(i.eval_str(b"lz l"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lz\"");
        assert_eq!(ok(i, b"info commands lz"), b"");
        i.set_runtime_version(TclVersion::V9_0);
    });
}

/// The same gate on the two *non*-dispatch shapes that would hand a
/// release-hidden builtin back ungated: `rename`, which would rebind it
/// under a name the registry has no spec for, and `interp hide`, which
/// would park it where `interp invokehidden` reaches it.
///
/// `rename`'s refusal is oracled against tclsh 8.6.16 / 9.0.4:
/// `rename nosuchcmd zz` → `can't rename "nosuchcmd": command doesn't
/// exist`; an empty destination uses `can't delete`. Hiding a command that
/// does not exist raises `unknown command`, matching Tcl_HideCommand and
/// avoiding a swallowed typo in a security-sensitive path.
#[test]
fn the_release_command_surface_gates_rename_and_hide() {
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_4);
        assert_eq!(i.eval_str(b"rename lassign lz"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't rename \"lassign\": command doesn't exist"
        );
        assert_eq!(i.eval_str(b"lz {a b} x y"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"lz\"");

        assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Error);
        assert_eq!(i.result_bytes(), b"unknown command \"lassign\"");
        assert_eq!(
            i.eval_str(b"interp invokehidden {} lassign {a b} x y"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"invalid hidden command name \"lassign\"");
        assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Error);
        assert_eq!(i.result_bytes(), b"unknown command \"lassign\"");
    });
    // Both stay ordinary operations on a release that carries `lassign`.
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"rename lassign lz"), Code::Ok);
        assert_eq!(ok(i, b"lz {a b} x y; list $x $y"), b"a b");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Ok);
        assert_eq!(
            ok(i, b"interp invokehidden {} lassign {a b} x y; list $x $y"),
            b"a b"
        );
    });
}

/// A fresh interpreter installs *its own* grammar rather than inheriting
/// whatever an earlier interpreter left in the thread-ambient slot — the
/// unchanged-version short-circuit in `set_runtime_version` would otherwise
/// leave a default-release interp reading numerals as 8.6.
#[cfg(have_tommath)]
#[test]
fn a_fresh_interp_reinstalls_the_number_grammar_after_an_8_6_interp() {
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(ok(i, b"expr {0755}"), b"493");
    });
    // Same thread, ambient left at 8.6 by the interp above.
    leak_free(|i| {
        assert_eq!(i.runtime_version(), DEFAULT_RUNTIME_VERSION);
        assert_eq!(ok(i, b"expr {0755}"), b"755");
        // …and re-pinning the release it already reports still installs it.
        i.set_runtime_version(DEFAULT_RUNTIME_VERSION);
        assert_eq!(ok(i, b"expr {0755}"), b"755");
    });
}

/// `return -code` / `try on` read their integer through the same one
/// facility, so the spellings they accept track the emulated release
/// (C reads it with `Tcl_GetIntFromObj`, whose grammar is the release's).
/// The ambient is thread-local, so each dialect is installed explicitly
/// here rather than inherited from another test.
#[test]
fn completion_code_integers_follow_the_release_selected_number_grammar() {
    use tcl_syntax::number::{set_runtime_syntax, NumberSyntax};

    // Release-independent: decimal, hex, the full signed *and* unsigned
    // 32-bit window, and its reduction to an `int`.
    for syntax in [
        NumberSyntax::Tcl84,
        NumberSyntax::Tcl85,
        NumberSyntax::Tcl90,
    ] {
        set_runtime_syntax(syntax);
        assert_eq!(parse_completion_int(b"42"), Some(42), "{syntax:?}");
        assert_eq!(parse_completion_int(b"+42"), Some(42), "{syntax:?}");
        assert_eq!(parse_completion_int(b"-7"), Some(-7), "{syntax:?}");
        assert_eq!(parse_completion_int(b" 7 "), Some(7), "{syntax:?}");
        assert_eq!(parse_completion_int(b"0x1f"), Some(31), "{syntax:?}");
        assert_eq!(
            parse_completion_int(b"-2147483648"),
            Some(i32::MIN),
            "{syntax:?}"
        );
        // The unsigned half is reachable and wraps to an `int`.
        assert_eq!(parse_completion_int(b"0xFFFFFFFF"), Some(-1), "{syntax:?}");
        assert_eq!(
            parse_completion_int(b"2147483648"),
            Some(i32::MIN),
            "{syntax:?}"
        );
        // Outside the window, a bare prefix, junk, a float, and a magnitude
        // past a wide are all rejected.
        assert_eq!(parse_completion_int(b"4294967296"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"-2147483649"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"0x"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b""), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"abc"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"1.5"), None, "{syntax:?}");
        assert_eq!(parse_completion_int(b"12x"), None, "{syntax:?}");
        assert_eq!(
            parse_completion_int(b"99999999999999999999"),
            None,
            "{syntax:?}"
        );
        // One sign only — the facility consumes it, so `--5` is not 5.
        assert_eq!(parse_completion_int(b"--5"), None, "{syntax:?}");
    }

    // Octal-by-leading-zero up to 8.6, decimal from 9.0.
    for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
        set_runtime_syntax(syntax);
        assert_eq!(parse_completion_int(b"010"), Some(8), "{syntax:?}");
        assert_eq!(parse_completion_int(b"-010"), Some(-8), "{syntax:?}");
        // An invalid octal digit stops the scan, so the whole word is not a
        // number (C's "bad octal" report).
        assert_eq!(parse_completion_int(b"08"), None, "{syntax:?}");
    }
    set_runtime_syntax(NumberSyntax::Tcl90);
    assert_eq!(parse_completion_int(b"010"), Some(10));
    assert_eq!(parse_completion_int(b"-010"), Some(-10));
    assert_eq!(parse_completion_int(b"08"), Some(8));

    // `0o`/`0b` arrive in 8.5, `0d` and `_` separators in 9.0 — an
    // unavailable prefix is not a prefix, so the word is not an integer.
    set_runtime_syntax(NumberSyntax::Tcl84);
    assert_eq!(parse_completion_int(b"0o17"), None);
    assert_eq!(parse_completion_int(b"0b101"), None);
    assert_eq!(parse_completion_int(b"0d99"), None);
    assert_eq!(parse_completion_int(b"1_0"), None);
    set_runtime_syntax(NumberSyntax::Tcl85);
    assert_eq!(parse_completion_int(b"0o17"), Some(15));
    assert_eq!(parse_completion_int(b"0b101"), Some(5));
    assert_eq!(parse_completion_int(b"0d99"), None);
    assert_eq!(parse_completion_int(b"1_0"), None);
    set_runtime_syntax(NumberSyntax::Tcl90);
    assert_eq!(parse_completion_int(b"0o17"), Some(15));
    assert_eq!(parse_completion_int(b"0b101"), Some(5));
    assert_eq!(parse_completion_int(b"0d99"), Some(99));
    assert_eq!(parse_completion_int(b"1_0"), Some(10));
}

/// The same release gate reached through the script surface: `return -level
/// 0 -code 010` completes with code 8 up to 8.6 and 10 from 9.0.
#[test]
fn return_code_word_reads_its_integer_in_the_emulated_release() {
    use tcl_dialect::TclVersion;

    leak_free(|i| {
        i.set_runtime_version(TclVersion::V8_6);
        assert_eq!(ok(i, b"catch {return -level 0 -code 010}"), b"8");
    });
    leak_free(|i| {
        i.set_runtime_version(TclVersion::V9_0);
        assert_eq!(ok(i, b"catch {return -level 0 -code 010}"), b"10");
    });
}
/// An empty operand (and a whitespace-only or sign-only one) must report a
/// Tcl error on **every** release rather than abort the process. Regression:
/// the shared parser's octal-by-leading-zero branch read its first byte
/// unguarded, so with a pre-9.0 grammar installed — which is exactly what
/// `set_runtime_version` now does — `expr {$empty + 1}` panicked in
/// `tcl_syntax::number::parse` instead of failing the expression.
#[cfg(have_tommath)]
#[test]
fn an_empty_numeral_errors_instead_of_panicking_on_every_release() {
    use tcl_dialect::TclVersion;

    for version in [TclVersion::V8_4, TclVersion::V8_6, TclVersion::V9_0] {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(
                i.eval_str(b"set x {}; expr {$x + 1}"),
                Code::Error,
                "{version:?}"
            );
            assert_eq!(
                i.eval_str(b"set x { }; expr {$x + 1}"),
                Code::Error,
                "{version:?}"
            );
            assert_eq!(
                i.eval_str(b"set x -; expr {$x + 1}"),
                Code::Error,
                "{version:?}"
            );
        });
    }
}

/// Every command `interp create -safe` hides, per release, measured on
/// the reference interpreters with
/// `interp create -safe s; lsort [interp hidden s]` — top-level command
/// names only. The `tcl:file:*` / `tcl:zipfs:*` / `tcl:clock:*` entries a
/// real 8.6+ interpreter also lists are C's internal rewrite names for
/// the *unsafe subcommands* of an ensemble, not commands a script can
/// name; this runtime does not model ensembles that way.
///
/// Patch levels measured: 8.4.20, 8.5.19, 8.6.14, 9.0.4, 9.1b0.
#[cfg(all(test, have_tommath))]
const MEASURED_SAFE_HIDDEN: &[(tcl_dialect::TclVersion, &[&str])] = &[
    (
        tcl_dialect::TclVersion::V8_4,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
        ],
    ),
    // 8.5 adds `unload` (TIP 100); 8.6 is the same set.
    (
        tcl_dialect::TclVersion::V8_5,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
        ],
    ),
    (
        tcl_dialect::TclVersion::V8_6,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
        ],
    ),
    // 9.0 adds `zipfs`.
    (
        tcl_dialect::TclVersion::V9_0,
        &[
            "cd",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
            "zipfs",
        ],
    ),
    // 9.1 additionally lists `clock` — an artefact of the safe base
    // hiding the C `clock` and immediately re-providing a safe one, not
    // an unsafety fact, so it is deliberately absent from the registry
    // trait (`clock format 0 -gmt 1` works inside a 9.1 safe child).
    (
        tcl_dialect::TclVersion::V9_1,
        &[
            "cd",
            "clock",
            "encoding",
            "exec",
            "exit",
            "fconfigure",
            "file",
            "glob",
            "load",
            "open",
            "pwd",
            "socket",
            "source",
            "unload",
            "zipfs",
        ],
    ),
];

/// The hidden set under each pinned release is exactly the measured
/// tclsh set, narrowed to the commands this runtime carries — `make_safe`
/// reads that set from the registry's `Traits::SAFE_INTERP_HIDDEN` query
/// rather than a hand-kept name list.
///
/// The narrowing *is* the per-release mechanism, not a fudge: `unload`
/// (8.5+) and `zipfs` (9.0+) are release-gated commands, so "hide what
/// the trait names, if this interpreter carries it" reproduces the
/// differences between the rows with no second availability rule. Any
/// residue is asserted to be genuinely absent from `info commands`, so
/// implementing one of them forces this test to be revisited.
#[cfg(have_tommath)]
#[test]
fn safe_interp_hidden_set_matches_the_measured_tclsh_sets() {
    for &(version, measured) in MEASURED_SAFE_HIDDEN {
        leak_free(|i| {
            i.set_runtime_version(version);
            // `clock` is implemented and stays *visible*, which is what a
            // real 9.1 safe child does behaviourally; only its appearance
            // in `interp hidden` differs.
            let carried: Vec<&str> = measured
                .iter()
                .copied()
                .filter(|name| {
                    *name != "clock" && {
                        let script = format!("llength [info commands {name}]");
                        ok(i, script.as_bytes()) == b"1"
                    }
                })
                .collect();
            assert_eq!(i.eval_str(b"set s [interp create -safe]"), Code::Ok);
            let hidden = ok(i, b"lsort [$s hidden]");
            let hidden = String::from_utf8_lossy(&hidden);
            let actual: Vec<&str> = hidden.split_whitespace().collect();
            assert_eq!(actual, carried, "{version:?} hidden set");
            i.eval_str(b"interp delete $s");
        });
    }
}

/// TP: `clock` stays callable inside a safe child on every release —
/// measured on tclsh 8.6.14, 9.0.4 and 9.1b0, where
/// `s eval {clock format 0 -gmt 1}` succeeds even though 9.1 lists
/// `clock` in `interp hidden`.
#[cfg(have_tommath)]
#[test]
fn clock_remains_callable_in_a_safe_child() {
    for &(version, _) in MEASURED_SAFE_HIDDEN {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(i.eval_str(b"set s [interp create -safe]"), Code::Ok);
            assert_eq!(
                ok(i, b"$s eval {clock format 0 -gmt 1 -format %Y}"),
                b"1970",
                "{version:?}"
            );
            i.eval_str(b"interp delete $s");
        });
    }
}

/// The core packages a bare interpreter pre-provides follow the pinned
/// release, so `package require Tcl 8.5` fails under a 9.x pin exactly as
/// `tclsh9.0` fails it, rather than succeeding the way it would if both
/// engines hardcoded `9.0.4`/`Tcl`+`tcl` regardless of the pin.
///
/// Measured (`package provide <name>` in a fresh `tclsh`):
/// 8.4.20 → `Tcl` = `8.4`, no `tcl`; 8.5.19 → `Tcl` = `8.5.19`;
/// 8.6.14 → `Tcl` = `8.6.14`, `TclOO` = `1.1.0`; 9.0.4 and 9.1b0 → all
/// four names, at the patch level / `1.3.1`.
#[cfg(have_tommath)]
#[test]
fn core_package_provides_follow_the_pinned_release() {
    use tcl_dialect::TclVersion;

    for version in TclVersion::ALL {
        leak_free(|i| {
            i.set_runtime_version(version);
            for core in version.core_provided_packages() {
                let script = format!("package provide {}", core.name);
                assert_eq!(
                    ok(i, script.as_bytes()),
                    core.version.as_bytes(),
                    "{version:?} provides {}",
                    core.name
                );
            }
            // FN guard: a name this release does not pre-provide answers
            // with the empty string, not a stale earlier pin's version.
            if version < TclVersion::V9_0 {
                assert_eq!(ok(i, b"package provide tcl"), b"", "{version:?}");
            }
            if version < TclVersion::V8_6 {
                assert_eq!(ok(i, b"package provide TclOO"), b"", "{version:?}");
            }
            // `package require Tcl 8.5` means [8.5, 9) — satisfied on
            // 8.5/8.6, a version conflict on 8.4 and on every 9.x.
            let wanted = matches!(version, TclVersion::V8_5 | TclVersion::V8_6);
            assert_eq!(
                i.eval_str(b"package require Tcl 8.5") == Code::Ok,
                wanted,
                "{version:?}: package require Tcl 8.5"
            );
        });
    }
}

/// `::tcl::build-info` reports the pinned release's build identity, and
/// splits its fields the way C does — `patchlevel` up to the `+`,
/// `version` up to the second `.`. Measured: `tclsh9.0` answers
/// `9.0.4` / `9.0`, and `tclsh9.1` answers `9.1.0` / `9.1`.
#[cfg(have_tommath)]
#[test]
fn build_info_follows_the_pinned_release() {
    use tcl_dialect::TclVersion;

    for (version, patchlevel, short) in [
        (TclVersion::V9_0, &b"9.0.4"[..], &b"9.0"[..]),
        (TclVersion::V9_1, &b"9.1.0"[..], &b"9.1"[..]),
    ] {
        leak_free(|i| {
            i.set_runtime_version(version);
            assert_eq!(ok(i, b"::tcl::build-info patchlevel"), patchlevel);
            assert_eq!(ok(i, b"::tcl::build-info version"), short);
            // An unset build flag reports 0 rather than erroring.
            assert_eq!(ok(i, b"::tcl::build-info memdebug"), b"0");
        });
    }
}

/// `set_dialect_profile` is the profile form of `pin_context`: the context a
/// profile names, resolved through the ingress, pins the same profile, the
/// same release and the same identity.
#[test]
fn the_profile_form_of_a_pin_is_the_context_the_profile_names() {
    for profile in tcl_dialect::DialectProfile::all().iter().chain([
        tcl_dialect::DialectProfile::plain_tcl(),
        tcl_dialect::DialectProfile::tk(),
    ]) {
        let mut by_profile = Interp::new();
        by_profile.set_dialect_profile(profile);
        let mut by_context = Interp::new();
        by_context
            .pin_context(&tcl_registry::model::runtime_context_for_profile(profile))
            .unwrap_or_else(|error| panic!("{}: {error}", profile.name));

        assert!(
            std::ptr::eq(by_context.dialect_profile(), by_profile.dialect_profile()),
            "{}",
            profile.name
        );
        assert_eq!(by_context.runtime_version(), by_profile.runtime_version());
        assert_eq!(by_context.runtime_context(), by_profile.runtime_context());
        assert_eq!(by_context.held_identity(), by_profile.held_identity());
    }
}

/// The runtime states the identity of the world it is pinned to: the
/// context's fields, this build's ABI, intrinsic table and embedded library,
/// and no pack facts.
#[test]
fn an_interp_states_the_identity_of_the_world_it_is_pinned_to() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
    let mut interp = Interp::new();
    interp.set_dialect_profile(profile);
    let expected = tcl_registry::model::runtime_context_for_profile(profile)
        .identity(&[], tcl_registry::intrinsic_table_hash());
    assert_eq!(interp.held_identity(), expected);
    assert_eq!(expected.environment, "tcl8.6");
    assert_eq!(expected.release, "8.6");
    assert_eq!(
        expected.abi_version,
        tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION
    );
    assert!(expected.packs.is_empty());

    interp.set_dialect_profile(tcl_dialect::DialectProfile::plain_tcl());
    assert_eq!(interp.held_identity().environment, "tcl");
}

/// A context the ingress does not agree with is an error and leaves the pin
/// as it was; an overlay nothing has installed is one of them, and is never
/// the un-overlaid generation under another name.
#[test]
fn a_context_the_ingress_refuses_leaves_the_pin_unchanged() {
    use tcl_registry::model::PinError;

    const OVERLAY: u64 = 0x0C0_1704;
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
    let context = tcl_registry::model::runtime_context_for_profile(profile);
    let mut interp = Interp::new();
    interp
        .pin_context(&context)
        .expect("the profile's own context");

    let mut unknown = context.clone();
    unknown.environment = "no-such-environment".to_owned();
    let mut wrong_build = context.clone();
    wrong_build.build = tcl_dialect::model::BuildProfileId::JimFull;
    let mut missing_overlay = context.clone();
    missing_overlay.overlay_generation = OVERLAY;
    for (what, refused) in [
        ("unknown", unknown),
        ("build", wrong_build),
        ("overlay", missing_overlay.clone()),
    ] {
        assert!(interp.pin_context(&refused).is_err(), "{what}");
        assert!(std::ptr::eq(interp.dialect_profile(), profile), "{what}");
        assert_eq!(interp.runtime_context(), context, "{what}");
    }
    assert!(matches!(
        interp.pin_context(&missing_overlay),
        Err(PinError::OverlayMiss(miss)) if miss.overlay == OVERLAY
    ));

    tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
    interp
        .pin_context(&missing_overlay)
        .expect("installed, so it pins");
    assert_eq!(interp.runtime_context().overlay_generation, OVERLAY);
    assert!(std::ptr::eq(interp.dialect_profile(), profile));
}
