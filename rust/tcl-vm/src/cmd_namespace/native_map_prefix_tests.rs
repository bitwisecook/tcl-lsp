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

//! Replay original counted map vectors independently from command lookup.

use crate::{Code, Completion, Value, Vm};
use tcl_test_support::ensemble_map_prefix::{self as original, CONTROLS};

fn hex(bytes: &[u8]) -> String {
    String::from_utf8(tcl_cmd_core::binary::hex_encode(bytes)).unwrap()
}
fn row(vm: &Vm, tag: &str, completion: &Completion<Value>) -> String {
    let bytes = vm.native_name_operand_bytes(&completion.result).unwrap();
    format!("{tag}|{}|{}", completion.code.as_int(), hex(&bytes))
}
struct OriginalVector(usize);
impl crate::command::NativeCommand for OriginalVector {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        assert!(args.is_empty());
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_string_materialization(None)
            .unwrap()
            .protocol();
        let head = Value::new_native_string_bytes(original::original_head(self.0));
        let first = Value::new_native_string_bytes(b"FIRST".as_slice());
        let prefix = Value::native_list_constructor(vec![head, first], protocol);
        let key = Value::new_native_string_bytes(b"go".as_slice());
        let map = Value::native_list_constructor(vec![key, prefix], protocol);
        let mut words: Vec<_> = original::operation_words(self.0)
            .iter()
            .map(|word| Value::new_native_string_bytes(*word))
            .collect();
        words.push(map);
        vm.invoke_host_original_object_vector(&words[0], &words[1..])
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
    let query = [
        b"namespace".as_slice(),
        b"ensemble",
        b"configure",
        b"::r2286_map_E",
        b"-map",
    ]
    .map(Value::new_native_string_bytes);
    let completion = vm.invoke_host_original_object_vector(&query[0], &query[1..]);
    assert_eq!(completion.code, Code::Ok);
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_string_materialization(None)
        .unwrap()
        .protocol();
    let mut search = completion
        .result
        .native_lifetime_lease()
        .into_value()
        .into_native_dictionary_search(protocol)
        .unwrap();
    let (_, prefix) = search.next_original_pair().unwrap().unwrap();
    let members = prefix.native_object_list_elements(protocol).unwrap();
    let stored = vm.native_name_operand_bytes(&members[0]).unwrap();
    rows.push(format!("STORED_HEAD|{}|{}", stored.len(), hex(&stored)));
    rows.push(row(vm, "MAP_QUERY", &completion));
    let call = [b"::r2286_map_E".as_slice(), b"go"].map(Value::new_native_string_bytes);
    let completion = vm.invoke_host_original_object_vector(&call[0], &call[1..]);
    rows.push(row(vm, "CALL", &completion));
    rows
}

#[test]
fn original_counted_map_prefix_vectors_match_all_20_c_public_windows() {
    // naming.ensemble.original-counted-map-prefix-construction
    // docs/design/analysis/name-resolution-proofs/ensemble-original-counted-map-prefix-construction.md
    // Every original construction/getter/call row is compared independently;
    // raw-NUL String heads never pass through a source or ByteArray substitute.
    // C8.4/Jim unsupported originals are evidence without C map-worker authority.
    // No native primary, resident bytes or pointer identity is asserted.
    let mut compared = 0;
    for control in CONTROLS {
        for (engine, column) in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
            .into_iter()
            .zip(control.columns)
        {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            vm.try_register_native_command(
                "::r2286_original_map_callback",
                std::rc::Rc::new(OriginalVector(control.index)),
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
