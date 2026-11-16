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

use super::*;
use crate::interp::ObjCommand;
use crate::obj::Owned;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

thread_local! {
    static UPDATES: RefCell<[usize; 4]> = const { RefCell::new([0; 4]) };
    static FREES: RefCell<[usize; 4]> = const { RefCell::new([0; 4]) };
    static GETTERS: RefCell<Vec<(usize, i64)>> = const { RefCell::new(Vec::new()) };
    static MEMBERS: RefCell<Vec<*mut TclObj>> = const { RefCell::new(Vec::new()) };
    static CALLBACK_REFS: RefCell<Vec<Vec<i64>>> = const { RefCell::new(Vec::new()) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}
const PAYLOADS: [&[u8]; 4] = [b"aliasProbe", b"capture", b"alpha \0 x", b"beta y"];
extern "C" fn update(original: *mut TclObj) {
    let role = usize::try_from(obj::internal_rep(original)).unwrap();
    UPDATES.with(|counts| counts.borrow_mut()[role] += 1);
    // SAFETY: the actual updater owns this live original header.
    GETTERS.with(|rows| {
        rows.borrow_mut()
            .push((role, unsafe { (*original).ref_count } as i64))
    });
    unsafe { obj::set_native_updater_string_rep(original, PAYLOADS[role], false) };
}
extern "C" fn free(original: *mut TclObj) {
    let role = usize::try_from(obj::internal_rep(original)).unwrap();
    FREES.with(|counts| counts.borrow_mut()[role] += 1);
}
static TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalAliasStorageOperand".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
unsafe extern "C" fn capture(
    _: *mut std::ffi::c_void,
    interp: *mut Interp,
    count: std::ffi::c_int,
    argv: *const *mut TclObj,
) -> std::ffi::c_int {
    // SAFETY: this actual registered command receives its live owner/argv.
    let interp = unsafe { &mut *interp };
    let argv = unsafe { std::slice::from_raw_parts(argv, usize::try_from(count).unwrap()) };
    capture_original(interp, argv).as_int() as i32
}
// Jim's public command registration owns a fresh CString key; it does not
// borrow the C CreateObjCommand publication address or callback ABI.
fn capture_jim(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    capture_original(interp, argv)
}
fn capture_original(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    assert_eq!(argv.len(), 4);
    MEMBERS.with(|members| assert_eq!(&argv[..3], members.borrow().as_slice()));
    CALLBACK_REFS.with(|rows| {
        rows.borrow_mut().push(
            argv[..3]
                .iter()
                .map(|&p| unsafe { (*p).ref_count } as i64)
                .collect(),
        )
    });
    CALLS.with(|calls| calls.set(calls.get() + 1));
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    for index in [2, 1] {
        let bytes = arguments.bytes(index).expect("original callback getter");
        assert_eq!(bytes.as_ref(), PAYLOADS[index + 1]);
    }
    interp.set_result_bytes(b"SEEN");
    arguments.finish(Code::Ok)
}
fn reset() {
    UPDATES.with(|v| *v.borrow_mut() = [0; 4]);
    FREES.with(|v| *v.borrow_mut() = [0; 4]);
    GETTERS.with(|v| v.borrow_mut().clear());
    MEMBERS.with(|v| v.borrow_mut().clear());
    CALLBACK_REFS.with(|v| v.borrow_mut().clear());
    CALLS.with(|v| v.set(0));
}
fn fields<'a>(
    stream: &'a str,
    kind: &str,
    case: usize,
    label: Option<&str>,
) -> HashMap<&'a str, &'a str> {
    let prefix = format!("{kind}|{case}|");
    let line = stream
        .lines()
        .find(|line| {
            line.starts_with(&prefix)
                && label.is_none_or(|label| {
                    line.strip_prefix(&prefix).unwrap().split('|').next() == Some(label)
                })
        })
        .expect("actual pinned observer row");
    line.split('|')
        .filter_map(|field| field.split_once('='))
        .collect()
}
fn number(fields: &HashMap<&str, &str>, key: &str) -> i64 {
    fields[key].parse().unwrap()
}
fn fixtures() -> [(&'static str, &'static str); 6] {
    [
            (
                "tcl8.4",
                include_str!(
                    "../../tests/data/native_alias_object_prefix_capture_original/415/providers/8.4.20/execute.stdout"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../tests/data/native_alias_object_prefix_capture_original/415/providers/8.5.19/execute.stdout"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../tests/data/native_alias_object_prefix_capture_original/415/providers/8.6.18/execute.stdout"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../tests/data/native_alias_object_prefix_capture_original/415/providers/9.0.4/execute.stdout"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../tests/data/native_alias_object_prefix_capture_original/415/providers/9.1.0/execute.stdout"
                ),
            ),
            (
                "jim",
                include_str!(
                    "../../tests/data/native_alias_object_prefix_capture_original/415/providers/jim/execute.stdout"
                ),
            ),
        ]
}
fn query(interp: &mut Interp, original_name: *mut TclObj, jim: bool, child: bool) -> Owned {
    let jim_words: &[&[u8]] = &[b"info", b"alias"];
    let c_words: &[&[u8]] = &[b"interp", b"alias", if child { b"child" } else { b"" }];
    let words = super::original_operand_tests::words(if jim { jim_words } else { c_words });
    let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
    argv.push(original_name);
    let code = if jim {
        interp.dispatch(&argv)
    } else {
        super::interp_cmd(interp, &argv)
    };
    assert_eq!(code, Code::Ok);
    Owned::retain(interp.get_obj_result())
}
fn check_row(stream: &str, case: usize, label: &str, originals: &[Owned]) {
    let row = fields(stream, "ROW", case, Some(label));
    for (role, original) in originals.iter().enumerate() {
        assert_eq!(
            unsafe { (*original.as_ptr()).ref_count } as i64,
            number(&row, &format!("r{role}_refs")),
            "{label} role{role}"
        );
        assert_eq!(
            obj::has_string_rep(original.as_ptr()),
            number(&row, &format!("r{role}_string")) != 0,
            "{label} role{role}"
        );
        assert_eq!(
            UPDATES.with(|v| v.borrow()[role]) as i64,
            number(&row, &format!("r{role}_update")),
            "{label} role{role}"
        );
    }
}

#[test]
fn original_alias_capture_and_query_match_the_measured_member_and_header_recipe() {
    // naming.alias.original-object-prefix-capture-query-invocation-lifetime
    // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
    // Bounded exact physical fields; Jim body-only name updater refs are not
    // compared with its public EvalObjVector registration invocation.
    for (version, stream) in fixtures() {
        for case in [1, 2, 3, 6] {
            if version == "jim" && case == 6 {
                continue;
            }
            reset();
            let mut interp = super::original_operand_tests::native(version);
            if version == "jim" {
                interp.register_builtin(b"capture", capture_jim);
                assert!(interp.native_execution_refusal().is_none());
            } else {
                interp
                    .create_obj_command(
                        b"capture",
                        ObjCommand::new(capture, std::ptr::null_mut(), None),
                    )
                    .expect("actual original C object command");
            }
            let child = case == 6;
            if child {
                assert_eq!(interp.eval_str(b"interp create child"), Code::Ok);
            }
            let originals = (0..4)
                .map(|role| Owned::fresh(obj::alloc_typed(&TYPE, role)))
                .collect::<Vec<_>>();
            MEMBERS.with(|members| {
                *members.borrow_mut() = originals[1..].iter().map(Owned::as_ptr).collect()
            });
            let jim = version == "jim";
            let jim_words: &[&[u8]] = &[b"alias"];
            let c_words: &[&[u8]] = &[b"interp", b"alias", if child { b"child" } else { b"" }];
            let words = super::original_operand_tests::words(if jim { jim_words } else { c_words });
            let target_path = Owned::fresh(obj::new_string_bytes(b""));
            let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            argv.push(originals[0].as_ptr());
            if !jim {
                argv.push(Owned::as_ptr(&target_path));
            }
            argv.extend(originals[1..].iter().map(Owned::as_ptr));
            assert_eq!(
                if jim {
                    super::jim_alias(&mut interp, &argv)
                } else {
                    super::interp_cmd(&mut interp, &argv)
                },
                Code::Ok
            );
            assert_eq!(interp.get_obj_result(), originals[0].as_ptr());
            check_row(stream, case, "after-capture", &originals);
            let actual_getters = GETTERS.with(|v| v.borrow().clone());
            let expected = stream
                .lines()
                .filter(|line| {
                    line.starts_with(&format!("EVENT|{case}|")) && line.contains("|update|")
                })
                .take(if jim { 1 } else { 2 })
                .map(|line| {
                    let row = line
                        .split('|')
                        .filter_map(|field| field.split_once('='))
                        .collect::<HashMap<_, _>>();
                    (
                        usize::try_from(number(&row, "role")).unwrap(),
                        number(&row, "refs"),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(
                actual_getters.iter().map(|v| v.0).collect::<Vec<_>>(),
                expected.iter().map(|v| v.0).collect::<Vec<_>>()
            );
            if !jim {
                assert_eq!(actual_getters, expected);
            }
            interp.set_result_bytes(b"");
            let mut queries = Vec::new();
            if case != 3 {
                let first = query(&mut interp, originals[0].as_ptr(), jim, child);
                assert_eq!(
                    ValueOps::list_elements(&mut interp, &first.as_ptr()).unwrap(),
                    originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
                );
                check_row(stream, case, "query-one-raw", &originals);
                let second = query(&mut interp, originals[0].as_ptr(), jim, child);
                let measured = fields(stream, "LIST", case, Some("query-two-raw"));
                assert_eq!(
                    first.as_ptr() == second.as_ptr(),
                    number(&measured, "same_other") != 0
                );
                assert_eq!(
                    ValueOps::list_elements(&mut interp, &second.as_ptr()).unwrap(),
                    originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
                );
                check_row(stream, case, "query-two-raw", &originals);
                if case == 2 {
                    let bytes = interp.native_object_string_bytes(first.as_ptr()).unwrap();
                    let present = fields(stream, "PRESENT", case, None);
                    assert_eq!(
                        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                        present["hex"]
                    );
                    let measured = fields(
                        stream,
                        "LIST",
                        case,
                        Some("query-two-after-other-presentation"),
                    );
                    assert_eq!(
                        obj::has_string_rep(second.as_ptr()),
                        number(&measured, "string") != 0
                    );
                }
                queries.extend([first, second]);
            }
            let tail = Owned::fresh(obj::new_string_bytes(b"TAIL"));
            for label in ["first-invocation", "second-invocation"] {
                let argv = [originals[0].as_ptr(), tail.as_ptr()];
                let code = if child {
                    interp
                        .with_child(b"child", |owner| owner.dispatch(&argv))
                        .unwrap()
                } else {
                    interp.dispatch(&argv)
                };
                assert_eq!(
                        code,
                        Code::Ok,
                        "{version}/{case}/{label}: parent cause {:?}, parent resident {:?}, child cause/resident {:?}",
                        interp.native_execution_refusal(),
                        obj::native_object_snapshot(interp.get_obj_result())
                            .map(|snapshot| snapshot.resident),
                        if child {
                            interp.with_child(b"child", |owner| {
                                (
                                    owner.native_execution_refusal(),
                                    obj::native_object_snapshot(owner.get_obj_result())
                                        .map(|snapshot| snapshot.resident),
                                )
                            })
                        } else {
                            None
                        }
                    );
                check_row(stream, case, label, &originals);
            }
            assert_eq!(CALLS.with(Cell::get), 2);
            let measured = fields(stream, "ROW", case, Some("capture-before-getters"));
            for refs in CALLBACK_REFS.with(|v| v.borrow().clone()) {
                for (role, reference) in refs[..3].iter().enumerate() {
                    assert_eq!(
                        reference,
                        &number(&measured, &format!("r{}_refs", role + 1))
                    );
                }
            }
            let prefix_updates = GETTERS.with(|v| {
                v.borrow()
                    .iter()
                    .filter(|(role, _)| *role >= 2)
                    .copied()
                    .collect::<Vec<_>>()
            });
            let expected = stream
                .lines()
                .filter(|line| {
                    line.starts_with(&format!("EVENT|{case}|")) && line.contains("|update|")
                })
                .filter_map(|line| {
                    let row = line
                        .split('|')
                        .filter_map(|field| field.split_once('='))
                        .collect::<HashMap<_, _>>();
                    let role = usize::try_from(number(&row, "role")).unwrap();
                    (role >= 2).then(|| (role, number(&row, "refs")))
                })
                .collect::<Vec<_>>();
            assert_eq!(prefix_updates, expected);
            interp.set_result_bytes(b"");
            drop(originals);
            let deleted = if child {
                interp
                    .with_child(b"child", |owner| owner.delete_command(b"aliasProbe"))
                    .unwrap()
            } else {
                interp.delete_command(b"aliasProbe")
            };
            assert!(deleted);
            if !queries.is_empty() {
                let measured = fields(stream, "LIST", case, Some("query-two-held-after-delete"));
                let members = ValueOps::list_elements(&mut interp, &queries[1].as_ptr()).unwrap();
                for (index, &member) in members.iter().enumerate() {
                    assert_eq!(
                        unsafe { (*member).ref_count } as i64,
                        number(&measured, &format!("m{index}_refs"))
                    );
                    assert_eq!(
                        interp.native_object_string_bytes(member).unwrap().as_ref(),
                        PAYLOADS[index + 1]
                    );
                }
                drop(queries.remove(0));
                assert_eq!(FREES.with(|v| v.borrow()[2..].to_vec()), vec![0, 0]);
                drop(queries);
            }
            assert_eq!(FREES.with(|v| v.borrow()[2..].to_vec()), vec![1, 1]);
            assert_eq!(UPDATES.with(|v| v.borrow()[2..].to_vec()), vec![1, 1]);
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_alias_query_retains_unread_members_until_the_last_query_is_released() {
    // naming.alias.original-object-prefix-capture-query-invocation-lifetime
    // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
    for (version, stream) in fixtures() {
        reset();
        let mut interp = super::original_operand_tests::native(version);
        let originals = (0..4)
            .map(|role| Owned::fresh(obj::alloc_typed(&TYPE, role)))
            .collect::<Vec<_>>();
        let jim = version == "jim";
        let words = super::original_operand_tests::words(if jim {
            &[b"alias"]
        } else {
            &[b"interp", b"alias", b""]
        });
        let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
        argv.push(originals[0].as_ptr());
        if !jim {
            argv.push(words[2].as_ptr());
        }
        argv.extend(originals[1..].iter().map(Owned::as_ptr));
        assert_eq!(
            if jim {
                super::jim_alias(&mut interp, &argv)
            } else {
                super::interp_cmd(&mut interp, &argv)
            },
            Code::Ok
        );
        let first = query(&mut interp, originals[0].as_ptr(), jim, false);
        let second = query(&mut interp, originals[0].as_ptr(), jim, false);
        let measured = fields(stream, "LIST", 1, Some("query-two-raw"));
        assert_eq!(
            first.as_ptr() == second.as_ptr(),
            number(&measured, "same_other") != 0
        );
        interp.set_result_bytes(b"");
        drop(originals);
        assert!(interp.delete_command(b"aliasProbe"));
        let members = ValueOps::list_elements(&mut interp, &second.as_ptr()).unwrap();
        assert_eq!(UPDATES.with(|v| v.borrow()[2..].to_vec()), vec![0, 0]);
        assert!(members[1..]
            .iter()
            .all(|&original| !obj::has_string_rep(original)));
        drop(first);
        assert_eq!(FREES.with(|v| v.borrow()[2..].to_vec()), vec![0, 0]);
        drop(second);
        assert_eq!(FREES.with(|v| v.borrow()[2..].to_vec()), vec![1, 1]);
        assert_eq!(UPDATES.with(|v| v.borrow()[2..].to_vec()), vec![0, 0]);
    }
}

#[test]
fn original_child_handle_alias_retains_lazy_parent_members_and_selected_name_role() {
    // naming.alias.original-object-prefix-capture-query-invocation-lifetime
    // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
    // C aliases own the supplied name; Jim_RegisterCmd owns a fresh child
    // key. These are software role controls, not unmeasured native counts.
    for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        reset();
        let jim = version == "jim";
        let mut interp = super::original_operand_tests::native(version);
        if jim {
            interp.register_builtin(b"capture", capture_jim);
            assert!(interp.native_execution_refusal().is_none());
        } else {
            interp
                .create_obj_command(
                    b"capture",
                    ObjCommand::new(capture, std::ptr::null_mut(), None),
                )
                .expect("actual parent C object command");
        }
        let child = if jim {
            interp.create_jim_child()
        } else {
            interp.create_child(Some(b"child".to_vec()))
        };
        let originals = (0..4)
            .map(|role| Owned::fresh(obj::alloc_typed(&TYPE, role)))
            .collect::<Vec<_>>();
        MEMBERS.with(|members| {
            *members.borrow_mut() = originals[1..].iter().map(Owned::as_ptr).collect();
        });
        let words = super::original_operand_tests::words(&[&child, b"alias"]);
        let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
        argv.extend(originals.iter().map(Owned::as_ptr));
        assert_eq!(interp.dispatch(&argv), Code::Ok);
        if jim {
            assert_eq!(interp.result_bytes(), b"");
        } else {
            assert_eq!(interp.get_obj_result(), originals[0].as_ptr());
        }
        assert_eq!(
            UPDATES.with(|counts| *counts.borrow()),
            if jim { [1, 0, 0, 0] } else { [1, 1, 0, 0] }
        );
        assert_eq!(
            GETTERS.with(|rows| rows.borrow().iter().map(|row| row.0).collect::<Vec<_>>()),
            if jim { vec![0] } else { vec![1, 0] }
        );
        interp.set_result_bytes(b"");
        assert_eq!(
            unsafe { (*originals[0].as_ptr()).ref_count },
            if jim { 1 } else { 2 }
        );
        let prefix = interp
            .with_child(&child, |owner| {
                let Some(crate::interp::Command::ParentAlias {
                    original_prefix,
                    registration_name,
                    ..
                }) = owner
                    .namespaces()
                    .command_in(crate::namespace::GLOBAL, b"aliasProbe")
                else {
                    panic!("actual original child alias record");
                };
                assert_eq!(registration_name.is_some(), !jim);
                original_prefix
            })
            .unwrap();
        assert_eq!(
            ValueOps::list_elements(&mut interp, &prefix.as_ptr()).unwrap(),
            originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
        );
        for member in &originals[1..] {
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 2);
        }
        assert!(!obj::has_string_rep(originals[2].as_ptr()));
        assert!(!obj::has_string_rep(originals[3].as_ptr()));
        if !jim {
            let first = interp
                .with_child(&child, |owner| {
                    owner.alias_query_object(b"aliasProbe").unwrap().unwrap()
                })
                .unwrap();
            let second = interp
                .with_child(&child, |owner| {
                    owner.alias_query_object(b"aliasProbe").unwrap().unwrap()
                })
                .unwrap();
            assert_ne!(first.as_ptr(), second.as_ptr());
            assert_ne!(first.as_ptr(), prefix.as_ptr());
            assert_eq!(
                ValueOps::list_elements(&mut interp, &first.as_ptr()).unwrap(),
                originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
            );
            drop(first);
            drop(second);
        }
        let call = super::original_operand_tests::words(&[b"aliasProbe", b"TAIL"]);
        let call_argv = call.iter().map(Owned::as_ptr).collect::<Vec<_>>();
        assert_eq!(
            interp
                .with_child(&child, |owner| owner.dispatch(&call_argv))
                .unwrap(),
            Code::Ok,
            "{version}: parent cause {:?}, parent resident {:?}, child cause/resident {:?}",
            interp.native_execution_refusal(),
            obj::native_object_snapshot(interp.get_obj_result()).map(|snapshot| snapshot.resident),
            interp.with_child(&child, |owner| {
                (
                    owner.native_execution_refusal(),
                    obj::native_object_snapshot(owner.get_obj_result())
                        .map(|snapshot| snapshot.resident),
                )
            })
        );
        assert_eq!(CALLS.with(Cell::get), 1);
        assert_eq!(UPDATES.with(|counts| *counts.borrow()), [1, 1, 1, 1]);
        let getter_roles =
            GETTERS.with(|rows| rows.borrow().iter().map(|row| row.0).collect::<Vec<_>>());
        assert_eq!(&getter_roles[getter_roles.len() - 2..], &[3, 2]);
        assert_eq!(
            MEMBERS.with(|members| members.borrow().clone()),
            originals[1..].iter().map(Owned::as_ptr).collect::<Vec<_>>()
        );
        interp
            .with_child(&child, |owner| {
                assert_eq!(owner.result_bytes(), b"SEEN");
                owner.delete_command(b"aliasProbe");
            })
            .unwrap();
        drop(prefix);
        assert_eq!(unsafe { (*originals[0].as_ptr()).ref_count }, 1);
        for member in &originals[1..] {
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, 1);
        }
    }
}
