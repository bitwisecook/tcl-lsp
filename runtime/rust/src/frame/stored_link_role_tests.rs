// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software observations of actual stored roles, distinct from Native chronology.
use super::*;
use crate::{
    interp::{Interp, WeakInterp},
    namespace::GLOBAL,
    obj::{self, Owned, TclObj},
};
use std::cell::Cell as Counter;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Counter<u8> = const { Counter::new(0) };
    static GENERIC: Counter<bool> = const { Counter::new(false) };
    static CALLS: Counter<usize> = const { Counter::new(0) };
    static REACHED: Counter<*mut TclObj> = const { Counter::new(std::ptr::null_mut()) };
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
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
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn arm(interpreter: &Interp, mode: u8, generic: bool) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|selected| selected.set(mode));
    GENERIC.with(|selected| selected.set(generic));
    CALLS.with(|calls| calls.set(0));
    REACHED.with(|result| result.set(std::ptr::null_mut()));
    Cleanup
}
extern "C" fn retire(original: *mut TclObj) {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    assert_eq!(unsafe { (*original).ref_count }, 0);
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.original_variable_storage_borrows_are_available());
    let generic = GENERIC.with(Counter::get);
    {
        let namespaces = interpreter.namespaces();
        let target = namespaces
            .var_table(GLOBAL)
            .contents(b"next")
            .unwrap()
            .borrow();
        assert_eq!(target.native_alias_refs, usize::from(generic));
    }
    {
        let frames = interpreter.frames.borrow();
        let local = frames.table(1).unwrap().get(b"alias").unwrap();
        let Var::Link(link) = &*local else {
            panic!("actual link remains at the selected callback stage");
        };
        assert_eq!(
            link.name,
            if generic {
                b"next".as_slice()
            } else {
                b"old".as_slice()
            }
        );
        assert_eq!(link.check_stored_roles().is_ok(), generic);
    }
    let mode = MODE.with(Counter::get);
    if mode == 3 {
        let retired = {
            let mut frames = interpreter.frames.borrow_mut();
            frames
                .table_mut(1)
                .unwrap()
                .publish_remove_after_default(b"alias")
                .1
        };
        drop(retired);
        let replacement = text(b"REPLACEMENT CELL");
        let retired = {
            let mut frames = interpreter.frames.borrow_mut();
            frames
                .table_mut(1)
                .unwrap()
                .publish_scalar(b"alias", replacement.as_ptr())
                .unwrap()
        };
        drop(retired);
    }
    interpreter.set_result_bytes(b"REACHED ROLE RETIREMENT");
    REACHED.with(|result| result.set(interpreter.get_obj_result()));
    if mode == 1 {
        interpreter.refuse_host_command("first stored alias retirement");
    }
    if mode == 2 {
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed
            .packages
            .push(("stored-role-aba".into(), "1.0".into()));
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
    }
}
static OLD: obj::TclObjType = obj::TclObjType {
    name: c"storedAliasRoleRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn assert_first(interpreter: &Interp, mode: u8) {
    let first = interpreter
        .native_execution_refusal()
        .expect("original full refusal");
    if mode == 1 {
        assert!(
            matches!(first, NativeExecutionError::HostCommandRefusal(first) if first.reason == "first stored alias retirement")
        );
    } else if mode == 2 {
        assert_eq!(
            first,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                )
            )
        );
    } else {
        assert!(matches!(
            first,
            NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(_)
            )
        ));
    }
}

#[test]
fn original_c_alias_retirement_keeps_metadata_without_postponing_its_native_role() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The C9 object-key callback observes the genuine key role, not a C8 string-key surrogate.
    for profile in ["tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 3] {
            let mut interpreter = native(profile);
            interpreter.frames.borrow_mut().push(GLOBAL);
            let old = text(b"old");
            let local = text(b"alias");
            let next = text(b"next");
            assert_eq!(
                interpreter.link_original_c_upvar_objects(old.as_ptr(), 0, local.as_ptr()),
                crate::interp::Code::Ok
            );
            let retained = {
                let frames = interpreter.frames.borrow();
                let local = frames.table(1).unwrap().get(b"alias").unwrap();
                let Var::Link(link) = &*local else {
                    panic!("original alias");
                };
                link.clone()
            };
            assert_eq!(
                retained
                    .native_scalar_entry
                    .as_ref()
                    .unwrap()
                    .contents
                    .borrow()
                    .native_alias_refs,
                1
            );
            // The actual object hash retains this original name. Change only its
            // primary to observe that real last-owner retirement boundary.
            CALLS.with(|calls| calls.set(0));
            assert!(interpreter
                .namespaces()
                .var_table(GLOBAL)
                .original_native_key_is(b"old", old.as_ptr()));
            obj::change_type(old.as_ptr(), &OLD, 0);
            let allocation = obj::NativeObjectLifetime::retain(old.as_ptr());
            drop(old);
            assert!(!interpreter.var_unset(b"old"));
            assert_eq!(CALLS.with(Counter::get), 0);
            let _cleanup = arm(&interpreter, mode, false);
            let code = interpreter.link_original_c_upvar_objects(next.as_ptr(), 0, local.as_ptr());
            assert_eq!(
                code == crate::interp::Code::Ok,
                mode == 0,
                "{profile}/{mode}"
            );
            assert_eq!(CALLS.with(Counter::get), 1);
            assert!(obj::check_native_liveness(allocation.as_ptr()).is_err());
            assert!(retained.check_stored_roles().is_err());
            assert_eq!(
                retained
                    .native_scalar_entry
                    .as_ref()
                    .unwrap()
                    .contents
                    .borrow()
                    .native_alias_refs,
                0
            );
            assert_eq!(interpreter.get_obj_result(), REACHED.with(Counter::get));
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            let frames = interpreter.frames.borrow();
            let actual = frames.table(1).unwrap().get(b"alias").unwrap();
            if mode == 0 {
                assert!(
                    matches!(&*actual, Var::Link(link) if link.name == b"next" && link.check_stored_roles().is_ok())
                );
            } else {
                assert_first(&interpreter, mode);
                if mode == 3 {
                    assert!(
                        matches!(&*actual, Var::Scalar(value) if obj::bytes_of(*value) == b"REPLACEMENT CELL")
                    );
                } else {
                    assert!(matches!(&*actual, Var::Link(link) if link.name == b"old"));
                }
            }
        }
    }
}

#[test]
fn original_generic_link_publication_retires_old_scalar_after_actual_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Generic software publication keeps its existing adopt-before-retire order, independent of C upvar.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2] {
            let mut interpreter = native(profile);
            interpreter.frames.borrow_mut().push(GLOBAL);
            let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
            let old_allocation = obj::NativeObjectLifetime::retain(old.as_ptr());
            let old_parts = interpreter
                .frames
                .borrow_mut()
                .table_mut(1)
                .unwrap()
                .publish_scalar(b"alias", old.as_ptr())
                .unwrap();
            drop(old_parts);
            drop(old);
            let target = {
                let mut frames = interpreter.frames.borrow_mut();
                let mut ns = interpreter.namespaces_mut();
                let mut target =
                    crate::vars::link_target_at(&frames, &ns, b"next", None, 0).unwrap();
                crate::vars::prepare_upvar_target(&mut frames, &mut ns, &mut target).unwrap();
                target
            };
            let _cleanup = arm(&interpreter, mode, true);
            let code = interpreter.make_upvar(target, b"alias");
            assert_eq!(
                code == crate::interp::Code::Ok,
                mode == 0,
                "{profile}/{mode}"
            );
            assert_eq!(CALLS.with(Counter::get), 1);
            assert!(obj::check_native_liveness(old_allocation.as_ptr()).is_err());
            assert_eq!(interpreter.get_obj_result(), REACHED.with(Counter::get));
            let frames = interpreter.frames.borrow();
            assert!(
                matches!(frames.table(1).unwrap().get(b"alias").as_deref(), Some(Var::Link(link)) if link.name == b"next" && link.check_stored_roles().is_ok())
            );
            if mode == 0 {
                assert!(interpreter.native_execution_refusal().is_none());
            } else {
                assert_first(&interpreter, mode);
            }
        }
    }
}

#[test]
fn original_shared_link_metadata_transfers_one_role_and_real_removal_withdraws_it_once() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut target = VarTable::default();
    let role = target.retain_native_scalar_alias(b"target");
    let link = Link {
        original_jim_target: None,
        native_scalar_entry: Some(Rc::clone(&role)),
        native_element_entry: None,
        array_identity: None,
        array_cell: None,
        home: VarHome::Frame(0),
        name: b"target".to_vec(),
        elem: None,
    };
    let retained = link.clone();
    let mut local = VarTable::default();
    drop(local.publish_link_with_origin(b"local", link, FrameLinkOrigin::Ordinary));
    drop(local.publish_link_with_origin(b"local", retained.clone(), FrameLinkOrigin::Ordinary));
    assert_eq!(role.contents.borrow().native_alias_refs, 1);
    assert!(retained.check_stored_roles().is_ok());
    let removed = local.publish_remove_after_default(b"local").1;
    removed.finish(|| Ok::<(), ()>(())).unwrap();
    assert_eq!(role.contents.borrow().native_alias_refs, 0);
    assert!(retained.check_stored_roles().is_err());
    drop(retained);
    drop(role);
}

#[test]
fn original_c_same_target_noop_issues_no_new_alias_role_for_every_selected_profile() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(profile);
        interpreter.frames.borrow_mut().push(GLOBAL);
        let target = text(b"target");
        let local = text(b"alias");
        assert_eq!(
            interpreter.link_original_c_upvar_objects(target.as_ptr(), 0, local.as_ptr()),
            crate::interp::Code::Ok
        );
        let retained = {
            let frames = interpreter.frames.borrow();
            let local = frames.table(1).unwrap().get(b"alias").unwrap();
            let Var::Link(link) = &*local else {
                panic!("actual alias");
            };
            link.clone()
        };
        let role = retained.native_scalar_entry.as_ref().unwrap();
        assert_eq!(role.contents.borrow().native_alias_refs, 1);
        assert_eq!(
            interpreter.link_original_c_upvar_objects(target.as_ptr(), 0, local.as_ptr()),
            crate::interp::Code::Ok
        );
        assert_eq!(role.contents.borrow().native_alias_refs, 1);
        assert!(retained.check_stored_roles().is_ok());
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_compiled_alias_distinguishes_c84_target_reference_from_later_hash_only_role() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual installed compiled cells exercise the selected source recipe; software counters are not native measurements.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let interpreter = native(profile);
        let protocol = interpreter
            .native_invocation_dialect()
            .native_compiled_variable_protocol()
            .unwrap();
        interpreter.frames.borrow_mut().push(GLOBAL);
        interpreter
            .frames
            .borrow_mut()
            .install_formal_cells(&[b"target".to_vec()], protocol);
        let selected = interpreter
            .frames
            .borrow_mut()
            .select_original_compiled_alias(0, false)
            .unwrap();
        let incoming = Link {
            original_jim_target: None,
            native_scalar_entry: Some(selected),
            native_element_entry: None,
            array_identity: None,
            array_cell: None,
            home: VarHome::Frame(1),
            name: b"target".to_vec(),
            elem: None,
        };
        interpreter.frames.borrow_mut().push(GLOBAL);
        interpreter
            .frames
            .borrow_mut()
            .table_mut(2)
            .unwrap()
            .prepare_native_name_cell(b"alias", true);
        let operation =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
                .unwrap();
        let prepared = interpreter
            .frames
            .borrow_mut()
            .prepare_original_local_alias(b"alias", &incoming, false)
            .unwrap();
        prepared
            .finish(
                interpreter
                    .native_c_variable_name_protocol()
                    .unwrap()
                    .alias_publication_order(),
                || {
                    operation
                        .currency()
                        .ensure_current_or_refuse()
                        .map_err(tcl_cmd_core::CmdError::from_execution_refusal)
                },
                |error| {
                    tcl_cmd_core::CmdError::from_execution_refusal(
                        interpreter.clone().refuse_completion_value_access(error),
                    )
                },
            )
            .unwrap();
        let retained = {
            let frames = interpreter.frames.borrow();
            let actual = frames.table(2).unwrap().get(b"alias").unwrap();
            let Var::Link(link) = &*actual else {
                panic!("actual compiled target alias");
            };
            link.clone()
        };
        let role = retained.native_scalar_entry.as_ref().unwrap();
        assert_eq!(
            role.contents.borrow().native_alias_refs,
            usize::from(profile == "tcl8.4")
        );
        assert!(retained.check_stored_roles().is_ok());
        let retired = interpreter
            .frames
            .borrow_mut()
            .table_mut(2)
            .unwrap()
            .publish_remove_after_default(b"alias")
            .1;
        retired.finish(|| Ok::<(), ()>(())).unwrap();
        assert_eq!(role.contents.borrow().native_alias_refs, 0);
        assert!(retained.check_stored_roles().is_err());
        assert!(interpreter.native_execution_refusal().is_none());
    }
}
