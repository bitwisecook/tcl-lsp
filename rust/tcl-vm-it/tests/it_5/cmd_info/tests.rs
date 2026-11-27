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

//! Behaviour tests moved from `tcl-vm`'s `cmd_info::tests`.

use crate::prelude::*;

#[test]
fn original_info_complete_keeps_string_worker_and_document_source_configuration_separate() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    // Six original String observation fields under actual fixture ingress;
    // no native header encoding or entered-source equivalence is inferred.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::fixture::core(tcl_vm::test_support::profile(engine));
        let jim = engine == "jim";
        let modern = matches!(engine, "tcl9.0" | "tcl9.1");
        for (source, expected) in [
            ("set x ${a{}", !modern),
            ("set x {*}{", engine == "tcl8.4"),
            ("set x \\", !jim),
            ("set x \\\n", jim),
            ("set x \"a\"b {", !jim),
            ("# c\\\r\n", true),
        ] {
            let original = Value::string(source);
            let completion = vm
                .try_invoke_command("info", &[Value::string("complete"), original])
                .unwrap();
            assert_eq!(
                completion.code,
                Code::Ok,
                "{engine}/{source:?}: {completion:?}"
            );
            assert_eq!(
                vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                if expected { b"1" } else { b"0" },
                "{engine}/{source:?}"
            );
        }
    }
    let mut vm = crate::fixture::core(tcl_vm::test_support::profile("tcl9.0"));
    vm.set_dialect_profile(tcl_vm::test_support::profile("tcl8.4"));
    assert_eq!(vm.source_is_complete("set x ${a{}"), Ok(true));
    let completion = vm
        .try_invoke_command(
            "info",
            &[Value::string("complete"), Value::string("set x ${a{}")],
        )
        .unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(
        vm.native_string_bytes(&completion.result).unwrap().as_ref(),
        b"0"
    );
    assert_eq!(vm.source_is_complete("# c\\\r\n"), Ok(false));
}

#[test]
fn original_info_functions_follow_selected_native_script_and_helpers() {
    // Native controls: naming.info.functions-native-script
    // docs/design/analysis/name-resolution-proofs/info-functions-native-script.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_vm::test_support::profile(engine);
        let mut vm = crate::fixture::core(profile);
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let completion = vm
            .eval_source("info functions sin")
            .unwrap_or_else(|error| panic!("{engine}/info functions sin: {error:?}"));
        assert_eq!(completion.code, Code::Ok, "{engine}: {completion:?}");
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            b"sin",
            "{engine}"
        );
        let completion = vm.eval_source("namespace eval ::InfoScope085 {namespace eval tcl::mathfunc {proc local085 {} {return LOCAL}}; info functions local085}").unwrap();
        assert_eq!(completion.code, Code::Ok, "{engine}: {completion:?}");
        let expected: &[u8] = if engine == "tcl8.4" { b"" } else { b"local085" };
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            expected,
            "{engine}"
        );
        if engine != "tcl8.4" {
            let completion = vm.eval_source("rename ::apply ::SavedInfoApply085; proc ::apply args {error APPLY_HELPER085}; info functions sin").unwrap();
            assert_eq!(completion.code, Code::Error, "{engine}: {completion:?}");
            assert_eq!(
                vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                b"APPLY_HELPER085",
                "{engine}"
            );
        }
    }
}

#[test]
fn original_info_loaded_uses_counted_interpreter_path_elements() {
    // Native controls: naming.info.loaded-original-interpreter-path
    // docs/design/analysis/name-resolution-proofs/info-loaded-original-interpreter-path.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_vm::test_support::profile(engine);
        let mut vm = crate::fixture::core(profile);
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        for source in [
            "info loaded {}",
            "interp create InfoParent085; interp eval InfoParent085 {interp create InfoChild085}; info loaded {InfoParent085 InfoChild085}",
            "set child [binary format H* 496e666f5a65726f30383500ff]; interp create $child; info loaded $child",
        ] {
            let completion = vm.eval_source(source).unwrap();
            assert_eq!(completion.code, Code::Ok, "{engine}: {completion:?}");
            assert_eq!(
                completion.result.resident_string_bytes().unwrap().as_ref(),
                b"",
                "{engine}"
            );
        }
        let completion = vm.eval_source("info loaded MissingInfo085").unwrap();
        assert_eq!(completion.code, Code::Error, "{engine}: {completion:?}");
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            b"could not find interpreter \"MissingInfo085\"",
            "{engine}"
        );
    }
}
