// SPDX-License-Identifier: AGPL-3.0-or-later
//! Copied procedure and variable publication owners; software controls.
use super::*;
use crate::obj::Owned;
use std::cell::{Cell, RefCell};
thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static DESTINATION: Cell<NsId> = const { Cell::new(GLOBAL) };
    static REBORN: Cell<NsId> = const { Cell::new(GLOBAL) };
    static ARMED: Cell<bool> = const { Cell::new(false) };
    static FREES: Cell<usize> = const { Cell::new(0) };
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|value| value.borrow_mut().take());
    }
}
fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
extern "C" fn duplicate(_: *mut TclObj, _: *mut TclObj) {}
extern "C" fn retire(_: *mut TclObj) {
    if !ARMED.with(|value| value.replace(false)) {
        return;
    }
    FREES.with(|value| value.set(value.get() + 1));
    let Some(mut interpreter) = OWNER.with(|value| {
        value
            .borrow()
            .as_ref()
            .and_then(crate::interp::WeakInterp::upgrade)
    }) else {
        return;
    };
    assert!(interpreter
        .namespaces_mut()
        .command_names(DESTINATION.with(Cell::get))
        .is_empty());
    interpreter.set_result_bytes(b"REACHED COPY");
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("copied procedure retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("procedure-copy".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            interpreter.delete_namespace_by_id(DESTINATION.with(Cell::get));
            let new = interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"Destination");
            REBORN.with(|value| value.set(new));
        }
        _ => {}
    }
}
static BODY: obj::TclObjType = obj::TclObjType {
    name: c"originalCopiedProcedure".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
#[test]
fn original_namespace_copy_checks_duplicate_cleanup_before_original_holder_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual duplicate primary retirement; no C copy chronology or Class grant.
    for mode in 0..4 {
        crate::counters::reset();
        {
            let mut interpreter = native("tcl8.6");
            let source = interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"Source");
            let destination = interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"Destination");
            let body = Owned::fresh(obj::alloc_typed(&BODY, 0));
            unsafe { obj::set_string_rep(body.as_ptr(), b"return OLD") };
            let definition = crate::interp::native_procedure_resources::NativeProcedureDefinition {
                location: crate::interp::ProcLocation {
                    namespace: source,
                    qualified_name: b"::Source::p".to_vec(),
                    jim_namespace: None,
                },
                jim_parameters: None,
                source: None,
                body_line_base: 0,
                native: None,
                statics: None,
            };
            let command = crate::interp::NativeProcedureCommand::new(Vec::new(), body, definition);
            interpreter.bind_command_replacement(source, b"p", Command::Proc(command));
            OWNER.with(|value| *value.borrow_mut() = Some(interpreter.downgrade()));
            let _cleanup = Cleanup;
            MODE.with(|value| value.set(mode));
            DESTINATION.with(|value| value.set(destination));
            REBORN.with(|value| value.set(destination));
            FREES.with(|value| value.set(0));
            ARMED.with(|value| value.set(true));
            let reached = interpreter.oo_clone_namespace(source, destination);
            assert_eq!(FREES.with(Cell::get), 1);
            assert_eq!(reached.is_ok(), mode == 0);
            assert_eq!(
                interpreter
                    .namespaces()
                    .command_in(destination, b"p")
                    .is_some(),
                mode == 0
            );
            assert_eq!(interpreter.native_execution_refusal().is_some(), mode != 0);
            assert_eq!(obj::bytes_of(interpreter.result_ptr()), b"REACHED COPY");
            if mode == 3 {
                assert_ne!(REBORN.with(Cell::get), destination);
            }
            ARMED.with(|value| value.set(false));
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}
#[test]
fn original_namespace_copy_propagates_reached_ordinary_store_failure() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Ordinary Array/scalar transport is independent of the retained Host cause.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        crate::counters::reset();
        {
            let mut interpreter = native(profile);
            let source = interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"Source");
            let destination = interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"Destination");
            let value = Owned::fresh(obj::new_string_bytes(b"SOURCE"));
            let previous = Owned::fresh(obj::new_string_bytes(b"DESTINATION"));
            interpreter
                .var_set_at(b"::Source::value", value.as_ptr(), 0)
                .unwrap();
            interpreter
                .var_set_elem_at(b"::Destination::value", b"member", previous.as_ptr(), 0)
                .unwrap();
            assert_eq!(
                interpreter.oo_clone_namespace(source, destination),
                Err(Code::Error)
            );
            assert!(interpreter.native_execution_refusal().is_none());
            assert_eq!(
                crate::vars::get_elem_at(
                    &interpreter.frames.borrow(),
                    &interpreter.namespaces(),
                    b"::Destination::value",
                    b"member",
                    0
                ),
                Some(previous.as_ptr())
            );
            assert_eq!(
                crate::vars::get_at(
                    &interpreter.frames.borrow(),
                    &interpreter.namespaces(),
                    b"::Source::value",
                    0
                ),
                Some(value.as_ptr())
            );
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}
