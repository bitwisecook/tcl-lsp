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

//! Appending to a string value leaves every other holder of it unchanged.

use tcl_syntax::value::ValueOps;

#[test]
fn appending_to_a_shared_string_leaves_its_other_holders_unchanged() {
    for engine in ["tcl8.6", "tcl9.0"] {
        let mut vm =
            crate::fixture::interpreter(tcl_dialect::DialectProfile::find(engine).unwrap());
        let completion = vm.try_eval_source(r#"set name world; set greeting "hello, $name"; set alias $greeting; append greeting "!"; string length $greeting"#).unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok, "{engine}");
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            b"13"
        );
        let greeting = vm.get_var("greeting").unwrap();
        let alias = vm.get_var("alias").unwrap();
        assert_eq!(
            vm.native_string_bytes(&greeting).unwrap().as_ref(),
            b"hello, world!"
        );
        assert_eq!(
            vm.native_string_bytes(&alias).unwrap().as_ref(),
            b"hello, world"
        );
        let completion = vm
            .try_eval_source(r#"set donor original; set retained "$donor"; append retained """#)
            .unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
        let donor = vm.get_var("donor").unwrap();
        let retained = vm.get_var("retained").unwrap();
        assert_eq!(
            vm.native_string_bytes(&donor).unwrap().as_ref(),
            b"original"
        );
        assert_eq!(
            vm.native_string_bytes(&retained).unwrap().as_ref(),
            b"original"
        );
    }
}
