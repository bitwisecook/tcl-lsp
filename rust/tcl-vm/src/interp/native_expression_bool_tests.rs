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

//! Public condition evaluation retains actual completion and operation ownership.

#[test]
fn original_expression_condition_does_not_install_source_compiler() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = crate::native_fixture::core(profile);
    assert!(vm.compiler.is_none());
    assert!(
        vm.try_eval_expr_bool_bytes(b"[set condition_probe 1]")
            .is_err()
    );
    assert!(vm.compiler.is_none());
    assert_eq!(vm.host_execution_depth, 0);
    assert!(!vm.has_entered_native_operation());
}
