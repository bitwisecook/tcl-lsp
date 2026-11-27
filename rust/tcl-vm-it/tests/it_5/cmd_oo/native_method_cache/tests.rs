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

//! Method dispatch results across renames, redefinitions, object methods, mixins and `next`.

use crate::prelude::*;

const VERSIONS: [tcl_dialect::TclVersion; 3] = [
    tcl_dialect::TclVersion::V8_6,
    tcl_dialect::TclVersion::V9_0,
    tcl_dialect::TclVersion::V9_1,
];

fn vm(version: tcl_dialect::TclVersion) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(
        version.dialect_profile_name(),
    ))
}

fn source(vm: &mut Vm, text: &str) -> Completion<Value> {
    let completion = crate::fixture::eval(vm, text);
    assert_eq!(
        completion.code,
        Code::Ok,
        "{text}: {}",
        completion.result.to_str()
    );
    completion
}

fn call(vm: &mut Vm, object: &str, method: &Value) -> String {
    let completion = crate::fixture::call(vm, object, std::slice::from_ref(method));
    completion.result.to_str().to_string()
}

#[test]
fn original_method_primary_and_table_keys_share_the_actual_chain() {
    for version in VERSIONS {
        let mut vm = vm(version);
        source(&mut vm, "oo::class create C {method m {} {return FIRST}}");
        source(&mut vm, "C create obj");
        let original = Value::string("m");
        assert_eq!(call(&mut vm, "obj", &original), "FIRST");
        assert_eq!(call(&mut vm, "obj", &Value::string("m")), "FIRST");
        source(&mut vm, "rename obj renamed");
        assert_eq!(call(&mut vm, "renamed", &original), "FIRST");
        for (change, expected) in [
            ("oo::define C method m {} {return SECOND}", "SECOND"),
            ("oo::objdefine renamed method m {} {return LOCAL}", "LOCAL"),
            ("oo::objdefine renamed deletemethod m", "SECOND"),
            ("oo::objdefine renamed mixin", "SECOND"),
        ] {
            source(&mut vm, change);
            assert_eq!(call(&mut vm, "renamed", &original), expected, "{change}");
        }
    }
}

#[test]
fn reached_method_records_distinguish_replacement_from_recreation() {
    for version in VERSIONS {
        for (mutation, expected) in [
            ("oo::define B deletemethod m", "BASE"),
            ("oo::define B method m {} {return REPLACED}", "REPLACED"),
            (
                "oo::define B deletemethod m; oo::define B method m {} {return NEW_RECORD}",
                "BASE",
            ),
        ] {
            let mut vm = vm(version);
            source(&mut vm, "oo::class create B {method m {} {return BASE}}");
            source(
                &mut vm,
                &format!("oo::class create C {{superclass B; method m {{}} {{{mutation}; next}}}}"),
            );
            source(&mut vm, "C create obj");
            assert_eq!(call(&mut vm, "obj", &Value::string("m")), expected);
        }
    }
}

#[test]
fn foreign_cache_and_missing_methods_cannot_supply_an_owner() {
    let mut first = vm(tcl_dialect::TclVersion::V8_6);
    let mut second = vm(tcl_dialect::TclVersion::V8_6);
    for vm in [&mut first, &mut second] {
        source(vm, "oo::class create C {method m {} {return RESULT}}");
        source(vm, "C create obj");
    }
    let original = Value::string("m");
    assert_eq!(call(&mut first, "obj", &original), "RESULT");
    assert_eq!(call(&mut second, "obj", &original), "RESULT");
    let missing = crate::fixture::call(&mut second, "obj", &[Value::string("absent")]);
    assert_eq!(missing.code, Code::Error);
}
