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

use tcl_compiler::analyser::Analyser;
use tcl_core_types::DiagCode;

#[test]
fn mask_inside_script_is_reported_at_the_literal_only_2440() {
    for (source, dialect, expected) in [
        (
            "proc p {} {\n set m 255.0.255.0\n return $m\n}",
            "tcl8.6",
            &["255.0.255.0", "$m"][..],
        ),
        (
            "if {1} { set m 255.0.255.0 }",
            "tcl8.6",
            &["255.0.255.0"][..],
        ),
        (
            "when CLIENT_ACCEPTED { set m 255.0.255.0 }",
            "f5-irules",
            &["255.0.255.0"][..],
        ),
    ] {
        let result = Analyser::new().analyse(source, dialect);
        let findings: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W121)
            .collect();
        let spans: Vec<_> = findings
            .iter()
            .filter_map(|finding| source.get(finding.span.as_range()))
            .collect();
        assert_eq!(spans, expected, "{source}: {findings:?}");
    }
    let result = Analyser::new().analyse("set value {255.0.255.0}", "tcl8.6");
    assert!(result.diagnostics.iter().any(|d| d.code == DiagCode::W121));
}
