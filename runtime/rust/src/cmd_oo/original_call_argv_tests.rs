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

use crate::{
    interp::{Code, Interp},
    list, obj,
};
use std::cell::RefCell;
thread_local! { static OBSERVED: RefCell<Vec<Vec<usize>>> = const { RefCell::new(Vec::new()) }; }
thread_local! { static OBSERVED_REFS: RefCell<Vec<isize>> = const { RefCell::new(Vec::new()) }; }
fn native_detail(version: &str, purpose: &str, stage: &str, field: &str) -> usize {
    let release = match version {
        "tcl8.6" => "8.6.18",
        "tcl9.0" => "9.0.4",
        "tcl9.1" => "9.1.0",
        _ => unreachable!(),
    };
    include_str!("../../tests/data/native_call_argv/observations.tsv")
        .lines()
        .find_map(|row| {
            let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
            if fields.first() == Some(&release)
                && fields.get(1) == Some(&purpose)
                && fields.get(2) == Some(&stage)
            {
                fields[3..]
                    .iter()
                    .find_map(|value| value.strip_prefix(field))
                    .map(|value| value.parse().unwrap())
            } else {
                None
            }
        })
        .expect("exact native original argv fixture")
}
fn observe(interp: &mut Interp, _argv: &[*mut obj::TclObj]) -> Code {
    let original = interp
        .frames
        .borrow()
        .original_error_stack_argv()
        .expect("actual original OO invocation");
    OBSERVED.with(|observed| {
        observed
            .borrow_mut()
            .push(original.iter().map(|value| *value as usize).collect())
    });
    OBSERVED_REFS.with(|refs| {
        refs.borrow_mut().push(
            original
                .last()
                .map_or(0, |value| unsafe { (**value).ref_count }),
        )
    });
    Code::Ok
}
fn instance(version: &str, setup: &[u8]) -> Interp {
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .unwrap();
    interp.register_builtin(b"observe_original", observe);
    assert_eq!(
        interp.eval_str(setup),
        Code::Ok,
        "{}",
        String::from_utf8_lossy(&interp.result_bytes())
    );
    interp
}
fn original_argument(interp: &Interp) -> obj::Owned {
    let member = obj::Owned::fresh(obj::new_string_bytes(b"ORIGINAL"));
    obj::Owned::fresh(list::new_list_obj_native(
        &[member.as_ptr()],
        interp.invocation_string_protocol().unwrap(),
    ))
}
#[test]
fn method_next_call_lists_retain_same_original_native_argument_objects() {
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for next in [b"next $x".as_slice(), b"nextto Base $x"] {
            let mut script=b"oo::class create Base {method m {x} {observe_original; error FAILURE}}; oo::class create Derived {superclass Base; method m {x} {".to_vec();
            script.extend_from_slice(next);
            script.extend_from_slice(b"}}; Derived create o");
            let mut interp = instance(version, &script);
            let argument = original_argument(&interp);
            let head = obj::Owned::fresh(obj::new_string_bytes(b"o"));
            let method = obj::Owned::fresh(obj::new_string_bytes(b"m"));
            OBSERVED.with(|observed| observed.borrow_mut().clear());
            OBSERVED_REFS.with(|refs| refs.borrow_mut().clear());
            assert_eq!(
                interp.dispatch(&[head.as_ptr(), method.as_ptr(), argument.as_ptr()]),
                Code::Error
            );
            OBSERVED.with(|observed| {
                let observed = observed.borrow();
                assert_eq!(observed.len(), 1);
                assert_eq!(
                    observed[0].len(),
                    if next.starts_with(b"nextto") { 3 } else { 2 }
                );
                assert_eq!(observed[0].last(), Some(&(argument.as_ptr() as usize)));
            });
            let stack = interp.original_error_stack_value();
            let protocol = interp.invocation_string_protocol().unwrap();
            let flat = list::list_elements_native_checked(stack.as_ptr(), protocol).unwrap();
            let calls: Vec<_> = flat
                .chunks_exact(2)
                .filter(|pair| obj::bytes_of(pair[0]) == b"CALL")
                .map(|pair| list::list_elements_native_checked(pair[1], protocol).unwrap())
                .collect();
            assert_eq!(calls.len(), 2);
            assert!(calls
                .iter()
                .all(|call| call.last() == Some(&argument.as_ptr())));
            assert_eq!(
                calls.last().unwrap(),
                &vec![head.as_ptr(), method.as_ptr(), argument.as_ptr()]
            );
            let purpose = if next.starts_with(b"nextto") {
                "nextto"
            } else {
                "next"
            };
            OBSERVED_REFS.with(|refs| {
                assert_eq!(
                    refs.borrow().as_slice(),
                    &[native_detail(
                        version,
                        purpose,
                        "frame",
                        if purpose == "nextto" {
                            "refs2="
                        } else {
                            "refs1="
                        }
                    ) as isize]
                )
            });
            assert_eq!(
                unsafe { (*argument.as_ptr()).ref_count } as usize,
                native_detail(version, purpose, "after", "argRefs=")
            );
        }
    }
}
#[test]
fn constructor_call_frame_borrows_the_original_create_invocation() {
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = instance(
            version,
            b"oo::class create C {constructor {x} {observe_original; error FAILURE}}",
        );
        let argument = original_argument(&interp);
        let prefix: Vec<_> = [b"C".as_slice(), b"create", b"obj"]
            .iter()
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
            .collect();
        let mut argv: Vec<_> = prefix.iter().map(obj::Owned::as_ptr).collect();
        argv.push(argument.as_ptr());
        OBSERVED.with(|observed| observed.borrow_mut().clear());
        OBSERVED_REFS.with(|refs| refs.borrow_mut().clear());
        assert_eq!(interp.dispatch(&argv), Code::Error);
        OBSERVED.with(|observed| {
            assert_eq!(
                observed.borrow().as_slice(),
                &[argv.iter().map(|value| *value as usize).collect::<Vec<_>>()]
            )
        });
        let protocol = interp.invocation_string_protocol().unwrap();
        let stack = interp.original_error_stack_value();
        let flat = list::list_elements_native_checked(stack.as_ptr(), protocol).unwrap();
        let calls: Vec<_> = flat
            .chunks_exact(2)
            .filter(|pair| obj::bytes_of(pair[0]) == b"CALL")
            .map(|pair| list::list_elements_native_checked(pair[1], protocol).unwrap())
            .collect();
        assert_eq!(calls, vec![argv]);
        OBSERVED_REFS.with(|refs| {
            assert_eq!(
                refs.borrow().as_slice(),
                &[native_detail(version, "constructor", "frame", "refs3=") as isize]
            )
        });
        assert_eq!(
            unsafe { (*argument.as_ptr()).ref_count } as usize,
            native_detail(version, "constructor", "after", "argRefs=")
        );
    }
}
#[test]
fn native_destructor_frame_has_no_original_call_arguments_or_call_stack_entry() {
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = instance(
            version,
            b"oo::class create C {destructor {observe_original; error FAILURE}}; C create o",
        );
        let argv: Vec<_> = [b"o".as_slice(), b"destroy"]
            .iter()
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
            .collect();
        let pointers: Vec<_> = argv.iter().map(obj::Owned::as_ptr).collect();
        OBSERVED.with(|observed| observed.borrow_mut().clear());
        assert_eq!(interp.dispatch(&pointers), Code::Error);
        OBSERVED.with(|observed| {
            assert_eq!(observed.borrow().as_slice(), &[Vec::<usize>::new()]);
        });
        assert_eq!(native_detail(version, "destructor", "frame", "objc="), 0);
        let protocol = interp.invocation_string_protocol().unwrap();
        let stack = interp.original_error_stack_value();
        let parts = list::list_elements_native_checked(stack.as_ptr(), protocol).unwrap();
        assert!(!parts
            .chunks_exact(2)
            .any(|pair| obj::bytes_of(pair[0]) == b"CALL"));
    }
}
