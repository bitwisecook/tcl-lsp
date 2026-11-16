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

#[test]
fn original_namespace_unknown_operands_remain_coverage_obligations() {
    let source = "namespace eval ::old {}\nproc mk {ns} {namespace eval $ns {}; namespace eval ::other::$ns {}}";
    let mut analyser = crate::analyser::Analyser::new();
    let analysis = analyser.analyse(source, "tcl8.6");
    let spellings = analysis
        .namespace_name_unknowns
        .iter()
        .map(|span| &source[span.as_range()])
        .collect::<Vec<_>>();
    assert_eq!(spellings, ["$ns", "::other::$ns"]);
    assert!(
        analysis
            .namespace_refs
            .iter()
            .all(|row| row.original_name != "$ns")
    );
}
