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

//! Behaviour tests moved from `tcl-vm`'s `error::tests`.

use crate::prelude::*;

#[test]
fn expression_quotes_follow_selected_native_control_and_level_protocol() {
    for engine in ["tcl9.0", "jim"] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
        let source =
            "\"prefix [return -level 2 -code error -errorcode {X Y} -errorinfo ORIGINAL MESSAGE]\"";
        let completion = vm.try_eval_expr(source).expect("guest quote completion");
        if engine == "jim" {
            assert_eq!(completion.code, Code::Ok);
            assert_eq!(completion.result.string_bytes().as_ref(), b"prefix MESSAGE");
            assert_eq!(
                crate::fixture::option(&completion.options, "-level")
                    .expect("retained level")
                    .as_int()
                    .unwrap(),
                2
            );
        } else {
            assert_eq!(completion.code, Code::Return);
            assert_eq!(completion.result.string_bytes().as_ref(), b"MESSAGE");
            assert_eq!(
                crate::fixture::option(&completion.options, "-errorcode")
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"X Y"
            );
            assert_eq!(
                crate::fixture::option(&completion.options, "-errorinfo")
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"ORIGINAL"
            );
        }
    }
}

#[test]
fn typed_access_refusal_bypasses_catch_and_finally() {
    fn refuse(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        vm.set_var_bytes(b"reached", Value::int(1))
            .expect("unobserved handler entry marker");
        vm.refuse_host_command("byte refusal".into())
    }
    for source in [
        "catch {byte_refusal} caught; set after 1",
        "try {byte_refusal} on error {m o} {set handled 1} finally {set final 1}; set after 1",
        "after 0 byte_refusal; after 0 {set later 1}; update; set after 1",
    ] {
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
        vm.register("byte_refusal", refuse);
        let refusal = vm.try_eval_source(source).expect_err("host refusal");
        assert!(
            matches!(
                refusal,
                tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_)
            ),
            "{source}: {refusal:?}"
        );
        assert_eq!(
            vm.get_var_bytes(b"reached")
                .expect("actual refusing handler reached")
                .string_bytes()
                .as_ref(),
            b"1"
        );
        for name in ["caught", "handled", "final", "later", "after"] {
            assert!(
                vm.get_var_bytes(name.as_bytes()).is_none(),
                "{source}: {name}"
            );
        }
    }
}
