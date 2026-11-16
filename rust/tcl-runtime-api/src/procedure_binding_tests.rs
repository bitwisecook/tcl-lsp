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

use super::ProcedureBindingIdentity;

#[test]
fn rooted_constructed_namespace_loses_exactly_one_root_marker() {
    let binding =
        ProcedureBindingIdentity::in_rooted_namespace("::n", "p", "::n::p", "", "return ok");
    assert_eq!(binding.resolution_namespace, "n");
    assert_eq!(binding.invocation_name, "p");
    assert_eq!(binding.name, "::n::p");

    assert_eq!(
        ProcedureBindingIdentity::in_rooted_namespace(":::", "p", ":::::p", "", "return ok",)
            .resolution_namespace,
        ":",
        "a literal-colon namespace segment must not be canonicalised as Tcl source",
    );
}
