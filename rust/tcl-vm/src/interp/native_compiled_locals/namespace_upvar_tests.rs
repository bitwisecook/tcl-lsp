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

//! Native C8.5 namespace-upvar execution uses original namespace operands.

use crate::interp::Vm;
use std::rc::Rc;
use tcl_runtime_api::Code;

mod inputs {
    include!(
        "../../../../tcl-registry/tests/data/native_namespace_upvar_compilation/execution_cases.rs"
    );
}

#[test]
fn original_c85_namespace_upvar_opcode_matches_native_namespace_lifetimes() {
    for &(label, body, expected_code, expected, visited) in inputs::EXECUTIONS {
        let profile = tcl_dialect::DialectProfile::find("tcl8.5").unwrap();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let mut source = b"namespace eval ::N {variable x X;variable y Y};proc p {} {".to_vec();
        source.extend_from_slice(body);
        source.extend_from_slice(b"};p");
        let completion = vm
            .try_eval_source_bytes(&source)
            .expect("authentic original source admission");
        assert_eq!(
            completion.code,
            if expected_code == 0 {
                Code::Ok
            } else {
                Code::Error
            },
            "{label}"
        );
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            expected,
            "{label}"
        );
        assert!(vm.refused_completion().is_none(), "{label}");
        let completion = vm.try_eval_source("info exists ::visited").unwrap();
        assert_eq!(completion.code, Code::Ok, "{label}");
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            if visited {
                b"1".as_slice()
            } else {
                b"0".as_slice()
            },
            "{label}: target precedes namespace lookup"
        );
    }
}
