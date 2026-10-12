// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original OO name objects, imported tokens and checked compatibility aliases.

use super::*;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;

thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static UPDATES: Cell<usize> = const { Cell::new(0) };
    static PAYLOAD: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}
fn native(profile: &str) -> Interp {
    let interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    UPDATES.with(|count| count.set(0));
    interpreter
}
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
extern "C" fn update(original: *mut TclObj) {
    UPDATES.with(|count| count.set(count.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original OO resolver getter");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("original-resolver-context".to_owned(), "1".to_owned()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            assert_eq!(
                interpreter.eval_str(b"rename ::ResolverClass {};oo::class create ::ResolverClass"),
                Code::Ok
            );
        }
        _ => {}
    }
    let bytes = PAYLOAD.with(|payload| payload.borrow().clone());
    unsafe { obj::set_native_updater_string_rep(original, &bytes, false) };
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalOoResolverArgument".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn lazy(bytes: &[u8], mode: u8) -> obj::Owned {
    MODE.with(|selected| selected.set(mode));
    PAYLOAD.with(|payload| *payload.borrow_mut() = bytes.to_vec());
    obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0))
}

#[test]
fn original_oo_lookup_keeps_stringless_command_and_import_token_roles() {
    // naming.namespace.origin-original-cmdname-residency
    // docs/design/analysis/name-resolution-proofs/namespace-origin-original-cmdname-residency.md
    // Imported token traversal is inspected C source; OO identity assertions are software controls.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        assert_eq!(interpreter.eval_str(b"namespace eval src {};oo::class create ::src::C;namespace eval src {namespace export C};namespace eval dst {namespace import ::src::C}"), Code::Ok);
        let original = word(b"::dst::C");
        let selected = interpreter
            .oo_resolve_original_object(original.as_ptr())
            .unwrap();
        assert_ne!(selected, OoId(0));
        let cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
        obj::invalidate_string(original.as_ptr());
        let references = unsafe { (*original.as_ptr()).ref_count };
        assert_eq!(
            interpreter.eval_str(b"rename ::src::C ::src::D;oo::class create ::src::C"),
            Code::Ok
        );
        assert_eq!(
            interpreter
                .oo_resolve_original_object(original.as_ptr())
                .unwrap(),
            selected
        );
        assert_eq!(
            interpreter
                .native_namespace_origin(original.as_ptr())
                .unwrap(),
            Some(b"::src::D".to_vec())
        );
        assert_eq!(
            obj::native_command_name_cache(original.as_ptr())
                .unwrap()
                .token,
            cache.token
        );
        assert!(!obj::has_string_rep(original.as_ptr()));
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, references);
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(
            interpreter.eval_str(b"rename ::dst::C {};proc ::dst::C {} {return REPLACED}"),
            Code::Ok
        );
        let prior = word(b"ACTUAL PRIOR RESULT");
        unsafe { interpreter.set_obj_result(prior.as_ptr()) };
        let refusal = interpreter
            .oo_resolve_original_object(original.as_ptr())
            .unwrap_err();
        assert!(refusal.native_execution_refusal().is_some());
        assert_eq!(
            interpreter.native_execution_refusal().as_ref(),
            refusal.native_execution_refusal()
        );
        assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
        assert!(!obj::has_string_rep(original.as_ptr()));
    }
}

#[test]
fn original_oo_getter_keeps_full_cause_before_command_cache_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [1, 2] {
            let mut interpreter = native(profile);
            assert_eq!(
                interpreter.eval_str(b"oo::class create ResolverClass"),
                Code::Ok
            );
            let original = lazy(b"ResolverClass", mode);
            let prior = word(b"ACTUAL PRIOR RESULT");
            unsafe { interpreter.set_obj_result(prior.as_ptr()) };
            let refusal = interpreter
                .oo_resolve_original_object(original.as_ptr())
                .unwrap_err();
            let first = interpreter.native_execution_refusal().unwrap();
            assert_eq!(refusal.native_execution_refusal(), Some(&first));
            if mode == 1 {
                let NativeExecutionError::HostCommandRefusal(payload) = first else {
                    panic!("complete original Host cause")
                };
                assert_eq!(payload.reason, "first original OO resolver getter");
            }
            assert_eq!(UPDATES.with(Cell::get), 1);
            assert!(obj::has_string_rep(original.as_ptr()));
            assert!(obj::native_command_name_cache(original.as_ptr()).is_none());
            assert_eq!(
                unsafe { (*original.as_ptr()).type_ptr },
                &ORIGINAL as *const obj::TclObjType
            );
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
            assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_oo_getter_rebinding_selects_the_actual_post_getter_command() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Ordinary table mutation is independent of execution-context currency.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        assert_eq!(
            interpreter.eval_str(b"oo::class create ResolverClass"),
            Code::Ok
        );
        let before = interpreter
            .oo_resolve_object_bytes(b"ResolverClass")
            .unwrap();
        let original = lazy(b"ResolverClass", 3);
        let selected = interpreter
            .oo_resolve_original_object(original.as_ptr())
            .unwrap();
        assert_ne!(selected, before);
        let actual = interpreter
            .oo_resolve_object_bytes(b"ResolverClass")
            .unwrap();
        assert_eq!(selected, actual);
        assert!(interpreter.oo.borrow().objects.contains_key(&selected));
        assert!(!interpreter.oo.borrow().objects.contains_key(&before));
        assert_eq!(UPDATES.with(Cell::get), 1);
        let cache = obj::native_command_name_cache(original.as_ptr()).unwrap();
        let token = interpreter
            .namespaces()
            .resolve_generation(GLOBAL, b"ResolverClass")
            .unwrap();
        assert_eq!(cache.token, token);
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
    }
}

#[test]
fn original_import_projection_never_substitutes_a_recreated_source_spelling() {
    // naming.namespace.origin-original-cmdname-residency
    // docs/design/analysis/name-resolution-proofs/namespace-origin-original-cmdname-residency.md
    // Retained Rust import metadata is a software negative, not a native dangling Command pointer.
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        assert_eq!(interpreter.eval_str(b"namespace eval src {};oo::class create ::src::C;namespace eval src {namespace export C};namespace eval dst {namespace import ::src::C}"), Code::Ok);
        let captured = interpreter
            .resolve_dispatchable(GLOBAL, b"::dst::C")
            .unwrap();
        assert!(matches!(&captured, Command::Imported { .. }));
        assert_eq!(
            interpreter.eval_str(b"rename ::src::C {};oo::class create ::src::C"),
            Code::Ok
        );
        interpreter.bind_command_replacement(GLOBAL, b"retainedImport", captured);
        let generation = interpreter
            .namespaces()
            .resolve_generation(GLOBAL, b"retainedImport")
            .unwrap();
        assert!(interpreter
            .original_import_origin_command(generation)
            .is_none());
        let id = interpreter
            .find_command_id(GLOBAL, b"retainedImport")
            .unwrap();
        assert!(interpreter.imported_source_id(id).is_none());
        let original = word(b"retainedImport");
        assert_eq!(
            interpreter
                .oo_resolve_original_object(original.as_ptr())
                .unwrap(),
            OoId(0)
        );
        assert_ne!(
            interpreter.oo_resolve_object_bytes(b"::src::C").unwrap(),
            OoId(0)
        );
        assert!(interpreter.native_execution_refusal().is_none());
    }
}

#[test]
fn original_jim_alias_lookup_uses_the_stored_list_and_keeps_absent_context_terminal() {
    // naming.alias.original-object-prefix-capture-query-invocation-lifetime
    // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
    // Jim alias storage is the actual selected owner; this authored OO carrier is not native Jim TclOO availability.
    for retire in [false, true] {
        let mut interpreter = native("jim");
        // A distinct authored record does not reuse the unavailable stock root's
        // registry identity. It supplies no Jim-native TclOO invocation purpose.
        let class = interpreter.oo.borrow().object_root.unwrap();
        let creation = interpreter.oo_next_id();
        let target = {
            let mut state = interpreter.oo.borrow_mut();
            let target = state.allocate(b"::portableTarget".to_vec());
            state
                .objects
                .insert(target, Object::new(class, GLOBAL, creation));
            target
        };
        interpreter.bind_command_replacement(GLOBAL, b"portableTarget", Command::OoObject(target));
        let member = lazy(b"portableTarget", 0);
        let list = Rc::new(crate::interp::NativeAliasHeader::new(obj::Owned::fresh(
            interpreter.new_list_object(&[member.as_ptr()]),
        )));
        interpreter
            .install_alias_objects(
                b"originalAlias",
                Rc::new(crate::interp::NativeAliasHeader::new(word(b"originalAlias"))),
                Rc::clone(&list),
            )
            .unwrap();
        assert_eq!(UPDATES.with(Cell::get), 0);
        assert!(!obj::has_string_rep(member.as_ptr()));
        let original = word(b"originalAlias");
        let prior = word(b"ACTUAL PRIOR RESULT");
        unsafe { interpreter.set_obj_result(prior.as_ptr()) };
        if retire {
            interpreter.native_jim_object_context().unwrap().retire();
            let refusal = interpreter
                .oo_resolve_original_object(original.as_ptr())
                .unwrap_err();
            assert_eq!(
                interpreter.native_execution_refusal().as_ref(),
                refusal.native_execution_refusal()
            );
            assert!(refusal.native_execution_refusal().is_some());
            assert_eq!(UPDATES.with(Cell::get), 0);
            assert!(!obj::has_string_rep(member.as_ptr()));
        } else {
            assert_eq!(
                interpreter
                    .oo_resolve_original_object(original.as_ptr())
                    .unwrap(),
                target
            );
            assert_eq!(UPDATES.with(Cell::get), 1);
            let Command::Alias {
                original_prefix, ..
            } = interpreter
                .resolve_dispatchable(GLOBAL, b"originalAlias")
                .unwrap()
            else {
                panic!("authentic retained alias")
            };
            assert_eq!(original_prefix.as_ptr(), list.as_ptr());
            assert_eq!(unsafe { (*list.as_ptr()).ref_count }, 1);
        }
        assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
