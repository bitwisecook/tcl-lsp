// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Software controls of actual Runtime command roles, not Native outcomes.
use super::*;
use crate::interp::{default_host, Code, ObjCommand, WeakInterp};
use crate::namespace::GLOBAL;
use crate::obj::{self, NativeObjectLifetime, Owned, TclObj, TclObjType};
use std::cell::{Cell, RefCell};
use std::ffi::{c_int, c_void};

fn actual(profile: &str) -> Interp {
    Interp::with_native_core(
        default_host(),
        tcl_registry::model::ingress::resolve_environment(profile).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

struct Probe {
    interpreter: WeakInterp,
    events: RefCell<Vec<&'static str>>,
    generation: Cell<u64>,
    replacement: Cell<*mut Record>,
    mode: usize,
    deletes: [Cell<usize>; 2],
    member: Cell<*mut TclObj>,
    free_count: Cell<usize>,
}
struct Record {
    probe: *mut Probe,
    role: usize,
}
fn probe(interpreter: &Interp, mode: usize) -> (Box<Probe>, Box<[Record; 2]>) {
    let mut p = Box::new(Probe {
        interpreter: interpreter.downgrade(),
        events: RefCell::new(Vec::new()),
        generation: Cell::new(0),
        replacement: Cell::new(std::ptr::null_mut()),
        mode,
        deletes: std::array::from_fn(|_| Cell::new(0)),
        member: Cell::new(std::ptr::null_mut()),
        free_count: Cell::new(0),
    });
    let pointer = std::ptr::from_mut(p.as_mut());
    let mut records = Box::new(std::array::from_fn(|role| Record {
        probe: pointer,
        role,
    }));
    p.replacement.set(std::ptr::from_mut(&mut records[1]));
    (p, records)
}
unsafe extern "C" fn invoke_record(
    data: *mut c_void,
    actual: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    // Records are independent observations retained through full teardown.
    let r = unsafe { &*data.cast::<Record>() };
    let p = unsafe { &*r.probe };
    let actual = unsafe { &mut *actual };
    if r.role == 1 {
        actual.set_result_bytes(b"REPLACEMENT");
        return 0;
    }
    if p.mode <= 1 {
        p.events.borrow_mut().push("invoke-enter");
        assert!(actual.delete_command_generation(p.generation.get()));
        p.events.borrow_mut().push("delete-return");
        actual.set_result_bytes(b"ACTIVE");
        p.events.borrow_mut().push("invoke-return");
    } else if p.mode == 9 {
        p.events.borrow_mut().push("invoke-enter");
        actual
            .create_obj_command(
                b"active",
                ObjCommand::new(
                    invoke_record,
                    p.replacement.get().cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        p.events.borrow_mut().push("creation-return");
        actual.set_result_bytes(b"ACTIVE");
        p.events.borrow_mut().push("invoke-return");
    } else if p.mode == 3 {
        assert!(actual.delete_command(b"alias"));
        assert_eq!(
            unsafe { (*p.member.get()).ref_count },
            1,
            "actual forwarded argv holds the member"
        );
        assert_eq!(p.free_count.get(), 0);
        actual.set_result_bytes(b"FORWARDED");
    }
    0
}
unsafe extern "C" fn delete_record(data: *mut c_void) {
    let r = unsafe { &*data.cast::<Record>() };
    let p = unsafe { &*r.probe };
    p.deletes[r.role].set(p.deletes[r.role].get() + 1);
    p.events.borrow_mut().push(if r.role == 0 {
        "delete-original"
    } else {
        "delete-replacement"
    });
    if r.role == 0 && p.mode == 1 {
        let mut actual = p.interpreter.upgrade().unwrap();
        actual
            .create_obj_command(
                b"active",
                ObjCommand::new(
                    invoke_record,
                    p.replacement.get().cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
    }
    if r.role == 0 && (4..=6).contains(&p.mode) {
        let mut actual = p.interpreter.upgrade().unwrap();
        // Real callback effects precede later mandatory import retirement.
        if p.mode == 4 {
            actual.refuse_host_command("original deletion first Host");
        }
        if p.mode == 5 {
            let original = actual.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("deletion-aba".into(), "1.0".into())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&original).unwrap();
        }
        if p.mode == 6 {
            actual.frames.borrow_mut().push(GLOBAL);
        }
    }
}
unsafe extern "C" fn trace_record(
    data: *mut c_void,
    _: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    let r = unsafe { &*data.cast::<Record>() };
    let p = unsafe { &*r.probe };
    p.events.borrow_mut().push(if r.role == 0 {
        "middle-trace"
    } else {
        "leaf-trace"
    });
    0
}
fn install_chain(actual: &mut Interp, p: &Probe, records: &mut [Record; 2]) {
    assert_eq!(
        actual.eval_str(
            b"namespace eval origin {}; namespace eval middle {}; namespace eval leaf {}"
        ),
        Code::Ok
    );
    actual
        .create_obj_command(
            b"::origin::target",
            ObjCommand::new(
                invoke_record,
                std::ptr::from_mut(&mut records[0]).cast(),
                Some(delete_record),
            ),
        )
        .unwrap();
    assert_eq!(actual.eval_str(b"namespace eval origin {namespace export target}; namespace eval middle {namespace import ::origin::target; namespace export target}; namespace eval leaf {namespace import ::middle::target}"), Code::Ok);
    p.generation
        .set(actual.resolve_cmd_token(b"::origin::target").unwrap());
}

#[test]
fn original_active_object_deletion_separates_callback_from_dispatch_role_and_recreation() {
    // naming.embedding.original-active-object-command-deletion
    // docs/design/analysis/name-resolution-proofs/embedding-original-active-object-command-deletion.md
    // Software callback timing and later fresh-script replacement only; no Native held-word/client-data lifetime grant.
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for mode in 0..=1 {
            let mut actual = actual(profile);
            let (p, mut records) = probe(&actual, mode);
            let generation = actual
                .create_obj_command(
                    b"active",
                    ObjCommand::new(
                        invoke_record,
                        std::ptr::from_mut(&mut records[0]).cast(),
                        Some(delete_record),
                    ),
                )
                .unwrap();
            p.generation.set(generation);
            let original = match actual.namespaces().command_in(GLOBAL, b"active").unwrap() {
                Command::ObjCmd(record) => Rc::downgrade(&record),
                _ => unreachable!(),
            };
            assert_eq!(actual.eval_str(b"active"), Code::Ok);
            assert_eq!(actual.result_bytes(), b"ACTIVE");
            let expected = if profile == "jim" {
                [
                    "invoke-enter",
                    "delete-return",
                    "invoke-return",
                    "delete-original",
                ]
            } else {
                [
                    "invoke-enter",
                    "delete-original",
                    "delete-return",
                    "invoke-return",
                ]
            };
            assert_eq!(p.events.borrow().as_slice(), &expected, "{profile}/{mode}");
            assert_eq!(p.deletes[0].get(), 1);
            assert!(
                original.upgrade().is_none(),
                "observation does not retain dispatch"
            );
            if mode == 1 {
                assert_ne!(actual.resolve_cmd_token(b"active"), Some(generation));
                assert_eq!(actual.eval_str(b"active"), Code::Ok);
                assert_eq!(actual.result_bytes(), b"REPLACEMENT");
                assert!(actual.delete_command(b"active"));
                assert_eq!(p.deletes[1].get(), 1);
            } else {
                assert!(actual.resolve_cmd_token(b"active").is_none());
            }
            assert!(!actual.delete_command_generation(generation));
            assert_eq!(p.deletes[0].get(), 1);
            drop(actual);
        }
    }
}

#[test]
fn original_import_retirement_uses_release_order_and_depth_first_original_generations() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut actual = actual(profile);
        let (p, mut records) = probe(&actual, 2);
        install_chain(&mut actual, &p, &mut records[..]);
        actual
            .create_obj_command(
                b"middleObserver",
                ObjCommand::new(
                    trace_record,
                    std::ptr::from_mut(&mut records[0]).cast(),
                    None,
                ),
            )
            .unwrap();
        actual
            .create_obj_command(
                b"leafObserver",
                ObjCommand::new(
                    trace_record,
                    std::ptr::from_mut(&mut records[1]).cast(),
                    None,
                ),
            )
            .unwrap();
        assert_eq!(actual.eval_str(b"trace add command ::middle::target delete middleObserver; trace add command ::leaf::target delete leafObserver"), Code::Ok);
        assert!(actual.delete_command_generation(p.generation.get()));
        let expected = if matches!(profile, "tcl8.4" | "tcl8.5") {
            ["delete-original", "middle-trace", "leaf-trace"]
        } else {
            ["middle-trace", "leaf-trace", "delete-original"]
        };
        assert_eq!(p.events.borrow().as_slice(), &expected, "{profile}");
        for name in [
            b"::origin::target".as_slice(),
            b"::middle::target",
            b"::leaf::target",
        ] {
            assert!(actual.resolve_cmd_token(name).is_none());
        }
        assert_eq!(p.deletes[0].get(), 1);
        drop(actual);
    }
}

extern "C" fn member_free(value: *mut TclObj) {
    let p = unsafe { &*((*value).internal_rep as *const Probe) };
    p.free_count.set(p.free_count.get() + 1);
    let actual = p.interpreter.upgrade().unwrap();
    assert!(
        actual.namespaces.try_borrow_mut().is_ok(),
        "actual last role retires outside table access"
    );
}
extern "C" fn member_string(value: *mut TclObj) {
    unsafe { obj::set_string_rep(value, b"MEMBER") };
}
static MEMBER: TclObjType = TclObjType {
    name: c"deletion-member".as_ptr(),
    free_int_rep_proc: Some(member_free),
    dup_int_rep_proc: None,
    update_string_proc: Some(member_string),
    set_from_any_proc: None,
};
fn install_alias(
    actual: &mut Interp,
    p: &Probe,
    target: &[u8],
) -> (
    Rc<crate::interp::NativeAliasHeader>,
    NativeObjectLifetime,
    NativeObjectLifetime,
) {
    let member = obj::alloc_typed(&MEMBER, std::ptr::from_ref(p) as usize as u64);
    p.member.set(member);
    let member_observation = NativeObjectLifetime::retain(member);
    let target = Owned::fresh(obj::new_string_bytes(target));
    let prefix = Rc::new(crate::interp::NativeAliasHeader::new(Owned::fresh(
        actual.new_list_object(&[target.as_ptr(), member]),
    )));
    let prefix_observation = NativeObjectLifetime::retain(prefix.checked_ptr().unwrap());
    let name = Rc::new(crate::interp::NativeAliasHeader::new(Owned::fresh(
        obj::new_string_bytes(b"alias"),
    )));
    assert_eq!(
        actual
            .install_alias_objects(b"alias", name, Rc::clone(&prefix))
            .unwrap(),
        None
    );
    drop(target);
    (prefix, member_observation, prefix_observation)
}

#[test]
fn original_alias_metadata_views_do_not_duplicate_stored_native_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut actual = actual(profile);
        let (p, _records) = probe(&actual, 2);
        let (view, member, prefix) = install_alias(&mut actual, &p, b"target");
        let observer = actual.namespaces().command_in(GLOBAL, b"alias").unwrap();
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*prefix.as_ptr()).ref_count }, 1);
        assert!(actual.delete_command(b"alias"));
        assert_eq!(p.free_count.get(), 1);
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 0);
        assert_eq!(unsafe { (*prefix.as_ptr()).ref_count }, 0);
        assert!(matches!(
            view.checked_ptr(),
            Err(ValueError::CommandProtocolUnavailable(
                "original alias stored role retired"
            ))
        ));
        drop(observer);
        drop(view);
        drop(prefix);
        drop(member);
        drop(actual);
    }
}

#[test]
fn original_active_alias_keeps_only_its_genuine_forwarded_argument_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut actual = actual(profile);
        let (p, mut records) = probe(&actual, 3);
        actual
            .create_obj_command(
                b"target",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[0]).cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        let (view, member, prefix) = install_alias(&mut actual, &p, b"target");
        assert_eq!(actual.eval_str(b"alias"), Code::Ok);
        assert_eq!(actual.result_bytes(), b"FORWARDED");
        assert_eq!(p.free_count.get(), 1);
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 0);
        assert!(view.checked_ptr().is_err());
        drop(view);
        drop(member);
        drop(prefix);
        drop(actual);
    }
}

#[test]
fn original_deletion_retains_first_cause_and_finishes_actual_import_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 4..=6 {
            let mut actual = actual(profile);
            let (p, mut records) = probe(&actual, mode);
            install_chain(&mut actual, &p, &mut records[..]);
            actual.set_result_bytes(b"PRIOR");
            assert!(actual
                .retire_original_command_generation(p.generation.get(), true)
                .is_err());
            let first = actual.native_execution_refusal().unwrap();
            if mode == 4 {
                assert_eq!(
                    first,
                    tcl_runtime_api::NativeExecutionError::HostCommandRefusal {
                        message: "original deletion first Host".into()
                    }
                );
            }
            assert_eq!(p.deletes[0].get(), 1);
            for name in [
                b"::origin::target".as_slice(),
                b"::middle::target",
                b"::leaf::target",
            ] {
                assert!(actual
                    .namespaces()
                    .home_of(GLOBAL, name)
                    .and_then(|(ns, tail)| actual.namespaces().command_in(ns, &tail))
                    .is_none());
            }
            assert_eq!(obj::bytes_of(actual.get_obj_result()), b"PRIOR");
            if mode == 6 {
                let _ = actual.frames.borrow_mut().pop();
            }
            assert_eq!(actual.native_execution_refusal(), Some(first));
            drop(actual);
        }
    }
}

#[test]
fn original_namespace_and_forget_routes_retire_captured_command_generations() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut actual = actual(profile);
        let (p, mut records) = probe(&actual, 2);
        install_chain(&mut actual, &p, &mut records[..]);
        assert_eq!(
            actual.eval_str(b"namespace eval middle {namespace forget target}"),
            Code::Ok
        );
        assert_eq!(
            p.deletes[0].get(),
            0,
            "forget retires imports, not the actual source implementation"
        );
        assert!(actual.resolve_cmd_token(b"::middle::target").is_none());
        assert!(actual.resolve_cmd_token(b"::leaf::target").is_none());
        assert!(actual.resolve_cmd_token(b"::origin::target").is_some());
        actual
            .create_obj_command(
                b"::origin::second",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[1]).cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        assert_eq!(actual.eval_str(b"namespace delete origin"), Code::Ok);
        assert_eq!(p.deletes[0].get(), 1);
        assert_eq!(p.deletes[1].get(), 1);
        assert!(actual.resolve_cmd_token(b"::origin::target").is_none());
        assert!(actual.resolve_cmd_token(b"::origin::second").is_none());
        assert!(actual.native_execution_refusal().is_none());
        drop(actual);
    }
}
unsafe extern "C" fn move_or_recursive_trace(
    data: *mut c_void,
    actual: *mut Interp,
    _: c_int,
    _: *const *mut TclObj,
) -> c_int {
    let r = unsafe { &*data.cast::<Record>() };
    let p = unsafe { &*r.probe };
    let actual = unsafe { &mut *actual };
    p.events.borrow_mut().push("original-trace");
    if p.mode == 7 {
        assert!(actual.delete_command_generation(p.generation.get()));
    } else {
        assert_eq!(
            actual.rename_command(b"::origin::target", b"::other::moved"),
            crate::namespace::RenameOutcome::Renamed
        );
    }
    actual
        .create_obj_command(
            b"::origin::target",
            ObjCommand::new(
                invoke_record,
                p.replacement.get().cast(),
                Some(delete_record),
            ),
        )
        .unwrap();
    0
}
#[test]
fn original_recursive_and_moved_deletion_preserves_same_name_replacement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [7, 8] {
            let mut actual = actual(profile);
            let (p, mut records) = probe(&actual, mode);
            assert_eq!(
                actual.eval_str(b"namespace eval origin {}; namespace eval other {}"),
                Code::Ok
            );
            let generation = actual
                .create_obj_command(
                    b"::origin::target",
                    ObjCommand::new(
                        invoke_record,
                        std::ptr::from_mut(&mut records[0]).cast(),
                        Some(delete_record),
                    ),
                )
                .unwrap();
            p.generation.set(generation);
            actual
                .create_obj_command(
                    b"moveObserver",
                    ObjCommand::new(
                        move_or_recursive_trace,
                        std::ptr::from_mut(&mut records[0]).cast(),
                        None,
                    ),
                )
                .unwrap();
            assert_eq!(
                actual.eval_str(b"trace add command ::origin::target delete moveObserver"),
                Code::Ok
            );
            assert!(actual.delete_command_generation(generation));
            assert_eq!(p.deletes[0].get(), 1);
            assert_eq!(p.deletes[1].get(), 0);
            assert!(actual.resolve_cmd_token(b"::other::moved").is_none());
            assert_ne!(
                actual.resolve_cmd_token(b"::origin::target"),
                Some(generation)
            );
            assert_eq!(actual.eval_str(b"::origin::target"), Code::Ok);
            assert_eq!(actual.result_bytes(), b"REPLACEMENT");
            assert!(actual.delete_command(b"::origin::target"));
            assert_eq!(p.deletes[1].get(), 1);
            drop(actual);
        }
    }
}
#[test]
fn original_hidden_and_oo_automatic_cleanup_withdraw_real_implementation_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut actual = actual(profile);
        let (p, mut records) = probe(&actual, 2);
        let generation = actual
            .create_obj_command(
                b"hiddenTarget",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[0]).cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        assert_eq!(
            actual.hide_command(b"hiddenTarget", b"parked"),
            crate::interp::CommandVisibilityOutcome::Moved
        );
        assert!(actual.delete_command_generation(generation));
        assert_eq!(p.deletes[0].get(), 1);
        assert!(actual
            .hidden
            .borrow()
            .values()
            .all(|binding| binding.generation != generation));
        if matches!(profile, "tcl8.6" | "tcl9.0" | "tcl9.1") {
            assert_eq!(
                actual.eval_str(b"oo::object create victim; info object namespace victim"),
                Code::Ok
            );
            let mut helper = actual.result_bytes();
            helper.extend_from_slice(b"::helper");
            actual
                .create_obj_command(
                    &helper,
                    ObjCommand::new(
                        invoke_record,
                        std::ptr::from_mut(&mut records[1]).cast(),
                        Some(delete_record),
                    ),
                )
                .unwrap();
            let victim = actual.resolve_cmd_token(b"victim").unwrap();
            assert!(actual.delete_command_generation(victim));
            assert_eq!(
                p.deletes[1].get(),
                1,
                "automatic OO namespace teardown uses genuine command deletion"
            );
            assert!(actual.resolve_cmd_token(b"victim").is_none());
            assert!(actual.resolve_cmd_token(&helper).is_none());
        }
        assert!(actual.native_execution_refusal().is_none());
        drop(actual);
    }
}

#[test]
fn original_creation_delete_reuses_active_driver_and_preserves_actual_import_roster() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // C creation's import preservation differs from true ordinary deletion.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut actual = actual(profile);
        let (p, mut records) = probe(&actual, 9);
        actual
            .create_obj_command(
                b"active",
                ObjCommand::new(
                    invoke_record,
                    std::ptr::from_mut(&mut records[0]).cast(),
                    Some(delete_record),
                ),
            )
            .unwrap();
        if profile != "jim" {
            assert_eq!(
                actual.eval_str(
                    b"namespace export active; namespace eval imported {namespace import ::active}"
                ),
                Code::Ok
            );
        }
        assert_eq!(actual.eval_str(b"active"), Code::Ok);
        let expected = if profile == "jim" {
            [
                "invoke-enter",
                "creation-return",
                "invoke-return",
                "delete-original",
            ]
        } else {
            [
                "invoke-enter",
                "delete-original",
                "creation-return",
                "invoke-return",
            ]
        };
        assert_eq!(p.events.borrow().as_slice(), &expected);
        assert_eq!(p.deletes[0].get(), 1);
        if profile != "jim" {
            assert_eq!(actual.eval_str(b"::imported::active"), Code::Ok);
            assert_eq!(actual.result_bytes(), b"REPLACEMENT");
        }
        assert!(actual.delete_command(b"active"));
        assert_eq!(p.deletes[1].get(), 1);
        drop(actual);
    }
}
