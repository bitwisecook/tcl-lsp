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

use super::*;

#[test]
fn cargo_fixture_environment_bridge() {
    if env::var_os("TCL_LSP_CARGO_FIXTURE_TEST_BRIDGE").is_some() {
        cargo_fixture_self_test_inner().expect("isolated Cargo fixture self-test");
    }
}

#[test]
fn target_helpers_cover_static_edge_cases() {
    self_test().expect("smoke target helper self-test");
}

#[test]
fn namespaced_include_preserves_smoke_module_context() {
    inline_smoke_module_include_self_test()
        .expect("namespaced include smoke-module context self-test");
}

#[test]
fn namespaced_include_scanner_recognises_builtin_namespaces() {
    namespaced_include_scanner_self_test().expect("namespaced include lexical scanner self-test");
}

#[test]
fn macro_rules_include_preserves_smoke_module_context() {
    macro_rules_smoke_module_include_self_test()
        .expect("macro_rules include smoke-module context self-test");
}
