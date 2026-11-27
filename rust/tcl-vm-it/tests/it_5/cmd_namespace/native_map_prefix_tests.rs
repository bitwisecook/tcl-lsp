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

//! Original counted map vectors, replayed independently from command lookup.

use crate::prelude::*;
use tcl_test_support::ensemble_map_prefix::{self as original, CONTROLS};

fn hex(bytes: &[u8]) -> String {
    String::from_utf8(tcl_cmd_core::binary::hex_encode(bytes)).unwrap()
}
fn row(vm: &Vm, tag: &str, completion: &Completion<Value>) -> String {
    let bytes = crate::fixture::bytes(vm, &completion.result);
    format!("{tag}|{}|{}", completion.code.as_int(), hex(&bytes))
}
fn words(words: &[&[u8]]) -> Vec<Value> {
    words
        .iter()
        .map(|word| Value::new_native_string_bytes(*word))
        .collect()
}
struct OriginalVector(usize);
impl tcl_vm::NativeCommand for OriginalVector {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        assert!(args.is_empty());
        let protocol = crate::fixture::strings(vm);
        let head = Value::new_native_string_bytes(original::original_head(self.0));
        let first = Value::new_native_string_bytes(b"FIRST".as_slice());
        let prefix = Value::native_list_constructor(vec![head, first], protocol);
        let key = Value::new_native_string_bytes(b"go".as_slice());
        let map = Value::native_list_constructor(vec![key, prefix], protocol);
        let operation = original::operation_words(self.0);
        let mut arguments = words(&operation[1..]);
        arguments.push(map);
        crate::fixture::call(vm, std::str::from_utf8(operation[0]).unwrap(), &arguments)
    }
}
fn observe(vm: &mut Vm, index: usize) -> Vec<String> {
    let completion = vm.try_eval_source_bytes(original::SETUP).unwrap();
    let mut rows = vec![row(vm, "SETUP", &completion)];
    if index == 2 || index == 3 {
        let completion = vm.try_eval_source_bytes(original::INITIAL).unwrap();
        rows.push(row(vm, "INITIAL", &completion));
    }
    let head = original::original_head(index);
    rows.push(format!("INPUT|{}|{}", head.len(), hex(head)));
    let completion = vm
        .try_eval_source_bytes(original::OPERATIONS[index])
        .unwrap();
    rows.push(row(vm, "OPERATION", &completion));
    let query = words(&[b"ensemble", b"configure", b"::r2286_map_E", b"-map"]);
    let completion = crate::fixture::call(vm, "namespace", &query);
    assert_eq!(completion.code, Code::Ok);
    let protocol = crate::fixture::strings(vm);
    let map = completion
        .result
        .native_object_list_elements(protocol)
        .unwrap();
    let members = map.elements().unwrap()[1]
        .native_object_list_elements(protocol)
        .unwrap();
    let stored = crate::fixture::bytes(vm, &members.elements().unwrap()[0]);
    rows.push(format!("STORED_HEAD|{}|{}", stored.len(), hex(&stored)));
    rows.push(row(vm, "MAP_QUERY", &completion));
    let completion = crate::fixture::call(vm, "::r2286_map_E", &words(&[b"go"]));
    rows.push(row(vm, "CALL", &completion));
    rows
}

#[test]
fn original_counted_map_prefix_vectors_match_all_20_c_public_windows() {
    // naming.ensemble.original-counted-map-prefix-construction
    // docs/design/analysis/name-resolution-proofs/ensemble-original-counted-map-prefix-construction.md
    let mut compared = 0;
    for control in CONTROLS {
        for (engine, column) in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
            .into_iter()
            .zip(control.columns)
        {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
            vm.try_register_native_command(
                "::r2286_original_map_callback",
                Rc::new(OriginalVector(control.index)),
            )
            .unwrap();
            assert_eq!(
                observe(&mut vm, control.index),
                original::observation_rows(column),
                "{engine}/{}",
                control.case
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 20);
}
