// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software callback boundaries of the actual selected command cache worker.
use super::*;
use crate::interp::{Code, WeakInterp};
use crate::namespace::GLOBAL;
use std::cell::{Cell, RefCell};

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static NAMESPACE: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static NAMESPACE_REFS: Cell<obj::TclSize> = const { Cell::new(0) };
    static FREE_NAMESPACE_REFS: Cell<obj::TclSize> = const { Cell::new(0) };
    static DELETES: RefCell<Option<std::rc::Rc<Cell<usize>>>> = const { RefCell::new(None) };
    static DELETES_SEEN: Cell<usize> = const { Cell::new(0) };
    static REACHED: RefCell<Option<obj::Owned>> = const { RefCell::new(None) };
}
fn worker(_interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    Code::Ok
}
fn native(profile: &str) -> Interp {
    let mut interpreter = Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    interpreter.register_builtin(b"cacheSelected", worker);
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    FREES.with(|count| count.set(0));
    NAMESPACE.with(|pointer| pointer.set(std::ptr::null_mut()));
    REACHED.with(|owner| *owner.borrow_mut() = None);
    interpreter
}
extern "C" fn retire(_original: *mut TclObj) {
    FREES.with(|count| count.set(count.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    // These are the actual owners the reached lookup has released before free.
    drop(interpreter.namespaces_mut());
    NAMESPACE.with(|pointer| {
        if !pointer.get().is_null() {
            FREE_NAMESPACE_REFS.with(|refs| refs.set(unsafe { (*pointer.get()).ref_count }));
        }
    });
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original command cache retirement");
        }
        2 => {
            let actual = interpreter.runtime_context();
            let mut changed = actual.clone();
            changed
                .packages
                .push(("command-cache-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&actual).unwrap();
        }
        3 => {
            interpreter.register_builtin(b"cacheSelected", worker);
        }
        4 => {
            interpreter.rename_command(b"cacheSelected", b"");
            DELETES.with(|count| {
                DELETES_SEEN.with(|seen| seen.set(count.borrow().as_ref().unwrap().get()))
            });
        }
        _ => {}
    }
    interpreter.set_result_bytes(b"REACHED COMMAND CACHE RETIREMENT");
    REACHED
        .with(|owner| *owner.borrow_mut() = Some(obj::Owned::retain(interpreter.get_obj_result())));
}
static PRIMARY: obj::TclObjType = obj::TclObjType {
    name: c"originalCommandCacheRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn original(bytes: &[u8]) -> obj::Owned {
    let original = obj::Owned::fresh(obj::new_string_bytes(bytes));
    obj::change_type(original.as_ptr(), &PRIMARY, 0);
    original
}
fn finish(interpreter: &Interp, original: &obj::Owned, mode: u8) {
    assert_eq!(FREES.with(Cell::get), 1);
    assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    REACHED.with(|owner| {
        assert_eq!(
            interpreter.get_obj_result(),
            owner.borrow().as_ref().unwrap().as_ptr()
        )
    });
    match mode {
        1 => assert!(matches!(interpreter.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first))
                if first.reason=="first original command cache retirement")),
        2 => assert!(interpreter.native_execution_refusal().is_some()),
        _ => assert!(interpreter.native_execution_refusal().is_none()),
    }
    OWNER.with(|owner| *owner.borrow_mut() = None);
    REACHED.with(|owner| *owner.borrow_mut() = None);
}

#[test]
fn original_cache_retirement_checks_actual_currency_and_binding_before_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Source-only owner control; no native header/refcount equivalence claim.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for mode in 0..4 {
            let mut interpreter = native(profile);
            MODE.with(|current| current.set(mode));
            let original = original(b"cacheSelected");
            let result = interpreter.resolve_original_command(original.as_ptr());
            if mode == 0 {
                assert!(matches!(result, Ok(Some((_, Some(_))))));
                if profile == "jim" {
                    assert!(obj::with_jim_command_cache(original.as_ptr(), |_| ()).is_some());
                } else {
                    assert!(obj::native_command_name_cache(original.as_ptr()).is_some());
                }
            } else {
                assert!(result.is_err());
                assert!(obj::obj_type_ptr(original.as_ptr()).is_null());
                assert!(obj::native_command_name_cache(original.as_ptr()).is_none());
                assert!(obj::with_jim_command_cache(original.as_ptr(), |_| ()).is_none());
            }
            assert_eq!(obj::bytes_of(original.as_ptr()), b"cacheSelected");
            finish(&interpreter, &original, mode);
        }
    }
}

#[test]
fn original_unresolved_c8_cache_stops_after_reached_retirement_and_c9_keeps_primary() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..3 {
            let mut interpreter = native(profile);
            MODE.with(|current| current.set(mode));
            let original = original(b"missingCacheSelected");
            let result = interpreter.resolve_original_command(original.as_ptr());
            if profile.starts_with("tcl9") {
                assert!(matches!(result, Ok(None)));
                assert_eq!(FREES.with(Cell::get), 0);
                assert!(std::ptr::eq(obj::obj_type_ptr(original.as_ptr()), &PRIMARY));
                assert!(interpreter.native_execution_refusal().is_none());
                // Complete the unchanged native primary's actual mandatory retirement.
                MODE.with(|current| current.set(0));
                obj::discard_native_internal_representation(original.as_ptr()).unwrap();
                finish(&interpreter, &original, 0);
            } else {
                if mode == 0 {
                    assert!(matches!(result, Ok(None)));
                    assert!(
                        obj::native_command_name_unresolved_version(original.as_ptr()).is_some()
                    );
                } else {
                    assert!(result.is_err());
                    assert!(obj::obj_type_ptr(original.as_ptr()).is_null());
                    assert!(
                        obj::native_command_name_unresolved_version(original.as_ptr()).is_none()
                    );
                }
                assert_eq!(obj::bytes_of(original.as_ptr()), b"missingCacheSelected");
                finish(&interpreter, &original, mode);
            }
        }
    }
}

#[test]
fn original_jim_cache_namespace_reference_starts_after_actual_primary_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in 0..3 {
        let mut interpreter = native("jim");
        MODE.with(|current| current.set(mode));
        let namespace = interpreter
            .namespaces()
            .jim_namespace_original_owner(GLOBAL)
            .unwrap();
        NAMESPACE.with(|pointer| pointer.set(namespace.as_ptr()));
        let references = unsafe { (*namespace.as_ptr()).ref_count };
        NAMESPACE_REFS.with(|refs| refs.set(references));
        let original = original(b"cacheSelected");
        let result = interpreter.resolve_original_command(original.as_ptr());
        assert_eq!(
            FREE_NAMESPACE_REFS.with(Cell::get),
            NAMESPACE_REFS.with(Cell::get)
        );
        if mode == 0 {
            assert!(matches!(result, Ok(Some(_))));
            assert_eq!(unsafe { (*namespace.as_ptr()).ref_count }, references + 1);
            assert!(obj::with_jim_command_cache(original.as_ptr(), |cache| cache
                .namespace
                .as_ptr()
                == namespace.as_ptr())
            .unwrap());
        } else {
            assert!(result.is_err());
            assert_eq!(unsafe { (*namespace.as_ptr()).ref_count }, references);
            assert!(obj::with_jim_command_cache(original.as_ptr(), |_| ()).is_none());
        }
        finish(&interpreter, &original, mode);
        NAMESPACE.with(|pointer| pointer.set(std::ptr::null_mut()));
    }
}

unsafe extern "C" fn owned_invoke(
    _data: *mut std::ffi::c_void,
    _interpreter: *mut Interp,
    _argc: std::ffi::c_int,
    _argv: *const *mut TclObj,
) -> std::ffi::c_int {
    0
}
unsafe extern "C" fn owned_delete(data: *mut std::ffi::c_void) {
    let count = unsafe { Box::from_raw(data.cast::<std::rc::Rc<Cell<usize>>>()) };
    count.set(count.get() + 1);
}
#[test]
fn original_cache_preparation_does_not_keep_idle_owned_worker_alive_during_free() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = native(profile);
        let deleted = std::rc::Rc::new(Cell::new(0));
        let data = Box::into_raw(Box::new(std::rc::Rc::clone(&deleted))).cast();
        interpreter.bind_command_replacement(
            GLOBAL,
            b"cacheSelected",
            Command::ObjCmd(std::rc::Rc::new(
                crate::interp::ObjCommand::new_owned_runtime_shim(
                    owned_invoke,
                    data,
                    Some(owned_delete),
                ),
            )),
        );
        let generation = interpreter
            .namespaces()
            .command_generation(GLOBAL, b"cacheSelected")
            .unwrap();
        assert!(interpreter
            .namespaces()
            .native_command_at_node(generation)
            .is_some());
        DELETES.with(|count| *count.borrow_mut() = Some(std::rc::Rc::clone(&deleted)));
        MODE.with(|mode| mode.set(4));
        DELETES_SEEN.with(|seen| seen.set(0));
        let original = original(b"cacheSelected");
        let reached = interpreter.resolve_original_command(original.as_ptr());
        assert!(reached.is_err());
        assert_eq!(DELETES_SEEN.with(Cell::get), 1);
        assert_eq!(deleted.get(), 1);
        assert!(interpreter
            .namespaces()
            .native_command_at_node(generation)
            .is_none());
        assert!(obj::obj_type_ptr(original.as_ptr()).is_null());
        assert!(obj::native_command_name_cache(original.as_ptr()).is_none());
        assert!(obj::with_jim_command_cache(original.as_ptr(), |_| ()).is_none());
        finish(&interpreter, &original, 4);
        DELETES.with(|count| *count.borrow_mut() = None);
    }
}
