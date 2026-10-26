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

//! Direct VM command predicates bounded to the original636 observations.
use crate::Value;
use tcl_cmd_core::index::NativeIndexOps;
use tcl_syntax::native_string::NativeStringProtocol;
include!(
    "../../../../runtime/rust/tests/data/native_original_option_index636/software_controls.rs"
);
fn word(input: &[u8]) -> Value {
    Value::new_native_string_bytes(input).into_native_reference()
}
fn argument(value: &Value) -> Value {
    value.native_lifetime_lease().into_value()
}
fn containing(members: &[&Value], protocol: NativeStringProtocol) -> Value {
    Value::native_list_constructor(
        members.iter().map(|member| argument(member)).collect(),
        protocol,
    )
    .into_native_reference()
}
fn check_original_option_index_scope(ops: &mut crate::Vm) {
    crate::interp::native_operation_currency::NativeOperationScope::enter(ops)
        .unwrap()
        .check(ops)
        .unwrap();
}

fn check_original_option_index_result(
    ops: &mut crate::Vm,
    row: &Row,
    completion: &tcl_runtime_api::Completion<Value>,
    observed: &[Option<&Value>; 2],
) {
    for (ordinal, observed) in observed.iter().enumerate() {
        if let Some(original) = observed {
            assert_eq!(
                original.native_object_type_name(),
                row.member_types[ordinal],
                "{}:{}:{}",
                row.dialect,
                row.case,
                ordinal
            );
            if let Some(resident) = row.member_resident[ordinal] {
                assert_eq!(
                    original.resident_string_bytes().is_some(),
                    resident,
                    "{}:{}:{}",
                    row.dialect,
                    row.case,
                    ordinal
                );
            }
        }
    }
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

#[test]
fn original_search_sort_public_outcomes_and_child_cache_match_native636() {
    // Declared observer ref1 plus real containing-List references. Original
    // cached Int input uses a selected local cache constructor; this does not
    // claim the provider's stack allocation, EvalObjv or transient reference
    // schedule. Error-stack argv formatting and private caches are excluded.
    assert_eq!(ROWS.len(), 192);
    for row in ROWS {
        let mut ops =
            crate::native_fixture::core(crate::environment::profile_for_dialect(row.dialect));
        let dialect = ops.actual_native_invocation_dialect();
        let protocol = dialect.native_string_protocol().unwrap();
        let first = if row.path_kind == 2 {
            let cache = if protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
                tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(0)
            } else {
                tcl_syntax::scalar_getter::NativeScalarCache::Number(
                    tcl_syntax::number::Number::Int(0),
                )
            };
            Value::from_native_scalar_cache(cache, None, dialect)
                .unwrap()
                .into_native_reference()
        } else {
            word(&unhex(row.first))
        };
        let last = if row.path_kind == 3 || row.repeat {
            Some(word(&unhex(row.last)))
        } else {
            None
        };
        let path = if row.path_kind == 0 {
            argument(&first)
        } else {
            let mut members = vec![&first];
            if row.path_kind == 3 {
                members.push(last.as_ref().unwrap());
            }
            if row.path_kind == 4 {
                members.push(&first);
            }
            containing(&members, protocol)
        };
        let path2 = row
            .repeat
            .then(|| containing(&[last.as_ref().unwrap()], protocol));
        let keys = [word(b"B"), word(b"A"), word(b"A"), word(b"B")];
        let inner = [
            containing(&[&keys[0], &keys[1]], protocol),
            containing(&[&keys[2], &keys[3]], protocol),
        ];
        let tails = [word(b"tail0"), word(b"tail1")];
        let rows = if row.nested {
            Some([
                containing(&[&inner[0], &tails[0]], protocol),
                containing(&[&inner[1], &tails[1]], protocol),
            ])
        } else {
            None
        };
        let container = if row.empty {
            containing(&[], protocol)
        } else if let Some([left_row, right_row]) = &rows {
            containing(&[left_row, right_row], protocol)
        } else {
            containing(&[&inner[0], &inner[1]], protocol)
        };
        let option = word(b"-index");
        let pattern = word(b"B");
        let mut args = vec![argument(&option), argument(&path)];
        if let Some(path2) = &path2 {
            args.push(argument(&option));
            args.push(argument(path2));
        }
        args.push(argument(&container));
        if row.case < 16 {
            args.push(argument(&pattern));
        }
        let completion = if row.case < 16 {
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
        check_original_option_index_scope(&mut ops);
        let observed = [
            (row.path_kind != 0).then_some(&first),
            if row.path_kind == 4 {
                Some(&first)
            } else {
                last.as_ref()
            },
        ];
        check_original_option_index_result(&mut ops, row, &completion, &observed);
    }
}
