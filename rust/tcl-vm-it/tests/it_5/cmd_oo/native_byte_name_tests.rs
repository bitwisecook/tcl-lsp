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

//! `TclOO` class, object, method and variable names with raw bytes.

use crate::prelude::*;

/// Invoke `words` as one command through `eval` of a list, so a command
/// name need not be UTF-8.
fn invoke(vm: &mut Vm, words: Vec<Value>) -> Completion<Value> {
    crate::fixture::call(vm, "eval", &[Value::list(words)])
}

fn make_class(vm: &mut Vm, name: &[u8]) -> Completion<Value> {
    crate::fixture::call(
        vm,
        "oo::class",
        &[
            Value::string("create"),
            Value::from_native_string_bytes(name),
        ],
    )
}

#[test]
fn declared_variables_retain_original_values_and_counted_object_fields() {
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let profile = tcl_vm::test_support::profile(version.dialect_profile_name());
        let mut vm = crate::fixture::interpreter(profile);
        assert_eq!(make_class(&mut vm, b"C").code, Code::Ok);
        let class = Value::string("C");
        let original = Value::from_native_string_bytes(b"a\0z".as_slice());
        let declaration = crate::fixture::call(
            &mut vm,
            "oo::define",
            &[class.clone(), Value::string("variable"), original.clone()],
        );
        assert_eq!(declaration.code, Code::Ok);
        let listed = crate::fixture::call(
            &mut vm,
            "info",
            &[
                Value::string("class"),
                Value::string("variables"),
                class.clone(),
            ],
        );
        assert_eq!(listed.code, Code::Ok);
        let listed = listed.result.as_list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].string_bytes().as_ref(), b"a\0z");
        let mut body = b"set {".to_vec();
        body.extend_from_slice(b"a\0z");
        body.extend_from_slice(b"} HIT; return [info object vars [self]]");
        let definition = crate::fixture::call(
            &mut vm,
            "oo::define",
            &[
                class.clone(),
                Value::string("method"),
                Value::string("write"),
                Value::empty(),
                Value::from_native_string_bytes(body),
            ],
        );
        assert_eq!(definition.code, Code::Ok);
        assert_eq!(
            crate::fixture::call(&mut vm, "C", &[Value::string("create"), Value::string("o")]).code,
            Code::Ok
        );
        let fields = crate::fixture::call(&mut vm, "o", &[Value::string("write")]);
        assert_eq!(
            fields.code,
            Code::Ok,
            "{} {:?}",
            profile.name,
            fields.result.string_bytes()
        );
        let fields = fields.result.as_list().unwrap();
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].string_bytes().as_ref(), b"a\0z");
        let rejected = crate::fixture::call(
            &mut vm,
            "oo::define",
            &[
                class.clone(),
                Value::string("variable"),
                Value::from_native_string_bytes(b"bad::name".as_slice()),
            ],
        );
        assert_eq!(rejected.code, Code::Error);
        let retained = crate::fixture::call(
            &mut vm,
            "info",
            &[Value::string("class"), Value::string("variables"), class],
        );
        assert_eq!(retained.result.as_list().unwrap().len(), 1);
    }
}

#[test]
fn native_object_and_method_keys_keep_independent_extents_and_original_usage() {
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let profile = tcl_vm::test_support::profile(version.dialect_profile_name());
        let mut vm = crate::fixture::interpreter(profile);
        let class = Value::from_native_string_bytes(b"C\xff".as_slice());
        let created = make_class(&mut vm, b"C\xff");
        assert_eq!(created.code, Code::Ok, "{}", profile.name);
        assert_eq!(created.result.string_bytes().as_ref(), b"::C\xff");
        for method in [
            b"a\0z".as_slice(),
            b"a\xffz".as_slice(),
            b"a\xc0\x80z".as_slice(),
        ] {
            let original = Value::from_native_string_bytes(method);
            let defined = invoke(
                &mut vm,
                vec![
                    Value::string("oo::define"),
                    class.clone(),
                    Value::string("method"),
                    original,
                    Value::empty(),
                    Value::string("return HIT"),
                ],
            );
            assert_eq!(defined.code, Code::Ok, "{} {method:?}", profile.name);
        }
        let object = Value::from_native_string_bytes(b"O\xff".as_slice());
        let created = invoke(
            &mut vm,
            vec![class.clone(), Value::string("create"), object.clone()],
        );
        assert_eq!(created.code, Code::Ok);
        assert_eq!(created.result.string_bytes().as_ref(), b"::O\xff");
        for method in [
            b"a\0z".as_slice(),
            b"a\xffz".as_slice(),
            b"a\xc0\x80z".as_slice(),
        ] {
            let hit = invoke(
                &mut vm,
                vec![object.clone(), Value::from_native_string_bytes(method)],
            );
            assert_eq!(hit.code, Code::Ok, "{} {method:?}", profile.name);
            assert_eq!(hit.result.string_bytes().as_ref(), b"HIT");
        }
        let miss = invoke(&mut vm, vec![object.clone(), Value::string("a")]);
        assert_eq!(miss.code, Code::Error);
        assert_counted_object_usage(&mut vm, &class);
    }
}

fn assert_counted_object_usage(vm: &mut Vm, class: &Value) {
    let raw_name = Value::from_native_string_bytes(b"N\0z".as_slice());
    let created = invoke(
        vm,
        vec![class.clone(), Value::string("create"), raw_name.clone()],
    );
    assert_eq!(created.code, Code::Ok);
    assert_eq!(created.result.string_bytes().as_ref(), b"::N");
    let invocation = invoke(vm, vec![raw_name.clone()]);
    assert_eq!(invocation.code, Code::Error);
    // C9 uses native list quoting for usage; the original input remains counted.
    assert_eq!(raw_name.string_bytes().as_ref(), b"N\0z");
    assert!(
        invocation
            .result
            .string_bytes()
            .as_ref()
            .windows(3)
            .any(|part| part == b"N\0z")
    );
}
