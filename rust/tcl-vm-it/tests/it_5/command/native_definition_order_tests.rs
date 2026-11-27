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

//! `proc` definition completions, compared with the tclsh and jimsh captures.

use crate::prelude::*;

fn actual(engine: &str) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(engine))
}

fn row<'a>(capture: &'a str, case: usize, phase: &str) -> Vec<&'a str> {
    capture
        .lines()
        .find(|line| {
            let fields: Vec<_> = line.split('|').collect();
            fields.first() == Some(&"ROW")
                && fields[1].parse::<usize>() == Ok(case)
                && fields[2] == phase
        })
        .unwrap()
        .split('|')
        .collect()
}

#[test]
fn original_procedure_public_completions_match_all_eighteen_native_definition_windows() {
    // naming.invocation.original-naming-argument-getter-and-effect-order
    // docs/design/analysis/name-resolution-proofs/invocation-original-naming-argument-getter-and-effect-order.md
    // Public worker and invocation code/result only: these values already have
    // Strings and cannot assert the original extension's getter tape or counts.
    let providers = [
        (
            "tcl8.4",
            include_str!(
                "../../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/tcl8.4/execute.stdout"
            ),
        ),
        (
            "tcl8.5",
            include_str!(
                "../../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/tcl8.5/execute.stdout"
            ),
        ),
        (
            "tcl8.6",
            include_str!(
                "../../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/tcl8.6/execute.stdout"
            ),
        ),
        (
            "tcl9.0",
            include_str!(
                "../../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/tcl9.0/execute.stdout"
            ),
        ),
        (
            "tcl9.1",
            include_str!(
                "../../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499c/providers/tcl9.1/execute.stdout"
            ),
        ),
        (
            "jim",
            include_str!(
                "../../../../tcl-registry/tests/data/native_naming_argument_getter_effect_order_original/499d/providers/jim/execute.stdout"
            ),
        ),
    ];
    let mut compared = 0;
    for (engine, capture) in providers {
        for case in [9, 10, 11] {
            let mut vm = actual(engine);
            crate::fixture::eval(&mut vm, "namespace eval ::d {}");
            let completion = crate::fixture::call(
                &mut vm,
                "proc",
                &[
                    Value::new_native_string_bytes(if case == 9 {
                        b"::missing::p".as_slice()
                    } else {
                        b"::d::p".as_slice()
                    }),
                    Value::new_native_string_bytes(if case == 11 {
                        b"".as_slice()
                    } else {
                        b"{bad".as_slice()
                    }),
                    Value::new_native_string_bytes(b"return OK".as_slice()),
                ],
            );
            let original = row(capture, case, "worker");
            assert_eq!(
                completion.code.as_int(),
                original[3].parse::<i64>().unwrap(),
                "{engine}/{case}"
            );
            assert_eq!(
                crate::fixture::bytes(&vm, &completion.result),
                crate::fixture::unhex(original[4]),
                "{engine}/{case}"
            );
            if case == 11 {
                let completion = crate::fixture::eval(&mut vm, "::d::p");
                let original = row(capture, case, "invoke");
                assert_eq!(
                    completion.code.as_int(),
                    original[3].parse::<i64>().unwrap(),
                    "{engine}/{case}"
                );
                assert_eq!(
                    crate::fixture::bytes(&vm, &completion.result),
                    crate::fixture::unhex(original[4]),
                    "{engine}/{case}"
                );
            }
            compared += 1;
        }
    }
    assert_eq!(compared, 18);
}

#[test]
fn original_definition_guards_do_not_read_later_unselected_name_or_body_objects() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(engine);
        let completion = crate::fixture::eval(&mut vm, "catch {proc ::missing::p {x} {unread}}");
        assert_eq!(
            crate::fixture::bytes(&vm, &completion.result),
            b"1",
            "{engine}"
        );
    }
    let mut vm = actual("jim");
    let completion = crate::fixture::eval(&mut vm, "proc unread {{x y z}} {unread}");
    assert_eq!(completion.code, Code::Error);
    assert!(
        crate::fixture::bytes(&vm, &completion.result)
            .starts_with(b"too many fields in argument specifier")
    );
}
