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

//! Coroutine introspection over byte-exact command names.

use crate::prelude::*;

fn native() -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.1"))
}

fn bytes(vm: &mut Vm, value: &Value) -> Vec<u8> {
    vm.native_string_bytes(value).unwrap().to_vec()
}

#[test]
fn coroutine_probe_and_injection_keep_opaque_names_and_original_arguments() {
    let mut vm = native();
    let ready = crate::fixture::eval(
        &mut vm,
        "coroutine c apply {{} {return [yield READY]}}; proc returnfirst {first args} {return $first}",
    );
    assert!(ready.code.is_ok());
    let name = Value::from_string_bytes(b"c\xff".as_slice());
    let head = Value::from_string_bytes(b"probe\xff".as_slice());
    for (from, to) in [("c", &name), ("returnfirst", &head)] {
        let renamed = crate::fixture::call(&mut vm, "rename", &[Value::string(from), to.clone()]);
        assert!(renamed.code.is_ok(), "{renamed:?}");
    }
    let argument = Value::from_string_bytes(b"A\0B\xff".as_slice());
    let kind = crate::fixture::call(
        &mut vm,
        "::tcl::unsupported::corotype",
        std::slice::from_ref(&name),
    );
    assert!(kind.code.is_ok(), "{kind:?}");
    assert_eq!(bytes(&mut vm, &kind.result), b"yield");
    let probe = crate::fixture::call(
        &mut vm,
        "coroprobe",
        &[name.clone(), head.clone(), argument.clone()],
    );
    assert!(probe.code.is_ok(), "{probe:?}");
    assert_eq!(bytes(&mut vm, &probe.result), b"A\0B\xff");
    let queued = crate::fixture::call(&mut vm, "coroinject", &[name.clone(), head, argument]);
    assert!(queued.code.is_ok(), "{queued:?}");
    let resumed = crate::fixture::call(&mut vm, "eval", &[Value::list(vec![name])]);
    assert!(resumed.code.is_ok(), "{resumed:?}");
    assert_eq!(bytes(&mut vm, &resumed.result), b"A\0B\xff");
}

#[test]
fn coroutine_original_name_lookup_reports_a_missing_name_as_a_guest_error() {
    let missing = Value::from_string_bytes(b"absent\xff".as_slice());
    let mut vm = native();
    let error = crate::fixture::call(&mut vm, "::tcl::unsupported::corotype", &[missing]);
    assert_eq!(error.code, Code::Error);
}
