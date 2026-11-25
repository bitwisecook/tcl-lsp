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

use super::*;
use tcl_syntax::value::ValueOps;

fn native() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .unwrap()
}
fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn actual_native_c86_split_preserves_18_raw_rows_and_result_ownership() {
    crate::counters::reset();
    {
        let rows =
            include_str!("../../../../rust/tcl-cmd-core/tests/data/native_split_c86/rows.tsv");
        let mut count = 0;
        for row in rows.lines() {
            let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
            let mut interp = native();
            let original_bytes = hex(fields[2]);
            let input = obj::Owned::fresh(if fields[1] == "binary" {
                crate::bytearray::new_byte_array(
                    &original_bytes,
                    interp
                        .native_invocation_dialect()
                        .byte_array_string_recipe(None)
                        .unwrap(),
                )
            } else {
                obj::new_string_bytes(&original_bytes)
            });
            let delimiter = (fields[4] != "DEFAULT")
                .then(|| obj::Owned::fresh(obj::new_string_bytes(&hex(fields[4]))));
            let mut argv = vec![obj::new_string_bytes(b"split"), input.as_ptr()];
            let head = obj::Owned::fresh(argv[0]);
            if let Some(delimiter) = &delimiter {
                argv.push(delimiter.as_ptr());
            }
            assert_eq!(
                super::split(&mut interp, &argv),
                Code::Ok,
                "{}: {:?}",
                fields[0],
                interp.native_execution_refusal()
            );
            assert_eq!(
                obj_bytes(input.as_ptr()),
                hex(fields[3]),
                "{} original resident bytes",
                fields[0]
            );
            let result = obj::Owned::retain(interp.get_obj_result());
            if fields[5] == "0" {
                assert!(obj::obj_type_ptr(result.as_ptr()).is_null());
            }
            let members = interp.list_elements(&result.as_ptr()).unwrap();
            let expected: Vec<_> = if fields[5] == "0" {
                Vec::new()
            } else {
                fields[6].split(',').map(hex).collect()
            };
            assert_eq!(
                members
                    .iter()
                    .map(|&part| obj_bytes(part))
                    .collect::<Vec<_>>(),
                expected,
                "{}",
                fields[0]
            );
            drop(head);
            count += 1;
        }
        assert_eq!(count, 18);
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

#[test]
fn empty_set_reuses_actual_children_and_retained_child_survives_list_retirement() {
    crate::counters::reset();
    {
        let mut interp = native();
        assert_eq!(interp.eval_str(b"split ababa {}"), Code::Ok);
        let result = obj::Owned::retain(interp.get_obj_result());
        let members = interp.list_elements(&result.as_ptr()).unwrap();
        assert_eq!(members[0], members[2]);
        assert_eq!(members[0], members[4]);
        assert_ne!(members[0], members[1]);
        assert_eq!(unsafe { (*members[0]).ref_count }, 3);
        let retained = obj::Owned::retain(members[0]);
        interp.set_result(obj::new_obj());
        drop(result);
        assert_eq!(unsafe { (*retained.as_ptr()).ref_count }, 1);
        assert_eq!(obj_bytes(retained.as_ptr()), b"a");
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

#[test]
fn written_nul_delimiter_reaches_actual_source_producer_before_binary_split() {
    let mut interp = native();
    assert_eq!(
        interp.eval_str(b"set b [binary format H* 61626300]; split $b \\0"),
        Code::Ok
    );
    let members = interp.list_elements(&interp.get_obj_result()).unwrap();
    assert_eq!(
        members
            .iter()
            .map(|&part| obj_bytes(part))
            .collect::<Vec<_>>(),
        vec![b"abc".to_vec(), Vec::new()]
    );
}
