// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual callback boundaries during OO command publication.

use super::*;
use crate::interp::{native_operation_currency::NativeOperationScope, ObjCommand, WeakInterp};
use std::{cell::Cell, ffi::c_void, os::raw::c_int};

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static WORKERS: Cell<usize> = const { Cell::new(0) };
    static COPY_UPDATES: Cell<usize> = const { Cell::new(0) };
    static COPY_CALLS: RefCell<Option<Rc<Cell<usize>>>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct Observations {
    calls: Rc<Cell<usize>>,
    recreated: Rc<Cell<Option<NsId>>>,
    allocated: Rc<Cell<Option<NsId>>>,
}
struct Retirement {
    interpreter: WeakInterp,
    observations: Observations,
    mode: u8,
    namespace: Option<NsId>,
    name: &'static [u8],
}

unsafe extern "C" fn command(
    _: *mut c_void,
    _: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    0
}
unsafe extern "C" fn marker(
    _: *mut c_void,
    _: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    WORKERS.with(|workers| workers.set(workers.get() + 1));
    0
}
unsafe extern "C" fn retire(data: *mut c_void) {
    // SAFETY: the owned runtime shim transfers its one boxed callback record.
    let state = unsafe { Box::from_raw(data.cast::<Retirement>()) };
    state
        .observations
        .calls
        .set(state.observations.calls.get() + 1);
    let mut interpreter = state.interpreter.upgrade().unwrap();
    assert!(interpreter.oo.try_borrow_mut().is_ok());
    let allocated = {
        let oo = interpreter.oo.borrow();
        oo.names
            .iter()
            .find(|(_, name)| name.as_slice() == state.name)
            .and_then(|(id, _)| oo.objects.get(id).map(|owner| owner.var_ns))
    };
    state.observations.allocated.set(allocated);
    match state.mode {
        1 => {
            interpreter.refuse_host_command("first original OO publication callback");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("oo-publication".to_owned(), "1.0".to_owned()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            let namespace = state.namespace.unwrap();
            let spelling = interpreter.namespaces().qualified_name(namespace);
            interpreter.delete_namespace_by_id(namespace);
            let recreated = interpreter.ensure_namespace(&spelling);
            assert_ne!(recreated, namespace);
            state.observations.recreated.set(Some(recreated));
        }
        _ => {}
    }
}
fn install_old(
    interpreter: &mut Interp,
    namespace: NsId,
    tail: &[u8],
    mode: u8,
    retired_namespace: Option<NsId>,
    name: &'static [u8],
) -> Observations {
    let observations = Observations {
        calls: Rc::new(Cell::new(0)),
        recreated: Rc::new(Cell::new(None)),
        allocated: Rc::new(Cell::new(None)),
    };
    let state = Box::new(Retirement {
        interpreter: interpreter.downgrade(),
        observations: observations.clone(),
        mode,
        namespace: retired_namespace,
        name,
    });
    let command =
        ObjCommand::new_owned_runtime_shim(command, Box::into_raw(state).cast(), Some(retire));
    interpreter.bind_command_replacement(namespace, tail, Command::ObjCmd(Rc::new(command)));
    observations
}
fn native(version: &str) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert!(interpreter
        .create_obj_command(
            b"publicationMarker",
            ObjCommand::new_owned_runtime_shim(marker, core::ptr::null_mut(), None)
        )
        .is_some());
    assert_eq!(interpreter.eval_str(b"oo::class create PublicationClass {constructor {} {publicationMarker}}; oo::object create PublicationObject; oo::objdefine PublicationObject method <cloned> args {publicationMarker}"), Code::Ok);
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    WORKERS.with(|workers| workers.set(0));
    COPY_UPDATES.with(|updates| updates.set(0));
    COPY_CALLS.with(|calls| *calls.borrow_mut() = None);
    interpreter
}
fn assert_first(interpreter: &Interp, mode: u8) {
    match mode {
        1 => {
            let Some(tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first)) =
                interpreter.native_execution_refusal()
            else {
                panic!("original Host cause missing")
            };
            assert_eq!(first.reason, "first original OO publication callback");
        }
        2 => assert_eq!(
            interpreter.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                ),
            ))
        ),
        _ => assert!(interpreter.native_execution_refusal().is_none()),
    }
}

#[test]
fn private_dispatcher_retirement_stops_the_next_binding_and_preserves_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real owned-shim delete callbacks; no external Tcl callback timing claim.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..3 {
            let mut interpreter = native(version);
            let object = interpreter.oo_resolve_object(b"PublicationObject");
            let namespace = interpreter.oo.borrow().objects[&object].var_ns;
            let later = interpreter
                .namespaces()
                .command_generation(namespace, b"myclass");
            let observed = install_old(
                &mut interpreter,
                namespace,
                b"my",
                mode,
                None,
                b"::PublicationObject",
            );
            interpreter.set_result_bytes(b"PRIOR");
            let prior = obj::Owned::retain(interpreter.get_obj_result());
            let result = interpreter.oo_register_my(object);
            assert_eq!(observed.calls.get(), 1);
            if mode == 0 {
                assert!(result.is_ok());
                assert!(
                    matches!(interpreter.namespaces().command_in(namespace, b"my"), Some(Command::OoMy(id)) if id == object)
                );
                assert!(
                    matches!(interpreter.namespaces().command_in(namespace, b"myclass"), Some(Command::OoMyClass(id)) if id == object)
                );
                assert_ne!(
                    interpreter
                        .namespaces()
                        .command_generation(namespace, b"myclass"),
                    later
                );
            } else {
                let failure = result.err().unwrap();
                let first = interpreter.native_execution_refusal().unwrap();
                assert_eq!(failure.native_execution_refusal(), Some(&first));
                assert_eq!(
                    interpreter
                        .namespaces()
                        .command_generation(namespace, b"myclass"),
                    later
                );
            }
            assert_first(&interpreter, mode);
            assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            OWNER.with(|owner| *owner.borrow_mut() = None);
        }
    }
}

#[test]
fn private_dispatcher_refuses_retired_namespace_without_adopting_recreated_holder() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = native("tcl9.1");
    let object = interpreter.oo_resolve_object(b"PublicationObject");
    let namespace = interpreter.oo.borrow().objects[&object].var_ns;
    let original = interpreter.native_namespace_token_at(namespace).unwrap();
    let observed = install_old(
        &mut interpreter,
        namespace,
        b"my",
        3,
        Some(namespace),
        b"::PublicationObject",
    );
    assert!(interpreter.oo_register_my(object).is_err());
    assert_eq!(observed.calls.get(), 1);
    assert_eq!(
        original.lifecycle(),
        tcl_syntax::native_namespace_name::NativeNamespaceLifecycle::Dead
    );
    let fresh = observed.recreated.get().unwrap();
    assert_ne!(fresh, namespace);
    assert!(interpreter.namespaces().command_in(fresh, b"my").is_none());
    assert!(interpreter
        .namespaces()
        .command_in(fresh, b"myclass")
        .is_none());
    assert!(interpreter.native_execution_refusal().is_some());
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    OWNER.with(|owner| *owner.borrow_mut() = None);
}

#[test]
fn public_replacement_refusal_stops_creation_workers_and_retires_partial_objects() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Select a genuine empty publication slot before installing the actual old
    // shim. This isolates the continuation, not collision-pretest parity.
    for class in [false, true] {
        for mode in 0..3 {
            let mut interpreter = native("tcl9.1");
            let publication = interpreter.oo_publication(b"CreatedPublication").unwrap();
            let observed = install_old(
                &mut interpreter,
                publication.namespace,
                &publication.simple,
                mode,
                None,
                b"::CreatedPublication",
            );
            interpreter.set_result_bytes(b"PRIOR");
            let prior = obj::Owned::retain(interpreter.get_obj_result());
            let scope = NativeOperationScope::enter(&interpreter).unwrap();
            let code = if class {
                let script = obj::Owned::fresh(obj::new_string_bytes(b"publicationMarker"));
                interpreter.oo_make_class(publication, Some(script.as_ptr()))
            } else {
                let class = interpreter.oo_resolve_object(b"PublicationClass");
                interpreter.oo_new_ns(
                    class,
                    Some(publication),
                    None,
                    &[],
                    MethodInvocation::internal(),
                )
            };
            assert_eq!(observed.calls.get(), 1);
            if mode == 0 {
                assert_eq!(code, Code::Ok);
                assert_eq!(WORKERS.with(Cell::get), 1);
                assert!(interpreter
                    .namespaces()
                    .namespace_is_live(observed.allocated.get().unwrap()));
            } else {
                assert_eq!(code, Code::Error);
                assert_eq!(WORKERS.with(Cell::get), 0);
                assert!(!interpreter
                    .namespaces()
                    .namespace_is_live(observed.allocated.get().unwrap()));
                assert!(!interpreter
                    .oo
                    .borrow()
                    .names
                    .values()
                    .any(|name| name == b"::CreatedPublication"));
                assert!(interpreter
                    .namespaces()
                    .command_in(GLOBAL, b"CreatedPublication")
                    .is_none());
                assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
            }
            assert_first(&interpreter, mode);
            drop(scope);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            OWNER.with(|owner| *owner.borrow_mut() = None);
        }
    }
}

extern "C" fn namespace_updater(original: *mut TclObj) {
    COPY_UPDATES.with(|updates| updates.set(updates.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    let observed = install_old(
        &mut interpreter,
        GLOBAL,
        b"CopiedPublication",
        MODE.with(Cell::get),
        None,
        b"::CopiedPublication",
    );
    COPY_CALLS.with(|calls| *calls.borrow_mut() = Some(observed.calls));
    unsafe { obj::set_native_updater_string_rep(original, b"::CopiedPublicationNamespace", false) };
}
static NAMESPACE: obj::TclObjType = obj::TclObjType {
    name: c"originalOOCopyNamespace".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(namespace_updater),
    set_from_any_proc: None,
};

#[test]
fn copy_replacement_callback_stops_cloned_worker_after_original_namespace_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in 0..3 {
        let mut interpreter = native("tcl9.1");
        MODE.with(|selected| selected.set(mode));
        let head = obj::Owned::fresh(obj::new_string_bytes(b"oo::copy"));
        let source = obj::Owned::fresh(obj::new_string_bytes(b"PublicationObject"));
        let destination = obj::Owned::fresh(obj::new_string_bytes(b"CopiedPublication"));
        let namespace = obj::Owned::fresh(obj::alloc_typed(&NAMESPACE, 0));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = obj::Owned::retain(interpreter.get_obj_result());
        let code = oo_copy_cmd(
            &mut interpreter,
            &[
                head.as_ptr(),
                source.as_ptr(),
                destination.as_ptr(),
                namespace.as_ptr(),
            ],
        );
        assert_eq!(COPY_UPDATES.with(Cell::get), 1);
        assert_eq!(
            COPY_CALLS.with(|calls| calls.borrow().as_ref().unwrap().get()),
            1
        );
        assert_eq!(
            obj::bytes_of(namespace.as_ptr()),
            b"::CopiedPublicationNamespace"
        );
        if mode == 0 {
            assert_eq!(code, Code::Ok);
            assert_eq!(WORKERS.with(Cell::get), 1);
        } else {
            assert_eq!(code, Code::Error);
            assert_eq!(WORKERS.with(Cell::get), 0);
            assert!(!interpreter
                .oo
                .borrow()
                .names
                .values()
                .any(|name| name == b"::CopiedPublication"));
            assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
        }
        assert_first(&interpreter, mode);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert_eq!(unsafe { (*namespace.as_ptr()).ref_count }, 1);
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}

#[test]
fn foundation_installation_does_not_continue_after_prior_typed_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = native("tcl9.1");
    let original = {
        let oo = interpreter.oo.borrow();
        (
            oo.objects.len(),
            oo.classes.len(),
            oo.counter,
            oo.next_id,
            oo.define_name.as_ref().unwrap().as_ptr(),
        )
    };
    interpreter.set_result_bytes(b"PRIOR");
    let prior = obj::Owned::retain(interpreter.get_obj_result());
    interpreter.refuse_host_command("first original OO publication callback");
    install(&mut interpreter);
    let oo = interpreter.oo.borrow();
    assert_eq!(
        (
            oo.objects.len(),
            oo.classes.len(),
            oo.counter,
            oo.next_id,
            oo.define_name.as_ref().unwrap().as_ptr()
        ),
        original
    );
    drop(oo);
    assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
    assert_first(&interpreter, 1);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    OWNER.with(|owner| *owner.borrow_mut() = None);
}
