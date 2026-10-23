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

//! Real physical original numeric search/sort commands; bounded public rows.
use crate::{interp::Interp, list, obj};
use tcl_cmd_core::index::NativeIndexOps;
include!("../../tests/data/native_original_list_numeric645_647/software_controls.rs");
fn make(input: (&str, &str)) -> Option<*mut obj::TclObj> {
    match input.0 {
        "absent" => None,
        "counted-string" => Some(obj::new_string_bytes(&unhex(input.1))),
        "typed-wide" => Some(obj::new_wide_int_obj(input.1.parse().unwrap())),
        "typed-double" => Some(obj::new_double_obj(if input.1 == "NaN" {
            f64::NAN
        } else {
            input.1.parse().unwrap()
        })),
        _ => unreachable!(),
    }
}
fn kind(original: *mut obj::TclObj) -> &'static str {
    let descriptor = obj::obj_type_ptr(original);
    if descriptor.is_null() {
        "none"
    } else {
        // SAFETY: actual live original header retains its static type descriptor.
        unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_str()
            .unwrap()
    }
}
#[test]
fn original_numeric_search_sort_public_predicates_match_independent_native645_and647() {
    // naming.list.original-numeric-pattern-key-getters-and-results

    //645 Glob negative controls and647 Exact/Sorted are independent original rows.
    // Narrow Guest/bytes and reached public primary CATEGORY/residency only.
    // No private cache, argv/error transient refs, result alias, callback sequence
    // or C84 literal Wide descriptor equivalence. Child hold is List-only.
    assert_eq!(ROWS.len(), 384);
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
        let originals = row.input.map(make);
        let members = originals[..2].iter().flatten().copied().collect::<Vec<_>>();
        // Original fresh-zero children are owned only by this constructor List.
        let container = obj::Owned::fresh(list::new_list_obj_native(&members, protocol));
        let pattern = originals[2].map(obj::Owned::fresh);
        let name = obj::Owned::fresh(obj::new_string_bytes(if row.sort {
            b"lsort"
        } else {
            b"lsearch"
        }));
        let mode = obj::Owned::fresh(obj::new_string_bytes(if row.real {
            b"-real"
        } else {
            b"-integer"
        }));
        let exact = obj::Owned::fresh(obj::new_string_bytes(b"-exact"));
        let extra = row
            .extra
            .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes.as_bytes())));
        let mut argv = vec![name.as_ptr(), mode.as_ptr()];
        if !row.sort && row.series == "exact647" {
            argv.push(exact.as_ptr());
        }
        if let Some(extra) = &extra {
            argv.push(extra.as_ptr());
        }
        argv.push(container.as_ptr());
        if let Some(pattern) = &pattern {
            argv.push(pattern.as_ptr());
        }
        let code = if row.sort {
            super::lsort(&mut ops, &argv)
        } else {
            super::lsearch(&mut ops, &argv)
        };
        let label = format!(
            "{}:{}:{}:{}",
            row.series, row.dialect, row.case, row.stream_sha256
        );
        assert_eq!(
            code,
            if row.code == 0 {
                crate::interp::Code::Ok
            } else {
                crate::interp::Code::Error
            },
            "{label}"
        );
        assert!(ops.native_execution_refusal().is_none(), "{label}");
        for (original, effect) in originals.into_iter().zip(row.effect) {
            match (original, effect) {
                (Some(original), Some((expected, resident))) => {
                    assert_eq!(category(kind(original)), category(expected), "{label}");
                    assert_eq!(obj::has_string_rep(original), resident, "{label}");
                }
                (None, None) => {}
                _ => panic!("literal original-presence mismatch {label}"),
            }
        }
        let result = ops.get_obj_result();
        assert_eq!(
            ops.index_operand_bytes(&result).unwrap().as_ref(),
            unhex(row.result),
            "{label}"
        );
    }
}
