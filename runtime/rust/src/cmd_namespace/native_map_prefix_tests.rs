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

use crate::{
    interp::{BuiltinFn, Code, Interp},
    obj::{self, Owned, TclObj},
};
use tcl_test_support::ensemble_map_prefix::{self as original, CONTROLS};

fn hex(bytes: &[u8]) -> String {
    String::from_utf8(tcl_cmd_core::binary::hex_encode(bytes)).unwrap()
}
fn row(interp: &Interp, tag: &str, code: Code) -> String {
    format!("{tag}|{}|{}", code.as_int(), hex(&interp.result_bytes()))
}
fn original_vector(interp: &mut Interp, index: usize) -> Code {
    let protocol = interp
        .native_invocation_dialect()
        .native_string_materialization(None)
        .unwrap()
        .protocol();
    let head = Owned::fresh(obj::new_string_bytes(original::original_head(index)));
    let first = Owned::fresh(obj::new_string_bytes(b"FIRST"));
    let prefix = Owned::fresh(crate::list::new_list_obj_native(
        &[head.as_ptr(), first.as_ptr()],
        protocol,
    ));
    let key = Owned::fresh(obj::new_string_bytes(b"go"));
    let map = Owned::fresh(crate::list::new_list_obj_native(
        &[key.as_ptr(), prefix.as_ptr()],
        protocol,
    ));
    let mut words: Vec<_> = original::operation_words(index)
        .iter()
        .map(|word| Owned::fresh(obj::new_string_bytes(word)))
        .collect();
    words.push(Owned::retain(map.as_ptr()));
    let pointers: Vec<_> = words.iter().map(Owned::as_ptr).collect();
    interp.eval_original_object_vector(&pointers)
}
fn create_relative(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    original_vector(interp, 0)
}
fn create_rooted(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    original_vector(interp, 1)
}
fn configure_relative(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    original_vector(interp, 2)
}
fn configure_rooted(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    original_vector(interp, 3)
}
fn create_global(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
    original_vector(interp, 4)
}

fn observe(interp: &mut Interp, index: usize) -> Vec<String> {
    let code = interp.eval_str(original::SETUP);
    let mut rows = vec![row(interp, "SETUP", code)];
    if index == 2 || index == 3 {
        let code = interp.eval_str(original::INITIAL);
        rows.push(row(interp, "INITIAL", code));
    }
    let head = original::original_head(index);
    rows.push(format!("INPUT|{}|{}", head.len(), hex(head)));
    let code = interp.eval_str(original::OPERATIONS[index]);
    rows.push(row(interp, "OPERATION", code));
    let query = [
        b"namespace".as_slice(),
        b"ensemble",
        b"configure",
        b"::r2286_map_E",
        b"-map",
    ]
    .map(|word| Owned::fresh(obj::new_string_bytes(word)));
    let code = interp.eval_original_object_vector(&query.each_ref().map(Owned::as_ptr));
    let map = Owned::retain(interp.result_obj());
    assert_eq!(code, Code::Ok);
    let protocol = interp
        .native_invocation_dialect()
        .native_string_materialization(None)
        .unwrap()
        .protocol();
    let pairs = crate::dict::native_dict_pairs(map.as_ptr(), protocol).unwrap();
    assert_eq!(pairs.len(), 1);
    let members = crate::list::list_elements_native_checked(pairs[0].1, protocol).unwrap();
    let stored = crate::dict::native_object_bytes(members[0], protocol).unwrap();
    rows.push(format!("STORED_HEAD|{}|{}", stored.len(), hex(&stored)));
    rows.push(row(interp, "MAP_QUERY", code));
    let call =
        [b"::r2286_map_E".as_slice(), b"go"].map(|word| Owned::fresh(obj::new_string_bytes(word)));
    let code = interp.eval_original_object_vector(&call.each_ref().map(Owned::as_ptr));
    rows.push(row(interp, "CALL", code));
    assert!(
        !interp.host_refusal_pending(),
        "{:?}",
        interp.native_access_refusal()
    );
    rows
}

#[test]
fn original_counted_map_prefix_vectors_match_all_20_c_public_windows() {
    // naming.ensemble.original-counted-map-prefix-construction
    // docs/design/analysis/name-resolution-proofs/ensemble-original-counted-map-prefix-construction.md
    // Original counted String/list/map vectors in an actual entered namespace;
    // all construction/readback/call rows match independently. C8.4 and Jim
    // incompatible original vectors remain in the native record and do not
    // grant this C map worker. No native header/pointer identity is asserted.
    let callbacks: [BuiltinFn; 5] = [
        create_relative,
        create_rooted,
        configure_relative,
        configure_rooted,
        create_global,
    ];
    let mut compared = 0;
    for control in CONTROLS {
        for (engine, column) in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
            .into_iter()
            .zip(control.columns)
        {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"::r2286_original_map_callback", callbacks[control.index]);
            assert_eq!(
                observe(&mut interp, control.index),
                original::observation_rows(column),
                "{engine}/{}",
                control.case
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 20);
}
