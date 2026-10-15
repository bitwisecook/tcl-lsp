// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::interp::{Code, TclObj};
use tcl_syntax::native_jim_lookup::NativeJimVariableInventoryLookup;

fn jim() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn original_jim_catch_error_code_keys_are_published_owned_and_retired_with_the_entry() {
    let mut actual = jim();
    let name = tcl_registry::special_vars::NativeBootstrapVariable::ErrorCode
        .name()
        .as_bytes();
    assert_eq!(actual.eval_str(b"catch {}"), Code::Ok);
    let inventory = actual
        .namespaces()
        .var_table(GLOBAL)
        .capture_original_jim_inventory(true, NativeJimVariableInventoryLookup::Exact(name))
        .unwrap();
    assert_eq!(inventory.keys().len(), 1);
    let original = inventory.keys()[0].original().unwrap();
    // SAFETY: the real table owns this current original key.
    assert_eq!(unsafe { (*original).ref_count }, 1);
    assert!(obj::with_jim_variable_cache(original, |_| ()).is_some());
    assert_eq!(actual.eval_str(b"catch {}"), Code::Ok);
    assert_eq!(inventory.keys()[0].original().unwrap(), original);
    assert!(actual.var_unset(name));
    assert!(inventory.check().is_err());
    assert!(inventory.keys()[0].original().is_err());
    assert!(obj::check_native_liveness(original).is_err());
    assert_eq!(actual.eval_str(b"catch {}"), Code::Ok);
    let replacement = actual
        .namespaces()
        .var_table(GLOBAL)
        .capture_original_jim_inventory(true, NativeJimVariableInventoryLookup::Exact(name))
        .unwrap();
    assert_ne!(replacement.keys()[0].original().unwrap(), original);
    assert!(actual.native_execution_refusal().is_none());
}

fn reset_in_selected_caller(actual: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let frame = super::super::native_arguments::OriginalArgumentFrame::capture(actual);
    let namespace = actual.current_ns();
    assert_ne!(namespace, GLOBAL);
    assert_ne!(actual.frames.borrow().current_frame_index(), 0);
    let local = actual.var_get(b"errorCode").unwrap();
    let local = obj::NativeObjectLifetime::retain(local);
    let reached = actual.reset_original_jim_catch_error_code();
    assert_eq!(
        reached.is_err(),
        crate::interp::obj_bytes(argv[1]) == b"failure"
    );
    frame.check_frame(actual).unwrap();
    assert_eq!(actual.current_ns(), namespace);
    assert_eq!(
        actual.var_get(b"errorCode"),
        Some(local.checked_ptr().unwrap())
    );
    assert!(actual.native_execution_refusal().is_none());
    actual.set_result_bytes(b"");
    Code::Ok
}

#[test]
fn original_jim_global_reset_restores_the_actual_namespaced_caller_after_success_and_failure() {
    for (setup, outcome) in [
        (b"".as_slice(), "success"),
        (
            b"array set ::errorCode {member VALUE}".as_slice(),
            "failure",
        ),
    ] {
        let mut actual = jim();
        actual.register_builtin(b"reset_selected_error_code", reset_in_selected_caller);
        assert_eq!(actual.eval_str(setup), Code::Ok);
        let source = format!(
            "namespace eval CatchKey {{proc p {{}} {{set errorCode LOCAL; reset_selected_error_code {outcome}}}; p}}"
        );
        assert_eq!(
            actual.eval_str(source.as_bytes()),
            Code::Ok,
            "{:?}",
            actual.native_execution_refusal()
        );
        assert_eq!(actual.current_ns(), GLOBAL);
        assert_eq!(actual.frames.borrow().current_frame_index(), 0);
        assert!(actual.native_execution_refusal().is_none());
    }
}
