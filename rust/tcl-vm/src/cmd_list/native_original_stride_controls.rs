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

//! Genuine software callback/currentness controls; not native636 chronology.
use crate::Value;
use tcl_cmd_core::index::NativeIndexOps;
include!("../../../../runtime/rust/tests/data/native_original_list_stride644/software_controls.rs");
fn word(input: &[u8]) -> Value {
    Value::new_native_string_bytes(input).into_native_reference()
}
fn argument(value: &Value) -> Value {
    value.native_lifetime_lease().into_value()
}
#[test]
fn original_stride_guest_and_header_predicates_match_native644() {
    // Actual108 Guest/result/original stride primary/resident fields only.
    // Argref1 and allocation-only transport; no additional observer/nativehold,
    // resultalias, privatecache or provider EvalObjv/callback chronology assertion.
    assert_eq!(ROWS.len(), 108);
    for row in ROWS {
        let mut ops =
            crate::native_fixture::core(crate::environment::profile_for_dialect(row.dialect));
        let protocol = ops
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let stride = word(&unhex(row.stride));
        let container =
            Value::native_list_constructor(Vec::new(), protocol).into_native_reference();
        let option = word(b"-stride");
        let pattern = word(b"B");
        let mut args = vec![argument(&option), argument(&stride), argument(&container)];
        if row.case < 9 {
            args.push(argument(&pattern));
        }
        let completion = if row.case < 9 {
            super::cmd_lsearch(&mut ops, &args)
        } else {
            super::cmd_lsort(&mut ops, &args)
        };
        assert_eq!(
            completion.code,
            if row.code == 0 {
                tcl_runtime_api::Code::Ok
            } else {
                tcl_runtime_api::Code::Error
            },
            "{}:{}:{}",
            row.dialect,
            row.case,
            row.stream_sha256
        );
        crate::interp::native_operation_currency::NativeOperationScope::enter(&mut ops)
            .unwrap()
            .check(&mut ops)
            .unwrap();
        assert_eq!(
            stride.native_object_type_name(),
            row.kind,
            "{}:{}",
            row.dialect,
            row.case
        );
        assert_eq!(
            stride.resident_string_bytes().is_some(),
            row.resident,
            "{}:{}",
            row.dialect,
            row.case
        );
        assert_eq!(
            ops.index_operand_bytes(&completion.result)
                .unwrap()
                .as_ref(),
            unhex(row.result),
            "{}:{}",
            row.dialect,
            row.case
        );
    }
}
