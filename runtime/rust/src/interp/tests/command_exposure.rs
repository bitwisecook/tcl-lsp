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

//! Exposing a hidden command: retirement callbacks that run during cleanup, first causes and replacement generations.

use super::*;
use crate::interp::commands::CommandVisibilityOutcome;

struct ExposureRetirement {
    pub(super) interpreter: std::rc::Weak<InterpState>,
    pub(super) calls: Rc<std::cell::Cell<usize>>,
    pub(super) mode: u8,
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
