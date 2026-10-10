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
    // Execute the exact original object-vector operands, not a source/CLI
    // rewrite. Observe actual primary/residency before result getters, including
    // input ByteArray string materialization, raw output and distinct error
    // fields. Original pointer/refcount identity is not measured or asserted.
    use crate::{interp::Interp, obj::Owned};
    use tcl_test_support::encoding_utf8::{CONTROLS, object_observation_rows, original_input};
    let mut comparisons = 0;
    for control in CONTROLS {
        for (engine, column) in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]
            .into_iter()
            .zip(control.columns)
        {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let bytes = original_input(column);
            let input = Owned::fresh(if control.binary {
                interp.new_native_byte_array(&bytes).unwrap()
            } else {
                crate::obj::new_string_bytes(&bytes)
            });
            let before = crate::obj::native_object_snapshot(input.as_ptr()).unwrap();
            let mut rows = vec![format!(
                "INPUT|{}|{}|{}|{}",
                control.case,
                primary(&before),
                u8::from(before.resident.is_some()),
                hex(&bytes)
            )];
            let head = Owned::fresh(crate::obj::new_string_bytes(b"encoding"));
            let sub = Owned::fresh(crate::obj::new_string_bytes(b"convertto"));
            let target = Owned::fresh(crate::obj::new_string_bytes(b"utf-8"));
            let code = interp.eval_original_object_vector(&[
                head.as_ptr(),
                sub.as_ptr(),
                target.as_ptr(),
                input.as_ptr(),
            ]);
            assert!(
                !interp.host_refusal_pending(),
                "{engine}/{}: {:?}",
                control.case,
                interp.native_access_refusal()
            );
            let result = Owned::retain(interp.result_obj());
            let output = crate::obj::native_object_snapshot(result.as_ptr()).unwrap();
            let after = crate::obj::native_object_snapshot(input.as_ptr()).unwrap();
            rows.push(format!(
                "RETURN|{}|{}|{}",
                code.as_int(),
                primary(&output),
                u8::from(output.resident.is_some())
            ));
            rows.push(format!(
                "INPUT_AFTER|{}|{}",
                primary(&after),
                u8::from(after.resident.is_some())
            ));
            if code == crate::interp::Code::Ok {
                let raw = interp.binary_bytes(result.as_ptr()).unwrap();
                rows.push(format!("RAW|{}|{}", raw.len(), hex(&raw)));
            } else {
                let message = interp.native_object_string_bytes(result.as_ptr()).unwrap();
                rows.push(format!("MESSAGE|{}", hex(&message)));
                let getter = Owned::fresh(crate::obj::new_string_bytes(b"set"));
                let name = Owned::fresh(crate::obj::new_string_bytes(b"::errorCode"));
                assert_eq!(
                    interp.eval_original_object_vector(&[getter.as_ptr(), name.as_ptr()]),
                    crate::interp::Code::Ok
                );
                rows.push(format!("ERROR_CODE|{}", hex(&interp.result_bytes())));
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
    // Authored API negative control: no native encoding extension or result is
    // inferred from an independently allocated input String or chosen version.
    use tcl_syntax::value::ValueOps;
    for engine in ["tcl8.4", "jim"] {
        let mut interp = crate::interp::Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let result = interp.result_obj();
        assert!(
            ValueOps::native_external_utf8_result(
                &mut interp,
                b"DATA",
                tcl_dialect::TclVersion::V9_1
            )
            .is_err()
        );
        assert_eq!(interp.result_obj(), result);
    }
}
