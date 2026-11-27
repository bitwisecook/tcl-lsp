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

//! A namespace created inside a dying ancestor during its teardown, against
//! the tclsh captures.

use crate::prelude::*;
use std::cell::Cell;
use tcl_vm::NativeCommand;

fn ok(value: Value) -> Completion<Value> {
    Completion::new(Code::Ok, value, Value::empty())
}

fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn expected(rows: &str, label: &str) -> Vec<u8> {
    let field = |kind: &str| {
        let selected: Vec<_> = rows
            .lines()
            .filter(|line| line.starts_with(&format!("{kind}|{label}|")))
            .collect();
        assert_eq!(selected.len(), 1, "{kind}/{label}");
        let fields: Vec<_> = selected[0].split('|').collect();
        assert_eq!(fields[2], "0", "{kind}/{label}");
        String::from_utf8(unhex(fields[3])).unwrap()
    };
    let public = field("PUBLIC_LOOKUPS");
    let public: Vec<_> = public.split_ascii_whitespace().collect();
    let contents = field("CONTENTS");
    let during = [
        field("CREATE"),
        field("ORIGINAL_ANCESTOR"),
        public[1].to_owned(),
        public[2].to_owned(),
        contents,
    ]
    .join(" ");
    format!("{{{during}}} {} {}", field("FINAL"), field("CALLBACK_ONCE")).into_bytes()
}

fn setup(target: &[u8]) -> Vec<u8> {
    [br"
set ::target {".as_slice(), target, br"}
set ::observations UNREACHED
set ::callback_count 0
namespace eval retiring {
    namespace eval Existing {variable before BEFORE}
    variable sentinel HELD
}
proc observe_original_ancestor {} {
    string match ::retiring::* [uplevel 1 {namespace current}]
}
proc during args {
    incr ::callback_count
    namespace eval $::target {
        variable value HELD
        proc p {} {return CHILD}
        set ::observations [list [namespace current] [observe_original_ancestor] [namespace exists [namespace current]] [info exists value] [set value] [p]]
    }
}
"].concat()
}

const FINISH: &[u8] = br"
namespace delete ::retiring
list $::observations [namespace exists ::retiring] [namespace exists $::target] [info exists ${::target}::value] [info commands ${::target}::p] $::callback_count
";

struct RetiringCommand(Rc<Cell<usize>>);
impl NativeCommand for RetiringCommand {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        ok(Value::empty())
    }
    fn retire(&self, vm: &mut Vm) -> Completion<Value> {
        self.0.set(self.0.get() + 1);
        crate::fixture::eval(vm, "during")
    }
}

const DYING_DESCENDANT_ROWS: [(tcl_dialect::TclVersion, &str); 5] = [
    (
        tcl_dialect::TclVersion::V8_4,
        include_str!(
            "../../../../tcl-registry/tests/data/native_dying_descendant_physical/tcl8.4/stdout.tsv"
        ),
    ),
    (
        tcl_dialect::TclVersion::V8_5,
        include_str!(
            "../../../../tcl-registry/tests/data/native_dying_descendant_physical/tcl8.5/stdout.tsv"
        ),
    ),
    (
        tcl_dialect::TclVersion::V8_6,
        include_str!(
            "../../../../tcl-registry/tests/data/native_dying_descendant_physical/tcl8.6/stdout.tsv"
        ),
    ),
    (
        tcl_dialect::TclVersion::V9_0,
        include_str!(
            "../../../../tcl-registry/tests/data/native_dying_descendant_physical/tcl9.0/stdout.tsv"
        ),
    ),
    (
        tcl_dialect::TclVersion::V9_1,
        include_str!(
            "../../../../tcl-registry/tests/data/native_dying_descendant_physical/tcl9.1/stdout.tsv"
        ),
    ),
];

#[test]
fn original_missing_descendant_has_the_dying_ancestor_and_is_destroyed_with_it() {
    for (version, rows) in DYING_DESCENDANT_ROWS {
        for (case, target) in [
            ("MISSING_CHILD", b"::retiring::New".as_slice()),
            ("MISSING_PARENT", b"::retiring::New::"),
            ("DEEP_CHILD", b"::retiring::New::Leaf"),
            ("EXISTING_CHILD", b"::retiring::Existing"),
        ] {
            for command_window in [false, true] {
                let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(
                    version.dialect_profile_name(),
                ));
                assert_eq!(
                    crate::fixture::eval(&mut vm, std::str::from_utf8(&setup(target)).unwrap())
                        .code,
                    Code::Ok
                );
                let calls = Rc::new(Cell::new(0));
                if command_window {
                    vm.register_native_command(
                        "::retiring::sentinel_command",
                        Rc::new(RetiringCommand(Rc::clone(&calls))),
                    );
                } else {
                    assert_eq!(
                        crate::fixture::eval(&mut vm, if version == tcl_dialect::TclVersion::V8_4 {
                            "namespace eval retiring {trace variable sentinel u ::during}"
                        } else {
                            "namespace eval retiring {trace add variable sentinel unset ::during}"
                        })
                        .code,
                        Code::Ok
                    );
                }
                let completion =
                    crate::fixture::eval(&mut vm, std::str::from_utf8(FINISH).unwrap());
                let label = format!(
                    "{}_{}",
                    if command_window { "COMMAND" } else { "UNSET" },
                    case
                );
                assert_eq!(
                    completion.code,
                    Code::Ok,
                    "{version:?}/{label}/{completion:?}"
                );
                assert_eq!(
                    crate::fixture::bytes(&vm, &completion.result),
                    expected(rows, &label),
                    "{version:?}/{label}"
                );
                assert_eq!(calls.get(), usize::from(command_window));
                assert_eq!(
                    crate::fixture::eval(&mut vm, "namespace current")
                        .result
                        .to_str()
                        .as_ref(),
                    "::"
                );
            }
        }
    }
}
