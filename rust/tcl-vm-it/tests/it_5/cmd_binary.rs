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

//! `binary scan` assignments through variable traces.

mod original_format_scan_tests {
    use crate::prelude::*;

    #[test]
    fn original_binary_scan_trace_first_cause_stops_later_assignment() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl8.6"));
        vm.register("originalBinaryTrace", |vm, _| {
            vm.refuse_host_command("binary assignment first Host".into())
        });
        crate::fixture::eval(
            &mut vm,
            "trace add variable first write originalBinaryTrace",
        );
        let refusal = vm
            .try_invoke_command(
                "binary",
                &[
                    Value::string("scan"),
                    Value::string("AB"),
                    Value::string("a1a1"),
                    Value::string("first"),
                    Value::string("later"),
                ],
            )
            .unwrap_err();
        assert!(matches!(
            refusal,
            tcl_runtime_api::NativeExecutionError::HostCommandRefusal(_)
        ));
        assert_eq!(
            crate::fixture::eval(&mut vm, "info exists later")
                .result
                .to_str()
                .as_ref(),
            "0"
        );
    }
}
