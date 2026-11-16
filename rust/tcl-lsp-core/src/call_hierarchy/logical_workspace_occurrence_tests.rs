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
use tcl_compiler::analyser::Analyser;

#[test]
fn logical_outgoing_inventory_keeps_separate_scope_recipes_for_repeated_heads() {
    let source = "proc caller {} {namespace eval A {helper}; namespace eval B {helper}}";
    let analysis = Analyser::new().analyse(source, "tcl");
    let item = prepare(source, 0, 6, &analysis).remove(0);
    let calls = unresolved_outgoing_calls(
        source,
        tcl_dialect::DialectProfile::plain_tcl(),
        &item,
        &analysis,
    );
    let helper = calls.iter().find(|call| call.name == "helper").unwrap();
    assert_eq!(helper.from_ranges.len(), 2);
    assert_eq!(helper.logical_occurrences.len(), 2);
    let first = helper.logical_occurrences[0].candidates.as_ref().unwrap();
    let second = helper.logical_occurrences[1].candidates.as_ref().unwrap();
    assert_ne!(first.coordinates(), second.coordinates());
}
