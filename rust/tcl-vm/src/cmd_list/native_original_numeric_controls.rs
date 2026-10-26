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

//! Real original numeric search/sort commands; bounded public rows.
use crate::Value;
use tcl_cmd_core::index::NativeIndexOps;
include!(
    "../../../../runtime/rust/tests/data/native_original_list_numeric645_647/software_controls.rs"
);
fn make(input: (&str, &str), dialect: tcl_registry::InvocationDialect) -> Option<Value> {
    match input.0 {
        "absent" => None,
        "counted-string" => Some(Value::new_native_string_bytes(unhex(input.1))),
        "typed-wide" => Some(Value::int(input.1.parse().unwrap()).into_native_unowned_lifetime()),
        "typed-double" => Some(
            Value::native_double(
                if input.1 == "NaN" {
                    f64::NAN
                } else {
                    input.1.parse().unwrap()
                },
                dialect,
            )
            .into_native_unowned_lifetime(),
        ),
        _ => unreachable!(),
    }
}
fn word(input: &[u8]) -> Value {
    Value::new_native_string_bytes(input).into_native_reference()
}
fn argument(value: &Value) -> Value {
    value.native_lifetime_lease().into_value()
}
#[test]
fn original_numeric_search_sort_public_predicates_match_independent_native645_and647() {

    //645 Glob negative controls and647 Exact/Sorted are independent original rows.
    // Narrow Guest/bytes and reached public primary CATEGORY/residency only.
    // No private cache, argv/error transient refs, result alias, callback sequence
    // or C84 literal Wide descriptor equivalence. Child hold is List-only.
    assert_eq!(ROWS.len(), 384);
    for row in ROWS {
        let mut ops =
            crate::native_fixture::core(crate::environment::profile_for_dialect(row.dialect));
        let dialect = ops.actual_native_invocation_dialect();
        let protocol = dialect.native_string_protocol().unwrap();
        let originals = row.input.map(|input| make(input, dialect));
        // Metadata retains only allocation leases; actual List owns each member.
        let members = originals[..2].iter().flatten().map(argument).collect();
        let container = Value::native_list_constructor(members, protocol).into_native_reference();
        let pattern = originals[2]
            .as_ref()
            .map(argument)
            .map(Value::into_native_reference);
        let mode = word(if row.real { b"-real" } else { b"-integer" });
        let exact = word(b"-exact");
        let extra = row.extra.map(|bytes| word(bytes.as_bytes()));
        let mut args = vec![argument(&mode)];
        if !row.sort && row.series == "exact647" {
            args.push(argument(&exact));
        }
        if let Some(extra) = &extra {
            args.push(argument(extra));
        }
        args.push(argument(&container));
        if let Some(pattern) = &pattern {
            args.push(argument(pattern));
        }
        let completion = if row.sort {
            super::cmd_lsort(&mut ops, &args)
        } else {
            super::cmd_lsearch(&mut ops, &args)
        };
        let label = format!(
            "{}:{}:{}:{}",
            row.series, row.dialect, row.case, row.stream_sha256
        );
        assert_eq!(
            completion.code,
            if row.code == 0 {
                tcl_runtime_api::Code::Ok
            } else {
                tcl_runtime_api::Code::Error
            },
            "{label}"
        );
        crate::interp::native_operation_currency::NativeOperationScope::enter(&mut ops)
            .unwrap()
            .check(&mut ops)
            .unwrap();
        for (original, effect) in originals.iter().zip(row.effect) {
            match (original, effect) {
                (Some(original), Some((expected, resident))) => {
                    assert_eq!(
                        category(original.native_object_type_name()),
                        category(expected),
                        "{label}"
                    );
                    assert_eq!(
                        original.resident_string_bytes().is_some(),
                        resident,
                        "{label}"
                    );
                }
                (None, None) => {}
                _ => panic!("literal original-presence mismatch {label}"),
            }
        }
        assert_eq!(
            ops.index_operand_bytes(&completion.result)
                .unwrap()
                .as_ref(),
            unhex(row.result),
            "{label}"
        );
    }
}
