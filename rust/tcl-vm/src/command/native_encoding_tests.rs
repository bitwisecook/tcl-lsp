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

//! Original external UTF-8 object-vector observation controls.

fn primary(snapshot: &tcl_syntax::native_object::NativeObjectSnapshot) -> &'static str {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    match &snapshot.cache {
        Cache::None => "none",
        Cache::String { .. } => "string",
        Cache::ByteArray { .. } => "bytearray",
        other => panic!("unexpected original primary {other:?}"),
    }
}
fn hex(bytes: &[u8]) -> String {
    String::from_utf8(tcl_cmd_core::binary::hex_encode(bytes)).unwrap()
}

#[test]
fn original_utf8_conversion_matches_all_55_c_object_storage_windows() {
    // naming.encoding.original-utf8-convertto-object-storage
    // docs/design/analysis/name-resolution-proofs/encoding-original-utf8-convertto-object-storage.md
    // naming.encoding.original-utf8-convertto-error-offset
    // docs/design/analysis/name-resolution-proofs/encoding-original-utf8-convertto-error-offset.md
    // Original direct object-vector invocation and before-getter snapshots;
    // no source compiler/CLI rewrite or original pointer/refcount assertion.
    use std::rc::Rc;
    use tcl_syntax::value::ValueOps;
    use tcl_test_support::encoding_utf8::{CONTROLS, object_observation_rows, original_input};
    let mut comparisons = 0;
    for control in CONTROLS {
        for (engine, column) in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
            .into_iter()
            .zip(control.columns)
        {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::core(profile);
            let bytes = original_input(column);
            let input = if control.binary {
                crate::Value::from_native_byte_array(
                    Rc::from(bytes.as_slice()),
                    vm.actual_native_invocation_dialect(),
                )
                .unwrap()
            } else {
                crate::Value::new_native_string_bytes(bytes.as_slice())
            };
            let before = input.native_object_snapshot();
            let mut rows = vec![format!(
                "INPUT|{}|{}|{}|{}",
                control.case,
                primary(&before),
                u8::from(before.resident.is_some()),
                hex(&bytes)
            )];
            let completion = vm
                .try_invoke_command(
                    "encoding",
                    &[
                        crate::Value::new_native_string_bytes(b"convertto".as_slice()),
                        crate::Value::new_native_string_bytes(b"utf-8".as_slice()),
                        input.clone(),
                    ],
                )
                .unwrap();
            let output = completion.result.native_object_snapshot();
            let after = input.native_object_snapshot();
            rows.push(format!(
                "RETURN|{}|{}|{}",
                completion.code.as_int(),
                primary(&output),
                u8::from(output.resident.is_some())
            ));
            rows.push(format!(
                "INPUT_AFTER|{}|{}",
                primary(&after),
                u8::from(after.resident.is_some())
            ));
            if completion.code == crate::Code::Ok {
                let raw = completion
                    .result
                    .byte_array_representation()
                    .expect("actual binary result getter");
                rows.push(format!("RAW|{}|{}", raw.len(), hex(&raw)));
            } else {
                let message = ValueOps::native_string_bytes(&mut vm, &completion.result).unwrap();
                rows.push(format!("MESSAGE|{}", hex(&message)));
                let error_code = vm
                    .try_invoke_command(
                        "set",
                        &[crate::Value::new_native_string_bytes(
                            b"::errorCode".as_slice(),
                        )],
                    )
                    .unwrap();
                assert_eq!(error_code.code, crate::Code::Ok);
                let code = ValueOps::native_string_bytes(&mut vm, &error_code.result).unwrap();
                rows.push(format!("ERROR_CODE|{}", hex(&code)));
            }
            assert_eq!(
                rows,
                object_observation_rows(column),
                "{engine}/{}",
                control.case
            );
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 55);
}

#[test]
fn external_utf8_binary_result_refuses_foreign_or_jim_issuer() {
    // Authored API negative control, separate from original native availability.
    use tcl_syntax::value::ValueOps;
    for engine in ["tcl8.4", "jim"] {
        let mut vm = crate::native_fixture::core(
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        );
        assert!(
            ValueOps::native_external_utf8_result(&mut vm, b"DATA", tcl_dialect::TclVersion::V9_1)
                .is_err()
        );
    }
}
