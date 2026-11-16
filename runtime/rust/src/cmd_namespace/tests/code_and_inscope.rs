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

//! `namespace code` and `namespace inscope`: the script prefix they build and how tail arguments join it.

use super::*;

#[test]
fn current_and_code_wrong_arity_preserve_native_header_bytes() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        let command =
            crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"n s\0tail\xc0\x80\xff"));
        let member = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"current"));
        let extra = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"extra"));
        let expected: &[u8] = match version {
            tcl_dialect::TclVersion::V8_4 => b"n s",
            tcl_dialect::TclVersion::V8_5 | tcl_dialect::TclVersion::V8_6 => {
                b"n s\0tail\xc0\x80\xff"
            }
            tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1 => {
                b"{n s\0tail\xc0\x80\xff}"
            }
        };
        assert_eq!(
            super::super::ns_current(
                &mut interp,
                &[command.as_ptr(), member.as_ptr(), extra.as_ptr()]
            ),
            Code::Error
        );
        let mut message = b"wrong # args: should be \"".to_vec();
        message.extend_from_slice(expected);
        message.extend_from_slice(b" current\"");
        assert_eq!(interp.result_bytes(), message);
        assert_eq!(
            interp.error_code(),
            interp
                .native_invocation_dialect()
                .wrong_arguments_error_code()
                .unwrap()
                .as_bytes()
        );
        let member = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"code"));
        assert_eq!(
            super::super::ns_code(&mut interp, &[command.as_ptr(), member.as_ptr()]),
            Code::Error
        );
        let mut message = b"wrong # args: should be \"".to_vec();
        message.extend_from_slice(expected);
        message.extend_from_slice(b" code arg\"");
        assert_eq!(interp.result_bytes(), message);
        assert_eq!(
            crate::obj::bytes_of(command.as_ptr()),
            b"n s\0tail\xc0\x80\xff"
        );
    }
}

#[test]
fn code_preserves_original_argument_objects_for_all_native_prefix_fixtures() {
    let specimens: [&[u8]; 11] = [
        b"::namespace inscope ",
        b"::namespace inscope :: cmd",
        b"namespace inscope :: cmd",
        b":namespace inscope :: cmd",
        b"::::namespace inscope :: cmd",
        b"namespace    inscope :: cmd",
        b"namespace\tinscope :: cmd",
        b"namespaceinscopeXX",
        b"namespaceinscopeX",
        b"::namespace inscopeX",
        b"::namespace inscope \0",
    ];
    let jim = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    )));
    let mut rows = 0;
    for profile in tcl_dialect::TclVersion::ALL
        .map(|version| {
            tcl_dialect::DialectProfile::find(version.dialect_profile_name()).expect("C profile")
        })
        .into_iter()
        .chain(std::iter::once(&*jim))
    {
        let preserved: &[usize] = if std::ptr::eq(profile, &*jim) {
            &[0, 1, 10]
        } else if profile.vm_runtime_version == tcl_dialect::TclVersion::V8_4 {
            &[0, 1, 2, 3, 4, 5, 7, 9, 10]
        } else {
            &[1, 10]
        };
        leak_free(|interp| {
            interp.set_dialect_profile(profile);
            let namespace = crate::obj::Owned::fresh(crate::interp::new_string(b"namespace"));
            let code = crate::obj::Owned::fresh(crate::interp::new_string(b"code"));
            for (index, bytes) in specimens.into_iter().enumerate() {
                let script = crate::obj::Owned::fresh(crate::interp::new_string(bytes));
                assert_eq!(
                    super::super::ns_code(
                        interp,
                        &[namespace.as_ptr(), code.as_ptr(), script.as_ptr()]
                    ),
                    Code::Ok
                );
                if preserved.contains(&index) {
                    assert_eq!(
                        interp.result_obj(),
                        script.as_ptr(),
                        "{}: {index}",
                        profile.name
                    );
                } else {
                    let elements = crate::list::list_elements(interp.result_obj()).unwrap();
                    assert_eq!(elements.len(), 4);
                    assert_eq!(crate::interp::obj_bytes(elements[0]), b"::namespace");
                    assert_eq!(crate::interp::obj_bytes(elements[1]), b"inscope");
                    assert_eq!(crate::interp::obj_bytes(elements[2]), b"::");
                    assert_eq!(elements[3], script.as_ptr(), "{}: {index}", profile.name);
                }
                assert_eq!(crate::interp::obj_bytes(script.as_ptr()), bytes);
                rows += 1;
            }
        });
    }
    assert_eq!(rows, 66);
}

#[test]
fn code_retains_non_unicode_bytes_and_refuses_unknown_native_policy() {
    leak_free(|interp| {
        interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        let namespace = crate::obj::Owned::fresh(crate::interp::new_string(b"namespace"));
        let code = crate::obj::Owned::fresh(crate::interp::new_string(b"code"));
        for bytes in [
            b"\xff\0script".as_slice(),
            b"::namespace inscope \xff\0".as_slice(),
        ] {
            let script = crate::obj::Owned::fresh(crate::interp::new_string(bytes));
            assert_eq!(
                super::super::ns_code(
                    interp,
                    &[namespace.as_ptr(), code.as_ptr(), script.as_ptr()]
                ),
                Code::Ok
            );
            let retained = if interp.result_obj() == script.as_ptr() {
                interp.result_obj()
            } else {
                crate::list::list_elements(interp.result_obj()).unwrap()[3]
            };
            assert_eq!(retained, script.as_ptr());
            assert_eq!(crate::interp::obj_bytes(retained), bytes);
        }
        let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )));
        interp.set_dialect_profile(unknown);
        // Installing a profile performs no unavailable namespace-code
        // operation. The actual attempted purpose owns its typed refusal.
        assert_eq!(interp.native_access_refusal(), None);
        assert!(
                interp
                    .native_invocation_dialect()
                    .namespace_code_handler_policy(Some(
                        tcl_registry::native_namespace_code::LogicalNamespaceCodeProvider::Tcl84CoreSimulation,
                    ))
                    .is_none()
            );
        let script = crate::obj::Owned::fresh(crate::interp::new_string(b"puts example"));
        interp.set_result_bytes(b"UNCHANGED");
        assert_eq!(
            super::super::ns_code(
                interp,
                &[namespace.as_ptr(), code.as_ptr(), script.as_ptr()]
            ),
            Code::Error
        );
        assert_eq!(interp.result_bytes(), b"UNCHANGED");
        assert!(interp.host_refusal_pending());
        assert_eq!(
            interp.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "namespace code",
                )
            )
        );
    });
}

#[test]
fn code_uses_the_explicit_f5_logical_provider() {
    leak_free(|interp| {
        interp.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        let namespace = crate::obj::Owned::fresh(crate::interp::new_string(b"namespace"));
        let code = crate::obj::Owned::fresh(crate::interp::new_string(b"code"));
        let script = crate::obj::Owned::fresh(crate::interp::new_string(b"namespaceinscopeXX"));
        assert_eq!(
            super::super::ns_code(
                interp,
                &[namespace.as_ptr(), code.as_ptr(), script.as_ptr()]
            ),
            Code::Ok
        );
        assert_eq!(interp.result_obj(), script.as_ptr());
        assert!(interp
            .native_invocation_dialect()
            .native_namespace_code_policy()
            .is_none());
    });
}

// namespace inscope.
//
// These avoid `if`/`while`/`for`/`expr` (and anything else `have_tommath`-
// gated) entirely, so they run identically with or without the bignum
// tower.

#[test]
fn inscope_zero_tail_args_evaluates_script_verbatim() {
    leak_free(|i| {
        i.eval_str(b"proc probe {args} { return $args }");
        // C's `objc == 3` arm: no tail, so no list is appended and no
        // concat/trim/trailing space happens — the script runs as-is.
        assert_eq!(i.eval_str(b"namespace inscope :: probe"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
    });
}

#[test]
fn inscope_tail_args_become_list_elements_not_joined_words() {
    // `NamespaceInscopeCmd` (`generic/tclNamesp.c`) collects the tail
    // into a LIST and concatenates its string rep onto `script`
    // (`Tcl_ConcatObj` over `[script, list(tail)]`), so however many
    // words a tail argument holds, it reaches the invoked command as
    // exactly one argument.
    leak_free(|i| {
        i.eval_str(b"proc probe {args} { return $args }");

        // A tail word with an embedded space stays ONE argument (the
        // pre-fix bug: it split into two words, "x" and "y").
        assert_eq!(
            i.eval_str(b"llength [namespace inscope :: probe {x y}]"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(
            i.eval_str(b"lindex [namespace inscope :: probe {x y}] 0"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"x y");

        // An empty-string tail arg round-trips as one empty argument.
        assert_eq!(
            i.eval_str(b"llength [namespace inscope :: probe {}]"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(
            i.eval_str(b"lindex [namespace inscope :: probe {}] 0"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"");

        // Multiple tail words each keep their own arg boundary.
        assert_eq!(
            i.eval_str(b"llength [namespace inscope :: probe a {b c} d]"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"3");

        // Special characters round-trip through the list-element
        // quoting (`list::append_list_element`): an unbalanced brace, a
        // lone backslash, and an embedded double quote. `lindex`
        // recovers the raw element value regardless of which of the
        // four renderings (none/brace/mask/escape) quoting picked.
        i.eval_str(b"set v1 \"a\\{b\"");
        assert_eq!(
            i.eval_str(b"lindex [namespace inscope :: probe $v1] 0"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"a{b");

        i.eval_str(b"set v2 \"\\\\\"");
        assert_eq!(
            i.eval_str(b"lindex [namespace inscope :: probe $v2] 0"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"\\");

        i.eval_str(b"set v3 {a\"b}");
        assert_eq!(
            i.eval_str(b"lindex [namespace inscope :: probe $v3] 0"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"a\"b");

        i.eval_str(b"unset v1 v2 v3");
    });
}

#[test]
fn inscope_uses_a_byte_arrays_string_rep_in_both_arms() {
    // `binary format` returns a typed byte array.  When it becomes a
    // script, both `namespace inscope` forms reach the same unknown
    // command.  Capture the error through `binary encode hex`: this is the
    // C Tcl observable, and proves that the byte array's 0x80 payload is
    // preserved rather than becoming U+FFFD or UTF-8's `c2 80` payload.
    // The vector is identical on tclsh 8.6.18 and 9.0.4.
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"set name [binary format a3c cmd 128]; \
                      catch {namespace inscope :: $name} result; \
                      binary encode hex $result",
            ),
            Code::Ok,
            "zero-tail arm"
        );
        assert_eq!(
            i.result_bytes(),
            b"696e76616c696420636f6d6d616e64206e616d652022636d648022",
            "zero-tail arm preserves the raw command-name byte"
        );

        assert_eq!(
            i.eval_str(
                b"catch {namespace inscope :: $name extra} result; \
                      binary encode hex $result",
            ),
            Code::Ok,
            "tail arm must agree with the zero-tail arm"
        );
        assert_eq!(
            i.result_bytes(),
            b"696e76616c696420636f6d6d616e64206e616d652022636d648022",
            "tail arm preserves the same raw command-name byte"
        );

        i.eval_str(b"unset name result");
    });
}
