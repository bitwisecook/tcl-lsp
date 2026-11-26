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
    #[test]
    fn sequence_backend_capacity_bypasses_guest_capture_and_finally() {
        for script in [
            "set prior BEFORE; catch {lseq 100000001} captured options; set after YES",
            "set prior BEFORE; try {lseq 100000001} finally {set final YES}; set after YES",
        ] {
            let mut vm = tcl_vm::test_support::native_vm();
            vm.set_dialect_profile(
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            );
            vm.set_compiler(Box::new(
                tcl_codegen::compile_service::BytecodeCompileService::default(),
            ));
            let error = vm.try_eval_source(script).unwrap_err();
            let tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::Materialization(refusal),
            ) = error
            else {
                panic!("expected reached materialisation refusal: {error:?}");
            };
            assert_eq!(refusal.requested(), 100_000_001);
            assert_eq!(refusal.limit(), 100_000_000);
            assert_eq!(
                vm.get_var("prior").unwrap().string_bytes().as_ref(),
                b"BEFORE"
            );
            for name in ["captured", "options", "final", "after"] {
                assert!(vm.get_var(name).is_none());
            }
        }
    }
}
