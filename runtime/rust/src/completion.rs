// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact public completion capture, with host refusals outside guest values.
//! The shared Family-B capture supplies original object ownership and options.
//! The host channel is checked before object access and after every materialiser.

use crate::interp::{Code as RuntimeCode, Interp};
use crate::obj;
use tcl_runtime_api::{NativeExecutionError, ScriptCompletion};

fn refuse_access(
    interp: &mut Interp,
    error: tcl_syntax::value::ValueError,
) -> NativeExecutionError {
    interp.refuse_completion_value_access(error)
}

/// Snapshot actual guest result/options bytes or return the original host cause.
pub(crate) fn capture_bytes(
    interp: &mut Interp,
    code: RuntimeCode,
) -> Result<ScriptCompletion, NativeExecutionError> {
    if let Some(error) = interp.native_execution_refusal() {
        return Err(error);
    }
    let completion = crate::state_traits::capture_completion(interp, code);
    if completion.result.is_null() || completion.options.is_null() {
        // The pointer completion's transport placeholders grant no object access.
        // Release any actual independent reference without touching a null slot.
        unsafe {
            if !completion.result.is_null() {
                obj::decr_ref_count(completion.result);
            }
            if !completion.options.is_null() {
                obj::decr_ref_count(completion.options);
            }
        }
        interp.refuse_host_command("native completion contains no original guest object");
        return Err(interp
            .native_execution_refusal()
            .expect("original host cause retained"));
    }
    // SAFETY: shared capture transfers one live independent reference to each.
    let result = unsafe { obj::Owned::from_raw(completion.result) };
    let options = unsafe { obj::Owned::from_raw(completion.options) };
    if let Some(error) = interp.native_execution_refusal() {
        return Err(error);
    }
    let result = interp
        .native_object_string_bytes(result.as_ptr())
        .map_err(|error| refuse_access(interp, error))?;
    let options = interp
        .native_object_string_bytes(options.as_ptr())
        .map_err(|error| refuse_access(interp, error))?;
    if let Some(error) = interp.native_execution_refusal() {
        return Err(error);
    }
    let mut projected = ScriptCompletion::new(completion.code, result.to_vec(), options.to_vec());
    projected.option_origin = completion.option_origin;
    Ok(projected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::raw_string::NativeValueAccessRefusal;

    #[test]
    fn retained_host_cause_never_reads_null_transport_objects_or_changes_guest_state() {
        let mut interp = Interp::new();
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        interp.set_result_bytes(b"original\0\xff");
        interp.set_return_state(2, RuntimeCode::Other(7));
        interp.set_return_options(vec![(b"-custom".to_vec(), b"original".to_vec())]);
        interp.refuse_host_command("first original cause");
        let expected = interp.native_host_command_refusal().unwrap();
        let error = capture_bytes(&mut interp, RuntimeCode::Error).unwrap_err();
        assert_eq!(
            error,
            NativeExecutionError::HostCommandRefusal(Box::new(expected))
        );
        assert_eq!(interp.result_bytes(), b"original\0\xff");
        assert_eq!(interp.pending_return_level(), 2);
        assert_eq!(interp.pending_return_code(), RuntimeCode::Other(7));
        assert_eq!(interp.pending_return_option_objects().len(), 1);
        interp.refuse_native_access(NativeValueAccessRefusal::ExpressionEngineUnavailable);
        assert!(
            interp.native_access_refusal().is_none(),
            "earlier cause remains authoritative"
        );
    }

    fn refusal_callback(interp: &mut Interp, _: &[*mut obj::TclObj]) -> RuntimeCode {
        interp.refuse_host_command("reached original callback")
    }

    #[test]
    fn public_completion_returns_original_reached_host_metadata_outside_catch() {
        let mut interp = Interp::new();
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        interp.register_builtin(b"refusal_callback", refusal_callback);
        let error = interp.eval_completion(b"namespace eval scope {proc within {} {set ::before 1; catch {refusal_callback}; set ::after 1}; within}").unwrap_err();
        let NativeExecutionError::HostCommandRefusal(failure) = error else {
            panic!("lost original host cause");
        };
        assert_eq!(failure.reason, "reached original callback");
        assert_eq!(failure.namespace.as_ref(), "::scope");
        assert_eq!(failure.frame, 1);
        assert_eq!(failure.source_profile, interp.dialect_profile().cache_key());
        assert_eq!(failure.native_profile, interp.dialect_profile().cache_key());
        assert_eq!(Some(*failure), interp.native_host_command_refusal());
        assert!(interp.var_get(b"::before").is_ok());
        assert!(interp.var_get(b"::after").is_err());
        let completion = interp
            .eval_sourced_completion(
                b"catch {refusal_callback}; set ::after 1",
                b"original-source.tcl",
            )
            .unwrap_err();
        assert!(matches!(
            completion,
            NativeExecutionError::HostCommandRefusal(_)
        ));
        assert!(interp.var_get(b"::after").is_err());
    }

    #[test]
    fn child_host_transport_retains_original_metadata_and_an_earlier_parent_cause() {
        let mut child = Interp::new();
        child.set_runtime_version(tcl_dialect::TclVersion::V8_5);
        child.refuse_host_command("child original cause");
        let expected = child.native_host_command_refusal().unwrap();
        let mut parent = Interp::new();
        parent.set_runtime_version(tcl_dialect::TclVersion::V9_1);
        parent.set_result_bytes(b"parent original");
        parent.transport_host_refusal_from(&child);
        assert_eq!(parent.native_host_command_refusal(), Some(expected));
        assert_eq!(parent.result_bytes(), b"parent original");
        parent.reset_native_compilation_admission();
        parent.refuse_native_access(NativeValueAccessRefusal::ExpressionEngineUnavailable);
        parent.transport_host_refusal_from(&child);
        assert_eq!(
            parent.native_execution_refusal(),
            Some(NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::ExpressionEngineUnavailable
            ))
        );
        assert!(parent.native_host_command_refusal().is_none());
    }
}

#[cfg(test)]
mod c_publication_tests {
    use super::*;
    use crate::interp::ObjCommand;
    use crate::obj::TclObj;
    use std::ffi::{c_int, c_void};

    unsafe extern "C" fn procedure(
        _: *mut c_void,
        interp: *mut Interp,
        _: c_int,
        _: *const *mut TclObj,
    ) -> c_int {
        // SAFETY: the actual command dispatcher supplies its live interpreter.
        unsafe { &mut *interp }.set_result_bytes(b"native callback");
        0
    }

    fn install(interp: &mut Interp, _: &[*mut TclObj]) -> RuntimeCode {
        let global = interp
            .create_obj_command(
                b"plain",
                ObjCommand::new(procedure, std::ptr::null_mut(), None),
            )
            .unwrap();
        let relative = interp
            .create_obj_command(
                b"child::qualified",
                ObjCommand::new(procedure, std::ptr::null_mut(), None),
            )
            .unwrap();
        let (global_ns, global_simple) = interp
            .namespaces()
            .native_command_slot_at_node(global)
            .unwrap();
        let (relative_ns, relative_simple) = interp
            .namespaces()
            .native_command_slot_at_node(relative)
            .unwrap();
        assert_eq!(global_ns, crate::namespace::GLOBAL);
        assert_eq!(global_simple, b"plain");
        assert_ne!(relative_ns, crate::namespace::GLOBAL);
        assert_eq!(relative_simple, b"qualified");
        assert_eq!(
            interp
                .namespaces()
                .command_generation(global_ns, &global_simple),
            Some(global)
        );
        assert_eq!(
            interp
                .namespaces()
                .command_generation(relative_ns, &relative_simple),
            Some(relative)
        );
        assert_eq!(
            interp.namespaces().qualified_name(relative_ns),
            b"::scope::child"
        );
        RuntimeCode::Ok
    }

    // Software integration of the shared C publication recipe and actual table
    // generation. The C entry's name windows are proved by the naming owner.
    #[test]
    fn c_object_creation_uses_global_unqualified_and_actual_current_qualified_holders() {
        for version in [
            tcl_dialect::TclVersion::V8_4,
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            interp.register_builtin(b"install_original", install);
            assert_eq!(interp.eval_completion(b"namespace eval scope {install_original}; list [plain] [::scope::child::qualified]").unwrap().result,
                b"{native callback} {native callback}");
        }
    }
}
