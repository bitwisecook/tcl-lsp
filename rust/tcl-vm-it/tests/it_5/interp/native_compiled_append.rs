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

mod tests {
    use crate::prelude::*;
    use std::rc::Rc;
    use tcl_runtime_api::Code;

    #[test]
    fn c91_empty_list_append_returns_original_array_default_without_store() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let mut vm = Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            Rc::new(tcl_vm::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        assert_eq!(vm.eval_source("proc watch {n1 n2 op} {lappend ::events $op}; array default set a {A B}; set saved [array default get a]; set events {}; trace add variable a {read write} watch; proc p {} {lappend ::a(missing)}").unwrap().code, Code::Ok);
        let completion = vm.invoke_command("p", &[]);
        assert_eq!(
            completion.code,
            Code::Ok,
            "{:?}",
            completion.result.string_bytes()
        );
        assert_eq!(completion.result.string_bytes().as_ref(), b"A B");
        // Pinned C9.1 control: only READ; the missing element remains undefined.
        assert_eq!(
            vm.get_var_bytes(b"events").unwrap().string_bytes().as_ref(),
            b"read"
        );
        let exists = vm.invoke_command(
            "info",
            &[
                Value::new_native_string_bytes(&b"exists"[..]),
                Value::new_native_string_bytes(&b"a(missing)"[..]),
            ],
        );
        assert_eq!(exists.code, Code::Ok);
        assert_eq!(exists.result.string_bytes().as_ref(), b"0");

        let appended = vm.try_eval_source("lappend absent").unwrap();
        assert_eq!(appended.code, Code::Ok);
        assert_eq!(appended.result.string_bytes().as_ref(), b"");
        assert_eq!(
            vm.get_var_bytes(b"absent").unwrap().string_bytes().as_ref(),
            b""
        );
    }

    #[test]
    fn registered_append_instructions_match_43_native_callback_controls() {
        fn decode(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for row in
            include_str!("../../../../../runtime/rust/tests/data/native_registered_append.tsv")
                .lines()
        {
            let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
            assert_eq!(fields.len(), 7);
            let profile =
                tcl_registry::model::ingress::resolve_environment(fields[0]).unit_profile();
            let mut vm = Vm::with_native_core(
                Box::new(Vec::<u8>::new()),
                Rc::new(tcl_vm::host_native::NativeHost::new()),
                profile,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            vm.set_compiler(Box::new(
                tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
            ));
            assert_eq!(vm.eval_source("proc watch {n1 n2 op} {lappend ::events $op}; proc arm {} {uplevel 1 {trace add variable r {read write} watch}}; set events {}").unwrap().code, Code::Ok);
            let body = Value::new_native_string_bytes(decode(fields[2]));
            let definition = vm.invoke_command(
                "proc",
                &[
                    Value::new_native_string_bytes(&b"p"[..]),
                    Value::new_native_string_bytes(&b"input"[..]),
                    body,
                ],
            );
            assert_eq!(definition.code, Code::Ok);
            let completion =
                vm.invoke_command("p", &[Value::new_native_string_bytes(decode(fields[3]))]);
            assert_eq!(
                completion.code.as_int(),
                fields[4].parse::<i64>().unwrap(),
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                completion.result.string_bytes()
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                decode(fields[5]),
                "{}/{}: result",
                fields[0],
                fields[1]
            );
            let events = vm
                .get_var_bytes(b"events")
                .expect("actual callback event variable");
            assert_eq!(
                events.string_bytes().as_ref(),
                decode(fields[6]),
                "{}/{}: observer order",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 43);
    }
}
