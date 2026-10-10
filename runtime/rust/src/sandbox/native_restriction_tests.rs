// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{
    interp::{Code, Interp, ObjCommand},
    obj::{Owned, TclObj},
};
use core::ffi::{c_int, c_void};

fn live_generations(interp: &Interp) -> Vec<u64> {
    interp
        .namespaces()
        .native_command_generations()
        .into_iter()
        .filter(|generation| {
            interp
                .namespaces()
                .native_command_slot_at_node(*generation)
                .is_some()
        })
        .collect()
}

fn core() -> Interp {
    Interp::with_native_core(
        Interp::new().host(),
        tcl_registry::model::resolve_environment("tcl8.6").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn stock_dependency_closure_retains_generations_and_drops_prefix_impostors() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = core();
    let worker = interp.resolve_cmd_token(b"::tcl::dict::get").unwrap();
    interp.eval_completion(b"rename ::dict ::renamed_dict; rename ::tcl::dict::get ::moved_worker; proc ::tcl::dict::get {} {set ::replacement_ran 1}; proc ::tcl::dict::impostor {} {set ::impostor_ran 1}").unwrap();
    let replacement = interp.resolve_cmd_token(b"::tcl::dict::get").unwrap();
    assert_ne!(replacement, worker);
    interp.restrict_to_tokens(&["renamed_dict"], &[]).unwrap();
    assert_eq!(interp.resolve_cmd_token(b"::moved_worker"), Some(worker));
    assert!(interp.resolve_cmd_token(b"::tcl::dict::get").is_none());
    assert!(interp.resolve_cmd_token(b"::tcl::dict::impostor").is_none());
    assert_eq!(
        interp
            .eval_completion(b"::renamed_dict size {a 1 b 2}")
            .unwrap()
            .result,
        b"2"
    );
}

#[cfg(have_tommath)]
#[test]
fn only_original_expression_registration_keeps_shared_worker_inventory() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut original = core();
    let absolute = original.resolve_cmd_token(b"::tcl::mathfunc::abs").unwrap();
    original.eval_completion(b"rename ::expr ::renamed_expr; rename ::tcl::mathfunc::abs ::moved_abs; proc ::tcl::mathfunc::impostor {} {set ::impostor_ran 1}").unwrap();
    original.restrict_to_tokens(&["renamed_expr"], &[]).unwrap();
    assert_eq!(original.resolve_cmd_token(b"::moved_abs"), Some(absolute));
    assert!(
        original
            .resolve_cmd_token(b"::tcl::mathfunc::rand")
            .is_none()
    );
    assert!(
        original
            .resolve_cmd_token(b"::tcl::mathfunc::srand")
            .is_none()
    );
    assert!(
        original
            .resolve_cmd_token(b"::tcl::mathfunc::impostor")
            .is_none()
    );
    let mut replaced = core();
    replaced
        .eval_completion(b"rename ::expr {}; proc ::expr {} {return REPLACED}")
        .unwrap();
    replaced.restrict_to_tokens(&["expr"], &[]).unwrap();
    assert!(
        replaced
            .resolve_cmd_token(b"::tcl::mathfunc::abs")
            .is_none()
    );
}

#[test]
fn legacy_restriction_validates_both_lists_before_effects_and_preserves_first_cause() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for (allowed, kept) in [
        (b"{".as_slice(), b"".as_slice()),
        (b"".as_slice(), b"opaque\xff".as_slice()),
    ] {
        let mut interp = core();
        interp.set_result_bytes(b"ORIGINAL\0\xff");
        let before = live_generations(&interp);
        let allowed = Owned::fresh(crate::interp::new_string(allowed));
        let kept = Owned::fresh(crate::interp::new_string(kept));
        // SAFETY: two original objects and interpreter remain live.
        unsafe {
            crate::engine_abi::tcl_engine_restrict(&mut interp, allowed.as_ptr(), kept.as_ptr())
        };
        let first = interp.native_execution_refusal().unwrap();
        assert_eq!(live_generations(&interp), before);
        assert_eq!(interp.result_bytes(), b"ORIGINAL\0\xff");
        let empty = Owned::fresh(crate::interp::new_string(b""));
        // SAFETY: repeated compatibility getter must not consume the first cause.
        unsafe {
            crate::engine_abi::tcl_engine_restrict(&mut interp, empty.as_ptr(), empty.as_ptr())
        };
        assert_eq!(interp.native_execution_refusal(), Some(first));
        assert_eq!(live_generations(&interp), before);
    }
}

struct Retirement {
    interp: Interp,
    calls: std::rc::Rc<std::cell::Cell<usize>>,
}
unsafe extern "C" fn original_command(
    _: *mut c_void,
    _: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    0
}
unsafe extern "C" fn retirement_callback(client: *mut c_void) {
    // SAFETY: the original command owns this callback state until its single drop.
    let mut state = unsafe { Box::from_raw(client.cast::<Retirement>()) };
    state.calls.set(state.calls.get() + 1);
    // This independent interpreter handle shares the original tables without
    // reborrowing the caller's mutable handle or retaining a table borrow.
    state
        .interp
        .register_builtin(b"retirement_original", |_, _| Code::Ok);
    state.interp.set_result_bytes(b"REACHED\0\xff");
    state
        .interp
        .refuse_host_command("original restriction retirement callback");
}

#[test]
fn retirement_callback_keeps_reentrant_replacement_and_stops_at_first_host_cause() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = core();
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let state = Box::new(Retirement {
        interp: interp.clone(),
        calls: std::rc::Rc::clone(&calls),
    });
    let original = interp
        .create_obj_command(
            b"retirement_original",
            ObjCommand::new(
                original_command,
                Box::into_raw(state).cast(),
                Some(retirement_callback),
            ),
        )
        .unwrap();
    interp.register_builtin(b"later_victim", |_, _| Code::Ok);
    let later = interp.resolve_cmd_token(b"later_victim").unwrap();
    let failure = interp.restrict_to_tokens(&[], &[]).unwrap_err();
    assert_eq!(calls.get(), 1);
    assert_eq!(interp.native_execution_refusal(), Some(failure.clone()));
    assert_eq!(interp.result_bytes(), b"REACHED\0\xff");
    assert_eq!(interp.resolve_cmd_token(b"later_victim"), Some(later));
    assert_ne!(
        interp.resolve_cmd_token(b"retirement_original"),
        Some(original)
    );
    assert!(interp.resolve_cmd_token(b"retirement_original").is_some());
    assert_eq!(interp.restrict_to_tokens(&[], &[]), Err(failure));
    assert_eq!(calls.get(), 1);
}

#[test]
fn changed_stock_mapping_cannot_donate_original_worker_retention() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = core();
    let original = interp.resolve_cmd_token(b"::tcl::dict::size").unwrap();
    interp.eval_completion(b"proc ::replacement {} {set ::replacement_ran 1}; namespace ensemble configure ::dict -map {size ::replacement}").unwrap();
    interp.restrict_to_tokens(&["dict"], &[]).unwrap();
    assert!(interp.resolve_cmd_token(b"::replacement").is_none());
    assert!(
        interp
            .namespaces()
            .native_command_slot_at_node(original)
            .is_none()
    );
    assert!(interp.resolve_cmd_token(b"dict").is_some());
}
