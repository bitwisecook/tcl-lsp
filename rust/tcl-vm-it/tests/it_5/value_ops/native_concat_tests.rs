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

//! Expanded `concat` in compiled procedures against the original captures.

use crate::prelude::*;

include!("../../../../tcl-registry/tests/data/native_concat_expansion/cases.rs");
include!("../../../../tcl-cmd-core/tests/data/native_concat/compiled_cases.rs");

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").unwrap();
        text
    })
}

#[test]
fn compiled_concat_expansion_preserves_all_48_original_native_windows() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for (case, body) in EXPANSION_BODIES.iter().enumerate() {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
            let words = [
                Value::string("p"),
                Value::string("a c"),
                Value::from_string_bytes(*body),
            ];
            let definition = crate::fixture::call(&mut vm, "proc", &words);
            assert_eq!(definition.code, Code::Ok, "{engine}/{case}: {definition:?}");
            let result =
                crate::fixture::call(&mut vm, "p", &[Value::string("D E"), Value::string("C")]);
            let expected = expansion_result(engine, case);
            assert_eq!(
                result.code.as_int().to_string(),
                expected[2],
                "{engine}/{case}: {result:?}"
            );
            assert_eq!(
                hex(&vm.native_string_bytes(&result.result).unwrap()),
                expected[6],
                "{engine}/{case} original result"
            );
        }
    }
}

#[test]
fn compiled_concat_preserves_all_eight_original_c_result_headers() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (case, &body) in COMPILED_BODIES.iter().enumerate() {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
            let words = [
                Value::string("p"),
                Value::string(""),
                Value::from_string_bytes(body),
            ];
            let definition = crate::fixture::call(&mut vm, "proc", &words);
            assert_eq!(
                definition.code,
                Code::Ok,
                "{engine}/{case} definition {definition:?}"
            );
            let result = crate::fixture::call(&mut vm, "p", &[]);
            let expected = compiled_row(engine, case);
            assert_eq!(
                result.code.as_int().to_string(),
                expected[3],
                "{engine}/{case} {result:?}"
            );
        }
    }
}
