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

use crate::interp::{Code, Interp};
use crate::obj::{self, Owned, TclObj};
use std::cell::{Cell, RefCell};

thread_local! {
    static NAMESPACE: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static PAYLOAD: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static GETTER_OBSERVATIONS: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
}
extern "C" fn update_argument(original: *mut TclObj) {
    GETTER_OBSERVATIONS.with(|observed| {
        observed
            .borrow_mut()
            .push(NAMESPACE.with(|namespace| obj::has_string_rep(namespace.get())))
    });
    PAYLOAD.with(|payload| {
        // SAFETY: this actual updater owns the original object's absent String slot.
        unsafe { obj::set_string_rep(original, &payload.borrow()) };
    });
}
static ARGUMENT: obj::TclObjType = obj::TclObjType {
    name: c"origin-argument-probe".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_argument),
    set_from_any_proc: None,
};
fn native_jim() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn type_name(original: *mut TclObj) -> Vec<u8> {
    let kind = obj::obj_type_ptr(original);
    if kind.is_null() {
        return b"none".to_vec();
    }
    // SAFETY: this retained original's actual static descriptor owns its terminated name.
    unsafe { std::ffi::CStr::from_ptr((*kind).name) }
        .to_bytes()
        .to_vec()
}

#[test]
fn original_jim_namespace_origin_keeps_reached_helper_getters_and_original_argument_identity() {
    let evidence =
        include_str!("../../tests/data/native_jim_namespace_origin/getter-identity-20261015.tsv");
    assert_eq!(evidence.lines().count(), 293);
    let row = |kind: &str, phase: &str| {
        let mut matches = evidence
            .lines()
            .map(|line| line.split('|').collect::<Vec<_>>())
            .filter(|fields| fields.first() == Some(&kind) && fields.get(1) == Some(&phase));
        let selected = matches.next().expect("genuine original SDK row");
        assert!(matches.next().is_none(), "unique original SDK row");
        selected
    };
    for (label, payload, direct, count, change) in [
        ("PUBLIC_RELATIVE", b"target".as_slice(), false, 1, 0),
        ("PUBLIC_ABSOLUTE", b"::target".as_slice(), false, 1, 0),
        ("PUBLIC_ALIAS", b"alias_target".as_slice(), false, 1, 0),
        ("PUBLIC_CHAIN", b"alias_chain".as_slice(), false, 1, 0),
        ("PUBLIC_NUL", b"target\0TAIL".as_slice(), false, 1, 0),
        ("PUBLIC_MISSING", b"missing".as_slice(), false, 1, 0),
        ("PUBLIC_EMPTY", b"".as_slice(), false, 1, 0),
        ("DIRECT_RELATIVE", b"target".as_slice(), true, 1, 0),
        ("DIRECT_ABSOLUTE", b"::target".as_slice(), true, 1, 0),
        ("DIRECT_ALIAS", b"alias_target".as_slice(), true, 1, 0),
        ("DIRECT_CHAIN", b"alias_chain".as_slice(), true, 1, 0),
        ("DIRECT_NUL", b"target\0TAIL".as_slice(), true, 1, 0),
        ("DIRECT_MISSING", b"missing".as_slice(), true, 1, 0),
        ("DIRECT_EMPTY", b"".as_slice(), true, 1, 0),
        ("PUBLIC_ARITY_ZERO", b"target".as_slice(), false, 0, 0),
        ("PUBLIC_ARITY_TWO", b"target".as_slice(), false, 2, 0),
        ("PUBLIC_REPLACED", b"target".as_slice(), false, 1, 1),
        ("PUBLIC_DELETED", b"target".as_slice(), false, 1, 2),
        ("DIRECT_ARITY_ZERO", b"target".as_slice(), true, 0, 0),
        ("DIRECT_ARITY_TWO", b"target".as_slice(), true, 2, 0),
        ("DIRECT_REPLACED", b"target".as_slice(), true, 1, 1),
        ("DIRECT_DELETED", b"target".as_slice(), true, 1, 2),
    ] {
        let mut interp = native_jim();
        assert_eq!(interp.eval_str(b"proc 42::target {} {return VALUE}; proc target {} {return GLOBAL}; alias 42::alias_target 42::target; alias 42::alias_chain 42::alias_target"), Code::Ok);
        if change != 0 {
            assert_eq!(
                interp.eval_str(if change == 1 {
                    b"rename {namespace origin} saved; proc {namespace origin} arg {return REPLACED}"
                } else {
                    b"rename {namespace origin} {}"
                }),
                Code::Ok
            );
        }
        let context = interp.native_jim_object_context().unwrap();
        let namespace = Owned::fresh(obj::new_wide_int_obj(42));
        crate::native_source::bind_context(namespace.as_ptr(), &context).unwrap();
        let head = Owned::fresh(obj::new_string_bytes(if direct {
            b"namespace origin"
        } else {
            b"namespace"
        }));
        let member = Owned::fresh(obj::new_string_bytes(b"origin"));
        crate::native_source::bind_context(head.as_ptr(), &context).unwrap();
        NAMESPACE.with(|slot| slot.set(namespace.as_ptr()));
        PAYLOAD.with(|slot| *slot.borrow_mut() = payload.to_vec());
        for hot in [false, true] {
            let phase = format!("{label}_{}", if hot { "HOT" } else { "COLD" });
            if hot {
                if change != 2 {
                    assert_eq!(
                        obj::with_jim_command_cache(head.as_ptr(), |cache| cache
                            .namespace
                            .as_ptr()),
                        Some(namespace.as_ptr())
                    );
                }
                obj::invalidate_string(namespace.as_ptr());
            }
            // Keep the real head cache and fresh original payload/List, exactly
            // as the independent Origin probe does for its hot invocation.
            let argument = Owned::fresh(obj::alloc_typed(&ARGUMENT, 0));
            crate::native_source::bind_context(argument.as_ptr(), &context).unwrap();
            let mut words = vec![head.as_ptr()];
            if !direct {
                words.push(member.as_ptr());
            }
            words.extend(std::iter::repeat_n(argument.as_ptr(), count));
            let body = Owned::fresh(interp.new_list_object(&words));
            let current = interp
                .namespaces_mut()
                .retain_unmaterialized_jim_namespace(Owned::retain(namespace.as_ptr()));
            assert_eq!(
                obj::has_string_rep(namespace.as_ptr()),
                row("NAMESPACE_BEFORE", &phase)[2] == "1",
                "{phase}"
            );
            assert_eq!(
                type_name(namespace.as_ptr()),
                row("NAMESPACE_BEFORE", &phase)[3].as_bytes(),
                "{phase}"
            );
            assert_eq!(
                obj::has_string_rep(argument.as_ptr()),
                row("ARG_BEFORE", &phase)[2] == "1",
                "{phase}"
            );
            assert_eq!(
                type_name(argument.as_ptr()),
                row("ARG_BEFORE", &phase)[3].as_bytes(),
                "{phase}"
            );
            assert!(interp.namespaces().jim_namespace_bytes(current).is_none());
            GETTER_OBSERVATIONS.with(|observed| observed.borrow_mut().clear());
            let code = interp.ns_eval_in_token(
                current,
                &[],
                None,
                false,
                Some((
                    tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
                    body.as_ptr(),
                )),
                None,
            );
            let observed = GETTER_OBSERVATIONS.with(|observed| observed.borrow().clone());
            let expected: Vec<_> = evidence
                .lines()
                .map(|line| line.split('|').collect::<Vec<_>>())
                .filter(|fields| {
                    fields.first() == Some(&"NAMESPACE_AT_ARG_GETTER")
                        && fields.get(1) == Some(&phase.as_str())
                })
                .map(|fields| fields[2] == "1")
                .collect();
            assert_eq!(observed, expected, "{phase}");
            assert_eq!(
                obj::has_string_rep(namespace.as_ptr()),
                row("NAMESPACE_AFTER", &phase)[2] == "1",
                "{phase}"
            );
            assert_eq!(
                type_name(namespace.as_ptr()),
                row("NAMESPACE_AFTER", &phase)[3].as_bytes(),
                "{phase}"
            );
            assert_eq!(
                obj::has_string_rep(argument.as_ptr()),
                row("ARG_AFTER", &phase)[2] == "1",
                "{phase}"
            );
            assert_eq!(
                type_name(argument.as_ptr()),
                row("ARG_AFTER", &phase)[3].as_bytes(),
                "{phase}"
            );
            assert_eq!(
                interp.result_obj() == argument.as_ptr(),
                row("RESULT_IS_ARG", &phase)[2] == "1",
                "{phase}"
            );
            let result = row("RESULT", &phase);
            assert_eq!(code.as_int().to_string(), result[2], "{phase}");
            let bytes: Vec<_> = result[3]
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(interp.result_bytes(), bytes, "{phase}");
            assert!(interp.native_execution_refusal().is_none(), "{phase}");
        }
        interp.set_current_ns(crate::namespace::GLOBAL);
        NAMESPACE.with(|slot| slot.set(std::ptr::null_mut()));
    }
}

#[test]
fn jim_namespace_origin_keeps_actual_helper_publication_through_refresh_and_replacement() {
    let mut interp = native_jim();
    let original = interp
        .resolve_cmd_token(b"namespace origin")
        .expect("actual helper generation");
    assert!(matches!(
        interp.namespaces().command_by_generation(original),
        Some((_, crate::interp::Command::Proc(_)))
    ));
    assert!(
        interp.resolve_cmd_token(b"namespace which").is_some(),
        "independent existing which helper remains installed"
    );
    assert_eq!(
        interp.eval_str(
            b"rename {namespace origin} saved; proc {namespace origin} arg {return REPLACED}"
        ),
        Code::Ok
    );
    let replacement = interp.resolve_cmd_token(b"namespace origin").unwrap();
    assert_ne!(original, replacement);
    assert_eq!(interp.resolve_cmd_token(b"saved"), Some(original));
    interp.install_stock_jim_namespace_helpers();
    assert_eq!(
        interp.resolve_cmd_token(b"namespace origin"),
        Some(replacement)
    );
    interp.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
    interp.set_dialect_profile(crate::environment::profile_for_dialect("jimtcl"));
    assert_eq!(
        interp.resolve_cmd_token(b"namespace origin"),
        Some(replacement)
    );
    assert_eq!(interp.eval_str(b"namespace origin VALUE"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"REPLACED");
    assert_eq!(
        interp.eval_str(b"rename {namespace origin} {}; namespace origin VALUE"),
        Code::Error
    );
    assert!(interp.resolve_cmd_token(b"namespace origin").is_none());
    assert!(interp.native_execution_refusal().is_none());
    let descriptor = tcl_registry::InvocationDialect::of_profile(
        crate::environment::profile_for_dialect("jimtcl"),
    )
    .native_jim_namespace_helper(
        tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Origin,
    )
    .unwrap();
    let mut foreign = Interp::new();
    let head = Owned::fresh(obj::new_string_bytes(b"namespace"));
    let member = Owned::fresh(obj::new_string_bytes(b"origin"));
    let argument = Owned::fresh(obj::alloc_typed(&ARGUMENT, 0));
    GETTER_OBSERVATIONS.with(|observed| observed.borrow_mut().clear());
    assert_eq!(
        foreign.dispatch_stock_jim_namespace_helper(
            &[head.as_ptr(), member.as_ptr(), argument.as_ptr()],
            descriptor,
        ),
        Code::Error
    );
    assert!(foreign.native_execution_refusal().is_some());
    assert!(!obj::has_string_rep(argument.as_ptr()));
    assert!(GETTER_OBSERVATIONS.with(|observed| observed.borrow().is_empty()));
    assert!(foreign.resolve_cmd_token(b"namespace origin").is_none());
}
