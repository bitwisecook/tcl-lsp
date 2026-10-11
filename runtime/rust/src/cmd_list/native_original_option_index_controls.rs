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

//! Direct physical command predicates bounded to the original636 observations.
use crate::{
    interp::Interp,
    list,
    obj::{self, TclObj},
};
use tcl_cmd_core::index::NativeIndexOps;
use tcl_syntax::native_string::NativeStringProtocol;
include!("../../tests/data/native_original_option_index636/software_controls.rs");

fn own(owners: &mut Vec<obj::Owned>, pointer: *mut TclObj) -> *mut TclObj {
    owners.push(obj::Owned::fresh(pointer));
    pointer
}
fn bytes(owners: &mut Vec<obj::Owned>, input: &[u8]) -> *mut TclObj {
    own(owners, obj::new_string_bytes(input))
}
fn containing(
    owners: &mut Vec<obj::Owned>,
    members: &[*mut TclObj],
    protocol: NativeStringProtocol,
) -> *mut TclObj {
    own(owners, list::new_list_obj_native(members, protocol))
}
fn kind(pointer: *mut TclObj) -> &'static str {
    let ty = obj::obj_type_ptr(pointer);
    if ty.is_null() {
        return "none";
    }
    // SAFETY: the independently owned observed child has this static descriptor.
    unsafe { std::ffi::CStr::from_ptr((*ty).name) }
        .to_str()
        .unwrap()
}
#[test]
fn original_search_sort_public_outcomes_and_child_cache_match_native636() {
    // naming.list.original-option-index-path-preparation-and-results
    // docs/design/analysis/name-resolution-proofs/list-original-option-index-path-preparation-and-results.md
    // Each observed original child has a declared observer ref1 plus the real
    // containing List holds. No provider EvalObjv transient count, error-stack
    // argv formatter, result alias/refcount or private cache is asserted here.
    assert_eq!(ROWS.len(), 192);
    for row in ROWS {
        let mut ops = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(row.dialect),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let protocol = ops
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let version = protocol.tcl_version();
        let mut owners = Vec::new();
        let mut observed = [None, None];
        let path = if row.path_kind == 0 {
            bytes(&mut owners, &unhex(row.first))
        } else {
            let first = if row.path_kind == 2 {
                // Actual local selected Int primary constructor, not a Wide
                // or textual 0 substitute. C84's native-long descriptor is distinct.
                let descriptor = if version == Some(tcl_dialect::TclVersion::V8_4) {
                    &obj::TCL_LONG84_TYPE
                } else {
                    &obj::TCL_INT_TYPE
                };
                own(&mut owners, obj::alloc_typed(descriptor, 0))
            } else {
                bytes(&mut owners, &unhex(row.first))
            };
            observed[0] = Some(first);
            let mut members = vec![first];
            if row.path_kind == 3 {
                let last = bytes(&mut owners, &unhex(row.last));
                observed[1] = Some(last);
                members.push(last);
            }
            if row.path_kind == 4 {
                observed[1] = Some(first);
                members.push(first);
            }
            containing(&mut owners, &members, protocol)
        };
        let path2 = if row.repeat {
            let last = bytes(&mut owners, &unhex(row.last));
            observed[1] = Some(last);
            Some(containing(&mut owners, &[last], protocol))
        } else {
            None
        };
        let keys = [
            bytes(&mut owners, b"B"),
            bytes(&mut owners, b"A"),
            bytes(&mut owners, b"A"),
            bytes(&mut owners, b"B"),
        ];
        let inner = [
            containing(&mut owners, &keys[..2], protocol),
            containing(&mut owners, &keys[2..], protocol),
        ];
        let rows = if row.nested {
            let tail0 = bytes(&mut owners, b"tail0");
            let tail1 = bytes(&mut owners, b"tail1");
            [
                containing(&mut owners, &[inner[0], tail0], protocol),
                containing(&mut owners, &[inner[1], tail1], protocol),
            ]
        } else {
            inner
        };
        let container = containing(&mut owners, if row.empty { &[] } else { &rows }, protocol);
        let command = if row.case < 16 {
            b"lsearch".as_slice()
        } else {
            b"lsort".as_slice()
        };
        let mut argv = vec![
            bytes(&mut owners, command),
            bytes(&mut owners, b"-index"),
            path,
        ];
        if let Some(path2) = path2 {
            argv.push(bytes(&mut owners, b"-index"));
            argv.push(path2);
        }
        argv.push(container);
        if row.case < 16 {
            argv.push(bytes(&mut owners, b"B"));
        }
        let code = if row.case < 16 {
            super::lsearch(&mut ops, &argv)
        } else {
            super::lsort(&mut ops, &argv)
        };
        assert_eq!(
            code,
            if row.code == 0 {
                crate::interp::Code::Ok
            } else {
                crate::interp::Code::Error
            },
            "{}:{}:{}",
            row.dialect,
            row.case,
            row.stream_sha256
        );
        assert!(
            ops.native_execution_refusal().is_none(),
            "{}:{}",
            row.dialect,
            row.case
        );
        for (ordinal, observed) in observed.iter().enumerate() {
            if let Some(original) = observed {
                assert_eq!(
                    kind(*original),
                    row.member_types[ordinal],
                    "{}:{}:{}",
                    row.dialect,
                    row.case,
                    ordinal
                );
                if let Some(resident) = row.member_resident[ordinal] {
                    assert_eq!(
                        obj::has_string_rep(*original),
                        resident,
                        "{}:{}:{}",
                        row.dialect,
                        row.case,
                        ordinal
                    );
                }
            }
        }
        let result = ops.get_obj_result();
        assert_eq!(
            ops.index_operand_bytes(&result).unwrap().as_ref(),
            unhex(row.result),
            "{}:{}",
            row.dialect,
            row.case
        );
        // Native reference observations belong to these explicitly declared
        // local constructors. Readable raw results do not donate a new owner.
        drop(ops);
        while let Some(owner) = owners.pop() {
            drop(owner);
        }
    }
}
