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

thread_local! {
    static MODE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

const GLOBAL_OBSERVATIONS: &str = include_str!(
    "../../../tests/data/native_jim_catch_try_original_output_keys/observations-20261015.tsv"
);

fn reset_in_selected_caller(actual: &mut Interp, _: &[*mut TclObj]) -> Code {
    let frame = super::super::native_arguments::OriginalArgumentFrame::capture(actual);
    let namespace = actual.current_ns();
    assert_ne!(namespace, GLOBAL);
    assert_ne!(actual.frames.borrow().current_frame_index(), 0);
    let local = actual.var_get(b"errorCode").unwrap();
    let local = obj::NativeObjectLifetime::retain(local);
    let reached = actual.reset_original_jim_catch_error_code();
    let mode = MODE.with(std::cell::Cell::get);
    let row = GLOBAL_OBSERVATIONS
        .lines()
        .find(|row| row.starts_with(&format!("GLOBAL_RESET|{mode}|")))
        .unwrap()
        .split('|')
        .collect::<Vec<_>>();
    assert_eq!(reached.is_err(), row[2] != "0");
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
    for line in include_str!(
        "../../../tests/data/native_jim_catch_try_original_output_keys/global-setups.tsv"
    )
    .lines()
    {
        let (mode, setup) = line.split_once('|').unwrap();
        MODE.with(|selected| selected.set(mode.parse().unwrap()));
        let mut actual = jim();
        actual.register_builtin(b"reset_selected_error_code", reset_in_selected_caller);
        assert_eq!(actual.eval_str(setup.as_bytes()), Code::Ok);
        assert_eq!(
            actual.eval_str(include_bytes!(
                "../../../tests/data/native_jim_catch_try_original_output_keys/global-call.tcl"
            )),
            Code::Ok,
            "{:?}",
            actual.native_execution_refusal()
        );
        assert_eq!(actual.current_ns(), GLOBAL);
        assert_eq!(actual.frames.borrow().current_frame_index(), 0);
        assert!(actual.native_execution_refusal().is_none());
    }
}
