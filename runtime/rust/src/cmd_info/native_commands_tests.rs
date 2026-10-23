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

//! Original command-table slots and native pattern-object enumeration.
use crate::{
    interp::{Code, Interp},
    obj,
};
include!("../../../../rust/tcl-registry/tests/data/native_info_commands/cases.rs");

fn define_names(interp: &mut Interp) {
    assert_eq!(interp.eval_str(b"namespace eval N {}"), Code::Ok);
    let head = obj::Owned::fresh(obj::new_string_bytes(b"proc"));
    for name in INFO_COMMAND_NAMES {
        let original = obj::Owned::fresh(obj::new_string_bytes(name));
        let empty = obj::Owned::fresh(obj::new_string_bytes(b""));
        let code = interp.eval_original_object_vector(&[
            head.as_ptr(),
            original.as_ptr(),
            empty.as_ptr(),
            empty.as_ptr(),
        ]);
        assert_eq!(code, Code::Ok);
        assert!(
            !interp.host_refusal_pending(),
            "{:?}",
            interp.native_access_refusal()
        );
    }
}

#[test]
fn original_info_commands_matches_all_110_native_pattern_object_windows() {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    let rows =
        include_str!("../../../../rust/tcl-registry/tests/data/native_info_commands/windows.tsv");
    let mut count = 0;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interp = Interp::with_native_core(
            std::rc::Rc::new(tcl_host_native::NativeHost::new()),
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        define_names(&mut interp);
        for row in rows.lines().skip(1) {
            let fields: Vec<_> = row.split('|').collect();
            if fields[0] != engine {
                continue;
            }
            let byte_array = fields[1] == "1";
            let input = INFO_COMMAND_PATTERNS[fields[2].parse::<usize>().unwrap()];
            let original = obj::Owned::fresh(if byte_array {
                crate::bytearray::new_byte_array(
                    input,
                    interp
                        .native_invocation_dialect()
                        .byte_array_string_recipe(None)
                        .unwrap(),
                )
            } else {
                obj::new_string_bytes(input)
            });
            if byte_array {
                assert!(matches!(
                    obj::native_object_snapshot(original.as_ptr())
                        .unwrap()
                        .cache,
                    Cache::ByteArray { .. }
                ));
            }
            let head = obj::Owned::fresh(obj::new_string_bytes(b"info"));
            let command = obj::Owned::fresh(obj::new_string_bytes(b"commands"));
            let code = interp.eval_original_object_vector(&[
                head.as_ptr(),
                command.as_ptr(),
                original.as_ptr(),
            ]);
            assert!(
                !interp.host_refusal_pending(),
                "{row}: {:?}",
                interp.native_access_refusal()
            );
            assert_eq!(code, Code::from_int(fields[3].parse().unwrap()), "{row}");
            if byte_array {
                assert!(
                    matches!(
                        obj::native_object_snapshot(original.as_ptr())
                            .unwrap()
                            .cache,
                        Cache::ByteArray { .. }
                    ),
                    "{row}"
                );
            }
            let members = crate::list::list_elements_native_checked(
                interp.result_obj(),
                interp
                    .native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(members.len().to_string(), fields[4], "{row}");
            let mut names: Vec<_> = members
                .iter()
                .map(|&name| info_command_hex(&crate::interp::obj_bytes(name)))
                .collect();
            names.sort();
            assert_eq!(names.join(","), fields[5], "{row}");
            count += 1;
        }
    }
    assert_eq!(count, 110);
}
