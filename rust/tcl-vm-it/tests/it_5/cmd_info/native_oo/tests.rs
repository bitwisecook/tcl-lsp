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

//! Behaviour tests moved from `tcl-vm`'s `cmd_info::native_oo::tests`.

use crate::prelude::*;

#[test]
fn specialized_object_info_executes_original_operand_before_native_getter() {
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
        let mut vm = crate::fixture::core(profile);
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let result = vm.eval_source("::oo::object create O; set count 0; proc original {} {global count; incr count; return O}; proc c {} {info object class [original]}; proc n {} {info object namespace [original]}; proc i {} {info object isa object [original]}; list [c] [string match ::oo::* [n]] [i] $count").unwrap();
        assert_eq!(
            result.code,
            Code::Ok,
            "{engine}: {}",
            result.result.to_str()
        );
        assert_eq!(
            result.result.string_bytes().as_ref(),
            b"::oo::object 1 1 3",
            "{engine}"
        );
        if engine == "tcl9.1" {
            let epoch = vm
                .eval_source("proc d {} {info object creationid [original]}; d")
                .unwrap();
            assert_eq!(epoch.code, Code::Ok);
            assert!(epoch.result.to_str().parse::<u64>().is_ok());
        }
    }
}

#[test]
fn string_nested_operand_geometry_preserves_quoted_options_and_repeated_calls() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let profile = tcl_vm::test_support::profile(engine);
        let mut vm = crate::fixture::interpreter(profile);
        let source = r#"proc pick {x} {incr ::count; return $x}; set ::count 0; list [string match "*" [pick word]] [string equal -nocase [pick X] [pick x]] [string equal [pick x] [pick x]] [string length "[pick abc]"] $::count"#;
        let result = vm.eval_source(source).unwrap();
        assert_eq!(
            result.code,
            Code::Ok,
            "{engine}: {}",
            result.result.to_str()
        );
        assert_eq!(
            result.result.string_bytes().as_ref(),
            b"1 1 1 3 6",
            "{engine}"
        );
    }
}
