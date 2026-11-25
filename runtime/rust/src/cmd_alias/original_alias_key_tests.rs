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
use crate::obj::Owned;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;

thread_local! {
    static OWNER: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static RENAME: Cell<bool> = const { Cell::new(false) };
    static RETIRE: Cell<u8> = const { Cell::new(0) };
    static RETIRE_ROLE: Cell<usize> = const { Cell::new(0) };
    static ORDER: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    static GETTERS: RefCell<Vec<(usize, i64)>> = const { RefCell::new(Vec::new()) };
    static FIRST: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
}
fn payload(role: usize) -> &'static [u8] {
    match role {
        0 => b"aliasKeyProbe",
        1 => b"list",
        2 => b"first \0 member",
        3 => b"last member",
        4 if RENAME.with(Cell::get) => b"aliasKeyMoved",
        4 => b"aliasKeyProbe",
        _ => unreachable!("known fixed original role"),
    }
}
extern "C" fn update(original: *mut TclObj) {
    let role = usize::try_from(obj::internal_rep(original)).unwrap();
    GETTERS.with(|rows| {
        rows.borrow_mut()
            .push((role, unsafe { (*original).ref_count } as i64))
    });
    unsafe { obj::set_native_updater_string_rep(original, payload(role), false) };
}
extern "C" fn free(original: *mut TclObj) {
    let role = usize::try_from(obj::internal_rep(original)).unwrap();
    // An interpreter left in OWNER is dropped while this thread's locals
    // are being destroyed; its objects then retire with nothing to record.
    if ORDER.try_with(|rows| rows.borrow_mut().push(role)).is_err() {
        return;
    }
    let (Ok(retire_role), Ok(retire)) =
        (RETIRE_ROLE.try_with(Cell::get), RETIRE.try_with(Cell::get))
    else {
        return;
    };
    if role == retire_role && retire != 0 {
        let Ok(owner) = OWNER.try_with(|owner| owner.borrow().clone()) else {
            return;
        };
        let mut interp = owner.expect("retirement owner interpreter");
        // A real key destructor runs outside the Namespace borrow. The
        // original entry is already unlinked; later slot owners are distinct.
        assert!(interp.resolve_cmd_token(b"aliasKeyProbe").is_none());
        if role == 0 {
            assert_eq!(ORDER.with(|rows| rows.borrow().clone()), vec![0]);
        } else {
            // Replacement retires only the old value. Its occupied original
            // key remains owned even while the binding payload is absent.
            let key = interp
                .namespaces()
                .original_jim_command_key(crate::namespace::GLOBAL, b"aliasKeyProbe")
                .unwrap();
            assert_eq!(obj::bytes_of(key), b"aliasKeyProbe");
            assert_eq!(unsafe { (*key).ref_count }, 1);
        }
        match RETIRE.with(Cell::get) {
            1 => {
                interp.refuse_host_command("original Jim key destructor first cause");
                FIRST.with(|first| *first.borrow_mut() = interp.native_execution_refusal());
            }
            2 => {
                let original = interp.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("original-key-retirement".to_owned(), "1.0".to_owned())];
                interp.pin_context(&changed).unwrap();
                interp.pin_context(&original).unwrap();
            }
            3 => interp.register_builtin(b"aliasKeyProbe", replacement),
            _ => unreachable!(),
        }
    }
}
fn replacement(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"REPLACEMENT");
    Code::Ok
}
static TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalAliasKeyOperand".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn reset() {
    OWNER.with(|owner| owner.borrow_mut().take());
    RENAME.with(|rename| rename.set(false));
    RETIRE.with(|mode| mode.set(0));
    RETIRE_ROLE.with(|role| role.set(0));
    ORDER.with(|rows| rows.borrow_mut().clear());
    GETTERS.with(|rows| rows.borrow_mut().clear());
    FIRST.with(|first| first.borrow_mut().take());
}
fn originals() -> Vec<Owned> {
    (0..5)
        .map(|role| Owned::fresh(obj::alloc_typed(&TYPE, role)))
        .collect()
}
fn define(interp: &mut Interp, originals: &[Owned], name: usize, jim: bool) {
    let jim_words = [b"alias".as_slice()];
    let c_words = [b"interp".as_slice(), b"alias", b"", b"unused", b""];
    let words = super::original_operand_tests::words(if jim { &jim_words } else { &c_words });
    let mut argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
    if jim {
        argv.push(originals[name].as_ptr());
    } else {
        argv[3] = originals[name].as_ptr();
    }
    argv.extend(originals[1..4].iter().map(Owned::as_ptr));
    assert_eq!(
        if jim {
            super::jim_alias(interp, &argv)
        } else {
            super::interp_cmd(interp, &argv)
        },
        Code::Ok
    );
    interp.set_result_bytes(b"");
}
fn rows() -> [(&'static str, &'static str); 6] {
    [
            ("tcl8.4", include_str!("../../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/tcl8.4/execute.stdout")),
            ("tcl8.5", include_str!("../../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/tcl8.5/execute.stdout")),
            ("tcl8.6", include_str!("../../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/tcl8.6/execute.stdout")),
            ("tcl9.0", include_str!("../../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/tcl9.0/execute.stdout")),
            ("tcl9.1", include_str!("../../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/tcl9.1/execute.stdout")),
            ("jim", include_str!("../../tests/data/native_alias_command_key_replacement_child_capture_rename_original/421b/providers/jim/execute.stdout")),
        ]
}
fn expected(stream: &str, case: usize, label: &str, role: usize, field: &str) -> i64 {
    let prefix = format!("ROW|{case}|{label}|");
    let row = stream
        .lines()
        .find(|row| row.starts_with(&prefix))
        .expect("actual pinned observer stage");
    let key = format!("r{role}_{field}=");
    row.split('|')
        .find_map(|field| field.strip_prefix(&key))
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn original_alias_replacement_keeps_measured_name_key_and_query_roles() {
    // naming.alias.original-command-key-replacement-child-capture-and-rename
    // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
    // Only observed name refs and query header/member roles are compared;
    // no Jim body-only updater refcount or general frame claim is inferred.
    for (version, stream) in rows() {
        reset();
        let jim = version == "jim";
        let mut interp = super::original_operand_tests::native(version);
        let originals = originals();
        define(&mut interp, &originals, 0, jim);
        let first_query = interp.alias_query_object(payload(0)).unwrap().unwrap();
        define(&mut interp, &originals, 4, jim);
        for role in [0, 4] {
            assert_eq!(
                unsafe { (*originals[role].as_ptr()).ref_count } as i64,
                expected(
                    stream,
                    7,
                    "replacement-name-after-result-clear",
                    role,
                    "refs"
                ),
                "{version} role {role}"
            );
        }
        let current_query = interp.alias_query_object(payload(0)).unwrap().unwrap();
        assert_ne!(first_query.as_ptr(), current_query.as_ptr());
        let selected = interp.alias_query_object(payload(0)).unwrap().unwrap();
        assert_eq!(selected.as_ptr() == current_query.as_ptr(), jim);
        let members =
            tcl_syntax::value::ValueOps::list_elements(&mut interp, &current_query.as_ptr())
                .unwrap();
        assert_eq!(
            members,
            originals[1..4]
                .iter()
                .map(Owned::as_ptr)
                .collect::<Vec<_>>()
        );
        if jim {
            assert_eq!(
                interp
                    .namespaces()
                    .original_jim_command_key(crate::namespace::GLOBAL, payload(0)),
                Some(originals[0].as_ptr())
            );
        }
        assert!(interp.delete_command(payload(0)));
        for role in [0, 4] {
            assert_eq!(unsafe { (*originals[role].as_ptr()).ref_count }, 1);
        }
        assert_eq!(
            tcl_syntax::value::ValueOps::list_elements(&mut interp, &first_query.as_ptr()).unwrap(),
            members
        );
        assert!(interp.native_execution_refusal().is_none());
    }
}

#[test]
fn original_alias_rename_keeps_measured_record_and_destination_key_roles() {
    // naming.alias.original-command-key-replacement-child-capture-and-rename
    // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
    for (version, stream) in rows() {
        reset();
        RENAME.with(|rename| rename.set(true));
        let jim = version == "jim";
        let mut interp = super::original_operand_tests::native(version);
        let originals = originals();
        define(&mut interp, &originals, 0, jim);
        let words = super::original_operand_tests::words(&[b"rename"]);
        assert_eq!(
            super::rename(
                &mut interp,
                &[
                    words[0].as_ptr(),
                    originals[0].as_ptr(),
                    originals[4].as_ptr()
                ]
            ),
            Code::Ok
        );
        for role in [0, 4] {
            assert_eq!(
                unsafe { (*originals[role].as_ptr()).ref_count } as i64,
                expected(stream, 10, "rename-name-after-result-clear", role, "refs"),
                "{version} role {role}"
            );
        }
        if jim {
            assert_eq!(
                interp
                    .namespaces()
                    .original_jim_command_key(crate::namespace::GLOBAL, payload(4)),
                Some(originals[4].as_ptr())
            );
            assert!(interp
                .namespaces()
                .original_jim_command_key(crate::namespace::GLOBAL, payload(0))
                .is_none());
        }
        assert!(interp.delete_command(payload(4)));
        for role in [0, 4] {
            assert_eq!(unsafe { (*originals[role].as_ptr()).ref_count }, 1);
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_jim_key_retirement_runs_before_value_cleanup_without_adopting_replacement() {
    // naming.alias.original-command-key-replacement-child-capture-and-rename
    // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
    // Real destructor callbacks are software controls of cleanup/reentry.
    // Native421b did not execute these Host/context callback modes.
    for mode in [1, 2, 3] {
        reset();
        let mut interp = super::original_operand_tests::native("jim");
        let originals = originals();
        define(&mut interp, &originals, 0, true);
        let prior = super::original_operand_tests::seed(&mut interp);
        drop(originals);
        ORDER.with(|rows| rows.borrow_mut().clear());
        OWNER.with(|owner| *owner.borrow_mut() = Some(interp.clone()));
        RETIRE.with(|selected| selected.set(mode));
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interp).unwrap();
        assert!(interp.delete_command(b"aliasKeyProbe"));
        let order = ORDER.with(|rows| rows.borrow().clone());
        assert_eq!(order.first(), Some(&0));
        let mut members = order[1..].to_vec();
        members.sort_unstable();
        assert_eq!(members, vec![1, 2, 3]);
        if mode == 3 {
            assert!(scope.currency().ensure_current_or_refuse().is_ok());
            assert!(interp.resolve_cmd_token(b"aliasKeyProbe").is_some());
            assert_ne!(
                interp
                    .namespaces()
                    .original_jim_command_key(crate::namespace::GLOBAL, b"aliasKeyProbe"),
                None
            );
        } else {
            assert!(scope.currency().ensure_current_or_refuse().is_err());
            if mode == 1 {
                assert_eq!(
                    interp.native_execution_refusal(),
                    FIRST.with(|first| first.borrow().clone())
                );
            }
            assert_eq!(interp.get_obj_result(), prior.as_ptr());
        }
        drop(scope);
        RETIRE.with(|selected| selected.set(0));
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_jim_replacement_refusal_keeps_occupied_key_and_reached_name_result() {
    // naming.alias.original-command-key-replacement-child-capture-and-rename
    // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
    // These real value-destructor Host/context modes are software controls;
    // the original421b probe did not execute these refusal callbacks.
    for mode in [1, 2] {
        reset();
        let mut interp = super::original_operand_tests::native("jim");
        let originals = originals();
        define(&mut interp, &originals, 0, true);
        drop(originals);
        ORDER.with(|rows| rows.borrow_mut().clear());
        OWNER.with(|owner| *owner.borrow_mut() = Some(interp.clone()));
        RETIRE_ROLE.with(|role| role.set(1));
        RETIRE.with(|selected| selected.set(mode));
        let name = Owned::fresh(obj::alloc_typed(&TYPE, 4));
        let words = super::original_operand_tests::words(&[b"alias", b"list", b"replacement"]);
        let code = super::jim_alias(
            &mut interp,
            &[
                words[0].as_ptr(),
                name.as_ptr(),
                words[1].as_ptr(),
                words[2].as_ptr(),
            ],
        );
        assert_eq!(code, Code::Error);
        assert_eq!(
            interp.get_obj_result(),
            name.as_ptr(),
            "Jim's original name result was reached before replacement"
        );
        assert_eq!(
            unsafe { (*name.as_ptr()).ref_count },
            2,
            "caller plus reached result; no failed incoming table-key role"
        );
        assert!(interp.native_execution_refusal().is_some());
        if mode == 1 {
            assert_eq!(
                interp.native_execution_refusal(),
                FIRST.with(|first| first.borrow().clone())
            );
        }
        assert!(interp
            .namespaces()
            .command_in(crate::namespace::GLOBAL, b"aliasKeyProbe")
            .is_none());
        assert!(interp
            .namespaces()
            .original_jim_command_key(crate::namespace::GLOBAL, b"aliasKeyProbe")
            .is_some());
        RETIRE.with(|selected| selected.set(0));
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}

#[test]
fn original_jim_rename_qualification_and_table_key_keep_distinct_native_references() {
    // naming.alias.original-command-key-replacement-child-capture-and-rename
    // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
    // The held temporary stage checks the pinned QualifyName/keyDup source
    // roles. Native421b's measured comparison is after both temporaries
    // have actually retired, rather than a claim of an observed callback.
    reset();
    let mut interp = super::original_operand_tests::native("jim");
    let originals = originals();
    define(&mut interp, &originals, 0, true);
    RENAME.with(|rename| rename.set(true));
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&interp).unwrap();
    let (old, new, old_key, new_key) = interp
        .original_jim_rename_operands(originals[0].as_ptr(), originals[4].as_ptr())
        .unwrap();
    assert_eq!(old_key.as_ptr(), originals[0].as_ptr());
    let new_key = new_key.unwrap();
    assert_eq!(new_key.as_ptr(), originals[4].as_ptr());
    assert_eq!(
        unsafe { (*old_key.as_ptr()).ref_count },
        3,
        "caller, original source table key and qualification temporary"
    );
    assert_eq!(
        unsafe { (*new_key.as_ptr()).ref_count },
        2,
        "caller and destination qualification temporary"
    );
    assert_eq!(
        interp.rename_command_with_original_jim_key(&old, &new, Some(std::rc::Rc::clone(&new_key))),
        RenameOutcome::Renamed
    );
    assert_eq!(
        unsafe { (*old_key.as_ptr()).ref_count },
        2,
        "retired source table role leaves caller plus qualification temporary"
    );
    assert_eq!(
        unsafe { (*new_key.as_ptr()).ref_count },
        3,
        "fresh table keyDup is independent of caller and qualification temporary"
    );
    drop(old_key);
    drop(new_key);
    let stream = rows()
        .into_iter()
        .find(|(provider, _)| *provider == "jim")
        .unwrap()
        .1;
    for role in [0, 4] {
        assert_eq!(
            unsafe { (*originals[role].as_ptr()).ref_count } as i64,
            expected(stream, 10, "rename-name-after-result-clear", role, "refs")
        );
    }
    assert!(scope.currency().ensure_current_or_refuse().is_ok());
}
