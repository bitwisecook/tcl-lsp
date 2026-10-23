// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native ensemble table and reached prefix ownership controls.

use super::*;
use std::cell::{Cell, RefCell};
use tcl_dialect::TclVersion;
use tcl_syntax::native_string::NativeStringProtocol;

thread_local! {
    static MARKER_FREES: Cell<usize> = const { Cell::new(0) };
    static CALLBACK_INTERP: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static ORIGINAL_HEAD: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
}

extern "C" fn marker_free(_: *mut TclObj) {
    MARKER_FREES.with(|count| count.set(count.get() + 1));
}
extern "C" fn delete_ensemble(_: *mut TclObj) {
    CALLBACK_INTERP.with(|holder| {
        if let Some(interp) = holder.borrow().as_ref() {
            assert!(interp.clone().delete_command(b"::E"));
        }
    });
}
extern "C" fn update_and_delete_ensemble(original: *mut TclObj) {
    delete_ensemble(original);
    // SAFETY: native string lookup has a live original invocation root.
    unsafe {
        obj::set_string_rep(original, b"::target");
    }
}
static UPDATING_HEAD: obj::TclObjType = obj::TclObjType {
    name: c"updating-head".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_and_delete_ensemble),
    set_from_any_proc: None,
};
static MARKER: obj::TclObjType = obj::TclObjType {
    name: c"marker".as_ptr(),
    free_int_rep_proc: Some(marker_free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static DELETING_HEAD: obj::TclObjType = obj::TclObjType {
    name: c"deleting-head".as_ptr(),
    free_int_rep_proc: Some(delete_ensemble),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn owned(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn first(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"FIRST");
    Code::Ok
}
fn second(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"SECOND");
    Code::Ok
}
fn reached_original(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    ORIGINAL_HEAD.with(|head| assert_eq!(argv[0], head.get()));
    assert!(!interp.is_ensemble(b"::E"));
    assert!(obj::allocation_is_live(argv[0]));
    // SAFETY: the genuine invocation root owns this original head.
    unsafe {
        assert_eq!((*argv[0]).ref_count, 1);
    }
    interp.set_result_bytes(b"REACHED");
    Code::Ok
}
fn create(interp: &mut Interp, map: *mut TclObj) {
    let words: Vec<_> = [
        b"namespace".as_slice(),
        b"ensemble",
        b"create",
        b"-command",
        b"::E",
        b"-map",
    ]
    .into_iter()
    .map(owned)
    .collect();
    let mut argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
    argv.push(map);
    assert_eq!(ens_create(interp, &argv), Code::Ok);
}
fn invoke(interp: &mut Interp) {
    let words = [owned(b"::E"), owned(b"m")];
    assert_eq!(
        interp.dispatch(&words.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()),
        Code::Ok
    );
}

#[test]
fn counted_map_collision_preserves_native_unreleased_reference_after_teardown() {
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        MARKER_FREES.with(|count| count.set(0));
        let first_pointer;
        let marker_pointer;
        {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            interp.bind_command_replacement(GLOBAL, b"first", Command::Builtin(first));
            interp.bind_command_replacement(GLOBAL, b"second", Command::Builtin(second));
            let protocol = NativeStringProtocol::C(version);
            let left = owned(b"m\0left");
            let right = owned(b"m\0right");
            let marker = owned(b"MARK");
            marker_pointer = marker.as_ptr();
            // SAFETY: this original String owns no previous internal payload.
            unsafe {
                (*marker_pointer).type_ptr = &MARKER;
            }
            let heads = [owned(b"::first"), owned(b"::second")];
            let prefixes = [
                obj::Owned::fresh(list::new_list_obj_native(
                    &[heads[0].as_ptr(), marker_pointer],
                    protocol,
                )),
                obj::Owned::fresh(list::new_list_obj_native(&[heads[1].as_ptr()], protocol)),
            ];
            first_pointer = prefixes[0].as_ptr();
            let map = obj::Owned::fresh(
                crate::dict::new_dict_obj_native(
                    &[
                        (left.as_ptr(), prefixes[0].as_ptr()),
                        (right.as_ptr(), prefixes[1].as_ptr()),
                    ],
                    None,
                    protocol,
                )
                .unwrap(),
            );
            create(&mut interp, map.as_ptr());
            invoke(&mut interp);
            assert_eq!(interp.result_bytes(), b"SECOND");
            // SAFETY: both prefix headers have actual test and configured owners.
            unsafe {
                assert_eq!((*prefixes[0].as_ptr()).ref_count, 3);
                assert_eq!((*prefixes[1].as_ptr()).ref_count, 3);
            }
            let subcommands = obj::Owned::fresh(list::new_list_obj_native(
                &[left.as_ptr(), right.as_ptr()],
                protocol,
            ));
            let words: Vec<_> = [
                b"namespace".as_slice(),
                b"ensemble",
                b"configure",
                b"::E",
                b"-subcommands",
            ]
            .into_iter()
            .map(owned)
            .collect();
            let mut argv: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
            argv.push(subcommands.as_ptr());
            assert_eq!(ens_configure(&mut interp, &argv), Code::Ok);
            invoke(&mut interp);
            assert_eq!(interp.result_bytes(), b"FIRST");
            unsafe {
                assert_eq!((*prefixes[0].as_ptr()).ref_count, 4);
                assert_eq!((*prefixes[1].as_ptr()).ref_count, 2);
            }
        }
        assert!(obj::allocation_is_live(first_pointer));
        assert!(obj::allocation_is_live(marker_pointer));
        // Native Tcl leaves this unreachable +1 after Tcl_DeleteInterp too.
        unsafe {
            assert_eq!((*first_pointer).ref_count, 1);
            assert_eq!((*marker_pointer).ref_count, 1);
        }
        MARKER_FREES.with(|count| assert_eq!(count.get(), 0));
        // Explicit probe cleanup is outside the engine teardown under test.
        unsafe {
            obj::decr_ref_count(first_pointer);
        }
        MARKER_FREES.with(|count| assert_eq!(count.get(), 1));
    }
}

#[test]
fn reached_original_prefix_survives_custom_head_retiring_the_ensemble() {
    for updating in [false, true] {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            interp.bind_command_replacement(GLOBAL, b"target", Command::Builtin(reached_original));
            let protocol = NativeStringProtocol::C(version);
            let head = owned(b"::target");
            unsafe {
                (*head.as_ptr()).type_ptr = if updating {
                    &UPDATING_HEAD
                } else {
                    &DELETING_HEAD
                };
            }
            ORIGINAL_HEAD.with(|original| original.set(head.as_ptr()));
            let prefix = obj::Owned::fresh(list::new_list_obj_native(&[head.as_ptr()], protocol));
            let key = owned(b"m");
            let map = obj::Owned::fresh(
                crate::dict::new_dict_obj_native(
                    &[(key.as_ptr(), prefix.as_ptr())],
                    None,
                    protocol,
                )
                .unwrap(),
            );
            create(&mut interp, map.as_ptr());
            if updating {
                obj::invalidate_string(head.as_ptr());
            }
            drop(map);
            drop(prefix);
            drop(key);
            drop(head);
            CALLBACK_INTERP.with(|holder| *holder.borrow_mut() = Some(interp.clone()));
            invoke(&mut interp);
            CALLBACK_INTERP.with(|holder| drop(holder.borrow_mut().take()));
            assert_eq!(interp.result_bytes(), b"REACHED");
            assert!(!obj::allocation_is_live(ORIGINAL_HEAD.with(Cell::get)));
        }
    }
}
