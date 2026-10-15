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

use crate::interp::{Code, Command, Interp};
use crate::obj::{self, Owned, TclObj};
use std::cell::RefCell;

#[derive(Default)]
struct Invocation {
    pointers: [*mut TclObj; 5],
    worker_return: bool,
    rows: Vec<(String, Vec<String>)>,
}
thread_local! {static INVOCATION: RefCell<Invocation> = RefCell::new(Invocation::default());}
fn type_name(original: *mut TclObj) -> String {
    let kind = obj::obj_type_ptr(original);
    if kind.is_null() {
        return "none".to_owned();
    }
    // SAFETY: the retained actual object's static descriptor owns its terminated name.
    unsafe { std::ffi::CStr::from_ptr((*kind).name) }
        .to_str()
        .unwrap()
        .to_owned()
}
fn header(original: *mut TclObj) -> Vec<String> {
    vec![
        u8::from(obj::has_string_rep(original)).to_string(),
        type_name(original),
    ]
}
fn observe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    INVOCATION.with(|slot| {
        let mut state = slot.borrow_mut();
        let [caller, target, body, token, extra] = state.pointers;
        state.rows.push((
            "INVOKE".to_owned(),
            vec![
                argv.len().to_string(),
                u8::from(argv.get(1) == Some(&token)).to_string(),
                u8::from(argv.get(2) == Some(&extra)).to_string(),
            ],
        ));
        for (kind, original) in [
            ("CALLER_INVOKE", caller),
            ("TARGET_INVOKE", target),
            ("BODY_INVOKE", body),
            ("TOKEN_INVOKE", token),
            ("EXTRA_INVOKE", extra),
        ] {
            state.rows.push((kind.to_owned(), header(original)));
        }
        let current = interp
            .namespaces()
            .jim_namespace_original_owner(interp.current_ns())
            .unwrap()
            .as_ptr();
        state.rows.push((
            "CURRENT_IS_TARGET".to_owned(),
            vec![u8::from(current == target).to_string()],
        ));
        interp.set_result(token);
        if state.worker_return {
            Code::Return
        } else {
            Code::Ok
        }
    })
}
fn native_jim() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

#[test]
fn original_jim_namespace_inscope_keeps_selected_tailcall_and_original_list_elements() {
    let evidence =
        include_str!("../../tests/data/native_jim_namespace_inscope/getter-identity-20261015.tsv");
    assert_eq!(evidence.lines().count(), 421);
    let row = |kind: &str, phase: &str| {
        let mut rows = evidence
            .lines()
            .map(|line| line.split('|').collect::<Vec<_>>())
            .filter(|fields| fields.first() == Some(&kind) && fields.get(1) == Some(&phase));
        let selected = rows.next().expect("actual original inscope row");
        assert!(rows.next().is_none());
        selected[2..]
            .iter()
            .map(|field| (*field).to_owned())
            .collect::<Vec<_>>()
    };
    for (label, direct, count, change, worker_return) in [
        ("PUBLIC_LIST", false, 2, 0, false),
        ("PUBLIC_EXTRA", false, 3, 0, false),
        ("DIRECT_LIST", true, 2, 0, false),
        ("DIRECT_EXTRA", true, 3, 0, false),
        ("PUBLIC_RETURN", false, 2, 0, true),
        ("DIRECT_RETURN", true, 2, 0, true),
        ("PUBLIC_ARITY_ZERO", false, 0, 0, false),
        ("PUBLIC_ARITY_ONE", false, 1, 0, false),
        ("DIRECT_ARITY_ZERO", true, 0, 0, false),
        ("DIRECT_ARITY_ONE", true, 1, 0, false),
        ("PUBLIC_REPLACED", false, 2, 1, false),
        ("DIRECT_REPLACED", true, 2, 1, false),
        ("PUBLIC_DELETED", false, 2, 2, false),
        ("DIRECT_DELETED", true, 2, 2, false),
    ] {
        let mut interp = native_jim();
        interp.ns_register(b"observe", Command::Builtin(observe));
        if change != 0 {
            assert_eq!(interp.eval_str(if change==1 {b"rename {namespace inscope} saved; proc {namespace inscope} {name arg args} {return REPLACED}"} else {b"rename {namespace inscope} {}"}),Code::Ok);
        }
        let context = interp.native_jim_object_context().unwrap();
        let caller = Owned::fresh(obj::new_wide_int_obj(42));
        let target = Owned::fresh(obj::new_wide_int_obj(84));
        let head = Owned::fresh(obj::new_string_bytes(if direct {
            b"namespace inscope"
        } else {
            b"namespace"
        }));
        let member = Owned::fresh(obj::new_string_bytes(b"inscope"));
        for object in [
            caller.as_ptr(),
            target.as_ptr(),
            head.as_ptr(),
            member.as_ptr(),
        ] {
            crate::native_source::bind_context(object, &context).unwrap();
        }
        for hot in [false, true] {
            let phase = format!("{label}_{}", if hot { "HOT" } else { "COLD" });
            if hot {
                obj::invalidate_string(caller.as_ptr());
                obj::invalidate_string(target.as_ptr());
            }
            let token = Owned::fresh(obj::new_wide_int_obj(123));
            let extra = Owned::fresh(obj::new_wide_int_obj(456));
            let command = Owned::fresh(obj::new_string_bytes(b"observe"));
            for object in [token.as_ptr(), extra.as_ptr(), command.as_ptr()] {
                crate::native_source::bind_context(object, &context).unwrap();
            }
            let body = Owned::fresh(interp.new_list_object(&[command.as_ptr(), token.as_ptr()]));
            let mut words = vec![head.as_ptr()];
            if !direct {
                words.push(member.as_ptr());
            }
            words.extend(
                [target.as_ptr(), body.as_ptr(), extra.as_ptr()]
                    .into_iter()
                    .take(count),
            );
            let vector = Owned::fresh(interp.new_list_object(&words));
            let pointers = [
                caller.as_ptr(),
                target.as_ptr(),
                body.as_ptr(),
                token.as_ptr(),
                extra.as_ptr(),
            ];
            let current = interp
                .namespaces_mut()
                .retain_unmaterialized_jim_namespace(Owned::retain(caller.as_ptr()));
            for (kind, original) in ["CALLER", "TARGET", "BODY", "TOKEN", "EXTRA"]
                .into_iter()
                .zip(pointers)
            {
                assert_eq!(
                    header(original),
                    row(&format!("{kind}_BEFORE"), &phase),
                    "{phase}/{kind}"
                );
            }
            INVOCATION.with(|slot| {
                *slot.borrow_mut() = Invocation {
                    pointers,
                    worker_return,
                    rows: Vec::new(),
                }
            });
            let code = interp.ns_eval_in_token(
                current,
                &[],
                None,
                false,
                Some((
                    tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
                    vector.as_ptr(),
                )),
                None,
            );
            let observed = INVOCATION.with(|slot| std::mem::take(&mut slot.borrow_mut().rows));
            let expected = evidence
                .lines()
                .map(|line| line.split('|').collect::<Vec<_>>())
                .filter(|fields| {
                    fields.get(1) == Some(&phase.as_str())
                        && (fields[0] == "INVOKE"
                            || fields[0].ends_with("_INVOKE")
                            || fields[0] == "CURRENT_IS_TARGET")
                })
                .map(|fields| {
                    (
                        fields[0].to_owned(),
                        fields[2..]
                            .iter()
                            .map(|field| (*field).to_owned())
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(observed, expected, "{phase}");
            for (kind, original) in ["CALLER", "TARGET", "BODY", "TOKEN", "EXTRA"]
                .into_iter()
                .zip(pointers)
            {
                assert_eq!(
                    header(original),
                    row(&format!("{kind}_AFTER"), &phase),
                    "{phase}/{kind}"
                );
            }
            assert_eq!(
                u8::from(interp.result_obj() == token.as_ptr()).to_string(),
                row("RESULT_IS_TOKEN", &phase)[0],
                "{phase}"
            );
            let result = row("RESULT", &phase);
            assert_eq!(code.as_int().to_string(), result[0], "{phase}");
            let bytes = result[1]
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(interp.result_bytes(), bytes, "{phase}");
            assert!(interp.native_execution_refusal().is_none(), "{phase}");
        }
        interp.set_current_ns(crate::namespace::GLOBAL);
        INVOCATION.with(|slot| *slot.borrow_mut() = Invocation::default());
    }
}

#[test]
fn jim_namespace_inscope_keeps_actual_helper_publication_through_refresh_and_replacement() {
    let mut interp = native_jim();
    let original = interp.resolve_cmd_token(b"namespace inscope").unwrap();
    assert!(matches!(
        interp.namespaces().command_by_generation(original),
        Some((_, Command::Proc(_)))
    ));
    for name in [
        b"namespace which".as_slice(),
        b"namespace code",
        b"namespace origin",
    ] {
        assert!(interp.resolve_cmd_token(name).is_some());
    }
    assert_eq!(interp.eval_str(b"rename {namespace inscope} saved; proc {namespace inscope} {name arg args} {return REPLACED}"),Code::Ok);
    let replacement = interp.resolve_cmd_token(b"namespace inscope").unwrap();
    assert_ne!(original, replacement);
    assert_eq!(interp.resolve_cmd_token(b"saved"), Some(original));
    interp.install_stock_jim_namespace_helpers();
    assert_eq!(
        interp.resolve_cmd_token(b"namespace inscope"),
        Some(replacement)
    );
    interp.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
    interp.set_dialect_profile(crate::environment::profile_for_dialect("jimtcl"));
    assert_eq!(
        interp.resolve_cmd_token(b"namespace inscope"),
        Some(replacement)
    );
    assert_eq!(interp.eval_str(b"namespace inscope :: VALUE"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"REPLACED");
    assert_eq!(
        interp.eval_str(b"rename {namespace inscope} {}; namespace inscope :: VALUE"),
        Code::Error
    );
    assert!(interp.resolve_cmd_token(b"namespace inscope").is_none());
    assert!(interp.native_execution_refusal().is_none());
    let descriptor = tcl_registry::InvocationDialect::of_profile(
        crate::environment::profile_for_dialect("jimtcl"),
    )
    .native_jim_namespace_helper(
        tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Inscope,
    )
    .unwrap();
    let mut foreign = Interp::new();
    let head = Owned::fresh(obj::new_string_bytes(b"namespace"));
    let member = Owned::fresh(obj::new_string_bytes(b"inscope"));
    let target = Owned::fresh(obj::new_wide_int_obj(84));
    let arg = Owned::fresh(obj::new_wide_int_obj(123));
    assert_eq!(
        foreign.dispatch_stock_jim_namespace_helper(
            &[
                head.as_ptr(),
                member.as_ptr(),
                target.as_ptr(),
                arg.as_ptr()
            ],
            descriptor
        ),
        Code::Error
    );
    assert!(foreign.native_execution_refusal().is_some());
    assert!(!obj::has_string_rep(target.as_ptr()));
    assert!(!obj::has_string_rep(arg.as_ptr()));
    assert!(foreign.resolve_cmd_token(b"namespace inscope").is_none());
}
