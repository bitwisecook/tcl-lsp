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

//! `return` option merging in compiled procedures against the C captures.

mod execution_tests {
    use crate::prelude::*;

    #[test]
    fn registered_return_instructions_match_44_native_controls() {
        let mut compared = 0;
        for line in include_str!("../../../tcl-registry/tests/data/registered-return44.tsv").lines()
        {
            let fields = tcl_test_support::fixture::tsv_fields(line).collect::<Vec<_>>();
            let profile = tcl_vm::test_support::profile(match fields[0] {
                "8.5.19" => "tcl8.5",
                "8.6.18" => "tcl8.6",
                "9.0.4" => "tcl9.0",
                "9.1.0" => "tcl9.1",
                _ => panic!("original C version"),
            });
            let mut vm = crate::fixture::interpreter(profile);
            let body = Value::from_string_bytes(crate::fixture::unhex(fields[2]).as_slice());
            assert_eq!(
                crate::fixture::call(
                    &mut vm,
                    "proc",
                    &[Value::string("p"), Value::string("opts msg code"), body]
                )
                .code,
                Code::Ok
            );
            let result = crate::fixture::call(
                &mut vm,
                "p",
                &[
                    Value::string("-custom DYNAMIC"),
                    Value::string("MSG"),
                    Value::string("error"),
                ],
            );
            assert_eq!(
                result.code,
                Code::from_int(fields[3].parse().unwrap()),
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                result
            );
            assert_eq!(
                vm.native_string_bytes(&result.result).unwrap().as_ref(),
                crate::fixture::unhex(fields[4]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 44);
    }
}
