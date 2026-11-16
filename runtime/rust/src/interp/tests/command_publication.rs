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

//! Publishing a C command: the namespace holder, delete procedures and client data retired around the publication.

use super::*;

struct PublicationNamespaceRetirement {
    pub(super) interpreter: std::rc::Weak<InterpState>,
    namespace: NsId,
    pub(super) visible: Rc<std::cell::Cell<Option<u64>>>,
    recreated: Rc<std::cell::Cell<Option<NsId>>>,
    pub(super) calls: Rc<std::cell::Cell<usize>>,
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
