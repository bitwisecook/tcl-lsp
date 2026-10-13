// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Operation-specific fresh replay from actual namespace geometry and original bytes.

use super::command_receipts::original_bytes;
use super::{EngineCommandReceipt, TCL_ERROR};
use crate::interp::{new_string, Interp};
use crate::namespace::{NsId, GLOBAL};
use crate::obj::{self, Owned, TclObj};
use core::ffi::{c_int, c_void};
use std::rc::Rc;
use tcl_runtime_api::engine_command_replay::{
    EngineCommandReplayContext as Context, EngineCommandReplayNamespace as Namespace,
    EngineCommandReplayPurpose as Purpose,
};
use tcl_syntax::naming::NamePolicyAuthority;

fn capture(interp: &Interp, purpose: Purpose) -> Option<Context> {
    let policy = interp.name_policy_protocol()?;
    if policy.authority() != NamePolicyAuthority::Native {
        return None;
    }
    let protocol = policy.recipe();
    let names = interp.namespaces();
    let current = interp.current_ns();
    let namespace = if protocol.is_jim084() {
        let _holder = names.jim_namespace_original_owner(current)?;
        Namespace::Jim(names.jim_namespace_bytes(current)?.into())
    } else {
        let command_path = names
            .path(current)
            .iter()
            .filter(|ns| names.namespace_is_live(**ns))
            .map(|ns| names.native_context_path(*ns))
            .collect::<Option<Vec<_>>>()?;
        Namespace::C {
            namespace: names.native_context_path(current)?,
            live: names.namespace_is_live(current),
            command_path,
        }
    };
    Some(Context {
        root_interpreter: interp.engine_replay_root_interpreter(),
        protocol,
        purpose,
        namespace,
    })
}

/// Capture fresh replay data from the actual current namespace holder.
/// It grants no old command generation, object or namespace identity.
///
/// # Safety
/// `interp` must be live. The caller adopts the returned object reference.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_command_replay_context(
    interp: *mut Interp,
    purpose: c_int,
) -> *mut TclObj {
    // SAFETY: caller guarantees a live interpreter.
    let interp = unsafe { &mut *interp };
    if interp.host_refusal_pending() {
        return core::ptr::null_mut();
    }
    let purpose = match purpose {
        1 => Purpose::Create,
        2 => Purpose::Remove,
        _ => {
            interp.refuse_host_command("engine replay has no selected command purpose");
            return core::ptr::null_mut();
        }
    };
    let Some(bytes) = capture(interp, purpose).and_then(|context| context.encode()) else {
        interp.refuse_host_command("engine replay has no actual namespace context");
        return core::ptr::null_mut();
    };
    let original = new_string(&bytes);
    // SAFETY: caller adopts one independent original reference.
    unsafe {
        obj::incr_ref_count(original);
    }
    original
}

struct ReplayScope {
    interp: Interp,
    saved: NsId,
    target: NsId,
    _jim_holder: Option<Rc<Owned>>,
}

impl ReplayScope {
    fn enter(interp: &mut Interp, context: Context, purpose: Purpose) -> Option<Self> {
        if !context.root_interpreter || !interp.engine_replay_root_interpreter() {
            interp.refuse_host_command(
                "engine replay of a child interpreter requires authentic child ownership transport",
            );
            return None;
        }
        if interp.name_policy_protocol().is_none_or(|policy| {
            policy.authority() != NamePolicyAuthority::Native || policy.recipe() != context.protocol
        }) || context.purpose != purpose
        {
            interp.refuse_host_command("engine replay command purpose or protocol changed");
            return None;
        }
        let (target, jim_holder) = match context.namespace {
            Namespace::C {
                namespace,
                live,
                command_path,
            } => {
                if !live {
                    interp.refuse_host_command("engine replay source namespace has retired");
                    return None;
                }
                let mut names = interp.namespaces_mut();
                let target = names.ensure_namespace_at_components(GLOBAL, namespace.as_segments());
                let path = command_path
                    .iter()
                    .map(|path| names.ensure_namespace_at_components(GLOBAL, path.as_segments()))
                    .collect();
                names.set_path(target, path);
                (target, None)
            }
            Namespace::Jim(namespace) => {
                let holder = Owned::fresh(new_string(namespace.as_bytes()));
                let Ok(context) = interp.native_jim_object_context() else {
                    interp.refuse_host_command("Jim replay original object context unavailable");
                    return None;
                };
                if crate::native_source::bind_context(holder.as_ptr(), &context).is_err() {
                    interp.refuse_host_command("Jim replay namespace holder unavailable");
                    return None;
                }
                let mut names = interp.namespaces_mut();
                let target = names.retain_jim_namespace(holder, Rc::from(namespace.as_bytes()));
                let holder = names.take_jim_namespace_owner(target)?;
                (target, Some(holder))
            }
        };
        let saved = interp.current_ns();
        interp.enter_namespace_activation(target);
        interp.set_current_ns(target);
        Some(Self {
            interp: interp.clone(),
            saved,
            target,
            _jim_holder: jim_holder,
        })
    }
}

impl Drop for ReplayScope {
    fn drop(&mut self) {
        self.interp.set_current_ns(self.saved);
        self.interp.leave_namespace_activation(Some(self.target));
    }
}

fn replay_scope(
    interp: &mut Interp,
    bytes: *const u8,
    length: c_int,
    purpose: Purpose,
) -> Option<ReplayScope> {
    if interp.host_refusal_pending() {
        return None;
    }
    let original = original_bytes(interp, bytes, length).ok()?;
    let Some(context) = Context::decode(original) else {
        interp.refuse_host_command("engine replay context has no counted original recipe");
        return None;
    };
    ReplayScope::enter(interp, context, purpose)
}

/// Replay counted C API publication in a fresh actual namespace context.
/// Existing publication selects its own input extent and returns a new receipt.
///
/// # Safety
/// Interpreter, counted inputs, callback and output receipt must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_replay_create_command_counted(
    interp: *mut Interp,
    context: *const u8,
    context_length: c_int,
    name: *const u8,
    length: c_int,
    procedure: Option<crate::interp::TclObjCmdProc>,
    client: *mut c_void,
    out: *mut EngineCommandReceipt,
) -> c_int {
    // SAFETY: caller guarantees a live original interpreter.
    if unsafe { &*interp }.host_refusal_pending() {
        return TCL_ERROR;
    }
    if procedure.is_none() || out.is_null() {
        // SAFETY: preserve the original publication's pre-effect refusal order.
        return unsafe {
            super::tcl_engine_create_command_counted(interp, name, length, procedure, client, out)
        };
    }
    if original_bytes(unsafe { &mut *interp }, name, length).is_err() {
        return TCL_ERROR;
    }
    let creation = unsafe { &*interp }
        .native_invocation_dialect()
        .native_command_creation_protocol()
        .and_then(|protocol| protocol.public_registration_recipe(
            tcl_registry::native_command_creation::NativeObjectCommandCreationPurpose::CountedEngineObjectCommand,
        ));
    if creation.is_none() {
        // SAFETY: the original facade owns the unsupported purpose's refusal.
        return unsafe {
            super::tcl_engine_create_command_counted(interp, name, length, procedure, client, out)
        };
    }
    let Some(_scope) = replay_scope(
        unsafe { &mut *interp },
        context,
        context_length,
        Purpose::Create,
    ) else {
        return TCL_ERROR;
    };
    // SAFETY: original counted operation owns publication and the fresh receipt.
    unsafe {
        super::tcl_engine_create_command_counted(interp, name, length, procedure, client, out)
    }
}

/// Replay original counted retirement with its authentic namespace lookup bases.
/// Existing deletion returns a fresh selected pre-retirement receipt.
///
/// # Safety
/// Interpreter, counted inputs and output receipt must be live.
#[no_mangle]
pub unsafe extern "C" fn tcl_engine_replay_delete_command_counted(
    interp: *mut Interp,
    context: *const u8,
    context_length: c_int,
    name: *const u8,
    length: c_int,
    out: *mut EngineCommandReceipt,
) -> c_int {
    // SAFETY: caller guarantees a live original interpreter.
    if unsafe { &*interp }.host_refusal_pending() {
        return TCL_ERROR;
    }
    if out.is_null() {
        // SAFETY: the original deletion refuses its missing output before effects.
        return unsafe { super::tcl_engine_delete_command_counted(interp, name, length, out) };
    }
    if original_bytes(unsafe { &mut *interp }, name, length).is_err() {
        return TCL_ERROR;
    }
    let Some(_scope) = replay_scope(
        unsafe { &mut *interp },
        context,
        context_length,
        Purpose::Remove,
    ) else {
        return TCL_ERROR;
    };
    // SAFETY: original counted deletion owns lookup, retirement and its receipt.
    unsafe { super::tcl_engine_delete_command_counted(interp, name, length, out) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_core_types::NameBytes;

    fn core(profile: &str) -> Interp {
        Interp::with_native_core(
            Interp::new().host(),
            tcl_registry::model::resolve_environment(profile).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    unsafe extern "C" fn callback(
        _: *mut c_void,
        interp: *mut Interp,
        _: c_int,
        _: *const *mut TclObj,
    ) -> c_int {
        // SAFETY: actual native dispatcher supplies its live interpreter.
        unsafe { &mut *interp }.set_result_bytes(b"REPLAY\0\xff");
        super::super::TCL_OK
    }

    fn context(interp: &mut Interp, purpose: c_int) -> Vec<u8> {
        // SAFETY: actual interpreter supplies an independent counted context object.
        let original = unsafe { tcl_engine_command_replay_context(interp, purpose) };
        assert!(!original.is_null());
        // SAFETY: successful producer transfers exactly one original reference.
        let original = unsafe { Owned::from_raw(original) };
        obj::bytes_of(original.as_ptr()).to_vec()
    }

    fn create(
        interp: &mut Interp,
        name: &[u8],
        context: Option<&[u8]>,
    ) -> super::super::EngineCommandIdentity {
        let mut receipt = core::mem::MaybeUninit::uninit();
        // SAFETY: original counted inputs, actual callback and writable receipt are live.
        let code = unsafe {
            match context {
                Some(context) => tcl_engine_replay_create_command_counted(
                    interp,
                    context.as_ptr(),
                    context.len().try_into().unwrap(),
                    name.as_ptr(),
                    name.len().try_into().unwrap(),
                    Some(callback),
                    core::ptr::null_mut(),
                    receipt.as_mut_ptr(),
                ),
                None => super::super::tcl_engine_create_command_counted(
                    interp,
                    name.as_ptr(),
                    name.len().try_into().unwrap(),
                    Some(callback),
                    core::ptr::null_mut(),
                    receipt.as_mut_ptr(),
                ),
            }
        };
        assert_eq!(code, super::super::TCL_OK, "{:?}", interp.result_bytes());
        // SAFETY: genuine successful producer initialized the receipt.
        let receipt = unsafe { receipt.assume_init() };
        // SAFETY: this test inspects identity, then retires the independent report reference.
        drop(unsafe { Owned::from_raw(receipt.qualified) });
        receipt.into()
    }

    fn actual(interp: &Interp, parts: &[&[u8]]) -> NsId {
        let parts = parts
            .iter()
            .map(|part| NameBytes::from(*part))
            .collect::<Vec<_>>();
        interp
            .namespaces()
            .namespace_at_components(GLOBAL, &parts)
            .unwrap()
    }

    unsafe extern "C" fn capture_callback(
        client: *mut c_void,
        interp: *mut Interp,
        _: c_int,
        _: *const *mut TclObj,
    ) -> c_int {
        // SAFETY: original dispatcher supplies a live interpreter and fixture purpose.
        let original =
            unsafe { tcl_engine_command_replay_context(interp, client as usize as c_int) };
        if original.is_null() {
            return TCL_ERROR;
        }
        // SAFETY: actual context producer transfers one owned reference.
        let original = unsafe { Owned::from_raw(original) };
        unsafe { &mut *interp }.set_result(original.as_ptr());
        super::super::TCL_OK
    }

    #[test]
    fn jim_replay_retains_counted_holder_and_refuses_unavailable_creation() {
        use crate::interp::ObjCommand;
        let mut source = core("jim");
        for (name, purpose) in [
            (b"capture_creation".as_slice(), 1usize),
            (b"capture_retirement", 2),
        ] {
            assert!(source
                .create_obj_command(
                    name,
                    ObjCommand::new(capture_callback, purpose as *mut c_void, None)
                )
                .is_some());
        }
        let creation = source
            .eval_completion(b"namespace eval :outer {capture_creation}")
            .unwrap()
            .result;
        let retirement = source
            .eval_completion(b"namespace eval :outer {capture_retirement}")
            .unwrap()
            .result;
        assert!(matches!(
            Context::decode(&creation).unwrap().namespace,
            Namespace::Jim(_)
        ));
        let mut refused = core("jim");
        let mut receipt = core::mem::MaybeUninit::uninit();
        // SAFETY: live counted context, original name, callback and writable output.
        assert_eq!(
            unsafe {
                tcl_engine_replay_create_command_counted(
                    &mut refused,
                    creation.as_ptr(),
                    creation.len().try_into().unwrap(),
                    b"kept".as_ptr(),
                    4,
                    Some(callback),
                    core::ptr::null_mut(),
                    receipt.as_mut_ptr(),
                )
            },
            TCL_ERROR
        );
        assert!(format!("{:?}", refused.native_execution_refusal())
            .contains("public command constructor purpose has no selected native recipe"));
        assert_eq!(refused.current_ns(), GLOBAL);
        assert!(refused
            .namespaces()
            .command_generation(GLOBAL, b"kept")
            .is_none());
        let mut fresh = core("jim");
        let selected = fresh
            .create_obj_command(
                b":outer::kept",
                ObjCommand::new(callback, core::ptr::null_mut(), None),
            )
            .unwrap();
        let global = fresh
            .create_obj_command(
                b"kept",
                ObjCommand::new(callback, core::ptr::null_mut(), None),
            )
            .unwrap();
        // SAFETY: actual captured holder, original name and writable receipt.
        assert_eq!(
            unsafe {
                tcl_engine_replay_delete_command_counted(
                    &mut fresh,
                    retirement.as_ptr(),
                    retirement.len().try_into().unwrap(),
                    b"kept".as_ptr(),
                    4,
                    receipt.as_mut_ptr(),
                )
            },
            super::super::TCL_OK,
            "{:?}",
            fresh.native_execution_refusal()
        );
        // SAFETY: actual retirement transfers its selected pre-retirement report.
        let retired = unsafe { receipt.assume_init() };
        drop(unsafe { Owned::from_raw(retired.qualified) });
        assert_eq!(retired.generation, selected);
        assert_eq!(fresh.resolve_cmd_token(b"kept"), Some(global));
        assert_eq!(fresh.current_ns(), GLOBAL);
        assert!(!fresh.host_refusal_pending());
    }

    #[test]
    fn actual_child_receiver_refuses_root_replay_data_before_publication() {
        let mut parent = core("tcl9.0");
        let root_context = context(&mut parent, 1);
        assert!(Context::decode(&root_context).unwrap().root_interpreter);
        let name = parent.create_child(Some(b"receiver".to_vec()));
        parent
            .with_child(&name, |child| {
                let installed = create(child, b"existing::kept", None);
                let existing = actual(child, &[b"existing"]);
                let mut receipt = core::mem::MaybeUninit::uninit();
                // SAFETY: actual child, root capture data, original name and writable receipt.
                assert_eq!(
                    unsafe {
                        tcl_engine_replay_create_command_counted(
                            child,
                            root_context.as_ptr(),
                            root_context.len().try_into().unwrap(),
                            b"replayed::kept".as_ptr(),
                            14,
                            Some(callback),
                            core::ptr::null_mut(),
                            receipt.as_mut_ptr(),
                        )
                    },
                    TCL_ERROR
                );
                assert!(child.host_refusal_pending());
                assert!(child
                    .namespaces()
                    .namespace_at_components(GLOBAL, &[NameBytes::from(b"replayed".as_slice())])
                    .is_none());
                assert_eq!(
                    child.namespaces().command_generation(existing, b"kept"),
                    Some(installed.generation)
                );
            })
            .unwrap();
    }

    #[test]
    fn actual_child_publication_succeeds_and_fresh_root_replay_refuses_its_realm() {
        let mut parent = core("tcl9.0");
        let name = parent.create_child(Some(b"child".to_vec()));
        let captured = parent
            .with_child(&name, |child| {
                child.eval_completion(b"namespace eval local {}").unwrap();
                child.set_current_ns(actual(child, &[b"local"]));
                let captured = context(child, 1);
                let installed = create(child, b"nested::kept", None);
                let holder = actual(child, &[b"local", b"nested"]);
                assert_eq!(
                    child
                        .namespaces()
                        .native_command_slot_at_node(installed.generation),
                    Some((holder, b"kept".to_vec()))
                );
                assert!(!child.host_refusal_pending());
                captured
            })
            .unwrap();
        assert!(!Context::decode(&captured).unwrap().root_interpreter);
        let mut fresh = core("tcl9.0");
        let before = fresh.namespaces().native_command_generations();
        let children = fresh.namespaces().children(GLOBAL);
        let mut receipt = core::mem::MaybeUninit::uninit();
        // SAFETY: captured actual child data, original input and a writable, refused receipt.
        assert_eq!(
            unsafe {
                tcl_engine_replay_create_command_counted(
                    &mut fresh,
                    captured.as_ptr(),
                    captured.len().try_into().unwrap(),
                    b"nested::kept".as_ptr(),
                    12,
                    Some(callback),
                    core::ptr::null_mut(),
                    receipt.as_mut_ptr(),
                )
            },
            TCL_ERROR
        );
        assert!(fresh.host_refusal_pending());
        assert_eq!(fresh.namespaces().native_command_generations(), before);
        assert_eq!(fresh.namespaces().children(GLOBAL), children);
        assert_eq!(fresh.current_ns(), GLOBAL);
    }

    #[test]
    fn fresh_replay_keeps_actual_literal_colon_context_and_original_c_api_input() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut source = core(profile);
            source
                .eval_completion(b"namespace eval :outer {namespace eval :inner {}} ")
                .unwrap();
            source.set_current_ns(actual(&source, &[b":outer", b":inner"]));
            let captured = context(&mut source, 1);
            let original = b"child::kept\0unused";
            let old = create(&mut source, original, None);
            let mut fresh = core(profile);
            fresh.eval_completion(b"namespace eval outer {namespace eval inner {namespace eval child {proc kept {} {return REPLACEMENT}}}} ").unwrap();
            let replacement = fresh
                .resolve_cmd_token(b"::outer::inner::child::kept")
                .unwrap();
            let replayed = create(&mut fresh, original, Some(&captured));
            assert_ne!(old.owner, replayed.owner);
            let holder = actual(&fresh, &[b":outer", b":inner", b"child"]);
            assert_eq!(
                fresh
                    .namespaces()
                    .native_command_slot_at_node(replayed.generation),
                Some((holder, b"kept".to_vec())),
                "{profile}"
            );
            assert_eq!(
                fresh.resolve_cmd_token(b"::outer::inner::child::kept"),
                Some(replacement)
            );
            assert_eq!(fresh.current_ns(), GLOBAL);
            assert_eq!(fresh.eval_completion(b"namespace eval :outer {namespace eval :inner {namespace eval child {kept}}}").unwrap().result, b"REPLAY\0\xff", "{profile}");
            assert!(!fresh.host_refusal_pending());
        }
    }

    #[test]
    fn fresh_retirement_uses_actual_command_path_and_preserves_the_global_replacement() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut source = core(profile);
            source
                .eval_completion(b"namespace eval :caller {}; namespace eval search {}")
                .unwrap();
            source.set_current_ns(actual(&source, &[b"search"]));
            let creation_context = context(&mut source, 1);
            create(&mut source, b"nested::victim", None);
            source.set_current_ns(GLOBAL);
            source
                .eval_completion(b"namespace eval :caller {namespace path ::search::nested}")
                .unwrap();
            source.set_current_ns(actual(&source, &[b":caller"]));
            let retirement_context = context(&mut source, 2);
            let mut fresh = core(profile);
            let selected = create(&mut fresh, b"nested::victim", Some(&creation_context));
            let global = create(&mut fresh, b"victim", None);
            let mut receipt = core::mem::MaybeUninit::uninit();
            // SAFETY: real captured context, original counted name and writable receipt.
            assert_eq!(
                unsafe {
                    tcl_engine_replay_delete_command_counted(
                        &mut fresh,
                        retirement_context.as_ptr(),
                        retirement_context.len().try_into().unwrap(),
                        b"victim".as_ptr(),
                        6,
                        receipt.as_mut_ptr(),
                    )
                },
                super::super::TCL_OK
            );
            // SAFETY: actual deletion transferred its selected pre-retirement receipt.
            let retired = unsafe { receipt.assume_init() };
            drop(unsafe { Owned::from_raw(retired.qualified) });
            assert_eq!(retired.generation, selected.generation, "{profile}");
            assert!(fresh
                .namespaces()
                .native_command_slot_at_node(selected.generation)
                .is_none());
            assert_eq!(fresh.resolve_cmd_token(b"victim"), Some(global.generation));
            assert_eq!(fresh.current_ns(), GLOBAL);
        }
    }

    #[test]
    fn retired_or_wrong_purpose_replay_context_refuses_before_fresh_publication() {
        let mut source = core("tcl9.0");
        source
            .eval_completion(b"namespace eval :retired {}")
            .unwrap();
        let holder = actual(&source, &[b":retired"]);
        source.enter_namespace_activation(holder);
        source.set_current_ns(holder);
        source.delete_namespace_by_id(holder);
        let retired = context(&mut source, 1);
        source.set_current_ns(GLOBAL);
        source.leave_namespace_activation(Some(holder));
        let wrong_purpose = context(&mut source, 2);
        for bytes in [
            &retired[..],
            &wrong_purpose[..],
            b"NCR1\x01\x04\x01\x01\xff\xff\xff\xff",
        ] {
            let mut fresh = core("tcl9.0");
            let before = fresh.namespaces().native_command_generations();
            let children = fresh.namespaces().children(GLOBAL);
            let mut receipt = core::mem::MaybeUninit::uninit();
            // SAFETY: counted fixture bytes and real interpreter; refused receipt is unread.
            assert_eq!(
                unsafe {
                    tcl_engine_replay_create_command_counted(
                        &mut fresh,
                        bytes.as_ptr(),
                        bytes.len().try_into().unwrap(),
                        b"child::kept".as_ptr(),
                        11,
                        Some(callback),
                        core::ptr::null_mut(),
                        receipt.as_mut_ptr(),
                    )
                },
                TCL_ERROR
            );
            assert!(fresh.host_refusal_pending());
            assert_eq!(fresh.namespaces().native_command_generations(), before);
            assert_eq!(fresh.namespaces().children(GLOBAL), children);
            assert_eq!(fresh.current_ns(), GLOBAL);
        }
    }
}
