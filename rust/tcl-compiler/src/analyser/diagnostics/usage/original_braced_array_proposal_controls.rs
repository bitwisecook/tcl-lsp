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
fn actual_diagnostic_uses_original_word_and_scanner_index_extent() {
    // `arr` is bound first: unset, `${arr}` raises (tclsh 8.6.18 / 9.0.4)
    // and the third line is never reached.
    let source = "set index x\nset arr v\nputs ${arr}(a\\)b)\nputs ${arr($index)}";
    let result = Analyser::new().analyse(source, "tcl8.6");
    let findings = result
        .diagnostics
        .iter()
        .filter(|finding| finding.code == DiagCode::W216)
        .collect::<Vec<_>>();
    assert_eq!(findings.len(), 2);
    assert_eq!(
        source.get(findings[0].span.as_range()),
        Some("${arr}(a\\)b)")
    );
    assert_eq!(
        source.get(findings[1].span.as_range()),
        Some("${arr($index)}")
    );
    assert_eq!(findings[0].fixes[0].new_text, "$arr(a\\)b)");
    assert_eq!(findings[1].fixes[0].new_text, "$arr($index)");
    for source in ["set \"${arr}(foo)\" 1", "upvar 1 remote \"${arr}(foo)\""] {
        let result = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|finding| finding.code == DiagCode::W216),
            "{source}"
        );
    }
    let result = Analyser::new().analyse("puts \"${arr}(foo)\"", "tcl8.6");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|finding| finding.code == DiagCode::W216)
    );
    let result = Analyser::new().analyse("set \"prefix${arr}(foo)\" 1", "tcl8.6");
    assert!(
        result
            .diagnostics
            .iter()
            .any(|finding| finding.code == DiagCode::W216)
    );
}
