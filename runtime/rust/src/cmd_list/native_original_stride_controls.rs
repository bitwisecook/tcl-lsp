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
use crate::{interp::Interp, list, obj};
use tcl_cmd_core::index::NativeIndexOps;
include!("../../tests/data/native_original_list_stride644/software_controls.rs");
fn kind(original: *mut obj::TclObj) -> &'static str {
    let descriptor = obj::obj_type_ptr(original);
    if descriptor.is_null() {
        return "none";
    }
    // SAFETY: the independently owned original has this static descriptor.
    unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
        .to_str()
        .unwrap()
}
#[test]
fn original_stride_guest_and_header_predicates_match_native644() {
    // naming.list.original-stride-primitive-and-results
    // docs/design/analysis/name-resolution-proofs/list-original-stride-primitive-and-results.md
    // Direct software commands compare actual108 Guest/result/stride-primary/resident
    // fields before resultgetter. Nativeargvtemporarycounts, resultaliases/privatecache,
    // errorInfo and external callback schedule are excluded. Original argref1 only.
    assert_eq!(ROWS.len(), 108);
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
        let stride = obj::Owned::fresh(obj::new_string_bytes(&unhex(row.stride)));
        let container = obj::Owned::fresh(list::new_list_obj_native(&[], protocol));
        let name = obj::Owned::fresh(obj::new_string_bytes(if row.case < 9 {
            b"lsearch"
        } else {
            b"lsort"
        }));
        let option = obj::Owned::fresh(obj::new_string_bytes(b"-stride"));
        let pattern = obj::Owned::fresh(obj::new_string_bytes(b"B"));
        let mut argv = vec![
            name.as_ptr(),
            option.as_ptr(),
            stride.as_ptr(),
            container.as_ptr(),
        ];
        if row.case < 9 {
            argv.push(pattern.as_ptr());
        }
        let code = if row.case < 9 {
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
        assert_eq!(
            kind(stride.as_ptr()),
            row.kind,
            "{}:{}",
            row.dialect,
            row.case
        );
        assert_eq!(
            obj::has_string_rep(stride.as_ptr()),
            row.resident,
            "{}:{}",
            row.dialect,
            row.case
        );
        let result = ops.get_obj_result();
        assert_eq!(
            ops.index_operand_bytes(&result).unwrap().as_ref(),
            unhex(row.result),
            "{}:{}",
            row.dialect,
            row.case
        );
    }
}
