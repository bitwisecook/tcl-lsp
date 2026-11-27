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

//! `subst` completions under each flag set, and the namespace and procedure each substitution reads.

use crate::prelude::*;

fn actual(engine: &str) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(engine))
}
fn call(vm: &mut Vm, original: &Value, flags: u8) -> Completion<Value> {
    let mut arguments = Vec::new();
    for (bit, spelling) in [
        (4, "-nobackslashes"),
        (1, "-nocommands"),
        (2, "-novariables"),
    ] {
        if flags & bit == 0 {
            arguments.push(Value::string(spelling));
        }
    }
    arguments.push(original.clone());
    crate::fixture::call(vm, "subst", &arguments)
}
const SUBSTITUTION_OWNER_ROWS: [(&str, &str); 3] = [
    (
        "tcl8.6",
        include_str!(
            "../../../../tcl-registry/tests/data/native_substitution_owner/v6/tcl8.6/stdout.tsv"
        ),
    ),
    (
        "tcl9.0",
        include_str!(
            "../../../../tcl-registry/tests/data/native_substitution_owner/v6/tcl9.0/stdout.tsv"
        ),
    ),
    (
        "tcl9.1",
        include_str!(
            "../../../../tcl-registry/tests/data/native_substitution_owner/v6/tcl9.1/stdout.tsv"
        ),
    ),
];

#[test]
fn original_compiled_substitution_matches_counted_native_flags_and_completions() {
    // Native proof: naming.substitution.counted-template-completions
    // docs/design/analysis/name-resolution-proofs/substitution-counted-template-completions.md
    let inputs: Vec<_> =
        include_str!("../../../../tcl-registry/tests/data/native_substitution_owner/inputs.json")
            .lines()
            .filter_map(|line| {
                line.trim()
                    .trim_end_matches(',')
                    .strip_prefix('"')?
                    .strip_suffix('"')
            })
            .collect();
    assert_eq!(inputs.len(), 10);
    let mut compared = 0;
    for (engine, rows) in SUBSTITUTION_OWNER_ROWS {
        for (source, input) in inputs.iter().enumerate() {
            let mut vm = actual(engine);
            let original = Value::new_native_string_bytes(crate::fixture::unhex(input));
            for flags in (0..=7).rev() {
                for label in ["ROOT", "ROOT_REPEAT"] {
                    for (name, value) in [("x", "X"), ("y", "Y"), ("a(k)", "K"), ("k", "k")] {
                        assert_eq!(
                            crate::fixture::run(&mut vm, "set", &[name, value]).0,
                            Code::Ok
                        );
                    }
                    let fields: Vec<_> = rows
                        .lines()
                        .find(|row| row.starts_with(&format!("R|{label}|{source}|{flags}|")))
                        .unwrap()
                        .split('|')
                        .collect();
                    let result = call(&mut vm, &original, flags);
                    assert_eq!(
                        result.code.as_int(),
                        fields[4].parse::<i64>().unwrap(),
                        "{engine}/{source}/{flags}/{label}: {result:?}"
                    );
                    assert_eq!(
                        crate::fixture::bytes(&vm, &result.result),
                        crate::fixture::unhex(fields[9]),
                        "{engine}/{source}/{flags}/{label}"
                    );
                    compared += 1;
                }
            }
        }
    }
    assert_eq!(compared, 480);
}

#[test]
fn original_substitution_changes_namespace_and_borrowed_local_cache_without_proc_header() {
    // Native proof: naming.substitution.namespace-and-local-context
    // docs/design/analysis/name-resolution-proofs/substitution-namespace-and-local-context.md
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = actual(engine);
        assert_eq!(
            crate::fixture::run(&mut vm, "set", &["::template", "$x"]).0,
            Code::Ok
        );
        let setup = crate::fixture::eval(
            &mut vm,
            "namespace eval A {variable x A}; namespace eval B {variable x B}; set x ROOT; proc P {x} {subst $::template}; proc Q {padding x} {subst $::template}",
        );
        assert_eq!(setup.code, Code::Ok, "{engine}: {setup:?}");
        for (source, expected) in [
            ("namespace eval A {subst $::template}", "A"),
            ("namespace eval B {subst $::template}", "B"),
            ("subst $::template", "ROOT"),
        ] {
            let result = crate::fixture::eval(&mut vm, source);
            assert_eq!(result.code, Code::Ok, "{engine}: {result:?}");
            assert_eq!(result.result.to_str().as_ref(), expected);
        }
        for (command, arguments, expected) in [
            ("P", ["P_VALUE"].as_slice(), "P_VALUE"),
            ("P", &["P_REPEAT"], "P_REPEAT"),
            ("Q", &["PAD", "Q_VALUE"], "Q_VALUE"),
            ("P", &["P_AGAIN"], "P_AGAIN"),
        ] {
            assert_eq!(
                crate::fixture::run(&mut vm, command, arguments),
                (Code::Ok, expected.as_bytes().to_vec()),
                "{engine}/{command}"
            );
        }
    }
}
